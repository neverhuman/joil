use crucible_search::archive::{Candidate, EvidenceClass, SearchArchive};
use crucible_search::emitters::{
    EmitterState, FixedDimensionBo, FixedDimensionCmaEs, ParentSource, OPERATORS,
};
use crucible_search::reference::{
    constrained_dominates, hypervolume_2d, AttemptKind, Checkpoint, CostCurve, CostEvent, Policy,
};

fn candidate(
    id: u64,
    class: EvidenceClass,
    descriptor: f64,
    objectives: [f64; 2],
    feasible: bool,
) -> Candidate {
    Candidate {
        proposal_id: id,
        class,
        descriptors: vec![descriptor],
        objectives: objectives.to_vec(),
        feasible,
    }
}

#[test]
fn executable_policy_is_frozen_and_attempt_quotas_are_exact() {
    let policy = Policy::reference_v1().unwrap();
    assert_eq!(
        (
            policy.islands,
            policy.population_per_island,
            policy.attempted_offspring_per_island_epoch
        ),
        (4, 32, 32)
    );
    let mut state = Checkpoint::new(17, 128, &policy);
    state.charge_epoch(&policy).unwrap();
    assert_eq!(state.budget.charged, 128);
    assert_eq!(state.budget.records.len(), 128);
    assert!(state
        .budget
        .records
        .iter()
        .all(|r| r.kind == AttemptKind::Offspring));
    let counts: Vec<_> = state.emitters[0].operator_counts.into_iter().collect();
    assert_eq!(counts, [13, 7, 3, 3, 3, 3]);
    assert_eq!(
        state
            .emitters
            .iter()
            .map(|e| e.next_proposal_id)
            .sum::<u64>(),
        128
    );
}

#[test]
fn failed_and_inapplicable_operators_are_charged_without_hidden_resampling() {
    let p = Policy::reference_v1().unwrap();
    let mut s = Checkpoint::new(1, 5, &p);
    let mut calls = 0;
    let accepted = s
        .bounded_refill(&p, 0, 0, |_| {
            calls += 1;
            false
        })
        .unwrap();
    assert_eq!(accepted, None);
    assert_eq!(calls, p.refill_attempt_limit_per_epoch_slot);
    assert_eq!(s.budget.charged, calls as u64);
    assert_eq!(s.budget.records.len(), calls);
    assert!(s.budget.records.iter().all(|x| x.outcome == "attempted"));
}

#[test]
fn constrained_front_and_exact_hypervolume_match_independent_oracle() {
    let front = [[1.0, 4.0], [2.0, 2.0], [4.0, 1.0]];
    assert!(!constrained_dominates(&front[0], 0.0, &front[1], 0.0));
    assert!(constrained_dominates(&[3.0, 3.0], 0.0, &[4.0, 4.0], 0.0));
    assert!(constrained_dominates(
        &[100.0, 100.0],
        0.0,
        &[0.1, 0.1],
        0.2
    ));
    assert_eq!(hypervolume_2d(&front, [5.0, 5.0]).unwrap(), 11.0);
}

#[test]
fn evidence_classes_and_measured_qd_bins_do_not_cross_contaminate() {
    let cfg = Policy::reference_v1().unwrap().archive_config(vec![1.0]);
    let mut a = SearchArchive::default();
    a.insert(
        candidate(1, EvidenceClass::Qualified, 0.2, [2.0, 2.0], true),
        0,
        &cfg,
    );
    a.insert(
        candidate(2, EvidenceClass::Unknown, 0.2, [0.0, 0.0], true),
        0,
        &cfg,
    );
    a.insert(
        candidate(3, EvidenceClass::NumericalFailure, 0.2, [-1.0, -1.0], true),
        0,
        &cfg,
    );
    a.insert(
        candidate(4, EvidenceClass::Unsupported, 0.2, [-2.0, -2.0], true),
        0,
        &cfg,
    );
    assert_eq!(
        a.qualified
            .iter()
            .map(|x| x.proposal_id)
            .collect::<Vec<_>>(),
        [1]
    );
    assert!(a.screen.is_empty() && a.exploration.is_empty());
    assert_eq!(a.evidence_front.len(), 4);
    // A gene family string is absent from the qualified-cell API; only evaluator descriptors key bins.
    a.insert(
        candidate(5, EvidenceClass::Qualified, 1.2, [1.0, 1.0], true),
        1,
        &cfg,
    );
    assert_eq!(a.qualified.len(), 2);
}

#[test]
fn operator_shares_source_fallback_migration_and_protection_are_deterministic() {
    assert_eq!(OPERATORS.map(|x| x.1), [40, 20, 10, 10, 10, 10]);
    assert_eq!(
        EmitterState::scheduled_source(0, [false, true, false]),
        Some(ParentSource::Screen)
    );
    assert_eq!(
        EmitterState::scheduled_source(0, [false, false, false]),
        None
    );
    let mut e = EmitterState::default();
    assert!(e
        .migration_ring(9, &[vec![1], vec![2], vec![3], vec![4]])
        .is_empty());
    assert_eq!(
        e.migration_ring(10, &[vec![1], vec![2], vec![3], vec![4]]),
        [(0, 1, 1), (1, 2, 2), (2, 3, 3), (3, 0, 4)]
    );
    let p = Policy::reference_v1().unwrap();
    let cfg = p.archive_config(vec![1.0]);
    let mut a = SearchArchive::default();
    a.insert(
        candidate(1, EvidenceClass::Qualified, 0.2, [1.0, 1.0], true),
        5,
        &cfg,
    );
    assert_eq!(a.protected_until.get(&1), Some(&15));
    a.expire_protection(15);
    assert!(!a.protected_until.contains_key(&1));
}

#[test]
fn selection_checkpoint_and_all_cost_stages_are_serialized_and_charged() {
    let p = Policy::reference_v1().unwrap();
    let mut s = Checkpoint::new(100, 8, &p);
    let a = s.next_random();
    let serialized = serde_json::to_vec(&s).unwrap();
    let mut restored: Checkpoint = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(restored.next_random(), s.next_random());
    let next = s.next_random();
    assert_ne!(a, next);
    assert_eq!(next, restored.next_random());
    assert_eq!(
        s.random_control(|seed| seed),
        restored.random_control(|seed| seed)
    );
    assert_eq!(s.select_parent_source(0, [false, false, false]), None);
    restored.charge_stage(AttemptKind::Inference, "ok").unwrap();
    restored.charge_stage(AttemptKind::Audit, "failed").unwrap();
    restored
        .charge_stage(AttemptKind::Confirmation, "ok")
        .unwrap();
    assert_eq!(restored.budget.charged, 3);
    assert_eq!(
        restored
            .budget
            .records
            .iter()
            .map(|r| r.kind.clone())
            .collect::<Vec<_>>(),
        [
            AttemptKind::Inference,
            AttemptKind::Audit,
            AttemptKind::Confirmation
        ]
    );
}

#[test]
fn checkpoint_resume_continues_the_epoch_and_migrates_on_epoch_ten() {
    let p = Policy::reference_v1().unwrap();
    let mut s = Checkpoint::new(4, 128, &p);
    s.budget.limit = 20;
    assert_eq!(s.charge_epoch(&p), Err("attempt budget exhausted"));
    assert_eq!((s.epoch, s.epoch_progress, s.budget.charged), (0, 20, 20));
    let bytes = serde_json::to_vec(&s).unwrap();
    let mut resumed: Checkpoint = serde_json::from_slice(&bytes).unwrap();
    resumed.budget.limit = 128;
    resumed.charge_epoch(&p).unwrap();
    assert_eq!(
        (
            resumed.epoch,
            resumed.epoch_progress,
            resumed.budget.charged
        ),
        (1, 0, 128)
    );
    assert_eq!(
        resumed
            .budget
            .records
            .iter()
            .map(|x| x.proposal_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        128
    );
    resumed.epoch = 10;
    resumed.set_island_members(0, vec![10]);
    resumed.set_island_members(1, vec![20]);
    resumed.set_island_members(2, vec![30]);
    resumed.set_island_members(3, vec![40]);
    resumed.migrate_ring(32);
    assert_eq!(
        resumed.island_members,
        [vec![10, 40], vec![10, 20], vec![20, 30], vec![30, 40]]
    );
    assert_eq!(resumed.emitters[0].migration_events, 1);
}

#[test]
fn equal_objectives_keep_stable_proposal_id_order() {
    let p = Policy::reference_v1().unwrap();
    let cfg = p.archive_config(vec![1.0]);
    let mut a = SearchArchive::default();
    a.insert(
        candidate(9, EvidenceClass::Screen, 0.0, [1.0, 1.0], true),
        0,
        &cfg,
    );
    a.insert(
        candidate(3, EvidenceClass::Screen, 0.0, [1.0, 1.0], true),
        0,
        &cfg,
    );
    assert_eq!(
        a.screen.iter().map(|x| x.proposal_id).collect::<Vec<_>>(),
        [3, 9]
    );
}

#[test]
fn fixed_dimension_controls_are_deterministic_bounded_and_budget_visible() {
    let mut cma = FixedDimensionCmaEs::new(2, 8, 5).unwrap();
    let mut cma_replay = cma.clone();
    let points = cma.ask();
    assert_eq!(points, cma_replay.ask());
    assert!(points.iter().flatten().all(|x| (0.0..=1.0).contains(x)));
    let scores: Vec<_> = points
        .iter()
        .map(|x| (x[0] - 0.2).powi(2) + (x[1] - 0.7).powi(2))
        .collect();
    cma.tell(&points, &scores).unwrap();
    assert!(cma.best.is_some());
    let mut bo = FixedDimensionBo::new(2).unwrap();
    bo.observe(vec![0.0, 0.0], 1.0).unwrap();
    bo.observe(vec![1.0, 1.0], 0.5).unwrap();
    let selected = bo.select(&[vec![0.2, 0.2], vec![0.8, 0.8]]).unwrap();
    assert!(selected.iter().all(|x| (0.0..=1.0).contains(x)));
    let mut curve = CostCurve::default();
    for kind in [
        AttemptKind::Initialization,
        AttemptKind::Offspring,
        AttemptKind::Inference,
        AttemptKind::Audit,
        AttemptKind::Confirmation,
    ] {
        curve.record(CostEvent {
            representation: "shared-grammar".into(),
            optimizer: "cma-es".into(),
            kind,
            quality: None,
            charged_units: 1,
        });
    }
    curve.record(CostEvent {
        representation: "shared-grammar".into(),
        optimizer: "cma-es".into(),
        kind: AttemptKind::Offspring,
        quality: Some(0.4),
        charged_units: 1,
    });
    assert_eq!(
        curve.points("shared-grammar", "cma-es"),
        [
            (1, None),
            (2, None),
            (3, None),
            (4, None),
            (5, None),
            (6, Some(0.4))
        ]
    );
    assert_eq!(
        curve.points("shared-grammar", "cma-es").last().unwrap().0,
        6
    );
}
