mod mesh_support;
use mesh_support::*;
use std::fs;
#[test]
fn extension_dictionaries_mesh_proofs_and_runtime_match_python() {
    for args in [
        vec![],
        vec!["--length", "1e-7", "--layers", "2"],
        vec!["--length", "0.0001234567890123456", "--layers", "2"],
        vec!["--length", "0.3", "--layers", "300"],
    ] {
        let root = setup();
        if args.is_empty() {
            put(
                &root,
                "runs/source/mesh-input.json",
                r#"{"analytic_volume_m3":2,"patches":{"overflow":{}},"label":"é"}"#,
            );
            put(
                &root,
                "runs/source/mesh-audit.json",
                r#"{"pilot_mesh_pass":false,"patches":{"overflow":{"area_m2":1}}}"#,
            );
        }
        let py = invoke(&root, true, "extend_outlet", &args, "ok");
        assert!(py.status.success(), "{py:?}");
        fs::rename(root.join("runs/case1"), root.join("python-case")).unwrap();
        let rs = invoke(&root, false, "extend_outlet", &args, "ok");
        assert!(rs.status.success(), "{rs:?}");
        compare_cases(&root);
        assert!(!invoke(&root, false, "extend_outlet", &args, "ok")
            .status
            .success());
    }
}
#[test]
fn extension_rejects_budget_reuse_changed_points_and_runtime_failure() {
    for (mode, args, message) in [
        (
            "ok",
            vec!["--length", "0"],
            "extension outside pilot budget",
        ),
        (
            "ok",
            vec!["--length", "nan"],
            "extension outside pilot budget",
        ),
        (
            "ok",
            vec!["--length", "0.31"],
            "extension outside pilot budget",
        ),
        (
            "ok",
            vec!["--layers", "1"],
            "extension outside pilot budget",
        ),
        (
            "ok",
            vec!["--layers", "301"],
            "extension outside pilot budget",
        ),
        ("ok", vec!["--case", "outside"], "project runs only"),
        ("repeat", vec![], "extend the original domain only"),
        ("existing", vec![], "fresh case required"),
        (
            "changed",
            vec![],
            "extrusion changed an original mesh point",
        ),
        ("docker", vec![], "extrudeMesh"),
        ("nan", vec![], "extrusion changed an original mesh point"),
    ] {
        let root = setup();
        if mode == "nan" {
            put(
                &root,
                "runs/source/constant/polyMesh/points",
                b"4\n((nan 0 0) (1 0 0) (0 1 0) (0 0 -1))\n",
            );
        }
        if mode == "repeat" {
            put(&root, "runs/source/domain-extension.json", b"{}");
        }
        if mode == "existing" {
            put(&root, "runs/case1/retained", b"retained");
        }
        let py = invoke(&root, true, "extend_outlet", &args, mode);
        if root.join("runs/case1").exists() {
            fs::rename(root.join("runs/case1"), root.join("python-case")).unwrap();
        }
        if mode == "existing" {
            put(&root, "runs/case1/retained", b"retained");
        }
        let rs = invoke(&root, false, "extend_outlet", &args, mode);
        for output in [&py, &rs] {
            assert!(!output.status.success(), "{mode}: {output:?}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(message),
                "{mode}: {output:?}"
            );
        }
        if root.join("python-case").exists() {
            compare_cases(&root);
        }
        if mode == "changed" {
            assert!(!root.join("runs/case1/domain-extension.json").exists());
        }
    }
}
