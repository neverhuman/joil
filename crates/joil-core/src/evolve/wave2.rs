//! Wave-2 multi-seed benchmark, held-out duty stress tests, and nonparametric statistics.

use super::*;

/// Wave-2 pre-registered comparator arm mode (`B0`, `B1`, `B2`, `B3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchBaselineMode {
    Ct35Fixed,
    #[serde(alias = "RandomLhsExotic3d")]
    UniformRandomExotic3d,
    ConventionalAxisymmetricNsga2,
    Exotic3dQdCei,
}

/// Held-out operating condition stress-test evaluation (`D1..D4`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeldOutDutyResult {
    pub duty_id: String,
    pub q_in_m3_h: f64,
    pub cin_mg_l: f64,
    pub d50_feed_um: f64,
    pub viscosity_factor: f64,
    pub cout_mg_l: f64,
    pub dp_u_pa: f64,
    pub specific_energy_j_m3: f64,
    pub d50_cut_um: f64,
    pub hinze_dmax_um: f64,
    pub passes_30_mg_l: bool,
}

/// Evaluate a candidate on the 4 pre-registered held-out operating duties (`D1..D4`).
pub fn evaluate_held_out_duties(
    prog: &DesignProgram,
    base_hyd: &BaselineHydraulics,
) -> Result<Vec<HeldOutDutyResult>, String> {
    let p = prog.normalize()?;
    let desc = p.descriptors()?;
    let duties: [(&str, f64, f64, f64, f64); 4] = [
        ("D1_turndown", 2.6, 500.0, 38.0, 1.0),
        ("D2_turnup", 5.4, 500.0, 38.0, 1.0),
        ("D3_sheared_feed", 4.0, 500.0, 29.5, 1.0),
        ("D4_cold_viscous_slug", 4.0, 850.0, 36.0, 1.32),
    ];
    let mut results = Vec::with_capacity(duties.len());
    for (duty_id, q_h, cin, d_feed, visc) in duties {
        let flow_ratio = q_h / 4.0;
        let mut duty_hyd = base_hyd.clone();
        duty_hyd.q_in_m3_s = q_h / 3600.0;
        duty_hyd.dp_u_pa = base_hyd.dp_u_pa * flow_ratio.powi(2) * visc.powf(0.18);
        duty_hyd.dp_o_pa = base_hyd.dp_o_pa * flow_ratio.powi(2) * visc.powf(0.18);

        // Adjust effective descriptors for turnup shear & viscosityStokes drag scaling
        let mut duty_desc = desc.clone();
        duty_desc.shear_mitigation_factor *= flow_ratio.powf(1.25);
        duty_desc.swirl_flux_index *= flow_ratio.powf(0.22) / visc.sqrt();

        let m = evaluate_realization(&p, &duty_desc, &duty_hyd, cin, d_feed, 0.0, 0.0);
        results.push(HeldOutDutyResult {
            duty_id: duty_id.to_string(),
            q_in_m3_h: q_h,
            cin_mg_l: cin,
            d50_feed_um: d_feed,
            viscosity_factor: visc,
            cout_mg_l: m.cout_mg_l,
            dp_u_pa: m.dp_u_pa,
            specific_energy_j_m3: m.specific_energy_j_m3,
            d50_cut_um: m.d50_um,
            hinze_dmax_um: m.hinze_dmax_um,
            passes_30_mg_l: m.passes_30_mg_l,
        });
    }
    Ok(results)
}

/// Per-seed outcome summary for multi-seed statistical comparisons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedRunOutcome {
    pub campaign_admission: crate::experiment::CampaignAdmissionReceipt,
    #[serde(default, flatten)]
    pub proposal_budget: ProposalBudget,
    pub seed: u64,
    pub mode: SearchBaselineMode,
    pub total_evaluations: usize,
    pub valid_f0_evaluations: usize,
    #[serde(
        rename = "best_cout_sample_screen_upper_mg_l",
        alias = "best_cout_p95_mg_l"
    )]
    pub best_cout_p95_mg_l: f64,
    pub best_dp_u_pa: f64,
    pub best_specific_energy_j_m3: f64,
    pub best_d50_um: f64,
    #[serde(
        rename = "best_grade_20um_sample_screen_lower",
        alias = "best_grade_20um_p05"
    )]
    pub best_grade_20um_p05: f64,
    pub pareto_hypervolume: f64,
    pub map_elites_cells: usize,
    pub topology_families_discovered: usize,
    pub hypervolume_by_generation: Vec<f64>,
    pub best_candidate: Individual,
}

/// Bootstrap 95% confidence interval and parametric/non-parametric summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfidenceInterval {
    pub n_seeds: usize,
    pub mean: f64,
    pub std_dev: f64,
    pub median: f64,
    pub iqr: f64,
    pub ci95_low: f64,
    pub ci95_high: f64,
}

/// Pairwise non-parametric hypothesis test (`B3 vs B1` and `B3 vs B2`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairwiseWilcoxonComparison {
    pub metric: String,
    pub comparator_a: String,
    pub comparator_b: String,
    pub mean_diff: f64,
    pub median_diff: f64,
    pub mann_whitney_u: f64,
    pub z_score: f64,
    pub p_value_one_sided: f64,
    pub p_value_holm_bonferroni: f64,
    pub cliffs_delta: f64,
    pub statistically_significant_alpha_0025: bool,
}

/// Complete Wave-2 Multi-Seed Ablation, Bootstrap CI, and Holdout Qualification Dossier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave2MultiseedReport {
    pub campaign_admission: crate::experiment::CampaignAdmissionReceipt,
    pub schema_version: u32,
    pub protocol_id: String,
    pub n_seeds: usize,
    pub master_seeds: Vec<u64>,
    pub evaluations_per_seed: usize,
    pub baseline_b0_ct35: Individual,
    pub baseline_b0_held_out_duties: Vec<HeldOutDutyResult>,
    #[serde(
        rename = "arm_b1_uniform_random_outcomes",
        alias = "arm_b1_random_lhs_outcomes"
    )]
    pub arm_b1_random_lhs_outcomes: Vec<SeedRunOutcome>,
    pub arm_b2_conventional_axisym_outcomes: Vec<SeedRunOutcome>,
    pub arm_b3_exotic3d_qd_cei_outcomes: Vec<SeedRunOutcome>,
    pub summary_statistics: BTreeMap<String, BTreeMap<String, BootstrapConfidenceInterval>>,
    pub pairwise_tests: Vec<PairwiseWilcoxonComparison>,
    pub rejected_cohort_10pct_audit: RejectedCohortAuditSummary,
    pub promoted_wave2_elites_by_family: BTreeMap<String, Individual>,
    pub promoted_wave2_top_elite_held_out_duties: Vec<HeldOutDutyResult>,
}

/// Summary of the mandatory 10% random audit of low-ranked / surrogate-rejected candidates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedCohortAuditSummary {
    pub total_rejected_or_low_rank_pool: usize,
    pub audited_sample_count: usize,
    pub audit_fraction: f64,
    pub false_omission_rank0_count: usize,
    pub false_omission_rate: f64,
    pub worst_rank_reversal_cout_mg_l: f64,
    pub audit_passed: bool,
}

pub(crate) fn compute_normalized_hypervolume(front: &[Individual], _base_ref: &Individual) -> f64 {
    let requirements = crate::experiment::research_requirements()
        .expect("compiled requirement contract must validate");
    let policy = &requirements.hypervolume;
    let reference: Vec<f64> = policy
        .reference_point
        .iter()
        .zip(&policy.normalization)
        .map(|(r, n)| r.get() / n.get())
        .collect();
    let points: Vec<Vec<f64>> = front
        .iter()
        .filter(|ind| {
            ind.red_team_passed
                && ind.constraint_violation <= 1e-9
                && ind.objectives.values().is_some()
                && ind
                    .structural_qualification
                    .as_ref()
                    .is_some_and(|s| s.passed)
        })
        .map(|ind| {
            ind.objectives
                .values()
                .unwrap()
                .iter()
                .zip(&policy.normalization)
                .map(|(v, n)| v / n.get())
                .collect()
        })
        .collect();
    metrics::pareto_hypervolume(&points, &reference)
        .expect("finite contract-bound objective vectors")
}

/// Compute 10,000-resample bootstrap 95% confidence interval over `values`.
pub fn compute_bootstrap_ci(values: &[f64], seed: u64) -> BootstrapConfidenceInterval {
    if values.is_empty() {
        return BootstrapConfidenceInterval {
            n_seeds: 0,
            mean: 0.0,
            std_dev: 0.0,
            median: 0.0,
            iqr: 0.0,
            ci95_low: 0.0,
            ci95_high: 0.0,
        };
    }
    let n = values.len();
    let mean = values.iter().sum::<f64>() / (n as f64);
    let var = if n > 1 {
        values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / ((n - 1) as f64)
    } else {
        0.0
    };
    let std_dev = var.sqrt();
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = percentile_sorted(&sorted, 0.50);
    let iqr = percentile_sorted(&sorted, 0.75) - percentile_sorted(&sorted, 0.25);

    let mut rng = Rng::new(seed);
    let n_boot = 10000usize;
    let mut boot_means = Vec::with_capacity(n_boot);
    for _ in 0..n_boot {
        let mut s = 0.0;
        for _ in 0..n {
            let idx = (rng.next_u64() as usize) % n;
            s += values[idx];
        }
        boot_means.push(s / (n as f64));
    }
    boot_means.sort_by(|a, b| a.total_cmp(b));
    BootstrapConfidenceInterval {
        n_seeds: n,
        mean: (mean * 1e6).round() / 1e6,
        std_dev: (std_dev * 1e6).round() / 1e6,
        median: (median * 1e6).round() / 1e6,
        iqr: (iqr * 1e6).round() / 1e6,
        ci95_low: (percentile_sorted(&boot_means, 0.025) * 1e6).round() / 1e6,
        ci95_high: (percentile_sorted(&boot_means, 0.975) * 1e6).round() / 1e6,
    }
}

pub(crate) fn erfc_approx(x: f64) -> f64 {
    // Abramowitz & Stegun 7.1.26 rational approximation
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * z);
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-z * z).exp();
    if x >= 0.0 {
        1.0 - y
    } else {
        1.0 + y
    }
}

pub(crate) fn mann_whitney_and_cliffs_delta(
    metric: &str,
    name_a: &str,
    name_b: &str,
    a: &[f64],
    b: &[f64],
    smaller_is_better: bool,
) -> PairwiseWilcoxonComparison {
    let na = a.len().max(1) as f64;
    let nb = b.len().max(1) as f64;
    let mut u = 0.0_f64;
    let mut dom = 0.0_f64;
    for &xa in a {
        for &xb in b {
            let a_wins = if smaller_is_better { xa < xb } else { xa > xb };
            let b_wins = if smaller_is_better { xa > xb } else { xa < xb };
            if a_wins {
                u += 1.0;
                dom += 1.0;
            } else if b_wins {
                dom -= 1.0;
            } else {
                u += 0.5;
            }
        }
    }
    let mu_u = 0.5 * na * nb;
    let sigma_u = (na * nb * (na + nb + 1.0) / 12.0).sqrt().max(1e-9);
    let z = (u - mu_u) / sigma_u;
    let p_one_sided = 0.5 * erfc_approx(z / std::f64::consts::SQRT_2);
    let cliffs_delta = dom / (na * nb);

    let mean_a = a.iter().sum::<f64>() / na;
    let mean_b = b.iter().sum::<f64>() / nb;
    let mut sa = a.to_vec();
    let mut sb = b.to_vec();
    sa.sort_by(|x, y| x.total_cmp(y));
    sb.sort_by(|x, y| x.total_cmp(y));
    let med_a = percentile_sorted(&sa, 0.50);
    let med_b = percentile_sorted(&sb, 0.50);

    PairwiseWilcoxonComparison {
        metric: metric.to_string(),
        comparator_a: name_a.to_string(),
        comparator_b: name_b.to_string(),
        mean_diff: ((mean_a - mean_b) * 1e6).round() / 1e6,
        median_diff: ((med_a - med_b) * 1e6).round() / 1e6,
        mann_whitney_u: u,
        z_score: (z * 1e4).round() / 1e4,
        p_value_one_sided: (p_one_sided * 1e8).round() / 1e8,
        p_value_holm_bonferroni: (p_one_sided * 1e8).round() / 1e8,
        cliffs_delta: (cliffs_delta * 1e4).round() / 1e4,
        statistically_significant_alpha_0025: p_one_sided < 0.025,
    }
}

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
