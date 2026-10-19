//! Explicit local software fixture lifecycle. No installer, fleet or physical evaluator.
pub mod explore;
#[cfg(test)]
#[allow(dead_code)]
mod ordered_json;
#[cfg(test)]
#[allow(dead_code)]
mod port_support;
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
    // SAFETY: interrupt has the C signal-handler ABI and static lifetime. It only
    // stores to a lock-free AtomicBool; it neither allocates nor acquires locks.
    // SIGINT and SIGTERM are valid catchable signals; no pointers are dereferenced.
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
        // SAFETY: f owns a live descriptor throughout this call; flock takes no
        // pointers, and these are valid exclusive/nonblocking operation flags.
        // Failure returns before constructing WriterLock, so f is closed by RAII.
        if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(RuntimeError::Conflict("campaign_already_has_writer"));
        }
        Ok(Self(f))
    }
}
impl Drop for WriterLock {
    fn drop(&mut self) {
        // SAFETY: self.0 still owns the descriptor until Drop returns. LOCK_UN
        // releases only this open file description's lock. File drop closes it
        // even if explicit unlock fails, releasing the kernel lock on final close.
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

pub mod runner;
pub use runner::*;
