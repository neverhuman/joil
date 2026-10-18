use crate::ledger::{Resources, WorkClass};
use crate::{BlobDigest, Result, RuntimeError, SemanticId};
use crucible_contract::keys::EvaluationKey;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureMode {
    Succeed,
    NumericalFailure,
    Timeout,
    Unsupported,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RngAddress {
    pub campaign: SemanticId,
    pub island: u32,
    pub epoch: u64,
    pub proposal: u64,
    pub operator: String,
    pub draw: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureRequest {
    pub schema_version: u32,
    pub values: Vec<i64>,
    pub address: RngAddress,
    pub mode: FixtureMode,
    pub executable: BlobDigest,
}
impl FixtureRequest {
    pub fn check(&self) -> Result<()> {
        if self.schema_version != 1
            || self.values.is_empty()
            || self.values.len() > 64
            || self.values.iter().any(|v| v.unsigned_abs() > 1_000_000)
            || self.address.operator.trim().is_empty()
            || self.address.epoch > 2_u64.pow(53)
            || self.address.proposal > 2_u64.pow(53)
            || self.address.draw > 2_u64.pow(53)
        {
            return Err(RuntimeError::Invalid(
                "nonempty_bounded_software_fixture_request",
            ));
        }
        Ok(())
    }
    pub fn synthetic_score(&self) -> Result<i64> {
        self.check()?;
        self.values.iter().try_fold(0_i64, |sum, v| {
            sum.checked_add(
                v.checked_mul(*v)
                    .ok_or(RuntimeError::Invalid("fixture_product_overflow"))?,
            )
            .ok_or(RuntimeError::Invalid("fixture_sum_overflow"))
        })
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "domain", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    SoftwareFixture { request: FixtureRequest },
    Physical { key: Box<EvaluationKey> },
}
impl Request {
    pub fn identity(&self) -> Result<SemanticId> {
        match self {
            Self::SoftwareFixture { request } => {
                request.check()?;
                SemanticId::of(&(
                    "crucible.software-fixture/1",
                    request.schema_version,
                    &request.values,
                    &request.address,
                    &request.executable,
                ))
                .map_err(|e| RuntimeError::Wire(e.to_string()))
            }
            Self::Physical { key } => key
                .identity()
                .map_err(|e| RuntimeError::Wire(e.to_string())),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    pub request: Request,
    pub due_epoch: u64,
    pub ordinal: u64,
    pub class: WorkClass,
    pub reserve: Resources,
    pub ranks: u32,
    pub threads: u32,
}
impl TaskSpec {
    pub fn check(&self) -> Result<()> {
        self.request.identity()?;
        self.reserve.check()?;
        if self.due_epoch > 2_u64.pow(53)
            || self.ordinal > 2_u64.pow(53)
            || self.ranks == 0
            || self.threads == 0
            || self.ranks.checked_mul(self.threads).is_none()
        {
            return Err(RuntimeError::Invalid("task_address_or_threads"));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Ready,
    Leased,
    Running,
    Staged,
    Verified,
    Committed,
    NumericalFailure,
    InfrastructureFailure,
    Unsupported,
    TimedOut,
    Cancelled,
    Quarantined,
}
impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Leased => "leased",
            Self::Running => "running",
            Self::Staged => "staged",
            Self::Verified => "verified",
            Self::Committed => "committed",
            Self::NumericalFailure => "numerical_failure",
            Self::InfrastructureFailure => "infrastructure_failure",
            Self::Unsupported => "unsupported",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
            Self::Quarantined => "quarantined",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub id: String,
    pub task_id: String,
    pub fence: u64,
    pub lease_id: String,
    pub expires_ms: u64,
    pub evaluation: SemanticId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureObservation {
    pub schema_version: u32,
    pub synthetic_only: bool,
    pub request: SemanticId,
    pub score: i64,
    pub address: RngAddress,
}

use crate::{
    db::RuntimeDb,
    lease::{scratch_bytes, AdmittedLease, FixtureExecutable, OwnedProcess},
    ledger::Charge,
};
use crucible_contract::outcomes::TaskOutcome;
use crucible_store::{
    manifest::{Metadata, ObjectKind},
    Store,
};
use std::{
    collections::BTreeSet,
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    time::{Duration, Instant},
};
#[derive(Clone, Debug)]
pub struct Execution {
    pub outcome: TaskOutcome,
    pub phase: Phase,
    pub charge: Charge,
    pub object: Option<BlobDigest>,
    pub diagnostic: Option<BlobDigest>,
}
fn cpu_thread_us() -> Result<u64> {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrusage(libc::RUSAGE_THREAD, &mut usage) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let total = i128::from(usage.ru_utime.tv_sec) * 1_000_000
        + i128::from(usage.ru_utime.tv_usec)
        + i128::from(usage.ru_stime.tv_sec) * 1_000_000
        + i128::from(usage.ru_stime.tv_usec);
    total
        .try_into()
        .map_err(|_| RuntimeError::Invalid("coordinator_cpu_usage_range"))
}
fn new_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}
fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    if !f.metadata()?.is_file() || f.metadata()?.len() > 1_048_576 {
        return Err(RuntimeError::Invalid("fixture_output_type_or_size"));
    }
    let mut bytes = Vec::new();
    f.take(1_048_577).read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err(RuntimeError::Invalid("fixture_output_size"));
    }
    Ok(bytes)
}
/// Runs only the bounded synthetic executable. A worker exit code is never a
/// physical verdict. Fixed fault injection is operational, outside observation
/// identity; successful retry uses exactly the same algorithm/input/executable.
pub fn execute_fixture(
    db: &mut RuntimeDb,
    store: &Store,
    attempt: &Attempt,
    lease: &AdmittedLease,
    executable: &FixtureExecutable,
    timeout: Duration,
) -> Result<Execution> {
    execute_fixture_checked(db, store, attempt, lease, executable, timeout, || {})
}
/// Preparation fault hook is trusted coordinator test code, never a worker payload.
/// `timeout` bounds native execution; preparation/verification stay in the actual
/// CPU/wall charge, and the absolute lease still bounds every phase.
pub fn execute_fixture_checked(
    db: &mut RuntimeDb,
    store: &Store,
    attempt: &Attempt,
    lease: &AdmittedLease,
    executable: &FixtureExecutable,
    timeout: Duration,
    prepare: impl FnOnce(),
) -> Result<Execution> {
    if timeout.is_zero() || timeout > Duration::from_secs(30) {
        return Err(RuntimeError::Invalid("fixture_deadline"));
    }
    let start = Instant::now();
    let cpu_start = cpu_thread_us()?;
    let row = db.task(&attempt.task_id)?;
    let now = crate::utc_millis()?;
    db.start(attempt, now)?;
    let mut cpu_us = 0_u64;
    let mut peak_ram = 0;
    let mut io_bytes = 0;
    let mut scratch = 0;
    let mut object = None;
    let mut outcome = TaskOutcome::Unsupported;
    let mut state = Phase::Unsupported;
    let mut diagnostics = Vec::<String>::new();
    if let Request::SoftwareFixture { request } = &row.spec.request {
        if &request.executable != executable.digest() {
            diagnostics.push("fixture executable digest differs from frozen request".into());
            state = Phase::InfrastructureFailure;
            outcome = TaskOutcome::InfrastructureFailure { diagnostic: None };
        } else if request.mode != FixtureMode::Unsupported {
            let path = store.path().join(format!(
                "attempt-{}",
                BlobDigest::of_bytes(attempt.id.as_bytes()).as_str()
            ));
            if path.exists() {
                return Err(RuntimeError::Conflict(
                    "attempt_directory_already_exists_preserved",
                ));
            }
            let _private = Store::open(&path)?;
            let mut invocation = request.clone();
            if attempt.fence > 1
                && matches!(
                    invocation.mode,
                    FixtureMode::Timeout | FixtureMode::NumericalFailure
                )
            {
                invocation.mode = FixtureMode::Succeed;
            }
            new_bytes(
                &path.join("request.json"),
                crate::encode(&invocation)?.as_bytes(),
            )?;
            prepare();
            match OwnedProcess::spawn_fixture(
                executable,
                &path,
                lease,
                row.spec.reserve,
                row.spec.ranks,
                row.spec.threads,
                now,
            ) {
                Err(e) => {
                    let unsupported = matches!(e, RuntimeError::Unsupported(_));
                    diagnostics.push(e.to_string());
                    state = if unsupported {
                        Phase::Unsupported
                    } else {
                        Phase::InfrastructureFailure
                    };
                    outcome = if unsupported {
                        TaskOutcome::Unsupported
                    } else {
                        TaskOutcome::InfrastructureFailure { diagnostic: None }
                    };
                }
                Ok(mut child) => {
                    let native_start = Instant::now();
                    db.record_process(attempt, &child.identity()?, crate::utc_millis()?)?;
                    let mut timed_out = false;
                    let exit = loop {
                        if let Some(exit) = child.poll()? {
                            break exit;
                        }
                        if native_start.elapsed() >= timeout
                            || crate::utc_millis()? >= lease.record().expires_ms
                            || scratch_bytes(&path)? > row.spec.reserve.scratch_bytes
                        {
                            diagnostics.push("owned child reached deadline, lease expiry or scratch budget; no physical failure inferred".into());
                            timed_out = true;
                            break child.cancel(Duration::from_millis(20))?;
                        }
                        std::thread::sleep(Duration::from_millis(2));
                    };
                    cpu_us = exit.cpu_us;
                    peak_ram = exit.peak_ram_bytes;
                    io_bytes = exit.block_io_bytes;
                    scratch = scratch_bytes(&path)?;
                    if timed_out {
                        state = Phase::TimedOut;
                        outcome = TaskOutcome::InfrastructureFailure { diagnostic: None };
                    } else if exit.status.code() == Some(65) {
                        diagnostics.push(
                            "injected numerical failure; not a physical infeasibility label".into(),
                        );
                        state = Phase::NumericalFailure;
                        outcome = TaskOutcome::NumericalFailure { diagnostic: None };
                    } else if exit.status.code() == Some(69) {
                        state = Phase::Unsupported;
                        outcome = TaskOutcome::Unsupported;
                    } else if exit.status.success() {
                        match read_bytes(&path.join("output.json")) {
                            Ok(bytes) => {
                                let input = store.publish(
                                    crate::encode(&invocation)?.as_bytes(),
                                    Metadata::private(ObjectKind::Input),
                                )?;
                                let root = store.publish(
                                    &bytes,
                                    Metadata {
                                        semantic: Some(attempt.evaluation.clone()),
                                        dependencies: BTreeSet::from([input]),
                                        ..Metadata::private(ObjectKind::Observation)
                                    },
                                )?;
                                db.stage(attempt, &root, crate::utc_millis()?)?;
                                match db.verify(attempt, store, crate::utc_millis()?) {
                                    Ok(_) => {
                                        object = Some(root);
                                        state = Phase::Verified;
                                        outcome = TaskOutcome::Succeeded {
                                            output_manifest: object.clone(),
                                        };
                                    }
                                    Err(e) => {
                                        diagnostics.push(e.to_string());
                                        state = Phase::InfrastructureFailure;
                                        outcome =
                                            TaskOutcome::InfrastructureFailure { diagnostic: None };
                                    }
                                }
                            }
                            Err(e) => {
                                diagnostics.push(e.to_string());
                                state = Phase::InfrastructureFailure;
                                outcome = TaskOutcome::InfrastructureFailure { diagnostic: None };
                            }
                        }
                    } else {
                        diagnostics.push(format!(
                            "owned fixture exited {:?}; private stdout/stderr retained",
                            exit.status
                        ));
                        state = Phase::InfrastructureFailure;
                        outcome = TaskOutcome::InfrastructureFailure { diagnostic: None };
                    }
                }
            }
        } else {
            diagnostics.push(
                "injected unsupported software capability; no physical training label".into(),
            );
        }
    } else {
        diagnostics.push(
            "native physical evaluator capability unsupported; no solver or GPU dispatched".into(),
        );
    }
    let diagnostic = if diagnostics.is_empty() {
        None
    } else {
        Some(store.publish(
            crate::encode(&diagnostics)?.as_bytes(),
            Metadata::private(ObjectKind::Checkpoint),
        )?)
    };
    if let Some(diagnostic) = &diagnostic {
        store.pin_root(
            diagnostic.clone(),
            crucible_store::store::PinReason::Checkpoint,
        )?;
        db.note_diagnostic(attempt, diagnostic)?;
        match &mut outcome {
            TaskOutcome::InfrastructureFailure { diagnostic: d }
            | TaskOutcome::NumericalFailure { diagnostic: d } => *d = Some(diagnostic.clone()),
            _ => (),
        }
    }
    let overhead = cpu_thread_us()?
        .checked_sub(cpu_start)
        .ok_or(RuntimeError::Invalid("coordinator_cpu_clock_regressed"))?;
    cpu_us = cpu_us
        .checked_add(overhead)
        .ok_or(RuntimeError::Invalid("attempt_cpu_usage_overflow"))?;
    let charge=Charge {measured:Resources {cpu_ms:cpu_us.div_ceil(1000).max(1),gpu_ms:0,ram_bytes:peak_ram,vram_bytes:0,scratch_bytes:scratch,io_bytes},wall_ns:start.elapsed().as_nanos().try_into().map_err(|_|RuntimeError::Invalid("attempt_wall_time_overflow"))?,accounting_basis:"wait4 child CPU/maxrss/block IO plus coordinator thread CPU through verification; ceil millisecond; coordinator shared memory and final accounting transaction excluded".into()};
    db.charge(attempt, &charge, crate::utc_millis()?)?;
    if state == Phase::Verified {
        match db.commit(attempt, store, crate::utc_millis()?) {
            Ok(id) => {
                object = Some(id);
                state = Phase::Committed;
            }
            Err(RuntimeError::StaleFence | RuntimeError::ExpiredLease) => {
                state = Phase::Quarantined;
                outcome = TaskOutcome::InfrastructureFailure { diagnostic: None };
            }
            Err(e) => return Err(e),
        }
    } else {
        db.fail(attempt, state, crate::utc_millis()?)?;
    }
    Ok(Execution {
        outcome,
        phase: state,
        charge,
        object,
        diagnostic,
    })
}
pub fn fixture_child(request_path: &Path) -> Result<i32> {
    let request: FixtureRequest =
        joil_evidence::identity::strict_decode(&read_bytes(request_path)?)
            .map_err(RuntimeError::Wire)?;
    request.check()?;
    new_bytes(
        Path::new("process-audit.json"),
        crate::encode(&crate::lease::current_process_audit()?)?.as_bytes(),
    )?;
    let deadline: u64 = std::env::var("JOIL_FIXTURE_DEADLINE_MS")
        .map_err(|_| RuntimeError::Invalid("missing_owned_fixture_deadline"))?
        .parse()
        .map_err(|_| RuntimeError::Invalid("fixture_deadline_wire"))?;
    if crate::utc_millis()? >= deadline {
        return Ok(124);
    }
    match request.mode {
        FixtureMode::Unsupported => Ok(69),
        FixtureMode::NumericalFailure => {
            new_bytes(Path::new("diagnostic.json"),br#"{"synthetic_only":true,"kind":"injected_numerical_failure_not_physical_infeasibility"}"#)?;
            Ok(65)
        }
        FixtureMode::Timeout => {
            while crate::utc_millis()? < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(124)
        }
        FixtureMode::Succeed => {
            let identity = Request::SoftwareFixture {
                request: request.clone(),
            }
            .identity()?;
            let observation = FixtureObservation {
                schema_version: 1,
                synthetic_only: true,
                request: identity,
                score: request.synthetic_score()?,
                address: request.address,
            };
            new_bytes(
                Path::new("output.json"),
                crate::encode(&observation)?.as_bytes(),
            )?;
            Ok(0)
        }
    }
}
