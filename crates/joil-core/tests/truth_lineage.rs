use joil_core::{
    evolve::{run_campaign, EvolutionConfig},
    program::Rng,
};
use joil_evidence::{observation::EvidenceTier, subject::Lineage};

#[test]
fn truth_lineage_archive_matches_evidence_contract() {
    let cfg = EvolutionConfig {
        seed: 4567,
        islands: 1,
        pop_per_island: 8,
        generations: 1,
        mc_samples: 4,
        ..EvolutionConfig::default()
    };
    let output =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/truth-lineage-smoke");
    let report = run_campaign(&cfg, &output).unwrap();
    for individual in std::iter::once(&report.baseline_reference).chain(&report.top_pareto_elites) {
        let lineage = individual
            .lineage
            .as_ref()
            .expect("every new individual needs lineage");
        lineage.check().unwrap();
        assert_eq!(lineage.seed, cfg.seed);
        assert_eq!(lineage.compiler.sha256.len(), 64);
        assert_eq!(individual.evidence_tier, EvidenceTier::SoftwareContract);
    }
    let archive: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("map-elites-archive.json")).unwrap())
            .unwrap();
    for individual in archive.as_object().unwrap().values() {
        let mut lineage: Lineage = serde_json::from_value(individual["lineage"].clone()).unwrap();
        lineage.check().unwrap();
        lineage.compiler.sha256.clear();
        assert!(
            lineage.check().is_err(),
            "a missing compiler cannot silently validate"
        );
    }
    assert!(report
        .proposal_budget
        .proposal_receipts
        .iter()
        .any(|receipt| {
            !receipt.parent_ids.is_empty()
                && receipt.parent_ids.iter().all(|id| id.starts_with("gen"))
        }));
}

#[test]
fn truth_lineage_rng_draw_count_includes_wrapping_states() {
    let mut rng = Rng::new(u64::MAX);
    let before = rng.0;
    assert_eq!(Rng::draws_between(before, before), 0);
    for draws in 1..=100 {
        rng.next_u64();
        assert_eq!(Rng::draws_between(before, rng.0), draws);
    }
}
