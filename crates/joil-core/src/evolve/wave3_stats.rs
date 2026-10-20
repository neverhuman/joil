use super::*;

pub(super) type Wave3StatsBundle = (
    BTreeMap<String, Vec<Wave3GenerationBand>>,
    BTreeMap<String, BTreeMap<String, BootstrapConfidenceInterval>>,
    Vec<PairwiseWilcoxonComparison>,
);

pub(super) fn compute_wave3_bands_and_pairwise_tests(
    cfg: &EvolutionConfig,
    arm_outcomes: &BTreeMap<String, Vec<Wave3SeedOutcome>>,
) -> Wave3StatsBundle {
    let mut generation_band_curves: BTreeMap<String, Vec<Wave3GenerationBand>> = BTreeMap::new();
    let mut final_summary_statistics: BTreeMap<
        String,
        BTreeMap<String, BootstrapConfidenceInterval>,
    > = BTreeMap::new();

    for (arm_name, outcomes) in arm_outcomes {
        let n_gens = cfg.generations + 1;
        let mut bands = Vec::with_capacity(n_gens);
        for g in 0..n_gens {
            let cum_evals = outcomes[0].generation_trace[g].cumulative_evaluations;
            let best_kpis: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].best_kpi_score)
                .collect();
            let med_kpis: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].median_kpi_score)
                .collect();
            let best_couts: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].best_cout_p95_mg_l)
                .collect();
            let med_couts: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].median_cout_p95_mg_l)
                .collect();
            let hvs: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].pareto_hypervolume)
                .collect();
            let cells: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].map_elites_occupied_cells as f64)
                .collect();
            let divs: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].shannon_diversity_bits)
                .collect();
            let exotics: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].mean_exoticness_index)
                .collect();

            bands.push(Wave3GenerationBand {
                generation: g,
                cumulative_evaluations: cum_evals,
                best_kpi_score: compute_bootstrap_ci(&best_kpis, 20269100 + g as u64),
                median_kpi_score: compute_bootstrap_ci(&med_kpis, 20269200 + g as u64),
                best_cout_p95_mg_l: compute_bootstrap_ci(&best_couts, 20269300 + g as u64),
                median_cout_p95_mg_l: compute_bootstrap_ci(&med_couts, 20269400 + g as u64),
                pareto_hypervolume: compute_bootstrap_ci(&hvs, 20269500 + g as u64),
                map_elites_occupied_cells: compute_bootstrap_ci(&cells, 20269600 + g as u64),
                shannon_diversity_bits: compute_bootstrap_ci(&divs, 20269700 + g as u64),
                mean_exoticness_index: compute_bootstrap_ci(&exotics, 20269800 + g as u64),
            });
        }
        let final_band = bands.last().unwrap().clone();
        let mut stat_map = BTreeMap::new();
        stat_map.insert("best_kpi_score".into(), final_band.best_kpi_score);
        stat_map.insert("median_kpi_score".into(), final_band.median_kpi_score);
        stat_map.insert(
            "best_cout_sample_screen_upper_mg_l".into(),
            final_band.best_cout_p95_mg_l,
        );
        stat_map.insert(
            "median_cout_sample_screen_upper_mg_l".into(),
            final_band.median_cout_p95_mg_l,
        );
        stat_map.insert("pareto_hypervolume".into(), final_band.pareto_hypervolume);
        stat_map.insert(
            "map_elites_occupied_cells".into(),
            final_band.map_elites_occupied_cells,
        );
        stat_map.insert(
            "shannon_diversity_bits".into(),
            final_band.shannon_diversity_bits,
        );
        final_summary_statistics.insert(arm_name.clone(), stat_map);
        generation_band_curves.insert(arm_name.clone(), bands);
    }

    // Pairwise Mann-Whitney U + Cliff's delta + Holm-Bonferroni across all comparator arms
    let b3_outs = arm_outcomes.get("B3_Wave3_Exotic3D_QD_CEI").unwrap();
    let b3_best_kpi: Vec<f64> = b3_outs.iter().map(|o| o.final_best_kpi_score).collect();
    let b3_med_kpi: Vec<f64> = b3_outs.iter().map(|o| o.final_median_kpi_score).collect();
    let b3_best_cout: Vec<f64> = b3_outs.iter().map(|o| o.final_best_cout_p95_mg_l).collect();
    let b3_hv: Vec<f64> = b3_outs.iter().map(|o| o.final_pareto_hypervolume).collect();

    let mut tests: Vec<PairwiseWilcoxonComparison> = Vec::new();
    for other_arm in [
        "B1_Random_Search",
        "B2_Conventional_Axisym_NSGA2",
        "A1_No_Mutation_Crossover_Only",
        "A2_No_Selection_Random_Walk",
    ] {
        let outs = arm_outcomes.get(other_arm).unwrap();
        let o_best_kpi: Vec<f64> = outs.iter().map(|o| o.final_best_kpi_score).collect();
        let o_med_kpi: Vec<f64> = outs.iter().map(|o| o.final_median_kpi_score).collect();
        let o_best_cout: Vec<f64> = outs.iter().map(|o| o.final_best_cout_p95_mg_l).collect();
        let o_hv: Vec<f64> = outs.iter().map(|o| o.final_pareto_hypervolume).collect();

        tests.push(mann_whitney_and_cliffs_delta(
            "final_best_kpi_score",
            "B3_Wave3_Exotic3D_QD_CEI",
            other_arm,
            &b3_best_kpi,
            &o_best_kpi,
            false,
        ));
        tests.push(mann_whitney_and_cliffs_delta(
            "final_median_kpi_score",
            "B3_Wave3_Exotic3D_QD_CEI",
            other_arm,
            &b3_med_kpi,
            &o_med_kpi,
            false,
        ));
        tests.push(mann_whitney_and_cliffs_delta(
            "final_best_cout_sample_screen_upper_mg_l",
            "B3_Wave3_Exotic3D_QD_CEI",
            other_arm,
            &b3_best_cout,
            &o_best_cout,
            true,
        ));
        tests.push(mann_whitney_and_cliffs_delta(
            "final_pareto_hypervolume",
            "B3_Wave3_Exotic3D_QD_CEI",
            other_arm,
            &b3_hv,
            &o_hv,
            false,
        ));
    }

    // Holm-Bonferroni step-down multiplicity correction across all 16 pairwise tests
    let m_tests = tests.len();
    let mut order: Vec<usize> = (0..m_tests).collect();
    order.sort_by(|&i, &j| {
        tests[i]
            .p_value_one_sided
            .total_cmp(&tests[j].p_value_one_sided)
    });
    let mut max_adj = 0.0_f64;
    for (rank_idx, &orig_idx) in order.iter().enumerate() {
        let mult = (m_tests - rank_idx) as f64;
        let adj = (tests[orig_idx].p_value_one_sided * mult)
            .min(1.0)
            .max(max_adj);
        max_adj = adj;
        tests[orig_idx].p_value_holm_bonferroni = (adj * 1e8).round() / 1e8;
        tests[orig_idx].statistically_significant_alpha_0025 = adj < 0.025;
    }

    (generation_band_curves, final_summary_statistics, tests)
}
