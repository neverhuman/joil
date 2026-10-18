use crucible_search::acquisition::{allocate, largest_remainder, Arm, Proposal};
use crucible_search::archive::{ArchiveConfig, Candidate, EvidenceClass, SearchArchive};
use crucible_search::audit::{audit, verify_epoch, AuditPolicy, FrozenEpoch};
use crucible_search::reference::{hypervolume_2d, AttemptKind, AttemptRecord, Budget, Policy};
use crucible_search::surrogate::{
    CalibrationIdentity, CalibrationState, DatasetNamespace, Prediction,
};

#[test]
fn v5_22_02_to_05_sc_dec_038_multi_objective_acquisition_calibrated_surrogate_and_equal_budget_ablation(
) {
    // The archive accepts a Pareto set on two objectives; allocation remains
    // proposal-only and cannot assign evidence class or scientific authority.
    let config = ArchiveConfig {
        capacity_per_pool: 8,
        nondominated_capacity_per_descriptor_bin: 8,
        descriptor_bin_widths: vec![1.0],
        protection_lifetime_epochs: 2,
    };
    let mut archive = SearchArchive::default();
    for (proposal_id, objectives) in [
        (1, vec![1.0, 4.0]),
        (2, vec![2.0, 2.0]),
        (3, vec![4.0, 1.0]),
    ] {
        archive.insert(
            Candidate {
                proposal_id,
                class: EvidenceClass::Exploration,
                descriptors: vec![0.0],
                objectives,
                feasible: true,
            },
            0,
            &config,
        );
    }
    assert_eq!(
        archive.exploration.len(),
        3,
        "trade-off candidates remain nondominated"
    );
    let before = hypervolume_2d(&[[1.0, 4.0], [2.0, 2.0], [4.0, 1.0]], [5.0, 5.0]).unwrap();
    let after = hypervolume_2d(
        &[[0.5, 3.0], [1.0, 4.0], [2.0, 2.0], [4.0, 1.0]],
        [5.0, 5.0],
    )
    .unwrap();
    assert!(
        after > before,
        "multi-objective improvement increases reference hypervolume"
    );

    let shares = largest_remainder(
        10,
        &[
            (Arm::Improvement, 60),
            (Arm::Uncertainty, 20),
            (Arm::Diversity, 10),
            (Arm::Random, 10),
        ],
    )
    .unwrap();
    assert_eq!(shares.iter().map(|(_, n)| n).sum::<usize>(), 10);
    let proposed = allocate(
        2,
        20,
        4,
        true,
        2,
        &[
            Proposal {
                id: 10,
                arm: Arm::Improvement,
                cost: 1,
            },
            Proposal {
                id: 11,
                arm: Arm::Random,
                cost: 1,
            },
        ],
    )
    .unwrap();
    assert!(proposed.selected.iter().all(|p| p.id != 0));
    assert!(proposed.charged_cost <= 2);

    let calibrated = CalibrationState {
        identity: CalibrationIdentity {
            training_dataset_sha256: "a".repeat(64),
            observer_sha256: "b".repeat(64),
            model_sha256: "c".repeat(64),
            epoch: 1,
        },
        heldout_count: 20,
        interval_coverage: 0.95,
        coverage_lcb_95: 0.9,
        max_ood_distance: 1.0,
        supported_geometry_families: vec!["supported".into()],
        calibrated: true,
        abstain_reason: None,
    };
    let in_domain = Prediction {
        mean: 2.0,
        lower: 1.0,
        upper: 3.0,
        geometry_family: "supported".into(),
        ood_distance: 0.2,
        namespace: DatasetNamespace::SandboxPrior,
    };
    assert!(calibrated.verify(&in_domain, 0.8, 0.5).is_ok());
    let mut ood = in_domain.clone();
    ood.ood_distance = 1.1;
    assert!(calibrated.verify(&ood, 0.8, 0.5).is_err());
    // A surrogate proposal is still exploratory; it cannot become qualified evidence.
    archive.insert(
        Candidate {
            proposal_id: 99,
            class: EvidenceClass::Exploration,
            descriptors: vec![0.0],
            objectives: vec![0.5, 0.5],
            feasible: true,
        },
        1,
        &config,
    );
    assert!(archive.qualified.is_empty());
    assert_eq!(
        archive.evidence_front.last().unwrap().class,
        EvidenceClass::Exploration
    );

    // Equal charged attempts establish comparable bookkeeping against the frozen reference.
    let reference = Policy::reference_v1().unwrap();
    assert_eq!(reference.policy_id, "reference-search/1");
    let mut reference_budget = Budget {
        limit: 4,
        ..Budget::default()
    };
    let mut candidate_budget = Budget {
        limit: 4,
        ..Budget::default()
    };
    for budget in [&mut reference_budget, &mut candidate_budget] {
        for proposal_id in 0..4 {
            budget
                .charge(AttemptRecord {
                    proposal_id,
                    island: 0,
                    epoch: 0,
                    kind: AttemptKind::Offspring,
                    operator: None,
                    source: None,
                    outcome: "synthetic_attempt".into(),
                    charged_units: 1,
                })
                .unwrap();
        }
    }
    assert_eq!(reference_budget.charged, candidate_budget.charged);
    assert_eq!(reference_budget.records.len(), 4);

    let audit_policy = AuditPolicy {
        screen_id: "surrogate-screen".into(),
        epoch: 1,
        audit_budget: 4,
        audit_cost: 1,
        min_recall_lcb: 0.0,
        max_rank_disagreement: 0.5,
        authority_suspended: false,
    };
    let audit_result = audit(
        &audit_policy,
        &[true, false, true, false],
        &[true, false, true, false],
        &[1.0, 2.0, 3.0, 4.0],
        &[1.0, 2.0, 3.0, 4.0],
    )
    .unwrap();
    assert!(audit_result.pruning_authorized);
    let no_budget = AuditPolicy {
        audit_budget: 0,
        ..audit_policy
    };
    assert!(
        !audit(
            &no_budget,
            &[true, false],
            &[true, false],
            &[1.0, 2.0],
            &[1.0, 2.0],
        )
        .unwrap()
        .pruning_authorized
    );
}

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

#[test]
fn g009_guard_geometry_holdout_cannot_leak_shared_lineage() {
    use crucible_search::surrogate::{grouped_folds, AdmittedLabel, SubjectIdentity};
    let subject = SubjectIdentity {
        requirement_sha256: "a".repeat(64),
        geometry_sha256: "b".repeat(64),
        case_sha256: "c".repeat(64),
        model_sha256: "d".repeat(64),
        solver_sha256: "e".repeat(64),
        scenario_sha256: "f".repeat(64),
    };
    let rows: Vec<_> = (0..6)
        .map(|i| AdmittedLabel {
            id: format!("id{i}"),
            subject: subject.clone(),
            cout_mg_per_litre_dispersed_oil: 1.0,
            lineage_groups: vec!["shared-lineage".into()],
            geometry_family_groups: vec![format!("family{i}")],
            preparation_id: format!("prep{i}"),
        })
        .collect();
    assert!(grouped_folds(&rows, true).is_err());
}
#[test]
fn g009_guard_nonfinite_prediction_policy_forces_abstention() {
    let state = CalibrationState {
        identity: CalibrationIdentity {
            training_dataset_sha256: "a".repeat(64),
            observer_sha256: "b".repeat(64),
            model_sha256: "c".repeat(64),
            epoch: 1,
        },
        heldout_count: 20,
        interval_coverage: 0.95,
        coverage_lcb_95: 0.9,
        max_ood_distance: 1.0,
        supported_geometry_families: vec!["family".into()],
        calibrated: true,
        abstain_reason: None,
    };
    let p = Prediction {
        mean: 1.0,
        lower: 0.5,
        upper: 1.5,
        geometry_family: "family".into(),
        ood_distance: 0.1,
        namespace: DatasetNamespace::IndependentQualified,
    };
    assert!(state.verify(&p, 0.8, 0.5).is_ok());
    assert!(state.verify(&p, f64::NAN, f64::NAN).is_err());
}
