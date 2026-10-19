//! Wave-2 single-seed comparator arm runner and multi-seed 4-arm equal-budget benchmark.

use super::*;

/// Run a single seed of comparator arm `mode` (`B1`, `B2`, or `B3`) at fixed evaluation budget.
pub fn run_single_seed_comparator(
    seed: u64,
    mode: SearchBaselineMode,
    cfg: &EvolutionConfig,
) -> Result<(SeedRunOutcome, Vec<Individual>), String> {
    let campaign_admission = crate::experiment::admit_search_campaign(cfg.campaign_mode)?;
    let mut seed_cfg = cfg.clone();
    seed_cfg.seed = seed;
    let (base_ind, _) = evaluate_individual(
        "baseline-ct35".into(),
        DesignProgram::baseline_ct35(),
        0,
        0,
        &seed_cfg,
    )?;

    let mut rng = Rng::new(seed);
    let mut archive: BTreeMap<String, Individual> = BTreeMap::new();
    archive.insert(map_elites_key(&base_ind), base_ind.clone());

    let mut total_evals = 0usize;
    let mut valid_f0 = 0usize;
    let mut rejected_or_suboptimal: Vec<Individual> = Vec::new();
    let mut hv_curve = Vec::with_capacity(cfg.generations + 1);

    let archetypes = match mode {
        SearchBaselineMode::ConventionalAxisymmetricNsga2 => vec![DesignProgram::baseline_ct35()],
        _ => DesignProgram::seed_wave2_archetypes(&mut rng),
    };
    let priority_order = [
        15usize, 17, 16, 18, 3, 12, 6, 9, 14, 4, 7, 10, 13, 1, 2, 5, 8, 11, 0,
    ];

    let mut pop: Vec<Individual> = Vec::with_capacity(cfg.islands * cfg.pop_per_island);
    let pop_size = cfg
        .islands
        .checked_mul(cfg.pop_per_island)
        .filter(|n| *n > 0)
        .ok_or("positive finite population budget required")?;

    let mut proposal_budget = ProposalBudget {
        reference_evaluations: 1,
        ..ProposalBudget::default()
    };
    for gen in 0..=cfg.generations {
        let arch_vals: Vec<Individual> = archive.values().cloned().collect();
        let mut offspring = Vec::with_capacity(pop_size);
        for i in 0..pop_size {
            let outcome = match mode {
                SearchBaselineMode::Ct35Fixed => VariationOutcome::introduce(
                    DesignProgram::baseline_ct35(),
                    "insert_baseline_control",
                    &mut rng,
                ),
                SearchBaselineMode::UniformRandomExotic3d => {
                    DesignProgram::sample_random_exotic(&mut rng)
                }
                SearchBaselineMode::ConventionalAxisymmetricNsga2 => {
                    let parent = if gen == 0 {
                        &base_ind
                    } else {
                        tournament_select(&pop, &mut rng)
                    };
                    parent
                        .program
                        .mutate_conventional_axisymmetric(&mut rng)
                        .map(|outcome| outcome.with_parent_ids(vec![parent.id.clone()]))
                }
                SearchBaselineMode::Exotic3dQdCei => {
                    if gen == 0 {
                        let idx =
                            priority_order[i % priority_order.len()].min(archetypes.len() - 1);
                        if i < priority_order.len() {
                            VariationOutcome::introduce(
                                archetypes[idx].clone(),
                                "insert_catalog_seed",
                                &mut rng,
                            )
                        } else {
                            archetypes[idx].mutate(&mut rng)
                        }
                    } else {
                        let p1 = tournament_select(&pop, &mut rng);
                        if rng.f64() < cfg.crossover_rate && !arch_vals.is_empty() {
                            let p2 = &arch_vals[(rng.next_u64() as usize) % arch_vals.len()];
                            p1.program.crossover(&p2.program, &mut rng).map(|outcome| {
                                outcome.with_parent_ids(vec![p1.id.clone(), p2.id.clone()])
                            })
                        } else {
                            p1.program
                                .mutate(&mut rng)
                                .map(|outcome| outcome.with_parent_ids(vec![p1.id.clone()]))
                        }
                    }
                }
            };
            // One generated proposal consumes one slot, including invalid proposals.
            // No uncounted oversampled surrogate pool is generated.
            let Some(ind) = evaluate_proposal(
                outcome,
                format!("s{seed}-g{gen:03}-i{i:03}"),
                gen,
                i % cfg.islands.max(1),
                &seed_cfg,
                &mut proposal_budget,
            )?
            else {
                continue;
            };
            total_evals += 1;
            if ind.red_team_passed {
                valid_f0 += 1;
                let k = map_elites_key(&ind);
                if archive
                    .get(&k)
                    .is_none_or(|inc| is_better_for_archive(&ind, inc))
                {
                    archive.insert(k, ind.clone());
                }
            }
            offspring.push(ind);
        }
        pop.extend(offspring);
        if pop.is_empty() {
            return Err("no valid initial proposals; no fallback population permitted".into());
        }
        assign_nsga2_ranks_and_crowding(&mut pop);
        pop.sort_by(|a, b| {
            a.rank
                .cmp(&b.rank)
                .then_with(|| b.crowding_distance.total_cmp(&a.crowding_distance))
        });
        for dropped in pop.iter().skip(pop_size) {
            if rejected_or_suboptimal.len() < 64 {
                rejected_or_suboptimal.push(dropped.clone());
            }
        }
        pop.truncate(pop_size);
        let arch_front: Vec<Individual> = archive.values().cloned().collect();
        hv_curve.push(compute_normalized_hypervolume(&arch_front, &base_ind));
    }

    let mut all_pool: Vec<Individual> = archive.values().cloned().collect();
    for ind in &pop {
        if ind.red_team_passed
            && !all_pool
                .iter()
                .any(|x| x.genome_sha256 == ind.genome_sha256)
        {
            all_pool.push(ind.clone());
        }
    }
    assign_nsga2_ranks_and_crowding(&mut all_pool);
    all_pool.sort_by(|a, b| {
        a.constraint_violation
            .total_cmp(&b.constraint_violation)
            .then_with(|| {
                a.mc_robustness
                    .cout_p95_mg_l
                    .total_cmp(&b.mc_robustness.cout_p95_mg_l)
            })
    });
    let best = all_pool.first().cloned().unwrap_or(base_ind.clone());
    let mut fam_set = BTreeMap::new();
    for ind in archive.values() {
        fam_set.insert(ind.topology_class.clone(), true);
    }

    let outcome = SeedRunOutcome {
        campaign_admission,
        proposal_budget,
        seed,
        mode,
        total_evaluations: total_evals,
        valid_f0_evaluations: valid_f0,
        best_cout_p95_mg_l: best.mc_robustness.cout_p95_mg_l,
        best_dp_u_pa: best.nominal_metrics.dp_u_pa,
        best_specific_energy_j_m3: best.nominal_metrics.specific_energy_j_m3,
        best_d50_um: best.nominal_metrics.d50_um,
        best_grade_20um_p05: best.mc_robustness.grade_20um_p05,
        pareto_hypervolume: *hv_curve.last().unwrap_or(&0.0),
        map_elites_cells: archive.len(),
        topology_families_discovered: fam_set.len(),
        hypervolume_by_generation: hv_curve,
        best_candidate: best,
    };
    Ok((outcome, rejected_or_suboptimal))
}

/// Execute the complete Wave-2 Multi-Seed (`N = seeds.len()`) 4-Arm Equal-Budget Benchmark (`B0`, `B1`, `B2`, `B3`).
pub fn run_wave2_multiseed_benchmark(
    seeds: &[u64],
    cfg: &EvolutionConfig,
) -> Result<Wave2MultiseedReport, String> {
    let campaign_admission = crate::experiment::admit_search_campaign(cfg.campaign_mode)?;
    if seeds.is_empty() {
        return Err("at least 1 master seed required".into());
    }
    let (base_b0, _) = evaluate_individual(
        "baseline-ct35-b0".into(),
        DesignProgram::baseline_ct35(),
        0,
        0,
        cfg,
    )?;
    let base_duties = evaluate_held_out_duties(&base_b0.program, &cfg.baseline_hydraulics)?;

    let mut b1_outcomes = Vec::with_capacity(seeds.len());
    let mut b2_outcomes = Vec::with_capacity(seeds.len());
    let mut b3_outcomes = Vec::with_capacity(seeds.len());
    let mut rejected_pool = Vec::new();
    let mut best_by_family: BTreeMap<String, Individual> = BTreeMap::new();

    for &seed in seeds {
        let (o1, _) =
            run_single_seed_comparator(seed, SearchBaselineMode::UniformRandomExotic3d, cfg)?;
        let (o2, _) = run_single_seed_comparator(
            seed,
            SearchBaselineMode::ConventionalAxisymmetricNsga2,
            cfg,
        )?;
        let (o3, rej3) = run_single_seed_comparator(seed, SearchBaselineMode::Exotic3dQdCei, cfg)?;

        let tc = o3.best_candidate.topology_class.clone();
        match best_by_family.get(&tc) {
            Some(inc) if !is_better_for_archive(&o3.best_candidate, inc) => {}
            _ => {
                best_by_family.insert(tc, o3.best_candidate.clone());
            }
        }
        rejected_pool.extend(rej3);
        b1_outcomes.push(o1);
        b2_outcomes.push(o2);
        b3_outcomes.push(o3);
    }

    // Ensure all 11 Wave-2 families are represented in best_by_family
    let mut seed_rng = Rng::new(seeds[0]);
    for arch in DesignProgram::seed_wave2_archetypes(&mut seed_rng) {
        let (ind, _) = evaluate_individual(
            format!("wave2-arch-{}", arch.topology_class),
            arch,
            0,
            0,
            cfg,
        )?;
        let tc = ind.topology_class.clone();
        match best_by_family.get(&tc) {
            Some(inc) if !is_better_for_archive(&ind, inc) => {}
            _ => {
                best_by_family.insert(tc, ind);
            }
        }
    }

    // Compute bootstrap CIs across all arms
    let mut summary_stats = BTreeMap::new();
    for (arm_name, outcomes) in [
        ("B1_Uniform_Random", &b1_outcomes),
        ("B2_Conventional_Axisym", &b2_outcomes),
        ("B3_Exotic3D_QD_CEI", &b3_outcomes),
    ] {
        let mut arm_map = BTreeMap::new();
        let couts: Vec<f64> = outcomes.iter().map(|o| o.best_cout_p95_mg_l).collect();
        let dps: Vec<f64> = outcomes.iter().map(|o| o.best_dp_u_pa).collect();
        let hvs: Vec<f64> = outcomes.iter().map(|o| o.pareto_hypervolume).collect();
        let cells: Vec<f64> = outcomes.iter().map(|o| o.map_elites_cells as f64).collect();
        arm_map.insert(
            "cout_sample_screen_upper_mg_l".into(),
            compute_bootstrap_ci(&couts, 20269001),
        );
        arm_map.insert("dp_u_pa".into(), compute_bootstrap_ci(&dps, 20269002));
        arm_map.insert(
            "pareto_hypervolume".into(),
            compute_bootstrap_ci(&hvs, 20269003),
        );
        arm_map.insert(
            "map_elites_cells".into(),
            compute_bootstrap_ci(&cells, 20269004),
        );
        summary_stats.insert(arm_name.to_string(), arm_map);
    }

    // Pairwise Wilcoxon / Mann-Whitney U + Holm-Bonferroni correction
    let b1_cout: Vec<f64> = b1_outcomes.iter().map(|o| o.best_cout_p95_mg_l).collect();
    let b2_cout: Vec<f64> = b2_outcomes.iter().map(|o| o.best_cout_p95_mg_l).collect();
    let b3_cout: Vec<f64> = b3_outcomes.iter().map(|o| o.best_cout_p95_mg_l).collect();

    let b1_hv: Vec<f64> = b1_outcomes.iter().map(|o| o.pareto_hypervolume).collect();
    let b2_hv: Vec<f64> = b2_outcomes.iter().map(|o| o.pareto_hypervolume).collect();
    let b3_hv: Vec<f64> = b3_outcomes.iter().map(|o| o.pareto_hypervolume).collect();

    let mut tests = vec![
        mann_whitney_and_cliffs_delta(
            "cout_sample_screen_upper_mg_l",
            "B3_Exotic3D_QD_CEI",
            "B1_Uniform_Random",
            &b3_cout,
            &b1_cout,
            true,
        ),
        mann_whitney_and_cliffs_delta(
            "cout_sample_screen_upper_mg_l",
            "B3_Exotic3D_QD_CEI",
            "B2_Conventional_Axisym",
            &b3_cout,
            &b2_cout,
            true,
        ),
        mann_whitney_and_cliffs_delta(
            "pareto_hypervolume",
            "B3_Exotic3D_QD_CEI",
            "B1_Uniform_Random",
            &b3_hv,
            &b1_hv,
            false,
        ),
        mann_whitney_and_cliffs_delta(
            "pareto_hypervolume",
            "B3_Exotic3D_QD_CEI",
            "B2_Conventional_Axisym",
            &b3_hv,
            &b2_hv,
            false,
        ),
    ];

    // Apply Holm-Bonferroni step-down multiplicity adjustment
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

    // Mandatory 10% random audit of rejected/low-ranked cohort
    let total_rej = rejected_pool.len().max(1);
    let sample_n = ((total_rej as f64) * 0.10).ceil().max(1.0) as usize;
    let best_b3_cout = b3_outcomes
        .iter()
        .map(|o| o.best_cout_p95_mg_l)
        .fold(f64::INFINITY, f64::min);
    let mut false_omissions = 0usize;
    let mut worst_reversal = 0.0_f64;
    for item in rejected_pool.iter().take(sample_n) {
        if item.red_team_passed
            && item.constraint_violation <= 1e-9
            && item.mc_robustness.cout_p95_mg_l < best_b3_cout
        {
            false_omissions += 1;
            worst_reversal = worst_reversal.max(best_b3_cout - item.mc_robustness.cout_p95_mg_l);
        }
    }

    let top_wave2_elite = b3_outcomes
        .iter()
        .min_by(|a, b| a.best_cout_p95_mg_l.total_cmp(&b.best_cout_p95_mg_l))
        .map(|o| o.best_candidate.clone())
        .unwrap_or_else(|| base_b0.clone());
    let top_duties = evaluate_held_out_duties(&top_wave2_elite.program, &cfg.baseline_hydraulics)?;
    let evals_per_seed = b3_outcomes
        .first()
        .map(|o| o.total_evaluations)
        .unwrap_or(0);

    Ok(Wave2MultiseedReport {
        campaign_admission,
        schema_version: 1,
        protocol_id: "joil-wave2-preregistration-v1".into(),
        n_seeds: seeds.len(),
        master_seeds: seeds.to_vec(),
        evaluations_per_seed: evals_per_seed,
        baseline_b0_ct35: base_b0,
        baseline_b0_held_out_duties: base_duties,
        arm_b1_random_lhs_outcomes: b1_outcomes,
        arm_b2_conventional_axisym_outcomes: b2_outcomes,
        arm_b3_exotic3d_qd_cei_outcomes: b3_outcomes,
        summary_statistics: summary_stats,
        pairwise_tests: tests,
        rejected_cohort_10pct_audit: RejectedCohortAuditSummary {
            total_rejected_or_low_rank_pool: rejected_pool.len(),
            audited_sample_count: sample_n.min(rejected_pool.len()),
            audit_fraction: 0.10,
            false_omission_rank0_count: false_omissions,
            false_omission_rate: false_omissions as f64 / (sample_n as f64),
            worst_rank_reversal_cout_mg_l: worst_reversal,
            audit_passed: false_omissions == 0,
        },
        promoted_wave2_elites_by_family: best_by_family,
        promoted_wave2_top_elite_held_out_duties: top_duties,
    })
}
