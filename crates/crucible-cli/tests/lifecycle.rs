use crucible_cli::*;
use crucible_runtime::{checkpoint::*, db::RuntimeDb, task::Phase};
use serde_json::Value;
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};
fn base() -> PathBuf {
    let p = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute() && p.components().any(|c| c.as_os_str() == ".private"))
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join(".private/test-evidence")
        })
        .join(format!(
            "v5-cli-lifecycle-{}-{}",
            std::process::id(),
            crucible_runtime::utc_millis().unwrap()
        ));
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
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() > limit {
            child.kill().unwrap();
            let o = child.wait_with_output().unwrap();
            panic!(
                "owned CLI exceeded bound: {}",
                String::from_utf8_lossy(&o.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.wait_with_output().unwrap()
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
#[test]
fn actual_cli_lifecycle_kills_worker_replay_retry_cost_stop_fork_and_seed_import() {
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
    assert_eq!(first["accounting"]["charges"], 4);
    assert_eq!(first["accounting"]["exceeded"], false);
    let initial = call(&base, &["run", b, "--root", rb, "--until", "1"], 0);
    assert_eq!(initial["accounting"]["charges"], 2);
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
    assert_eq!(stopped["accounting"]["charges"], 2);
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
        2 + 2 * retry.workers as u64
    );
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
    assert_eq!(second["accounting"]["charges"], 6);
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
    assert_eq!(recovered["accounting"]["charges"], 6);
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
    assert_eq!(restored.checkpoint.ledger.accounting.charges, 6);
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
