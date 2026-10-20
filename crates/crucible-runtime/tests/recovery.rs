#[path = "../../crucible-contract/tests/common/scratch.rs"]
mod scratch;
use scratch::TestScratch;
#[path = "../../crucible-contract/tests/common/mod.rs"]
mod review_fixture;
use crucible_runtime::{checkpoint::*, db::RuntimeDb, lease::*, ledger::*, replay::*, task::*, *};
use crucible_store::{
    manifest::{Metadata, ObjectKind},
    store::Limits,
    Store,
};
use joil_evidence::{decision::*, identity::ContentRef, observation::Finite};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::Duration,
};
static NATIVE: Mutex<()> = Mutex::new(());
fn path(label: &str) -> PathBuf {
    TestScratch::path(&format!("v5-recovery-{label}"))
}
fn budget() -> Resources {
    Resources {
        cpu_ms: 120_000,
        gpu_ms: 0,
        ram_bytes: 512 * 1024 * 1024,
        vram_bytes: 0,
        scratch_bytes: 64 * 1024 * 1024,
        io_bytes: 128 * 1024 * 1024,
    }
}
fn reserve() -> Resources {
    Resources {
        cpu_ms: 2000,
        ram_bytes: 128 * 1024 * 1024,
        scratch_bytes: 4 * 1024 * 1024,
        io_bytes: 4 * 1024 * 1024,
        ..Resources::default()
    }
}
fn store(path: &Path) -> Store {
    Store::with_limits(
        path,
        Limits {
            object_bytes: 64 * 1024 * 1024,
            closure_bytes: 128 * 1024 * 1024,
            closure_objects: 4096,
        },
    )
    .unwrap()
}
fn context(label: &str) -> (RuntimeDb, Store, Experiment, SearchState) {
    let s = store(&path(label));
    let dependencies = [
        "solver",
        "material",
        "interface",
        "requirements",
        "recipe",
        "config",
        "executable",
    ]
    .into_iter()
    .map(|n| {
        (
            n.into(),
            s.publish(
                format!("synthetic source-bound {n}; no physical evidence").as_bytes(),
                Metadata::private(ObjectKind::Input),
            )
            .unwrap(),
        )
    })
    .collect();
    let e = Experiment { dependencies };
    let id = e.identity().unwrap();
    let db = RuntimeDb::open(s.path(), budget(), &id).unwrap();
    let state = SearchState::new(id, vec![[3, 4], [5, 12]], [1, 0]).unwrap();
    (db, s, e, state)
}
fn lease(name: &str) -> AdmittedLease {
    let topology = Topology::system().unwrap();
    let cpu = *topology.allowed.iter().next().unwrap();
    let now = utc_millis().unwrap();
    let source = BlobDigest::of_bytes(b"synthetic test operator authorization");
    let host = BlobDigest::of_bytes(b"private recovery test realm");
    let authority = Authorization {
        host: host.clone(),
        cpus: topology.allowed.clone(),
        expires_ms: now + 30_000,
        limits: budget(),
        gpu_authorized: false,
        source: source.clone(),
    };
    AdmittedLease::from_operator_config(
        ResourceLease {
            id: name.into(),
            host,
            cpus: vec![cpu],
            expires_ms: now + 30_000,
            limits: budget(),
            gpu_authorized: false,
            authority: source,
        },
        &authority,
        &topology,
        now,
        &BTreeSet::new(),
    )
    .unwrap()
}
fn execute(db: &mut RuntimeDb, s: &Store, t: &PlannedTask, binary: &Path, digest: &BlobDigest) {
    db.enqueue(&t.id, &t.spec).unwrap();
    let l = lease(&format!("lease-{}", t.id));
    let a = db
        .lease(
            &t.id,
            &format!("attempt-{}", t.id),
            &l,
            utc_millis().unwrap(),
        )
        .unwrap();
    let x = FixtureExecutable::from_operator_binary(binary, digest).unwrap();
    let result = execute_fixture(db, s, &a, &l, &x, Duration::from_secs(1)).unwrap();
    assert_eq!(result.phase, Phase::Committed);
}
fn collect(db: &RuntimeDb, s: &Store, state: &mut SearchState, name: &str) {
    let row = db.task(name).unwrap();
    let output = row.accepted_object.unwrap();
    let observation: FixtureObservation =
        serde_json::from_slice(&s.read(&output).unwrap().payload).unwrap();
    state.collect(name, output, observation).unwrap();
}
fn reserve_family(db: &mut RuntimeDb) {
    let mut review = review_fixture::ReviewFixture::new();
    let r = review_fixture::reference("synthetic-dependency");
    let policy = FamilyPolicy {
        experiment_family_id: "fixture-recovery-family".into(),
        requirements: review.requirements.clone(),
        family_alpha: Finite::new(0.05).unwrap(),
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
    let source = ContentRef::from_bytes("synthetic-recovery-policy", "1", &bytes);
    review.approve(source.clone(), bytes);
    let admitted =
        ApprovedFamily::from_registry(&review.registry, source, &review.requirements).unwrap();
    db.reserve_family_attempt(
        &admitted,
        &review.registry,
        1,
        &SemanticId::of(&"synthetic-decision").unwrap(),
    )
    .unwrap();
}
#[test]
fn complete_nonempty_checkpoint_restores_real_results_and_detects_missing_state() {
    let _scratch = TestScratch::new();
    let _native = NATIVE.lock().unwrap();
    let (db, s, mut e, _) = context("complete");
    // Freeze the actual producer, not a claimed executable label.
    let binary = Path::new(env!("CARGO_BIN_EXE_runtime-fixture"));
    let bytes = std::fs::read(binary).unwrap();
    let digest = BlobDigest::of_bytes(&bytes);
    e.dependencies.insert(
        "executable".into(),
        s.publish(&bytes, Metadata::private(ObjectKind::Input))
            .unwrap(),
    );
    drop(db);
    let mut db = RuntimeDb::open(
        &s.path().join("canonical-native"),
        budget(),
        &e.identity().unwrap(),
    )
    .unwrap();
    let mut state = SearchState::new(e.identity().unwrap(), vec![[3, 4], [5, 12]], [1, 0]).unwrap();
    state
        .prepare_epoch(&digest, reserve(), FixtureMode::Succeed)
        .unwrap();
    for t in state.in_flight.values().cloned().collect::<Vec<_>>() {
        execute(&mut db, &s, &t, binary, &digest);
        collect(&db, &s, &mut state, &t.id);
    }
    state.apply_epoch().unwrap();
    let scores = state
        .archives
        .observations
        .values()
        .map(|r| r.observation.score)
        .collect::<BTreeSet<_>>();
    assert_eq!(scores, BTreeSet::from([25, 169])); // Independent arithmetic oracle, not synthetic_score().
    reserve_family(&mut db);
    state
        .prepare_epoch(&digest, reserve(), FixtureMode::Succeed)
        .unwrap();
    let jobs = state.in_flight.values().cloned().collect::<Vec<_>>();
    for t in &jobs {
        db.enqueue(&t.id, &t.spec).unwrap();
    }
    execute(&mut db, &s, &jobs[0], binary, &digest);
    collect(&db, &s, &mut state, &jobs[0].id);
    assert_eq!(state.pending_results.len(), 1);
    assert_eq!(state.in_flight.len(), 1);
    assert!(!state.archives.best.is_empty());
    let first = db.publish_checkpoint(&s, &e, &state).unwrap();
    let cp = verify_checkpoint(&s, &first.object).unwrap();
    assert_eq!(cp.state, state);
    assert_eq!(cp.ledger.tables["family_decisions"].rows.len(), 1);
    for missing in ["covariance", "pending_results", "family_attempt_ledger"] {
        let mut raw = serde_json::to_value(&cp).unwrap();
        match missing {
            "covariance" => {
                raw["state"]["operator"]
                    .as_object_mut()
                    .unwrap()
                    .remove("covariance");
            }
            "pending_results" => {
                raw["state"]
                    .as_object_mut()
                    .unwrap()
                    .remove("pending_results");
            }
            _ => {
                raw["ledger"]["tables"]
                    .as_object_mut()
                    .unwrap()
                    .remove("family_decisions");
            }
        }
        assert!(
            serde_json::from_value::<Checkpoint>(raw)
                .map(|c| c.check().is_err())
                .unwrap_or(true),
            "{missing}"
        );
    }
    // A committed result newer than the checkpoint is rehydrated; every charge/family attempt survives.
    execute(&mut db, &s, &jobs[1], binary, &digest);
    let spent = db.accounting().unwrap();
    let mut restored = db.restore_latest(&s, &e).unwrap().checkpoint.state;
    db.reconcile(&s, &mut restored).unwrap();
    restored.apply_epoch().unwrap();
    assert_eq!(restored.completed_epochs, 2);
    assert_eq!(db.accounting().unwrap(), spent);
    assert_eq!(
        db.ledger_snapshot().unwrap().tables["family_decisions"].rows,
        cp.ledger.tables["family_decisions"].rows
    );
    let fork = restored.fork([-1, 1]).unwrap();
    assert_eq!(fork.archives.observations, restored.archives.observations);
    assert_ne!(fork.lineage.decisions, restored.lineage.decisions);
    db.publish_checkpoint(&s, &e, &fork).unwrap();
    assert_eq!(db.accounting().unwrap(), spent);
    assert!(db.checkpoint_history().unwrap().len() >= 2);
    // Revoke only the acceptance verification; the output bytes themselves remain intact.
    let ledger = db.ledger_snapshot().unwrap();
    let refs = &ledger.tables["object_refs"];
    let ai = refs.columns.iter().position(|c| c == "attempt").unwrap();
    let vi = refs
        .columns
        .iter()
        .position(|c| c == "verification")
        .unwrap();
    let row = refs
        .rows
        .iter()
        .find(|r| r[ai].as_str() == Some(&format!("attempt-{}", jobs[1].id)))
        .unwrap();
    let verification = BlobDigest::parse(row[vi].as_str().unwrap()).unwrap();
    s.invalidate(
        verification,
        crucible_store::graph::InvalidationKind::Revoked,
        "synthetic acceptance-revocation negative control".into(),
    )
    .unwrap();
    let mut old_state = cp.state;
    assert!(
        db.reconcile(&s, &mut old_state).is_err(),
        "a current output cannot bypass its revoked acceptance record"
    );
    assert_eq!(db.accounting().unwrap(), spent);
}
#[test]
fn checkpoint_kill_child() {
    let _scratch = TestScratch::new();
    let Ok(root) = std::env::var("JOIL_RECOVERY_TEST_ROOT") else {
        return;
    };
    let s = store(Path::new(&root));
    let mut db = RuntimeDb::open(
        s.path(),
        budget(),
        &serde_json::from_slice::<Experiment>(
            &std::fs::read(s.path().join("experiment.json")).unwrap(),
        )
        .unwrap()
        .identity()
        .unwrap(),
    )
    .unwrap();
    let e: Experiment =
        serde_json::from_slice(&std::fs::read(s.path().join("experiment.json")).unwrap()).unwrap();
    let mut state = db.restore_latest(&s, &e).unwrap().checkpoint.state;
    state
        .archives
        .audit
        .push("synthetic checkpoint crash control".into());
    let phase = std::env::var("JOIL_RECOVERY_TEST_PHASE").unwrap();
    db.publish_checkpoint_checked(&s, &e, &state, |p| {
        if format!("{p:?}") == phase {
            unsafe {
                libc::raise(libc::SIGKILL);
            }
        }
    })
    .unwrap();
    panic!("expected actual owned kill");
}
#[test]
fn actual_kills_on_both_sides_of_rename_and_pointer_commit_preserve_verified_history() {
    let _scratch = TestScratch::new();
    use std::os::unix::process::ExitStatusExt;
    let (mut db, s, e, state) = context("kills");
    db.publish_checkpoint(&s, &e, &state).unwrap();
    std::fs::write(
        s.path().join("experiment.json"),
        serde_json::to_vec(&e).unwrap(),
    )
    .unwrap();
    for phase in [
        CheckpointPhase::BeforeRename,
        CheckpointPhase::AfterRename,
        CheckpointPhase::BeforePointerCommit,
        CheckpointPhase::AfterPointerCommit,
    ] {
        let before = db.checkpoint_history().unwrap()[0].generation;
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "checkpoint_kill_child", "--nocapture"])
            .env("JOIL_RECOVERY_TEST_ROOT", s.path())
            .env("JOIL_RECOVERY_TEST_PHASE", format!("{phase:?}"))
            .status()
            .unwrap();
        assert_eq!(status.signal(), Some(libc::SIGKILL));
        let recovered = db.restore_latest(&s, &e).unwrap();
        assert_eq!(
            recovered.pointer.generation,
            before + u64::from(phase == CheckpointPhase::AfterPointerCommit)
        );
        verify_checkpoint(&s, &recovered.pointer.object).unwrap();
        quarantine_staging(&s).unwrap();
        db.publish_checkpoint(&s, &e, &recovered.checkpoint.state)
            .unwrap();
    }
    assert!(db.checkpoint_history().unwrap().len() >= 2);
    assert!(!std::fs::read_dir(s.path().join("quarantined-staging"))
        .unwrap()
        .collect::<Vec<_>>()
        .is_empty());
}
#[test]
fn corrupt_latest_falls_back_with_reason_then_can_publish_without_referencing_bad_root() {
    let _scratch = TestScratch::new();
    let (mut db, s, e, mut state) = context("corrupt");
    let old = db.publish_checkpoint(&s, &e, &state).unwrap();
    state.archives.audit.push("new generation".into());
    let latest = db.publish_checkpoint(&s, &e, &state).unwrap();
    std::fs::write(
        s.path()
            .join("objects")
            .join(latest.object.as_str())
            .join("manifest.json"),
        b"truncated negative-control manifest",
    )
    .unwrap();
    let fallback = db.restore_latest(&s, &e).unwrap();
    assert_eq!(fallback.pointer.object, old.object);
    assert_eq!(fallback.rejected_newer.len(), 1);
    assert_eq!(fallback.rejected_newer[0].0, latest.object);
    db.quarantine_checkpoint_pointers(&fallback.rejected_newer)
        .unwrap();
    let next = db
        .publish_checkpoint(&s, &e, &fallback.checkpoint.state)
        .unwrap();
    assert!(next.generation > latest.generation);
    verify_checkpoint(&s, &next.object).unwrap();
}
fn synthetic_epoch(state: &mut SearchState, exe: &BlobDigest, reverse: bool) {
    state
        .prepare_epoch(exe, reserve(), FixtureMode::Succeed)
        .unwrap();
    let mut jobs = state.in_flight.values().cloned().collect::<Vec<_>>();
    if reverse {
        jobs.reverse();
    }
    for t in jobs {
        let Request::SoftwareFixture { request } = &t.spec.request else {
            panic!()
        };
        let score = request.values.iter().map(|v| v * v).sum();
        let observation = FixtureObservation {
            schema_version: 1,
            synthetic_only: true,
            request: t.spec.request.identity().unwrap(),
            score,
            address: request.address.clone(),
        };
        let output = BlobDigest::of_bytes(&serde_json::to_vec(&observation).unwrap());
        state.collect(&t.id, output, observation).unwrap();
    }
    state.apply_epoch().unwrap();
}
#[test]
fn addressed_rng_covariance_and_due_order_survive_shuffled_completion() {
    let _scratch = TestScratch::new();
    let (_, _, e, state) = context("logical");
    let exe =
        BlobDigest::of_bytes(b"explicit synthetic producer identity, no process run in this test");
    let mut a = state.clone();
    let mut b = state;
    for _ in 0..3 {
        synthetic_epoch(&mut a, &exe, false);
        synthetic_epoch(&mut b, &exe, true);
    }
    assert_ne!(a.arrival_order, b.arrival_order);
    assert_eq!(a.logical_digest().unwrap(), b.logical_digest().unwrap());
    assert!(!a.rng_counters.is_empty());
    assert_eq!(a.operator.applications, 3);
    // Force a nonzero addressed draw so a covariance change must alter actual proposals.
    let draw = (0..100)
        .find(|draw| {
            addressed_draw(
                &RngAddress {
                    campaign: a.campaign.clone(),
                    island: 0,
                    epoch: a.completed_epochs + 1,
                    proposal: a.next_ordinal,
                    operator: "correlated-integer/1".into(),
                    draw: *draw,
                },
                0,
            )
            .unwrap()
                != 0
        })
        .unwrap();
    a.rng_counters.insert("0/correlated-integer".into(), draw);
    let mut changed = a.clone();
    changed.operator.covariance[0][0] += 2;
    a.prepare_epoch(&exe, reserve(), FixtureMode::Succeed)
        .unwrap();
    changed
        .prepare_epoch(&exe, reserve(), FixtureMode::Succeed)
        .unwrap();
    let proposal = |s: &SearchState| {
        s.in_flight.values().find(|t| matches!(&t.spec.request,Request::SoftwareFixture{request} if request.address.island==0)).unwrap().genome.values
    };
    assert_ne!(proposal(&a), proposal(&changed));
    assert_ne!(
        a.logical_digest().unwrap(),
        changed.logical_digest().unwrap()
    );
    assert_eq!(a.campaign, e.identity().unwrap());
}
#[test]
fn immutable_drift_refuses_resume_fork_reuses_observations_seed_import_discards_pass() {
    let _scratch = TestScratch::new();
    let (_, _, e, mut state) = context("identity");
    let exe = BlobDigest::of_bytes(b"synthetic only");
    synthetic_epoch(&mut state, &exe, false);
    e.require_resume(&e).unwrap();
    for name in [
        "solver",
        "material",
        "interface",
        "requirements",
        "recipe",
        "config",
        "executable",
    ] {
        let mut changed = e.clone();
        changed.dependencies.insert(
            name.into(),
            BlobDigest::of_bytes(format!("changed {name}").as_bytes()),
        );
        assert!(e.require_resume(&changed).is_err());
    }
    let next = state.fork([-1, 1]).unwrap();
    assert_eq!(next.archives.observations, state.archives.observations);
    assert_ne!(next.lineage.id, state.lineage.id);
    assert_ne!(next.lineage.decisions, state.lineage.decisions);
    let seed =
        import_seed(br#"{"values":[3,4],"compiled":"forged","verdict":"PASS","eligible":true}"#)
            .unwrap();
    assert_eq!(seed, ToyGenome::compile([3, 4]).unwrap());
    assert!(!serde_json::to_string(&seed).unwrap().contains("PASS"));
    assert!(import_seed(br#"{"values":[101,4]}"#).is_err());
    assert!(import_seed(br#"{"values":[3,4],"values":[1,2]}"#).is_err());
}
#[test]
fn unaccounted_crashed_attempt_holds_resources_and_blocks_redispatch() {
    let _scratch = TestScratch::new();
    let (mut db, s, e, mut state) = context("unknown-cost");
    state
        .prepare_epoch(
            &BlobDigest::of_bytes(b"synthetic no executable launched"),
            reserve(),
            FixtureMode::Succeed,
        )
        .unwrap();
    db.publish_checkpoint(&s, &e, &state).unwrap();
    db.reconcile(&s, &mut state).unwrap();
    let job = state.in_flight.values().next().unwrap().clone();
    let l = lease("unknown-cost-lease");
    let a = db
        .lease(&job.id, "unaccounted-attempt", &l, utc_millis().unwrap())
        .unwrap();
    db.start(&a, utc_millis().unwrap()).unwrap();
    let before = db.accounting().unwrap();
    assert_ne!(before.reserved, Resources::default());
    assert!(matches!(
        db.reconcile(&s, &mut state),
        Err(RuntimeError::Conflict("crash_accounting_unresolved"))
    ));
    assert_eq!(before, db.accounting().unwrap());
    assert!(db.retry_ready(&job.id).is_err());
    let snapshot = db.ledger_snapshot().unwrap();
    let t = &snapshot.tables["events"];
    assert!(serde_json::to_string(t)
        .unwrap()
        .contains("crash_accounting_unresolved_reservation_held"));
}
#[test]
fn cost_digest_preserves_charge_data_and_family_ledger_reset_is_refused() {
    let _scratch = TestScratch::new();
    let (mut db, _, _, _) = context("cost");
    reserve_family(&mut db);
    let a = db.ledger_snapshot().unwrap();
    let mut missing = a.clone();
    missing.tables.remove("family_decisions");
    assert!(missing.check().is_err());
    let mut reset = a.clone();
    reset
        .tables
        .get_mut("family_decisions")
        .unwrap()
        .rows
        .clear();
    assert!(a.require_extension(&reset).is_err());
    let mut changed = a.clone();
    changed.accounting.spent.cpu_ms += 1;
    assert_ne!(
        a.actual_cost_digest().unwrap(),
        changed.actual_cost_digest().unwrap()
    );
    // Independent numeric control: different measured wall time cannot be erased by logical replay.
    let t = TableSnapshot {
        columns: vec!["attempt".into(), "charge".into()],
        rows: vec![vec![
            serde_json::json!("one"),
            serde_json::json!("{\"wall_ns\":17}"),
        ]],
    };
    let mut b = a.clone();
    b.tables.insert("charges".into(), t.clone());
    let mut c = b.clone();
    c.tables.get_mut("charges").unwrap().rows[0][1] = serde_json::json!("{\"wall_ns\":29}");
    assert_ne!(
        b.actual_cost_digest().unwrap(),
        c.actual_cost_digest().unwrap()
    );
}

#[test]
fn charged_preparation_does_not_consume_native_deadline_and_real_slow_child_still_times_out() {
    let _scratch = TestScratch::new();
    use std::os::unix::fs::PermissionsExt;
    let _native = NATIVE.lock().unwrap();
    let (mut db, s, _, state) = context("execution-clock");
    let binary = Path::new(env!("CARGO_BIN_EXE_runtime-fixture"));
    let frozen = BlobDigest::of_bytes(&std::fs::read(binary).unwrap());
    // Leave room for interpreter startup and durable fixture writes on a shared
    // host. The ordering is the oracle: preparation exceeds the native deadline,
    // while the slow child independently exceeds that same deadline.
    let native_deadline = Duration::from_secs(2);
    let preparation_delay = Duration::from_secs(3);
    for (name, delay, expected) in [
        ("fast", 0.08, Phase::Committed),
        ("slow", 4.0, Phase::TimedOut),
    ] {
        // Trusted, single-PID software fixture: Python sleeps then execs the byte-bound Rust fixture.
        // Verify the Rust producer in charged preparation, not again inside the native deadline.
        // No subprocess/model/GPU is invoked by this test producer.
        let script = s.path().join(format!("{name}.py"));
        let body=format!("#!/usr/bin/python3\nimport os,sys,time\np={}\n# Coordinator verifies producer sha256 {} before spawn.\ntime.sleep({delay})\nos.execv(p,[p]+sys.argv[1:])\n",serde_json::to_string(&binary.to_str().unwrap()).unwrap(),frozen.as_str());
        std::fs::write(&script, body).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        let digest = BlobDigest::of_bytes(&std::fs::read(&script).unwrap());
        let executable = FixtureExecutable::from_operator_binary(&script, &digest).unwrap();
        let mut proposal = state.clone();
        proposal
            .prepare_epoch(&digest, reserve(), FixtureMode::Succeed)
            .unwrap();
        let mut job = proposal.in_flight.values().next().unwrap().clone();
        job.id = format!("{name}-execution-clock");
        db.enqueue(&job.id, &job.spec).unwrap();
        let l = lease(&format!("{name}-lease"));
        let a = db
            .lease(
                &job.id,
                &format!("{name}-attempt"),
                &l,
                utc_millis().unwrap(),
            )
            .unwrap();
        let result = execute_fixture_checked(
            &mut db,
            &s,
            &a,
            &l,
            &executable,
            native_deadline,
            || {
                FixtureExecutable::from_operator_binary(binary, &frozen).unwrap();
                std::thread::sleep(preparation_delay);
            },
        )
        .unwrap();
        assert_eq!(
            result.phase, expected,
            "preparation is charged but not native execution time"
        );
        assert!(u128::from(result.charge.wall_ns) >= preparation_delay.as_nanos());
        assert!(result.charge.measured.cpu_ms > 0);
        if expected == Phase::Committed {
            let observation: FixtureObservation =
                serde_json::from_slice(&s.read(result.object.as_ref().unwrap()).unwrap().payload)
                    .unwrap();
            assert_eq!(observation.score, 25);
        } else {
            assert!(db.task(&job.id).unwrap().accepted_object.is_none());
        }
    }
    assert_eq!(db.accounting().unwrap().charges, 2);
    assert_eq!(db.accounting().unwrap().reserved, Resources::default());
}
