use crucible_runtime::{
    bench::*, lease::*, ledger::Resources, worker::*, BlobDigest, RuntimeError,
};
use std::{collections::BTreeSet, path::PathBuf};

#[path = "../../crucible-contract/tests/common/scratch.rs"]
mod scratch;
use scratch::TestScratch;

struct Peer(BlobDigest);
impl AuthenticatedPeer for Peer {
    fn peer_identity(&self) -> &BlobDigest {
        &self.0
    }
}
fn temp(name: &str) -> PathBuf {
    let p = TestScratch::path(&format!("v523-{name}"));
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn lease(now: u64) -> AdmittedLease {
    let root = temp("sysfs");
    std::fs::write(root.join("online"), "0,1\n").unwrap();
    for c in 0..2 {
        let d = root.join(format!("cpu{c}/topology"));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("physical_package_id"), "0").unwrap();
        std::fs::write(d.join("core_id"), c.to_string()).unwrap();
        std::fs::write(d.join("thread_siblings_list"), c.to_string()).unwrap();
    }
    let topo = Topology::from_sysfs(&root, BTreeSet::from([0, 1])).unwrap();
    let source = BlobDigest::of_bytes(b"synthetic authority");
    let host = BlobDigest::of_bytes(b"synthetic host");
    let limits = Resources {
        cpu_ms: 10000,
        gpu_ms: 0,
        ram_bytes: 1024,
        scratch_bytes: 1024,
        ..Resources::default()
    };
    let auth = Authorization {
        host: host.clone(),
        cpus: topo.allowed.clone(),
        expires_ms: now + 1000,
        limits,
        gpu_authorized: false,
        source: source.clone(),
    };
    AdmittedLease::from_operator_config(
        ResourceLease {
            id: "lease-v1".into(),
            host,
            cpus: vec![0],
            expires_ms: now + 1000,
            limits,
            gpu_authorized: false,
            authority: source,
        },
        &auth,
        &topo,
        now,
        &BTreeSet::new(),
    )
    .unwrap()
}
fn request() -> WorkerRequest {
    WorkerRequest {
        schema_version: 1,
        attempt_id: "attempt".into(),
        fence: 4,
        lease_id: "lease-v1".into(),
        operation: Operation::ReadArtifact,
        relative_paths: vec!["result.json".into()],
        arguments: vec![],
    }
}
#[test]
fn authenticated_scoped_current_fence_request_is_accepted() {
    let _scratch = TestScratch::new();
    let root = temp("task");
    std::fs::write(root.join("result.json"), "{}").unwrap();
    let l = lease(10);
    let trusted = BlobDigest::of_bytes(b"coordinator");
    let peer = Peer(trusted.clone());
    let cap = Capability {
        operation: Operation::ReadArtifact,
        solver: None,
        task_root: root.clone(),
    };
    let context = ValidationContext {
        trusted_coordinator: &trusted,
        attempt_id: "attempt",
        fence: 4,
        lease: &l,
        now_ms: 20,
        capability: &cap,
    };
    let paths = validate_request(&peer, &request(), &context).unwrap();
    assert_eq!(paths, [root.join("result.json")]);
}
#[test]
fn worker_rejects_untrusted_paths_schema_stale_fence_and_expired_lease() {
    let _scratch = TestScratch::new();
    let root = temp("task-negative");
    let trusted = BlobDigest::of_bytes(b"coordinator");
    let peer = Peer(trusted.clone());
    let l = lease(10);
    let cap = Capability {
        operation: Operation::ReadArtifact,
        solver: None,
        task_root: root,
    };
    let mut r = request();
    r.relative_paths = vec!["../secret".into()];
    let c = ValidationContext {
        trusted_coordinator: &trusted,
        attempt_id: "attempt",
        fence: 4,
        lease: &l,
        now_ms: 20,
        capability: &cap,
    };
    assert!(validate_request(&peer, &r, &c).is_err());
    let mut r = request();
    r.schema_version = 2;
    assert!(validate_request(&peer, &r, &c).is_err());
    let r = request();
    let stale = ValidationContext { fence: 5, ..c };
    assert!(matches!(
        validate_request(&peer, &r, &stale),
        Err(RuntimeError::StaleFence)
    ));
    let expired = ValidationContext { now_ms: 1010, ..c };
    assert!(matches!(
        validate_request(&peer, &r, &expired),
        Err(RuntimeError::ExpiredLease)
    ));
    let other = Peer(BlobDigest::of_bytes(b"attacker"));
    assert!(validate_request(&other, &r, &c).is_err());
}
fn row(id: &str, outcome: AttemptOutcome, synthetic: bool) -> BenchmarkAttempt {
    BenchmarkAttempt {
        attempt_id: id.into(),
        decision_id: id.into(),
        corpus: BlobDigest::of_bytes(b"fixed corpus v1"),
        benchmark: "BENCH-01".into(),
        cache: CacheState::Cold,
        outcome,
        wall_ms: 100,
        peak_rss_bytes: 1000,
        disk_bytes: 2000,
        resource_class: "2-physical-cpu".into(),
        numerical_quality_pass: true,
        physical_evidence: Some(BlobDigest::of_bytes(b"source-bound physical evidence")),
        synthetic_evaluator: synthetic,
        raw_cell_updates: Some(1000000),
        vram_occupancy_bytes: Some(1024),
    }
}
#[test]
fn benchmark_retains_failures_and_counts_only_qualified_unique_decisions() {
    let rows = [
        row("d1", AttemptOutcome::Qualified, false),
        row("d1-retry", AttemptOutcome::NumericalFailure, false),
        row("d2", AttemptOutcome::Rejected, true),
    ];
    let s = summarize(&rows).unwrap();
    assert_eq!(s.attempts, 3);
    assert_eq!(s.failed_attempts, 2);
    assert_eq!(s.qualified_unique_decisions, 1);
    assert_eq!(s.wall_ms.p50, 100);
    assert_eq!(s.wall_ms.p95, 100);
    assert_eq!(s.eligible_throughput_per_wall_second, Some(1000.0 / 300.0));
}
#[test]
fn fake_evaluator_cannot_claim_qualified_throughput() {
    let mut synthetic = row("fake", AttemptOutcome::Qualified, true);
    assert!(!synthetic.is_qualified());
    assert!(synthetic.check().is_err());
    synthetic.outcome = AttemptOutcome::Rejected;
    let s = summarize(&[synthetic]).unwrap();
    assert_eq!(s.qualified_unique_decisions, 0);
    assert_eq!(s.eligible_throughput_per_wall_second, None);
}
#[test]
fn warm_and_cold_cache_records_cannot_be_mixed() {
    let cold = row("cold", AttemptOutcome::Qualified, false);
    let mut warm = row("warm", AttemptOutcome::Qualified, false);
    warm.cache = CacheState::Warm;
    assert!(summarize(&[cold, warm]).is_err());
}
#[test]
fn process_arguments_are_bounded_and_never_parsed_as_shell() {
    let args = vec!["--case".to_string(), "case 1".to_string()];
    assert_eq!(bounded_argv(&args).unwrap().len(), 2);
    assert!(bounded_argv(&["x".repeat(MAX_ARGUMENT_BYTES + 1)]).is_err());
    assert!(bounded_argv(&["bad\0arg".into()]).is_err());
}

#[test]
fn benchmark_uses_declared_nearest_rank_and_keeps_failed_attempts() {
    let rows: Vec<_> = (1..=20)
        .map(|i| {
            let outcome = if i % 5 == 0 {
                AttemptOutcome::NumericalFailure
            } else {
                AttemptOutcome::Qualified
            };
            let mut attempt = row(&format!("attempt-{i}"), outcome, false);
            attempt.wall_ms = i;
            attempt
        })
        .collect();
    let summary = summarize(&rows).unwrap();
    assert_eq!(summary.attempts, 20);
    assert_eq!(summary.failed_attempts, 4);
    // Nearest-rank quantile: x[ceil(n*p)-1], independently numbered 1..20.
    assert_eq!(summary.wall_ms.p50, 10);
    assert_eq!(summary.wall_ms.p95, 19);
    assert_eq!(summary.qualified_unique_decisions, 16);
    assert_eq!(
        summary.eligible_throughput_per_wall_second,
        Some(16_000.0 / 210.0)
    );
}

#[test]
fn benchmark_rejects_elapsed_overflow_without_panicking() {
    let mut attempt = row("large", AttemptOutcome::NumericalFailure, false);
    attempt.wall_ms = 1_u64 << 53;
    let rows = vec![attempt; 2048];
    let outcome = std::panic::catch_unwind(|| summarize(&rows));
    assert!(matches!(outcome, Ok(Err(RuntimeError::Invalid(_)))));
}
