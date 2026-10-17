// Synthetic software probes of the frozen legacy source. No CFD or search.
use joil_core::{cfd_case, evolve, experiment, geometry, phenotype, population, program, scene, structure, surrogate};
use joil_ir::{BackendId, IrCompiledMesh, VoxelSdfObserver};

fn main() {
    let p = program::DesignProgram::baseline_ct35();
    let d = p.descriptors().unwrap();
    let h = evolve::BaselineHydraulics {
        q_in_m3_s: 4.0 / 3600.0, q_over_m3_s: 0.08 / 3600.0,
        q_under_m3_s: 3.92 / 3600.0, split_ratio: 0.02, mass_imbalance_rel: 0.0,
        dp_u_pa: 30_000.0, dp_o_pa: 150_000.0, pdr: 5.0,
        specific_hydraulic_loss_j_m3: 0.0, underflow_reverse_m3_s: 0.0, overflow_reverse_m3_s: 0.0,
    };
    let a = evolve::evaluate_realization(&p, &d, &h, 500.0, 22.0, 0.0, 0.0);
    let mut changed = d.clone();
    changed.hydraulic_resistance_factor *= 1.2;
    let b = evolve::evaluate_realization(&p, &changed, &h, 500.0, 22.0, 0.0, 0.0);
    assert_ne!(a.dp_u_pa, b.dp_u_pa);
    println!("D-01 production evaluation depends on caller descriptors for unchanged program: dp {} -> {} Pa", a.dp_u_pa, b.dp_u_pa);
    let per_feed = (1.0 - h.split_ratio) * a.dp_u_pa + h.split_ratio * a.dp_o_pa;
    assert!((a.specific_energy_j_m3 - per_feed).abs() < 1e-9);
    assert!((a.specific_energy_j_m3 - per_feed / (1.0 - h.split_ratio)).abs() > 1.0);
    println!("D-11 feed-volume proxy={} versus synthetic treated-volume pressure-work={}; neither establishes mechanical flux", a.specific_energy_j_m3, per_feed / 0.98);

    let mut tetra = IrCompiledMesh {
        ir_sha256: "synthetic-only".into(), backend: BackendId::RustMultiPatchCfdSurface,
        units: "m".into(), vertices_m: vec![[0.0,0.0,1.0], [0.01,0.0,1.0], [0.0,0.01,1.0], [0.01,0.01,1.01]],
        triangles: vec![[0,2,1], [0,1,3], [1,2,3], [2,0,3]],
        triangle_origin_ids: vec![0;4], origin_table: vec![], internal_feature_triangles: vec![],
    };
    let t = VoxelSdfObserver::derive_taxon_from_mesh(&tetra, 1, 0);
    assert_eq!(t.measured_skimmer_protrusion_mm, 1000.0);
    // This synthetic tetrahedron has no skimmer; only its cap-like axial point is near the axis.
    tetra.vertices_m[0][0] = 0.01;
    let control = VoxelSdfObserver::derive_taxon_from_mesh(&tetra, 1, 0);
    assert_eq!(control.measured_skimmer_protrusion_mm, 0.0);
    println!("D-04 cap-like point reports false 1000mm skimmer; moving it off-axis yields zero");
    println!("D-26 closed tetrahedron taxon: {:?}; volume ball oracle is (1,0,0), boundary sphere is (1,0,1)", t);

    let ir = geometry::compile_design_ir(&p).unwrap();
    println!("D-15 baseline IR constants are source-bound; values inspected separately against frozen genome");
    let mesh = IrCompiledMesh::compile(&ir, BackendId::RustMultiPatchCfdSurface).unwrap();
    let observed = VoxelSdfObserver::observe(&ir, &mesh);
    assert_eq!(observed.trapped_powder_cavities, 0);
    println!("D-05 baseline powder result=0; source probe establishes fixed assignment; no cavity-detection validation claimed");

    let profile = [0.0, 100.0, -40.0, 3.0];
    let dispersion = population::verify_dispersion_and_les_partition(&profile, 1.0, 10.0, 1.0, 1.0).unwrap();
    assert!(dispersion.passed); // Nonphysical negative covariance still passes the telescoping helper.
    assert!(population::verify_dispersion_and_les_partition(&profile, 1.0, 10.0, 1.0, 2.0).is_err());
    println!("D-13 negative covariance profile passes drift helper; excess LES energy control is rejected");

    let blocks: Vec<_> = [1.05,1.10,1.15,1.20,1.25].into_iter().enumerate().map(|(i,q)| experiment::AbaCrossoverBlock {
        block_id: format!("synthetic-{i}"), independent_preparation: true,
        baseline_a1_qmax_m3_h: 1.0, baseline_a2_qmax_m3_h: 1.0, candidate_b_qmax_m3_h: q, split_mismatch_abs: 0.0,
    }).collect();
    let trial = experiment::analyze_blinded_aba_crossover(&blocks).unwrap();
    let implied_t = (trial.mean_paired_gain_fraction - trial.lcb95_paired_gain_fraction) * 5.0_f64.sqrt() / trial.paired_sd_gain_fraction;
    assert!((implied_t - 2.72746398).abs() < 1e-10);
    // Independent exact df=4 inverse-CDF value, from integrating the normalized t4 density.
    let exact_t4 = 2.7764451051977987;
    assert!(implied_t < exact_t4);
    let mut invalid_blocks = blocks;
    invalid_blocks[0].independent_preparation = false;
    assert!(experiment::analyze_blinded_aba_crossover(&invalid_blocks).is_err());
    println!("D-12 implied t={} below df4 0.975 oracle {}; nonindependent preparation rejected", implied_t, exact_t4);

    let checkpoint = surrogate::CampaignCheckpoint {
        schema_version:1, evaluator_sha256:"literal".into(), cuda_binary_sha256:"literal".into(),
        generation:10, rng_states:vec![], islands:vec![], seen_phenotype_hashes:vec![], ledger:Default::default(),
    };
    assert!(checkpoint.verify_resume_compatibility(1, "literal", "literal").is_ok());
    assert!(checkpoint.verify_resume_compatibility(1, "changed", "literal").is_err());
    println!("D-09 empty-state compatibility succeeds, identity drift rejects; no restart determinism established");

    assert!(scene::verify_resource_lease("synthetic", &[0,1], 2, 8, 1, 0).is_ok());
    assert!(scene::verify_resource_lease("synthetic", &[0,8], 2, 8, 1, 0).is_err());
    println!("D-19 synthetic adjacent-SMT topology with 0/1 as siblings is incorrectly accepted as two cores; old offset-SMT control rejects");

    let surface = phenotype::MultiPatchSurface::from_program(&p, 0.12).unwrap();
    let out = std::env::args().nth(1).expect("private output path");
    assert!(scene::export_glb_scene_bundle(&surface, "not-a-digest", 1.0, std::path::Path::new(&out)).is_ok());
    assert!(scene::export_glb_scene_bundle(&surface, "", 1.0, std::path::Path::new(&out)).is_err());
    println!("D-20 arbitrary nonempty governing digest accepted; empty digest rejected; output is private synthetic probe");

    let (pheno, _) = phenotype::Phenotype3D::from_program(&p).unwrap();
    let s1 = structure::analytic_structural_screen(&p, &pheno, 30_000.0, 4.0/3600.0).unwrap();
    let s2 = structure::analytic_structural_screen(&p, &pheno, 60_000.0, 4.0/3600.0).unwrap();
    assert_eq!(s1.design_vessel_delta_p_pa, 2.5e6);
    assert_eq!(s1.max_wall_von_mises_mpa, s2.max_wall_von_mises_mpa);
    assert!(structure::analytic_structural_screen(&p, &pheno, -1.0, 4.0/3600.0).is_err());
    println!("D-08 wall screen keeps 2.5MPa external demand under changed hydraulic dp; negative hydraulic load rejects. Fixed external demand alone is not a defect; source authority is unresolved.");

    let case = std::env::args().nth(2).expect("private fresh case path");
    let config = cfd_case::CfdCaseConfig { n_subdomains:2, ..Default::default() };
    cfd_case::build_openfoam_candidate_case(&p, std::path::Path::new(&case), &config).unwrap();
    let schemes = std::fs::read_to_string(std::path::Path::new(&case).join("system/fvSchemes")).unwrap();
    let meshing = std::fs::read_to_string(std::path::Path::new(&case).join("system/snappyHexMeshDict")).unwrap();
    assert!(schemes.contains("steadyState") && schemes.contains("bounded Gauss upwind"));
    assert!(meshing.contains("addLayers false"));
    assert!(cfd_case::build_openfoam_candidate_case(&p, std::path::Path::new(&case), &config).is_err());
    println!("D-02 actual generator produces steady upwind/no-layer dictionaries; existing-directory negative rejects. No solver launched and no vortex-causation inference.");
}
