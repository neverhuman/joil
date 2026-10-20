use super::*;

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
