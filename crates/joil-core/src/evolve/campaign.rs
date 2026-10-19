//! Multi-island Constrained NSGA-II + 4D MAP-Elites campaign runner and cluster shard merger.

use super::*;

#[path = "nsga2.rs"]
mod nsga2;
pub use self::nsga2::*;

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
