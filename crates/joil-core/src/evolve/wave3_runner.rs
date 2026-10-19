//! Single-seed Wave-3 comparator arm execution loop.

use super::*;

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
