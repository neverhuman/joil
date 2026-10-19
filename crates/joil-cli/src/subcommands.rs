//! Qualification (`P05`, `P06`, surrogate) and 3D scene/mesh export CLI subcommands.

use crate::cli_support::*;
use joil_core::{
    evolve, experiment, openfoam, phenotype, population, program, redteam, scene, surrogate,
};
use std::path::PathBuf;

pub(crate) fn run_qualification_or_scene_subcommand(
    cmd: &str,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "qualify-surrogate" => {
            let summary_path = PathBuf::from(match parse_arg(args, "--summary") {
                Some(value) => value,
                None => "runs/rust-cuda-evolution-v1/campaign-summary.json".into(),
            });
            let out_path = fresh_required_output(args, "--out", "surrogate-cv-report", ".json")?;
            let raw = std::fs::read_to_string(&summary_path)?;
            let campaign: evolve::CampaignSummary = serde_json::from_str(&raw)?;
            let mut candidates = vec![campaign.baseline_reference.clone()];
            candidates.extend(campaign.top_pareto_elites.clone());
            let report = surrogate::run_surrogate_and_engine_qualification(&candidates)?;
            let out_json = serde_json::to_string_pretty(&report)?;
            write_new(&out_path, &out_json)?;
            println!("{out_json}");
        }
        "qualify-population" => {
            let out_path = fresh_required_output(args, "--out", "p05-population-checks", ".json")?;
            let oil04 =
                population::verify_added_mass_and_pressure_gradient(30.0, 50.0, 2.0e-7, 0.002)?;
            let oil05_06 = population::dispersion_partition_arithmetic(
                &[0.12, 0.28, 0.54, 0.88, 1.32],
                0.004,
                1.45,
                0.18,
                0.18,
            )?;
            let oil06_neg = population::dispersion_partition_arithmetic(
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
            let oil10_neg = population::verify_pbm_breakup_coalescence_and_resolution(
                180.0, true, 0.003, false,
            )
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
            let well_mixed_transport = oil05_06.well_mixed_transport.clone();
            let report = serde_json::json!({
                "schema_version": "0.3.0",
                "packet": "P05",
                "evaluator": "joil-core::population",
                "kind": "software_arithmetic_oracles_not_transport_or_restart_qualification",
                "oil04_added_mass_and_pressure_gradient": oil04,
                "oil05_06_dispersion_partition_arithmetic": oil05_06,
                "well_mixed_turbulent_transport": well_mixed_transport,
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
                "all_p05_software_checks_passed": oil04.passed
                    && oil05_06.arithmetic_checks_passed
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
            write_new(&out_path, &out_json)?;
            println!("{out_json}");
        }
        "qualify-experiment" => {
            let out_path = fresh_required_output(args, "--out", "p06-experiment-checks", ".json")?;
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
            write_new(&out_path, &out_json)?;
            println!("{out_json}");
        }
        "export-glb-scene" => {
            let summary_path = PathBuf::from(match parse_arg(args, "--summary") {
                Some(value) => value,
                None => "runs/rust-cuda-evolution-v1/campaign-summary.json".into(),
            });
            let glb_path = fresh_required_output(args, "--glb", "glb-scene", ".glb")?;
            let out = fresh_output(args, "--out", None, ".json")?;
            let raw = std::fs::read_to_string(&summary_path)?;
            let campaign: evolve::CampaignSummary = serde_json::from_str(&raw)?;
            let elite = campaign
                .top_pareto_elites
                .first()
                .ok_or("no top_pareto_elites in campaign summary")?;
            let surface = phenotype::MultiPatchSurface::from_program(&elite.program, 0.0)?;
            // V5-01-05: bind the scene to the STL of this exact surface, not a historical run file.
            let stl_path = glb_path.with_extension("governing.stl");
            if stl_path.exists() {
                return Err(
                    format!("refusing to replace existing output {}", stl_path.display()).into(),
                );
            }
            surface.write_openfoam_ascii_stl(&stl_path)?;
            let stl_sha = openfoam::file_sha256(&stl_path)?;
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
                "schema_version": "0.3.0",
                "packet": "P09",
                "evaluator": "joil-core::scene",
                "ops_view01_lease_arithmetic_selftest_synthetic_cpu_lists_not_this_run": {
                    "valid_lease": lease_valid,
                    "smt_sibling_mistake_rejected": lease_smt_rejected,
                },
                "governing_surface_stl": stl_path,
                "ops_view02_03_glb_scene_manifest": scene_manifest,
            }))?;
            write_optional_new(&out, &out_json)?;
            println!("{out_json}");
        }
        "compile-3d" => {
            let out_dir = fresh_required_output(args, "--out", "rust-3d-archetypes", "")?;
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
        _ => unreachable!("unexpected subcommand: {cmd}"),
    }
    Ok(())
}
