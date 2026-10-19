//! Physical topology and explicit current operator authorization; no CPU modulo.
use crate::{ledger::Resources, BlobDigest, Result, RuntimeError};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalCore {
    pub package: u32,
    pub core: u32,
}
#[derive(Clone, Debug)]
pub struct Topology {
    pub cpus: BTreeMap<u32, PhysicalCore>,
    pub siblings: BTreeMap<u32, BTreeSet<u32>>,
    pub allowed: BTreeSet<u32>,
}
pub fn parse_cpu_list(text: &str) -> Result<BTreeSet<u32>> {
    let mut cpus = BTreeSet::new();
    for part in text.trim().split(',') {
        if part.is_empty() {
            return Err(RuntimeError::Invalid("empty_cpu_component"));
        }
        let fields: Vec<_> = part.split('-').collect();
        let first = fields[0]
            .parse::<u32>()
            .map_err(|_| RuntimeError::Invalid("cpu_list"))?;
        let last = if fields.len() == 1 {
            first
        } else if fields.len() == 2 {
            fields[1]
                .parse()
                .map_err(|_| RuntimeError::Invalid("cpu_list"))?
        } else {
            return Err(RuntimeError::Invalid("cpu_list"));
        };
        if first > last || last >= 4096 {
            return Err(RuntimeError::Invalid("cpu_range_budget"));
        }
        for cpu in first..=last {
            if !cpus.insert(cpu) {
                return Err(RuntimeError::Invalid("duplicate_cpu"));
            }
        }
    }
    Ok(cpus)
}
impl Topology {
    pub fn from_sysfs(root: &Path, allowed: BTreeSet<u32>) -> Result<Self> {
        let online = parse_cpu_list(&std::fs::read_to_string(root.join("online"))?)?;
        if allowed.is_empty() || !allowed.is_subset(&online) {
            return Err(RuntimeError::Invalid("allowed_cpuset_not_online"));
        }
        let mut cpus = BTreeMap::new();
        let mut siblings = BTreeMap::new();
        for cpu in &online {
            let path = root.join(format!("cpu{cpu}/topology"));
            let parse = |name| -> Result<u32> {
                std::fs::read_to_string(path.join(name))?
                    .trim()
                    .parse()
                    .map_err(|_| RuntimeError::Invalid("topology_id"))
            };
            cpus.insert(
                *cpu,
                PhysicalCore {
                    package: parse("physical_package_id")?,
                    core: parse("core_id")?,
                },
            );
            siblings.insert(
                *cpu,
                parse_cpu_list(&std::fs::read_to_string(path.join("thread_siblings_list"))?)?,
            );
        }
        for (cpu, group) in &siblings {
            if !group.contains(cpu) {
                return Err(RuntimeError::Invalid("topology_missing_self"));
            }
            for sibling in group.intersection(&online) {
                if cpus[cpu] != cpus[sibling] || siblings[sibling] != *group {
                    return Err(RuntimeError::Invalid("inconsistent_sibling_topology"));
                }
            }
        }
        Ok(Self {
            cpus,
            siblings,
            allowed,
        })
    }
    pub fn system() -> Result<Self> {
        Self::from_sysfs(Path::new("/sys/devices/system/cpu"), current_affinity()?)
    }
    pub fn physical_set(&self, cpus: &[u32]) -> Result<BTreeSet<PhysicalCore>> {
        let unique: BTreeSet<_> = cpus.iter().copied().collect();
        if unique.len() != cpus.len() || unique.is_empty() || !unique.is_subset(&self.allowed) {
            return Err(RuntimeError::Invalid(
                "cpus_outside_current_affinity_or_duplicate",
            ));
        }
        let cores: BTreeSet<_> = cpus
            .iter()
            .map(|cpu| {
                self.cpus
                    .get(cpu)
                    .copied()
                    .ok_or(RuntimeError::Invalid("missing_cpu_topology"))
            })
            .collect::<Result<_>>()?;
        if cores.len() != cpus.len() {
            return Err(RuntimeError::Invalid("smt_siblings_are_one_physical_core"));
        }
        Ok(cores)
    }
}
pub fn current_affinity() -> Result<BTreeSet<u32>> {
    // SAFETY: cpu_set_t is an integer bitset; zero is a valid empty set.
    let mut set: libc::cpu_set_t = unsafe {
        std::ptr::read_unaligned(
            [0u8; std::mem::size_of::<libc::cpu_set_t>()].as_ptr() as *const libc::cpu_set_t
        )
    };
    // SAFETY: set is a live writable cpu_set_t and its exact byte size is supplied; PID 0 selects this thread. Errors are checked before reading the mask.
    if unsafe { libc::sched_getaffinity(0, std::mem::size_of_val(&set), &mut set) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok((0..libc::CPU_SETSIZE)
        // SAFETY: cpu is bounded by CPU_SETSIZE and set is initialized by successful sched_getaffinity.
        .filter(|cpu| unsafe { libc::CPU_ISSET(*cpu as usize, &set) })
        .map(|cpu| cpu as u32)
        .collect())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLease {
    pub id: String,
    pub host: BlobDigest,
    pub cpus: Vec<u32>,
    pub expires_ms: u64,
    pub limits: Resources,
    pub gpu_authorized: bool,
    pub authority: BlobDigest,
}
/// Operator configuration is authenticated outside the worker payload channel.
pub struct Authorization {
    pub host: BlobDigest,
    pub cpus: BTreeSet<u32>,
    pub expires_ms: u64,
    pub limits: Resources,
    pub gpu_authorized: bool,
    pub source: BlobDigest,
}
pub struct AdmittedLease {
    record: ResourceLease,
    cores: BTreeSet<PhysicalCore>,
}
impl AdmittedLease {
    pub fn from_operator_config(
        record: ResourceLease,
        authority: &Authorization,
        topology: &Topology,
        now: u64,
        occupied: &BTreeSet<PhysicalCore>,
    ) -> Result<Self> {
        record.limits.check()?;
        joil_evidence::identity::check_id(&record.id).map_err(RuntimeError::Wire)?;
        if now >= record.expires_ms || record.expires_ms > authority.expires_ms {
            return Err(RuntimeError::ExpiredLease);
        }
        let selected: BTreeSet<_> = record.cpus.iter().copied().collect();
        if record.authority != authority.source
            || record.host != authority.host
            || !selected.is_subset(&authority.cpus)
            || !record.limits.fits(authority.limits)
            || (record.gpu_authorized && !authority.gpu_authorized)
        {
            return Err(RuntimeError::Invalid(
                "lease_outside_operator_authorization",
            ));
        }
        if record.expires_ms > 2_u64.pow(53)
            || record.limits.ram_bytes == 0
            || record.limits.scratch_bytes == 0
            || (!record.gpu_authorized
                && (record.limits.gpu_ms != 0 || record.limits.vram_bytes != 0))
        {
            return Err(RuntimeError::Invalid("lease_resource_vector"));
        }
        let cores = topology.physical_set(&record.cpus)?;
        if !cores.is_disjoint(occupied) {
            return Err(RuntimeError::Conflict("physical_core_already_allocated"));
        }
        Ok(Self { record, cores })
    }
    pub fn record(&self) -> &ResourceLease {
        &self.record
    }
    pub fn cores(&self) -> &BTreeSet<PhysicalCore> {
        &self.cores
    }
    pub fn check_task(&self, reserve: Resources, ranks: u32, threads: u32, now: u64) -> Result<()> {
        if now >= self.record.expires_ms {
            return Err(RuntimeError::ExpiredLease);
        }
        if ranks == 0
            || threads == 0
            || ranks
                .checked_mul(threads)
                .is_none_or(|n| n as usize > self.cores.len())
            || !reserve.fits(self.record.limits)
        {
            return Err(RuntimeError::Budget("task_exceeds_live_lease"));
        }
        Ok(())
    }
}

use std::{
    fs::OpenOptions,
    os::unix::{
        fs::OpenOptionsExt,
        process::{CommandExt, ExitStatusExt},
    },
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};
/// Only an operator-selected, exact-byte fixture executable can be launched.
pub struct FixtureExecutable {
    path: std::path::PathBuf,
    digest: BlobDigest,
}
impl FixtureExecutable {
    pub fn from_operator_binary(path: &Path, expected: &BlobDigest) -> Result<Self> {
        use std::io::Read;
        if !path.is_absolute() {
            return Err(RuntimeError::Invalid(
                "fixture_executable_requires_absolute_path",
            ));
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)?;
        if !file.metadata()?.is_file() || file.metadata()?.len() > 64 * 1024 * 1024 {
            return Err(RuntimeError::Invalid("fixture_executable_size_or_type"));
        }
        let mut bytes = Vec::new();
        file.take(64 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        let digest = BlobDigest::of_bytes(&bytes);
        if &digest != expected {
            return Err(RuntimeError::Conflict("fixture_executable_digest"));
        }
        Ok(Self {
            path: path.to_path_buf(),
            digest,
        })
    }
    pub fn digest(&self) -> &BlobDigest {
        &self.digest
    }
}
#[derive(Clone, Debug)]
pub struct OwnedExit {
    pub status: ExitStatus,
    pub cpu_us: u64,
    pub peak_ram_bytes: u64,
    pub block_io_bytes: u64,
}
pub struct OwnedProcess {
    child: Child,
    group: i32,
    reaped: bool,
    result: Option<OwnedExit>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessIdentity {
    pid: u32,
    group: u32,
    start_ticks: u64,
    boot: BlobDigest,
}
fn process_start(pid: u32) -> Result<Option<u64>> {
    let raw = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let (_, fields) = raw
        .rsplit_once(") ")
        .ok_or(RuntimeError::Invalid("proc_stat_format"))?;
    let value = fields
        .split_whitespace()
        .nth(19)
        .ok_or(RuntimeError::Invalid("proc_stat_start_field"))?
        .parse()
        .map_err(|_| RuntimeError::Invalid("proc_start_ticks"))?;
    Ok(Some(value))
}
fn boot_identity() -> Result<BlobDigest> {
    Ok(BlobDigest::of_bytes(&std::fs::read(
        "/proc/sys/kernel/random/boot_id",
    )?))
}
impl ProcessIdentity {
    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub fn still_matches(&self) -> Result<bool> {
        Ok(boot_identity()? == self.boot
            && process_start(self.pid)? == Some(self.start_ticks)
            // SAFETY: getpgid takes only an integer PID and no pointers; failure returns -1 and fails the identity comparison.
            && unsafe { libc::getpgid(self.pid as i32) } == self.group as i32)
    }
    /// Recovery supports the single-process fixture capability only. pidfds
    /// prevent a reused numeric PID from receiving a signal. Arbitrary native
    /// trees require a separately owned cgroup capability, which is unsupported.
    pub fn cancel_recorded_fixture(&self, grace: Duration) -> Result<bool> {
        if grace > Duration::from_secs(2) {
            return Err(RuntimeError::Invalid("recorded_cancel_grace"));
        }
        if process_start(self.pid)?.is_none() {
            return Ok(true);
        }
        if self.pid != self.group || self.pid > i32::MAX as u32 {
            return Err(RuntimeError::Conflict("recorded_process_group_identity"));
        }
        // SAFETY: pidfd_open receives a range-checked process identity and zero flags, no pointers. Negative returns are rejected before ownership transfer.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, self.pid, 0) };
        if fd < 0 {
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() == Some(libc::ESRCH) {
                return Ok(true);
            }
            return Err(e.into());
        }
        use std::os::fd::{AsRawFd, FromRawFd};
        // SAFETY: successful pidfd_open returned a fresh owned descriptor, representable as a Linux int; File takes sole ownership and closes it on every return path.
        let handle = unsafe { std::fs::File::from_raw_fd(fd as i32) };
        if !self.still_matches()? {
            return Err(RuntimeError::Conflict(
                "recorded_process_was_replaced_no_signal",
            ));
        }
        let signal = |number: i32| -> Result<()> {
            // SAFETY: handle keeps the pidfd live; null siginfo and zero flags request a standard signal. Only SIGTERM/SIGKILL are passed; errors except already-exited ESRCH propagate.
            let result = unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    handle.as_raw_fd(),
                    number,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                )
            };
            if result < 0 {
                let e = std::io::Error::last_os_error();
                if e.raw_os_error() != Some(libc::ESRCH) {
                    return Err(e.into());
                }
            }
            Ok(())
        };
        let exited = |duration: Duration| -> Result<bool> {
            let start = Instant::now();
            loop {
                let mut descriptor = libc::pollfd {
                    fd: handle.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };
                let left = duration
                    .saturating_sub(start.elapsed())
                    .as_millis()
                    .min(i32::MAX as u128) as i32;
                // SAFETY: descriptor is a live writable pollfd, count is exactly one and timeout is bounded to i32. The owned pidfd remains open; EINTR retries within the deadline.
                let result = unsafe { libc::poll(&mut descriptor, 1, left) };
                if result > 0 {
                    return Ok(descriptor.revents & libc::POLLIN != 0);
                }
                if result == 0 {
                    return Ok(false);
                }
                let e = std::io::Error::last_os_error();
                if e.kind() != std::io::ErrorKind::Interrupted {
                    return Err(e.into());
                }
                if start.elapsed() >= duration {
                    return Ok(false);
                }
            }
        };
        signal(libc::SIGTERM)?;
        if exited(grace)? {
            return Ok(true);
        }
        signal(libc::SIGKILL)?;
        if exited(Duration::from_secs(2))? {
            Ok(true)
        } else {
            Err(RuntimeError::Unsupported(
                "recorded_fixture_not_reaped_reservation_remains",
            ))
        }
    }
}
impl OwnedProcess {
    pub fn spawn_fixture(
        executable: &FixtureExecutable,
        directory: &Path,
        lease: &AdmittedLease,
        reserve: Resources,
        ranks: u32,
        threads: u32,
        now: u64,
    ) -> Result<Self> {
        let actual_now = crate::utc_millis()?.max(now);
        lease.check_task(reserve, ranks, threads, actual_now)?;
        if ranks != 1 || threads != 1 {
            return Err(RuntimeError::Unsupported(
                "fixture_process_is_single_thread_no_mpi",
            ));
        }
        if reserve.cpu_ms == 0
            || reserve.ram_bytes < 32 * 1024 * 1024
            || reserve.scratch_bytes < 4096
            || reserve.gpu_ms != 0
            || reserve.vram_bytes != 0
        {
            return Err(RuntimeError::Budget("bounded_cpu_only_child_resources"));
        }
        // Recheck immediately before exec. The operator owns the executable and
        // private directory; arbitrary worker-selected command lines do not exist.
        FixtureExecutable::from_operator_binary(&executable.path, &executable.digest)?;
        let stdout = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(directory.join("stdout.log"))?;
        let stderr = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(directory.join("stderr.log"))?;
        // SAFETY: cpu_set_t is an integer bitset whose all-zero representation is a valid empty affinity mask.
        let mut affinity: libc::cpu_set_t = unsafe {
            std::ptr::read_unaligned(
                [0u8; std::mem::size_of::<libc::cpu_set_t>()].as_ptr() as *const libc::cpu_set_t
            )
        };
        for cpu in &lease.record.cpus {
            if *cpu >= libc::CPU_SETSIZE as u32 {
                return Err(RuntimeError::Unsupported(
                    "cpu_exceeds_native_affinity_mask",
                ));
            }
            // SAFETY: cpu was checked below CPU_SETSIZE and affinity is a live initialized, exclusively borrowed mask.
            unsafe { libc::CPU_SET(*cpu as usize, &mut affinity) };
        }
        let cpu_seconds = reserve
            .cpu_ms
            .div_ceil(1000)
            .checked_add(1)
            .ok_or(RuntimeError::Invalid("cpu_limit_overflow"))?;
        let mut command = Command::new(&executable.path);
        command
            .args(["--fixture-child", "request.json"])
            .current_dir(directory)
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr);
        for name in [
            "OMP_NUM_THREADS",
            "OPENBLAS_NUM_THREADS",
            "MKL_NUM_THREADS",
            "RAYON_NUM_THREADS",
            "VECLIB_MAXIMUM_THREADS",
            "NUMEXPR_NUM_THREADS",
            "BLIS_NUM_THREADS",
        ] {
            command.env(name, "1");
        }
        command
            .env("OMP_MAX_ACTIVE_LEVELS", "1")
            .env("CUDA_VISIBLE_DEVICES", "");
        command.env(
            "JOIL_FIXTURE_DEADLINE_MS",
            actual_now
                .checked_add(30_000)
                .ok_or(RuntimeError::Invalid("fixture_clock_overflow"))?
                .min(lease.record.expires_ms)
                .to_string(),
        );
        // SAFETY: the closure captures initialized values and uses only stack data and async-signal-safe libc calls after fork; no allocation, locks or panics. Affinity/rlimit pointers live through each call; failures abort exec via io::Error.
        unsafe {
            command.pre_exec(move || {
                if libc::sched_setaffinity(0, std::mem::size_of_val(&affinity), &affinity) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                for (resource, limit) in [
                    (libc::RLIMIT_CPU, cpu_seconds),
                    (libc::RLIMIT_AS, reserve.ram_bytes),
                    (libc::RLIMIT_FSIZE, reserve.scratch_bytes),
                    (libc::RLIMIT_CORE, 0),
                    (libc::RLIMIT_NOFILE, 64),
                ] {
                    let limit = libc::rlimit {
                        rlim_cur: limit,
                        rlim_max: limit,
                    };
                    if libc::setrlimit(resource, &limit) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
        let child = command.spawn()?;
        let group = child
            .id()
            .try_into()
            .map_err(|_| RuntimeError::Invalid("owned_process_id"))?;
        Ok(Self {
            child,
            group,
            reaped: false,
            result: None,
        })
    }
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
    pub fn identity(&self) -> Result<ProcessIdentity> {
        if self.reaped {
            return Err(RuntimeError::Invalid("cannot_record_reaped_process"));
        }
        Ok(ProcessIdentity {
            pid: self.child.id(),
            group: self.group as u32,
            start_ticks: process_start(self.child.id())?
                .ok_or(RuntimeError::Invalid("owned_process_disappeared"))?,
            boot: boot_identity()?,
        })
    }
    pub fn poll(&mut self) -> Result<Option<OwnedExit>> {
        if let Some(result) = &self.result {
            return Ok(Some(result.clone()));
        }
        if self.reaped {
            return Err(RuntimeError::Invalid(
                "owned_usage_decode_failed_after_reap",
            ));
        }
        let mut status = 0;
        // SAFETY: rusage contains only C integer/timeval fields; zero initializes a valid writable output buffer.
        let mut usage: libc::rusage = unsafe {
            std::ptr::read_unaligned(
                [0u8; std::mem::size_of::<libc::rusage>()].as_ptr() as *const libc::rusage
            )
        };
        // SAFETY: group is the owned unreaped child PID; status and usage are live exclusive output buffers. WNOHANG returns are checked before decoding, and reaped is set before fallible decoding.
        let waited = unsafe { libc::wait4(self.group, &mut status, libc::WNOHANG, &mut usage) };
        if waited == 0 {
            return Ok(None);
        }
        if waited < 0 {
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::Interrupted {
                return Ok(None);
            }
            return Err(e.into());
        }
        self.reaped = true;
        let micros = |v: libc::timeval| -> Result<u64> {
            let sec: u64 = v
                .tv_sec
                .try_into()
                .map_err(|_| RuntimeError::Invalid("negative_cpu_time"))?;
            let sub: u64 = v
                .tv_usec
                .try_into()
                .map_err(|_| RuntimeError::Invalid("negative_cpu_time"))?;
            if sub >= 1_000_000 {
                return Err(RuntimeError::Invalid("cpu_time_fraction"));
            }
            sec.checked_mul(1_000_000)
                .and_then(|v| v.checked_add(sub))
                .ok_or(RuntimeError::Invalid("cpu_time_overflow"))
        };
        let cpu_us = micros(usage.ru_utime)?
            .checked_add(micros(usage.ru_stime)?)
            .ok_or(RuntimeError::Invalid("cpu_usage_overflow"))?;
        let peak: u64 = usage
            .ru_maxrss
            .try_into()
            .map_err(|_| RuntimeError::Invalid("negative_peak_ram"))?;
        let blocks: i64 = usage
            .ru_inblock
            .checked_add(usage.ru_oublock)
            .ok_or(RuntimeError::Invalid("io_usage_overflow"))?;
        let blocks: u64 = blocks
            .try_into()
            .map_err(|_| RuntimeError::Invalid("negative_block_io"))?;
        let result = OwnedExit {
            status: ExitStatus::from_raw(status),
            cpu_us,
            peak_ram_bytes: peak
                .checked_mul(1024)
                .ok_or(RuntimeError::Invalid("peak_ram_overflow"))?,
            block_io_bytes: blocks
                .checked_mul(512)
                .ok_or(RuntimeError::Invalid("io_byte_overflow"))?,
        };
        self.result = Some(result.clone());
        Ok(Some(result))
    }
    pub fn cancel(&mut self, grace: Duration) -> Result<OwnedExit> {
        if let Some(exit) = self.poll()? {
            return Ok(exit);
        }
        self.signal_owned(libc::SIGTERM)?;
        let start = Instant::now();
        while start.elapsed() < grace {
            if let Some(exit) = self.poll()? {
                return Ok(exit);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        self.signal_owned(libc::SIGKILL)?;
        let killed = Instant::now();
        while killed.elapsed() < Duration::from_secs(2) {
            if let Some(exit) = self.poll()? {
                return Ok(exit);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        Err(RuntimeError::Unsupported(
            "owned_child_unreaped_after_kill_deadline_reservation_remains",
        ))
    }
    fn signal_owned(&self, signal: i32) -> Result<()> {
        if self.reaped {
            return Ok(());
        }
        // SAFETY: group is the owned child PID and getpgid accepts an integer without pointers; -1 or a changed group fails closed.
        if unsafe { libc::getpgid(self.group) } != self.group {
            return Err(RuntimeError::Conflict(
                "owned_process_group_identity_changed",
            ));
        }
        // SAFETY: this child is unreaped and its group identity was checked. Negating its positive PID targets only the group created by process_group(0); callers pass SIGTERM/SIGKILL.
        if unsafe { libc::kill(-self.group, signal) } != 0 {
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() != Some(libc::ESRCH) {
                return Err(e.into());
            }
        }
        Ok(())
    }
}
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        if !self.reaped && self.signal_owned(libc::SIGKILL).is_ok() {
            let mut status = 0;
            // SAFETY: group identifies our unreaped child and status is a live writable int. WNOHANG avoids blocking Drop; this is best-effort cleanup, not proof of reaping.
            unsafe { libc::waitpid(self.group, &mut status, libc::WNOHANG) };
        }
    }
}
pub fn scratch_bytes(path: &Path) -> Result<u64> {
    fn walk(path: &Path, count: &mut usize) -> Result<u64> {
        *count += 1;
        if *count > 4096 {
            return Err(RuntimeError::Budget("scratch_file_count"));
        }
        let meta = std::fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            return Err(RuntimeError::Invalid("scratch_symlink"));
        }
        if meta.is_file() {
            return Ok(meta.len());
        }
        if !meta.is_dir() {
            return Err(RuntimeError::Invalid("scratch_file_type"));
        }
        let mut size = 0_u64;
        for child in std::fs::read_dir(path)? {
            size = size
                .checked_add(walk(&child?.path(), count)?)
                .ok_or(RuntimeError::Invalid("scratch_size_overflow"))?;
        }
        Ok(size)
    }
    walk(path, &mut 0)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessAudit {
    pub schema_version: u32,
    pub synthetic_only: bool,
    pub cpus: BTreeSet<u32>,
    pub address_space_limit: u64,
    pub file_size_limit: u64,
    pub cpu_seconds_limit: u64,
    pub owns_process_group: bool,
    pub numerical_threads: BTreeMap<String, String>,
    pub cuda_visible_devices: String,
}
pub fn current_process_audit() -> Result<ProcessAudit> {
    let limit = |resource| -> Result<u64> {
        // SAFETY: rlimit contains integer fields and zero is a valid initialized output buffer.
        let mut limits: libc::rlimit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: resource is a supported RLIMIT constant and limits is a live exclusive output pointer. Failure returns before the output is read.
        if unsafe { libc::getrlimit(resource, &mut limits) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(limits.rlim_cur)
    };
    let numerical_threads = [
        "OMP_NUM_THREADS",
        "OPENBLAS_NUM_THREADS",
        "MKL_NUM_THREADS",
        "RAYON_NUM_THREADS",
        "VECLIB_MAXIMUM_THREADS",
        "NUMEXPR_NUM_THREADS",
        "BLIS_NUM_THREADS",
    ]
    .into_iter()
    .map(|n| (n.into(), std::env::var(n).unwrap_or_default()))
    .collect();
    Ok(ProcessAudit {
        schema_version: 1,
        synthetic_only: true,
        cpus: current_affinity()?,
        address_space_limit: limit(libc::RLIMIT_AS)?,
        file_size_limit: limit(libc::RLIMIT_FSIZE)?,
        cpu_seconds_limit: limit(libc::RLIMIT_CPU)?,
        // SAFETY: getpgrp/getpid have no pointer arguments and only query the calling process identity.
        owns_process_group: unsafe { libc::getpgrp() == libc::getpid() },
        numerical_threads,
        cuda_visible_devices: std::env::var("CUDA_VISIBLE_DEVICES").unwrap_or_default(),
    })
}
