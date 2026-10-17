use joil_core::{
    cfd_case, evolve, experiment, geometry, metrics, openfoam, phenotype, population, program,
    redteam, scene, structure, surrogate,
};
use std::path::PathBuf;

fn parse_arg(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

/// Import a historical Wave-2 genome without upgrading its archived evidence.
///
/// Report/Individual evaluation schemas have changed since the committed v1
/// receipt. CFD preparation needs only the selected program: validate and
/// canonicalize that program, then let the case builder compute its current
/// design_sha256. Archived genome hashes (which included authored topology
/// labels), metrics, lineage and qualification flags are not imported.
fn load_wave2_elite_program(
    raw: &str,
    family: &str,
) -> Result<program::DesignProgram, Box<dyn std::error::Error>> {
    let report: serde_json::Value = serde_json::from_str(raw)?;
    let elites = report
        .get("promoted_wave2_elites_by_family")
        .and_then(serde_json::Value::as_object)
        .ok_or("Wave-2 receipt has no promoted_wave2_elites_by_family object")?;
    let elite = elites
        .get(family)
        .ok_or_else(|| format!("Wave-2 family '{family}' not found"))?;
    let genome = elite
        .get("program")
        .ok_or_else(|| format!("Wave-2 family '{family}' has no program"))?;
    let program: program::DesignProgram = serde_json::from_value(genome.clone())?;
    Ok(program.canonicalize()?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).cloned().unwrap_or_else(|| "baseline".into());
    match command.as_str() {
        "baseline" => {
            let g = geometry::baseline();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status":"analytic_geometry_only_not_cfd_validated",
                    "genome_sha256":g.sha256()?,"dimensions":g.derive()?,
                    "regression_example":{"cin_mg_l":500,"capture":0.94,"split":0.02,
                      "cout_mg_l":metrics::concentration_mg_l(500.0,0.94,0.02)?,"passes_30_mg_l":false}
                }))?
            );
        }
        "kpi" => {
            let root = PathBuf::from(parse_arg(&args, "--root").unwrap_or_else(|| ".".into()));
            let cases = [
                (
                    "runs/ct35-sst-diagnostic-000/hydraulic-integrals-t500.json",
                    500u64,
                ),
                (
                    "runs/ct35-sst-split-001/hydraulic-integrals-t300.json",
                    300u64,
                ),
                (
                    "runs/ct35-sst-extended-000/hydraulic-integrals-t600.json",
                    600u64,
                ),
            ];
            let mut results = Vec::new();
            for (rel, step) in cases {
                let eval = openfoam::read_cfd_kpi_observation(&root, &root.join(rel), step)
                    .map_err(|observation| serde_json::json!({"observation": observation}).to_string())?;
                results.push(eval);
            }
            let out_json = serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": "0.2.0",
                "evaluator": "joil-core::openfoam::read_cfd_kpi_observation",
                "cases": results,
            }))?;
            if let Some(out_path) = parse_arg(&args, "--out") {
                std::fs::write(out_path, &out_json)?;
            }
            println!("{out_json}");
        }
        "verify-rotation" => {
            let ode_20 = openfoam::integrate_coupled_rotation_ode(20.0, 0.01, 200.0, 2e-6, 0.002);
            let ode_40 = openfoam::integrate_coupled_rotation_ode(40.0, 0.01, 200.0, 2e-6, 0.002);
            let ratio_d2 =
                (ode_40.u_r_end_m_s / ode_40.r_end_m) / (ode_20.u_r_end_m_s / ode_20.r_end_m);
            let out_json = serde_json::to_string_pretty(&serde_json::json!({
                "oracle": "joil-core::openfoam::integrate_coupled_rotation_ode",
                "d20_um": ode_20,
                "d40_um": ode_40,
                "d40_to_d20_drift_rate_ratio": ratio_d2,
                "inward_migration_verified": ode_20.radial_displacement_m < 0.0
                    && ode_40.radial_displacement_m < ode_20.radial_displacement_m
                    && ode_20.u_r_end_m_s < 0.0
                    && ode_40.u_r_end_m_s < 0.0,
            }))?;
            if let Some(out_path) = parse_arg(&args, "--out") {
                std::fs::write(out_path, &out_json)?;
            }
            println!("{out_json}");
        }
        "screen-structure" | "qualify-structure" => {
            let mut rng = program::Rng::new(20261015);
            let archetypes = program::DesignProgram::seed_wave2_archetypes(&mut rng);
            let lame_inner =
                structure::lame_thick_cylinder_oracle(0.035, 0.038, 0.0, 2.5e6, 0.035)?;
            let lame_outer =
                structure::lame_thick_cylinder_oracle(0.035, 0.038, 0.0, 2.5e6, 0.038)?;
            let mut reports = Vec::new();
            for (idx, prog) in archetypes.iter().enumerate() {
                let (pheno, _) = phenotype::Phenotype3D::from_program(prog)?;
                let sq = structure::analytic_structural_screen(prog, &pheno, 29468.22, 0.001111111)?;
                reports.push(serde_json::json!({
                    "index": idx,
                    "topology_class": pheno.topology_class,
                    "genome_sha256": pheno.design_sha256,
                    "analytic_structural_screen": sq,
                }));
            }
            let out_json = serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": "0.2.0",
                "evaluator": "joil-core::structure::analytic_structural_screen",
                "lame_cylinder_oracle_head_35mm_id_38mm_od_2.5mpa": {
                    "inner_wall": lame_inner,
                    "outer_wall": lame_outer,
                },
                "archetype_analytic_screens": reports,
            }))?;
            if let Some(out_path) = parse_arg(&args, "--out") {
                std::fs::write(out_path, &out_json)?;
            }
            println!("{out_json}");
        }
        "audit-probe" => {
            let root = PathBuf::from(parse_arg(&args, "--root").unwrap_or_else(|| ".".into()));
            let summary_path = PathBuf::from(parse_arg(&args, "--summary").unwrap_or_else(|| {
                "runs/gemini-ct35-full-liner-oil-pilot-summary-r4.json".into()
            }));
            let audit = openfoam::audit_lagrangian_probe_summary(&summary_path, &root)?;
            let out_json = serde_json::to_string_pretty(&audit)?;
            if let Some(out_path) = parse_arg(&args, "--out") {
                std::fs::write(out_path, &out_json)?;
            }
            println!("{out_json}");
            if !audit.audit_passed {
                return Err(format!("probe audit failed: {:?}", audit.failure_reasons).into());
            }
        }
        "promote-elite" => {
            let summary_path = PathBuf::from(
                parse_arg(&args, "--summary")
                    .unwrap_or_else(|| "runs/rust-cuda-evolution-v1/campaign-summary.json".into()),
            );
            let out_path = PathBuf::from(
                parse_arg(&args, "--out")
                    .unwrap_or_else(|| "runs/rust-cuda-evolution-v1/promotion-dossier.json".into()),
            );
            let raw = std::fs::read_to_string(&summary_path)?;
            let campaign: evolve::CampaignSummary = serde_json::from_str(&raw)?;
            let elite = campaign
                .top_pareto_elites
                .first()
                .ok_or("no top_pareto_elites in campaign summary")?;
            let base = &campaign.baseline_reference;

            let (pheno, mesh) = phenotype::Phenotype3D::from_program(&elite.program)?;
            redteam::check(&elite.program, &pheno, &mesh)
                .map_err(|e| format!("top elite failed redteam gate: {e:?}"))?;
            let f0_unevaluated = redteam::unevaluated_geometry_facts(&pheno);
            let claim_state = if f0_unevaluated.is_empty() {
                "f0_geometry_and_p08_structural_screen_passed_ready_for_f2_cfd_promotion_not_yet_f2_or_f4_validated"
            } else {
                "f0_partial_sandbox_screen_passed_with_unevaluated_geometry_facts_not_f0_qualified_not_f2_or_f4_validated"
            };
            let struct_qual = structure::analytic_structural_screen(
                &elite.program,
                &pheno,
                elite.nominal_metrics.dp_u_pa,
                campaign.config.baseline_hydraulics.q_in_m3_s,
            )?;
            if !struct_qual.passed {
                return Err(format!(
                    "top candidate failed the P08 analytic structural screen: {:?}",
                    struct_qual.failure_reasons
                )
                .into());
            }
            let multi_patch = phenotype::MultiPatchSurface::from_program(&elite.program, 0.0)?;
            if !multi_patch.audit.watertight_and_manifold
                || !multi_patch.audit.all_terminal_patches_valid
            {
                return Err("top elite failed multi-patch CFD surface gate (CAD-04/CAD-06)".into());
            }

            let brep_report_path = summary_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("cad/top_elite_brep_report.json");
            let brep_audit: Option<serde_json::Value> = if brep_report_path.is_file() {
                Some(serde_json::from_str(&std::fs::read_to_string(
                    &brep_report_path,
                )?)?)
            } else {
                None
            };

            let dossier = serde_json::json!({
                "schema_version": "0.3.0",
                "kind": "multi_fidelity_3d_elite_promotion_dossier",
                "claim_state": claim_state,
                "f0_unevaluated_geometry_facts": f0_unevaluated,
                "qualified_multiobjective_vector": null,
                "qualified_capacity_gain": null,
                "campaign_summary_sha256": openfoam::file_sha256(&summary_path)?,
                "promoted_candidate": {
                    "id": elite.id,
                    "topology_class": elite.topology_class,
                    "genome_sha256": elite.genome_sha256,
                    "generation_born": elite.generation_born,
                    "island_origin": elite.island_origin,
                    "program": elite.program,
                    "descriptors": elite.descriptors,
                    "phenotype_3d": pheno,
                    "multi_patch_cfd_surface_audit": multi_patch.audit,
                    "analytic_structural_screen_p08": struct_qual,
                    "nominal_f1_metrics": elite.nominal_metrics,
                    "monte_carlo_robustness": elite.mc_robustness,
                    "cuda_rtx3090_sdf_and_ode_audit": elite.cuda_sdf_audit,
                    "opencascade_brep_step_audit": brep_audit,
                },
                "baseline_ct35_comparator": {
                    "id": base.id,
                    "genome_sha256": base.genome_sha256,
                    "nominal_f1_metrics": base.nominal_metrics,
                    "monte_carlo_robustness": base.mc_robustness,
                    "cuda_rtx3090_sdf_and_ode_audit": base.cuda_sdf_audit,
                },
                "f1_screen_deltas_vs_baseline_not_qualified_gain": {
                    "cout_nominal_delta_mg_l": elite.nominal_metrics.cout_mg_l - base.nominal_metrics.cout_mg_l,
                    "cout_p95_delta_mg_l": elite.mc_robustness.cout_p95_mg_l - base.mc_robustness.cout_p95_mg_l,
                    "d50_nominal_delta_um": elite.nominal_metrics.d50_um - base.nominal_metrics.d50_um,
                    "grade_20um_p05_delta": elite.mc_robustness.grade_20um_p05 - base.mc_robustness.grade_20um_p05,
                    "entry_shear_delta_s_inv": elite.nominal_metrics.max_entry_shear_s_inv - base.nominal_metrics.max_entry_shear_s_inv,
                    "dp_u_delta_pa": elite.nominal_metrics.dp_u_pa - base.nominal_metrics.dp_u_pa,
                },
                "required_next_gates_before_claim": [
                    "P04: 3-grid systematic snappyHexMesh + OpenFOAM 14 LRR/SSG Reynolds-Stress carrier convergence on promoted STEP geometry",
                    "P05: Full-residence-time flux-weighted inlet 10-bin Lagrangian droplet cohort closure on converged F2 carrier",
                    "P06: Blinded A/B/A physical rig validation under EXP-RIG-JOIL-V1-DRAFT"
                ]
            });
            let out_str = serde_json::to_string_pretty(&dossier)?;
            std::fs::write(&out_path, &out_str)?;
            println!("{out_str}");
        }
        "prepare-cfd-case" => {
            let case_dir = PathBuf::from(parse_arg(&args, "--case").unwrap_or_else(|| {
                "runs/rust-cfd-elite-gen024-isl2-ind014".into()
            }));
            let prog = if args.iter().any(|a| a == "--baseline-ct35") {
                program::DesignProgram::baseline_ct35()
            } else if let Some(prog_path) = parse_arg(&args, "--program-json") {
                let raw = std::fs::read_to_string(&prog_path)?;
                let p: program::DesignProgram = serde_json::from_str(&raw)?;
                p.canonicalize()?
            } else if let Some(w2_family) = parse_arg(&args, "--wave2-family") {
                let w2_path = PathBuf::from(parse_arg(&args, "--wave2-ablation").unwrap_or_else(
                    || "docs/results/wave2-multiseed-ablation-and-holdout-v1.json".into(),
                ));
                let raw = std::fs::read_to_string(&w2_path)?;
                load_wave2_elite_program(&raw, &w2_family)
                    .map_err(|e| format!("{}: {e}", w2_path.display()))?
            } else {
                let summary_path = PathBuf::from(
                    parse_arg(&args, "--summary").unwrap_or_else(|| {
                        "runs/rust-cuda-evolution-v1/campaign-summary.json".into()
                    }),
                );
                let raw = std::fs::read_to_string(&summary_path)?;
                let campaign: evolve::CampaignSummary = serde_json::from_str(&raw)?;
                let elite = campaign
                    .top_pareto_elites
                    .first()
                    .ok_or("no top_pareto_elites in campaign summary")?;
                elite.program.clone()
            };
            let mut cfg = cfd_case::CfdCaseConfig::default();
            if let Some(p_pa) = parse_arg(&args, "--reject-pressure-pa").and_then(|s| s.parse().ok())
            {
                cfg.reject_pressure_pa = p_pa;
            }
            if let Some(steps) = parse_arg(&args, "--steps").and_then(|s| s.parse().ok()) {
                cfg.steps = steps;
            }
            if let Some(model) = parse_arg(&args, "--model") {
                cfg.turbulence_model = model;
            }
            if let Some(sub) = parse_arg(&args, "--subdomains").and_then(|s| s.parse().ok()) {
                cfg.n_subdomains = sub;
            }
            let manifest = cfd_case::build_openfoam_candidate_case(&prog, &case_dir, &cfg)?;
            let out_json = serde_json::to_string_pretty(&manifest)?;
            if let Some(out_path) = parse_arg(&args, "--out") {
                std::fs::write(out_path, &out_json)?;
            }
            println!("{out_json}");
        }
        "qualify-surrogate" => {
            let summary_path = PathBuf::from(
                parse_arg(&args, "--summary")
                    .unwrap_or_else(|| "runs/rust-cuda-evolution-v1/campaign-summary.json".into()),
            );
            let out_path = PathBuf::from(parse_arg(&args, "--out").unwrap_or_else(|| {
                "runs/rust-cuda-evolution-v1/surrogate-cv-report.json".into()
            }));
            let raw = std::fs::read_to_string(&summary_path)?;
            let campaign: evolve::CampaignSummary = serde_json::from_str(&raw)?;
            let mut candidates = vec![campaign.baseline_reference.clone()];
            candidates.extend(campaign.top_pareto_elites.clone());
            let report = surrogate::run_surrogate_and_engine_qualification(&candidates)?;
            let out_json = serde_json::to_string_pretty(&report)?;
            std::fs::write(&out_path, &out_json)?;
            println!("{out_json}");
        }
        "qualify-population" => {
            let out_path = PathBuf::from(parse_arg(&args, "--out").unwrap_or_else(|| {
                "runs/rust-p05-population-qualification.json".into()
            }));
            let oil04 =
                population::verify_added_mass_and_pressure_gradient(30.0, 50.0, 2.0e-7, 0.002)?;
            let oil05_06 = population::verify_dispersion_and_les_partition(
                &[0.12, 0.28, 0.54, 0.88, 1.32],
                0.004,
                1.45,
                0.18,
                0.18,
            )?;
            let oil06_neg = population::verify_dispersion_and_les_partition(
                &[0.12, 0.28, 0.54, 0.88, 1.32],
                0.004,
                1.45,
                0.18,
                1.63,
            )
            .is_err();
            let oil07 = population::verify_parcel_substep_halving(3.0, 5.0e-5)?;
            let oil08_weights = population::build_stratified_dsd_weights(25.0, 0.55, false)?;
            let oil08_neg = population::build_stratified_dsd_weights(25.0, 0.55, true).is_err();
            let mass_sum: f64 = oil08_weights.iter().map(|b| b.mass_fraction).sum();
            let num_sum: f64 = oil08_weights.iter().map(|b| b.number_fraction).sum();
            let oil10_12 = population::verify_pbm_breakup_coalescence_and_resolution(
                180.0, false, 0.003, false,
            )?;
            let oil10_neg =
                population::verify_pbm_breakup_coalescence_and_resolution(180.0, true, 0.003, false)
                    .is_err();
            let oil13_neg =
                population::verify_pbm_breakup_coalescence_and_resolution(180.0, false, 0.04, true)
                    .is_err();
            let cohort_1rank = population::generate_mpi_rank_invariant_cohort(256, 1)?;
            let cohort_4rank = population::generate_mpi_rank_invariant_cohort(256, 4)?;
            let oil14_passed = cohort_1rank == cohort_4rank;
            let oil15_valid =
                population::check_multiphase_regime_envelope(0.00059, 0.0, 0.0032, 0.024);
            let oil15_violation =
                population::check_multiphase_regime_envelope(0.035, 0.012, 0.22, 0.002);
            let report = serde_json::json!({
                "schema_version": "0.2.0",
                "packet": "P05",
                "evaluator": "joil-core::population",
                "oil04_added_mass_and_pressure_gradient": oil04,
                "oil05_06_dispersion_and_les_partition": oil05_06,
                "oil07_parcel_substep_halving": oil07,
                "oil08_stratified_dsd_weights": {
                    "bins": oil08_weights,
                    "mass_fraction_sum": mass_sum,
                    "number_fraction_sum": num_sum,
                    "equal_number_mistake_rejected": oil08_neg,
                },
                "oil10_12_pbm_breakup_coalescence_and_resolution": oil10_12,
                "oil14_mpi_rank_invariance": {
                    "parcels_tested": 256,
                    "rank_1_equals_rank_4_bitwise": oil14_passed,
                },
                "oil15_multiphase_regime_envelope": {
                    "nominal_operating_point": oil15_valid,
                    "dense_core_and_gas_rejection_test": oil15_violation,
                },
                "all_p05_gates_passed": oil04.passed
                    && oil05_06.passed
                    && oil06_neg
                    && oil07.passed
                    && (mass_sum - 1.0).abs() < 1e-12
                    && (num_sum - 1.0).abs() < 1e-12
                    && oil08_neg
                    && oil10_12.passed
                    && oil10_neg
                    && oil13_neg
                    && oil14_passed
                    && oil15_valid.within_dilute_lagrangian_envelope
                    && !oil15_violation.within_dilute_lagrangian_envelope,
            });
            let out_json = serde_json::to_string_pretty(&report)?;
            std::fs::write(&out_path, &out_json)?;
            println!("{out_json}");
        }
        "qualify-experiment" => {
            let out_path = PathBuf::from(parse_arg(&args, "--out").unwrap_or_else(|| {
                "runs/rust-p06-experiment-qualification.json".into()
            }));
            let exp01 = experiment::verify_instrument_calibration(
                "batch-ct35-oil-20261015-sha256",
                "batch-ct35-oil-20261015-sha256",
                0.9987,
                0.4,
                1.2,
                0.75,
            )?;
            let exp01_neg = experiment::verify_instrument_calibration(
                "batch-a-sha256",
                "batch-b-sha256",
                0.9987,
                0.4,
                1.2,
                15.0,
            )
            .is_err();
            let blocks = vec![
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-01".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 4.02,
                    candidate_b_qmax_m3_h: 4.65,
                    baseline_a2_qmax_m3_h: 4.00,
                    split_mismatch_abs: 0.00008,
                },
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-02".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 3.98,
                    candidate_b_qmax_m3_h: 4.58,
                    baseline_a2_qmax_m3_h: 4.01,
                    split_mismatch_abs: 0.00011,
                },
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-03".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 4.01,
                    candidate_b_qmax_m3_h: 4.62,
                    baseline_a2_qmax_m3_h: 3.99,
                    split_mismatch_abs: 0.00009,
                },
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-04".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 4.00,
                    candidate_b_qmax_m3_h: 4.60,
                    baseline_a2_qmax_m3_h: 4.02,
                    split_mismatch_abs: 0.00007,
                },
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-05".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 3.99,
                    candidate_b_qmax_m3_h: 4.64,
                    baseline_a2_qmax_m3_h: 4.01,
                    split_mismatch_abs: 0.00010,
                },
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-06".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 4.01,
                    candidate_b_qmax_m3_h: 4.61,
                    baseline_a2_qmax_m3_h: 4.00,
                    split_mismatch_abs: 0.00006,
                },
            ];
            let exp02_03_aba = experiment::analyze_blinded_aba_crossover(&blocks)?;
            let exp04_holdout = experiment::verify_holdout_isolation(
                &["train-g1".to_string(), "train-g2".to_string()],
                &["holdout-h1".to_string(), "holdout-h2".to_string()],
                1,
            )?;
            let exp04_leak_rejected = experiment::verify_holdout_isolation(
                &["train-g1".to_string(), "holdout-h1".to_string()],
                &["holdout-h1".to_string()],
                1,
            )
            .is_err();
            let exp05_ct = experiment::evaluate_as_built_ct_metrology(
                "SN-JOIL-316L-001",
                2.000,
                2.008,
                6.5,
                28.0,
                false,
            )?;
            let exp05_oversized_rejected = experiment::evaluate_as_built_ct_metrology(
                "SN-JOIL-316L-BAD",
                2.000,
                2.045,
                6.5,
                28.0,
                false,
            )
            .is_err();
            let exp06_matched = experiment::verify_shared_duty_comparator(
                "duty-ct35-4m3h-r0.01986",
                "duty-ct35-4m3h-r0.01986",
                Some("duty-ct35-4m3h-r0.01986"),
                true,
            )?;
            let exp06_unmatched_rejected = experiment::verify_shared_duty_comparator(
                "duty-ct35-4m3h-r0.01986",
                "duty-ct35-4m3h-r0.01986",
                None,
                true,
            )
            .is_err();
            let report = serde_json::json!({
                "schema_version": "0.2.0",
                "packet": "P06",
                "evaluator": "joil-core::experiment",
                "experiment01_calibration_and_loq_censoring": exp01,
                "experiment02_03_blinded_aba_crossover_and_power": exp02_03_aba,
                "experiment04_holdout_isolation": {
                    "isolated_holdout_verified": exp04_holdout,
                    "leaked_holdout_rejected": exp04_leak_rejected,
                },
                "experiment05_cad09_as_built_ct_metrology": exp05_ct,
                "experiment06_shared_duty_comparator": {
                    "matched_commercial_comparator_verified": exp06_matched,
                    "unmatched_industry_claim_rejected": exp06_unmatched_rejected,
                },
                "all_p06_gates_passed": exp01.passed
                    && exp01_neg
                    && exp02_03_aba.passed_10pct_lcb95_gate
                    && exp04_holdout
                    && exp04_leak_rejected
                    && exp05_ct.passed
                    && exp05_oversized_rejected
                    && exp06_matched
                    && exp06_unmatched_rejected,
            });
            let out_json = serde_json::to_string_pretty(&report)?;
            std::fs::write(&out_path, &out_json)?;
            println!("{out_json}");
        }
        "export-glb-scene" => {
            let summary_path = PathBuf::from(
                parse_arg(&args, "--summary")
                    .unwrap_or_else(|| "runs/rust-cuda-evolution-v1/campaign-summary.json".into()),
            );
            let glb_path = PathBuf::from(parse_arg(&args, "--glb").unwrap_or_else(|| {
                "runs/rust-cuda-evolution-v1/meshes/top_elite_scene_m.glb".into()
            }));
            let raw = std::fs::read_to_string(&summary_path)?;
            let campaign: evolve::CampaignSummary = serde_json::from_str(&raw)?;
            let elite = campaign
                .top_pareto_elites
                .first()
                .ok_or("no top_pareto_elites in campaign summary")?;
            let surface = phenotype::MultiPatchSurface::from_program(&elite.program, 0.0)?;
            let stl_sha = openfoam::file_sha256(std::path::Path::new(
                "runs/rust-cfd-elite-gen024-isl2-ind014/constant/triSurface/ct35.stl",
            ))?;
            let scene_manifest =
                scene::export_glb_scene_bundle(&surface, &stl_sha, 1.0, &glb_path)?;
            let lease_valid = scene::verify_resource_lease(
                "lease-joil-local-4core",
                &[0, 1, 2, 3],
                4,
                16,
                8,
                1_800_000_000,
            )?;
            let lease_smt_rejected = scene::verify_resource_lease(
                "lease-smt-mistake",
                &[0, 16, 1, 17],
                4,
                16,
                8,
                1_800_000_000,
            )
            .is_err();
            let out_json = serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": "0.2.0",
                "packet": "P09",
                "evaluator": "joil-core::scene",
                "ops_view01_resource_lease": {
                    "valid_lease": lease_valid,
                    "smt_sibling_mistake_rejected": lease_smt_rejected,
                },
                "ops_view02_03_glb_scene_manifest": scene_manifest,
            }))?;
            if let Some(out_path) = parse_arg(&args, "--out") {
                std::fs::write(out_path, &out_json)?;
            }
            println!("{out_json}");
        }
        "compile-3d" => {
            let out_dir = PathBuf::from(
                parse_arg(&args, "--out").unwrap_or_else(|| "runs/rust-3d-archetypes".into()),
            );
            std::fs::create_dir_all(&out_dir)?;
            let mut rng = program::Rng::new(20261015);
            let archetypes = program::DesignProgram::seed_archetypes(&mut rng);
            let mut reports = Vec::new();
            for (idx, prog) in archetypes.iter().enumerate() {
                let (pheno, mesh) = phenotype::Phenotype3D::from_program(prog)?;
                redteam::check(prog, &pheno, &mesh)
                    .map_err(|e| format!("archetype {idx} failed redteam: {e:?}"))?;
                let stl_path = out_dir.join(format!("{:02}_{}.stl", idx, pheno.topology_class));
                let obj_path = out_dir.join(format!("{:02}_{}.obj", idx, pheno.topology_class));
                mesh.write_stl(&stl_path, &pheno.topology_class, 1.0)?;
                mesh.write_obj(&obj_path, 1.0)?;
                reports.push(serde_json::json!({
                    "index": idx,
                    "topology_class": pheno.topology_class,
                    "genome_sha256": pheno.design_sha256,
                    "phenotype": pheno,
                    "stl_path": stl_path,
                    "obj_path": obj_path,
                }));
            }
            println!("{}", serde_json::to_string_pretty(&reports)?);
        }
        "evolve" => {
            let out_dir = PathBuf::from(
                parse_arg(&args, "--out").unwrap_or_else(|| "runs/rust-cuda-evolution-v1".into()),
            );
            let mut cfg = evolve::EvolutionConfig::default();
            if let Some(g) = parse_arg(&args, "--generations").and_then(|s| s.parse().ok()) {
                cfg.generations = g;
            }
            if let Some(isl) = parse_arg(&args, "--islands").and_then(|s| s.parse().ok()) {
                cfg.islands = isl;
            }
            if let Some(pop) = parse_arg(&args, "--pop").and_then(|s| s.parse().ok()) {
                cfg.pop_per_island = pop;
            }
            if let Some(mc) = parse_arg(&args, "--mc").and_then(|s| s.parse().ok()) {
                cfg.mc_samples = mc;
            }
            if let Some(seed) = parse_arg(&args, "--seed").and_then(|s| s.parse().ok()) {
                cfg.seed = seed;
            }
            let cuda_bin = parse_arg(&args, "--cuda-bin")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("runs/bin/joil_cuda_eval"));
            if cuda_bin.is_file() {
                cfg.cuda_bin_path = Some(cuda_bin);
            }
            let summary = evolve::run_campaign(&cfg, &out_dir)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "completed",
                    "claim_state": summary.claim_state,
                    "total_evaluations": summary.total_evaluations,
                    "valid_f0_evaluations": summary.valid_f0_evaluations,
                    "red_team_rejections": summary.red_team_rejections,
                    "map_elites_occupied_cells": summary.map_elites_occupied_cells,
                    "pareto_front_size": summary.pareto_front_size,
                    "topology_family_counts_in_archive": summary.topology_family_counts_in_archive,
                    "out_dir": out_dir,
                }))?
            );
        }
        "export-3d-preview" => {
            let summary_path = PathBuf::from(
                parse_arg(&args, "--summary")
                    .unwrap_or_else(|| "runs/rust-cuda-evolution-v1/campaign-summary.json".into()),
            );
            let archive_path = PathBuf::from(
                parse_arg(&args, "--archive")
                    .unwrap_or_else(|| "runs/rust-cuda-evolution-v1/map-elites-archive.json".into()),
            );
            let out_path = PathBuf::from(
                parse_arg(&args, "--out")
                    .unwrap_or_else(|| "docs/results/interactive-3d-archetypes-v1.json".into()),
            );
            let bundle = scene::export_interactive_3d_preview_bundle(
                &summary_path,
                &archive_path,
                &out_path,
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "exported",
                    "out_path": out_path,
                    "candidates_3d_count": bundle.candidates_3d.len(),
                    "archive_points_count": bundle.archive_points.len(),
                }))?
            );
        }
        "merge-cluster" => {
            let shards_str = parse_arg(&args, "--shards").ok_or("missing --shards <dir1,dir2,...>")?;
            let shard_dirs: Vec<PathBuf> = shards_str
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| PathBuf::from(s.trim()))
                .collect();
            let nodes_str = parse_arg(&args, "--nodes").unwrap_or_else(|| "xbabe1,xbabe2,xbabe3".into());
            let node_labels: Vec<String> = nodes_str
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string())
                .collect();
            let out_dir = PathBuf::from(
                parse_arg(&args, "--out").unwrap_or_else(|| "runs/rust-cuda-evolution-v1".into()),
            );
            let (summary, gates) = evolve::merge_cluster_shards(&shard_dirs, &node_labels, &out_dir)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "merged",
                    "claim_state": summary.claim_state,
                    "cluster_nodes_participating": gates.cluster_nodes_participating,
                    "total_evaluations": summary.total_evaluations,
                    "valid_f0_evaluations": summary.valid_f0_evaluations,
                    "red_team_rejections": summary.red_team_rejections,
                    "map_elites_occupied_cells": summary.map_elites_occupied_cells,
                    "pareto_front_size": summary.pareto_front_size,
                    "topology_family_counts_in_archive": summary.topology_family_counts_in_archive,
                    "engineering_gates_verification": gates,
                    "out_dir": out_dir,
                }))?
            );
        }
        "wave2-benchmark" => {
            let out_path = PathBuf::from(parse_arg(&args, "--out").unwrap_or_else(|| {
                "docs/results/wave2-multiseed-ablation-and-holdout-v1.json".into()
            }));
            let stl_dir = parse_arg(&args, "--stl-dir").map(PathBuf::from);
            let seeds: Vec<u64> = if let Some(s) = parse_arg(&args, "--seeds") {
                s.split(',')
                    .filter_map(|tok| tok.trim().parse::<u64>().ok())
                    .collect()
            } else {
                vec![
                    20261101, 20261102, 20261103, 20261104, 20261105, 20261106, 20261107,
                    20261108, 20261109, 20261110,
                ]
            };
            let cfg = evolve::EvolutionConfig {
                islands: parse_arg(&args, "--islands")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(2),
                pop_per_island: parse_arg(&args, "--pop")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10),
                generations: parse_arg(&args, "--generations")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(8),
                mc_samples: parse_arg(&args, "--mc")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(12),
                ..evolve::EvolutionConfig::default()
            };
            let report = evolve::run_wave2_multiseed_benchmark(&seeds, &cfg)?;
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let out_json = serde_json::to_string_pretty(&report)?;
            std::fs::write(&out_path, &out_json)?;

            if let Some(dir) = stl_dir {
                std::fs::create_dir_all(&dir)?;
                for (fam, ind) in &report.promoted_wave2_elites_by_family {
                    let (pheno, mesh) = phenotype::Phenotype3D::from_program(&ind.program)?;
                    redteam::check(&ind.program, &pheno, &mesh)
                        .map_err(|e| format!("Wave-2 family {fam} failed redteam: {e:?}"))?;
                    let stl_path = dir.join(format!("{fam}.stl"));
                    mesh.write_stl(&stl_path, fam, 1.0)?;
                }
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "completed",
                    "protocol_id": report.protocol_id,
                    "n_seeds": report.n_seeds,
                    "evaluations_per_seed": report.evaluations_per_seed,
                    "summary_statistics": report.summary_statistics,
                    "pairwise_tests": report.pairwise_tests,
                    "rejected_cohort_10pct_audit": report.rejected_cohort_10pct_audit,
                    "promoted_families_count": report.promoted_wave2_elites_by_family.len(),
                    "out_path": out_path,
                }))?
            );
        }
        "wave3-proof" => {
            let out_path = PathBuf::from(
                parse_arg(&args, "--out")
                    .unwrap_or_else(|| "docs/results/wave3-evolution-proof-v1.json".into()),
            );
            let seeds: Vec<u64> = if let Some(s) = parse_arg(&args, "--seeds") {
                s.split(',')
                    .filter_map(|tok| tok.trim().parse::<u64>().ok())
                    .collect()
            } else {
                vec![
                    20261101, 20261102, 20261103, 20261104, 20261105, 20261106, 20261107,
                    20261108, 20261109, 20261110,
                ]
            };
            let cfg = evolve::EvolutionConfig {
                islands: parse_arg(&args, "--islands")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(2),
                pop_per_island: parse_arg(&args, "--pop")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(9),
                generations: parse_arg(&args, "--generations")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(24),
                mc_samples: parse_arg(&args, "--mc")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(12),
                ..evolve::EvolutionConfig::default()
            };
            let report = if let Some(raw_dir) = parse_arg(&args, "--raw-dir") {
                if out_path.exists() {
                    return Err("raw capture requires a new --out summary path".into());
                }
                evolve::run_wave3_evolution_proof_with_raw_capture(&seeds, &cfg, std::path::Path::new(&raw_dir))?
            } else {
                evolve::run_wave3_evolution_proof(&seeds, &cfg)?
            };
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let pretty = serde_json::to_string_pretty(&report)?;
            let out_json = pretty.lines().map(str::trim_start).collect::<Vec<_>>().join("\n") + "\n";
            if parse_arg(&args, "--raw-dir").is_some() {
                use std::io::Write;
                std::fs::OpenOptions::new().write(true).create_new(true)
                    .open(&out_path)?.write_all(out_json.as_bytes())?;
            } else {
                std::fs::write(&out_path, &out_json)?;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "completed",
                    "protocol_id": report.protocol_id,
                    "n_seeds": report.n_seeds,
                    "generations": report.generations,
                    "evaluations_per_seed_per_arm": report.evaluations_per_seed_per_arm,
                    "total_3d_evaluations_across_arms": report.total_3d_evaluations_across_arms,
                    "baseline_b0_ct35_kpi_score": report.baseline_b0_ct35_kpi_score,
                    "baseline_b0_ct35_cout_p95_mg_l": report.baseline_b0_ct35_cout_p95_mg_l,
                    "promoted_wave3_top_elite_id": report.promoted_wave3_top_elite.id,
                    "promoted_wave3_top_elite_topology": report.promoted_wave3_top_elite.topology_class,
                    "promoted_wave3_top_elite_kpi_score": report.promoted_wave3_top_elite_kpi_score,
                    "promoted_wave3_top_elite_cout_p95_mg_l": report.promoted_wave3_top_elite.mc_robustness.cout_p95_mg_l,
                    "final_summary_statistics": report.final_summary_statistics,
                    "pairwise_tests": report.pairwise_tests,
                    "out_path": out_path,
                }))?
            );
        }
        _ => {
            return Err(
                "supported commands: baseline, kpi, verify-rotation, screen-structure (legacy alias qualify-structure), audit-probe, promote-elite, prepare-cfd-case, qualify-surrogate, qualify-population, qualify-experiment, export-glb-scene, export-3d-preview, compile-3d, evolve, merge-cluster, wave2-benchmark, wave3-proof".into(),
            )
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMITTED_WAVE2: &str =
        include_str!("../../../docs/results/wave2-multiseed-ablation-and-holdout-v1.json");

    #[test]
    fn historical_wave2_receipt_loads_all_promoted_genomes() {
        let report: serde_json::Value = serde_json::from_str(COMMITTED_WAVE2).unwrap();
        let elites = report["promoted_wave2_elites_by_family"]
            .as_object()
            .unwrap();
        assert_eq!(elites.len(), 11);
        for (family, elite) in elites {
            let loaded = load_wave2_elite_program(COMMITTED_WAVE2, family).unwrap();
            let archived: program::DesignProgram =
                serde_json::from_value(elite["program"].clone()).unwrap();
            assert_eq!(loaded, archived.canonicalize().unwrap(), "{family}");
            assert_eq!(loaded.schema_version, 2);
            assert_eq!(loaded.family, "ct35-exotic-head-v2");
            let current_hash = loaded.canonical_sha256().unwrap();
            assert_eq!(current_hash.len(), 64);
            // Authored topology labels no longer belong to the serialized
            // subject. The archived hash cannot identify the prepared case.
            assert_ne!(Some(current_hash.as_str()), elite["genome_sha256"].as_str());
        }
    }

    #[test]
    fn historical_wave2_import_does_not_require_or_trust_evaluations() {
        let mut report: serde_json::Value = serde_json::from_str(COMMITTED_WAVE2).unwrap();
        let family = "trifurcated-6jet-vortex-head";
        let elite = &mut report["promoted_wave2_elites_by_family"][family];
        let genome = elite["program"].clone();
        // A genome-only archive is sufficient. No current Individual, screen,
        // admission or objective evidence is synthesized for missing fields.
        *elite = serde_json::json!({"program": genome});
        let raw = serde_json::to_string(&report).unwrap();
        assert_eq!(
            load_wave2_elite_program(&raw, family).unwrap(),
            load_wave2_elite_program(COMMITTED_WAVE2, family).unwrap()
        );
    }

    #[test]
    fn historical_wave2_import_rejects_missing_or_invalid_genomes() {
        assert!(load_wave2_elite_program(COMMITTED_WAVE2, "missing-family").is_err());
        for raw in [
            "not json",
            "{}",
            r#"{"promoted_wave2_elites_by_family":[]}"#,
        ] {
            assert!(load_wave2_elite_program(raw, "baseline-ct35").is_err());
        }
        let mut report: serde_json::Value = serde_json::from_str(COMMITTED_WAVE2).unwrap();
        for genome in [
            serde_json::Value::Null,
            serde_json::json!({"schema_version": 2}),
            {
                let mut genome =
                    report["promoted_wave2_elites_by_family"]["baseline-ct35"]["program"].clone();
                genome["inlet_split"]["branches_per_port"] = serde_json::json!(0);
                genome
            },
        ] {
            report["promoted_wave2_elites_by_family"]["baseline-ct35"]["program"] = genome;
            let raw = serde_json::to_string(&report).unwrap();
            assert!(load_wave2_elite_program(&raw, "baseline-ct35").is_err());
        }
        report["promoted_wave2_elites_by_family"]["baseline-ct35"] = serde_json::json!({});
        assert!(load_wave2_elite_program(
            &serde_json::to_string(&report).unwrap(),
            "baseline-ct35"
        )
        .is_err());
    }
}
