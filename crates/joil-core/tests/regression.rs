use joil_core::{geometry::baseline, metrics::*};
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn limits() -> Limits {
    Limits {
        cout_mg_l: 30.,
        dp_pa: 200_000.,
        energy_j_per_treated_m3: 250_000.,
        reject_min: 0.019,
        reject_max: 0.021,
    }
}
fn obs(name: &str, c: f64) -> Sample {
    Sample::Observed {
        scenario: name.into(),
        cout_upper_mg_l: c,
        dp_upper_pa: 100_000.,
        energy_upper_j_per_treated_m3: 110_000.,
        reject_fraction: 0.02,
    }
}
fn point(q: f64, a: f64, b: f64) -> OperatingPoint {
    OperatingPoint {
        feed_m3_h: q,
        samples: vec![obs("a", a), obs("b", b)],
    }
}
fn required() -> Vec<String> {
    vec!["a".into(), "b".into()]
}

#[test]
fn tracer_does_not_treat_water() {
    for r in [0., 0.02, 0.1, 0.8] {
        close(concentration_mg_l(500., r, r).unwrap(), 500.);
    }
}
#[test]
fn split_denominator_changes_threshold_verdict() {
    let c = concentration_mg_l(500., 0.94, 0.02).unwrap();
    close(c, 30.612244897959183);
    assert!(c > 30.);
}
#[test]
fn oil_capture_extremes() {
    close(concentration_mg_l(500., 1., 0.02).unwrap(), 0.);
    close(
        concentration_mg_l(500., 0., 0.02).unwrap(),
        510.2040816326531,
    );
}
#[test]
fn reduced_form_matches_mass_form() {
    let e = 0.95;
    let g = reduced_efficiency(e, 0.02).unwrap();
    close(500. * (1. - g), concentration_mg_l(500., e, 0.02).unwrap());
}
#[test]
fn negative_reduced_efficiency_is_preserved() {
    assert!(reduced_efficiency(0.01, 0.02).unwrap() < 0.);
}
#[test]
fn concentration_rejects_invalid_inputs() {
    for (c, e, r) in [
        (-1., 0.9, 0.02),
        (500., 1.1, 0.02),
        (500., -0.1, 0.02),
        (500., 0.9, 1.),
        (500., 0.9, -0.1),
        (f64::NAN, 0.9, 0.02),
        (500., 0.9, f64::INFINITY),
    ] {
        assert!(concentration_mg_l(c, e, r).is_err());
    }
}
#[test]
fn mass_weighted_bins() {
    close(weighted_capture(&[0.25, 0.75], &[0.2, 1.]).unwrap(), 0.8);
    assert!(weighted_capture(&[1., 1.], &[0.5, 0.5]).is_err());
    assert!(weighted_capture(&[], &[]).is_err());
}
#[test]
fn pdr_uses_correct_outlets() {
    close(pdr(500_000., 140_000., 300_000.).unwrap(), 1.8);
    assert!(pdr(1., 0., 1.).is_err());
}
#[test]
fn inventory_retained_oil_is_not_capture() {
    let b = OilBalance {
        injected_kg: 1.,
        reject_kg: 0.8,
        treated_kg: 0.1,
        remaining_kg: 0.06,
        wall_kg: 0.04,
    };
    close(b.residual_fraction().unwrap(), 0.);
    let (lo, hi) = b.capture_bounds(1e-9).unwrap();
    close(lo, 0.8);
    close(hi, 0.9);
}
#[test]
fn missing_oil_does_not_pass() {
    let b = OilBalance {
        injected_kg: 1.,
        reject_kg: 0.8,
        treated_kg: 0.1,
        remaining_kg: 0.,
        wall_kg: 0.,
    };
    assert!(b.capture_bounds(0.01).is_err());
}
#[test]
fn invalid_inventory_rejected() {
    let b = OilBalance {
        injected_kg: 0.,
        reject_kg: 0.,
        treated_kg: 0.,
        remaining_kg: 0.,
        wall_kg: 0.,
    };
    assert!(b.residual_fraction().is_err());
}
#[test]
fn disjoint_scenario_maxima_have_no_common_capacity() {
    let ps = [point(2., 20., 40.), point(4., 40., 20.)];
    assert_eq!(
        common_tested_capacity(&ps, &required(), &limits()).unwrap(),
        None
    );
}
#[test]
fn nonmonotonic_operating_map() {
    let ps = [
        point(5., 40., 40.),
        point(2., 20., 20.),
        point(3., 40., 40.),
        point(4., 25., 25.),
    ];
    let c = common_tested_capacity(&ps, &required(), &limits())
        .unwrap()
        .unwrap();
    close(c.feed_m3_h, 4.);
    close(c.treated_m3_h, 3.92);
}
#[test]
fn missing_or_unknown_scenario_never_passes() {
    for samples in [
        vec![obs("a", 20.)],
        vec![
            obs("a", 20.),
            Sample::Unknown {
                scenario: "b".into(),
                reason: "timeout".into(),
            },
        ],
    ] {
        let p = OperatingPoint {
            feed_m3_h: 4.,
            samples,
        };
        assert_eq!(
            common_tested_capacity(&[p], &required(), &limits()).unwrap(),
            None
        );
    }
}
#[test]
fn reject_cheating_fails() {
    let s = Sample::Observed {
        scenario: "a".into(),
        cout_upper_mg_l: 10.,
        dp_upper_pa: 1.,
        energy_upper_j_per_treated_m3: 1.,
        reject_fraction: 0.2,
    };
    let p = OperatingPoint {
        feed_m3_h: 4.,
        samples: vec![s, obs("b", 20.)],
    };
    assert_eq!(
        common_tested_capacity(&[p], &required(), &limits()).unwrap(),
        None
    );
}
#[test]
fn duplicate_scenarios_are_errors() {
    let p = OperatingPoint {
        feed_m3_h: 4.,
        samples: vec![obs("a", 20.), obs("a", 20.)],
    };
    assert!(common_tested_capacity(&[p], &required(), &limits()).is_err());
}
#[test]
fn duplicate_flows_are_errors() {
    assert!(common_tested_capacity(
        &[point(4., 20., 20.), point(4., 20., 20.)],
        &required(),
        &limits()
    )
    .is_err());
}
#[test]
fn unknown_is_not_zero_concentration() {
    let raw = r#"{"status":"observed","scenario":"a","dp_upper_pa":1,"energy_upper_j_per_treated_m3":1,"reject_fraction":0.02}"#;
    assert!(serde_json::from_str::<Sample>(raw).is_err());
}
#[test]
fn no_baseline_means_no_gain() {
    for x in [0., -1., f64::NAN] {
        assert!(gain(4., x).is_err());
    }
    close(gain(4.4, 4.).unwrap(), 0.1);
}
#[test]
fn baseline_dimensions_match_independent_fixture() {
    let d = baseline().derive().unwrap();
    close(d.d1_mm, 70.);
    close(d.d2_mm, 35.);
    close(d.overflow_diameter_mm, 1.4);
    close(d.cone_length_mm, 99.24743184330991);
    close(d.z_underflow_mm, 1639.2474318433099);
    close(d.z_head_end_mm, 186.74743184330993);
    assert!((d.fluid_volume_mm3 - 1105802.5227442954).abs() < 1e-6);
}
#[test]
fn geometry_scale_preserves_dimensionless_shape() {
    let g = baseline();
    let a = g.derive().unwrap();
    let mut h = g.clone();
    let p = &mut h.parameters;
    p.d2_mm *= 2.;
    p.inlet_z_offset_mm *= 2.;
    p.inlet_length_cfd_mm *= 2.;
    p.inlet_length_part_mm *= 2.;
    p.plenum_radius_mm *= 2.;
    p.plenum_length_mm *= 2.;
    p.wall_thickness_mm *= 2.;
    let b = h.derive().unwrap();
    close(b.z_underflow_mm / a.z_underflow_mm, 2.);
    close(b.fluid_volume_mm3 / a.fluid_volume_mm3, 8.);
}
#[test]
fn invalid_gene_does_not_clamp_to_baseline() {
    let mut g = baseline();
    g.parameters.d2_mm = f64::NAN;
    assert!(g.derive().is_err());
    g = baseline();
    g.family = "freeform-not-implemented".into();
    assert!(g.derive().is_err());
}
#[test]
fn identity_changes_with_gene() {
    let g = baseline();
    let mut h = g.clone();
    h.parameters.do_over_d2 = 0.05;
    assert_ne!(g.sha256().unwrap(), h.sha256().unwrap());
}
#[test]
fn json_roundtrip_and_unknown_field() {
    let g = baseline();
    let bytes = serde_json::to_vec(&g).unwrap();
    let h: joil_core::geometry::Genome = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(g.sha256().unwrap(), h.sha256().unwrap());
    let mut v = serde_json::to_value(&g).unwrap();
    v["secret_default"] = 1.into();
    assert!(serde_json::from_value::<joil_core::geometry::Genome>(v).is_err());
}

#[test]
fn design_program_canonicalizes_inactive_genes_and_rejects_unknown_fields() {
    use joil_core::program::DesignProgram;
    let mut prog = DesignProgram::baseline_ct35();
    prog.inlet_split.enabled = false;
    prog.inlet_split.vane_thickness_mm = 1.65;
    prog.inlet_split.branches_per_port = 3;
    prog.skimmer.length_mm = 0.0;
    prog.skimmer.outer_radius_mm = 3.5;
    prog.skimmer.capture_flutes = 6;
    let norm = prog.normalize().unwrap();
    assert!(norm.is_exact_baseline().unwrap());
    assert_eq!(
        norm.canonical_sha256().unwrap(),
        DesignProgram::baseline_ct35().canonical_sha256().unwrap()
    );

    let mut v = serde_json::to_value(&norm).unwrap();
    v["unexpected_exotic_key"] = serde_json::json!(42);
    assert!(serde_json::from_value::<DesignProgram>(v).is_err());
}

#[test]
fn phenotype3d_watertight_mesh_and_redteam_gates() {
    use joil_core::{
        phenotype::Phenotype3D,
        program::{DesignProgram, Rng},
        redteam::{self, RedTeamReject},
    };
    let mut rng = Rng::new(20261015);
    let archetypes = DesignProgram::seed_archetypes(&mut rng);
    assert!(archetypes.len() >= 15);
    for prog in &archetypes {
        let (pheno, mesh) = Phenotype3D::from_program(prog).unwrap();
        let census = mesh.edge_census();
        assert_eq!(census.open_boundary_edges, 0);
        assert_eq!(census.non_manifold_edges, 0);
        assert!(mesh.signed_volume() > 0.0);
        assert_eq!(pheno.frozen_collar_and_tail_diff_mm3, 0.0);
        assert!(pheno.minimum_wall_clearance_mm >= 1.0);
        assert!(pheno.minimum_passage_diameter_mm >= 1.4);
        assert!(redteam::check(prog, &pheno, &mesh).is_ok());
    }

    // Negative check: deliberately violate wall clearance and verify RedTeamReject
    let base = DesignProgram::baseline_ct35();
    let (mut bad_pheno, mesh) = Phenotype3D::from_program(&base).unwrap();
    bad_pheno.minimum_wall_clearance_mm = 0.85;
    let err = redteam::check(&base, &bad_pheno, &mesh).unwrap_err();
    assert!(err.contains(&RedTeamReject::WallClearanceBelowMinimum));
}

#[test]
fn openfoam_strict_parsers_and_rotation_ode_oracle() {
    use joil_core::openfoam::{
        checked_near_integer_flux, integrate_coupled_rotation_ode, parse_labels,
        parse_surface_field_value_sums, parse_vectors,
    };

    // Valid labelField & vectorField
    let lbl = parse_labels("FoamFile { object origId; }\n3 ( 10 20 30 )\n").unwrap();
    assert_eq!(lbl, vec![10, 20, 30]);
    // Count mismatch rejected
    assert!(parse_labels("3 ( 10 20 )").is_err());

    let vecs = parse_vectors("2 ( (1.0 2.0 3.0) (4.0 5.0 6.0) )").unwrap();
    assert_eq!(vecs.len(), 2);
    // Trailing garbage rejected
    assert!(parse_vectors("1 ( (1.0 2.0 3.0) ) trailing_token").is_err());
    // 4-tuple rejected
    assert!(parse_vectors("1 ( (1.0 2.0 3.0 4.0) )").is_err());

    // Monotonic surfaceFieldValue integration
    let sfv = "# Time cloudA cloudB\n600.001 1000.0 2000.0\n600.002 1000.0 2000.0\n";
    let clouds = vec!["cloudA".to_string(), "cloudB".to_string()];
    let sums = parse_surface_field_value_sums(sfv, &clouds).unwrap();
    assert!(sums["cloudA"] > 0.0);
    // Non-monotonic timestamp rejected
    let sfv_bad = "600.002 100.0\n600.001 100.0\n";
    assert!(parse_surface_field_value_sums(sfv_bad, &["cloudA".to_string()]).is_err());

    // Near-integer flux check
    assert_eq!(checked_near_integer_flux(14.0000001, 1e-4).unwrap().0, 14);
    assert!(checked_near_integer_flux(14.25, 1e-3).is_err());

    // Coupled OIL-03 rotation ODE oracle
    let r20 = integrate_coupled_rotation_ode(20.0, 0.01, 200.0, 2e-6, 0.002);
    let r40 = integrate_coupled_rotation_ode(40.0, 0.01, 200.0, 2e-6, 0.002);
    assert!(r20.radial_displacement_m < 0.0);
    assert!(r40.radial_displacement_m < r20.radial_displacement_m);
    let ratio = (r40.u_r_end_m_s / r40.r_end_m) / (r20.u_r_end_m_s / r20.r_end_m);
    assert!((ratio - 3.850148).abs() < 1e-3);
}

#[test]
fn multi_island_evolution_discovers_feasible_3d_elites_with_null_qualified_claims() {
    use joil_core::evolve::{evaluate_individual, EvolutionConfig};
    use joil_core::program::{DesignProgram, Rng};

    let cfg = EvolutionConfig {
        islands: 2,
        pop_per_island: 8,
        generations: 3,
        mc_samples: 6,
        ..EvolutionConfig::default()
    };
    let (base_ind, _) =
        evaluate_individual("base".into(), DesignProgram::baseline_ct35(), 0, 0, &cfg).unwrap();
    assert!(base_ind.red_team_passed);
    assert!(base_ind.qualified_multiobjective_vector.is_none());
    assert!(base_ind.qualified_capacity_gain.is_none());

    let mut rng = Rng::new(42);
    let seeds = DesignProgram::seed_archetypes(&mut rng);
    let mut feasible_count = 0usize;
    for (idx, p) in seeds.iter().enumerate() {
        let (ind, _) = evaluate_individual(format!("s{idx}"), p.clone(), 0, 0, &cfg).unwrap();
        assert!(ind.red_team_passed);
        assert!(ind.qualified_multiobjective_vector.is_none());
        assert!(ind.qualified_capacity_gain.is_none());
        if ind.constraint_violation == 0.0 && ind.nominal_metrics.passes_30_mg_l {
            feasible_count += 1;
        }
    }
    assert!(feasible_count >= 5);
}

#[test]
fn structural_lame_oracle_and_modal_lpbf_qualification() {
    use joil_core::{
        phenotype::Phenotype3D,
        program::{DesignProgram, Rng},
        structure::{lame_thick_cylinder_oracle, qualify_structure},
    };

    // STRUCTURE-01: Lamé thick-wall cylinder boundary condition identity check
    let a = 0.035_f64;
    let b = 0.038_f64;
    let pi = 0.5e6_f64;
    let po = 2.5e6_f64;
    let s_in = lame_thick_cylinder_oracle(a, b, pi, po, a).unwrap();
    let s_out = lame_thick_cylinder_oracle(a, b, pi, po, b).unwrap();
    assert!((s_in.sigma_r_pa - (-pi)).abs() < 1e-6);
    assert!((s_out.sigma_r_pa - (-po)).abs() < 1e-6);
    assert!(s_in.von_mises_pa > s_out.von_mises_pa);
    // Negative check: invalid radii or out-of-domain r_m rejected
    assert!(lame_thick_cylinder_oracle(0.038, 0.035, pi, po, 0.036).is_err());
    assert!(lame_thick_cylinder_oracle(a, b, pi, po, 0.040).is_err());

    // STRUCTURE-02..05 across all seed 3D archetypes
    let mut rng = Rng::new(20261015);
    for prog in DesignProgram::seed_archetypes(&mut rng) {
        let (pheno, _) = Phenotype3D::from_program(&prog).unwrap();
        let sq = qualify_structure(&prog, &pheno, 29468.22, 0.001111111).unwrap();
        assert!(
            sq.passed,
            "archetype failed structure: {:?}",
            sq.failure_reasons
        );
        assert!(!sq.sour_service_qualified);
        assert!(sq.min_yield_safety_factor >= 2.5);
        assert!(sq.stress_gci_fine_rel <= 0.01);
        assert!(sq.load_transfer_rel_error <= 1e-12);
        assert!(sq.vane_modal_freq_ratio >= 3.0);
        assert!(sq.skimmer_tip_deflection_mm <= 0.010);
        assert!(sq.max_lpbf_overhang_from_vertical_deg <= 45.0);
    }
}

#[test]
fn multi_patch_cfd_surface_watertight_and_terminal_patches_valid() {
    use joil_core::{
        phenotype::MultiPatchSurface,
        program::{DesignProgram, Rng},
    };
    let mut rng = Rng::new(20261015);
    for prog in DesignProgram::seed_archetypes(&mut rng) {
        let surf = MultiPatchSurface::from_program(&prog, 0.0).unwrap();
        assert!(surf.audit.watertight_and_manifold);
        assert!(surf.audit.all_terminal_patches_valid);
        assert_eq!(surf.audit.census.open_boundary_edges, 0);
        assert_eq!(surf.audit.census.non_manifold_edges, 0);
        assert!(surf.audit.signed_volume_m3 > 0.0010);
        for name in ["inlet1", "inlet2", "overflow", "underflow"] {
            let p = &surf.audit.patches[name];
            assert!(p.triangle_count >= 16);
            assert!(p.relative_area_error.unwrap().abs() <= 1e-9);
            assert!(p.min_normal_alignment.unwrap() >= 0.999999);
        }
    }
}

#[test]
fn openfoam_candidate_case_builder_and_surrogate_engine_qualification() {
    use joil_core::{
        cfd_case::{build_openfoam_candidate_case, CfdCaseConfig},
        evolve::{evaluate_individual, EvolutionConfig},
        program::{DesignProgram, Rng},
        surrogate::{evaluate_batch_with_ledger, run_surrogate_and_engine_qualification},
    };

    let tmp_dir = std::env::temp_dir().join(format!("joil-cfd-case-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let mut rng = Rng::new(20261015);
    let archetypes = DesignProgram::seed_archetypes(&mut rng);
    let elite_prog = &archetypes[3]; // trifurcated-6jet-vortex-head
    let cfg = CfdCaseConfig::default();
    let manifest = build_openfoam_candidate_case(elite_prog, &tmp_dir, &cfg).unwrap();
    assert!(manifest.multi_patch_audit.watertight_and_manifold);
    assert!(manifest.multi_patch_audit.all_terminal_patches_valid);
    assert!(manifest.redteam_passed);
    assert!(manifest.structural_qualified);
    assert!(!manifest.qualified_for_oil_grade);
    assert!(tmp_dir.join("input-manifest.json").is_file());
    assert!(tmp_dir.join("constant/triSurface/ct35.stl").is_file());
    // Refuses to overwrite existing case directory
    assert!(build_openfoam_candidate_case(elite_prog, &tmp_dir, &cfg).is_err());
    let _ = std::fs::remove_dir_all(&tmp_dir);

    // ENGINE-03 budget ledger conservation with duplicates and infrastructure quarantine
    let mut batch = archetypes.clone();
    batch.push(archetypes[0].clone()); // duplicate
    let (ledger, fam_counts) = evaluate_batch_with_ledger(&batch, 2).unwrap();
    assert!(ledger.is_conserved());
    assert!(ledger.phenotype_cache_hits >= 1);
    assert_eq!(ledger.infrastructure_quarantined, 2);
    assert!(fam_counts.len() >= 6);

    // ENGINE-02..04 surrogate LOFO-CV and failure quarantine
    let ev_cfg = EvolutionConfig {
        mc_samples: 6,
        ..EvolutionConfig::default()
    };
    let mut individuals = Vec::new();
    for (i, p) in archetypes.iter().enumerate() {
        let (ind, _) = evaluate_individual(format!("a{i}"), p.clone(), 0, 0, &ev_cfg).unwrap();
        individuals.push(ind);
    }
    let sq = run_surrogate_and_engine_qualification(&individuals).unwrap();
    assert!(sq.failure_quarantine_verified);
    assert_eq!(sq.quarantined_failure_records, 2);
    assert!(sq.checkpoint_determinism_verified);
    assert!(sq.evaluator_drift_rejection_verified);
    assert!(sq.budget_conservation_verified);
    assert!(sq.ood_std_exceeds_in_dist_std);
    assert!(sq.family_holdouts.len() >= 6);
}

#[test]
fn p05_population_pbm_dsd_and_multiphase_regime_gates() {
    use joil_core::population::{
        build_stratified_dsd_weights, check_multiphase_regime_envelope,
        generate_mpi_rank_invariant_cohort, verify_added_mass_and_pressure_gradient,
        verify_dispersion_and_les_partition, verify_parcel_substep_halving,
        verify_pbm_breakup_coalescence_and_resolution,
    };

    let oil04 = verify_added_mass_and_pressure_gradient(30.0, 50.0, 2.0e-7, 0.002).unwrap();
    assert!(oil04.passed);
    assert!(oil04.relative_velocity_error < 1e-6);
    assert!(oil04.exact_initial_accel_ratio_ap_over_af > 1.05);
    assert!(oil04.wrong_omitted_pressure_accel_ratio < 0.5);

    let oil05_06 = verify_dispersion_and_les_partition(
        &[0.12, 0.28, 0.54, 0.88, 1.32],
        0.004,
        1.45,
        0.18,
        0.18,
    )
    .unwrap();
    assert!(oil05_06.passed);
    assert!(oil05_06.negative_control_rans_on_les_rejected);
    assert!(verify_dispersion_and_les_partition(
        &[0.12, 0.28, 0.54, 0.88, 1.32],
        0.004,
        1.45,
        0.18,
        1.63,
    )
    .is_err());

    let oil07 = verify_parcel_substep_halving(3.0, 5.0e-5).unwrap();
    assert!(oil07.passed);
    assert!(oil07.single_step_cfl_only_rejected);
    assert!(oil07.fine_rel_error < 5e-4);
    assert!(oil07.observed_order > 1.95);

    let oil08 = build_stratified_dsd_weights(25.0, 0.55, false).unwrap();
    let mass_sum: f64 = oil08.iter().map(|b| b.mass_fraction).sum();
    let num_sum: f64 = oil08.iter().map(|b| b.number_fraction).sum();
    assert!((mass_sum - 1.0).abs() < 1e-12);
    assert!((num_sum - 1.0).abs() < 1e-12);
    assert!(build_stratified_dsd_weights(25.0, 0.55, true).is_err());

    let oil10_12 =
        verify_pbm_breakup_coalescence_and_resolution(180.0, false, 0.003, false).unwrap();
    assert!(oil10_12.passed);
    assert!(oil10_12.breakup_mass_conservation_rel_err <= 1e-13);
    assert!(oil10_12.post_breakup_sauter_d32_um < oil10_12.initial_sauter_d32_um);
    assert!(oil10_12.smoluchowski_coalescence_rel_err < 1e-6);
    assert!(oil10_12.pbm_grid_10_vs_20_vs_40_gci_rel < 0.025);
    assert!(verify_pbm_breakup_coalescence_and_resolution(180.0, true, 0.003, false).is_err());
    assert!(verify_pbm_breakup_coalescence_and_resolution(180.0, false, 0.04, true).is_err());

    let c1 = generate_mpi_rank_invariant_cohort(128, 1).unwrap();
    let c4 = generate_mpi_rank_invariant_cohort(128, 4).unwrap();
    let c8 = generate_mpi_rank_invariant_cohort(128, 8).unwrap();
    assert_eq!(c1, c4);
    assert_eq!(c1, c8);

    let env_ok = check_multiphase_regime_envelope(0.00059, 0.0, 0.0032, 0.024);
    assert!(env_ok.within_dilute_lagrangian_envelope);
    let env_bad = check_multiphase_regime_envelope(0.035, 0.012, 0.22, 0.002);
    assert!(!env_bad.within_dilute_lagrangian_envelope);
    assert_eq!(env_bad.exclusion_reasons.len(), 4);
}

#[test]
fn p06_experiment_calibration_crossover_holdout_ct_and_comparator_gates() {
    use joil_core::experiment::{
        analyze_blinded_aba_crossover, evaluate_as_built_ct_metrology, required_blocks_for_power,
        verify_holdout_isolation, verify_instrument_calibration, verify_shared_duty_comparator,
        AbaCrossoverBlock,
    };

    let cal = verify_instrument_calibration("oil-a", "oil-a", 0.998, 0.4, 1.2, 0.65).unwrap();
    assert!(cal.passed);
    assert!((cal.censored_upper_bound_mg_l - 1.2).abs() < 1e-12);
    assert!(verify_instrument_calibration("oil-a", "oil-b", 0.998, 0.4, 1.2, 10.0).is_err());

    let pwr = required_blocks_for_power(0.025, 0.04).unwrap();
    assert!(pwr >= 5);

    let blocks = vec![
        AbaCrossoverBlock {
            block_id: "B1".into(),
            independent_preparation: true,
            baseline_a1_qmax_m3_h: 4.02,
            candidate_b_qmax_m3_h: 4.65,
            baseline_a2_qmax_m3_h: 4.00,
            split_mismatch_abs: 0.00008,
        },
        AbaCrossoverBlock {
            block_id: "B2".into(),
            independent_preparation: true,
            baseline_a1_qmax_m3_h: 3.98,
            candidate_b_qmax_m3_h: 4.58,
            baseline_a2_qmax_m3_h: 4.01,
            split_mismatch_abs: 0.00011,
        },
        AbaCrossoverBlock {
            block_id: "B3".into(),
            independent_preparation: true,
            baseline_a1_qmax_m3_h: 4.01,
            candidate_b_qmax_m3_h: 4.62,
            baseline_a2_qmax_m3_h: 3.99,
            split_mismatch_abs: 0.00009,
        },
        AbaCrossoverBlock {
            block_id: "B4".into(),
            independent_preparation: true,
            baseline_a1_qmax_m3_h: 4.00,
            candidate_b_qmax_m3_h: 4.60,
            baseline_a2_qmax_m3_h: 4.02,
            split_mismatch_abs: 0.00007,
        },
        AbaCrossoverBlock {
            block_id: "B5".into(),
            independent_preparation: true,
            baseline_a1_qmax_m3_h: 3.99,
            candidate_b_qmax_m3_h: 4.64,
            baseline_a2_qmax_m3_h: 4.01,
            split_mismatch_abs: 0.00010,
        },
        AbaCrossoverBlock {
            block_id: "B6".into(),
            independent_preparation: true,
            baseline_a1_qmax_m3_h: 4.01,
            candidate_b_qmax_m3_h: 4.61,
            baseline_a2_qmax_m3_h: 4.00,
            split_mismatch_abs: 0.00006,
        },
    ];
    let aba = analyze_blinded_aba_crossover(&blocks).unwrap();
    assert!(aba.passed_10pct_lcb95_gate);
    assert!(aba.lcb95_paired_gain_fraction >= 0.10);
    assert!(analyze_blinded_aba_crossover(&blocks[0..3]).is_err());

    assert!(verify_holdout_isolation(&["t1".into()], &["h1".into()], 1).unwrap());
    assert!(verify_holdout_isolation(&["t1".into(), "h1".into()], &["h1".into()], 1).is_err());
    assert!(verify_holdout_isolation(&["t1".into()], &["h1".into()], 2).is_err());

    let ct_ok = evaluate_as_built_ct_metrology("SN-01", 2.0, 2.008, 6.5, 28.0, false).unwrap();
    assert!(ct_ok.passed);
    assert!(ct_ok.ks_nominal_um > ct_ok.measured_ra_um);
    assert!(evaluate_as_built_ct_metrology("SN-01", 2.0, 2.008, 6.5, 28.0, true).is_err());
    assert!(evaluate_as_built_ct_metrology("SN-01", 2.0, 2.045, 6.5, 28.0, false).is_err());

    assert!(verify_shared_duty_comparator("d1", "d1", Some("d1"), true).unwrap());
    assert!(verify_shared_duty_comparator("d1", "d1", None, true).is_err());
    assert!(verify_shared_duty_comparator("d1", "d2", None, false).is_err());
}

#[test]
fn p09_ops_view_resource_lease_and_glb_scene_bundle_gates() {
    use joil_core::{
        phenotype::MultiPatchSurface,
        program::{DesignProgram, Rng},
        scene::{export_glb_scene_bundle, verify_resource_lease},
    };

    let lease_ok = verify_resource_lease("l1", &[0, 1, 2, 3], 4, 16, 8, 1_500_000_000).unwrap();
    assert!(lease_ok.admitted);
    assert_eq!(lease_ok.physical_cores_used, 4);

    // SMT sibling threads [0, 16, 1, 17] on 16-core host only occupy 2 physical cores, not 4
    assert!(verify_resource_lease("l-smt", &[0, 16, 1, 17], 4, 16, 8, 1_500_000_000).is_err());
    // Oversubscribed 6 cores > 4 limit
    assert!(verify_resource_lease("l-over", &[0, 1, 2, 3, 4, 5], 6, 16, 8, 1_500_000_000).is_err());

    let tmp_dir = std::env::temp_dir().join(format!("joil-glb-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let glb_path = tmp_dir.join("scene.glb");
    let mut rng = Rng::new(20261015);
    let archetypes = DesignProgram::seed_archetypes(&mut rng);
    let surf = MultiPatchSurface::from_program(&archetypes[3], 0.0).unwrap();
    let manifest = export_glb_scene_bundle(&surf, "stl-sha-test", 1.0, &glb_path).unwrap();
    assert_eq!(manifest.units, "m");
    assert!((manifest.true_deformation_scale - 1.0).abs() < 1e-12);
    assert!(glb_path.is_file());
    assert!(glb_path.with_extension("scene-manifest.json").is_file());
    let bytes = std::fs::read(&glb_path).unwrap();
    assert_eq!(&bytes[0..4], &0x46546C67u32.to_le_bytes()); // 'glTF' magic
    assert!(export_glb_scene_bundle(&surf, "stl-sha-test", 2.5, &glb_path).is_err());
    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[test]
fn p07_surrogate_constrained_expected_improvement_and_prescreen() {
    use joil_core::{
        evolve::{evaluate_individual, EvolutionConfig},
        program::{DesignProgram, Rng},
        surrogate::{active_prescreen_offspring, GaussianProcessSurrogate, SurrogateRecord},
    };

    let mut rng = Rng::new(20261015);
    let archetypes = DesignProgram::seed_archetypes(&mut rng);
    let ev_cfg = EvolutionConfig {
        mc_samples: 6,
        ..EvolutionConfig::default()
    };
    let mut records = Vec::new();
    for (i, p) in archetypes.iter().enumerate() {
        let (ind, _) = evaluate_individual(format!("s{i}"), p.clone(), 0, 0, &ev_cfg).unwrap();
        records.push(SurrogateRecord::from_candidate(&ind));
    }
    let gp = GaussianProcessSurrogate::fit(&records, 0.85, 0.015).unwrap();
    let mut pool = Vec::new();
    for p in &archetypes {
        pool.push(p.mutate(&mut rng));
    }
    let ranked = active_prescreen_offspring(&gp, &pool, 18.5, 5);
    assert_eq!(ranked.len(), 5);
    assert!(
        ranked[0].1.constrained_expected_improvement
            >= ranked[4].1.constrained_expected_improvement
    );
    assert!(ranked[0].1.feasibility_probability_30_mg_l > 0.0);
}

#[test]
fn wave2_exotic_3d_genome_operators_and_multiseed_ablation_gates() {
    use joil_core::{
        evolve::{evaluate_held_out_duties, run_wave2_multiseed_benchmark, EvolutionConfig},
        phenotype::MultiPatchSurface,
        program::{DesignProgram, Rng, TpmsBasis, TpmsInsertField3D},
        redteam,
        structure::qualify_structure,
    };
    use std::collections::BTreeSet;

    let default_tpms: TpmsInsertField3D = TpmsInsertField3D::default();
    assert!(default_tpms.is_inactive());

    let mut rng = Rng::new(20261101);
    let archetypes = DesignProgram::seed_wave2_archetypes(&mut rng);
    assert!(archetypes.len() >= 19);

    let mut families = BTreeSet::new();
    for prog in &archetypes {
        let (norm, pheno, mesh) = prog.canonicalize_and_preflight().unwrap();
        let fam = norm.classify_topology();
        families.insert(fam.clone());
        let census = mesh.edge_census();
        assert_eq!(census.open_boundary_edges, 0);
        assert_eq!(census.non_manifold_edges, 0);
        assert!(mesh.signed_volume() > 0.0);
        assert_eq!(pheno.frozen_collar_and_tail_diff_mm3, 0.0);
        assert!(pheno.minimum_wall_clearance_mm >= 1.0);
        assert!(pheno.minimum_passage_diameter_mm >= 1.4);
        assert!(redteam::check(&norm, &pheno, &mesh).is_ok());

        let sq = qualify_structure(&norm, &pheno, 29468.22, 0.001111111).unwrap();
        assert!(
            sq.passed,
            "Wave-2 archetype {fam} failed structure: {:?}",
            sq.failure_reasons
        );

        let surf = MultiPatchSurface::from_program(&norm, 0.0).unwrap();
        assert!(surf.audit.watertight_and_manifold);
        assert!(surf.audit.all_terminal_patches_valid);
    }

    // Verify all 11 canonical Wave-2 topology families are represented
    for expected_fam in [
        "baseline-ct35",
        "helical-pitched-ribbon",
        "bifurcated-quad-manifold",
        "trifurcated-6jet-vortex-head",
        "spiral-rifled-vortex-head",
        "contoured-s-cone-skimmer",
        "hybrid-omni-exotic",
        "cylindrical-gyroid-coalescer",
        "schwarz-p-rectifier-head",
        "octa-branch-helical-manifold",
        "counter-fluted-recovery-vortex",
    ] {
        assert!(
            families.contains(expected_fam),
            "missing Wave-2 family {expected_fam}"
        );
    }

    // Verify full 5-patch OpenFOAM 14 case generation + binary glTF 2.0 (.glb) export on all 4 new Wave-2 exotic families
    let tmp_root = std::env::temp_dir().join(format!("joil-wave2-cfd-glb-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_root);
    for exotic_fam in [
        "cylindrical-gyroid-coalescer",
        "schwarz-p-rectifier-head",
        "octa-branch-helical-manifold",
        "counter-fluted-recovery-vortex",
    ] {
        let prog = archetypes
            .iter()
            .find(|p| p.classify_topology() == exotic_fam)
            .expect("Wave-2 exotic family must exist in archetypes");
        let (norm, _, _) = prog.canonicalize_and_preflight().unwrap();
        let case_dir = tmp_root.join(exotic_fam);
        let cfd_manifest = joil_core::cfd_case::build_openfoam_candidate_case(
            &norm,
            &case_dir,
            &joil_core::cfd_case::CfdCaseConfig::default(),
        )
        .unwrap();
        assert!(cfd_manifest.multi_patch_audit.watertight_and_manifold);
        assert!(cfd_manifest.multi_patch_audit.all_terminal_patches_valid);
        assert!(cfd_manifest.redteam_passed);
        assert!(cfd_manifest.structural_qualified);
        let surf = MultiPatchSurface::from_program(&norm, 0.0).unwrap();
        let glb_path = case_dir.join(format!("{exotic_fam}.glb"));
        let glb_manifest = joil_core::scene::export_glb_scene_bundle(
            &surf,
            &cfd_manifest.surface_stl_sha256,
            1.0,
            &glb_path,
        )
        .unwrap();
        assert_eq!(glb_manifest.units, "m");
        assert_eq!(glb_manifest.true_deformation_scale, 1.0);
        assert!(glb_manifest.glb_bytes > 1024);
    }
    let _ = std::fs::remove_dir_all(&tmp_root);

    // Verify Wave-2 operators preserve watertightness, manifoldness, RedTeam checks, and 5-patch OpenFOAM meshability across single and chained mutations
    for i in 0..6 {
        let base_prog = &archetypes[i * 3 % archetypes.len()];
        let m1 = base_prog.mutate_diffeomorphic_spline(&mut rng);
        let m2 = base_prog.mutate_branch_topology(&mut rng);
        let m3 = base_prog.mutate_tpms_grading(&mut rng);
        let mate = &archetypes[(i * 3 + 5) % archetypes.len()];
        let c1 = base_prog.crossover_homologous_graph(mate, &mut rng);
        let chained = m1
            .mutate_branch_topology(&mut rng)
            .mutate_tpms_grading(&mut rng)
            .crossover_homologous_graph(&m2, &mut rng)
            .mutate_diffeomorphic_spline(&mut rng);
        let rand_exotic = DesignProgram::sample_random_exotic(&mut rng);

        for child in [m1, m2, m3, c1, chained, rand_exotic] {
            let (norm_c, pheno, mesh) = child.canonicalize_and_preflight().unwrap();
            let census = mesh.edge_census();
            assert_eq!(census.open_boundary_edges, 0);
            assert_eq!(census.non_manifold_edges, 0);
            assert!(mesh.signed_volume() > 0.0);
            assert!(redteam::check(&norm_c, &pheno, &mesh).is_ok());
            let surf = MultiPatchSurface::from_program(&norm_c, 0.0).unwrap();
            assert!(surf.audit.watertight_and_manifold);
            assert!(surf.audit.all_terminal_patches_valid);
            assert_eq!(surf.audit.census.open_boundary_edges, 0);
            assert_eq!(surf.audit.census.non_manifold_edges, 0);
            assert!(surf.audit.signed_volume_m3 > 0.0);
        }
    }

    // Verify TPMS basis serialization backward compatibility on baseline
    let base_ct35 = DesignProgram::baseline_ct35();
    assert_eq!(base_ct35.tpms_insert.basis, TpmsBasis::None);
    let base_json = serde_json::to_string(&base_ct35).unwrap();
    assert!(!base_json.contains("tpms_insert"));
    assert!(!base_json.contains("ribbon_cascade"));

    // Verify held-out operating duties D1..D4
    let cfg = EvolutionConfig {
        islands: 1,
        pop_per_island: 6,
        generations: 2,
        mc_samples: 4,
        ..EvolutionConfig::default()
    };
    let duties = evaluate_held_out_duties(&archetypes[15], &cfg.baseline_hydraulics).unwrap();
    assert_eq!(duties.len(), 4);
    assert_eq!(duties[0].duty_id, "D1_turndown");
    assert_eq!(duties[1].duty_id, "D2_turnup");
    assert_eq!(duties[2].duty_id, "D3_sheared_feed");
    assert_eq!(duties[3].duty_id, "D4_cold_viscous_slug");

    // Verify 4-arm multi-seed benchmark (3 seeds in debug unit test; full 10 seeds in release CLI)
    let seeds = [20261101, 20261102, 20261103];
    let report = run_wave2_multiseed_benchmark(&seeds, &cfg).unwrap();
    assert!(report
        .baseline_b0_ct35
        .qualified_multiobjective_vector
        .is_none());
    assert!(report.baseline_b0_ct35.qualified_capacity_gain.is_none());
    assert_eq!(report.summary_statistics.len(), 3);
    assert!(report.rejected_cohort_10pct_audit.audit_passed);
    assert_eq!(
        report
            .rejected_cohort_10pct_audit
            .false_omission_rank0_count,
        0
    );
    assert!(
        report.summary_statistics["B3_Exotic3D_QD_CEI"]["pareto_hypervolume"].mean
            > report.summary_statistics["B1_Random_LHS"]["pareto_hypervolume"].mean
    );
    assert!(
        report.summary_statistics["B3_Exotic3D_QD_CEI"]["pareto_hypervolume"].mean
            > report.summary_statistics["B2_Conventional_Axisym"]["pareto_hypervolume"].mean
    );
    assert!(
        report.summary_statistics["B3_Exotic3D_QD_CEI"]["cout_p95_mg_l"].mean
            < report.summary_statistics["B1_Random_LHS"]["cout_p95_mg_l"].mean
    );
    assert!(
        report.summary_statistics["B3_Exotic3D_QD_CEI"]["cout_p95_mg_l"].mean
            < report.summary_statistics["B2_Conventional_Axisym"]["cout_p95_mg_l"].mean
    );

    // Verify 11-family Wave-2 Leave-One-Topology-Family-Out CV (including cylindrical-gyroid-coalescer & octa-branch-helical-manifold)
    let wave2_elites: Vec<_> = report
        .promoted_wave2_elites_by_family
        .values()
        .cloned()
        .collect();
    assert_eq!(wave2_elites.len(), 11);
    let lofo = joil_core::surrogate::run_surrogate_and_engine_qualification(&wave2_elites).unwrap();
    assert!(lofo.failure_quarantine_verified);
    assert!(lofo.ood_std_exceeds_in_dist_std);
    assert_eq!(lofo.family_holdouts.len(), 11);
}

#[test]
fn genome_expressiveness_11family_sampling_and_exotic_3d_gallery_audit() {
    use joil_core::program::{DesignProgram, TpmsBasis};

    let audit = DesignProgram::audit_11family_genome_expressiveness(20, 20261115).unwrap();
    assert_eq!(audit.total_sampled, 220);
    assert_eq!(audit.total_preflight_passed, 220);
    assert_eq!(audit.overall_preflight_pass_fraction, 1.0);
    assert_eq!(audit.family_stats.len(), 11);
    for fam in &audit.family_stats {
        assert_eq!(fam.sampled_count, 20);
        assert_eq!(fam.preflight_passed_count, 20);
        assert_eq!(fam.preflight_pass_fraction, 1.0);
        assert!(fam.min_wall_clearance_mm >= 1.0);
        assert!(fam.min_section_jacobian >= 0.90);
    }

    assert_eq!(audit.top_8_exotic_valid_phenotypes.len(), 8);
    let top_fams: std::collections::BTreeSet<_> = audit
        .top_8_exotic_valid_phenotypes
        .iter()
        .map(|p| p.topology_class.as_str())
        .collect();
    assert_eq!(top_fams.len(), 8);
    assert!(top_fams.contains("cylindrical-gyroid-coalescer"));
    assert!(top_fams.contains("schwarz-p-rectifier-head"));
    assert!(top_fams.contains("octa-branch-helical-manifold"));
    assert!(top_fams.contains("counter-fluted-recovery-vortex"));
    assert!(audit.top_8_exotic_valid_phenotypes[0].exoticness_index > 1.20);

    // Negative check: unprojected out-of-bounds parameter perturbations must fail canonicalize_and_preflight
    let mut bad1 = DesignProgram::baseline_ct35();
    bad1.tpms_insert.basis = TpmsBasis::CylindricalGyroid;
    bad1.tpms_insert.radial_inner_fraction = 0.35; // encroaches on central air/oil core
    assert!(bad1.canonicalize_and_preflight().is_err());

    let mut bad2 = DesignProgram::baseline_ct35();
    bad2.inlet_split.enabled = true;
    bad2.inlet_split.vane_thickness_mm = 0.45; // below LPBF 1.0 mm wall minimum
    assert!(bad2.canonicalize_and_preflight().is_err());

    let mut bad3 = DesignProgram::baseline_ct35();
    bad3.inlet_ramp.axial_drop_mm = 18.0; // violates ramp pitch / overhang limit
    assert!(bad3.canonicalize_and_preflight().is_err());
}

#[test]
fn operator_random_walk_20step_11family_watertight_stress_test() {
    use joil_core::program::DesignProgram;

    assert!(DesignProgram::audit_11family_operator_random_walk(19, 20261116).is_err());
    let walk = DesignProgram::audit_11family_operator_random_walk(20, 20261116).unwrap();
    println!(
        "OPERATOR_RANDOM_WALK_SUMMARY: total_steps={} total_passed={} pass_fraction={} transitions={} global_families={} min_clearance_mm={:.4}",
        walk.total_steps,
        walk.total_passed,
        walk.overall_pass_fraction,
        walk.total_family_transitions,
        walk.global_distinct_families_visited,
        walk.family_walk_stats
            .iter()
            .map(|f| f.min_wall_clearance_mm)
            .fold(f64::INFINITY, f64::min),
    );
    assert_eq!(walk.steps_per_family, 20);
    assert_eq!(walk.total_steps, 220);
    assert_eq!(walk.total_passed, 220);
    assert_eq!(walk.overall_pass_fraction, 1.0);
    assert_eq!(walk.family_walk_stats.len(), 11);
    assert!(walk.total_family_transitions >= 50);
    assert!(walk.global_distinct_families_visited >= 8);

    for fam in &walk.family_walk_stats {
        assert_eq!(fam.steps_executed, 20);
        assert_eq!(fam.steps_passed, 20);
        assert_eq!(fam.pass_fraction, 1.0);
        assert_eq!(fam.max_open_edges, 0);
        assert_eq!(fam.max_non_manifold_edges, 0);
        assert!(fam.min_wall_clearance_mm >= 1.0);
        assert!(fam.min_section_jacobian >= 0.90);
        assert!(fam.family_transitions >= 1);
    }
}
