use crucible_search::archive::{Candidate, EvidenceClass, SearchArchive};
use crucible_search::emitters::{
    EmitterState, FixedDimensionBo, FixedDimensionCmaEs, ParentSource,
};
use crucible_search::reference::{
    constrained_dominates, hypervolume_2d, AttemptKind, Checkpoint, CostCurve, CostEvent, Policy,
    POLICY_TOML,
};

fn candidate(
    proposal_id: u64,
    class: EvidenceClass,
    descriptors: Vec<f64>,
    objectives: Vec<f64>,
    feasible: bool,
) -> Candidate {
    Candidate {
        proposal_id,
        class,
        descriptors,
        objectives,
        feasible,
    }
}

#[test]
fn v5_21_01_to_05_sc_dec_038_reference_search_policy_archive_controls_and_independent_fixture() {
    // V5-21-01: executable frozen policy, fixed quotas/shares, stable ties, and bounded attempts.
    let policy = Policy::reference_v1().expect("reference-search/1 policy parses and validates");
    assert!(POLICY_TOML.contains("policy_id = \"reference-search/1\""));
    assert_eq!(
        (
            policy.islands,
            policy.population_per_island,
            policy.attempted_offspring_per_island_epoch,
            policy.migration_interval_epochs
        ),
        (4, 32, 32, 10)
    );
    assert_eq!(policy.initialization_attempt_limit_per_slot, 8);
    assert_eq!(policy.refill_attempt_limit_per_epoch_slot, 2);
    assert_eq!(policy.archive_capacity_per_pool, 256);
    assert_eq!(policy.nondominated_capacity_per_descriptor_bin, 8);
    assert_eq!(
        [
            policy.operator_shares.mutation,
            policy.operator_shares.crossover,
            policy.operator_shares.repair,
            policy.operator_shares.novelty,
            policy.operator_shares.local_search,
            policy.operator_shares.restart
        ],
        [40, 20, 10, 10, 10, 10]
    );
    assert_eq!(
        [
            policy.parent_source_shares.qualified,
            policy.parent_source_shares.screen,
            policy.parent_source_shares.exploration
        ],
        [50, 30, 20]
    );
    let mut state = Checkpoint::new(7, 128, &policy);
    state.charge_epoch(&policy).unwrap();
    assert_eq!(state.budget.charged, 4 * 32);
    assert_eq!(state.budget.records.len(), 128);
    assert!(state
        .budget
        .records
        .iter()
        .all(|r| r.kind == AttemptKind::Offspring));
    let mut refill = Checkpoint::new(8, 2, &policy);
    let mut calls = 0;
    assert_eq!(
        refill
            .bounded_refill(&policy, 0, 0, |_| {
                calls += 1;
                false
            })
            .unwrap(),
        None
    );
    assert_eq!(calls, 2);
    assert_eq!(refill.budget.charged, 2); // failed/inapplicable attempts are charged, with no hidden retry.
    let mut changed = policy.clone();
    changed.operator_shares.restart += 1;
    assert!(changed.validate().is_err());

    // V5-21-02: evidence classes have separate pools; unknown/failure/unsupported cannot dominate.
    let cfg = policy.archive_config(vec![0.5, 0.5]);
    let mut archive = SearchArchive::default();
    archive.insert(
        candidate(
            1,
            EvidenceClass::Qualified,
            vec![0.2, 0.4],
            vec![2.0, 3.0],
            true,
        ),
        0,
        &cfg,
    );
    archive.insert(
        candidate(
            2,
            EvidenceClass::Unknown,
            vec![0.2, 0.4],
            vec![-99.0, -99.0],
            true,
        ),
        0,
        &cfg,
    );
    archive.insert(
        candidate(
            3,
            EvidenceClass::NumericalFailure,
            vec![0.2, 0.4],
            vec![-100.0, -100.0],
            true,
        ),
        0,
        &cfg,
    );
    archive.insert(
        candidate(
            4,
            EvidenceClass::Unsupported,
            vec![0.2, 0.4],
            vec![-101.0, -101.0],
            true,
        ),
        0,
        &cfg,
    );
    archive.insert(
        candidate(5, EvidenceClass::Exploration, vec![], vec![8.0], true),
        0,
        &cfg,
    );
    archive.insert(
        candidate(6, EvidenceClass::Screen, vec![], vec![6.0], true),
        0,
        &cfg,
    );
    archive.insert(
        candidate(9, EvidenceClass::Screen, vec![], vec![5.0], true),
        0,
        &cfg,
    );
    archive.insert(
        candidate(8, EvidenceClass::Screen, vec![], vec![5.0], true),
        0,
        &cfg,
    );
    assert_eq!(
        archive
            .qualified
            .iter()
            .map(|c| c.proposal_id)
            .collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        archive
            .exploration
            .iter()
            .map(|c| c.proposal_id)
            .collect::<Vec<_>>(),
        [5]
    );
    assert_eq!(
        archive
            .screen
            .iter()
            .map(|c| c.proposal_id)
            .collect::<Vec<_>>(),
        [8, 9]
    );
    assert_eq!(archive.evidence_front.len(), 8); // durable history retains all classes and dominated ties.
    assert!(!constrained_dominates(
        &[-99.0, -99.0],
        1.0,
        &[2.0, 3.0],
        0.0
    ));
    assert!(archive
        .qualified
        .iter()
        .all(|c| c.descriptors.len() == cfg.descriptor_bin_widths.len()));

    // V5-21-03: fixed schedules, available-source fallback, 10-epoch ring migration, protection.
    assert_eq!(
        EmitterState::scheduled_source(0, [false, true, false]),
        Some(ParentSource::Screen)
    );
    assert_eq!(
        EmitterState::scheduled_source(0, [false, false, false]),
        None
    );
    let mut emitter = EmitterState::default();
    assert!(emitter
        .migration_ring(9, &[vec![1], vec![2], vec![3], vec![4]])
        .is_empty());
    assert_eq!(
        emitter.migration_ring(10, &[vec![1], vec![2], vec![3], vec![4]]),
        [(0, 1, 1), (1, 2, 2), (2, 3, 3), (3, 0, 4)]
    );
    archive.insert(
        candidate(
            7,
            EvidenceClass::Qualified,
            vec![0.1, 0.1],
            vec![1.0, 2.0],
            true,
        ),
        5,
        &cfg,
    );
    assert_eq!(archive.protected_until.get(&7), Some(&15));
    archive.expire_protection(15);
    assert!(!archive.protected_until.contains_key(&7));

    // V5-21-04: random, CMA-ES and BO share representation/budget accounting; no hidden work.
    let mut cma = FixedDimensionCmaEs::new(2, 4, 11).unwrap();
    let points = cma.ask();
    assert_eq!(points.len(), 4);
    assert!(points.iter().flatten().all(|x| (0.0..=1.0).contains(x)));
    let scores: Vec<_> = points
        .iter()
        .map(|p| (p[0] - 0.25).powi(2) + (p[1] - 0.75).powi(2))
        .collect();
    cma.tell(&points, &scores).unwrap();
    let mut bo = FixedDimensionBo::new(2).unwrap();
    bo.observe(vec![0.0, 0.0], 1.0).unwrap();
    bo.observe(vec![1.0, 1.0], 0.5).unwrap();
    assert_eq!(
        bo.select(&[vec![0.2, 0.2], vec![0.8, 0.8]]).unwrap().len(),
        2
    );
    let mut costs = CostCurve::default();
    for optimizer in ["reference", "random", "cma-es", "bo"] {
        for (kind, quality) in [
            (AttemptKind::Initialization, None),
            (AttemptKind::Offspring, None),
            (AttemptKind::Inference, None),
            (AttemptKind::Audit, None),
            (AttemptKind::Confirmation, None),
            (AttemptKind::Offspring, Some(0.4)),
        ] {
            costs.record(CostEvent {
                representation: "shared-grammar".into(),
                optimizer: optimizer.into(),
                kind,
                quality,
                charged_units: 1,
            });
        }
    }
    for optimizer in ["reference", "random", "cma-es", "bo"] {
        assert_eq!(
            costs.points("shared-grammar", optimizer).last().unwrap().0,
            6
        );
    }
    assert!(costs.same_representation_contrast("shared-grammar", "random", "reference"));
    assert!(!costs.same_representation_contrast("other-grammar", "random", "reference"));

    // V5-21-05: synthetic independent ZDT-style fixture and exact hand-computable front oracle.
    // The candidate optimizer is allowed to lose: only cost visibility and oracle correctness are asserted.
    let fixture: [[f64; 2]; 4] = [
        [0.0, 1.0],
        [0.25, 0.5],
        [0.5, 0.2928932188134524],
        [1.0, 0.0],
    ];
    for [f1, f2] in fixture {
        assert!((f2 - (1.0 - f1.sqrt())).abs() < 1e-12);
    }
    let exact_front = [[1.0, 4.0], [2.0, 2.0], [4.0, 1.0]];
    assert_eq!(hypervolume_2d(&exact_front, [5.0, 5.0]).unwrap(), 11.0);
    assert!(hypervolume_2d(&exact_front, [3.0, 3.0]).is_err());
    assert!(hypervolume_2d(&[[f64::NAN, 1.0]], [5.0, 5.0]).is_err());
    assert!(constrained_dominates(
        &[1.0, 1.0],
        0.0,
        &[100.0, 100.0],
        0.2
    ));
    assert!(!constrained_dominates(&[f64::NAN], 0.0, &[2.0], 0.0));
}
