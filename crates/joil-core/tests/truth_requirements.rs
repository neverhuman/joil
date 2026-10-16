use joil_core::{
    evolve::{
        merge_cluster_shards, run_campaign, run_single_seed_comparator, run_single_seed_wave3_arm,
        run_wave2_multiseed_benchmark, run_wave3_evolution_proof,
        run_wave3_evolution_proof_with_raw_capture, EvolutionConfig, SearchBaselineMode,
        Wave3ComparatorArm,
    },
    experiment::{admit_search_campaign, research_requirements, CampaignAdmissionReceipt},
};
use joil_evidence::campaign::CampaignMode;

#[test]
fn truth_campaign_admission_refuses_unadmitted_claims_before_computation() {
    let sandbox = admit_search_campaign(CampaignMode::Sandbox).unwrap();
    let requirements = research_requirements().unwrap();
    assert_eq!(sandbox.schema, "joil.campaign.v1");
    requirements
        .verify_executed_objectives(&sandbox.executed_objectives, &sandbox.objective_sha256)
        .unwrap();
    let mut drifted = sandbox.executed_objectives;
    drifted.swap(0, 1);
    assert!(requirements
        .verify_executed_objectives(&drifted, &sandbox.objective_sha256)
        .is_err());
    assert!(admit_search_campaign(CampaignMode::ClaimBearing).is_err());
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/truth-refused-campaign")
        .join(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string(),
        );
    let cfg = EvolutionConfig {
        campaign_mode: CampaignMode::ClaimBearing,
        ..EvolutionConfig::default()
    };
    assert!(run_campaign(&cfg, &output).is_err());
    assert!(
        !output.exists(),
        "admission must fail before creating campaign artifacts"
    );
}

#[test]
fn truth_campaign_receipt_binds_executed_objective_identity() {
    let cfg = EvolutionConfig {
        islands: 1,
        pop_per_island: 2,
        generations: 0,
        mc_samples: 4,
        ..EvolutionConfig::default()
    };
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/truth-requirements-smoke");
    let report = run_campaign(&cfg, &output).unwrap();
    let bytes = std::fs::read(output.join("campaign.json")).unwrap();
    let receipt: CampaignAdmissionReceipt = serde_json::from_slice(&bytes).unwrap();
    let requirements = research_requirements().unwrap();
    requirements
        .verify_executed_objectives(&receipt.executed_objectives, &receipt.objective_sha256)
        .unwrap();
    assert_eq!(
        receipt.objective_sha256,
        report.campaign_admission.objective_sha256
    );
    assert!(!receipt.unresolved_claim_gates.is_empty());
    assert_eq!(report.proposal_budget.budget, 3);

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/truth-shard-requirement-tests")
        .join(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string(),
        );
    let (merged, _) = merge_cluster_shards(
        std::slice::from_ref(&output),
        &[],
        &root.join("valid-sandbox-merge"),
    )
    .unwrap();
    assert_eq!(merged.campaign_admission, report.campaign_admission);

    let mut variants = Vec::new();
    let mut forged = report.clone();
    forged.config.campaign_mode = CampaignMode::ClaimBearing;
    forged.campaign_admission.mode = CampaignMode::ClaimBearing;
    variants.push(("claim-bearing", forged));
    let mut forged = report.clone();
    forged.campaign_admission.mode = CampaignMode::ClaimBearing;
    variants.push(("mode-mismatch", forged));
    let mut forged = report.clone();
    forged.campaign_admission.requirements.sha256 = "0".repeat(64);
    variants.push(("stale-requirement", forged));
    let mut forged = report.clone();
    forged.campaign_admission.objective_sha256 = "0".repeat(64);
    variants.push(("stale-objective-hash", forged));
    let mut forged = report.clone();
    forged.campaign_admission.executed_objectives.swap(0, 1);
    variants.push(("objective-order", forged));
    let mut forged = report.clone();
    forged.campaign_admission.unresolved_claim_gates.clear();
    variants.push(("forged-gate-closure", forged));
    for (name, forged) in variants {
        let source = root.join(format!("input-{name}"));
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            source.join("campaign-summary.json"),
            serde_json::to_vec(&forged).unwrap(),
        )
        .unwrap();
        let destination = root.join(format!("refused-{name}"));
        let error = merge_cluster_shards(&[output.clone(), source], &[], &destination).unwrap_err();
        assert!(
            error.contains("claim_bearing_requires_admitted_requirements")
                || error.contains("shard admission/requirement/objective receipt mismatch"),
            "{name}: must fail admission before trying to read the absent archive: {error}"
        );
        assert!(
            !destination.exists(),
            "{name}: output created before admission"
        );
    }
}

#[test]
fn truth_every_search_entrypoint_refuses_claims_before_budget_or_artifacts() {
    // Intentionally invalid numerical budgets prove admission runs first. These
    // are software negative controls, not a source of physical limit values.
    let cfg = EvolutionConfig {
        campaign_mode: CampaignMode::ClaimBearing,
        islands: 0,
        pop_per_island: 0,
        generations: 0,
        mc_samples: 0,
        ..EvolutionConfig::default()
    };
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/truth-refused-entrypoints")
        .join(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string(),
        );
    let errors = [
        run_campaign(&cfg, &output).unwrap_err(),
        run_single_seed_comparator(1, SearchBaselineMode::Ct35Fixed, &cfg).unwrap_err(),
        run_wave2_multiseed_benchmark(&[], &cfg).unwrap_err(),
        run_single_seed_wave3_arm(1, Wave3ComparatorArm::B0BaselineCt35, &cfg).unwrap_err(),
        run_wave3_evolution_proof(&[], &cfg).unwrap_err(),
        run_wave3_evolution_proof_with_raw_capture(&[], &cfg, &output).unwrap_err(),
    ];
    for error in errors {
        assert_eq!(error, "claim_bearing_requires_admitted_requirements");
    }
    assert!(!output.exists());
}
