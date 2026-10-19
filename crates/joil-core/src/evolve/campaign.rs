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

/// Runs the full multi-island Constrained NSGA-II + 4D MAP-Elites evolutionary campaign
/// using parallel CPU threads across islands and writes artifacts + watertight 3D `.stl` / `.obj` meshes to `out_dir`.
pub fn run_campaign(cfg: &EvolutionConfig, out_dir: &Path) -> Result<CampaignSummary, String> {
    let campaign_admission = crate::experiment::admit_search_campaign(cfg.campaign_mode)?;
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    fs::write(
        out_dir.join("campaign.json"),
        serde_json::to_vec_pretty(&campaign_admission).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
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
    let mut proposal_budget = ProposalBudget::default();
    proposal_budget.record(VariationOutcome::introduce(
        baseline_ind.program.clone(),
        "insert_baseline_control",
        &mut rng,
    )?);
    let mut islands: Vec<Vec<Individual>> = Vec::with_capacity(cfg.islands);
    let mut archive: BTreeMap<String, Individual> = BTreeMap::new();
    let mut total_evaluations = 1usize;
    let mut valid_f0_evaluations = usize::from(baseline_ind.red_team_passed);
    let mut red_team_rejections = usize::from(!baseline_ind.red_team_passed);
    archive.insert(map_elites_key(&baseline_ind), baseline_ind.clone());

    for isl in 0..cfg.islands {
        let mut isl_rng =
            Rng::new(cfg.seed ^ ((isl as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)));
        let mut pop = Vec::with_capacity(cfg.pop_per_island);
        for slot in 0..cfg.pop_per_island {
            let parent = &archetypes[(isl * 3 + slot) % archetypes.len()];
            let outcome = if slot >= archetypes.len() {
                parent.mutate(&mut isl_rng)
            } else {
                VariationOutcome::introduce(parent.clone(), "insert_catalog_seed", &mut isl_rng)
            };
            if let Some(ind) = evaluate_proposal(
                outcome,
                format!("gen000-isl{isl}-ind{slot:03}"),
                0,
                isl,
                cfg,
                &mut proposal_budget,
            )? {
                total_evaluations += 1;
                if ind.red_team_passed {
                    valid_f0_evaluations += 1;
                    let key = map_elites_key(&ind);
                    if archive
                        .get(&key)
                        .is_none_or(|inc| is_better_for_archive(&ind, inc))
                    {
                        archive.insert(key, ind.clone());
                    }
                } else {
                    red_team_rejections += 1;
                }
                pop.push(ind);
            }
        }
        if pop.is_empty() {
            return Err(format!(
                "island {isl} has no valid initial proposals; no fallback population permitted"
            ));
        }
        assign_nsga2_ranks_and_crowding(&mut pop);
        islands.push(pop);
    }

    for gen in 1..=cfg.generations {
        let archive_snapshot: Vec<Individual> = archive.values().cloned().collect();
        for (isl, pop) in islands.iter_mut().enumerate() {
            let mut isl_rng = Rng::new(
                cfg.seed
                    ^ ((isl as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15))
                    ^ ((gen as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)),
            );
            let mut offspring = Vec::with_capacity(cfg.pop_per_island);
            for slot in 0..cfg.pop_per_island {
                let p1 = tournament_select(pop, &mut isl_rng);
                let outcome = if isl_rng.f64() < cfg.crossover_rate {
                    let p2 = if !archive_snapshot.is_empty() && isl_rng.f64() < 0.35 {
                        &archive_snapshot[(isl_rng.next_u64() as usize) % archive_snapshot.len()]
                    } else {
                        tournament_select(pop, &mut isl_rng)
                    };
                    p1.program
                        .crossover(&p2.program, &mut isl_rng)
                        .map(|outcome| outcome.with_parent_ids(vec![p1.id.clone(), p2.id.clone()]))
                } else {
                    p1.program
                        .mutate(&mut isl_rng)
                        .map(|outcome| outcome.with_parent_ids(vec![p1.id.clone()]))
                };
                if let Some(ind) = evaluate_proposal(
                    outcome,
                    format!("gen{gen:03}-isl{isl}-ind{slot:03}"),
                    gen,
                    isl,
                    cfg,
                    &mut proposal_budget,
                )? {
                    total_evaluations += 1;
                    if ind.red_team_passed {
                        valid_f0_evaluations += 1;
                        let key = map_elites_key(&ind);
                        if archive
                            .get(&key)
                            .is_none_or(|inc| is_better_for_archive(&ind, inc))
                        {
                            archive.insert(key, ind.clone());
                        }
                    } else {
                        red_team_rejections += 1;
                    }
                    offspring.push(ind);
                }
            }
            pop.extend(offspring);
            assign_nsga2_ranks_and_crowding(pop);
            pop.sort_by(|a, b| {
                a.rank
                    .cmp(&b.rank)
                    .then_with(|| b.crowding_distance.total_cmp(&a.crowding_distance))
            });
            pop.truncate(cfg.pop_per_island);
        }
        if cfg.islands > 1 && gen % cfg.migration_interval.max(1) == 0 {
            let migrants: Vec<Individual> = islands.iter().map(|pop| pop[0].clone()).collect();
            for (isl, migrant) in migrants.into_iter().enumerate() {
                let target = (isl + 1) % cfg.islands;
                if let Some(last) = islands[target].last_mut() {
                    *last = migrant;
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
        // Explicit opt-in only (`V5-01-04`): attach the external audit; a VRAM workspace target is
        // passed only when configured, for the baseline and top three elites.
        attach_cuda_audit_with_vram(&mut baseline_ref, cuda_bin, cfg.gpu_vram_target_pct);
        for (idx, elite) in top_elites.iter_mut().enumerate() {
            let vram_pct = if idx < 3 {
                cfg.gpu_vram_target_pct
            } else {
                None
            };
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
        campaign_admission,
        proposal_budget,
        schema_version: "0.2.0".to_string(),
        engine: "joil-core::evolve (multi-island constrained NSGA-II + 4D MAP-Elites + Monte Carlo sandbox screen; external CUDA audit only on explicit opt-in)".to_string(),
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
    // A serialized shard cannot grant physical admission or replace the frozen
    // requirement/objective policy. Validate every receipt before creating any
    // output, and retain these exact parsed summaries for the merge below.
    let summaries: Vec<CampaignSummary> = shard_dirs
        .iter()
        .map(|directory| {
            let bytes =
                fs::read(directory.join("campaign-summary.json")).map_err(|e| e.to_string())?;
            let summary: CampaignSummary =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let expected = crate::experiment::admit_search_campaign(summary.config.campaign_mode)?;
            if summary.campaign_admission != expected {
                return Err("shard admission/requirement/objective receipt mismatch".into());
            }
            Ok(summary)
        })
        .collect::<Result<_, String>>()?;
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let meshes_dir = out_dir.join("meshes");
    fs::create_dir_all(&meshes_dir).map_err(|e| e.to_string())?;

    let mut total_evaluations = 0usize;
    let mut valid_f0_evaluations = 0usize;
    let mut red_team_rejections = 0usize;
    let mut merged_archive: BTreeMap<String, Individual> = BTreeMap::new();
    let mut all_candidates: Vec<Individual> = Vec::new();
    let mut first_summary: Option<CampaignSummary> = None;
    let mut merged_budget = ProposalBudget::default();

    let mut gpu_devices_used: Vec<String> = Vec::new();
    let mut peak_vram_pct = 0.0_f64;
    let mut peak_vram_mib = 0.0_f64;
    let mut total_gpu_parcels = 0u64;

    for (s_idx, (sdir, sum)) in shard_dirs.iter().zip(summaries).enumerate() {
        let node_tag = node_labels
            .get(s_idx)
            .cloned()
            .unwrap_or_else(|| format!("shard{}", s_idx));
        let arch_raw =
            fs::read_to_string(sdir.join("map-elites-archive.json")).map_err(|e| e.to_string())?;
        let arch: BTreeMap<String, Individual> =
            serde_json::from_str(&arch_raw).map_err(|e| e.to_string())?;
        let pf_raw =
            fs::read_to_string(sdir.join("pareto-front.json")).map_err(|e| e.to_string())?;
        let pf: Vec<Individual> = serde_json::from_str(&pf_raw).map_err(|e| e.to_string())?;

        merged_budget.absorb(sum.proposal_budget.clone());
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
                    total_gpu_parcels.saturating_add(cuda.gpu_workspace_tensor_cells_evaluated);
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

    let all_rank0_feasible =
        !pareto_front.is_empty() && pareto_front.iter().all(|x| x.constraint_violation <= 1e-9);
    let all_rank0_redteam =
        !pareto_front.is_empty() && pareto_front.iter().all(|x| x.red_team_passed);
    let all_rank0_p08 = !pareto_front.is_empty()
        && pareto_front.iter().all(|x| {
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
        total_gpu_workspace_tensor_cells_evaluated: total_gpu_parcels,
    };

    let merged_summary = CampaignSummary {
        campaign_admission: base_sum.campaign_admission,
        proposal_budget: merged_budget,
        schema_version: "0.2.0".to_string(),
        engine: "joil-core::evolve (merged multi-island shards: constrained NSGA-II + 4D MAP-Elites + Monte Carlo sandbox screen; external CUDA audit only on explicit opt-in)".to_string(),
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
