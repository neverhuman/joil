use joil_core::{
    evolve::{run_campaign, run_single_seed_wave3_arm, EvolutionConfig, Wave3ComparatorArm},
    experiment::ProposalBudget,
    program::{DesignProgram, Rng, VariationOutcome},
};

#[test]
fn truth_budget_invalid_repaired_evaluated_are_exclusive() {
    let parent = DesignProgram::baseline_ct35();
    let mut rng = Rng::new(10);
    let mut budget = ProposalBudget::default();
    let mut bad = parent.clone();
    bad.inlet_split.enabled = true;
    bad.inlet_split.branches_per_port = 99;
    budget.record(parent.propose_edit(bad, &mut rng).unwrap());
    budget.record(
        VariationOutcome::introduce(parent.clone(), "insert_baseline_control", &mut rng).unwrap(),
    );
    let repaired = (0..100)
        .map(|_| parent.mutate_wave3_progressive(&mut rng).unwrap())
        .find(|outcome| outcome.child.is_some() && !outcome.repairs.is_empty())
        .unwrap();
    budget.record(repaired);
    assert_eq!(budget.budget, 3);
    assert_eq!(
        (
            budget.proposals_invalid,
            budget.proposals_repaired,
            budget.evaluations
        ),
        (1, 1, 1)
    );
    assert!(budget.proposal_receipts[0].failure.is_some());
    assert_eq!(
        budget
            .operator_statistics
            .values()
            .map(|s| s.attempts)
            .sum::<usize>(),
        3
    );
    let json = serde_json::to_value(&budget).unwrap();
    assert_eq!(json["budget"], 3);
    assert_eq!(json["proposals_invalid"], 1);
}

#[test]
fn truth_tiny_campaign_budget() {
    let cfg = EvolutionConfig {
        seed: 777,
        islands: 1,
        pop_per_island: 8,
        generations: 1,
        mc_samples: 4,
        ..EvolutionConfig::default()
    };
    let output =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/truth-budget-smoke");
    let report = run_campaign(&cfg, &output).unwrap();
    let b = &report.proposal_budget;
    assert_eq!(b.budget, 17); // one explicit reference control + 16 proposal slots
    assert_eq!(
        b.budget,
        b.proposals_invalid + b.proposals_repaired + b.evaluations
    );
    assert_eq!(
        report.total_evaluations,
        b.proposals_repaired + b.evaluations
    );
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("campaign-summary.json")).unwrap())
            .unwrap();
    assert_eq!(saved["budget"], 17);
    for arm in [
        Wave3ComparatorArm::B0BaselineCt35,
        Wave3ComparatorArm::B1RandomSearchExotic3d,
        Wave3ComparatorArm::B2ConventionalAxisymNsga2,
        Wave3ComparatorArm::A1NoMutationCrossoverOnly,
        Wave3ComparatorArm::A2NoSelectionRandomWalk,
        Wave3ComparatorArm::B3Wave3Exotic3dQdCei,
    ] {
        let report = run_single_seed_wave3_arm(cfg.seed, arm, &cfg).unwrap();
        let b = report.proposal_budget;
        assert_eq!(b.budget, 16, "arm {arm:?}");
        assert_eq!(
            b.budget,
            b.proposals_invalid + b.proposals_repaired + b.evaluations
        );
        assert_eq!(
            report.total_evaluations,
            b.proposals_repaired + b.evaluations
        );
    }
}
