#[path = "../../crucible-contract/tests/common/scratch.rs"]
mod scratch;
use scratch::TestScratch;
#[path = "../../crucible-contract/tests/common/mod.rs"]
mod review_fixture;
use crucible_runtime::{db::RuntimeDb, lease::*, ledger::*, task::*, *};
use crucible_store::{
    manifest::{Metadata, ObjectKind},
    Store,
};
use joil_evidence::{claims::ReviewAction, decision::*, identity::ContentRef, observation::Finite};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::Duration,
};
fn path(label: &str) -> PathBuf {
    TestScratch::path(&format!("v5-runtime-{label}"))
}
fn experiment() -> SemanticId {
    SemanticId::of(&"synthetic runtime software fixture").unwrap()
}
fn authorized() -> Resources {
    Resources {
        cpu_ms: 20_000,
        gpu_ms: 0,
        ram_bytes: 512 * 1024 * 1024,
        vram_bytes: 0,
        scratch_bytes: 32 * 1024 * 1024,
        io_bytes: 128 * 1024 * 1024,
    }
}
fn reserve() -> Resources {
    Resources {
        cpu_ms: 1000,
        gpu_ms: 0,
        ram_bytes: 128 * 1024 * 1024,
        vram_bytes: 0,
        scratch_bytes: 2 * 1024 * 1024,
        io_bytes: 16 * 1024 * 1024,
    }
}
fn context(label: &str) -> (RuntimeDb, Store) {
    let path = path(label);
    let s = Store::open(&path).unwrap();
    (
        RuntimeDb::open(&path, authorized(), &experiment()).unwrap(),
        s,
    )
}
fn spec(mode: FixtureMode) -> TaskSpec {
    TaskSpec {
        request: Request::SoftwareFixture {
            request: FixtureRequest {
                schema_version: 1,
                values: vec![3, 4],
                address: RngAddress {
                    campaign: experiment(),
                    island: 0,
                    epoch: 1,
                    proposal: 0,
                    operator: "synthetic_integer_sum_squares".into(),
                    draw: 0,
                },
                mode,
                executable: BlobDigest::of_bytes(
                    b"SYNTHETIC executable identity for transaction-only tests",
                ),
            },
        },
        due_epoch: 1,
        ordinal: 0,
        class: WorkClass::Evaluation,
        reserve: reserve(),
        ranks: 1,
        threads: 1,
    }
}
fn fake_topology() -> Topology {
    let root = path("synthetic-sysfs");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("online"), "0,2,7\n").unwrap();
    for (cpu, core, siblings) in [(0, 10, "0,7"), (2, 11, "2"), (7, 10, "0,7")] {
        let d = root.join(format!("cpu{cpu}/topology"));
        std::fs::create_dir_all(&d).unwrap();
        for (name, value) in [
            ("physical_package_id", "0".into()),
            ("core_id", core.to_string()),
            ("thread_siblings_list", siblings.into()),
        ] {
            std::fs::write(d.join(name), value).unwrap();
        }
    }
    Topology::from_sysfs(&root, BTreeSet::from([0, 2, 7])).unwrap()
}
fn lease(topology: &Topology, name: &str, cpus: Vec<u32>, expires: u64, now: u64) -> AdmittedLease {
    let source = BlobDigest::of_bytes(b"synthetic operator authorization, not a physical campaign");
    let host = BlobDigest::of_bytes(b"synthetic or private test host realm");
    let authority = Authorization {
        host: host.clone(),
        cpus: topology.allowed.clone(),
        expires_ms: expires,
        limits: authorized(),
        gpu_authorized: false,
        source: source.clone(),
    };
    let record = ResourceLease {
        id: name.into(),
        host,
        cpus,
        expires_ms: expires,
        limits: authorized(),
        gpu_authorized: false,
        authority: source,
    };
    AdmittedLease::from_operator_config(record, &authority, topology, now, &BTreeSet::new())
        .unwrap()
}
fn output(s: &Store, task: &TaskSpec, score: i64) -> BlobDigest {
    let Request::SoftwareFixture { request } = &task.request else {
        panic!()
    };
    let observation = FixtureObservation {
        schema_version: 1,
        synthetic_only: true,
        request: task.request.identity().unwrap(),
        score,
        address: request.address.clone(),
    };
    s.publish(
        &serde_json::to_vec(&observation).unwrap(),
        Metadata {
            semantic: Some(task.request.identity().unwrap()),
            ..Metadata::private(ObjectKind::Observation)
        },
    )
    .unwrap()
}
fn synthetic_charge(ms: u64) -> Charge {
    Charge {
        measured: Resources {
            cpu_ms: ms,
            ram_bytes: 1024,
            scratch_bytes: 1024,
            io_bytes: 512,
            ..Resources::default()
        },
        wall_ns: 1_000_000,
        accounting_basis: "SYNTHETIC transaction fixture, not measured process cost".into(),
    }
}

#[test]
fn real_sqlite_transitions_require_verified_closure_charge_and_exact_duplicate_identity() {
    let _scratch = TestScratch::new();
    let (mut db, s) = context("states");
    let spec = spec(FixtureMode::Succeed);
    db.enqueue("task", &spec).unwrap();
    assert_eq!(db.task("task").unwrap().phase, Phase::Ready);
    let topo = fake_topology();
    let lease = lease(&topo, "lease", vec![0], 100, 1);
    let attempt = db.lease("task", "attempt", &lease, 2).unwrap();
    assert_eq!(db.task("task").unwrap().phase, Phase::Leased);
    assert!(db.commit(&attempt, &s, 3).is_err());
    db.start(&attempt, 3).unwrap();
    assert!(db.start(&attempt, 3).is_err());
    let object = output(&s, &spec, 25); // independent 3² + 4² = 25
    db.stage(&attempt, &object, 4).unwrap();
    assert!(db.commit(&attempt, &s, 4).is_err());
    db.verify(&attempt, &s, 5).unwrap();
    assert!(db.commit(&attempt, &s, 5).is_err());
    db.charge(&attempt, &synthetic_charge(5), 6).unwrap();
    assert_eq!(db.commit(&attempt, &s, 7).unwrap(), object);
    assert_eq!(db.commit(&attempt, &s, 500).unwrap(), object);
    assert_eq!(db.accounting().unwrap().charges, 1);
    let mut forged = attempt.clone();
    forged.evaluation = SemanticId::of(&"different evaluation").unwrap();
    assert!(matches!(
        db.commit(&forged, &s, 8),
        Err(RuntimeError::StaleFence)
    ));
    assert!(db.fail(&attempt, Phase::NumericalFailure, 8).is_err());
    assert_eq!(db.task("task").unwrap().phase, Phase::Committed);
    assert!(db.retry_ready("task").is_err());
    let mut changed = spec.clone();
    if let Request::SoftwareFixture { request } = &mut changed.request {
        request.values = vec![5, 12];
    }
    assert!(db.enqueue("task", &changed).is_err());
    assert!(db.conflict_count().unwrap() >= 3);
}
#[test]
fn absent_corrupt_or_wrong_value_outputs_never_commit_from_a_successful_process_claim() {
    let _scratch = TestScratch::new();
    let (mut db, s) = context("invalid-output");
    let topo = fake_topology();
    for (i, kind) in ["missing", "wrong-value", "corrupt"]
        .into_iter()
        .enumerate()
    {
        let spec = spec(FixtureMode::Succeed);
        let name = format!("task-{i}");
        db.enqueue(&name, &spec).unwrap();
        let lease = lease(&topo, &format!("lease-{i}"), vec![0], 100, 1);
        let a = db.lease(&name, &format!("attempt-{i}"), &lease, 2).unwrap();
        db.start(&a, 3).unwrap();
        let object = if kind == "missing" {
            BlobDigest::of_bytes(b"absent output manifest")
        } else {
            output(&s, &spec, if kind == "wrong-value" { 26 } else { 25 })
        };
        if kind == "corrupt" {
            std::fs::write(
                s.path()
                    .join("objects")
                    .join(object.as_str())
                    .join("payload"),
                b"corrupt",
            )
            .unwrap();
        }
        db.stage(&a, &object, 4).unwrap();
        assert!(db.verify(&a, &s, 5).is_err());
        db.charge(&a, &synthetic_charge(1), 6).unwrap();
        assert!(db.commit(&a, &s, 7).is_err());
        db.fail(&a, Phase::InfrastructureFailure, 8).unwrap();
        assert!(db.task(&name).unwrap().accepted_object.is_none());
    }
    assert_eq!(db.accounting().unwrap().charges, 3);
}
#[test]
fn late_stale_worker_is_quarantined_and_both_attempts_are_charged() {
    let _scratch = TestScratch::new();
    let (mut db, s) = context("fences");
    let spec = spec(FixtureMode::Succeed);
    db.enqueue("task", &spec).unwrap();
    let topo = fake_topology();
    let old_lease = lease(&topo, "old-lease", vec![0], 20, 1);
    let old = db.lease("task", "old", &old_lease, 2).unwrap();
    db.start(&old, 3).unwrap();
    let object = output(&s, &spec, 25);
    db.stage(&old, &object, 4).unwrap();
    db.verify(&old, &s, 5).unwrap();
    // Expiry alone does not release physical ownership while real cost is
    // unresolved. Retry on a different admitted core within the remaining vector.
    let sibling = lease(&topo, "sibling", vec![7], 100, 25);
    assert!(db.lease("task", "overlap", &sibling, 25).is_err());
    let new_lease = lease(&topo, "new-lease", vec![2], 100, 25);
    let new = db.lease("task", "new", &new_lease, 25).unwrap();
    assert_eq!(new.fence, old.fence + 1);
    db.charge(&old, &synthetic_charge(7), 26).unwrap();
    assert!(matches!(
        db.commit(&old, &s, 26),
        Err(RuntimeError::StaleFence)
    ));
    assert_eq!(db.task("task").unwrap().phase, Phase::Leased);
    db.start(&new, 27).unwrap();
    db.stage(&new, &object, 28).unwrap();
    db.verify(&new, &s, 29).unwrap();
    db.charge(&new, &synthetic_charge(11), 30).unwrap();
    db.commit(&new, &s, 31).unwrap();
    let accounting = db.accounting().unwrap();
    assert_eq!(accounting.charges, 2);
    assert_eq!(accounting.spent.cpu_ms, 18);
    assert_eq!(accounting.reserved, Resources::default());
    assert!(db.conflict_count().unwrap() >= 1);
    assert!(db.charge(&old, &synthetic_charge(8), 32).is_err());
    assert_eq!(db.accounting().unwrap().spent.cpu_ms, 18);
}
#[test]
fn resource_vector_reservations_and_every_work_class_survive_reopen_without_free_failures() {
    let _scratch = TestScratch::new();
    let (mut db, s) = context("resources");
    let topo = fake_topology();
    for (i, class) in [
        WorkClass::Evaluation,
        WorkClass::Retry,
        WorkClass::CacheLookup,
        WorkClass::Audit,
        WorkClass::SurrogateFit,
    ]
    .into_iter()
    .enumerate()
    {
        let mut spec = spec(FixtureMode::Unsupported);
        spec.class = class;
        db.enqueue(&format!("task-{i}"), &spec).unwrap();
        let l = lease(&topo, &format!("lease-{i}"), vec![0], 100, 1);
        let a = db
            .lease(&format!("task-{i}"), &format!("a-{i}"), &l, 2)
            .unwrap();
        db.start(&a, 3).unwrap();
        db.charge(&a, &synthetic_charge(i as u64 + 1), 4).unwrap();
        db.fail(&a, Phase::Unsupported, 5).unwrap();
    }
    assert_eq!(db.accounting().unwrap().spent.cpu_ms, 15);
    drop(db);
    let mut db = RuntimeDb::open(s.path(), authorized(), &experiment()).unwrap();
    assert_eq!(db.accounting().unwrap().charges, 5);
    let mut huge = spec(FixtureMode::Succeed);
    huge.reserve.ram_bytes = authorized().ram_bytes;
    db.enqueue("large-one", &huge).unwrap();
    let one = lease(&topo, "large-lease-one", vec![0], 100, 1);
    db.lease("large-one", "large-a", &one, 2).unwrap();
    db.enqueue("large-two", &spec(FixtureMode::Succeed))
        .unwrap();
    let two = lease(&topo, "large-lease-two", vec![2], 100, 1);
    assert!(matches!(
        db.lease("large-two", "large-b", &two, 2),
        Err(RuntimeError::Budget(_))
    ));
    assert_eq!(
        db.accounting().unwrap().reserved.ram_bytes,
        authorized().ram_bytes
    );
}
#[test]
fn real_sysfs_shape_and_explicit_expiry_gpu_sibling_and_rank_negatives() {
    let _scratch = TestScratch::new();
    let topo = fake_topology();
    assert!(topo.physical_set(&[0, 7]).is_err());
    assert_eq!(topo.physical_set(&[0, 2]).unwrap().len(), 2);
    assert!(parse_cpu_list("0-2,2").is_err());
    assert!(parse_cpu_list("0-999999999").is_err());
    let admitted = lease(&topo, "valid", vec![0, 2], 100, 1);
    assert!(admitted.check_task(reserve(), 3, 1, 2).is_err());
    assert!(admitted.check_task(reserve(), 1, 1, 100).is_err());
    let authority = Authorization {
        host: admitted.record().host.clone(),
        cpus: topo.allowed.clone(),
        expires_ms: 100,
        limits: authorized(),
        gpu_authorized: false,
        source: admitted.record().authority.clone(),
    };
    let mut historical = admitted.record().clone();
    historical.gpu_authorized = true;
    historical.limits.gpu_ms = 1;
    assert!(AdmittedLease::from_operator_config(
        historical,
        &authority,
        &topo,
        2,
        &BTreeSet::new()
    )
    .is_err());
    assert!(AdmittedLease::from_operator_config(
        admitted.record().clone(),
        &authority,
        &topo,
        101,
        &BTreeSet::new()
    )
    .is_err());
}
#[test]
fn strict_schema_versions_and_online_backup_preserve_rows_and_spend() {
    let _scratch = TestScratch::new();
    let (mut db, s) = context("backup");
    let spec = spec(FixtureMode::Unsupported);
    db.enqueue("task", &spec).unwrap();
    let topo = fake_topology();
    let l = lease(&topo, "l", vec![0], 100, 1);
    let a = db.lease("task", "a", &l, 2).unwrap();
    db.start(&a, 3).unwrap();
    db.charge(&a, &synthetic_charge(13), 4).unwrap();
    db.fail(&a, Phase::Unsupported, 5).unwrap();
    let backup = path("backup-copy");
    db.backup(&backup).unwrap();
    let restored = RuntimeDb::open(&backup, authorized(), &experiment()).unwrap();
    assert_eq!(restored.accounting().unwrap().spent.cpu_ms, 13);
    assert_eq!(restored.task("task").unwrap().phase, Phase::Unsupported);
    assert_eq!(
        restored.event_position().unwrap(),
        db.event_position().unwrap()
    );
    let unknown = path("unknown-schema");
    Store::open(&unknown).unwrap();
    let c = rusqlite::Connection::open(unknown.join("runtime.sqlite")).unwrap();
    c.execute_batch(
        "PRAGMA user_version=99;CREATE TABLE canary(value INTEGER);INSERT INTO canary VALUES(42);",
    )
    .unwrap();
    drop(c);
    let before = std::fs::read(unknown.join("runtime.sqlite")).unwrap();
    assert!(RuntimeDb::open(&unknown, authorized(), &experiment()).is_err());
    assert_eq!(
        before,
        std::fs::read(unknown.join("runtime.sqlite")).unwrap()
    );
    assert!(RuntimeDb::open(
        s.path(),
        Resources {
            cpu_ms: authorized().cpu_ms + 1,
            ..authorized()
        },
        &experiment()
    )
    .is_err());
}
fn family(
    review: &mut review_fixture::ReviewFixture,
    id: &str,
    alpha: f64,
) -> (ApprovedFamily, ContentRef) {
    let r = review_fixture::reference("synthetic-policy-dependency");
    let policy = FamilyPolicy {
        experiment_family_id: id.into(),
        requirements: review.requirements.clone(),
        family_alpha: Finite::new(alpha).unwrap(),
        max_attempts: 2,
        criterion: ErrorCriterion::AdaptiveWinnerQualification,
        procedure: Procedure::Bonferroni,
        sidedness: Sidedness::Upper,
        selection: Selection::AdaptiveSearch,
        fixed_subject: None,
        claim_scope: "synthetic_software_only".into(),
        sampling_design: r.clone(),
        joint_model: r.clone(),
        selection_policy: r.clone(),
        stopping_policy: r.clone(),
        holdout_policy: r.clone(),
        operating_population: r,
    };
    let bytes = serde_json::to_vec(&policy).unwrap();
    let source = ContentRef::from_bytes(
        &format!("synthetic-family-policy-{}", review.sequence),
        "1",
        &bytes,
    );
    review.approve(source.clone(), bytes);
    (
        ApprovedFamily::from_registry(&review.registry, source.clone(), &review.requirements)
            .unwrap(),
        source,
    )
}
#[test]
fn alpha_attempt_spend_is_exact_source_bound_nonresetting_and_durable() {
    let _scratch = TestScratch::new();
    let (mut db, s) = context("family");
    let mut review = review_fixture::ReviewFixture::new();
    let (policy, source) = family(&mut review, "synthetic-family", 0.05);
    let d = SemanticId::of(&"decision-one").unwrap();
    let first = db
        .reserve_family_attempt(&policy, &review.registry, 1, &d)
        .unwrap();
    assert_eq!(first.spent_alpha_units, 25_000_000_000);
    assert_eq!(
        db.reserve_family_attempt(&policy, &review.registry, 1, &d)
            .unwrap(),
        first
    );
    assert!(db
        .reserve_family_attempt(
            &policy,
            &review.registry,
            1,
            &SemanticId::of(&"fork tries fresh ordinal").unwrap()
        )
        .is_err());
    drop(db);
    let mut db = RuntimeDb::open(s.path(), authorized(), &experiment()).unwrap();
    let second = db
        .reserve_family_attempt(
            &policy,
            &review.registry,
            2,
            &SemanticId::of(&"decision-two").unwrap(),
        )
        .unwrap();
    assert_eq!(second.spent_alpha_units, 50_000_000_000);
    assert!(db
        .reserve_family_attempt(&policy, &review.registry, 3, &d)
        .is_err());
    let (changed, _) = family(&mut review, "synthetic-family", 0.1);
    assert!(db
        .reserve_family_attempt(&changed, &review.registry, 1, &d)
        .is_err());
    review.review(source, ReviewAction::Revoked);
    assert!(db
        .reserve_family_attempt(&policy, &review.registry, 1, &d)
        .is_err());
    assert_eq!(
        alpha_units(Finite::new(0.03).unwrap()).unwrap(),
        30_000_000_000
    );
    assert!(alpha_units(Finite::new(1e-13).unwrap()).is_err());
}
#[test]
fn native_artifact_executor_charges_success_timeout_numerical_failure_and_unsupported_distinctly() {
    let _scratch = TestScratch::new();
    let (mut db, s) = context("native-executor");
    let topology = Topology::system().unwrap();
    let cpu = *topology.allowed.iter().next().unwrap();
    let binary = Path::new(env!("CARGO_BIN_EXE_runtime-fixture"));
    let digest = BlobDigest::of_bytes(&std::fs::read(binary).unwrap());
    let executable = FixtureExecutable::from_operator_binary(binary, &digest).unwrap();
    for (i, mode) in [
        FixtureMode::Succeed,
        FixtureMode::Timeout,
        FixtureMode::NumericalFailure,
        FixtureMode::Unsupported,
    ]
    .into_iter()
    .enumerate()
    {
        let mut spec = spec(mode.clone());
        if let Request::SoftwareFixture { request } = &mut spec.request {
            request.executable = digest.clone();
        }
        let name = format!("native-{i}");
        db.enqueue(&name, &spec).unwrap();
        let now = utc_millis().unwrap();
        let admitted = lease(
            &topology,
            &format!("native-l-{i}"),
            vec![cpu],
            now + 30_000,
            now,
        );
        let a = db
            .lease(&name, &format!("native-a-{i}"), &admitted, now)
            .unwrap();
        let result = execute_fixture(
            &mut db,
            &s,
            &a,
            &admitted,
            &executable,
            if mode == FixtureMode::Timeout {
                Duration::from_millis(40)
            } else {
                Duration::from_secs(2)
            },
        )
        .unwrap();
        assert!(result.charge.measured.cpu_ms >= 1);
        assert!(result.charge.wall_ns > 0);
        if mode != FixtureMode::Unsupported {
            let dir = s.path().join(format!(
                "attempt-{}",
                BlobDigest::of_bytes(a.id.as_bytes()).as_str()
            ));
            let audit: ProcessAudit =
                serde_json::from_slice(&std::fs::read(dir.join("process-audit.json")).unwrap())
                    .unwrap();
            assert_eq!(audit.cpus, BTreeSet::from([cpu]));
            assert_eq!(audit.address_space_limit, reserve().ram_bytes);
            assert_eq!(audit.file_size_limit, reserve().scratch_bytes);
            assert_eq!(audit.cpu_seconds_limit, 2);
            assert!(audit.owns_process_group);
            assert!(audit.numerical_threads.values().all(|v| v == "1"));
            assert_eq!(audit.cuda_visible_devices, "");
            assert!(db.process_identity(&a.id).unwrap().is_some());
        }

        match mode {
            FixtureMode::Succeed => {
                assert_eq!(result.phase, Phase::Committed);
                let object = s.read(result.object.as_ref().unwrap()).unwrap();
                let observation: FixtureObservation =
                    serde_json::from_slice(&object.payload).unwrap();
                assert_eq!(observation.score, 25);
                assert!(observation.synthetic_only);
            }
            FixtureMode::Timeout => assert_eq!(result.phase, Phase::TimedOut),
            FixtureMode::NumericalFailure => assert_eq!(result.phase, Phase::NumericalFailure),
            FixtureMode::Unsupported => assert_eq!(result.phase, Phase::Unsupported),
        }
        if mode != FixtureMode::Succeed {
            assert!(db.task(&name).unwrap().accepted_object.is_none());
            assert!(result.diagnostic.is_some());
        }
    }
    assert_eq!(db.accounting().unwrap().charges, 4);
    assert_eq!(db.accounting().unwrap().reserved, Resources::default());
}
#[test]
fn cancellation_is_owned_and_does_not_signal_another_fixture_process_group() {
    let _scratch = TestScratch::new();
    let (_, s) = context("owned-cancel");
    let topology = Topology::system().unwrap();
    let cpu = *topology.allowed.iter().next().unwrap();
    let binary = Path::new(env!("CARGO_BIN_EXE_runtime-fixture"));
    let digest = BlobDigest::of_bytes(&std::fs::read(binary).unwrap());
    let executable = FixtureExecutable::from_operator_binary(binary, &digest).unwrap();
    let now = utc_millis().unwrap();
    let admitted = lease(&topology, "cancel-lease", vec![cpu], now + 30_000, now);
    let mut children = Vec::new();
    for i in 0..2 {
        let dir = s.path().join(format!("owned-child-{i}"));
        std::fs::create_dir(&dir).unwrap();
        let mut request = match spec(FixtureMode::Timeout).request {
            Request::SoftwareFixture { request } => request,
            _ => unreachable!(),
        };
        request.executable = digest.clone();
        std::fs::write(
            dir.join("request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        children.push(
            OwnedProcess::spawn_fixture(&executable, &dir, &admitted, reserve(), 1, 1, now)
                .unwrap(),
        );
    }
    let stopped = children[0].cancel(Duration::from_millis(10)).unwrap();
    assert!(!stopped.status.success());
    assert!(children[1].poll().unwrap().is_none());
    children[1].cancel(Duration::from_millis(10)).unwrap();
}

#[test]
fn recorded_process_identity_uses_pidfds_and_rejects_a_changed_start_without_signaling() {
    let _scratch = TestScratch::new();
    let (_, s) = context("recorded-cancel");
    let topology = Topology::system().unwrap();
    let cpu = *topology.allowed.iter().next().unwrap();
    let binary = Path::new(env!("CARGO_BIN_EXE_runtime-fixture"));
    let digest = BlobDigest::of_bytes(&std::fs::read(binary).unwrap());
    let executable = FixtureExecutable::from_operator_binary(binary, &digest).unwrap();
    let now = utc_millis().unwrap();
    let admitted = lease(
        &topology,
        "recorded-cancel-lease",
        vec![cpu],
        now + 30_000,
        now,
    );
    let dir = s.path().join("recorded-child");
    std::fs::create_dir(&dir).unwrap();
    let mut request = match spec(FixtureMode::Timeout).request {
        Request::SoftwareFixture { request } => request,
        _ => unreachable!(),
    };
    request.executable = digest;
    std::fs::write(
        dir.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let mut child =
        OwnedProcess::spawn_fixture(&executable, &dir, &admitted, reserve(), 1, 1, now).unwrap();
    let identity = child.identity().unwrap();
    let mut altered = serde_json::to_value(&identity).unwrap();
    altered["start_ticks"] = serde_json::json!(altered["start_ticks"].as_u64().unwrap() + 1);
    let altered: ProcessIdentity = serde_json::from_value(altered).unwrap();
    assert!(altered
        .cancel_recorded_fixture(Duration::from_millis(10))
        .is_err());
    assert!(child.poll().unwrap().is_none());
    assert!(identity
        .cancel_recorded_fixture(Duration::from_millis(10))
        .unwrap());
    let start = std::time::Instant::now();
    while child.poll().unwrap().is_none() && start.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(child.poll().unwrap().is_some());
}
#[test]
fn an_actual_accounted_overrun_is_retained_and_halts_further_admission() {
    let _scratch = TestScratch::new();
    let (mut db, _) = context("overrun");
    let topology = fake_topology();
    let task = spec(FixtureMode::NumericalFailure);
    db.enqueue("overrun", &task).unwrap();
    let l = lease(&topology, "overrun-lease", vec![0], 100, 1);
    let a = db.lease("overrun", "overrun-a", &l, 2).unwrap();
    db.start(&a, 3).unwrap();
    db.charge(&a, &synthetic_charge(task.reserve.cpu_ms + 1), 4)
        .unwrap();
    db.fail(&a, Phase::NumericalFailure, 5).unwrap();
    assert!(db.accounting().unwrap().exceeded);
    db.enqueue("next", &spec(FixtureMode::Succeed)).unwrap();
    let l = lease(&topology, "next-lease", vec![2], 100, 1);
    assert!(matches!(
        db.lease("next", "next-a", &l, 6),
        Err(RuntimeError::Budget(_))
    ));
    assert_eq!(db.accounting().unwrap().spent.cpu_ms, 1001);
}

#[test]
fn a_physical_request_remains_unsupported_and_cannot_spawn_a_fixture_as_a_solver() {
    let _scratch = TestScratch::new();
    let (mut db, s) = context("unsupported-physical");
    let topology = Topology::system().unwrap();
    let cpu = *topology.allowed.iter().next().unwrap();
    let binary = Path::new(env!("CARGO_BIN_EXE_runtime-fixture"));
    let digest = BlobDigest::of_bytes(&std::fs::read(binary).unwrap());
    let executable = FixtureExecutable::from_operator_binary(binary, &digest).unwrap();
    let mut task = spec(FixtureMode::Succeed);
    task.request = Request::Physical {
        key: Box::new(
            serde_json::from_str(include_str!(
                "../../../schemas/v5/fixtures/valid-evaluation-key.json"
            ))
            .unwrap(),
        ),
    };
    db.enqueue("physical", &task).unwrap();
    let now = utc_millis().unwrap();
    let admitted = lease(
        &topology,
        "physical-unsupported-lease",
        vec![cpu],
        now + 30_000,
        now,
    );
    let a = db
        .lease("physical", "physical-attempt", &admitted, now)
        .unwrap();
    let result = execute_fixture(
        &mut db,
        &s,
        &a,
        &admitted,
        &executable,
        Duration::from_secs(1),
    )
    .unwrap();
    assert_eq!(result.phase, Phase::Unsupported);
    assert!(matches!(
        result.outcome,
        crucible_contract::outcomes::TaskOutcome::Unsupported
    ));
    assert!(db.process_identity(&a.id).unwrap().is_none());
    assert!(db.task("physical").unwrap().accepted_object.is_none());
    assert_eq!(db.accounting().unwrap().charges, 1);
}
#[test]
fn fixture_self_deadline_survives_a_missing_coordinator_poll_and_faults_do_not_change_observation_identity(
) {
    let _scratch = TestScratch::new();
    let (_, s) = context("self-deadline");
    let topology = Topology::system().unwrap();
    let cpu = *topology.allowed.iter().next().unwrap();
    let binary = Path::new(env!("CARGO_BIN_EXE_runtime-fixture"));
    let digest = BlobDigest::of_bytes(&std::fs::read(binary).unwrap());
    let executable = FixtureExecutable::from_operator_binary(binary, &digest).unwrap();
    let now = utc_millis().unwrap();
    let admitted = lease(&topology, "self-deadline-lease", vec![cpu], now + 500, now);
    let dir = s.path().join("self-deadline-child");
    std::fs::create_dir(&dir).unwrap();
    let mut request = match spec(FixtureMode::Timeout).request {
        Request::SoftwareFixture { request } => request,
        _ => unreachable!(),
    };
    request.executable = digest;
    let failing = Request::SoftwareFixture {
        request: request.clone(),
    }
    .identity()
    .unwrap();
    let mut nominal = request.clone();
    nominal.mode = FixtureMode::Succeed;
    assert_eq!(
        failing,
        Request::SoftwareFixture { request: nominal }
            .identity()
            .unwrap()
    );
    std::fs::write(
        dir.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let mut child =
        OwnedProcess::spawn_fixture(&executable, &dir, &admitted, reserve(), 1, 1, now).unwrap();
    let start = std::time::Instant::now();
    let exit = loop {
        if let Some(exit) = child.poll().unwrap() {
            break exit;
        }
        assert!(start.elapsed() < Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(exit.status.code(), Some(124));
}
