use joil_core::evolve::{run_wave3_evolution_proof, EvolutionConfig};

#[test]
fn truth_proof_retains_features_for_sandbox_audit_without_inventing_discovery() {
    let cfg = EvolutionConfig {
        islands: 1,
        pop_per_island: 2,
        generations: 0,
        mc_samples: 4,
        ..EvolutionConfig::default()
    };
    let report = run_wave3_evolution_proof(&[4567, 4568, 4569, 4570, 4571], &cfg).unwrap();
    assert_eq!(report.n_seeds, 5);
    assert!(!report.held_out_geometry_generalization.is_empty());
    assert!(report
        .held_out_geometry_generalization
        .iter()
        .any(|record| record.de_novo_rediscovery_seeds == 0));
    for record in &report.held_out_geometry_generalization {
        assert!(record.de_novo_rediscovery_seeds <= report.n_seeds);
        assert!(record.lofo_surrogate_rmse_kpi_score.is_none());
    }
}
