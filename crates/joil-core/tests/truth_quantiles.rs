use joil_core::{
    evolve::{evaluate_individual, EvolutionConfig, ObjectiveOutcome},
    metrics::{quantile_estimate, sampled_fraction_ci95, QuantileEstimate},
    program::DesignProgram,
};

#[test]
fn truth_tail_quantiles_require_declared_sample_count_and_ci() {
    let small = quantile_estimate(&[1.0; 12], 0.95, 0.95, 72).unwrap();
    assert!(matches!(
        small,
        QuantileEstimate::Insufficient {
            n: 12,
            minimum_n: 72,
            ..
        }
    ));
    let serialized = serde_json::to_value(&small).unwrap();
    assert!(serialized.get("value").is_none());
    assert!(serialized.get("ci_low").is_none());
    let median =
        quantile_estimate(&(0..10).map(|i| i as f64).collect::<Vec<_>>(), 0.5, 0.95, 0).unwrap();
    // For Binomial(10, .5), the two-sided 95% interval is X_(2)..X_(9).
    assert!(matches!(
        median,
        QuantileEstimate::Estimated {
            value: 4.5,
            ci_low: 1.0,
            ci_high: 8.0,
            ..
        }
    ));
    for (q, minimum) in [(0.05, 72), (0.95, 72), (0.01, 368), (0.99, 368)] {
        let values: Vec<f64> = (0..minimum).map(|i| i as f64).collect();
        assert!(quantile_estimate(&values[..minimum - 1], q, 0.95, minimum)
            .unwrap()
            .value()
            .is_none());
        assert!(quantile_estimate(&values, q, 0.95, minimum)
            .unwrap()
            .value()
            .is_some());
    }
    assert!(quantile_estimate(&[f64::NAN], 0.95, 0.95, 72).is_err());
    assert!(quantile_estimate(&[1.0], 1.0, 0.95, 72).is_err());
    assert!(sampled_fraction_ci95(1, 0).is_err());
    let interval = sampled_fraction_ci95(4, 4).unwrap();
    assert!(interval[0] < 1.0 && interval[1] == 1.0);
}

#[test]
fn truth_insufficient_tail_cannot_supply_objective_vector() {
    for n in [12, 72] {
        let cfg = EvolutionConfig {
            mc_samples: n,
            ..EvolutionConfig::default()
        };
        let (individual, _) = evaluate_individual(
            "synthetic-quantile-check".into(),
            DesignProgram::baseline_ct35(),
            0,
            0,
            &cfg,
        )
        .unwrap();
        assert_eq!(individual.mc_robustness.samples, n);
        if n < 72 {
            assert!(matches!(
                individual.objectives,
                ObjectiveOutcome::Insufficient { .. }
            ));
        } else {
            assert!(individual.objectives.values().is_some());
        }
        let json = serde_json::to_value(&individual.mc_robustness).unwrap();
        assert!(json.get("cout_p95_mg_l").is_none());
        assert!(json.get("probability_passes_30_mg_l").is_none());
        assert!(json.get("cout_sample_screen_upper_mg_l").is_some());
        assert_eq!(
            json["tail_quantiles"]["cout_p95"]["status"],
            if n < 72 { "insufficient" } else { "estimated" }
        );
    }
    let cfg = EvolutionConfig {
        mc_samples: 0,
        ..EvolutionConfig::default()
    };
    assert!(evaluate_individual(
        "zero-budget".into(),
        DesignProgram::baseline_ct35(),
        0,
        0,
        &cfg
    )
    .is_err());
}
