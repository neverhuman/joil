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
    let gp = GaussianProcessSurrogate::fit_sandbox_prior(&records, 0.85, 0.015).unwrap();
    let mut pool = Vec::new();
    for p in &archetypes {
        pool.push(p.mutate(&mut rng).unwrap().into_child().unwrap());
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
        structure::analytic_structural_screen,
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

        let sq = analytic_structural_screen(&norm, &pheno, 29468.22, 0.001111111).unwrap();
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
        let m1 = base_prog
            .mutate_diffeomorphic_spline(&mut rng)
            .unwrap()
            .into_child()
            .unwrap();
        let m2 = base_prog
            .mutate_branch_topology(&mut rng)
            .unwrap()
            .into_child()
            .unwrap();
        let m3 = base_prog
            .mutate_tpms_grading(&mut rng)
            .unwrap()
            .into_child()
            .unwrap();
        let mate = &archetypes[(i * 3 + 5) % archetypes.len()];
        let c1 = base_prog
            .crossover_non_homologous_macro(mate, &mut rng)
            .unwrap()
            .into_child()
            .unwrap();
        let chained = m1
            .mutate_branch_topology(&mut rng)
            .unwrap()
            .into_child()
            .unwrap()
            .mutate_tpms_grading(&mut rng)
            .unwrap()
            .into_child()
            .unwrap()
            .crossover_non_homologous_macro(&m2, &mut rng)
            .unwrap()
            .into_child()
            .unwrap()
            .mutate_diffeomorphic_spline(&mut rng)
            .unwrap()
            .into_child()
            .unwrap();
        let rand_exotic = DesignProgram::sample_random_exotic(&mut rng)
            .unwrap()
            .into_child()
            .unwrap();

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
    // Software checks must preserve negative scientific outcomes. No algorithm
    // is required to beat another on a chosen three-seed synthetic screen.
    for outcome in report
        .arm_b1_random_lhs_outcomes
        .iter()
        .chain(&report.arm_b2_conventional_axisym_outcomes)
        .chain(&report.arm_b3_exotic3d_qd_cei_outcomes)
    {
        let budget = &outcome.proposal_budget;
        assert_eq!(
            budget.budget,
            cfg.islands * cfg.pop_per_island * (cfg.generations + 1)
        );
        assert_eq!(
            budget.budget,
            budget.proposals_invalid + budget.proposals_repaired + budget.evaluations
        );
        assert_eq!(
            outcome.total_evaluations,
            budget.proposals_repaired + budget.evaluations
        );
        assert!(outcome.best_cout_p95_mg_l.is_finite());
        assert!(outcome.pareto_hypervolume.is_finite());
    }

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
