use joil_core::{
    evolve::{run_campaign, EvolutionConfig},
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
}
