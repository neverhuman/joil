use crucible_adapters::openfoam::*;

fn ref_at_datum() -> PressureReference {
    PressureReference {
        rho_kg_m3: 1000.0,
        gravity_m_s2: [0.0, 0.0, -9.81],
        datum_position_m: [0.0, 0.0, 0.0],
        datum_absolute_pa: Some(101_325.0),
    }
}

#[test]
fn capability_mismatch_is_refused_even_if_executable_exists() {
    let expected = ExpectedIdentity {
        source_revision: "a4867fc914c1".into(),
        build: "14-a4867fc914c1".into(),
        container_image_digest: "sha256:pinned".into(),
        compiler: "gcc-pinned".into(),
        mpi: "openmpi-pinned".into(),
        model: "SSG".into(),
        solver_executable_sha256: "a".repeat(64),
    };
    let actual = RuntimeIdentity {
        source_revision: "other".into(),
        build: "other".into(),
        container_image_digest: "sha256:other".into(),
        compiler: "gcc-other".into(),
        mpi: "mpi-other".into(),
        models_available: vec!["kOmegaSST".into()],
        models_absent: vec!["SSG".into()],
        solver_executable_sha256: "b".repeat(64),
    };
    assert!(verify_identity(&expected, &actual).is_err());
}

#[test]
fn capability_card_is_the_runtime_identity_source_and_digest_is_enforced() {
    let card_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../physics/openfoam/verification/v5/capability-card.json");
    let card = read_capability_card(&card_path).unwrap();
    let actual = runtime_identity(&card);
    let expected = ExpectedIdentity {
        source_revision: card.source_revision.clone(),
        build: card.build.clone(),
        container_image_digest: card.container_image_digest.clone(),
        compiler: card.compiler.clone(),
        mpi: card.mpi.clone(),
        model: "SSG".into(),
        solver_executable_sha256: card.solver_executable_sha256.clone(),
    };
    assert!(verify_identity(&expected, &actual).is_ok());
    let bogus = ExpectedIdentity {
        solver_executable_sha256: "f".repeat(64),
        ..expected
    };
    assert!(verify_identity(&bogus, &actual).is_err());
}

#[test]
fn executable_path_is_digest_bound_and_wall_limited() {
    use std::time::Duration;
    let executable = std::path::Path::new("/usr/bin/true");
    let digest = sha256_file(executable).unwrap();
    let expected = ExpectedIdentity {
        source_revision: "a4867fc914c1".into(),
        build: "14-a4867fc914c1".into(),
        container_image_digest:
            "sha256:8866eda29be2bcbc05e0fa9536cacb3c63ff25c97179e058959ffbeb90da7199".into(),
        compiler: "g++ 13.3.0-6ubuntu2~24.04.1".into(),
        mpi: "Open MPI 4.1.6".into(),
        model: "SSG".into(),
        solver_executable_sha256: digest.clone(),
    };
    let card_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../physics/openfoam/verification/v5/capability-card.json");
    let card = read_capability_card(&card_path).unwrap();
    let mut actual = runtime_identity(&card);
    actual.solver_executable_sha256 = digest;
    let case = std::env::temp_dir().join(format!("joil-openfoam-adapter-{}", std::process::id()));
    std::fs::create_dir_all(&case).unwrap();
    let run = execute_verified(
        executable,
        &expected,
        &actual,
        &case,
        &[],
        Duration::from_secs(1),
    )
    .unwrap();
    assert_eq!(run.exit_code, Some(0));
    assert_eq!(run.log_sha256.len(), 64);
    let wrong = std::path::Path::new("/usr/bin/echo");
    assert!(execute_verified(
        wrong,
        &expected,
        &actual,
        &case,
        &[],
        Duration::from_secs(1)
    )
    .is_err());
    let sleeper = std::path::Path::new("/usr/bin/sleep");
    if sleeper.is_file() {
        let sleep_digest = sha256_file(sleeper).unwrap();
        let slow_expected = ExpectedIdentity {
            solver_executable_sha256: sleep_digest.clone(),
            ..expected
        };
        let mut slow_actual = actual;
        slow_actual.solver_executable_sha256 = sleep_digest;
        let timed = execute_verified(
            sleeper,
            &slow_expected,
            &slow_actual,
            &case,
            &["2".into()],
            Duration::from_millis(50),
        );
        assert!(timed.is_err());
        assert!(std::fs::read_dir(&case).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("native-execution-")
        }));
    }
    std::fs::remove_dir_all(case).unwrap();
}

#[test]
fn pressure_dimensions_density_datum_and_two_elevation_outlets_are_explicit() {
    let lower = pressure(
        0.0,
        KINEMATIC_PRESSURE_DIMENSIONS,
        PressureKind::Reduced,
        [0.0; 3],
        [0.0, 0.0, 0.0],
        ref_at_datum(),
    )
    .unwrap();
    let upper = pressure(
        0.0,
        KINEMATIC_PRESSURE_DIMENSIONS,
        PressureKind::Reduced,
        [0.0; 3],
        [0.0, 0.0, 10.0],
        ref_at_datum(),
    )
    .unwrap();
    assert_eq!(lower.static_gauge_pa, 0.0);
    assert!((upper.static_gauge_pa + 98_100.0).abs() < 1e-9);
    assert!((upper.static_absolute_pa.unwrap() - 3_225.0).abs() < 1e-9);
    let taps = pressure_tap_drop(lower, upper);
    assert!((taps.static_pa - 98_100.0).abs() < 1e-9);
    assert_ne!(
        taps.static_pa,
        mechanical_flux_w(
            FluxState {
                pressure_pa: 0.0,
                velocity_m_s: [0.0; 3],
                area_m2: 1.0,
                mass_flow_kg_s: 1.0,
                elevation_m: 0.0
            },
            1000.0,
            9.81,
        )
        .unwrap()
    );
    assert!(pressure(
        1.0,
        [0; 7],
        PressureKind::Static,
        [0.0; 3],
        [0.0; 3],
        ref_at_datum()
    )
    .is_err());
    let no_datum = PressureReference {
        datum_absolute_pa: None,
        ..ref_at_datum()
    };
    let unknown_absolute = pressure(
        -10.0,
        PA_DIMENSIONS,
        PressureKind::Static,
        [0.0; 3],
        [0.0; 3],
        no_datum,
    )
    .unwrap();
    assert!(unknown_absolute.static_absolute_pa.is_none());
    let wrong_density = PressureReference {
        rho_kg_m3: 1.0,
        ..ref_at_datum()
    };
    let p = pressure(
        1000.0,
        PA_DIMENSIONS,
        PressureKind::Static,
        [0.0; 3],
        [0.0; 3],
        wrong_density,
    )
    .unwrap();
    assert!((p.static_gauge_pa - 1000.0).abs() < 1e-12);
}

#[test]
fn total_pressure_and_integrated_mechanical_flux_are_separate() {
    let total = pressure(
        5.0,
        KINEMATIC_PRESSURE_DIMENSIONS,
        PressureKind::Total,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        ref_at_datum(),
    )
    .unwrap();
    assert!((total.static_gauge_pa + 7_500.0).abs() < 1e-9);
    assert!((total.total_gauge_pa - 5_000.0).abs() < 1e-9);
    let state = FluxState {
        pressure_pa: 10_000.0,
        velocity_m_s: [3.0, 4.0, 0.0],
        area_m2: 0.02,
        mass_flow_kg_s: 2.0,
        elevation_m: 2.0,
    };
    let flux = mechanical_flux_w(state, 1000.0, 9.81).unwrap();
    assert!((flux - (10.0 + 12.5 + 19.62) * 2.0).abs() < 1e-10);
}

fn controller() -> SplitController {
    SplitController::new(SplitControllerConfig {
        target_reject_fraction: 0.25,
        reachable_min: 0.1,
        reachable_max: 0.4,
        initial_backpressure_pa: -200.0,
        pressure_min_pa: -400.0,
        pressure_max_pa: 0.0,
        kp_pa_per_fraction: 100.0,
        ki_pa_per_fraction_s: 1.0,
        settle_tolerance: 0.02,
        settle_window: 3,
        sample_count: 4,
        sample_interval_s: 1.0,
    })
    .unwrap()
}

fn controller_config() -> SplitControllerConfig {
    SplitControllerConfig {
        target_reject_fraction: 0.25,
        reachable_min: 0.1,
        reachable_max: 0.4,
        initial_backpressure_pa: -200.0,
        pressure_min_pa: -400.0,
        pressure_max_pa: 0.0,
        kp_pa_per_fraction: 100.0,
        ki_pa_per_fraction_s: 1.0,
        settle_tolerance: 0.02,
        settle_window: 3,
        sample_count: 4,
        sample_interval_s: 1.0,
    }
}

#[test]
fn controller_enforces_reachability_settling_saturation_and_frozen_sampling() {
    let config = SplitControllerConfig {
        target_reject_fraction: 0.9,
        reachable_min: 0.1,
        reachable_max: 0.4,
        initial_backpressure_pa: -200.0,
        pressure_min_pa: -400.0,
        pressure_max_pa: 0.0,
        kp_pa_per_fraction: 100.0,
        ki_pa_per_fraction_s: 1.0,
        settle_tolerance: 0.02,
        settle_window: 3,
        sample_count: 4,
        sample_interval_s: 1.0,
    };
    assert!(SplitController::new(config).is_err());
    let mut c = controller();
    let commanded = c.observe(0.5, 0.20, 0.5).unwrap();
    assert!(commanded < -200.0);
    // Restore a target-matched sequence to exercise the settle gate.
    assert!(c.observe(1.0, 0.25, 1.0).is_ok());
    assert!(c.accepted_statistics(4).is_err());
    c.observe(2.0, 0.25, 1.0).unwrap();
    c.observe(3.0, 0.25, 1.0).unwrap();
    c.observe(4.0, 0.25, 1.0).unwrap();
    c.accepted_statistics(4).unwrap();
    let record = c.record();
    assert_eq!(record.config.sample_count, 4);
    assert_eq!(record.config.sample_interval_s, 1.0);
    assert_eq!(record.history.len(), 5);
    assert!(serde_json::to_value(record)
        .unwrap()
        .get("history")
        .is_some());
    assert!(c.accepted_statistics(3).is_err());
    let mut saturated_config = controller_config();
    saturated_config.kp_pa_per_fraction = 10_000.0;
    let mut saturated = SplitController::new(saturated_config).unwrap();
    saturated.observe(1.0, 0.1, 1.0).unwrap();
    assert!(saturated.history()[0].saturated);
    assert!(saturated.accepted_statistics(4).is_err());
}

#[test]
fn oscillatory_refinement_and_incomplete_native_restart_never_pass() {
    assert!(richardson_certificate(&[1.0, 1.2, 1.1], 2.0, 1.0).is_err());
    assert!(richardson_certificate(&[1.0, 1.2, 1.5], 2.0, 1.0).is_err());
    let incomplete = NativeCaseEvidence {
        requested_model: "SSG".into(),
        native_solver_exit_code: Some(0),
        field_write_complete: false,
        absolute_pressure_reference_pa: None,
        mesh_health_passed: false,
        restart_state_sha256: None,
        statistics_state_sha256: None,
    };
    assert!(native_case_pass(&incomplete).is_err());
    let good_restart = RestartComparison {
        fields_complete_before: true,
        fields_complete_after: true,
        statistics_complete_before: true,
        statistics_complete_after: true,
        mesh_health_before: true,
        mesh_health_after: true,
        absolute_pressure_known: true,
        observables_before: vec![4.0, -200.0, 101_325.0],
        observables_after: vec![4.0001, -200.01, 101_326.0],
        observable_tolerances: vec![0.001, 0.1, 2.0],
    };
    assert!(verify_restart(&good_restart).is_ok());
    let mut half_written = good_restart.clone();
    half_written.fields_complete_after = false;
    assert!(verify_restart(&half_written).is_err());
    let mut pressure_unknown = good_restart.clone();
    pressure_unknown.absolute_pressure_known = false;
    assert!(verify_restart(&pressure_unknown).is_err());
    let mut changed = good_restart;
    changed.observables_after[0] = 5.0;
    assert!(verify_restart(&changed).is_err());
}

#[test]
fn pressure_rejects_1000_fold_input_error_and_zero_or_wrong_density() {
    let mut bad = ref_at_datum();
    bad.rho_kg_m3 = 0.0;
    assert!(pressure(
        100_000.0,
        PA_DIMENSIONS,
        PressureKind::Static,
        [0.0; 3],
        [0.0; 3],
        bad
    )
    .is_err());
    // Independent pressure-control fixture is 100 Pa. Both a 1000x scale error and
    // a density mismatch fail the control-volume comparison.
    let expected = 100.0;
    let wrong_scale = pressure(
        100_000.0,
        PA_DIMENSIONS,
        PressureKind::Static,
        [0.0; 3],
        [0.0; 3],
        ref_at_datum(),
    )
    .unwrap();
    let wrong_density = PressureReference {
        rho_kg_m3: 1.0,
        ..ref_at_datum()
    };
    let wrong_rho = pressure(
        0.1,
        KINEMATIC_PRESSURE_DIMENSIONS,
        PressureKind::Static,
        [0.0; 3],
        [0.0; 3],
        wrong_density,
    )
    .unwrap();
    assert!((wrong_scale.static_gauge_pa - expected).abs() > 1.0);
    assert!((wrong_rho.static_gauge_pa - expected).abs() > 1.0);
    let good = pressure(
        100.0,
        PA_DIMENSIONS,
        PressureKind::Static,
        [0.0; 3],
        [0.0; 3],
        ref_at_datum(),
    )
    .unwrap();
    assert!((good.static_gauge_pa - expected).abs() < 1e-12);
}
