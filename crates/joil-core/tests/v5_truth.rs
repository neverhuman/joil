//! V5-01 truth repairs: production-path regressions for corrected claim semantics.
//! Mutated artifacts are SYNTHETIC; no test asserts a physical result.

use joil_core::{geometry::compile_design_ir, program::DesignProgram};
use joil_ir::{
    require_observed_descriptors_for_qualification, BackendId, GeometryCountObservation,
    GeometryObservedDescriptors, IrCompiledMesh, ObservedDescriptors, VoxelSdfAuditReport,
    VoxelSdfObserver,
};

fn baseline_ir_and_mesh() -> (joil_ir::DesignIrV1, IrCompiledMesh) {
    let ir = compile_design_ir(&DesignProgram::baseline_ct35()).unwrap();
    let mesh = IrCompiledMesh::compile(&ir, BackendId::RustMultiPatchCfdSurface).unwrap();
    (ir, mesh)
}

/// SYNTHETIC closed tetrahedral void inside the chamber wall (r about 36.5 mm, between the 35 mm
/// fluid radius and the 38 mm wall), wound inward as an enclosed cavity.
fn wall_void_tetra() -> [[f64; 3]; 4] {
    let (x, y, z, s) = (0.0365, 0.0, 0.040, 0.0008);
    [[x, y, z], [x + s, y, z], [x, y + s, z], [x, y, z + s]]
}

fn tetra_faces(v: [[f64; 3]; 4]) -> [[[f64; 3]; 3]; 4] {
    [
        [v[0], v[1], v[2]],
        [v[0], v[3], v[1]],
        [v[0], v[2], v[3]],
        [v[1], v[3], v[2]],
    ]
}

#[test]
fn v5_01_02_enclosed_cavity_ignored_by_legacy_observer_is_not_a_measured_zero() {
    let (ir, mesh) = baseline_ir_and_mesh();
    let base = VoxelSdfObserver::observe(&ir, &mesh);

    let mut cavity = mesh.clone();
    for face in tetra_faces(wall_void_tetra()) {
        cavity.internal_feature_triangles.push((face, 0));
    }
    let report = VoxelSdfObserver::observe(&ir, &cavity);

    // The legacy proxy fields ignore the artifact's enclosed cavity entirely.
    assert_eq!(report.trapped_powder_cavities, 0);
    assert_eq!(report.trapped_cavities_total, base.trapped_cavities_total);
    // Corrected status: neither cavity count is a measurement, so no measured zero is returned.
    assert_eq!(
        report.trapped_powder_cavity_observation.measured_value(),
        None
    );
    assert_eq!(
        report.enclosed_fluid_cavity_observation.measured_value(),
        None
    );
    assert!(report
        .observed_geometry
        .unmeasured_facts
        .iter()
        .any(|f| f.starts_with("trapped_powder_cavities")));
    let obs = ObservedDescriptors {
        geometry: report.observed_geometry.clone(),
        physics: None,
    };
    let err = require_observed_descriptors_for_qualification(&obs).unwrap_err();
    assert!(err.contains("unmeasured"), "{err}");
}

#[test]
fn v5_01_02_separate_mesh_shell_is_measured_but_powder_stays_unmeasured() {
    let (ir, mesh) = baseline_ir_and_mesh();
    let mut shelled = mesh.clone();
    let base_index = shelled.vertices_m.len();
    shelled.vertices_m.extend(wall_void_tetra());
    for [a, b, c] in [[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]] {
        shelled
            .triangles
            .push([base_index + a, base_index + b, base_index + c]);
        shelled.triangle_origin_ids.push(0);
    }
    let base = VoxelSdfObserver::observe(&ir, &mesh);
    let report = VoxelSdfObserver::observe(&ir, &shelled);
    let base_shells = base
        .mesh_shell_component_observation
        .measured_value()
        .unwrap();
    assert_eq!(
        report.mesh_shell_component_observation.measured_value(),
        Some(base_shells + 1)
    );
    assert_eq!(
        report.trapped_powder_cavity_observation.measured_value(),
        None
    );
}

#[test]
fn v5_01_02_baseline_observer_output_cannot_qualify() {
    let (ir, mesh) = baseline_ir_and_mesh();
    let report = VoxelSdfObserver::observe(&ir, &mesh);
    assert!(matches!(
        report.trapped_powder_cavity_observation,
        GeometryCountObservation::NotMeasured { .. }
    ));
    let obs = ObservedDescriptors {
        geometry: report.observed_geometry.clone(),
        physics: None,
    };
    assert!(require_observed_descriptors_for_qualification(&obs).is_err());

    // Positive control: a SYNTHETIC record whose facts are all measured still passes the gate.
    let mut measured = obs.clone();
    measured.geometry.unmeasured_facts.clear();
    assert!(require_observed_descriptors_for_qualification(&measured).is_ok());
}

#[test]
fn v5_01_02_phenotype_record_carries_unmeasured_powder_status() {
    let (pheno, _) =
        joil_core::phenotype::Phenotype3D::from_program(&DesignProgram::baseline_ct35()).unwrap();
    assert_eq!(
        pheno.trapped_powder_cavity_observation.measured_value(),
        None
    );
    let mut value = serde_json::to_value(&pheno).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("trapped_powder_cavity_observation");
    let legacy: joil_core::phenotype::Phenotype3D = serde_json::from_value(value).unwrap();
    assert_eq!(legacy.trapped_cavities, pheno.trapped_cavities);
    assert_eq!(
        legacy.trapped_powder_cavity_observation.measured_value(),
        None
    );
}

#[test]
fn v5_01_02_legacy_records_stay_readable_and_read_as_unmeasured() {
    let (ir, mesh) = baseline_ir_and_mesh();
    let report = VoxelSdfObserver::observe(&ir, &mesh);

    let mut geometry = serde_json::to_value(&report.observed_geometry).unwrap();
    geometry.as_object_mut().unwrap().remove("unmeasured_facts");
    let legacy: GeometryObservedDescriptors = serde_json::from_value(geometry).unwrap();
    assert_eq!(legacy.unmeasured_facts.len(), 1);
    let obs = ObservedDescriptors {
        geometry: legacy,
        physics: None,
    };
    assert!(require_observed_descriptors_for_qualification(&obs).is_err());

    let mut full = serde_json::to_value(&report).unwrap();
    let map = full.as_object_mut().unwrap();
    for key in [
        "trapped_powder_cavity_observation",
        "enclosed_fluid_cavity_observation",
        "mesh_shell_component_observation",
    ] {
        map.remove(key);
    }
    map.get_mut("observed_geometry")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("unmeasured_facts");
    let old: VoxelSdfAuditReport = serde_json::from_value(full).unwrap();
    assert_eq!(old.trapped_powder_cavities, report.trapped_powder_cavities);
    assert_eq!(old.trapped_powder_cavity_observation.measured_value(), None);
    assert_eq!(old.mesh_shell_component_observation.measured_value(), None);

    let round_trip: VoxelSdfAuditReport =
        serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
    assert_eq!(
        round_trip.trapped_powder_cavity_observation,
        report.trapped_powder_cavity_observation
    );
    assert_eq!(
        round_trip.mesh_shell_component_observation,
        report.mesh_shell_component_observation
    );
    assert_eq!(
        round_trip.observed_geometry.unmeasured_facts,
        report.observed_geometry.unmeasured_facts
    );
}

#[test]
fn v5_01_02_partial_red_team_screen_reports_unevaluated_facts() {
    use joil_core::{phenotype::Phenotype3D, redteam};
    let base = DesignProgram::baseline_ct35();
    let (pheno, mesh) = Phenotype3D::from_program(&base).unwrap();
    // The measured checks still pass, but the pass is explicitly incomplete.
    assert!(redteam::check(&base, &pheno, &mesh).is_ok());
    let facts = redteam::unevaluated_geometry_facts(&pheno);
    assert!(facts
        .iter()
        .any(|f| f.starts_with("trapped_powder_cavities")));
    assert!(facts
        .iter()
        .any(|f| f.starts_with("enclosed_fluid_cavities")));

    // A nonzero legacy count still rejects: no threshold is relaxed.
    let mut shelled = pheno.clone();
    shelled.trapped_cavities = 1;
    let err = redteam::check(&base, &shelled, &mesh).unwrap_err();
    assert!(err.contains(&redteam::RedTeamReject::TrappedPowderOrFluidCavity));

    // Missing observations stay unevaluated, including legacy defaults.
    let mut missing = pheno.clone();
    missing.observed_geometry = None;
    missing.trapped_powder_cavity_observation = GeometryCountObservation::legacy_unrecorded();
    let facts = redteam::unevaluated_geometry_facts(&missing);
    assert!(facts
        .iter()
        .any(|f| f.starts_with("observed_geometry missing")));
    assert!(facts
        .iter()
        .any(|f| f == "trapped_powder_cavities: legacy record without measurement status"));

    // Positive control: a SYNTHETIC record with every fact measured has nothing unevaluated.
    let mut measured = pheno.clone();
    measured.trapped_powder_cavity_observation = GeometryCountObservation::Measured {
        value: 0,
        method: "SYNTHETIC".into(),
    };
    measured
        .observed_geometry
        .as_mut()
        .unwrap()
        .unmeasured_facts
        .clear();
    assert!(redteam::unevaluated_geometry_facts(&measured).is_empty());
}

#[test]
fn v5_01_02_individual_serializes_incomplete_screen_and_legacy_reads_unevaluated() {
    use joil_core::evolve::{evaluate_individual, EvolutionConfig, Individual};
    let cfg = EvolutionConfig {
        mc_samples: 6,
        ..EvolutionConfig::default()
    };
    let (ind, _) =
        evaluate_individual("base".into(), DesignProgram::baseline_ct35(), 0, 0, &cfg).unwrap();
    assert!(ind.red_team_passed);
    assert!(!ind.red_team_unevaluated_facts.is_empty());
    let mut value = serde_json::to_value(&ind).unwrap();
    assert!(!value["red_team_unevaluated_facts"]
        .as_array()
        .unwrap()
        .is_empty());
    value
        .as_object_mut()
        .unwrap()
        .remove("red_team_unevaluated_facts");
    let legacy: Individual = serde_json::from_value(value).unwrap();
    assert!(legacy.red_team_passed);
    assert_eq!(
        legacy.red_team_unevaluated_facts,
        vec!["legacy record: cavity and topology facts not evaluated".to_string()]
    );
}

fn insufficient_missing(obs: &joil_evidence::observation::Observation) -> Vec<String> {
    match obs {
        joil_evidence::observation::Observation::Insufficient { missing, .. } => missing.clone(),
        other => panic!("expected Insufficient, got {other:?}"),
    }
}

#[test]
fn v5_01_03_empty_islands_and_literal_hashes_cannot_yield_verified_replay() {
    use joil_core::surrogate::{campaign_replay_status, BudgetLedger, CampaignCheckpoint};
    let ckpt = CampaignCheckpoint {
        schema_version: 1,
        evaluator_sha256: "eval-v1-sha256".into(),
        cuda_binary_sha256: "cuda-v1-sha256".into(),
        generation: 10,
        rng_states: vec![0x1234_5678, 0x9abc_def0],
        islands: vec![],
        seen_phenotype_hashes: vec![],
        ledger: BudgetLedger::default(),
    };
    // The label comparison still succeeds, which is why it cannot stand for replay.
    assert!(ckpt
        .verify_resume_compatibility(1, "eval-v1-sha256", "cuda-v1-sha256")
        .is_ok());
    let missing = insufficient_missing(&campaign_replay_status(&ckpt));
    for needle in [
        "native kill/resume/replay test of a real campaign",
        "non-empty island populations",
        "phenotype deduplication state",
        "content-addressed evaluator SHA-256",
        "content-addressed CUDA binary SHA-256",
    ] {
        assert!(missing.iter().any(|m| m == needle), "{needle}: {missing:?}");
    }
}

#[test]
fn v5_01_03_populated_checkpoint_still_reports_unsupported_replay() {
    use joil_core::evolve::{evaluate_individual, EvolutionConfig};
    use joil_core::surrogate::{campaign_replay_status, BudgetLedger, CampaignCheckpoint};
    let cfg = EvolutionConfig {
        mc_samples: 6,
        ..EvolutionConfig::default()
    };
    let (ind, _) =
        evaluate_individual("base".into(), DesignProgram::baseline_ct35(), 0, 0, &cfg).unwrap();
    let ckpt = CampaignCheckpoint {
        schema_version: 1,
        evaluator_sha256: "a".repeat(64),
        cuda_binary_sha256: "b".repeat(64),
        generation: 1,
        rng_states: vec![1],
        seen_phenotype_hashes: vec![ind.genome_sha256.clone()],
        islands: vec![vec![ind]],
        ledger: BudgetLedger::default(),
    };
    assert_eq!(
        insufficient_missing(&campaign_replay_status(&ckpt)),
        vec!["native kill/resume/replay test of a real campaign".to_string()]
    );
}

#[test]
fn v5_01_03_legacy_checkpoint_flag_reads_into_compatibility_selftest() {
    use joil_core::surrogate::SurrogateQualificationReport;
    let legacy = serde_json::json!({
        "schema_version": 2,
        "kind": "sandbox_prior_screen_cross_validation_not_independent_ood",
        "independent_label_validation": {"status": "insufficient", "reason": "SYNTHETIC", "missing": []},
        "total_records_input": 1,
        "quarantined_failure_records": 0,
        "valid_training_records": 1,
        "in_distribution_rmse_mg_l": 0.4,
        "sandbox_authored_family_holdout_mean_rmse_mg_l": 4.0,
        "sandbox_authored_family_holdout_mean_2sigma_coverage": 0.86,
        "sandbox_holdout_std_exceeds_training_std": true,
        "failure_quarantine_verified": true,
        "checkpoint_determinism_verified": true,
        "evaluator_drift_rejection_verified": true,
        "budget_conservation_verified": true,
        "family_holdouts": []
    });
    let report: SurrogateQualificationReport = serde_json::from_value(legacy).unwrap();
    assert!(report.checkpoint_compatibility_selftest_passed);
    insufficient_missing(&report.campaign_replay);
    let written = serde_json::to_value(&report).unwrap();
    assert!(written.get("checkpoint_determinism_verified").is_none());
    assert_eq!(written["campaign_replay"]["status"], "insufficient");
}

#[test]
fn v5_01_03_dispersion_helper_is_arithmetic_only() {
    use joil_core::population::{dispersion_partition_arithmetic, DispersionQualificationResult};
    // The drift identity holds for an arbitrary non-monotone SYNTHETIC profile: it cannot
    // discriminate correct from incorrect transport.
    let r = dispersion_partition_arithmetic(&[0.9, 0.1, 1.7, 0.3, 2.2], 0.004, 1.45, 0.18, 0.18)
        .unwrap();
    assert!(r.telescoping_drift_identity_residual <= 1e-13);
    assert_eq!(
        r.kind,
        "arithmetic_identity_check_not_transport_qualification"
    );
    insufficient_missing(&r.well_mixed_transport);

    let legacy = serde_json::json!({
        "uncompensated_turbophoresis_drift_err": 300.0,
        "drift_corrected_well_mixed_err": 0.0,
        "les_sgs_tke_fraction": 0.11,
        "negative_control_rans_on_les_rejected": true,
        "passed": true
    });
    let old: DispersionQualificationResult = serde_json::from_value(legacy).unwrap();
    assert!(old.arithmetic_checks_passed);
    assert_eq!(
        old.kind,
        "arithmetic_identity_check_not_transport_qualification"
    );
    insufficient_missing(&old.well_mixed_transport);
    let written = serde_json::to_value(&old).unwrap();
    assert!(written.get("passed").is_none());
    assert!(written
        .get("negative_control_rans_on_les_rejected")
        .is_none());
}

#[test]
fn v5_01_04_default_config_requests_no_gpu_and_missing_binary_attaches_nothing() {
    use joil_core::evolve::{attach_cuda_audit_with_vram, evaluate_individual, EvolutionConfig};
    let cfg = EvolutionConfig::default();
    assert!(cfg.cuda_bin_path.is_none());
    assert!(cfg.gpu_vram_target_pct.is_none());
    let mut legacy = serde_json::to_value(&cfg).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("gpu_vram_target_pct");
    let old: EvolutionConfig = serde_json::from_value(legacy).unwrap();
    assert!(old.gpu_vram_target_pct.is_none());

    let small = EvolutionConfig {
        mc_samples: 4,
        ..EvolutionConfig::default()
    };
    let (mut ind, _) =
        evaluate_individual("base".into(), DesignProgram::baseline_ct35(), 0, 0, &small).unwrap();
    assert!(ind.cuda_sdf_audit.is_none());
    attach_cuda_audit_with_vram(
        &mut ind,
        std::path::Path::new("runs/v5-01-04-missing-cuda-binary"),
        Some(85.5),
    );
    assert!(ind.cuda_sdf_audit.is_none());
}

/// D-17 regression: a SYNTHETIC recording executable at the CUDA boundary sees a VRAM target only
/// when one is explicitly requested; it exits nonzero, so nothing is attached as if measured.
#[cfg(unix)]
#[test]
fn v5_01_04_cuda_boundary_records_only_explicit_bounded_requests() {
    use joil_core::evolve::{attach_cuda_audit_with_vram, evaluate_individual, EvolutionConfig};
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("joil-v5-01-04-recorder-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let record = dir.join("argv.txt");
    let fake = dir.join("fake_cuda_eval");
    std::fs::write(
        &fake,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{}'\nexit 3\n",
            record.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let small = EvolutionConfig {
        mc_samples: 4,
        ..EvolutionConfig::default()
    };
    let (mut ind, _) =
        evaluate_individual("base".into(), DesignProgram::baseline_ct35(), 0, 0, &small).unwrap();

    attach_cuda_audit_with_vram(&mut ind, &fake, None);
    let first = std::fs::read_to_string(&record).unwrap();
    assert!(first.contains("--candidate-json"));
    assert!(!first.contains("--vram-target-pct"));

    attach_cuda_audit_with_vram(&mut ind, &fake, Some(50.0));
    let second = std::fs::read_to_string(&record).unwrap();
    assert!(second.contains("--vram-target-pct\n50.00\n"), "{second}");
    assert!(ind.cuda_sdf_audit.is_none());
    std::fs::remove_dir_all(dir).unwrap();
}
