//! Explicit local software fixture lifecycle. No installer, fleet or physical evaluator.
use crucible_runtime::{
    checkpoint::quarantine_staging, db::RuntimeDb, lease::*, ledger::Resources, replay::*, task::*,
    BlobDigest, Result, RuntimeError, SemanticId,
};
use crucible_store::{
    manifest::{Metadata, ObjectKind},
    store::Limits,
    Store,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};

static STOP: AtomicBool = AtomicBool::new(false);
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
extern "C" fn interrupt(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}
pub fn install_interrupt() {
    unsafe {
        libc::signal(libc::SIGINT, interrupt as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, interrupt as *const () as libc::sighandler_t);
    }
}
pub fn total_budget() -> Resources {
    Resources {
        cpu_ms: 120_000,
        gpu_ms: 0,
        ram_bytes: 512 * 1024 * 1024,
        vram_bytes: 0,
        scratch_bytes: 64 * 1024 * 1024,
        io_bytes: 128 * 1024 * 1024,
    }
}
pub fn job_budget() -> Resources {
    Resources {
        cpu_ms: 2000,
        gpu_ms: 0,
        ram_bytes: 256 * 1024 * 1024,
        vram_bytes: 0,
        scratch_bytes: 8 * 1024 * 1024,
        io_bytes: 4 * 1024 * 1024,
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Campaign {
    pub schema_version: u32,
    pub domain: String,
    pub epochs: u64,
    pub workers: usize,
    pub seeds: Vec<[i64; 2]>,
    pub weights: [i64; 2],
    pub mode: FixtureMode,
    pub timeout_ms: u64,
}
impl Campaign {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = read_private_or_source(path, 16_384)?;
        let c: Self = toml::from_str(
            std::str::from_utf8(&raw).map_err(|_| RuntimeError::Invalid("campaign_utf8"))?,
        )
        .map_err(|e| RuntimeError::Wire(e.to_string()))?;
        c.check()?;
        Ok(c)
    }
    pub fn check(&self) -> Result<()> {
        if self.schema_version != 1
            || self.domain != "synthetic_integer_fixture"
            || !(1..=8).contains(&self.epochs)
            || !(1..=2).contains(&self.workers)
            || self.seeds.is_empty()
            || self.seeds.len() > 2
            || !(5..=1000).contains(&self.timeout_ms)
            || self.mode == FixtureMode::Unsupported
        {
            return Err(RuntimeError::Invalid("bounded_software_campaign_admission"));
        }
        for v in &self.seeds {
            ToyGenome::compile(*v)?;
        }
        DecisionLineage::new(
            &SemanticId::of(&"admission").map_err(|e| RuntimeError::Wire(e.to_string()))?,
            None,
            self.weights,
        )?;
        Ok(())
    }
    fn immutable(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&("synthetic_integer_fixture/1", self.epochs, &self.seeds))
            .map_err(|e| RuntimeError::Wire(e.to_string()))
    }
}
pub fn open_store(path: &Path) -> Result<Store> {
    Ok(Store::with_limits(
        path,
        Limits {
            object_bytes: 64 * 1024 * 1024,
            closure_bytes: 128 * 1024 * 1024,
            closure_objects: 4096,
        },
    )?)
}
pub fn read_private_or_source(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    if !f.metadata()?.is_file() || f.metadata()?.len() > limit {
        return Err(RuntimeError::Invalid("input_size_or_type"));
    }
    let mut b = Vec::new();
    f.take(limit + 1).read_to_end(&mut b)?;
    if b.len() as u64 > limit {
        return Err(RuntimeError::Invalid("input_size"));
    }
    Ok(b)
}
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    if let Some(p) = path.parent() {
        File::open(p)?.sync_all()?;
    }
    Ok(())
}
fn wire(v: &impl Serialize) -> Result<Vec<u8>> {
    serde_json::to_vec(v).map_err(|e| RuntimeError::Wire(e.to_string()))
}
pub fn private_root(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if !absolute
        .components()
        .any(|c| c == Component::Normal(".private".as_ref()))
        || absolute
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(RuntimeError::Invalid(
            "run_root_must_be_normal_private_path",
        ));
    }
    let mut prefix = PathBuf::new();
    for c in absolute.components() {
        prefix.push(c);
        if std::fs::symlink_metadata(&prefix).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(RuntimeError::Invalid("private_path_symlink"));
        }
    }
    Ok(absolute)
}
fn unique() -> Result<String> {
    Ok(format!(
        "{}-{}-{}",
        crucible_runtime::utc_millis()?,
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}
pub fn default_root() -> Result<PathBuf> {
    let parent = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?.join(".private"));
    private_root(&parent.join("fixture-v5-runs").join(unique()?))
}
pub struct WriterLock(File);
impl WriterLock {
    pub fn acquire(root: &Path) -> Result<Self> {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(root.join("run.lock"))?;
        if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(RuntimeError::Conflict("campaign_already_has_writer"));
        }
        Ok(Self(f))
    }
}
impl Drop for WriterLock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunMetadata {
    pub schema_version: u32,
    pub synthetic_only: bool,
    pub experiment: Experiment,
    pub authorized: Resources,
    pub executable: BlobDigest,
}
pub fn metadata(root: &Path) -> Result<RunMetadata> {
    let m: RunMetadata = joil_evidence::identity::strict_decode(&read_private_or_source(
        &root.join("run.json"),
        65_536,
    )?)
    .map_err(RuntimeError::Wire)?;
    m.experiment.check()?;
    if m.schema_version != 1 || !m.synthetic_only || m.authorized != total_budget() {
        return Err(RuntimeError::Invalid("run_metadata_contract"));
    }
    Ok(m)
}
fn freeze(store: &Store, campaign: &Campaign, exe: &[u8]) -> Result<Experiment> {
    let mut dependencies = BTreeMap::new();
    for (name, bytes) in [
        (
            "solver",
            b"synthetic integer sum-of-squares/1; no physical capability".as_slice(),
        ),
        ("material", b"synthetic integers; no physical material"),
        ("interface", b"synthetic two bounded integers/1"),
        (
            "requirements",
            b"synthetic score arithmetic/1; physical eligibility Unsupported",
        ),
        (
            "recipe",
            b"fixture counter RNG/1; covariance correlated integer/1; due epoch ordinal/1",
        ),
        ("config", &campaign.immutable()?),
        ("executable", exe),
    ] {
        dependencies.insert(
            name.into(),
            store.publish(bytes, Metadata::private(ObjectKind::Input))?,
        );
    }
    let e = Experiment { dependencies };
    e.check()?;
    Ok(e)
}
fn cp(
    db: &mut RuntimeDb,
    store: &Store,
    experiment: &Experiment,
    state: &SearchState,
) -> Result<crucible_runtime::checkpoint::CheckpointPointer> {
    let generation = db
        .checkpoint_history()?
        .first()
        .map_or(1, |p| p.generation + 1);
    db.publish_checkpoint_checked(store, experiment, state, |phase| {
        // Explicit deterministic private crash-test barrier; no default production behavior.
        if std::env::var("JOIL_TEST_CHECKPOINT_PHASE").ok().as_deref()
            == Some(&format!("{phase:?}"))
            && std::env::var("JOIL_TEST_CHECKPOINT_GENERATION")
                .ok()
                .and_then(|n| n.parse::<u64>().ok())
                == Some(generation)
        {
            let marker = store.path().join("checkpoint-test-barrier.json");
            let _ = write_new(
                &marker,
                format!("{{\"phase\":\"{phase:?}\",\"generation\":{generation}}}").as_bytes(),
            );
            loop {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    })
}
fn cpus(workers: usize) -> Result<Vec<u32>> {
    let t = Topology::system()?;
    let mut physical = BTreeSet::new();
    let cpus = t
        .allowed
        .iter()
        .filter(|c| physical.insert(t.cpus[c]))
        .copied()
        .take(workers)
        .collect::<Vec<_>>();
    if cpus.len() != workers {
        return Err(RuntimeError::Budget(
            "fixture_workers_exceed_inherited_physical_cores",
        ));
    }
    Ok(cpus)
}
fn execute_one(
    root: &Path,
    meta: &RunMetadata,
    task: &PlannedTask,
    cpu: u32,
    exe_path: &Path,
    timeout_ms: u64,
) -> Result<()> {
    let store = open_store(root)?;
    let mut db = RuntimeDb::open(root, meta.authorized, &meta.experiment.identity()?)?;
    let executable = FixtureExecutable::from_operator_binary(exe_path, &meta.executable)?;
    for _ in 0..2 {
        let row = db.task(&task.id)?;
        if row.phase == Phase::Committed {
            return Ok(());
        }
        if row.phase != Phase::Ready {
            return Err(RuntimeError::Conflict(
                "task_not_reconciled_before_dispatch",
            ));
        }
        let now = crucible_runtime::utc_millis()?;
        let source = BlobDigest::of_bytes(
            b"explicit bounded local fixture invocation; inherited operator affinity",
        );
        let host = BlobDigest::of_bytes(&std::fs::read("/proc/sys/kernel/random/boot_id")?);
        let expiry = now + 30_000;
        let authority = Authorization {
            host: host.clone(),
            cpus: BTreeSet::from([cpu]),
            expires_ms: expiry,
            limits: job_budget(),
            gpu_authorized: false,
            source: source.clone(),
        };
        let next = row.fence + 1;
        let lease_name = format!(
            "lease-{}-{next}",
            BlobDigest::of_bytes(task.id.as_bytes()).as_str()
        );
        let lease = AdmittedLease::from_operator_config(
            ResourceLease {
                id: lease_name,
                host,
                cpus: vec![cpu],
                expires_ms: expiry,
                limits: job_budget(),
                gpu_authorized: false,
                authority: source,
            },
            &authority,
            &Topology::system()?,
            now,
            &BTreeSet::new(),
        )?;
        let attempt = db.lease(
            &task.id,
            &format!(
                "attempt-{}-{next}",
                BlobDigest::of_bytes(task.id.as_bytes()).as_str()
            ),
            &lease,
            now,
        )?;
        let execution = execute_fixture(
            &mut db,
            &store,
            &attempt,
            &lease,
            &executable,
            Duration::from_millis(timeout_ms),
        )?;
        if execution.phase == Phase::Committed {
            return Ok(());
        }
        if !matches!(
            execution.phase,
            Phase::NumericalFailure | Phase::TimedOut | Phase::InfrastructureFailure
        ) {
            return Err(RuntimeError::Unsupported(
                "fixture_execution_did_not_produce_observation",
            ));
        }
        db.retry_ready(&task.id)?;
    }
    Err(RuntimeError::Conflict("fixture_retry_limit"))
}
#[derive(Clone, Debug, Serialize)]
pub struct RunResult {
    pub root: PathBuf,
    pub synthetic_only: bool,
    pub physical_eligibility: String,
    pub epochs: u64,
    pub checkpoint: BlobDigest,
    pub logical_digest: SemanticId,
    pub actual_cost_digest: SemanticId,
    pub lineage: SemanticId,
    pub accounting: crucible_runtime::ledger::Accounting,
    pub stopped: bool,
    pub fallback_reasons: Vec<(BlobDigest, String)>,
}
pub fn run(
    campaign: &Campaign,
    root: &Path,
    resume: bool,
    until: u64,
    continue_stop: bool,
) -> Result<RunResult> {
    campaign.check()?;
    if until == 0 || until > campaign.epochs {
        return Err(RuntimeError::Invalid("until_within_campaign"));
    }
    let selected = cpus(campaign.workers)?;
    let root = private_root(root)?;
    if resume {
        metadata(&root)?;
        if !root.join("runtime.sqlite").is_file() {
            return Err(RuntimeError::Conflict(
                "missing_canonical_ledger_refuse_budget_reset",
            ));
        }
    } else if root.exists() {
        return Err(RuntimeError::Conflict("fresh_run_root_already_exists"));
    }
    let store = open_store(&root)?;
    let _writer = WriterLock::acquire(&root)?;
    let exe_path = std::env::current_exe()?;
    let exe = read_private_or_source(&exe_path, 64 * 1024 * 1024)?;
    let expected = BlobDigest::of_bytes(&exe);
    let experiment = freeze(&store, campaign, &exe)?;
    let meta = if resume {
        let m = metadata(&root)?;
        m.experiment.require_resume(&experiment)?;
        if m.executable != expected {
            return Err(RuntimeError::Conflict("frozen_executable_drift"));
        }
        m
    } else {
        let m = RunMetadata {
            schema_version: 1,
            synthetic_only: true,
            experiment,
            authorized: total_budget(),
            executable: expected,
        };
        write_new(&root.join("run.json"), &wire(&m)?)?;
        m
    };
    let mut db = RuntimeDb::open(&root, meta.authorized, &meta.experiment.identity()?)?;
    let (mut state, rejected) = if resume {
        let r = db.restore_latest(&store, &meta.experiment)?;
        db.quarantine_checkpoint_pointers(&r.rejected_newer)?;
        quarantine_staging(&store)?;
        (r.checkpoint.state, r.rejected_newer)
    } else {
        (
            SearchState::new(
                meta.experiment.identity()?,
                campaign.seeds.clone(),
                campaign.weights,
            )?,
            Vec::new(),
        )
    };
    if continue_stop {
        if root.join("stop.request").exists() {
            std::fs::rename(
                root.join("stop.request"),
                root.join(format!("stop-consumed-{}", unique()?)),
            )?;
        }
        if matches!(
            state.stop,
            StopState::Requested { .. } | StopState::Completed
        ) {
            state.stop = StopState::Running;
        }
    }
    if resume && state.stop == StopState::Completed && state.completed_epochs < until {
        state.stop = StopState::Running;
    }
    if !resume {
        cp(&mut db, &store, &meta.experiment, &state)?;
    }
    let pointer;
    loop {
        db.reconcile(&store, &mut state)?;
        if STOP.load(Ordering::Relaxed) || root.join("stop.request").exists() {
            state.stop = StopState::Requested {
                reason: "explicit signal or stop command; actual charged batch retained".into(),
            };
        }
        if matches!(state.stop, StopState::Requested { .. }) {
            pointer = cp(&mut db, &store, &meta.experiment, &state)?;
            break;
        }
        if state.in_flight.is_empty() && !state.pending_results.is_empty() {
            state.apply_epoch()?;
        }
        if state.completed_epochs >= until {
            state.stop = if until == campaign.epochs {
                StopState::Completed
            } else {
                StopState::Running
            };
            pointer = cp(&mut db, &store, &meta.experiment, &state)?;
            break;
        }
        if state.in_flight.is_empty() {
            state.prepare_epoch(&meta.executable, job_budget(), campaign.mode.clone())?;
            cp(&mut db, &store, &meta.experiment, &state)?;
            db.reconcile(&store, &mut state)?;
        }
        let tasks = state.in_flight.values().cloned().collect::<Vec<_>>();
        let mut arrivals = Vec::new();
        for batch in tasks.chunks(campaign.workers) {
            if STOP.load(Ordering::Relaxed) || root.join("stop.request").exists() {
                break;
            }
            let (send, receive) = std::sync::mpsc::channel();
            std::thread::scope(|scope| {
                for (i, task) in batch.iter().enumerate() {
                    let send = send.clone();
                    let root = &root;
                    let meta = &meta;
                    let exe_path = &exe_path;
                    let cpu = selected[i];
                    scope.spawn(move || {
                        let r = execute_one(root, meta, task, cpu, exe_path, campaign.timeout_ms);
                        let _ = send.send((task.id.clone(), r));
                    });
                }
            });
            drop(send);
            for (name, result) in receive {
                result?;
                arrivals.push(name);
            }
            // Threads are joined before any publication; native costs are known at these checkpoints.
        }
        for name in arrivals {
            let row = db.task(&name)?;
            let output = row
                .accepted_object
                .ok_or(RuntimeError::Invalid("accepted_fixture_output_absent"))?;
            let observed: FixtureObservation =
                joil_evidence::identity::strict_decode(&store.read(&output)?.payload)
                    .map_err(RuntimeError::Wire)?;
            state.collect(&name, output, observed)?;
        }
        // The joined batch already has durable, charged canonical results. STOP
        // needs its complete pending snapshot now. Normal complete epochs can use
        // their next boundary; recovery still verifies each original acceptance
        // when reconciling canonical results after interrupted publication.
        if STOP.load(Ordering::Relaxed) || root.join("stop.request").exists() {
            state.stop = StopState::Requested {
                reason: "explicit signal or stop command; actual charged batch retained".into(),
            };
            pointer = cp(&mut db, &store, &meta.experiment, &state)?;
            break;
        }
        // A complete normal epoch is folded on the next iteration, which then
        // publishes either the next prepared epoch or the terminal state. The
        // canonical ledger already durably owns every accepted result/charge;
        // reconciliation rehydrates them if we crash before that publication.
        // Keep partial and STOP snapshots, including their nonempty pending state.
        if !state.in_flight.is_empty() {
            cp(&mut db, &store, &meta.experiment, &state)?;
        }
    }
    let ledger = db.ledger_snapshot()?;
    Ok(RunResult {
        root,
        synthetic_only: true,
        physical_eligibility: "Unsupported: software fixture only".into(),
        epochs: state.completed_epochs,
        checkpoint: pointer.object,
        logical_digest: state.logical_digest()?,
        actual_cost_digest: ledger.actual_cost_digest()?,
        lineage: state.lineage.id,
        accounting: ledger.accounting,
        stopped: matches!(state.stop, StopState::Requested { .. }),
        fallback_reasons: rejected,
    })
}
pub fn status(root: &Path) -> Result<serde_json::Value> {
    let root = private_root(root)?;
    let meta = metadata(&root)?;
    let mut db = RuntimeDb::inspect_existing(&root)?;
    let history = db.checkpoint_history()?;
    Ok(
        serde_json::json!({"synthetic_only":true,"physical_eligibility":"Unsupported: software fixture only","experiment":meta.experiment.identity()?,"history":history,"accounting":db.accounting()?,"event_position":db.event_position()?}),
    )
}
pub fn fork(root: &Path, weights: [i64; 2]) -> Result<serde_json::Value> {
    let root = private_root(root)?;
    let meta = metadata(&root)?;
    let _writer = WriterLock::acquire(&root)?;
    let store = open_store(&root)?;
    let mut db = RuntimeDb::open(&root, meta.authorized, &meta.experiment.identity()?)?;
    let r = db.restore_latest(&store, &meta.experiment)?;
    let mut state = r.checkpoint.state;
    db.reconcile(&store, &mut state)?;
    if state.in_flight.is_empty() && !state.pending_results.is_empty() {
        state.apply_epoch()?;
    }
    let next = state.fork(weights)?;
    let before = db.accounting()?;
    let pointer = cp(&mut db, &store, &meta.experiment, &next)?;
    let result = serde_json::json!({"synthetic_only":true,"lineage":next.lineage,"checkpoint":pointer,"shared_canonical_ledger":true,"observations_reused":next.archives.observations.len(),"accounting":before});
    write_new(
        &root.join(format!("lineage-{}.json", next.lineage.id.as_str())),
        &wire(&result)?,
    )?;
    Ok(result)
}
pub fn verify(root: &Path) -> Result<serde_json::Value> {
    let root = private_root(root)?;
    let meta = metadata(&root)?;
    let store = open_store(&root)?;
    let mut db = RuntimeDb::inspect_existing(&root)?;
    let restored = db.restore_latest(&store, &meta.experiment)?;
    Ok(
        serde_json::json!({"synthetic_only":true,"offline":true,"checkpoint":restored.pointer,"logical_digest":restored.checkpoint.state.logical_digest()?,"rejected_newer":restored.rejected_newer,"physical_eligibility":"Unsupported"}),
    )
}
pub fn stop(root: &Path) -> Result<()> {
    let root = private_root(root)?;
    metadata(&root)?;
    let path = root.join("stop.request");
    if path.exists() {
        return Ok(());
    }
    write_new(
        &path,
        b"explicit operator stop; retain prior checkpoints and charges\n",
    )
}
