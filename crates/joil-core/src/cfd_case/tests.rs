//! Unit and negative-control tests for OpenFOAM 14 candidate case generation and EV-12 case identity.

use super::*;

use crate::program::{DesignProgram, InletSplit3D};

#[test]
fn ir_r1_openfoam_cfd_stl_is_ir_compiled_mesh_and_slices_to_3_inlet_loops_per_duct() {
    let mut prog = DesignProgram::baseline_ct35();
    prog.inlet_split = InletSplit3D {
        enabled: true,
        vane_thickness_mm: 1.2,
        vane_z_fraction: 0.5,
        branches_per_port: 3,
        azimuthal_phase_deg: 12.0,
    };
    prog.skimmer.length_mm = 5.5;
    prog.skimmer.outer_radius_mm = 2.5;
    prog.skimmer.capture_flutes = 4;

    let tmp_dir =
        std::env::temp_dir().join(format!("joil_ir_r1_cfd_case_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);

    let cfg = CfdCaseConfig::default();
    let manifest = build_openfoam_candidate_case(&prog, &tmp_dir, &cfg)
        .expect("3-branch 6jet OpenFOAM case generation must succeed");

    let receipt = manifest
        .enactment_receipt
        .as_ref()
        .expect("enactment_receipt must be present");
    assert!(receipt.all_active_features_enacted);
    assert_eq!(receipt.measured_disjoint_inlet_paths, 6);
    assert_eq!(receipt.expected_disjoint_inlet_paths, 6);

    let taxon = manifest
        .derived_taxon
        .as_ref()
        .expect("derived_taxon must be present");
    assert_eq!(taxon.measured_inlet_channel_count, 6);
    assert_eq!(taxon.betti_1, 4);

    let stl_path = tmp_dir.join(&manifest.surface_stl_relpath);
    let stl_text = fs::read_to_string(&stl_path).expect("read constant/triSurface/ct35.stl");
    let (total_loops, loops_inlet1, loops_inlet2) =
        joil_ir::IrCompiledMesh::audit_ascii_stl_inlet_enactment(&stl_text);
    assert_eq!(loops_inlet1, 3, "inlet1 duct must slice to 3 closed loops");
    assert_eq!(loops_inlet2, 3, "inlet2 duct must slice to 3 closed loops");
    assert_eq!(total_loops, 6, "total disjoint inlet loops must equal 6");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn int06_wave4_top_winners_cfd_stl_equals_ir_compiled_mesh_and_slice_loops() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wave4_path = manifest_dir.join("../../docs/results/wave4-evolution-proof-v1.json");
    let wave4_val: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&wave4_path).expect("read wave4 json"))
            .expect("parse wave4 json");

    let cases = [
        (
            "wave4_top_elite_s20261108_g016_i000_8jet",
            &wave4_val["promoted_wave3_top_elite"]["program"],
            4usize,
        ),
        (
            "wave4_trifurcated_6jet_vortex_head",
            &wave4_val["promoted_wave3_elites_by_family"]["trifurcated-6jet-vortex-head"]
                ["program"],
            3usize,
        ),
        (
            "wave4_counter_fluted_recovery_vortex_6jet",
            &wave4_val["promoted_wave3_elites_by_family"]["counter-fluted-recovery-vortex"]
                ["program"],
            3usize,
        ),
        (
            "wave4_baseline_ct35_control_2jet",
            &wave4_val["baseline_b0_ct35"]["program"],
            1usize,
        ),
    ];

    let mut case_records = Vec::new();
    for (label, prog_val, expected_loops_per_duct) in cases {
        let prog: DesignProgram =
            serde_json::from_value(prog_val.clone()).expect("deserialize DesignProgram");
        let p_norm = prog.normalize().expect("normalize DesignProgram");

        let ir = crate::geometry::compile_design_ir(&p_norm).expect("compile DesignIrV1");
        let ir_mesh = joil_ir::IrCompiledMesh::compile_with_extension(
            &ir,
            joil_ir::BackendId::RustMultiPatchCfdSurface,
            0.0,
        )
        .expect("compile IrCompiledMesh");

        let tmp_dir =
            std::env::temp_dir().join(format!("joil_int06_{label}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp_dir);

        let cfg = CfdCaseConfig::default();
        let manifest = build_openfoam_candidate_case(&p_norm, &tmp_dir, &cfg)
            .expect("build_openfoam_candidate_case must succeed");

        // 1. Verify MultiPatchSurface::from_program writes the exact same STL mesh as IrCompiledMesh::compile_with_extension
        let direct_surface = MultiPatchSurface::from_program(&p_norm, 0.0).unwrap();
        assert_eq!(
            direct_surface.vertices_m, ir_mesh.vertices_m,
            "{label}: MultiPatchSurface vertices must equal IrCompiledMesh::compile_with_extension"
        );
        assert_eq!(
                direct_surface.triangles, ir_mesh.triangles,
                "{label}: MultiPatchSurface triangles must equal IrCompiledMesh::compile_with_extension"
            );

        // 2. Verify slice loops at x = +-0.045 m and x = +-0.050 m on both IrCompiledMesh and emitted OpenFOAM STL
        let loops_neg_045 = ir_mesh.count_cross_section_loops_at_x(-0.045);
        let loops_pos_045 = ir_mesh.count_cross_section_loops_at_x(0.045);
        let loops_neg_050 = ir_mesh.count_cross_section_loops_at_x(-0.050);
        let loops_pos_050 = ir_mesh.count_cross_section_loops_at_x(0.050);
        assert_eq!(
            loops_neg_045, expected_loops_per_duct,
            "{label}: x = -0.045 m must have {expected_loops_per_duct} closed loops"
        );
        assert_eq!(
            loops_pos_045, expected_loops_per_duct,
            "{label}: x = +0.045 m must have {expected_loops_per_duct} closed loops"
        );
        assert_eq!(
            loops_neg_050, expected_loops_per_duct,
            "{label}: x = -0.050 m must have {expected_loops_per_duct} closed loops"
        );
        assert_eq!(
            loops_pos_050, expected_loops_per_duct,
            "{label}: x = +0.050 m must have {expected_loops_per_duct} closed loops"
        );

        let stl_path = tmp_dir.join(&manifest.surface_stl_relpath);
        let stl_text = fs::read_to_string(&stl_path).expect("read ct35.stl");
        let (total_loops, loops_in1, loops_in2) =
            joil_ir::IrCompiledMesh::audit_ascii_stl_inlet_enactment(&stl_text);
        assert_eq!(loops_in1, expected_loops_per_duct);
        assert_eq!(loops_in2, expected_loops_per_duct);
        assert_eq!(total_loops, 2 * expected_loops_per_duct);

        let taxon = manifest.derived_taxon.as_ref().unwrap();
        case_records.push(serde_json::json!({
            "label": label,
            "topology_class": manifest.topology_class,
            "design_sha256": manifest.design_sha256,
            "design_ir_sha256": manifest.design_ir_sha256,
            "surface_stl_sha256": manifest.surface_stl_sha256,
            "surface_stl_bytes": manifest.surface_stl_bytes,
            "vertex_count": ir_mesh.vertices_m.len(),
            "triangle_count": ir_mesh.triangles.len(),
            "equals_ir_compiled_mesh_vertices": true,
            "equals_ir_compiled_mesh_triangles": true,
            "watertight_and_manifold": manifest.multi_patch_audit.watertight_and_manifold,
            "slice_loops_x_neg_0_045_m": loops_neg_045,
            "slice_loops_x_pos_0_045_m": loops_pos_045,
            "slice_loops_x_neg_0_050_m": loops_neg_050,
            "slice_loops_x_pos_0_050_m": loops_pos_050,
            "stl_audit_inlet1_loops": loops_in1,
            "stl_audit_inlet2_loops": loops_in2,
            "stl_audit_total_inlet_loops": total_loops,
            "expected_loops_per_duct": expected_loops_per_duct,
            "betti": [taxon.betti_0, taxon.betti_1, taxon.betti_2],
            "taxon_signature": taxon.taxon_signature,
            "derived_cluster": taxon.derived_cluster,
        }));

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    let checks_dir = manifest_dir.join("../../.private/checks");
    let _ = fs::create_dir_all(&checks_dir);
    let receipt_path = checks_dir.join("int06-wave4-winner-cfd-stl-ir-parity-20261016.json");
    let receipt_doc = serde_json::json!({
        "schema": "joil.ir.int06_wave4_cfd_stl_parity_receipt.v1",
        "source_wave4_receipt": "docs/results/wave4-evolution-proof-v1.json",
        "generator_test": "joil_core::cfd_case::tests::int06_wave4_top_winners_cfd_stl_equals_ir_compiled_mesh_and_slice_loops",
        "cases": case_records,
    });
    let _ = fs::write(
        &receipt_path,
        format!("{}\n", serde_json::to_string_pretty(&receipt_doc).unwrap()),
    );
}

#[test]
fn ev12_prepare_finalize_after_mesh_and_rsm_edits_and_refuse_polymesh_prune_until_archive_verified()
{
    let mut prog = DesignProgram::baseline_ct35();
    prog.inlet_split = InletSplit3D {
        enabled: true,
        vane_thickness_mm: 1.2,
        vane_z_fraction: 0.5,
        branches_per_port: 3,
        azimuthal_phase_deg: 10.0,
    };
    let p_norm = prog.normalize().expect("normalize");

    let root_tmp = std::env::temp_dir().join(format!("joil_ev12_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root_tmp);
    fs::create_dir_all(&root_tmp).expect("create root_tmp");

    let case_dir = root_tmp.join("wave4-synthetic-rsm-continuation");
    let archive_dir = root_tmp.join("wave4-synthetic-rsm-continuation-archive");
    let cfg = CfdCaseConfig::default();

    // 1. Prepare candidate case via build_openfoam_candidate_case (prepare-cfd-case)
    let _manifest = build_openfoam_candidate_case(&p_norm, &case_dir, &cfg).expect("prepare case");
    assert!(case_dir.join("ev12-capture-workflow.sh").is_file());

    // 2. Finalizing BEFORE constant/polyMesh exists must fail closed
    assert!(
        finalize_wave4_case_identity(&case_dir).is_err(),
        "finalizing before volume mesh exists must fail closed"
    );

    // 3. Populate synthetic constant/polyMesh (points, faces, owner, neighbour, boundary + cellZones)
    //    and apply post-prepare dictionary + 0/ + RSM edits BEFORE solver start
    let poly_dir = case_dir.join("constant/polyMesh");
    fs::create_dir_all(&poly_dir).expect("create polyMesh");
    for name in [
        "points",
        "faces",
        "owner",
        "neighbour",
        "boundary",
        "cellZones",
    ] {
        fs::write(
            poly_dir.join(name),
            format!("synthetic polyMesh {name} payload\n").as_bytes(),
        )
        .expect("write polyMesh file");
    }
    // Dictionary edit (e.g., G1 grid sizing in system/blockMeshDict)
    let bm_path = case_dir.join("system/blockMeshDict");
    let bm_orig = fs::read_to_string(&bm_path).expect("read blockMeshDict");
    fs::write(&bm_path, format!("{bm_orig}\n// synthetic G1 grid edit\n"))
        .expect("write edited blockMeshDict");
    // 0/ and RSM field conversion edits (0/U, 0/R, 0/epsilon)
    for field in ["0/U", "0/R", "0/epsilon"] {
        let fpath = case_dir.join(field);
        let forig = fs::read_to_string(&fpath).expect("read 0/ field");
        fs::write(&fpath, format!("{forig}\n// synthetic RSM field edit\n"))
            .expect("write edited 0/ field");
    }

    // 4. Finalize case-identity.json (joil.wave4-case-identity.v1) after the last edit and before solver start
    let identity = finalize_wave4_case_identity(&case_dir)
        .expect("finalize_wave4_case_identity must succeed after mesh/0/dictionary/RSM edits");
    assert_eq!(identity.schema, WAVE4_CASE_IDENTITY_SCHEMA);
    assert!(case_dir.join(WAVE4_CASE_IDENTITY_FILE).is_file());
    assert!(identity
        .mesh_files
        .contains_key("constant/polyMesh/cellZones"));

    // Identical retry must be allowed and return the exact same identity
    let retry_identity =
        finalize_wave4_case_identity(&case_dir).expect("identical retry must succeed");
    assert_eq!(retry_identity, identity);

    let ver = verify_wave4_case_identity(&case_dir).expect("verify_wave4_case_identity");
    assert_eq!(ver.state, "exact_artifact_bytes_verified");
    assert_eq!(ver.case_hash, identity.case_hash);
    assert_eq!(ver.mesh_hash, identity.mesh_hash);

    // Post-finalization edits to mesh, 0/, or dictionary must be rejected by both verify and finalize
    for rel in ["constant/polyMesh/points", "0/U", "system/fvSchemes"] {
        let p = case_dir.join(rel);
        let orig = fs::read(&p).expect("read original");
        let mut mutated = orig.clone();
        mutated.extend_from_slice(b" tampered");
        fs::write(&p, &mutated).expect("write tampered");
        assert!(
            verify_wave4_case_identity(&case_dir).is_err(),
            "verify must reject tampered {rel}"
        );
        assert!(
            finalize_wave4_case_identity(&case_dir).is_err(),
            "finalize must refuse to overwrite stale sidecar after tampering {rel}"
        );
        fs::write(&p, &orig).expect("restore original");
    }

    // 5. Refuse to prune constant/polyMesh until the archive is verified
    assert!(
        prune_polymesh_after_verified_archive(&case_dir, &archive_dir).is_err(),
        "must refuse to prune polyMesh when archive_dir does not exist yet"
    );
    assert!(
        case_dir.join("constant/polyMesh/points").is_file(),
        "polyMesh must remain intact after refused prune"
    );

    // Archive and verify restored copy in archive_dir
    let arch_receipt = archive_and_verify_wave4_case_inputs(&case_dir, &archive_dir)
        .expect("archive_and_verify_wave4_case_inputs must succeed");
    assert_eq!(
        arch_receipt.restored_verification.state,
        "exact_artifact_bytes_verified"
    );
    assert_eq!(arch_receipt.case_hash, identity.case_hash);
    assert_eq!(arch_receipt.mesh_hash, identity.mesh_hash);

    // Corrupting the archive copy must cause prune_polymesh_after_verified_archive to refuse pruning
    let arch_points = archive_dir.join("constant/polyMesh/points");
    let arch_points_orig = fs::read(&arch_points).expect("read arch points");
    fs::write(&arch_points, b"corrupted archive mesh").expect("corrupt arch points");
    assert!(
        prune_polymesh_after_verified_archive(&case_dir, &archive_dir).is_err(),
        "must refuse to prune polyMesh when archive copy fails verification"
    );
    assert!(
        case_dir.join("constant/polyMesh/points").is_file(),
        "polyMesh must remain intact when archive verification fails"
    );
    fs::write(&arch_points, &arch_points_orig).expect("restore arch points");

    // Now that archive is verified, prune_polymesh_after_verified_archive succeeds and removes case_dir/constant/polyMesh
    let prune_receipt = prune_polymesh_after_verified_archive(&case_dir, &archive_dir)
        .expect("prune_polymesh_after_verified_archive must succeed after verified archive");
    assert_eq!(prune_receipt.case_hash, identity.case_hash);
    // EV12-WR-01: prune receipt must report exact deduplicated archive inventory including case-identity.json
    assert_eq!(
        prune_receipt.file_count, arch_receipt.file_count,
        "EV12-WR-01: prune receipt file_count must equal archive receipt file_count"
    );
    assert_eq!(
        prune_receipt.total_bytes, arch_receipt.total_bytes,
        "EV12-WR-01: prune receipt total_bytes must equal archive receipt total_bytes"
    );
    assert!(!case_dir.join("constant/polyMesh").exists());

    // The retained archive copy still passes full EV-12 verification
    let arch_ver_after = verify_wave4_case_identity(&archive_dir)
        .expect("retained archive copy must remain verifiable after source polyMesh prune");
    assert_eq!(arch_ver_after.state, "exact_artifact_bytes_verified");

    let _ = fs::remove_dir_all(&root_tmp);
}

#[test]
fn ev12_wr01_to_wr05_regression_suite() {
    let mut prog = DesignProgram::baseline_ct35();
    prog.inlet_split = InletSplit3D {
        enabled: true,
        vane_thickness_mm: 1.2,
        vane_z_fraction: 0.5,
        branches_per_port: 3,
        azimuthal_phase_deg: 10.0,
    };
    let p_norm = prog.normalize().expect("normalize");
    let cfg = CfdCaseConfig::default();

    let root_tmp = std::env::temp_dir().join(format!("joil_ev12_wr_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root_tmp);
    fs::create_dir_all(&root_tmp).expect("create root_tmp");

    let populate_synthetic_polymesh = |case_dir: &Path| {
        let poly_dir = case_dir.join("constant/polyMesh");
        fs::create_dir_all(&poly_dir).expect("create polyMesh");
        for name in [
            "points",
            "faces",
            "owner",
            "neighbour",
            "boundary",
            "cellZones",
        ] {
            fs::write(
                poly_dir.join(name),
                format!("synthetic polyMesh {name} payload\n").as_bytes(),
            )
            .expect("write polyMesh file");
        }
    };

    // --- EV12-WR-03: build_wave4_case_identity and finalize_wave4_case_identity must reject forged canonical_genome on non-DesignProgram JSON ---
    {
        let forged_case = root_tmp.join("wr03-forged-canonical");
        build_openfoam_candidate_case(&p_norm, &forged_case, &cfg).expect("prepare forged_case");
        populate_synthetic_polymesh(&forged_case);

        let prog_path = forged_case.join("design-program.json");
        fs::write(
            &prog_path,
            b"{\n  \"synthetic\": true,\n  \"z\": 2,\n  \"a\": 0\n}\n",
        )
        .expect("write synthetic non-DesignProgram JSON");
        let prog_bytes = fs::read(&prog_path).expect("read prog_path");
        let prog_sha = joil_ir::sha256_hex(&prog_bytes);
        let ordered_prog: OrderedJson =
            serde_json::from_slice(&prog_bytes).expect("parse OrderedJson");
        let valid_canonical_genome =
            joil_ir::sha256_hex(&serde_json::to_vec(&ordered_prog).expect("serialize OrderedJson"));
        let sorted_prog: serde_json::Value =
            serde_json::from_slice(&prog_bytes).expect("parse Value");
        let valid_ir_source_genome =
            joil_ir::sha256_hex(&serde_json::to_vec(&sorted_prog).expect("serialize Value"));

        let ir_path = forged_case.join("design-ir.json");
        let mut ir_doc: serde_json::Value =
            serde_json::from_slice(&fs::read(&ir_path).expect("read ir")).expect("parse ir");
        ir_doc["program_sha256"] = serde_json::Value::String(valid_ir_source_genome);
        let ir_bytes = format!("{}\n", serde_json::to_string_pretty(&ir_doc).unwrap()).into_bytes();
        let ir_file_sha = joil_ir::sha256_hex(&ir_bytes);
        fs::write(&ir_path, &ir_bytes).expect("write updated design-ir.json");

        let im_path = forged_case.join("input-manifest.json");
        let mut im: serde_json::Value =
            serde_json::from_slice(&fs::read(&im_path).expect("read im")).expect("parse im");
        im["design_sha256"] = serde_json::Value::String("f".repeat(64));
        im["subject_bundle"]["genome"]["sha256"] = serde_json::Value::String(prog_sha);
        im["subject_bundle"]["ir"]["sha256"] = serde_json::Value::String(ir_file_sha);
        fs::write(
            &im_path,
            format!("{}\n", serde_json::to_string_pretty(&im).unwrap()),
        )
        .expect("write forged im");

        let err = build_wave4_case_identity(&forged_case)
                .expect_err("EV12-WR-03: build_wave4_case_identity must reject forged canonical_genome on non-DesignProgram JSON");
        assert!(
            err.contains("canonical subjects"),
            "EV12-WR-03: expected canonical subjects error, got: {err}"
        );
        assert!(
                finalize_wave4_case_identity(&forged_case).is_err(),
                "EV12-WR-03: finalize_wave4_case_identity must reject forged canonical_genome on non-DesignProgram JSON"
            );

        // Restoring the valid canonical_genome makes Rust builder accept it
        im["design_sha256"] = serde_json::Value::String(valid_canonical_genome);
        fs::write(
            &im_path,
            format!("{}\n", serde_json::to_string_pretty(&im).unwrap()),
        )
        .expect("write valid im");
        assert!(
                build_wave4_case_identity(&forged_case).is_ok(),
                "EV12-WR-03: build_wave4_case_identity must accept valid canonical_genome on non-DesignProgram JSON"
            );
    }

    // --- EV12-WR-02: relative case_dir from foreign working directory must resolve against caller cwd, not repo_root ---
    {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("repo root")
            .to_path_buf();
        let rel_subpath = format!(
            ".private/ev12_wr02_rel_probe_{}/target_case",
            std::process::id()
        );
        let repo_rel_target = repo_root.join(&rel_subpath);
        let foreign_cwd = root_tmp.join("foreign_cwd");
        let foreign_target = foreign_cwd.join(&rel_subpath);

        let _ = fs::remove_dir_all(repo_root.join(format!(
            ".private/ev12_wr02_rel_probe_{}",
            std::process::id()
        )));
        build_openfoam_candidate_case(&p_norm, &repo_rel_target, &cfg)
            .expect("prepare repo_rel_target");
        populate_synthetic_polymesh(&repo_rel_target);
        build_openfoam_candidate_case(&p_norm, &foreign_target, &cfg)
            .expect("prepare foreign_target");
        populate_synthetic_polymesh(&foreign_target);

        let orig_cwd = std::env::current_dir().expect("get current_dir");
        std::env::set_current_dir(&foreign_cwd).expect("set_current_dir to foreign_cwd");
        let finalize_res = finalize_wave4_case_identity(Path::new(&rel_subpath));
        let verify_res = verify_wave4_case_identity(Path::new(&rel_subpath));
        std::env::set_current_dir(&orig_cwd).expect("restore orig_cwd");

        let _ = finalize_res.expect("EV12-WR-02: relative finalize from foreign cwd must succeed");
        let ver = verify_res.expect("EV12-WR-02: relative verify from foreign cwd must succeed");
        assert_eq!(ver.state, "exact_artifact_bytes_verified");
        assert!(
            foreign_target.join(WAVE4_CASE_IDENTITY_FILE).is_file(),
            "EV12-WR-02: foreign_target must receive case-identity.json"
        );
        assert!(
            !repo_rel_target.join(WAVE4_CASE_IDENTITY_FILE).exists(),
            "EV12-WR-02: repo_rel_target must NOT be touched when caller cwd is foreign_cwd"
        );

        let _ = fs::remove_dir_all(repo_root.join(format!(
            ".private/ev12_wr02_rel_probe_{}",
            std::process::id()
        )));
    }

    // --- EV12-WR-04 & EV12-WR-05: emitted ev12-capture-workflow.sh works from case cwd without PYTHONPATH, does not prune before solver, and prunes only with --prune-polymesh ---
    {
        let wf_case = root_tmp.join("wr04_05_workflow_case");
        let wf_archive = root_tmp.join("wr04_05_workflow_archive");
        build_openfoam_candidate_case(&p_norm, &wf_case, &cfg).expect("prepare wf_case");
        populate_synthetic_polymesh(&wf_case);

        // EV12-WR-04: Run `bash ev12-capture-workflow.sh` with no args from inside wf_case and without PYTHONPATH
        let out_no_args = std::process::Command::new("bash")
            .arg("ev12-capture-workflow.sh")
            .current_dir(&wf_case)
            .env_remove("PYTHONPATH")
            .output()
            .expect("spawn bash ev12-capture-workflow.sh");
        assert!(
                out_no_args.status.success(),
                "EV12-WR-04: ev12-capture-workflow.sh from case cwd without PYTHONPATH failed: stdout={} stderr={}",
                String::from_utf8_lossy(&out_no_args.stdout),
                String::from_utf8_lossy(&out_no_args.stderr)
            );
        assert!(wf_case.join(WAVE4_CASE_IDENTITY_FILE).is_file());
        assert!(wf_case.join("constant/polyMesh/points").is_file());

        // EV12-WR-05 (pre-solver capture with ARCHIVE_DIR): must archive + verify AND retain constant/polyMesh
        let out_capture = std::process::Command::new("bash")
            .arg(wf_case.join("ev12-capture-workflow.sh"))
            .arg(&wf_case)
            .arg(&wf_archive)
            .current_dir(&wf_case)
            .env_remove("PYTHONPATH")
            .output()
            .expect("spawn bash ev12-capture-workflow.sh with archive_dir");
        assert!(
            out_capture.status.success(),
            "EV12-WR-05: pre-solver archive capture failed: stdout={} stderr={}",
            String::from_utf8_lossy(&out_capture.stdout),
            String::from_utf8_lossy(&out_capture.stderr)
        );
        assert!(
            wf_archive.join(WAVE4_CASE_IDENTITY_FILE).is_file(),
            "EV12-WR-05: archive_dir must contain verified case-identity.json"
        );
        assert!(
                wf_case.join("constant/polyMesh/points").is_file(),
                "EV12-WR-05: pre-solver capture must NOT implicitly delete constant/polyMesh before solver runs"
            );

        // EV12-WR-05 (explicit post-solver --prune-polymesh): must verify archive and prune constant/polyMesh
        let out_prune = std::process::Command::new("bash")
            .arg(wf_case.join("ev12-capture-workflow.sh"))
            .arg(&wf_case)
            .arg(&wf_archive)
            .arg("--prune-polymesh")
            .current_dir(&wf_case)
            .env_remove("PYTHONPATH")
            .output()
            .expect("spawn bash ev12-capture-workflow.sh --prune-polymesh");
        assert!(
            out_prune.status.success(),
            "EV12-WR-05: explicit --prune-polymesh failed: stdout={} stderr={}",
            String::from_utf8_lossy(&out_prune.stdout),
            String::from_utf8_lossy(&out_prune.stderr)
        );
        assert!(
                !wf_case.join("constant/polyMesh").exists(),
                "EV12-WR-05: constant/polyMesh must be pruned when --prune-polymesh is explicitly passed"
            );
        let arch_ver = verify_wave4_case_identity(&wf_archive)
            .expect("retained archive must remain verified after --prune-polymesh");
        assert_eq!(arch_ver.state, "exact_artifact_bytes_verified");
    }

    let _ = fs::remove_dir_all(&root_tmp);
}
