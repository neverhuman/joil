//! Wave-3 multi-seed de-novo evolutionary proof orchestration and cross-family holdout audit.

use super::*;

/// Execute the complete Wave-3 Multi-Seed (`N = seeds.len()`) De-Novo Evolutionary Proof
/// across `B0` (`CT-35`), `B1` (`Random-Search`), `B2` (`Axisym-NSGA2`), `A1` (`No-Mutation`),
/// `A2` (`No-Selection`), and `B3` (`Wave3-Exotic3D-QD-CEI`).
pub fn run_wave3_evolution_proof(
    seeds: &[u64],
    cfg: &EvolutionConfig,
) -> Result<Wave3EvolutionProofReport, String> {
    run_wave3_evolution_proof_recorded(seeds, cfg, None)
}

pub(crate) fn run_wave3_evolution_proof_recorded(
    seeds: &[u64],
    cfg: &EvolutionConfig,
    raw: Option<&RawProofCapture>,
) -> Result<Wave3EvolutionProofReport, String> {
    let campaign_admission = crate::experiment::admit_search_campaign(cfg.campaign_mode)?;
    if seeds.len() < 5 {
        return Err("Wave-3 proof requires at least 5 independent master seeds".into());
    }
    let base_b0 = match raw {
        Some(raw) => raw.reference(
            "baseline-ct35-b0",
            "baseline-ct35-b0".into(),
            DesignProgram::baseline_ct35(),
            cfg,
        )?,
        None => {
            evaluate_individual(
                "baseline-ct35-b0".into(),
                DesignProgram::baseline_ct35(),
                0,
                0,
                cfg,
            )?
            .0
        }
    };
    if let Some(raw) = raw {
        raw.json(
            "wave4-proof-raw-reference.json",
            &serde_json::json!({
                "schema": "joil.wave4-raw-reference.v1", "candidate": base_b0,
            }),
        )?;
    }
    let dp_ref = base_b0.nominal_metrics.dp_u_pa;
    let base_kpi = compute_wave3_kpi_score(&base_b0, dp_ref);
    let base_duties = evaluate_held_out_duties(&base_b0.program, &cfg.baseline_hydraulics)?;

    let arms = [
        (
            "B1_Random_Search",
            Wave3ComparatorArm::B1RandomSearchExotic3d,
        ),
        (
            "B2_Conventional_Axisym_NSGA2",
            Wave3ComparatorArm::B2ConventionalAxisymNsga2,
        ),
        (
            "A1_No_Mutation_Crossover_Only",
            Wave3ComparatorArm::A1NoMutationCrossoverOnly,
        ),
        (
            "A2_No_Selection_Random_Walk",
            Wave3ComparatorArm::A2NoSelectionRandomWalk,
        ),
        (
            "B3_Wave3_Exotic3D_QD_CEI",
            Wave3ComparatorArm::B3Wave3Exotic3dQdCei,
        ),
    ];

    let mut arm_outcomes: BTreeMap<String, Vec<Wave3SeedOutcome>> = BTreeMap::new();
    let mut best_by_family: BTreeMap<String, Individual> = BTreeMap::new();
    let mut family_seed_discovery: BTreeMap<String, BTreeMap<u64, bool>> = BTreeMap::new();

    for (arm_name, arm_mode) in arms {
        // Run seeds in bounded 4-thread chunks to leave generous CPU headroom on xbabe2
        let mut outcomes: Vec<Wave3SeedOutcome> = Vec::with_capacity(seeds.len());
        for chunk in seeds.chunks(4) {
            let chunk_res: Result<Vec<Wave3SeedOutcome>, String> = std::thread::scope(|s| {
                let mut handles = Vec::with_capacity(chunk.len());
                for &seed in chunk {
                    let c_ref = cfg;
                    handles.push(s.spawn(move || {
                        run_single_seed_wave3_arm_recorded(seed, arm_mode, c_ref, raw)
                    }));
                }
                let mut v = Vec::with_capacity(handles.len());
                for h in handles {
                    v.push(
                        h.join()
                            .map_err(|_| "wave3 seed thread panic".to_string())??,
                    );
                }
                Ok(v)
            });
            outcomes.extend(chunk_res?);
        }
        if arm_mode == Wave3ComparatorArm::B3Wave3Exotic3dQdCei {
            for o in &outcomes {
                for m in &o.lineage_milestones {
                    family_seed_discovery
                        .entry(m.child_topology_class.clone())
                        .or_default()
                        .insert(o.seed, true);
                }
                let tc = o.best_candidate.topology_class.clone();
                family_seed_discovery
                    .entry(tc.clone())
                    .or_default()
                    .insert(o.seed, true);
                match best_by_family.get(&tc) {
                    Some(inc)
                        if compute_wave3_kpi_score(&o.best_candidate, dp_ref)
                            <= compute_wave3_kpi_score(inc, dp_ref) => {}
                    _ => {
                        best_by_family.insert(tc, o.best_candidate.clone());
                    }
                }
            }
        }
        arm_outcomes.insert(arm_name.to_string(), outcomes);
    }

    // Additional reference work is opt-in and never changes the original five-arm report.
    if raw.is_some() {
        for &seed in seeds {
            run_single_seed_wave3_arm_recorded(seed, Wave3ComparatorArm::B0BaselineCt35, cfg, raw)?;
        }
    }

    // Ensure all 11 canonical families are evaluated for held-out geometry generalization
    let mut arch_rng = Rng::new(seeds[0]);
    let mut lofo_candidates: Vec<Individual> = Vec::new();
    for (index, arch) in DesignProgram::seed_wave2_archetypes(&mut arch_rng)
        .into_iter()
        .enumerate()
    {
        let id = format!("wave3-family-ref-{}", arch.topology_class);
        let ind = match raw {
            Some(raw) => raw.reference(&format!("family-{index:03}"), id, arch, cfg)?,
            None => evaluate_individual(id, arch, 0, 0, cfg)?.0,
        };
        lofo_candidates.push(ind.clone());
        let mut ind_compact = ind;
        ind_compact.phenotype = None;
        let tc = ind_compact.topology_class.clone();
        match best_by_family.get(&tc) {
            Some(inc)
                if compute_wave3_kpi_score(&ind_compact, dp_ref)
                    <= compute_wave3_kpi_score(inc, dp_ref) => {}
            _ => {
                best_by_family.insert(tc, ind_compact);
            }
        }
    }
    for o in arm_outcomes.get("B3_Wave3_Exotic3D_QD_CEI").unwrap() {
        let mut candidate = o.best_candidate.clone();
        candidate.phenotype = Some(Phenotype3D::from_program(&candidate.program)?.0);
        lofo_candidates.push(candidate);
    }

    // Build per-generation confidence band curves across all 10 seeds for each arm
    let mut generation_band_curves: BTreeMap<String, Vec<Wave3GenerationBand>> = BTreeMap::new();
    let mut final_summary_statistics: BTreeMap<
        String,
        BTreeMap<String, BootstrapConfidenceInterval>,
    > = BTreeMap::new();

    for (arm_name, outcomes) in &arm_outcomes {
        let n_gens = cfg.generations + 1;
        let mut bands = Vec::with_capacity(n_gens);
        for g in 0..n_gens {
            let cum_evals = outcomes[0].generation_trace[g].cumulative_evaluations;
            let best_kpis: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].best_kpi_score)
                .collect();
            let med_kpis: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].median_kpi_score)
                .collect();
            let best_couts: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].best_cout_p95_mg_l)
                .collect();
            let med_couts: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].median_cout_p95_mg_l)
                .collect();
            let hvs: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].pareto_hypervolume)
                .collect();
            let cells: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].map_elites_occupied_cells as f64)
                .collect();
            let divs: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].shannon_diversity_bits)
                .collect();
            let exotics: Vec<f64> = outcomes
                .iter()
                .map(|o| o.generation_trace[g].mean_exoticness_index)
                .collect();

            bands.push(Wave3GenerationBand {
                generation: g,
                cumulative_evaluations: cum_evals,
                best_kpi_score: compute_bootstrap_ci(&best_kpis, 20269100 + g as u64),
                median_kpi_score: compute_bootstrap_ci(&med_kpis, 20269200 + g as u64),
                best_cout_p95_mg_l: compute_bootstrap_ci(&best_couts, 20269300 + g as u64),
                median_cout_p95_mg_l: compute_bootstrap_ci(&med_couts, 20269400 + g as u64),
                pareto_hypervolume: compute_bootstrap_ci(&hvs, 20269500 + g as u64),
                map_elites_occupied_cells: compute_bootstrap_ci(&cells, 20269600 + g as u64),
                shannon_diversity_bits: compute_bootstrap_ci(&divs, 20269700 + g as u64),
                mean_exoticness_index: compute_bootstrap_ci(&exotics, 20269800 + g as u64),
            });
        }
        let final_band = bands.last().unwrap().clone();
        let mut stat_map = BTreeMap::new();
        stat_map.insert("best_kpi_score".into(), final_band.best_kpi_score);
        stat_map.insert("median_kpi_score".into(), final_band.median_kpi_score);
        stat_map.insert(
            "best_cout_sample_screen_upper_mg_l".into(),
            final_band.best_cout_p95_mg_l,
        );
        stat_map.insert(
            "median_cout_sample_screen_upper_mg_l".into(),
            final_band.median_cout_p95_mg_l,
        );
        stat_map.insert("pareto_hypervolume".into(), final_band.pareto_hypervolume);
        stat_map.insert(
            "map_elites_occupied_cells".into(),
            final_band.map_elites_occupied_cells,
        );
        stat_map.insert(
            "shannon_diversity_bits".into(),
            final_band.shannon_diversity_bits,
        );
        final_summary_statistics.insert(arm_name.clone(), stat_map);
        generation_band_curves.insert(arm_name.clone(), bands);
    }

    // Pairwise Mann-Whitney U + Cliff's delta + Holm-Bonferroni across all comparator arms
    let b3_outs = arm_outcomes.get("B3_Wave3_Exotic3D_QD_CEI").unwrap();
    let b3_best_kpi: Vec<f64> = b3_outs.iter().map(|o| o.final_best_kpi_score).collect();
    let b3_med_kpi: Vec<f64> = b3_outs.iter().map(|o| o.final_median_kpi_score).collect();
    let b3_best_cout: Vec<f64> = b3_outs.iter().map(|o| o.final_best_cout_p95_mg_l).collect();
    let b3_hv: Vec<f64> = b3_outs.iter().map(|o| o.final_pareto_hypervolume).collect();

    let mut tests: Vec<PairwiseWilcoxonComparison> = Vec::new();
    for other_arm in [
        "B1_Random_Search",
        "B2_Conventional_Axisym_NSGA2",
        "A1_No_Mutation_Crossover_Only",
        "A2_No_Selection_Random_Walk",
    ] {
        let outs = arm_outcomes.get(other_arm).unwrap();
        let o_best_kpi: Vec<f64> = outs.iter().map(|o| o.final_best_kpi_score).collect();
        let o_med_kpi: Vec<f64> = outs.iter().map(|o| o.final_median_kpi_score).collect();
        let o_best_cout: Vec<f64> = outs.iter().map(|o| o.final_best_cout_p95_mg_l).collect();
        let o_hv: Vec<f64> = outs.iter().map(|o| o.final_pareto_hypervolume).collect();

        tests.push(mann_whitney_and_cliffs_delta(
            "final_best_kpi_score",
            "B3_Wave3_Exotic3D_QD_CEI",
            other_arm,
            &b3_best_kpi,
            &o_best_kpi,
            false,
        ));
        tests.push(mann_whitney_and_cliffs_delta(
            "final_median_kpi_score",
            "B3_Wave3_Exotic3D_QD_CEI",
            other_arm,
            &b3_med_kpi,
            &o_med_kpi,
            false,
        ));
        tests.push(mann_whitney_and_cliffs_delta(
            "final_best_cout_sample_screen_upper_mg_l",
            "B3_Wave3_Exotic3D_QD_CEI",
            other_arm,
            &b3_best_cout,
            &o_best_cout,
            true,
        ));
        tests.push(mann_whitney_and_cliffs_delta(
            "final_pareto_hypervolume",
            "B3_Wave3_Exotic3D_QD_CEI",
            other_arm,
            &b3_hv,
            &o_hv,
            false,
        ));
    }

    // Holm-Bonferroni step-down multiplicity correction across all 16 pairwise tests
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

    // Held-out geometry generalization across all 11 families
    // Legacy family reference names repeat across distinct archetypes. Give the
    // audit rows unique scoped IDs, retaining an explicit binding to each full
    // original candidate. Names do not enter the GP's feature/target arithmetic.
    let fold_subjects: Vec<Individual> = lofo_candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let mut subject = candidate.clone();
            subject.id = format!("lofo-{index:03}-{}", candidate.id);
            subject
        })
        .collect();
    let (lofo_audit, fold_rows) =
        crate::surrogate::run_surrogate_and_engine_qualification_with_fold_rows(&fold_subjects)?;
    if let Some(raw) = raw {
        let mut folds = Vec::new();
        let mut events = BufWriter::new(
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(raw.root.join("holdouts.jsonl"))
                .map_err(|e| e.to_string())?,
        );
        for (subject, original) in fold_subjects.iter().zip(&lofo_candidates) {
            raw_event(
                &mut events,
                &serde_json::json!({"event": "fold_subject",
                "subject_id": subject.id, "original_candidate": original}),
            )?;
        }
        for (family, finalist) in &best_by_family {
            let mut fold = if let Some(rows) =
                fold_rows.iter().find(|r| &r.topology_class == family)
            {
                serde_json::to_value(rows).map_err(|e| e.to_string())?
            } else {
                serde_json::json!({"topology_class": family, "state": "insufficient",
                    "train_subject_ids": fold_subjects.iter().filter(|s| &s.topology_class != family)
                        .map(crate::surrogate::SurrogateRecord::from_candidate)
                        .filter(|r| r.status == crate::surrogate::SurrogateRecordStatus::SandboxPriorScreen)
                        .map(|r| r.candidate_id).collect::<Vec<_>>(),
                    "predictions": []})
            };
            for prediction in fold["predictions"]
                .as_array_mut()
                .ok_or("missing raw fold predictions")?
            {
                prediction["target_kpi"] = serde_json::Value::Null;
                prediction["predicted_kpi"] = serde_json::Value::Null;
            }
            fold["evaluated_candidates"] = serde_json::json!([finalist]);
            raw_event(
                &mut events,
                &serde_json::json!({"event": "fold", "fold": fold}),
            )?;
            folds.push(fold);
        }
        events.flush().map_err(|e| e.to_string())?;
        raw.json("wave4-proof-raw-holdouts.json", &serde_json::json!({
            "schema": "joil.wave4-raw-holdouts.v1", "split_basis": "authored_family_screen",
            "folds": folds,
            "identity_note": "Scoped subject aliases bind duplicate legacy family IDs to original candidates in holdouts.jsonl; no new geometry or physical independence is implied",
        }))?;
    }
    let mut held_out_geometries = Vec::new();
    for (tc, elite_ind) in &best_by_family {
        let fold = lofo_audit
            .family_holdouts
            .iter()
            .find(|f| &f.held_out_family == tc);
        let rmse_cout = fold.map(|f| f.rmse_cout_mg_l);
        let cov_2s = fold.map(|f| f.coverage_within_2sigma);
        let sample_cnt = fold.map_or(0, |f| f.test_count);
        let elite_kpi = compute_wave3_kpi_score(elite_ind, dp_ref);
        let disc_seeds = family_seed_discovery.get(tc).map_or(0, |m| m.len());
        held_out_geometries.push(Wave3HeldOutGeometryRecord {
            topology_class: tc.clone(),
            sample_count: sample_cnt,
            lofo_surrogate_rmse_cout_mg_l: rmse_cout,
            lofo_surrogate_rmse_kpi_score: None, // KPI error was never independently evaluated.
            lofo_2sigma_coverage: cov_2s,
            de_novo_rediscovery_seeds: disc_seeds,
            de_novo_rediscovery_fraction: ((disc_seeds as f64 / seeds.len() as f64) * 1e4).round()
                / 1e4,
            held_out_best_kpi_score: elite_kpi,
            held_out_best_cout_p95_mg_l: elite_ind.mc_robustness.cout_p95_mg_l,
            held_out_best_dp_u_pa: elite_ind.nominal_metrics.dp_u_pa,
            held_out_best_grade_20um_p05: elite_ind.mc_robustness.grade_20um_p05,
            kpi_gain_vs_ct35: ((elite_kpi - base_kpi) * 1e6).round() / 1e6,
            cout_reduction_pct_vs_ct35: (((base_b0.mc_robustness.cout_p95_mg_l
                - elite_ind.mc_robustness.cout_p95_mg_l)
                / base_b0.mc_robustness.cout_p95_mg_l
                * 100.0)
                * 1e4)
                .round()
                / 1e4,
        });
    }

    let top_wave3_elite = b3_outs
        .iter()
        .max_by(|a, b| a.final_best_kpi_score.total_cmp(&b.final_best_kpi_score))
        .map(|o| o.best_candidate.clone())
        .unwrap_or_else(|| base_b0.clone());
    let top_wave3_kpi = compute_wave3_kpi_score(&top_wave3_elite, dp_ref);
    let top_duties = evaluate_held_out_duties(&top_wave3_elite.program, &cfg.baseline_hydraulics)?;
    if let Some(raw) = raw {
        raw.json("operating-duty-holdouts.jsonl", &serde_json::json!({
            "event": "operating_duty_screen", "baseline_subject_id": base_b0.id,
            "baseline_duties": base_duties, "top_subject_id": top_wave3_elite.id,
            "top_duties": top_duties,
            "interpretation": "deterministic reduced-screen operating duties; not independent observations",
        }))?;
    }
    let pop_size = cfg
        .islands
        .checked_mul(cfg.pop_per_island)
        .filter(|n| *n > 0)
        .ok_or("positive finite population budget required")?;
    let evals_per_seed = b3_outs.first().map(|o| o.total_evaluations).unwrap_or(0);
    let total_evals_all = evals_per_seed * seeds.len() * arms.len();

    for outcomes in arm_outcomes.values_mut() {
        for o in outcomes.iter_mut() {
            o.proposal_budget.proposal_receipts.clear();
        }
    }
    let mut base_b0_compact = base_b0;
    base_b0_compact.phenotype = None;
    base_b0_compact.cuda_sdf_audit = None;

    Ok(Wave3EvolutionProofReport {
        campaign_admission,
        schema_version: 1,
        protocol_id: "joil-wave3-de-novo-evolution-proof-v1".into(),
        kpi_formula: "KPI_score(g) = (requirements.quality_limit_mg_l - C_out,sample_screen_upper(g)) + 25.0 * E_reduced,20um,sample_screen_lower(g) + 10.0 * (dp_u_CT35 - dp_u(g)) / dp_u_CT35 - 500.0 * V(g)".into(),
        qualification_note: "Sandbox comparisons use a shared mild CT-35 initialization where specified. Scores and GP targets come from the prior-laden reduced screen. Archived carrier diagnostics are separate subjects and do not validate new winners. Independent oil, structural and physical evidence remains incomplete.".into(),
        n_seeds: seeds.len(),
        master_seeds: seeds.to_vec(),
        generations: cfg.generations,
        population_per_generation: pop_size,
        evaluations_per_seed_per_arm: evals_per_seed,
        total_3d_evaluations_across_arms: total_evals_all,
        baseline_b0_ct35_kpi_score: base_kpi,
        baseline_b0_ct35_cout_p95_mg_l: base_b0_compact.mc_robustness.cout_p95_mg_l,
        baseline_b0_ct35_dp_u_pa: base_b0_compact.nominal_metrics.dp_u_pa,
        baseline_b0_ct35_grade_20um_p05: base_b0_compact.mc_robustness.grade_20um_p05,
        baseline_b0_ct35: base_b0_compact,
        baseline_b0_held_out_duties: base_duties,
        arm_outcomes,
        generation_band_curves,
        final_summary_statistics,
        pairwise_tests: tests,
        held_out_geometry_generalization: held_out_geometries,
        promoted_wave3_top_elite: top_wave3_elite,
        promoted_wave3_top_elite_kpi_score: top_wave3_kpi,
        promoted_wave3_top_elite_held_out_duties: top_duties,
        promoted_wave3_elites_by_family: best_by_family,
    })
}

// NEW capture contract, introduced after the historical Wave-4 summary. These
// files describe a new run; they cannot recover observations never retained.
