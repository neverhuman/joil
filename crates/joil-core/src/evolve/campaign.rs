//! Multi-island Constrained NSGA-II + 4D MAP-Elites campaign runner and cluster shard merger.

use super::*;

#[path = "nsga2.rs"]
mod nsga2;
#[path = "shards.rs"]
mod shards;
pub use self::nsga2::*;
pub use self::shards::*;

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
