#[path = "../../crucible-contract/tests/common/scratch.rs"]
mod scratch;
use crucible_cli::*;
use crucible_runtime::{checkpoint::*, db::RuntimeDb, task::Phase};
use scratch::TestScratch;
use serde_json::Value;
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};
fn base() -> PathBuf {
    let p = TestScratch::path("cli-lifecycle");
    open_store(&p).unwrap();
    p
}
fn command(args: &[&str]) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_crucible"));
    c.args(args)
        .env("OMP_NUM_THREADS", "1")
        .env("OPENBLAS_NUM_THREADS", "1")
        .env("RAYON_NUM_THREADS", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    c
}
fn record(base: &Path, args: &[&str], out: &Output) {
    let row = serde_json::json!({"argv":args,"exit_code":out.status.code(),"stdout":String::from_utf8_lossy(&out.stdout),"stderr":String::from_utf8_lossy(&out.stderr)});
    let mut f = OpenOptions::new()
        .append(true)
        .create(true)
        .open(base.join("commands.jsonl"))
        .unwrap();
    writeln!(f, "{row}").unwrap();
}
fn wait(mut child: Child, limit: Duration) -> Output {
    // Drain both streams while the process is alive. A blocked pipe writer
    // cannot exit, so polling exit before reading can deadlock the test itself.
    let drain = |pipe: Option<_>| {
        pipe.map(|mut stream: std::process::ChildStdout| {
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                stream.read_to_end(&mut bytes).unwrap();
                bytes
            })
        })
    };
    let stdout = drain(child.stdout.take());
    let stderr = child.stderr.take().map(|mut stream| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).unwrap();
            bytes
        })
    });
    let start = Instant::now();
    let mut timed_out = false;
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() > limit {
            child.kill().unwrap();
            timed_out = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = Output {
        status: child.wait().unwrap(),
        stdout: stdout
            .map(|thread| thread.join().unwrap())
            .unwrap_or_default(),
        stderr: stderr
            .map(|thread| thread.join().unwrap())
            .unwrap_or_default(),
    };
    assert!(
        !timed_out,
        "owned CLI exceeded bound: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
fn call(base: &Path, args: &[&str], code: i32) -> Value {
    let out = wait(command(args).spawn().unwrap(), Duration::from_secs(60));
    record(base, args, &out);
    assert_eq!(
        out.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    if code == 0 || code == 130 {
        serde_json::from_slice(&out.stdout).unwrap()
    } else {
        Value::Null
    }
}

fn checked_attempt_charges(
    attempts: &TableSnapshot,
    charges: &TableSnapshot,
    committed_tasks: usize,
    reported: u64,
) -> Result<u64, &'static str> {
    let column = |table: &TableSnapshot, name: &str| {
        table.columns.iter().position(|c| c == name).ok_or("column")
    };
    let id = column(attempts, "id")?;
    let task = column(attempts, "task")?;
    let fence = column(attempts, "fence")?;
    let phase = column(attempts, "phase")?;
    let charged_attempt = column(charges, "attempt")?;
    let mut identities = std::collections::BTreeSet::new();
    let mut histories = std::collections::BTreeMap::<_, Vec<_>>::new();
    for row in &attempts.rows {
        let identity = row[id].as_str().ok_or("attempt identity")?;
        let task = row[task].as_str().ok_or("task identity")?;
        let fence = row[fence].as_u64().ok_or("fence")?;
        let phase = row[phase].as_str().ok_or("phase")?;
        if !identities.insert(identity)
            || !matches!(
                phase,
                "committed" | "timed_out" | "numerical_failure" | "infrastructure_failure"
            )
        {
            return Err("duplicate or unfinished attempt");
        }
        histories.entry(task).or_default().push((fence, phase));
    }
    if histories.len() != committed_tasks {
        return Err("wrong committed task count");
    }
    for history in histories.values_mut() {
        history.sort_unstable();
        for (index, (fence, phase)) in history.iter().enumerate() {
            if *fence != index as u64 + 1 || (*phase == "committed") != (index + 1 == history.len())
            {
                return Err("missing fence or noncanonical completion");
            }
        }
    }
    let mut charged = std::collections::BTreeSet::new();
    for row in &charges.rows {
        if !charged.insert(row[charged_attempt].as_str().ok_or("charge identity")?) {
            return Err("duplicate charge");
        }
    }
    if charged != identities || reported != charged.len() as u64 {
        return Err("missing, orphaned or unreported charge");
    }
    Ok(reported)
}

fn assert_run_accounting(root: &Path, report: &Value, committed_tasks: usize) -> u64 {
    let ledger = RuntimeDb::inspect_existing(root)
        .unwrap()
        .ledger_snapshot()
        .unwrap();
    let count = checked_attempt_charges(
        &ledger.tables["attempts"],
        &ledger.tables["charges"],
        committed_tasks,
        report["accounting"]["charges"].as_u64().unwrap(),
    )
    .unwrap();
    assert_eq!(ledger.accounting.charges, count);
    assert_eq!(
        report["accounting"],
        serde_json::to_value(ledger.accounting).unwrap()
    );
    count
}

#[test]
fn quiescent_charge_oracle_preserves_retries_and_rejects_unaccounted_attempts() {
    let attempts: TableSnapshot = serde_json::from_value(serde_json::json!({
        "columns": ["id", "task", "fence", "phase"],
        "rows": [["a1", "a", 1, "timed_out"], ["a2", "a", 2, "committed"],
                 ["b1", "b", 1, "committed"]]
    }))
    .unwrap();
    let charges: TableSnapshot = serde_json::from_value(serde_json::json!({
        "columns": ["attempt"], "rows": [["a1"], ["a2"], ["b1"]]
    }))
    .unwrap();
    assert_eq!(checked_attempt_charges(&attempts, &charges, 2, 3), Ok(3));
    assert!(checked_attempt_charges(&attempts, &charges, 2, 2).is_err());
    assert!(checked_attempt_charges(&attempts, &charges, 3, 3).is_err());
    let mut missing = charges.clone();
    missing.rows.remove(0);
    assert!(checked_attempt_charges(&attempts, &missing, 2, 2).is_err());
    let mut duplicate = charges.clone();
    duplicate.rows.push(charges.rows[0].clone());
    assert!(checked_attempt_charges(&attempts, &duplicate, 2, 4).is_err());
    let mut erased_failure = attempts.clone();
    erased_failure.rows.remove(0);
    assert!(checked_attempt_charges(&erased_failure, &charges, 2, 3).is_err());
    assert!(checked_attempt_charges(&erased_failure, &missing, 2, 2).is_err());
}

#[test]
fn bounded_wait_drains_small_pipes_before_child_closes() {
    const CHILD: &str = "JOIL_TEST_SMALL_PIPE_CHILD";
    const BYTES: usize = 128 * 1024;
    if std::env::var_os(CHILD).is_some() {
        let _ = std::io::Read::read(&mut std::io::stdin(), &mut [0u8; 1]);
        std::io::stdout().write_all(&vec![b'x'; BYTES]).unwrap();
        std::io::stderr().write_all(&vec![b'y'; BYTES]).unwrap();
        return;
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "bounded_wait_drains_small_pipes_before_child_closes",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    use std::os::fd::AsRawFd;
    for fd in [
        child.stdout.as_ref().unwrap().as_raw_fd(),
        child.stderr.as_ref().unwrap().as_raw_fd(),
    ] {
        // Deliberately smaller than either stream: waiting for exit before
        // draining now deadlocks independently of host pipe-default policy.
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETPIPE_SZ, 4096) }, 4096);
    }
    drop(child.stdin.take());
    let out = wait(child, Duration::from_secs(2));
    assert!(out.status.success());
    assert!(out.stdout.iter().filter(|b| **b == b'x').count() >= BYTES);
    assert!(out.stderr.iter().filter(|b| **b == b'y').count() >= BYTES);
}
#[test]
fn actual_cli_lifecycle_kills_worker_replay_retry_cost_stop_fork_and_seed_import() {
    let _scratch = TestScratch::new();
    let base = base();
    let root_a = base.join("one-worker");
    let root_b = base.join("two-workers");
    let cfg = Campaign {
        schema_version: 1,
        domain: "synthetic_integer_fixture".into(),
        epochs: 2,
        workers: 1,
        seeds: vec![[3, 4], [5, 12]],
        weights: [1, 0],
        mode: crucible_runtime::task::FixtureMode::Succeed,
        timeout_ms: 1000,
    };
    let a_cfg = base.join("one.toml");
    std::fs::write(&a_cfg, toml::to_string(&cfg).unwrap()).unwrap();
    let mut two = cfg.clone();
    two.workers = 2;
    let b_cfg = base.join("two.toml");
    let topology = crucible_runtime::lease::Topology::system().unwrap();
    let physical = topology
        .allowed
        .iter()
        .map(|cpu| topology.cpus[cpu])
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    std::fs::write(&b_cfg, toml::to_string(&two).unwrap()).unwrap();
    if physical < 2 {
        // Two workers are not permitted here. Test the refusal before using the allowed count.
        call(
            &base,
            &[
                "run",
                b_cfg.to_str().unwrap(),
                "--root",
                root_b.to_str().unwrap(),
            ],
            78,
        );
        assert!(!root_b.exists());
        two.workers = 1;
        std::fs::write(&b_cfg, toml::to_string(&two).unwrap()).unwrap();
    }
    write_new(&base.join("worker-capability.json"),serde_json::json!({"physical_cores_available":physical,"worker_counts_exercised":[1,two.workers],"two_workers_refused_if_unavailable":physical<2}).to_string().as_bytes()).unwrap();
    let a = a_cfg.to_str().unwrap();
    let b = b_cfg.to_str().unwrap();
    let ra = root_a.to_str().unwrap();
    let rb = root_b.to_str().unwrap();
    assert_eq!(call(&base, &["admit", a], 0)["admitted"], true);
    let first = call(&base, &["run", a, "--root", ra], 0);
    assert_eq!(first["epochs"], 2);
    assert_run_accounting(&root_a, &first, 4);
    assert_eq!(first["accounting"]["exceeded"], false);
    let initial = call(&base, &["run", b, "--root", rb, "--until", "1"], 0);
    // A bounded Succeed child can time out under host contention. Every actual
    // failed attempt must stay charged; logical task count is not attempt count.
    let initial_charges = assert_run_accounting(&root_b, &initial, 2);
    let initial_meta = std::fs::read(root_b.join("run.json")).unwrap();
    call(&base, &["run", b, "--root", rb], 78);
    assert_eq!(
        std::fs::read(root_b.join("run.json")).unwrap(),
        initial_meta
    );
    // Actual owned coordinator SIGKILL at all four publication boundaries. No native work is active here.
    for phase in [
        CheckpointPhase::BeforeRename,
        CheckpointPhase::AfterRename,
        CheckpointPhase::BeforePointerCommit,
        CheckpointPhase::AfterPointerCommit,
    ] {
        let status = call(&base, &["status", rb], 0);
        let next = status["history"][0]["generation"].as_u64().unwrap() + 1;
        let marker = root_b.join("checkpoint-test-barrier.json");
        // A previous test marker is retained with a unique name, never overwritten or removed.
        if marker.exists() {
            std::fs::rename(
                &marker,
                root_b.join(format!("barrier-{phase:?}-{next}.json")),
            )
            .unwrap();
        }
        let mut c = command(&["run", b, "--resume", rb, "--until", "1"]);
        c.env("JOIL_TEST_CHECKPOINT_PHASE", format!("{phase:?}"))
            .env("JOIL_TEST_CHECKPOINT_GENERATION", next.to_string());
        let mut child = c.spawn().unwrap();
        let start = Instant::now();
        while !marker.exists() {
            assert!(
                child.try_wait().unwrap().is_none(),
                "CLI exited before {phase:?}"
            );
            if start.elapsed() > Duration::from_secs(30) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("no checkpoint barrier");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        let out = child.wait_with_output().unwrap();
        record(&base, &["owned-SIGKILL", &format!("{phase:?}")], &out);
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(out.status.signal(), Some(libc::SIGKILL));
        let verified = call(&base, &["checkpoint", "verify", rb], 0);
        assert_eq!(verified["offline"], true);
        call(&base, &["run", b, "--resume", rb, "--until", "1"], 0);
    }
    call(&base, &["stop", rb], 0);
    let stopped = call(&base, &["run", b, "--resume", rb, "--until", "1"], 130);
    assert_eq!(assert_run_accounting(&root_b, &stopped, 2), initial_charges);
    call(
        &base,
        &["run", b, "--resume", rb, "--until", "1", "--continue"],
        0,
    );
    // SIGINT during a real injected timeout drains/charges its bounded batch, then checkpoints.
    let mut retry = two.clone();
    retry.mode = crucible_runtime::task::FixtureMode::Timeout;
    let retry_cfg = base.join("retry.toml");
    std::fs::write(&retry_cfg, toml::to_string(&retry).unwrap()).unwrap();
    let retry_path = retry_cfg.to_str().unwrap();
    let child = command(&["run", retry_path, "--resume", rb])
        .spawn()
        .unwrap();
    let pid = child.id();
    let start = Instant::now();
    let running_generation;
    loop {
        let running = RuntimeDb::inspect_existing(&root_b)
            .ok()
            .and_then(|db| db.task("fixture-epoch-2-ordinal-2").ok())
            .is_some_and(|row| row.phase == Phase::Running);
        if running {
            running_generation = RuntimeDb::inspect_existing(&root_b)
                .unwrap()
                .checkpoint_history()
                .unwrap()[0]
                .generation;
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
            panic!("fixture never ran");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let signal = Instant::now();
    assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGINT) }, 0);
    let out = wait(child, Duration::from_secs(30));
    record(&base, &["owned-SIGINT-during-fixture"], &out);
    assert_eq!(
        out.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(signal.elapsed() < Duration::from_secs(30));
    let stopped: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(stopped["stopped"], true);
    // One failed and one successful attempt per job in the drained batch; a
    // single-worker host must leave the second job planned, without charging it.
    assert_eq!(
        stopped["accounting"]["charges"],
        initial_charges + 2 * retry.workers as u64
    );
    assert_run_accounting(&root_b, &stopped, 2 + retry.workers);
    let mut stopped_db = RuntimeDb::inspect_existing(&root_b).unwrap();
    assert_eq!(
        stopped_db.checkpoint_history().unwrap()[0].generation,
        running_generation + 1,
        "the drained charged batch and stop state publish together"
    );
    let stopped_meta = metadata(&root_b).unwrap();
    let restored = stopped_db
        .restore_latest(&open_store(&root_b).unwrap(), &stopped_meta.experiment)
        .unwrap();
    assert_eq!(restored.checkpoint.state.in_flight.len(), 2 - retry.workers);
    assert_eq!(
        restored.checkpoint.state.pending_results.len(),
        retry.workers
    );
    assert!(matches!(
        restored.checkpoint.state.stop,
        crucible_runtime::replay::StopState::Requested { .. }
    ));
    drop(stopped_db);
    let second = call(&base, &["run", retry_path, "--resume", rb, "--continue"], 0);
    assert_eq!(second["epochs"], 2);
    assert_eq!(first["logical_digest"], second["logical_digest"]);
    assert_ne!(first["actual_cost_digest"], second["actual_cost_digest"]);
    let final_charges = initial_charges + 4;
    assert_eq!(assert_run_accounting(&root_b, &second, 4), final_charges);
    // Corruption is reported; fallback can reconcile canonical accepted results and continue.
    let meta = metadata(&root_b).unwrap();
    let mut db = RuntimeDb::inspect_existing(&root_b).unwrap();
    let latest = db.checkpoint_history().unwrap()[0].clone();
    drop(db);
    std::fs::write(
        root_b
            .join("objects")
            .join(latest.object.as_str())
            .join("manifest.json"),
        b"negative-control truncated latest manifest",
    )
    .unwrap();
    let fallback = call(&base, &["checkpoint", "verify", rb], 0);
    assert_eq!(fallback["rejected_newer"].as_array().unwrap().len(), 1);
    let recovered = call(&base, &["run", retry_path, "--resume", rb, "--continue"], 0);
    assert_eq!(recovered["logical_digest"], first["logical_digest"]);
    assert_eq!(assert_run_accounting(&root_b, &recovered, 4), final_charges);
    let before = status(&root_b).unwrap()["accounting"].clone();
    let forked = call(&base, &["fork", rb, "--weights", "-1,1"], 0);
    assert_eq!(forked["observations_reused"], 4);
    assert_eq!(forked["accounting"], before);
    assert_ne!(forked["lineage"]["id"], second["lineage"]);
    let mut db = RuntimeDb::inspect_existing(&root_b).unwrap();
    let restored = db
        .restore_latest(&open_store(&root_b).unwrap(), &meta.experiment)
        .unwrap();
    assert_eq!(restored.checkpoint.state.lineage.weights, [-1, 1]);
    assert_eq!(restored.checkpoint.ledger.accounting.charges, final_charges);
    let seed = base.join("seed.json");
    std::fs::write(
        &seed,
        br#"{"values":[3,4],"claim":"PASS","compiled":"forged"}"#,
    )
    .unwrap();
    let imported = call(&base, &["seed-import", seed.to_str().unwrap()], 0);
    assert_eq!(imported["inherited_verdicts"], false);
    assert!(!imported.to_string().contains("PASS"));
    let invalid = base.join("invalid.toml");
    two.workers = 3;
    std::fs::write(&invalid, toml::to_string(&two).unwrap()).unwrap();
    let never = base.join("invalid-never-created");
    call(
        &base,
        &[
            "run",
            invalid.to_str().unwrap(),
            "--root",
            never.to_str().unwrap(),
        ],
        64,
    );
    assert!(!never.exists());
    // Missing canonical accounting is never recreated from a checkpoint backup.
    let absent = base.join("missing-ledger");
    open_store(&absent).unwrap();
    write_new(&absent.join("run.json"), &initial_meta).unwrap();
    call(&base, &["run", b, "--resume", absent.to_str().unwrap()], 78);
    assert!(!absent.join("runtime.sqlite").exists());
}

#[test]
fn writer_lock_conflict_failure_and_drop_release_descriptor() {
    let _scratch = TestScratch::new();
    let root = base();
    let first = WriterLock::acquire(&root).unwrap();
    assert!(matches!(
        WriterLock::acquire(&root),
        Err(crucible_runtime::RuntimeError::Conflict(
            "campaign_already_has_writer"
        ))
    ));
    drop(first);
    let reacquired = WriterLock::acquire(&root).unwrap();
    assert!(WriterLock::acquire(&root).is_err());
    drop(reacquired);
    assert!(WriterLock::acquire(&root).is_ok());
    assert!(WriterLock::acquire(&root.join("absent")).is_err());
}
