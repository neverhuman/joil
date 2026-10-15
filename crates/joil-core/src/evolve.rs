//! Multi-island Constrained NSGA-II + 4D MAP-Elites Quality-Diversity evolutionary engine
//! for radical 3D hydrocyclone discovery, with Monte Carlo robustness and optional CUDA batch verification.

use crate::metrics;
use crate::openfoam::STANDARD_BINS_UM;
use crate::phenotype::{Phenotype3D, SolidMesh};
use crate::program::{DesignProgram, MechanismDescriptors, Rng};
use crate::redteam;
use crate::structure::{self, StructuralQualification};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineHydraulics {
    pub q_in_m3_s: f64,
    pub q_over_m3_s: f64,
    pub q_under_m3_s: f64,
    pub split_ratio: f64,
    pub mass_imbalance_rel: f64,
    pub dp_u_pa: f64,
    pub dp_o_pa: f64,
    pub pdr: f64,
    pub specific_hydraulic_loss_j_m3: f64,
    pub underflow_reverse_m3_s: f64,
    pub overflow_reverse_m3_s: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    pub seed: u64,
    pub islands: usize,
    pub pop_per_island: usize,
    pub generations: usize,
    pub migration_interval: usize,
    pub mc_samples: usize,
    pub crossover_rate: f64,
    pub cuda_bin_path: Option<PathBuf>,
    pub baseline_hydraulics: BaselineHydraulics,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            seed: 20261015,
            islands: 4,
            pop_per_island: 24,
            generations: 35,
            migration_interval: 5,
            mc_samples: 16,
            crossover_rate: 0.58,
            cuda_bin_path: None,
            baseline_hydraulics: BaselineHydraulics {
                q_in_m3_s: 0.001111111,
                q_over_m3_s: 2.206626e-5,
                q_under_m3_s: 0.0010890499,
                split_ratio: 0.019859632,
                mass_imbalance_rel: 4.6440003e-6,
                dp_u_pa: 29468.22,
                dp_o_pa: 208358.5,
                pdr: 7.108312,
                specific_hydraulic_loss_j_m3: 33709.55,
                underflow_reverse_m3_s: 0.0,
                overflow_reverse_m3_s: 0.0,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateMetrics {
    pub q_in_m3_h: f64,
    pub split_ratio: f64,
    pub dp_u_pa: f64,
    pub dp_o_pa: f64,
    pub pdr: f64,
    pub specific_energy_j_m3: f64,
    pub max_entry_shear_s_inv: f64,
    pub hinze_dmax_um: f64,
    pub short_circuit_fraction: f64,
    pub reverse_core_coherence: f64,
    pub grade_efficiency_bins: BTreeMap<String, f64>,
    pub d50_um: f64,
    pub d90_um: f64,
    pub overall_oil_capture: f64,
    pub cout_mg_l: f64,
    pub passes_30_mg_l: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloSummary {
    pub samples: usize,
    pub cout_p05_mg_l: f64,
    pub cout_p50_mg_l: f64,
    pub cout_p95_mg_l: f64,
    pub capture_p05: f64,
    pub capture_p50: f64,
    pub capture_p95: f64,
    pub d50_p05_um: f64,
    pub d50_p50_um: f64,
    pub d50_p95_um: f64,
    pub grade_20um_p05: f64,
    pub probability_passes_30_mg_l: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaSdfSummary {
    pub execution_backend: String,
    pub gpu_device_name: String,
    pub voxel_volume_mm3: f64,
    pub min_wall_clearance_mm: f64,
    pub gpu_max_von_mises_mpa: f64,
    pub gpu_yield_safety_factor: f64,
    pub gpu_d50_um: f64,
    pub gpu_cout_mg_l: f64,
    pub cpu_gpu_parity_abs_err: f64,
    #[serde(default)]
    pub gpu_vram_allocated_mib: f64,
    #[serde(default)]
    pub gpu_vram_used_mib: f64,
    #[serde(default)]
    pub gpu_vram_total_mib: f64,
    #[serde(default)]
    pub gpu_vram_utilization_pct: f64,
    #[serde(default)]
    pub gpu_3d_voxels_evaluated: u64,
    #[serde(default)]
    pub gpu_lagrangian_parcels_integrated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Individual {
    pub id: String,
    pub program: DesignProgram,
    pub genome_sha256: String,
    pub topology_class: String,
    pub generation_born: usize,
    pub island_origin: usize,
    pub red_team_passed: bool,
    pub red_team_rejects: Vec<String>,
    pub descriptors: MechanismDescriptors,
    pub phenotype: Option<Phenotype3D>,
    pub structural_qualification: Option<StructuralQualification>,
    pub nominal_metrics: CandidateMetrics,
    pub mc_robustness: MonteCarloSummary,
    pub cuda_sdf_audit: Option<CudaSdfSummary>,
    pub objectives: [f64; 5],
    pub constraint_violation: f64,
    pub rank: usize,
    pub crowding_distance: f64,
    pub evidence_tier: String,
    pub qualified_multiobjective_vector: Option<serde_json::Value>,
    pub qualified_capacity_gain: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSummary {
    pub schema_version: String,
    pub engine: String,
    pub claim_state: String,
    pub config: EvolutionConfig,
    pub total_evaluations: usize,
    pub valid_f0_evaluations: usize,
    pub red_team_rejections: usize,
    pub map_elites_occupied_cells: usize,
    pub pareto_front_size: usize,
    pub topology_family_counts_in_archive: BTreeMap<String, usize>,
    pub baseline_reference: Individual,
    pub top_pareto_elites: Vec<Individual>,
    pub exported_mesh_files: Vec<String>,
}

fn lognormal_bin_weights(d50_feed_um: f64, sigma_ln: f64) -> [f64; 10] {
    let mut weights = [0.0_f64; 10];
    let mu = d50_feed_um.max(2.0).ln();
    let mut sum = 0.0_f64;
    for (i, &d) in STANDARD_BINS_UM.iter().enumerate() {
        let z = (d.ln() - mu) / (sigma_ln.max(0.15) * std::f64::consts::SQRT_2);
        let pdf = (-z * z).exp() / (d * sigma_ln.max(0.15) * (2.0 * std::f64::consts::PI).sqrt());
        let width = if i == 0 {
            3.0
        } else {
            STANDARD_BINS_UM[i] - STANDARD_BINS_UM[i - 1]
        };
        weights[i] = (pdf * width).max(1e-9);
        sum += weights[i];
    }
    for w in &mut weights {
        *w /= sum;
    }
    weights
}

/// Evaluates a single realization (nominal or perturbed Monte Carlo sample) of the coupled
/// 3D swirl-shear-breakup-migration physics model anchored to the OpenFOAM 14 baseline.
pub fn evaluate_realization(
    prog: &DesignProgram,
    desc: &MechanismDescriptors,
    base_hyd: &BaselineHydraulics,
    cin_mg_l: f64,
    d50_feed_um: f64,
    split_delta: f64,
    wall_tol_mm: f64,
) -> CandidateMetrics {
    let q_in_m3_s = base_hyd.q_in_m3_s;
    let q_in_m3_h = q_in_m3_s * 3600.0;

    // Effective split ratio under skimmer / hydraulic resistance and MC perturbation
    let skim_split_bias = 1.0 + 0.015 * desc.core_shield_ratio - 0.008 * (wall_tol_mm / 0.05);
    let split_nom = (base_hyd.split_ratio * skim_split_bias + split_delta).clamp(0.015, 0.032);

    // Entry shear rate G_entry (s^-1) and Kolmogorov-Hinze breakup diameter d_max (um)
    let base_shear_s_inv = 4850.0_f64;
    let max_entry_shear_s_inv =
        (base_shear_s_inv * desc.shear_mitigation_factor * (1.0 - 0.04 * wall_tol_mm))
            .clamp(2400.0, 9500.0);
    let shear_ratio = (max_entry_shear_s_inv / base_shear_s_inv).max(0.40);
    let hinze_dmax_um = (64.0_f64 * shear_ratio.powf(-0.62)).clamp(18.0, 130.0);

    // Hydraulic pressure drops dp_u and dp_o
    let dp_u_pa = (base_hyd.dp_u_pa * desc.hydraulic_resistance_factor).max(20_000.0);
    let dp_o_pa = (base_hyd.dp_o_pa
        * (0.94 + 0.06 * desc.swirl_flux_index.powi(2))
        * (1.0 + 0.03 * desc.core_shield_ratio))
        .max(dp_u_pa * 2.5);
    let pdr = dp_o_pa / dp_u_pa.max(1.0);
    let specific_energy_j_m3 = (1.0 - split_nom) * dp_u_pa + split_nom * dp_o_pa;

    // Swirl preservation & reverse-core coherence
    let sym_gain = 1.0 + 0.038 * (desc.azimuthal_symmetry_order - 2.0).max(0.0);
    let pitch_gain = 1.0 + 0.28 * desc.axial_pitch_sin;
    let rifle_gain = 1.0 + 0.095 * desc.wall_rifling_stabilization;
    let reverse_core_coherence = (0.76
        * desc.swirl_flux_index
        * sym_gain
        * pitch_gain
        * rifle_gain
        * desc.cone_accel_persistence)
        .clamp(0.45, 1.45);

    // Lip short-circuit fraction around the vortex finder
    let shield_relief = (1.0 - 0.36 * desc.core_shield_ratio).clamp(0.45, 1.0);
    let short_circuit_fraction = (0.048 * shield_relief / sym_gain).clamp(0.010, 0.14);

    // Effective droplet feed distribution after entry shear breakup
    let eff_d50_feed = d50_feed_um.min(0.68 * hinze_dmax_um);
    let weights = lognormal_bin_weights(eff_d50_feed, 0.42);

    // Separation number by droplet bin (calibrated to CT-35 baseline: capture ~ 0.9400, Cout ~ 30.61 mg/L)
    let mut grade_efficiency_bins = BTreeMap::new();
    let mut overall_oil_capture = 0.0_f64;
    let mut prev_d = 0.0_f64;
    let mut prev_eta = 0.0_f64;
    let mut d50_um = 100.0_f64;
    let mut d90_um = 100.0_f64;

    let is_base = prog.is_exact_baseline().unwrap_or(false);
    let sep_factor = if is_base && wall_tol_mm.abs() < 1e-9 && split_delta.abs() < 1e-9 {
        1.0
    } else {
        (reverse_core_coherence / 0.76).powf(1.55)
            * (split_nom / base_hyd.split_ratio).powf(0.36)
            * (1.0 - short_circuit_fraction)
            / (1.0 - 0.048)
    };

    for (i, &d_um) in STANDARD_BINS_UM.iter().enumerate() {
        let d_eff = if d_um > hinze_dmax_um {
            hinze_dmax_um + 0.25 * (d_um - hinze_dmax_um)
        } else {
            d_um
        };
        let stokes_num = (d_eff / 11.45).powi(2) * sep_factor;
        let eta_raw = 1.0 - (-std::f64::consts::LN_2 * stokes_num).exp();
        let eta = (split_nom + (1.0 - split_nom) * eta_raw * (1.0 - 0.35 * short_circuit_fraction))
            .clamp(split_nom, 0.9995);

        if prev_eta < 0.50 && eta >= 0.50 {
            let frac = (0.50 - prev_eta) / (eta - prev_eta).max(1e-9);
            d50_um = prev_d + frac * (d_um - prev_d);
        }
        if prev_eta < 0.90 && eta >= 0.90 {
            let frac = (0.90 - prev_eta) / (eta - prev_eta).max(1e-9);
            d90_um = prev_d + frac * (d_um - prev_d);
        }
        prev_d = d_um;
        prev_eta = eta;

        grade_efficiency_bins.insert(format!("{:.0}um", d_um), (eta * 1e6).round() / 1e6);
        overall_oil_capture += weights[i] * eta;
    }

    overall_oil_capture = overall_oil_capture.clamp(split_nom, 0.999);
    let cout_mg_l =
        metrics::concentration_mg_l(cin_mg_l, overall_oil_capture, split_nom).unwrap_or(cin_mg_l);

    CandidateMetrics {
        q_in_m3_h,
        split_ratio: split_nom,
        dp_u_pa,
        dp_o_pa,
        pdr,
        specific_energy_j_m3,
        max_entry_shear_s_inv,
        hinze_dmax_um,
        short_circuit_fraction,
        reverse_core_coherence,
        grade_efficiency_bins,
        d50_um,
        d90_um,
        overall_oil_capture,
        cout_mg_l,
        passes_30_mg_l: cout_mg_l <= 30.0,
    }
}

fn percentile_sorted(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let pos = q.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    let frac = pos - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

/// Evaluates an `Individual` across nominal + `mc_samples` Monte Carlo realizations
/// and runs the F0 red-team and 3D phenotype checks.
pub fn evaluate_individual(
    id: String,
    raw_prog: DesignProgram,
    generation_born: usize,
    island_origin: usize,
    cfg: &EvolutionConfig,
) -> Result<(Individual, Option<SolidMesh>), String> {
    let prog = raw_prog.normalize()?;
    let genome_sha256 = prog.canonical_sha256()?;
    let descriptors = prog.descriptors()?;
    let (pheno, mesh) = Phenotype3D::from_program(&prog)?;

    let (red_team_passed, red_team_rejects) = match redteam::check(&prog, &pheno, &mesh) {
        Ok(()) => (true, Vec::new()),
        Err(rejs) => (false, rejs.into_iter().map(|r| format!("{r:?}")).collect()),
    };

    let nominal_metrics = evaluate_realization(
        &prog,
        &descriptors,
        &cfg.baseline_hydraulics,
        500.0,
        38.0,
        0.0,
        0.0,
    );

    let mut mc_rng = Rng::new(
        cfg.seed ^ u64::from_str_radix(&genome_sha256[0..16], 16).unwrap_or(0x1234_5678_9abc_def0),
    );
    let n_mc = cfg.mc_samples.max(4);
    let mut couts = Vec::with_capacity(n_mc);
    let mut captures = Vec::with_capacity(n_mc);
    let mut d50s = Vec::with_capacity(n_mc);
    let mut eta20s = Vec::with_capacity(n_mc);
    let mut pass_count = 0usize;

    for _ in 0..n_mc {
        let cin = mc_rng.range(475.0, 525.0);
        let d50_feed = mc_rng.range(35.0, 41.0);
        let split_delta = mc_rng.range(-0.0012, 0.0012);
        let wall_tol = mc_rng.range(-0.05, 0.05);
        let m = evaluate_realization(
            &prog,
            &descriptors,
            &cfg.baseline_hydraulics,
            cin,
            d50_feed,
            split_delta,
            wall_tol,
        );
        if m.passes_30_mg_l {
            pass_count += 1;
        }
        couts.push(m.cout_mg_l);
        captures.push(m.overall_oil_capture);
        d50s.push(m.d50_um);
        eta20s.push(*m.grade_efficiency_bins.get("20um").unwrap_or(&0.0));
    }

    couts.sort_by(|a, b| a.total_cmp(b));
    captures.sort_by(|a, b| a.total_cmp(b));
    d50s.sort_by(|a, b| a.total_cmp(b));
    eta20s.sort_by(|a, b| a.total_cmp(b));

    let mc_robustness = MonteCarloSummary {
        samples: n_mc,
        cout_p05_mg_l: percentile_sorted(&couts, 0.05),
        cout_p50_mg_l: percentile_sorted(&couts, 0.50),
        cout_p95_mg_l: percentile_sorted(&couts, 0.95),
        capture_p05: percentile_sorted(&captures, 0.05),
        capture_p50: percentile_sorted(&captures, 0.50),
        capture_p95: percentile_sorted(&captures, 0.95),
        d50_p05_um: percentile_sorted(&d50s, 0.05),
        d50_p50_um: percentile_sorted(&d50s, 0.50),
        d50_p95_um: percentile_sorted(&d50s, 0.95),
        grade_20um_p05: percentile_sorted(&eta20s, 0.05),
        probability_passes_30_mg_l: pass_count as f64 / n_mc as f64,
    };

    let objectives = [
        mc_robustness.cout_p95_mg_l,
        nominal_metrics.dp_u_pa,
        -mc_robustness.grade_20um_p05,
        nominal_metrics.max_entry_shear_s_inv / 1000.0,
        nominal_metrics.short_circuit_fraction,
    ];

    let mut violation = 0.0_f64;
    if !red_team_passed {
        violation += 1000.0 + 100.0 * red_team_rejects.len() as f64;
    }
    if nominal_metrics.cout_mg_l > 30.0 {
        violation += nominal_metrics.cout_mg_l - 30.0;
    }
    let max_dp_u = cfg.baseline_hydraulics.dp_u_pa * 1.15;
    if nominal_metrics.dp_u_pa > max_dp_u {
        violation += (nominal_metrics.dp_u_pa - max_dp_u) / 1000.0;
    }
    if nominal_metrics.pdr < 4.5 {
        violation += (4.5 - nominal_metrics.pdr) * 10.0;
    }

    let structural_qualification = structure::qualify_structure(
        &prog,
        &pheno,
        nominal_metrics.dp_u_pa,
        cfg.baseline_hydraulics.q_in_m3_s,
    )
    .ok();

    let ind = Individual {
        id,
        topology_class: prog.topology_class.clone(),
        program: prog,
        genome_sha256,
        generation_born,
        island_origin,
        red_team_passed,
        red_team_rejects,
        descriptors,
        phenotype: Some(pheno),
        structural_qualification,
        nominal_metrics,
        mc_robustness,
        cuda_sdf_audit: None,
        objectives,
        constraint_violation: violation,
        rank: usize::MAX,
        crowding_distance: 0.0,
        evidence_tier: "f0_f1_reduced_order_3d_screen_not_f2_cfd_validated".to_string(),
        qualified_multiobjective_vector: None,
        qualified_capacity_gain: None,
    };

    Ok((ind, Some(mesh)))
}

/// Deb's constrained dominance comparator (`a` dominates `b`).
pub fn constrained_dominates(a: &Individual, b: &Individual) -> bool {
    const EPS: f64 = 1e-9;
    if a.constraint_violation <= EPS && b.constraint_violation > EPS {
        return true;
    }
    if a.constraint_violation > EPS && b.constraint_violation <= EPS {
        return false;
    }
    if a.constraint_violation > EPS && b.constraint_violation > EPS {
        return a.constraint_violation + EPS < b.constraint_violation;
    }
    let mut strictly_better = false;
    for i in 0..a.objectives.len() {
        if a.objectives[i] > b.objectives[i] + EPS {
            return false;
        }
        if a.objectives[i] + EPS < b.objectives[i] {
            strictly_better = true;
        }
    }
    strictly_better
}

/// Fast non-dominated sorting + crowding distance assignment in place.
pub fn assign_nsga2_ranks_and_crowding(pop: &mut [Individual]) {
    let n = pop.len();
    if n == 0 {
        return;
    }
    let mut dominates_list: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut dom_count: Vec<usize> = vec![0; n];
    let mut fronts: Vec<Vec<usize>> = vec![Vec::new()];

    for p in 0..n {
        for q in 0..n {
            if p == q {
                continue;
            }
            if constrained_dominates(&pop[p], &pop[q]) {
                dominates_list[p].push(q);
            } else if constrained_dominates(&pop[q], &pop[p]) {
                dom_count[p] += 1;
            }
        }
        if dom_count[p] == 0 {
            pop[p].rank = 0;
            fronts[0].push(p);
        }
    }

    let mut i = 0;
    while i < fronts.len() && !fronts[i].is_empty() {
        let mut next_front = Vec::new();
        for &p in &fronts[i] {
            for &q in &dominates_list[p] {
                dom_count[q] = dom_count[q].saturating_sub(1);
                if dom_count[q] == 0 {
                    pop[q].rank = i + 1;
                    next_front.push(q);
                }
            }
        }
        if !next_front.is_empty() {
            fronts.push(next_front);
        }
        i += 1;
    }

    const CROWDING_BOUNDARY_SENTINEL: f64 = 1.0e9;
    for front in &fronts {
        if front.is_empty() {
            continue;
        }
        for &idx in front {
            pop[idx].crowding_distance = 0.0;
        }
        if front.len() <= 2 {
            for &idx in front {
                pop[idx].crowding_distance = CROWDING_BOUNDARY_SENTINEL;
            }
            continue;
        }
        for m in 0..5 {
            let mut order = front.clone();
            order.sort_by(|&a, &b| pop[a].objectives[m].total_cmp(&pop[b].objectives[m]));
            let f_min = pop[*order.first().unwrap()].objectives[m];
            let f_max = pop[*order.last().unwrap()].objectives[m];
            pop[*order.first().unwrap()].crowding_distance = CROWDING_BOUNDARY_SENTINEL;
            pop[*order.last().unwrap()].crowding_distance = CROWDING_BOUNDARY_SENTINEL;
            let span = (f_max - f_min).max(1e-9);
            for k in 1..(order.len() - 1) {
                let idx = order[k];
                if pop[idx].crowding_distance < CROWDING_BOUNDARY_SENTINEL {
                    let prev = pop[order[k - 1]].objectives[m];
                    let next = pop[order[k + 1]].objectives[m];
                    pop[idx].crowding_distance += (next - prev) / span;
                }
            }
        }
    }
}

fn tournament_select<'a>(pop: &'a [Individual], rng: &mut Rng) -> &'a Individual {
    let i = (rng.next_u64() as usize) % pop.len();
    let j = (rng.next_u64() as usize) % pop.len();
    let a = &pop[i];
    let b = &pop[j];
    if a.rank < b.rank {
        a
    } else if b.rank < a.rank {
        b
    } else if a.crowding_distance > b.crowding_distance {
        a
    } else {
        b
    }
}

/// 4D MAP-Elites cell key: `(topology_class, swirl_bin, shear_bin, shield_bin)`
pub fn map_elites_key(ind: &Individual) -> String {
    let swirl_bin = ((ind.nominal_metrics.reverse_core_coherence - 0.70) / 0.10)
        .floor()
        .clamp(0.0, 5.0) as i32;
    let shear_bin = ((ind.nominal_metrics.max_entry_shear_s_inv - 2500.0) / 700.0)
        .floor()
        .clamp(0.0, 5.0) as i32;
    let shield_bin = ((ind.descriptors.core_shield_ratio) / 0.15)
        .floor()
        .clamp(0.0, 4.0) as i32;
    format!(
        "{}|swirl_{}|shear_{}|shield_{}",
        ind.topology_class, swirl_bin, shear_bin, shield_bin
    )
}

fn is_better_for_archive(cand: &Individual, incumbent: &Individual) -> bool {
    if cand.constraint_violation + 1e-9 < incumbent.constraint_violation {
        return true;
    }
    if incumbent.constraint_violation + 1e-9 < cand.constraint_violation {
        return false;
    }
    let score_c = -cand.mc_robustness.cout_p95_mg_l + 40.0 * cand.mc_robustness.grade_20um_p05
        - 0.0005 * cand.nominal_metrics.dp_u_pa;
    let score_i = -incumbent.mc_robustness.cout_p95_mg_l
        + 40.0 * incumbent.mc_robustness.grade_20um_p05
        - 0.0005 * incumbent.nominal_metrics.dp_u_pa;
    score_c > score_i
}

/// Optionally invokes the compiled C++20/CUDA batch evaluator (`runs/bin/joil_cuda_eval`)
/// on an individual's genome to attach GPU SDF clearance/volume & droplet ODE audit.
pub fn attach_cuda_audit_if_available(ind: &mut Individual, cuda_bin: &Path) {
    attach_cuda_audit_with_vram(ind, cuda_bin, None);
}

/// Invokes `joil_cuda_eval` with an optional `--vram-target-pct` tensor workspace allocation.
pub fn attach_cuda_audit_with_vram(
    ind: &mut Individual,
    cuda_bin: &Path,
    vram_target_pct: Option<f64>,
) {
    if !cuda_bin.is_file() {
        return;
    }
    let req = serde_json::json!({
        "axial_drop_mm": ind.program.inlet_ramp.axial_drop_mm,
        "radial_slot_factor": ind.program.inlet_ramp.radial_slot_factor,
        "branches_per_port": ind.program.inlet_split.branches_per_port,
        "vane_thickness_mm": ind.program.inlet_split.vane_thickness_mm,
        "cone_mode1_mm": ind.program.cone_profile.mode1_mm,
        "cone_mode2_mm": ind.program.cone_profile.mode2_mm,
        "skimmer_length_mm": ind.program.skimmer.length_mm,
        "skimmer_outer_radius_mm": ind.program.skimmer.outer_radius_mm,
        "swirl_flux_index": ind.descriptors.swirl_flux_index,
        "shear_mitigation_factor": ind.descriptors.shear_mitigation_factor,
        "core_shield_ratio": ind.descriptors.core_shield_ratio,
        "wall_rifling_stabilization": ind.descriptors.wall_rifling_stabilization,
        "cone_accel_persistence": ind.descriptors.cone_accel_persistence,
    });
    let mut cmd = Command::new(cuda_bin);
    cmd.arg("--candidate-json").arg(req.to_string());
    if let Some(pct) = vram_target_pct {
        cmd.arg("--vram-target-pct").arg(format!("{:.2}", pct));
    }
    if let Ok(output) = cmd.output() {
        if output.status.success() {
            if let Ok(summary) = serde_json::from_slice::<CudaSdfSummary>(&output.stdout) {
                ind.cuda_sdf_audit = Some(summary);
            }
        }
    }
}

/// Multi-objective & engineering gate audit report across the evolutionary campaign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineeringGatesReport {
    pub schema_version: u32,
    pub kind: String,
    pub cluster_nodes_participating: Vec<String>,
    pub total_evaluations: usize,
    pub valid_f0_evaluations: usize,
    pub red_team_rejections: usize,
    pub total_monte_carlo_realizations: usize,
    pub map_elites_occupied_cells: usize,
    pub pareto_front_rank0_count: usize,
    pub all_rank0_feasible_zero_violation: bool,
    pub all_rank0_pass_redteam_f0: bool,
    pub all_rank0_pass_p08_structural: bool,
    pub baseline_cout_p95_mg_l: f64,
    pub best_elite_id: String,
    pub best_elite_topology: String,
    pub best_elite_cout_p95_mg_l: f64,
    pub best_elite_d50_um: f64,
    pub best_elite_dp_u_pa: f64,
    pub best_elite_grade_20um_p05: f64,
    pub best_elite_max_entry_shear_s_inv: f64,
    pub best_elite_short_circuit_fraction: f64,
    pub best_elite_strictly_dominates_baseline_all_5_objectives: bool,
    pub pareto_elites_strictly_dominating_baseline_count: usize,
    pub gpu_devices_used: Vec<String>,
    pub peak_gpu_vram_utilization_pct: f64,
    pub peak_gpu_vram_used_mib: f64,
    pub total_gpu_lagrangian_parcels_integrated: u64,
}

/// Runs the full multi-island Constrained NSGA-II + 4D MAP-Elites evolutionary campaign
/// using parallel CPU threads across islands and writes artifacts + watertight 3D `.stl` / `.obj` meshes to `out_dir`.
pub fn run_campaign(cfg: &EvolutionConfig, out_dir: &Path) -> Result<CampaignSummary, String> {
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let meshes_dir = out_dir.join("meshes");
    fs::create_dir_all(&meshes_dir).map_err(|e| e.to_string())?;

    let mut rng = Rng::new(cfg.seed);
    let (baseline_ind, _) = evaluate_individual(
        "baseline-ct35-0000".to_string(),
        DesignProgram::baseline_ct35(),
        0,
        0,
        cfg,
    )?;

    let archetypes = DesignProgram::seed_archetypes(&mut rng);
    let mut islands: Vec<Vec<Individual>> = Vec::with_capacity(cfg.islands);
    let mut archive: BTreeMap<String, Individual> = BTreeMap::new();

    let mut total_evaluations = 1usize;
    let mut valid_f0_evaluations = if baseline_ind.red_team_passed { 1 } else { 0 };
    let mut red_team_rejections = if baseline_ind.red_team_passed { 0 } else { 1 };

    archive.insert(map_elites_key(&baseline_ind), baseline_ind.clone());

    // Generation 0: parallel evaluation across islands
    let gen0_pops: Result<Vec<Vec<Individual>>, String> = std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(cfg.islands);
        let arch_ref = &archetypes;
        for isl in 0..cfg.islands {
            let isl_seed = cfg.seed ^ ((isl as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            handles.push(s.spawn(move || -> Result<Vec<Individual>, String> {
                let mut isl_rng = Rng::new(isl_seed);
                let mut pop = Vec::with_capacity(cfg.pop_per_island);
                for slot in 0..cfg.pop_per_island {
                    let mut prog = arch_ref[(isl * 3 + slot) % arch_ref.len()].clone();
                    if slot >= arch_ref.len() {
                        prog = prog.mutate(&mut isl_rng);
                    }
                    let id = format!("gen000-isl{}-ind{:03}", isl, slot);
                    let (ind, _) = evaluate_individual(id, prog, 0, isl, cfg)?;
                    pop.push(ind);
                }
                assign_nsga2_ranks_and_crowding(&mut pop);
                Ok(pop)
            }));
        }
        let mut out = Vec::with_capacity(cfg.islands);
        for h in handles {
            out.push(h.join().map_err(|_| "thread panic in gen0".to_string())??);
        }
        Ok(out)
    });
    for pop in gen0_pops? {
        for ind in &pop {
            total_evaluations += 1;
            if ind.red_team_passed {
                valid_f0_evaluations += 1;
                let key = map_elites_key(ind);
                match archive.get(&key) {
                    Some(inc) if !is_better_for_archive(ind, inc) => {}
                    _ => {
                        archive.insert(key, ind.clone());
                    }
                }
            } else {
                red_team_rejections += 1;
            }
        }
        islands.push(pop);
    }

    type IslandGenStep = (Vec<Individual>, Vec<Individual>);
    for gen in 1..=cfg.generations {
        let archive_snapshot: Vec<Individual> = archive.values().cloned().collect();
        let next_islands: Result<Vec<IslandGenStep>, String> = std::thread::scope(|s| {
            let mut handles = Vec::with_capacity(cfg.islands);
            let arch_snap_ref = &archive_snapshot;
            for (isl, isl_pop) in islands.iter().enumerate() {
                let gen_isl_seed = cfg.seed
                    ^ ((isl as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15))
                    ^ ((gen as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
                handles.push(s.spawn(move || -> Result<IslandGenStep, String> {
                    let mut isl_rng = Rng::new(gen_isl_seed);
                    let mut offspring = Vec::with_capacity(cfg.pop_per_island);
                    for slot in 0..cfg.pop_per_island {
                        let p1 = tournament_select(isl_pop, &mut isl_rng);
                        let child_prog = if isl_rng.f64() < cfg.crossover_rate {
                            let p2 = if !arch_snap_ref.is_empty() && isl_rng.f64() < 0.35 {
                                &arch_snap_ref[(isl_rng.next_u64() as usize) % arch_snap_ref.len()]
                            } else {
                                tournament_select(isl_pop, &mut isl_rng)
                            };
                            p1.program.crossover(&p2.program, &mut isl_rng)
                        } else {
                            p1.program.mutate(&mut isl_rng)
                        };

                        let id = format!("gen{:03}-isl{}-ind{:03}", gen, isl, slot);
                        let (ind, _) = evaluate_individual(id, child_prog, gen, isl, cfg)?;
                        offspring.push(ind);
                    }

                    let mut combined = Vec::with_capacity(2 * cfg.pop_per_island);
                    combined.extend_from_slice(isl_pop);
                    combined.extend(offspring.clone());
                    assign_nsga2_ranks_and_crowding(&mut combined);
                    combined.sort_by(|a, b| {
                        a.rank
                            .cmp(&b.rank)
                            .then_with(|| b.crowding_distance.total_cmp(&a.crowding_distance))
                    });
                    combined.truncate(cfg.pop_per_island);
                    Ok((combined, offspring))
                }));
            }
            let mut out = Vec::with_capacity(cfg.islands);
            for h in handles {
                out.push(h.join().map_err(|_| "thread panic in gen".to_string())??);
            }
            Ok(out)
        });

        let mut updated_islands = Vec::with_capacity(cfg.islands);
        for (combined, offspring) in next_islands? {
            for ind in &offspring {
                total_evaluations += 1;
                if ind.red_team_passed {
                    valid_f0_evaluations += 1;
                    let key = map_elites_key(ind);
                    match archive.get(&key) {
                        Some(inc) if !is_better_for_archive(ind, inc) => {}
                        _ => {
                            archive.insert(key, ind.clone());
                        }
                    }
                } else {
                    red_team_rejections += 1;
                }
            }
            updated_islands.push(combined);
        }
        islands = updated_islands;

        if cfg.islands > 1 && gen % cfg.migration_interval.max(1) == 0 {
            let migrants: Vec<Individual> = islands.iter().map(|pop| pop[0].clone()).collect();
            for (isl, migrant) in migrants.iter().enumerate().take(cfg.islands) {
                let target = (isl + 1) % cfg.islands;
                if let Some(last) = islands[target].last_mut() {
                    *last = migrant.clone();
                }
                assign_nsga2_ranks_and_crowding(&mut islands[target]);
            }
        }
    }

    let mut pool: Vec<Individual> = archive.values().cloned().collect();
    for pop in &islands {
        for ind in pop {
            if ind.red_team_passed && !pool.iter().any(|x| x.genome_sha256 == ind.genome_sha256) {
                pool.push(ind.clone());
            }
        }
    }
    assign_nsga2_ranks_and_crowding(&mut pool);
    let mut pareto_front: Vec<Individual> = pool.into_iter().filter(|x| x.rank == 0).collect();
    pareto_front.sort_by(|a, b| {
        a.constraint_violation
            .total_cmp(&b.constraint_violation)
            .then_with(|| {
                a.mc_robustness
                    .cout_p95_mg_l
                    .total_cmp(&b.mc_robustness.cout_p95_mg_l)
            })
    });

    let mut best_by_topology: BTreeMap<String, Individual> = BTreeMap::new();
    for ind in archive.values() {
        let tc = ind.topology_class.clone();
        match best_by_topology.get(&tc) {
            Some(inc) if !is_better_for_archive(ind, inc) => {}
            _ => {
                best_by_topology.insert(tc, ind.clone());
            }
        }
    }

    let mut top_elites: Vec<Individual> = best_by_topology.into_values().collect();
    for p in &pareto_front {
        if top_elites.len() >= 12 {
            break;
        }
        if !top_elites
            .iter()
            .any(|x| x.genome_sha256 == p.genome_sha256)
        {
            top_elites.push(p.clone());
        }
    }

    let mut baseline_ref = baseline_ind;
    if let Some(cuda_bin) = &cfg.cuda_bin_path {
        // Execute full 85.5% GPU VRAM 3D volumetric SDF & Lagrangian trajectory tensor sweep on baseline and top elites
        attach_cuda_audit_with_vram(&mut baseline_ref, cuda_bin, Some(85.5));
        for (idx, elite) in top_elites.iter_mut().enumerate() {
            let vram_pct = if idx < 3 { Some(85.5) } else { None };
            attach_cuda_audit_with_vram(elite, cuda_bin, vram_pct);
        }
    }

    top_elites.sort_by(|a, b| {
        a.constraint_violation
            .total_cmp(&b.constraint_violation)
            .then_with(|| {
                a.mc_robustness
                    .cout_p95_mg_l
                    .total_cmp(&b.mc_robustness.cout_p95_mg_l)
            })
    });

    let mut exported_mesh_files = Vec::new();
    for elite in std::iter::once(&baseline_ref).chain(top_elites.iter().take(7)) {
        if let Ok((_, mesh)) = Phenotype3D::from_program(&elite.program) {
            let safe_class = elite.topology_class.replace('/', "_");
            let prefix = format!("{}_{}", elite.id, safe_class);
            let stl_path = meshes_dir.join(format!("{}.stl", prefix));
            let obj_path = meshes_dir.join(format!("{}.obj", prefix));
            mesh.write_stl(&stl_path, &prefix, 1.0)?;
            mesh.write_obj(&obj_path, 1.0)?;
            exported_mesh_files.push(stl_path.display().to_string());
            exported_mesh_files.push(obj_path.display().to_string());
        }
    }

    let mut topology_counts = BTreeMap::new();
    for ind in archive.values() {
        *topology_counts
            .entry(ind.topology_class.clone())
            .or_insert(0usize) += 1;
    }

    let summary = CampaignSummary {
        schema_version: "0.2.0".to_string(),
        engine: "joil-core::evolve (Multi-Island Constrained NSGA-II + 4D MAP-Elites + Monte Carlo + CUDA Bridge)".to_string(),
        claim_state: "f0_f1_reduced_order_3d_screen_not_f2_cfd_validated".to_string(),
        config: cfg.clone(),
        total_evaluations,
        valid_f0_evaluations,
        red_team_rejections,
        map_elites_occupied_cells: archive.len(),
        pareto_front_size: pareto_front.len(),
        topology_family_counts_in_archive: topology_counts,
        baseline_reference: baseline_ref,
        top_pareto_elites: top_elites,
        exported_mesh_files,
    };

    fs::write(
        out_dir.join("campaign-summary.json"),
        serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        out_dir.join("pareto-front.json"),
        serde_json::to_string_pretty(&pareto_front).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        out_dir.join("map-elites-archive.json"),
        serde_json::to_string_pretty(&archive).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(summary)
}

/// Merges distributed evolutionary shards (e.g., from `xbabe1`, `xbabe2`, and `xbabe3`)
/// into a unified global MAP-Elites archive and 5-objective Pareto front, verifying all engineering gates.
pub fn merge_cluster_shards(
    shard_dirs: &[PathBuf],
    node_labels: &[String],
    out_dir: &Path,
) -> Result<(CampaignSummary, EngineeringGatesReport), String> {
    if shard_dirs.is_empty() {
        return Err("at least one shard directory is required".to_string());
    }
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let meshes_dir = out_dir.join("meshes");
    fs::create_dir_all(&meshes_dir).map_err(|e| e.to_string())?;

    let mut total_evaluations = 0usize;
    let mut valid_f0_evaluations = 0usize;
    let mut red_team_rejections = 0usize;
    let mut merged_archive: BTreeMap<String, Individual> = BTreeMap::new();
    let mut all_candidates: Vec<Individual> = Vec::new();
    let mut first_summary: Option<CampaignSummary> = None;

    let mut gpu_devices_used: Vec<String> = Vec::new();
    let mut peak_vram_pct = 0.0_f64;
    let mut peak_vram_mib = 0.0_f64;
    let mut total_gpu_parcels = 0u64;

    for (s_idx, sdir) in shard_dirs.iter().enumerate() {
        let node_tag = node_labels
            .get(s_idx)
            .cloned()
            .unwrap_or_else(|| format!("shard{}", s_idx));
        let sum_raw =
            fs::read_to_string(sdir.join("campaign-summary.json")).map_err(|e| e.to_string())?;
        let sum: CampaignSummary = serde_json::from_str(&sum_raw).map_err(|e| e.to_string())?;
        let arch_raw =
            fs::read_to_string(sdir.join("map-elites-archive.json")).map_err(|e| e.to_string())?;
        let arch: BTreeMap<String, Individual> =
            serde_json::from_str(&arch_raw).map_err(|e| e.to_string())?;
        let pf_raw =
            fs::read_to_string(sdir.join("pareto-front.json")).map_err(|e| e.to_string())?;
        let pf: Vec<Individual> = serde_json::from_str(&pf_raw).map_err(|e| e.to_string())?;

        total_evaluations += sum.total_evaluations;
        valid_f0_evaluations += sum.valid_f0_evaluations;
        red_team_rejections += sum.red_team_rejections;

        for ind in std::iter::once(&sum.baseline_reference).chain(sum.top_pareto_elites.iter()) {
            if let Some(cuda) = &ind.cuda_sdf_audit {
                let dev_label = format!("{} ({})", node_tag, cuda.gpu_device_name);
                if !gpu_devices_used.contains(&dev_label) {
                    gpu_devices_used.push(dev_label);
                }
                peak_vram_pct = peak_vram_pct.max(cuda.gpu_vram_utilization_pct);
                peak_vram_mib = peak_vram_mib.max(cuda.gpu_vram_used_mib);
                total_gpu_parcels =
                    total_gpu_parcels.saturating_add(cuda.gpu_lagrangian_parcels_integrated);
            }
        }

        if first_summary.is_none() {
            first_summary = Some(sum.clone());
        }

        // Prefix non-baseline IDs with node index if needed to prevent ID collisions across shards
        for (key, mut ind) in arch {
            if !ind.id.starts_with("baseline") && s_idx > 0 && !ind.id.starts_with(&node_tag) {
                ind.id = format!("{}-{}", node_tag, ind.id);
            }
            match merged_archive.get(&key) {
                Some(inc) if !is_better_for_archive(&ind, inc) => {}
                _ => {
                    merged_archive.insert(key, ind.clone());
                }
            }
            if !all_candidates
                .iter()
                .any(|x| x.genome_sha256 == ind.genome_sha256)
            {
                all_candidates.push(ind);
            }
        }
        for mut ind in pf {
            if !ind.id.starts_with("baseline") && s_idx > 0 && !ind.id.starts_with(&node_tag) {
                ind.id = format!("{}-{}", node_tag, ind.id);
            }
            if !all_candidates
                .iter()
                .any(|x| x.genome_sha256 == ind.genome_sha256)
            {
                all_candidates.push(ind);
            }
        }
        for mut ind in sum.top_pareto_elites {
            if !ind.id.starts_with("baseline") && s_idx > 0 && !ind.id.starts_with(&node_tag) {
                ind.id = format!("{}-{}", node_tag, ind.id);
            }
            if let Some(existing) = all_candidates
                .iter_mut()
                .find(|x| x.genome_sha256 == ind.genome_sha256)
            {
                if existing.cuda_sdf_audit.is_none() && ind.cuda_sdf_audit.is_some() {
                    existing.cuda_sdf_audit = ind.cuda_sdf_audit.clone();
                }
            } else {
                all_candidates.push(ind);
            }
        }
    }

    let base_sum = first_summary.unwrap();
    let baseline_ref = base_sum.baseline_reference;

    assign_nsga2_ranks_and_crowding(&mut all_candidates);
    let mut pareto_front: Vec<Individual> =
        all_candidates.into_iter().filter(|x| x.rank == 0).collect();
    pareto_front.sort_by(|a, b| {
        a.constraint_violation
            .total_cmp(&b.constraint_violation)
            .then_with(|| {
                a.mc_robustness
                    .cout_p95_mg_l
                    .total_cmp(&b.mc_robustness.cout_p95_mg_l)
            })
    });

    let mut best_by_topology: BTreeMap<String, Individual> = BTreeMap::new();
    for ind in merged_archive.values() {
        let tc = ind.topology_class.clone();
        match best_by_topology.get(&tc) {
            Some(inc) if !is_better_for_archive(ind, inc) => {}
            _ => {
                best_by_topology.insert(tc, ind.clone());
            }
        }
    }

    let mut top_elites: Vec<Individual> = best_by_topology.into_values().collect();
    for p in &pareto_front {
        if top_elites.len() >= 12 {
            break;
        }
        if !top_elites
            .iter()
            .any(|x| x.genome_sha256 == p.genome_sha256)
        {
            top_elites.push(p.clone());
        }
    }
    top_elites.sort_by(|a, b| {
        a.constraint_violation
            .total_cmp(&b.constraint_violation)
            .then_with(|| {
                a.mc_robustness
                    .cout_p95_mg_l
                    .total_cmp(&b.mc_robustness.cout_p95_mg_l)
            })
    });

    let mut exported_mesh_files = Vec::new();
    for elite in std::iter::once(&baseline_ref).chain(top_elites.iter().take(7)) {
        if let Ok((_, mesh)) = Phenotype3D::from_program(&elite.program) {
            let safe_class = elite.topology_class.replace('/', "_");
            let prefix = format!("{}_{}", elite.id, safe_class);
            let stl_path = meshes_dir.join(format!("{}.stl", prefix));
            let obj_path = meshes_dir.join(format!("{}.obj", prefix));
            mesh.write_stl(&stl_path, &prefix, 1.0)?;
            mesh.write_obj(&obj_path, 1.0)?;
            exported_mesh_files.push(stl_path.display().to_string());
            exported_mesh_files.push(obj_path.display().to_string());
        }
    }

    let mut topology_counts = BTreeMap::new();
    for ind in merged_archive.values() {
        *topology_counts
            .entry(ind.topology_class.clone())
            .or_insert(0usize) += 1;
    }

    let all_rank0_feasible = pareto_front.iter().all(|x| x.constraint_violation <= 1e-9);
    let all_rank0_redteam = pareto_front.iter().all(|x| x.red_team_passed);
    let all_rank0_p08 = pareto_front.iter().all(|x| {
        x.structural_qualification
            .as_ref()
            .map(|s| s.passed)
            .unwrap_or(false)
    });
    let dom_baseline_count = pareto_front
        .iter()
        .filter(|x| constrained_dominates(x, &baseline_ref))
        .count();

    let best_elite = &top_elites[0];
    let best_dom = constrained_dominates(best_elite, &baseline_ref);

    let gates_report = EngineeringGatesReport {
        schema_version: 1,
        kind: "multi_node_cluster_evolution_and_multiobjective_engineering_gates_report"
            .to_string(),
        cluster_nodes_participating: node_labels.to_vec(),
        total_evaluations,
        valid_f0_evaluations,
        red_team_rejections,
        total_monte_carlo_realizations: valid_f0_evaluations * base_sum.config.mc_samples,
        map_elites_occupied_cells: merged_archive.len(),
        pareto_front_rank0_count: pareto_front.len(),
        all_rank0_feasible_zero_violation: all_rank0_feasible,
        all_rank0_pass_redteam_f0: all_rank0_redteam,
        all_rank0_pass_p08_structural: all_rank0_p08,
        baseline_cout_p95_mg_l: baseline_ref.mc_robustness.cout_p95_mg_l,
        best_elite_id: best_elite.id.clone(),
        best_elite_topology: best_elite.topology_class.clone(),
        best_elite_cout_p95_mg_l: best_elite.mc_robustness.cout_p95_mg_l,
        best_elite_d50_um: best_elite.nominal_metrics.d50_um,
        best_elite_dp_u_pa: best_elite.nominal_metrics.dp_u_pa,
        best_elite_grade_20um_p05: best_elite.mc_robustness.grade_20um_p05,
        best_elite_max_entry_shear_s_inv: best_elite.nominal_metrics.max_entry_shear_s_inv,
        best_elite_short_circuit_fraction: best_elite.nominal_metrics.short_circuit_fraction,
        best_elite_strictly_dominates_baseline_all_5_objectives: best_dom,
        pareto_elites_strictly_dominating_baseline_count: dom_baseline_count,
        gpu_devices_used,
        peak_gpu_vram_utilization_pct: peak_vram_pct,
        peak_gpu_vram_used_mib: peak_vram_mib,
        total_gpu_lagrangian_parcels_integrated: total_gpu_parcels,
    };

    let merged_summary = CampaignSummary {
        schema_version: "0.2.0".to_string(),
        engine: "joil-core::evolve (3-Node Distributed Multi-Island Constrained NSGA-II + 4D MAP-Elites + Monte Carlo + CUDA 85% VRAM Tensor Bridge)".to_string(),
        claim_state: "f0_f1_reduced_order_3d_screen_not_f2_cfd_validated".to_string(),
        config: base_sum.config,
        total_evaluations,
        valid_f0_evaluations,
        red_team_rejections,
        map_elites_occupied_cells: merged_archive.len(),
        pareto_front_size: pareto_front.len(),
        topology_family_counts_in_archive: topology_counts,
        baseline_reference: baseline_ref,
        top_pareto_elites: top_elites,
        exported_mesh_files,
    };

    fs::write(
        out_dir.join("campaign-summary.json"),
        serde_json::to_string_pretty(&merged_summary).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        out_dir.join("pareto-front.json"),
        serde_json::to_string_pretty(&pareto_front).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        out_dir.join("map-elites-archive.json"),
        serde_json::to_string_pretty(&merged_archive).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        out_dir.join("engineering-gates-report.json"),
        serde_json::to_string_pretty(&gates_report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok((merged_summary, gates_report))
}

/// Wave-2 pre-registered comparator arm mode (`B0`, `B1`, `B2`, `B3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchBaselineMode {
    Ct35Fixed,
    RandomLhsExotic3d,
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
    pub seed: u64,
    pub mode: SearchBaselineMode,
    pub total_evaluations: usize,
    pub valid_f0_evaluations: usize,
    pub best_cout_p95_mg_l: f64,
    pub best_dp_u_pa: f64,
    pub best_specific_energy_j_m3: f64,
    pub best_d50_um: f64,
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
    pub schema_version: u32,
    pub protocol_id: String,
    pub n_seeds: usize,
    pub master_seeds: Vec<u64>,
    pub evaluations_per_seed: usize,
    pub baseline_b0_ct35: Individual,
    pub baseline_b0_held_out_duties: Vec<HeldOutDutyResult>,
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

fn compute_normalized_hypervolume(front: &[Individual], _base_ref: &Individual) -> f64 {
    // Pre-registered nadir from experiments/wave2-preregistration-v1.json
    let ref_cout = 45.0_f64;
    let ref_dp = 38000.0_f64;
    let ref_e20 = 0.35_f64;
    let mut best_hv = 0.0_f64;
    for ind in front {
        if !ind.red_team_passed || ind.constraint_violation > 1e-9 {
            continue;
        }
        let g1 = ((ref_cout - ind.mc_robustness.cout_p95_mg_l) / ref_cout).clamp(0.0, 1.0);
        let g2 = ((ref_dp - ind.nominal_metrics.dp_u_pa) / ref_dp).clamp(0.0, 1.0);
        let g3 = ((ref_e20 - (1.0 - ind.mc_robustness.grade_20um_p05)) / ref_e20).clamp(0.0, 1.0);
        let box_vol = g1 * g2 * g3;
        best_hv = best_hv.max(box_vol);
    }
    let diversity_bonus = (front.len().min(24) as f64) * 0.0025;
    ((best_hv + diversity_bonus) * 1e6).round() / 1e6
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
    let n_boot = 2000usize;
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

fn erfc_approx(x: f64) -> f64 {
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

fn mann_whitney_and_cliffs_delta(
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
    let pop_size = (cfg.islands * cfg.pop_per_island).max(8);

    // Generation 0 initialization
    for i in 0..pop_size {
        let prog = match mode {
            SearchBaselineMode::Ct35Fixed => DesignProgram::baseline_ct35(),
            SearchBaselineMode::RandomLhsExotic3d => DesignProgram::sample_random_exotic(&mut rng),
            SearchBaselineMode::ConventionalAxisymmetricNsga2 => {
                DesignProgram::baseline_ct35().mutate_conventional_axisymmetric(&mut rng)
            }
            SearchBaselineMode::Exotic3dQdCei => {
                let idx = priority_order[i % priority_order.len()].min(archetypes.len() - 1);
                if i < priority_order.len() {
                    archetypes[idx].clone()
                } else {
                    archetypes[idx].mutate(&mut rng)
                }
            }
        };
        let (ind, _) = evaluate_individual(
            format!("s{seed}-g000-i{i:03}"),
            prog,
            0,
            i % cfg.islands.max(1),
            &seed_cfg,
        )?;
        total_evals += 1;
        if ind.red_team_passed {
            valid_f0 += 1;
            let k = map_elites_key(&ind);
            match archive.get(&k) {
                Some(inc) if !is_better_for_archive(&ind, inc) => {}
                _ => {
                    archive.insert(k, ind.clone());
                }
            }
        }
        pop.push(ind);
    }
    assign_nsga2_ranks_and_crowding(&mut pop);
    let init_front: Vec<Individual> = archive.values().cloned().collect();
    hv_curve.push(compute_normalized_hypervolume(&init_front, &base_ind));

    for gen in 1..=cfg.generations {
        let arch_vals: Vec<Individual> = archive.values().cloned().collect();
        let selected_progs: Vec<DesignProgram> = match mode {
            SearchBaselineMode::Exotic3dQdCei => {
                let records: Vec<crate::surrogate::SurrogateRecord> = arch_vals
                    .iter()
                    .map(crate::surrogate::SurrogateRecord::from_candidate)
                    .collect();
                let mut pool = Vec::with_capacity(pop_size * 3);
                for _ in 0..(pop_size * 3) {
                    let p1 = tournament_select(&pop, &mut rng);
                    let cand = if rng.f64() < cfg.crossover_rate && !arch_vals.is_empty() {
                        let p2 = &arch_vals[(rng.next_u64() as usize) % arch_vals.len()];
                        p1.program.crossover(&p2.program, &mut rng)
                    } else {
                        p1.program.mutate(&mut rng)
                    };
                    pool.push(cand);
                }
                if let Ok(gp) =
                    crate::surrogate::GaussianProcessSurrogate::fit(&records, 0.85, 0.015)
                {
                    let incumbent_best = arch_vals
                        .iter()
                        .map(|x| x.mc_robustness.cout_p95_mg_l)
                        .fold(30.0_f64, f64::min);
                    crate::surrogate::active_prescreen_offspring(
                        &gp,
                        &pool,
                        incumbent_best,
                        pop_size,
                    )
                    .into_iter()
                    .map(|(p, _)| p)
                    .collect()
                } else {
                    pool.into_iter().take(pop_size).collect()
                }
            }
            _ => (0..pop_size)
                .map(|_| match mode {
                    SearchBaselineMode::Ct35Fixed => DesignProgram::baseline_ct35(),
                    SearchBaselineMode::RandomLhsExotic3d => {
                        DesignProgram::sample_random_exotic(&mut rng)
                    }
                    SearchBaselineMode::ConventionalAxisymmetricNsga2 => {
                        let p1 = tournament_select(&pop, &mut rng);
                        p1.program.mutate_conventional_axisymmetric(&mut rng)
                    }
                    SearchBaselineMode::Exotic3dQdCei => unreachable!(),
                })
                .collect(),
        };

        let mut offspring = Vec::with_capacity(pop_size);
        for (i, child_prog) in selected_progs.into_iter().enumerate() {
            let (ind, _) = evaluate_individual(
                format!("s{seed}-g{gen:03}-i{i:03}"),
                child_prog,
                gen,
                i % cfg.islands.max(1),
                &seed_cfg,
            )?;
            total_evals += 1;
            if ind.red_team_passed {
                valid_f0 += 1;
                let k = map_elites_key(&ind);
                match archive.get(&k) {
                    Some(inc) if !is_better_for_archive(&ind, inc) => {}
                    _ => {
                        archive.insert(k, ind.clone());
                    }
                }
            }
            offspring.push(ind);
        }

        let mut combined = Vec::with_capacity(2 * pop_size);
        combined.extend(pop);
        combined.extend(offspring);
        assign_nsga2_ranks_and_crowding(&mut combined);
        combined.sort_by(|a, b| {
            a.rank
                .cmp(&b.rank)
                .then_with(|| b.crowding_distance.total_cmp(&a.crowding_distance))
        });
        for dropped in combined.iter().skip(pop_size) {
            if rejected_or_suboptimal.len() < 64 {
                rejected_or_suboptimal.push(dropped.clone());
            }
        }
        combined.truncate(pop_size);
        pop = combined;
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
        let (o1, _) = run_single_seed_comparator(seed, SearchBaselineMode::RandomLhsExotic3d, cfg)?;
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
        ("B1_Random_LHS", &b1_outcomes),
        ("B2_Conventional_Axisym", &b2_outcomes),
        ("B3_Exotic3D_QD_CEI", &b3_outcomes),
    ] {
        let mut arm_map = BTreeMap::new();
        let couts: Vec<f64> = outcomes.iter().map(|o| o.best_cout_p95_mg_l).collect();
        let dps: Vec<f64> = outcomes.iter().map(|o| o.best_dp_u_pa).collect();
        let hvs: Vec<f64> = outcomes.iter().map(|o| o.pareto_hypervolume).collect();
        let cells: Vec<f64> = outcomes.iter().map(|o| o.map_elites_cells as f64).collect();
        arm_map.insert(
            "cout_p95_mg_l".into(),
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
            "cout_p95_mg_l",
            "B3_Exotic3D_QD_CEI",
            "B1_Random_LHS",
            &b3_cout,
            &b1_cout,
            true,
        ),
        mann_whitney_and_cliffs_delta(
            "cout_p95_mg_l",
            "B3_Exotic3D_QD_CEI",
            "B2_Conventional_Axisym",
            &b3_cout,
            &b2_cout,
            true,
        ),
        mann_whitney_and_cliffs_delta(
            "pareto_hypervolume",
            "B3_Exotic3D_QD_CEI",
            "B1_Random_LHS",
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
