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

#[path = "wave2_runner.rs"]
mod wave2_runner;
pub use self::wave2_runner::*;
