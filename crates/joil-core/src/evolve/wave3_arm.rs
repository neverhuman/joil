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

#[path = "wave3_runner.rs"]
mod wave3_runner;
pub use self::wave3_runner::*;
