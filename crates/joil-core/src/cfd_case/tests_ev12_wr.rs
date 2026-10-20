//! EV12-WR-01 through EV12-WR-05 regression suite for OpenFOAM 14 case identity and archive verification.

use super::*;

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
