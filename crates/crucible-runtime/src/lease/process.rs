//! Operator-verified fixture executables, owned process groups, pidfd cancellation, and resource audit.

use super::*;
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

mod owned;
pub use owned::*;
