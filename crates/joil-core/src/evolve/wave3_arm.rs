//! Wave-3 composite KPI score, comparator arm definitions, and single-seed arm runner.

use super::*;

/// Canonical Composite Separation-Hydraulic KPI Score matching `analysis/kpi_fitness.py`:
/// `KPI_score(g) = (requirements.quality_limit_mg_l - C_out,sample_screen_upper(g)) + 25.0 * E_reduced,20um,sample_screen_lower(g) + 10.0 * (dp_u_CT35 - dp_u(g)) / dp_u_CT35 - 500.0 * V(g)`.
/// Higher is better; `CT-35` baseline evaluates to `22.972463`.
pub fn compute_wave3_kpi_score(ind: &Individual, baseline_dp_u_pa: f64) -> f64 {
    let dp_ref = baseline_dp_u_pa.max(1.0);
    let sep_margin = crate::experiment::research_requirements()
        .expect("compiled requirements must validate")
        .thresholds
        .quality_limit_mg_l
        .get()
        - ind.mc_robustness.cout_p95_mg_l;
    let grade_term = 25.0 * ind.mc_robustness.grade_20um_p05;
    let dp_term = 10.0 * ((dp_ref - ind.nominal_metrics.dp_u_pa) / dp_ref);
    let penalty = 500.0 * ind.constraint_violation;
    let raw = sep_margin + grade_term + dp_term - penalty;
    (raw * 1e6).round() / 1e6
}

/// Wave-3 comparator and ablation arms (`N = 10` seeds, equal evaluation budget per seed).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Wave3ComparatorArm {
    B0BaselineCt35,
    B1RandomSearchExotic3d,
    B2ConventionalAxisymNsga2,
    A1NoMutationCrossoverOnly,
    A2NoSelectionRandomWalk,
    B3Wave3Exotic3dQdCei,
}

/// Per-generation KPI, concentration, hypervolume, archive illumination, and diversity telemetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave3GenerationStats {
    pub generation: usize,
    pub cumulative_evaluations: usize,
    pub best_kpi_score: f64,
    pub active_best_kpi_score: f64,
    pub median_kpi_score: f64,
    pub q25_kpi_score: f64,
    pub q75_kpi_score: f64,
    #[serde(
        rename = "best_cout_sample_screen_upper_mg_l",
        alias = "best_cout_p95_mg_l"
    )]
    pub best_cout_p95_mg_l: f64,
    #[serde(
        rename = "active_best_cout_sample_screen_upper_mg_l",
        alias = "active_best_cout_p95_mg_l"
    )]
    pub active_best_cout_p95_mg_l: f64,
    #[serde(
        rename = "median_cout_sample_screen_upper_mg_l",
        alias = "median_cout_p95_mg_l"
    )]
    pub median_cout_p95_mg_l: f64,
    #[serde(
        rename = "q25_cout_sample_screen_upper_mg_l",
        alias = "q25_cout_p95_mg_l"
    )]
    pub q25_cout_p95_mg_l: f64,
    #[serde(
        rename = "q75_cout_sample_screen_upper_mg_l",
        alias = "q75_cout_p95_mg_l"
    )]
    pub q75_cout_p95_mg_l: f64,
    pub best_dp_u_pa: f64,
    #[serde(
        rename = "best_grade_20um_sample_screen_lower",
        alias = "best_grade_20um_p05"
    )]
    pub best_grade_20um_p05: f64,
    pub pareto_hypervolume: f64,
    pub map_elites_occupied_cells: usize,
    pub active_topology_families: usize,
    pub shannon_diversity_bits: f64,
    pub mean_exoticness_index: f64,
    pub cumulative_lineage_jumps: usize,
}

/// Aggregated multi-seed confidence band at a single generation `g`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave3GenerationBand {
    pub generation: usize,
    pub cumulative_evaluations: usize,
    pub best_kpi_score: BootstrapConfidenceInterval,
    pub median_kpi_score: BootstrapConfidenceInterval,
    #[serde(
        rename = "best_cout_sample_screen_upper_mg_l",
        alias = "best_cout_p95_mg_l"
    )]
    pub best_cout_p95_mg_l: BootstrapConfidenceInterval,
    #[serde(
        rename = "median_cout_sample_screen_upper_mg_l",
        alias = "median_cout_p95_mg_l"
    )]
    pub median_cout_p95_mg_l: BootstrapConfidenceInterval,
    pub pareto_hypervolume: BootstrapConfidenceInterval,
    pub map_elites_occupied_cells: BootstrapConfidenceInterval,
    pub shannon_diversity_bits: BootstrapConfidenceInterval,
    pub mean_exoticness_index: BootstrapConfidenceInterval,
}

/// Recorded evolutionary milestone when a seed discovers a new incumbent best individual.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave3LineageMilestone {
    pub seed: u64,
    pub generation: usize,
    pub candidate_id: String,
    pub parent_topology_class: String,
    pub child_topology_class: String,
    pub operator_applied: String,
    pub cross_family_jump: bool,
    pub kpi_score: f64,
    #[serde(rename = "cout_sample_screen_upper_mg_l", alias = "cout_p95_mg_l")]
    pub cout_p95_mg_l: f64,
    pub dp_u_pa: f64,
    #[serde(rename = "grade_20um_sample_screen_lower", alias = "grade_20um_p05")]
    pub grade_20um_p05: f64,
    pub exoticness_index: f64,
    pub total_entry_jets: u32,
    pub tpms_basis: String,
}

/// Complete single-seed trajectory and terminal outcome for a Wave-3 arm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave3SeedOutcome {
    pub campaign_admission: crate::experiment::CampaignAdmissionReceipt,
    #[serde(default, flatten)]
    pub proposal_budget: ProposalBudget,
    pub seed: u64,
    pub arm: Wave3ComparatorArm,
    pub total_evaluations: usize,
    pub valid_f0_evaluations: usize,
    pub gen0_best_kpi_score: f64,
    pub gen0_median_kpi_score: f64,
    #[serde(
        rename = "gen0_best_cout_sample_screen_upper_mg_l",
        alias = "gen0_best_cout_p95_mg_l"
    )]
    pub gen0_best_cout_p95_mg_l: f64,
    #[serde(
        rename = "gen0_median_cout_sample_screen_upper_mg_l",
        alias = "gen0_median_cout_p95_mg_l"
    )]
    pub gen0_median_cout_p95_mg_l: f64,
    pub final_best_kpi_score: f64,
    pub final_median_kpi_score: f64,
    #[serde(
        rename = "final_best_cout_sample_screen_upper_mg_l",
        alias = "final_best_cout_p95_mg_l"
    )]
    pub final_best_cout_p95_mg_l: f64,
    #[serde(
        rename = "final_median_cout_sample_screen_upper_mg_l",
        alias = "final_median_cout_p95_mg_l"
    )]
    pub final_median_cout_p95_mg_l: f64,
    pub final_best_dp_u_pa: f64,
    pub final_best_specific_energy_j_m3: f64,
    pub final_best_d50_um: f64,
    #[serde(
        rename = "final_best_grade_20um_sample_screen_lower",
        alias = "final_best_grade_20um_p05"
    )]
    pub final_best_grade_20um_p05: f64,
    pub final_pareto_hypervolume: f64,
    pub final_map_elites_cells: usize,
    pub final_topology_families_discovered: usize,
    pub final_shannon_diversity_bits: f64,
    pub total_cross_family_lineage_jumps: usize,
    pub kpi_gain_from_gen0: f64,
    pub kpi_gain_vs_ct35: f64,
    pub generation_trace: Vec<Wave3GenerationStats>,
    pub lineage_milestones: Vec<Wave3LineageMilestone>,
    pub best_candidate: Individual,
}

/// Held-out 3D geometry family generalization (Leave-One-Family-Out GP surrogate + de-novo rediscovery).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave3HeldOutGeometryRecord {
    pub topology_class: String,
    pub sample_count: usize,
    #[serde(rename = "sandbox_authored_family_rmse_cout_mg_l")]
    pub lofo_surrogate_rmse_cout_mg_l: Option<f64>,
    #[serde(rename = "sandbox_authored_family_rmse_kpi_score")]
    pub lofo_surrogate_rmse_kpi_score: Option<f64>,
    #[serde(rename = "sandbox_authored_family_2sigma_coverage")]
    pub lofo_2sigma_coverage: Option<f64>,
    pub de_novo_rediscovery_seeds: usize,
    pub de_novo_rediscovery_fraction: f64,
    pub held_out_best_kpi_score: f64,
    #[serde(
        rename = "held_out_best_cout_sample_screen_upper_mg_l",
        alias = "held_out_best_cout_p95_mg_l"
    )]
    pub held_out_best_cout_p95_mg_l: f64,
    pub held_out_best_dp_u_pa: f64,
    #[serde(
        rename = "held_out_best_grade_20um_sample_screen_lower",
        alias = "held_out_best_grade_20um_p05"
    )]
    pub held_out_best_grade_20um_p05: f64,
    pub kpi_gain_vs_ct35: f64,
    pub cout_reduction_pct_vs_ct35: f64,
}

/// Complete Wave-3 Multi-Seed (`N = 10` seeds x `25` generations x `5` active arms = `22,500` 3D evals) Proof Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave3EvolutionProofReport {
    pub campaign_admission: crate::experiment::CampaignAdmissionReceipt,
    pub schema_version: u32,
    pub protocol_id: String,
    pub kpi_formula: String,
    #[serde(rename = "screen_comparison_note", alias = "qualification_note")]
    pub qualification_note: String,
    pub n_seeds: usize,
    pub master_seeds: Vec<u64>,
    pub generations: usize,
    pub population_per_generation: usize,
    pub evaluations_per_seed_per_arm: usize,
    pub total_3d_evaluations_across_arms: usize,
    pub baseline_b0_ct35_kpi_score: f64,
    #[serde(
        rename = "baseline_b0_ct35_cout_sample_screen_upper_mg_l",
        alias = "baseline_b0_ct35_cout_p95_mg_l"
    )]
    pub baseline_b0_ct35_cout_p95_mg_l: f64,
    pub baseline_b0_ct35_dp_u_pa: f64,
    #[serde(
        rename = "baseline_b0_ct35_grade_20um_sample_screen_lower",
        alias = "baseline_b0_ct35_grade_20um_p05"
    )]
    pub baseline_b0_ct35_grade_20um_p05: f64,
    pub baseline_b0_ct35: Individual,
    pub baseline_b0_held_out_duties: Vec<HeldOutDutyResult>,
    pub arm_outcomes: BTreeMap<String, Vec<Wave3SeedOutcome>>,
    pub generation_band_curves: BTreeMap<String, Vec<Wave3GenerationBand>>,
    pub final_summary_statistics: BTreeMap<String, BTreeMap<String, BootstrapConfidenceInterval>>,
    pub pairwise_tests: Vec<PairwiseWilcoxonComparison>,
    pub held_out_geometry_generalization: Vec<Wave3HeldOutGeometryRecord>,
    pub promoted_wave3_top_elite: Individual,
    pub promoted_wave3_top_elite_kpi_score: f64,
    pub promoted_wave3_top_elite_held_out_duties: Vec<HeldOutDutyResult>,
    pub promoted_wave3_elites_by_family: BTreeMap<String, Individual>,
}

pub(crate) fn shannon_entropy_bits(individuals: &[Individual]) -> (usize, f64) {
    if individuals.is_empty() {
        return (0, 0.0);
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for ind in individuals {
        *counts.entry(ind.topology_class.as_str()).or_insert(0) += 1;
    }
    let n = individuals.len() as f64;
    let mut h = 0.0_f64;
    for &c in counts.values() {
        if c > 0 {
            let p = (c as f64) / n;
            h -= p * p.log2();
        }
    }
    (counts.len(), (h * 1e6).round() / 1e6)
}

pub(crate) fn build_wave3_gen_stats(
    generation: usize,
    cumulative_evaluations: usize,
    active_pop: &[Individual],
    archive: &BTreeMap<String, Individual>,
    best_so_far: &Individual,
    base_ind: &Individual,
    cumulative_lineage_jumps: usize,
) -> Wave3GenerationStats {
    let dp_ref = base_ind.nominal_metrics.dp_u_pa;
    let mut active_kpis: Vec<f64> = active_pop
        .iter()
        .map(|ind| compute_wave3_kpi_score(ind, dp_ref))
        .collect();
    active_kpis.sort_by(|a, b| a.total_cmp(b));

    let mut active_couts: Vec<f64> = active_pop
        .iter()
        .map(|ind| ind.mc_robustness.cout_p95_mg_l)
        .collect();
    active_couts.sort_by(|a, b| a.total_cmp(b));

    let best_kpi = compute_wave3_kpi_score(best_so_far, dp_ref);
    let active_best_kpi = active_kpis.last().copied().unwrap_or(best_kpi);
    let median_kpi = percentile_sorted(&active_kpis, 0.50);
    let q25_kpi = percentile_sorted(&active_kpis, 0.25);
    let q75_kpi = percentile_sorted(&active_kpis, 0.75);

    let best_cout = best_so_far.mc_robustness.cout_p95_mg_l;
    let active_best_cout = active_couts.first().copied().unwrap_or(best_cout);
    let median_cout = percentile_sorted(&active_couts, 0.50);
    let q25_cout = percentile_sorted(&active_couts, 0.25);
    let q75_cout = percentile_sorted(&active_couts, 0.75);

    let arch_front: Vec<Individual> = archive.values().cloned().collect();
    let hv = compute_normalized_hypervolume(&arch_front, base_ind);
    let (active_fams, shannon_bits) = shannon_entropy_bits(active_pop);
    let mean_exotic = if active_pop.is_empty() {
        0.0
    } else {
        let s: f64 = active_pop
            .iter()
            .map(|x| x.program.exoticness_index().unwrap_or(0.0))
            .sum();
        ((s / (active_pop.len() as f64)) * 1e6).round() / 1e6
    };

    Wave3GenerationStats {
        generation,
        cumulative_evaluations,
        best_kpi_score: best_kpi,
        active_best_kpi_score: active_best_kpi,
        median_kpi_score: ((median_kpi * 1e6).round()) / 1e6,
        q25_kpi_score: ((q25_kpi * 1e6).round()) / 1e6,
        q75_kpi_score: ((q75_kpi * 1e6).round()) / 1e6,
        best_cout_p95_mg_l: best_cout,
        active_best_cout_p95_mg_l: active_best_cout,
        median_cout_p95_mg_l: ((median_cout * 1e6).round()) / 1e6,
        q25_cout_p95_mg_l: ((q25_cout * 1e6).round()) / 1e6,
        q75_cout_p95_mg_l: ((q75_cout * 1e6).round()) / 1e6,
        best_dp_u_pa: best_so_far.nominal_metrics.dp_u_pa,
        best_grade_20um_p05: best_so_far.mc_robustness.grade_20um_p05,
        pareto_hypervolume: hv,
        map_elites_occupied_cells: archive.len(),
        active_topology_families: active_fams,
        shannon_diversity_bits: shannon_bits,
        mean_exoticness_index: mean_exotic,
        cumulative_lineage_jumps,
    }
}

/// Run a single seed of a Wave-3 arm (`B1`, `B2`, `A1`, `A2`, or `B3`) starting from the shared
/// unseeded Generation-0 population (`init_wave3_de_novo_gen0`) over `cfg.generations` generations.
pub fn run_single_seed_wave3_arm(
    seed: u64,
    arm: Wave3ComparatorArm,
    cfg: &EvolutionConfig,
) -> Result<Wave3SeedOutcome, String> {
    run_single_seed_wave3_arm_recorded(seed, arm, cfg, None)
}

pub(crate) fn run_single_seed_wave3_arm_recorded(
    seed: u64,
    arm: Wave3ComparatorArm,
    cfg: &EvolutionConfig,
    raw: Option<&RawProofCapture>,
) -> Result<Wave3SeedOutcome, String> {
    let campaign_admission = crate::experiment::admit_search_campaign(cfg.campaign_mode)?;
    let mut seed_cfg = cfg.clone();
    seed_cfg.seed = seed;
    let mut reference_mc = Vec::new();
    let (base_ind, _) = evaluate_individual_recorded(
        "baseline-ct35".into(),
        DesignProgram::baseline_ct35(),
        0,
        0,
        &seed_cfg,
        raw.map(|_| &mut reference_mc),
    )?;
    let mut capture = raw
        .map(|raw| RawSeedCapture::new(raw, seed, arm, &base_ind, reference_mc))
        .transpose()?;
    let dp_ref = base_ind.nominal_metrics.dp_u_pa;
    let base_kpi = compute_wave3_kpi_score(&base_ind, dp_ref);

    let pop_size = cfg
        .islands
        .checked_mul(cfg.pop_per_island)
        .filter(|n| *n > 0)
        .ok_or("positive finite population budget required")?;
    let mut gen0_rng = Rng::new(seed ^ 0x517C_C1B7_2722_0A95);
    let shared_gen0_progs = DesignProgram::init_wave3_de_novo_gen0(pop_size, &mut gen0_rng)?;

    let mut rng = Rng::new(seed);
    let mut archive: BTreeMap<String, Individual> = BTreeMap::new();
    let mut pop: Vec<Individual> = Vec::with_capacity(pop_size);
    let mut all_evaluated_records: Vec<crate::surrogate::SurrogateRecord> = Vec::new();
    let mut total_evals = 0usize;
    let mut valid_f0 = 0usize;
    let mut cumulative_jumps = 0usize;
    let mut lineage_milestones: Vec<Wave3LineageMilestone> = Vec::new();
    let mut generation_trace: Vec<Wave3GenerationStats> = Vec::with_capacity(cfg.generations + 1);

    let mut best_so_far = base_ind.clone();
    let mut best_kpi_so_far = base_kpi;

    let mut proposal_budget = ProposalBudget {
        reference_evaluations: 1,
        ..ProposalBudget::default()
    };
    // Shared generated subjects remain identical; invalidity is recorded per arm.
    for (i, initial_program) in shared_gen0_progs.iter().enumerate() {
        let outcome = match arm {
            Wave3ComparatorArm::B0BaselineCt35 => VariationOutcome::introduce(
                DesignProgram::baseline_ct35(),
                "insert_baseline_control",
                &mut rng,
            ),
            Wave3ComparatorArm::B1RandomSearchExotic3d if i > 0 => {
                DesignProgram::sample_random_exotic(&mut rng)
            }
            Wave3ComparatorArm::B2ConventionalAxisymNsga2 => {
                let mut p = initial_program.clone();
                p.inlet_split.enabled = false;
                p.chamber_modes.clear();
                p.skimmer.capture_flutes = 0;
                p.inlet_ramp.axial_drop_mm = 0.0;
                p.inlet_ramp.superellipse_exponent = 2.0;
                VariationOutcome::introduce(p, "initialize_conventional_control", &mut rng)
            }
            _ => {
                VariationOutcome::introduce(initial_program.clone(), "initialize_de_novo", &mut rng)
            }
        };
        let Some(ind) = evaluate_proposal_recorded(
            outcome,
            format!("s{seed}-g000-i{i:03}"),
            0,
            i % cfg.islands.max(1),
            &seed_cfg,
            &mut proposal_budget,
            capture.as_mut(),
        )?
        else {
            continue;
        };
        total_evals += 1;
        if ind.red_team_passed {
            valid_f0 += 1;
            all_evaluated_records.push(crate::surrogate::SurrogateRecord::from_candidate(&ind));
            let k = map_elites_key(&ind);
            match archive.get(&k) {
                Some(inc) if !is_better_for_archive(&ind, inc) => {}
                _ => {
                    archive.insert(k, ind.clone());
                }
            }
            let cand_kpi = compute_wave3_kpi_score(&ind, dp_ref);
            if cand_kpi > best_kpi_so_far + 1e-9 {
                best_kpi_so_far = cand_kpi;
                best_so_far = ind.clone();
            }
        }
        pop.push(ind);
    }

    assign_nsga2_ranks_and_crowding(&mut pop);
    lineage_milestones.push(Wave3LineageMilestone {
        seed,
        generation: 0,
        candidate_id: best_so_far.id.clone(),
        parent_topology_class: "baseline-ct35".into(),
        child_topology_class: best_so_far.topology_class.clone(),
        operator_applied: "gen0_de_novo_init".into(),
        cross_family_jump: best_so_far.topology_class != "baseline-ct35",
        kpi_score: best_kpi_so_far,
        cout_p95_mg_l: best_so_far.mc_robustness.cout_p95_mg_l,
        dp_u_pa: best_so_far.nominal_metrics.dp_u_pa,
        grade_20um_p05: best_so_far.mc_robustness.grade_20um_p05,
        exoticness_index: best_so_far.program.exoticness_index().unwrap_or(0.0),
        total_entry_jets: if best_so_far.program.inlet_split.enabled {
            2 * best_so_far.program.inlet_split.branches_per_port
        } else {
            2
        },
        tpms_basis: format!("{:?}", best_so_far.program.tpms_insert.basis),
    });

    generation_trace.push(build_wave3_gen_stats(
        0,
        total_evals,
        &pop,
        &archive,
        &best_so_far,
        &base_ind,
        cumulative_jumps,
    ));
    if let Some(capture) = capture.as_mut() {
        capture.generation(0, &pop, generation_trace.last().unwrap().pareto_hypervolume)?;
    }

    // Generations 1..=cfg.generations
    for gen in 1..=cfg.generations {
        let arch_vals: Vec<Individual> = archive.values().cloned().collect();
        let mut offspring_specs: Vec<(VariationOutcome, String, String)> =
            Vec::with_capacity(pop_size);

        match arm {
            Wave3ComparatorArm::B0BaselineCt35 => {
                for _ in 0..pop_size {
                    offspring_specs.push((
                        VariationOutcome::introduce(
                            DesignProgram::baseline_ct35(),
                            "insert_baseline_control",
                            &mut rng,
                        )?,
                        "baseline-ct35".into(),
                        "identity".into(),
                    ));
                }
            }
            Wave3ComparatorArm::B1RandomSearchExotic3d => {
                for _ in 0..pop_size {
                    offspring_specs.push((
                        DesignProgram::sample_random_exotic(&mut rng)?,
                        "random_lhs".into(),
                        "sample_random_exotic".into(),
                    ));
                }
            }
            Wave3ComparatorArm::B2ConventionalAxisymNsga2 => {
                for _ in 0..pop_size {
                    let p1 = tournament_select(&pop, &mut rng);
                    offspring_specs.push((
                        p1.program
                            .mutate_conventional_axisymmetric(&mut rng)
                            .map(|outcome| outcome.with_parent_ids(vec![p1.id.clone()]))?,
                        p1.topology_class.clone(),
                        "mutate_conventional_axisymmetric".into(),
                    ));
                }
            }
            Wave3ComparatorArm::A1NoMutationCrossoverOnly => {
                for _ in 0..pop_size {
                    let p1 = tournament_select(&pop, &mut rng);
                    let p2 = tournament_select(&pop, &mut rng);
                    offspring_specs.push((
                        p1.program
                            .crossover_non_homologous_macro(&p2.program, &mut rng)
                            .map(|outcome| {
                                outcome.with_parent_ids(vec![p1.id.clone(), p2.id.clone()])
                            })?,
                        p1.topology_class.clone(),
                        "crossover_non_homologous_macro_only".into(),
                    ));
                }
            }
            Wave3ComparatorArm::A2NoSelectionRandomWalk => {
                for _ in 0..pop_size {
                    let idx1 = (rng.next_u64() as usize) % pop.len();
                    let p1 = &pop[idx1];
                    let (child, op_name) = if rng.f64() < cfg.crossover_rate {
                        let idx2 = (rng.next_u64() as usize) % pop.len();
                        let p2 = &pop[idx2];
                        let c = p1
                            .program
                            .crossover_non_homologous_macro(&p2.program, &mut rng)
                            .map(|outcome| {
                                outcome.with_parent_ids(vec![p1.id.clone(), p2.id.clone()])
                            })?;
                        (
                            c.then(|p| p.mutate_wave3_progressive(&mut rng))?,
                            "unselected_crossover_and_progressive_mutate",
                        )
                    } else {
                        (
                            p1.program
                                .mutate_wave3_progressive(&mut rng)
                                .map(|outcome| outcome.with_parent_ids(vec![p1.id.clone()]))?,
                            "unselected_progressive_mutate",
                        )
                    };
                    offspring_specs.push((child, p1.topology_class.clone(), op_name.into()));
                }
            }
            Wave3ComparatorArm::B3Wave3Exotic3dQdCei => {
                let pool_n = pop_size;
                let mut pool: Vec<(VariationOutcome, String, String)> = Vec::with_capacity(pool_n);
                for slot in 0..pool_n {
                    let p1 = if slot % 4 == 0 && !arch_vals.is_empty() {
                        &arch_vals[(rng.next_u64() as usize) % arch_vals.len()]
                    } else {
                        tournament_select(&pop, &mut rng)
                    };
                    let (cand, op_name) = if rng.f64() < cfg.crossover_rate && !arch_vals.is_empty()
                    {
                        let p2 = &arch_vals[(rng.next_u64() as usize) % arch_vals.len()];
                        let crossed = p1
                            .program
                            .crossover_non_homologous_macro(&p2.program, &mut rng)
                            .map(|outcome| {
                                outcome.with_parent_ids(vec![p1.id.clone(), p2.id.clone()])
                            })?;
                        (
                            crossed.then(|p| p.mutate_wave3_progressive(&mut rng))?,
                            "crossover_homologous_plus_progressive_mutate",
                        )
                    } else {
                        (
                            p1.program
                                .mutate_wave3_progressive(&mut rng)
                                .map(|outcome| outcome.with_parent_ids(vec![p1.id.clone()]))?,
                            "mutate_wave3_progressive",
                        )
                    };
                    pool.push((cand, p1.topology_class.clone(), op_name.into()));
                }
                if let Ok(gp) = crate::surrogate::GaussianProcessSurrogate::fit_sandbox_prior(
                    &all_evaluated_records,
                    0.85,
                    0.015,
                ) {
                    let incumbent_best_cout = best_so_far.mc_robustness.cout_p95_mg_l;
                    let mut scored: Vec<(usize, f64)> = pool
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, (outcome, _, _))| {
                            outcome.child.as_ref().and_then(|program| {
                                gp.expected_constrained_improvement(program, incumbent_best_cout)
                                    .ok()
                                    .map(|acquisition| {
                                        (idx, acquisition.constrained_expected_improvement)
                                    })
                            })
                        })
                        .collect();
                    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
                    let mut selected = std::collections::BTreeSet::new();
                    for (idx, _) in scored {
                        selected.insert(idx);
                        offspring_specs.push(pool[idx].clone());
                    }
                    for (idx, outcome) in pool.into_iter().enumerate() {
                        if !selected.contains(&idx) {
                            offspring_specs.push(outcome);
                        }
                    }
                } else {
                    offspring_specs.extend(pool.into_iter().take(pop_size));
                }
            }
        }

        let mut offspring = Vec::with_capacity(pop_size);
        for (i, (outcome, parent_tc, op_name)) in offspring_specs.into_iter().enumerate() {
            let Some(ind) = evaluate_proposal_recorded(
                Ok(outcome),
                format!("s{seed}-g{gen:03}-i{i:03}"),
                gen,
                i % cfg.islands.max(1),
                &seed_cfg,
                &mut proposal_budget,
                capture.as_mut(),
            )?
            else {
                continue;
            };
            total_evals += 1;
            if ind.topology_class != parent_tc {
                cumulative_jumps += 1;
            }
            if ind.red_team_passed {
                valid_f0 += 1;
                all_evaluated_records.push(crate::surrogate::SurrogateRecord::from_candidate(&ind));
                let k = map_elites_key(&ind);
                match archive.get(&k) {
                    Some(inc) if !is_better_for_archive(&ind, inc) => {}
                    _ => {
                        archive.insert(k, ind.clone());
                    }
                }
                let cand_kpi = compute_wave3_kpi_score(&ind, dp_ref);
                if cand_kpi > best_kpi_so_far + 1e-9 {
                    best_kpi_so_far = cand_kpi;
                    best_so_far = ind.clone();
                    let is_jump = ind.topology_class != parent_tc;
                    lineage_milestones.push(Wave3LineageMilestone {
                        seed,
                        generation: gen,
                        candidate_id: ind.id.clone(),
                        parent_topology_class: parent_tc,
                        child_topology_class: ind.topology_class.clone(),
                        operator_applied: op_name,
                        cross_family_jump: is_jump,
                        kpi_score: cand_kpi,
                        cout_p95_mg_l: ind.mc_robustness.cout_p95_mg_l,
                        dp_u_pa: ind.nominal_metrics.dp_u_pa,
                        grade_20um_p05: ind.mc_robustness.grade_20um_p05,
                        exoticness_index: ind.program.exoticness_index().unwrap_or(0.0),
                        total_entry_jets: if ind.program.inlet_split.enabled {
                            2 * ind.program.inlet_split.branches_per_port
                        } else {
                            2
                        },
                        tpms_basis: format!("{:?}", ind.program.tpms_insert.basis),
                    });
                }
            }
            offspring.push(ind);
        }

        // Survival update per arm semantics
        match arm {
            Wave3ComparatorArm::B1RandomSearchExotic3d => {
                // Random search has no parental inheritance in pop, but retains cumulative best in archive
                pop = offspring;
                assign_nsga2_ranks_and_crowding(&mut pop);
            }
            Wave3ComparatorArm::A2NoSelectionRandomWalk => {
                // No-selection random walk: zero tournament selection AND zero elitist archive survival
                pop = offspring;
                assign_nsga2_ranks_and_crowding(&mut pop);
                archive.clear();
                for ind in &pop {
                    if ind.red_team_passed {
                        let k = map_elites_key(ind);
                        match archive.get(&k) {
                            Some(inc) if !is_better_for_archive(ind, inc) => {}
                            _ => {
                                archive.insert(k, ind.clone());
                            }
                        }
                    }
                }
                if let Some(active_best) =
                    pop.iter().filter(|x| x.red_team_passed).max_by(|a, b| {
                        compute_wave3_kpi_score(a, dp_ref)
                            .total_cmp(&compute_wave3_kpi_score(b, dp_ref))
                    })
                {
                    best_so_far = active_best.clone();
                    best_kpi_so_far = compute_wave3_kpi_score(&best_so_far, dp_ref);
                }
            }
            _ => {
                let mut combined = Vec::with_capacity(2 * pop_size);
                combined.extend(pop);
                combined.extend(offspring);
                assign_nsga2_ranks_and_crowding(&mut combined);
                combined.sort_by(|a, b| {
                    a.rank.cmp(&b.rank).then_with(|| {
                        let ka = compute_wave3_kpi_score(a, dp_ref);
                        let kb = compute_wave3_kpi_score(b, dp_ref);
                        kb.total_cmp(&ka)
                    })
                });
                combined.truncate(pop_size);
                pop = combined;
            }
        }

        generation_trace.push(build_wave3_gen_stats(
            gen,
            total_evals,
            &pop,
            &archive,
            &best_so_far,
            &base_ind,
            cumulative_jumps,
        ));
        if let Some(capture) = capture.as_mut() {
            capture.generation(
                gen,
                &pop,
                generation_trace.last().unwrap().pareto_hypervolume,
            )?;
        }
    }
    if let Some(capture) = capture {
        capture.finish()?;
    }

    let mut fam_set = BTreeMap::new();
    for ind in archive.values() {
        fam_set.insert(ind.topology_class.clone(), true);
    }
    let g0 = generation_trace.first().unwrap().clone();
    let gf = generation_trace.last().unwrap().clone();
    let mut compact_best = best_so_far;
    compact_best.phenotype = None;
    compact_best.cuda_sdf_audit = None;

    Ok(Wave3SeedOutcome {
        campaign_admission,
        proposal_budget,
        seed,
        arm,
        total_evaluations: total_evals,
        valid_f0_evaluations: valid_f0,
        gen0_best_kpi_score: g0.best_kpi_score,
        gen0_median_kpi_score: g0.median_kpi_score,
        gen0_best_cout_p95_mg_l: g0.best_cout_p95_mg_l,
        gen0_median_cout_p95_mg_l: g0.median_cout_p95_mg_l,
        final_best_kpi_score: gf.best_kpi_score,
        final_median_kpi_score: gf.median_kpi_score,
        final_best_cout_p95_mg_l: gf.best_cout_p95_mg_l,
        final_median_cout_p95_mg_l: gf.median_cout_p95_mg_l,
        final_best_dp_u_pa: compact_best.nominal_metrics.dp_u_pa,
        final_best_specific_energy_j_m3: compact_best.nominal_metrics.specific_energy_j_m3,
        final_best_d50_um: compact_best.nominal_metrics.d50_um,
        final_best_grade_20um_p05: compact_best.mc_robustness.grade_20um_p05,
        final_pareto_hypervolume: gf.pareto_hypervolume,
        final_map_elites_cells: gf.map_elites_occupied_cells,
        final_topology_families_discovered: fam_set.len(),
        final_shannon_diversity_bits: gf.shannon_diversity_bits,
        total_cross_family_lineage_jumps: cumulative_jumps,
        kpi_gain_from_gen0: ((gf.best_kpi_score - g0.best_kpi_score) * 1e6).round() / 1e6,
        kpi_gain_vs_ct35: ((gf.best_kpi_score - base_kpi) * 1e6).round() / 1e6,
        generation_trace,
        lineage_milestones,
        best_candidate: compact_best,
    })
}
