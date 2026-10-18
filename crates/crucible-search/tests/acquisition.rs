use crucible_search::acquisition::{allocate, largest_remainder, Arm, Proposal};
use crucible_search::audit::{audit, verify_epoch, AuditPolicy, FrozenEpoch};
use crucible_search::surrogate::{
    CalibrationIdentity, CalibrationState, DatasetNamespace, Prediction,
};

#[test]
fn fixed_shares_have_deterministic_largest_remainder_and_no_duplicate_or_overcommit() {
    let shares = largest_remainder(
        7,
        &[
            (Arm::Improvement, 60),
            (Arm::Uncertainty, 20),
            (Arm::Diversity, 10),
            (Arm::Random, 10),
        ],
    )
    .unwrap();
    assert_eq!(shares.iter().map(|x| x.1).sum::<usize>(), 7);
    let candidates = vec![
        Proposal {
            id: 1,
            arm: Arm::Improvement,
            cost: 2,
        },
        Proposal {
            id: 1,
            arm: Arm::Improvement,
            cost: 1,
        },
        Proposal {
            id: 2,
            arm: Arm::Random,
            cost: 10,
        },
    ];
    let out = allocate(7, 100, 32, true, 2, &candidates).unwrap();
    assert_eq!(out.selected.len(), 1);
    assert_eq!(out.selected[0].id, 1);
    assert_eq!(out.charged_cost, 2);
    assert!(out.charged_cost <= 2);
}
#[test]
fn audit_budget_failure_suspends_pruning_even_with_elite_correlation() {
    let p = AuditPolicy {
        screen_id: "screen".into(),
        epoch: 1,
        audit_budget: 0,
        audit_cost: 1,
        min_recall_lcb: 0.1,
        max_rank_disagreement: 1.,
        authority_suspended: false,
    };
    let result = audit(
        &p,
        &[true, true, true, false],
        &[true, true, true, false],
        &[1., 2., 3., 4.],
        &[1., 2., 3., 4.],
    )
    .unwrap();
    assert!(!result.pruning_authorized);
    assert_eq!(result.cost_charged, 1);
}
#[test]
fn calibrated_prediction_abstains_and_policy_revision_needs_new_epoch() {
    let state = CalibrationState {
        identity: CalibrationIdentity {
            training_dataset_sha256: "a".repeat(64),
            observer_sha256: "b".repeat(64),
            model_sha256: "c".repeat(64),
            epoch: 2,
        },
        heldout_count: 20,
        interval_coverage: 0.95,
        coverage_lcb_95: 0.9,
        max_ood_distance: 1.,
        supported_geometry_families: vec!["supported".into()],
        calibrated: true,
        abstain_reason: None,
    };
    let p = Prediction {
        mean: 4.,
        lower: 3.,
        upper: 5.,
        geometry_family: "unknown".into(),
        ood_distance: 0.1,
        namespace: DatasetNamespace::IndependentQualified,
    };
    assert!(state.verify(&p, 0.8, 0.5).is_err());
    let old = FrozenEpoch {
        epoch: 2,
        coordinate_sha256: "d".into(),
        crosswalk_sha256: "e".into(),
        policy_version: "v1".into(),
    };
    let mut new = old.clone();
    new.policy_version = "v2".into();
    assert!(verify_epoch(&old, &new).is_err());
}
