//! Constrained NSGA-II ranking, 4D MAP-Elites archive, CUDA SDF attachment, and campaign runner.

use super::*;

/// Deb's constrained dominance comparator (`a` dominates `b`).
pub fn constrained_dominates(a: &Individual, b: &Individual) -> bool {
    const EPS: f64 = 1e-9;
    let (Some(a_objectives), Some(b_objectives)) = (a.objectives.values(), b.objectives.values())
    else {
        return false;
    };
    let structurally_feasible = |ind: &Individual| {
        matches!(ind.structural_screen_state, StructuralScreenState::Screened)
            && ind.structural_qualification.as_ref().is_some_and(|screen| {
                screen.passed
                    && screen.genome_sha256 == ind.genome_sha256
                    && structure::structural_load_case_hash(
                        &ind.genome_sha256,
                        ind.nominal_metrics.dp_u_pa,
                        ind.nominal_metrics.q_in_m3_h / 3600.0,
                    )
                    .is_ok_and(|hash| hash == screen.load_case_sha256)
            })
    };
    if structurally_feasible(a) != structurally_feasible(b) {
        return structurally_feasible(a);
    }
    if !structurally_feasible(a) {
        return false;
    }
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
    for i in 0..a_objectives.len() {
        if a_objectives[i] > b_objectives[i] + EPS {
            return false;
        }
        if a_objectives[i] + EPS < b_objectives[i] {
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
        if pop[p].objectives.values().is_none() {
            pop[p].rank = usize::MAX;
            continue;
        }
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
            let mut order: Vec<usize> = front
                .iter()
                .copied()
                .filter(|idx| pop[*idx].objectives.values().is_some())
                .collect();
            if order.is_empty() {
                continue;
            }
            order.sort_by(|&a, &b| {
                pop[a].objectives.values().unwrap()[m]
                    .total_cmp(&pop[b].objectives.values().unwrap()[m])
            });
            let f_min = pop[*order.first().unwrap()].objectives.values().unwrap()[m];
            let f_max = pop[*order.last().unwrap()].objectives.values().unwrap()[m];
            pop[*order.first().unwrap()].crowding_distance = CROWDING_BOUNDARY_SENTINEL;
            pop[*order.last().unwrap()].crowding_distance = CROWDING_BOUNDARY_SENTINEL;
            let span = (f_max - f_min).max(1e-9);
            for k in 1..(order.len() - 1) {
                let idx = order[k];
                if pop[idx].crowding_distance < CROWDING_BOUNDARY_SENTINEL {
                    let prev = pop[order[k - 1]].objectives.values().unwrap()[m];
                    let next = pop[order[k + 1]].objectives.values().unwrap()[m];
                    pop[idx].crowding_distance += (next - prev) / span;
                }
            }
        }
    }
}

pub(crate) fn tournament_select<'a>(pop: &'a [Individual], rng: &mut Rng) -> &'a Individual {
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

pub(crate) fn is_better_for_archive(cand: &Individual, incumbent: &Individual) -> bool {
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
    #[serde(
        rename = "baseline_cout_sample_screen_upper_mg_l",
        alias = "baseline_cout_p95_mg_l"
    )]
    pub baseline_cout_p95_mg_l: f64,
    pub best_elite_id: String,
    pub best_elite_topology: String,
    #[serde(
        rename = "best_elite_cout_sample_screen_upper_mg_l",
        alias = "best_elite_cout_p95_mg_l"
    )]
    pub best_elite_cout_p95_mg_l: f64,
    pub best_elite_d50_um: f64,
    pub best_elite_dp_u_pa: f64,
    #[serde(
        rename = "best_elite_grade_20um_sample_screen_lower",
        alias = "best_elite_grade_20um_p05"
    )]
    pub best_elite_grade_20um_p05: f64,
    pub best_elite_max_entry_shear_s_inv: f64,
    pub best_elite_short_circuit_fraction: f64,
    pub best_elite_strictly_dominates_baseline_all_5_objectives: bool,
    pub pareto_elites_strictly_dominating_baseline_count: usize,
    pub gpu_devices_used: Vec<String>,
    pub peak_gpu_vram_utilization_pct: f64,
    pub peak_gpu_vram_used_mib: f64,
    #[serde(default, alias = "total_gpu_lagrangian_parcels_integrated")]
    pub total_gpu_workspace_tensor_cells_evaluated: u64,
}
