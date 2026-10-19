mod mesh_support;
use mesh_support::*;
use std::fs;
fn fixture(root: &std::path::Path) {
    put(root,"runs/case1/domain-extension.json",serde_json::json!({"source_case":"source","source_mesh_sha256":{"faces":sha(&fs::read(root.join("runs/source/constant/polyMesh/faces")).unwrap())}}).to_string());
    put(
        root,
        "runs/case1/constant/polyMesh/points",
        b"4\n((0 0 -1) (0 1 0) (1 0 0) (0 0 0))\n",
    );
    put(
        root,
        "runs/case1/constant/polyMesh/faces",
        b"2\n(3(1 3 2) 3(0 3 2))\n",
    );
    put(root, "runs/case1/constant/polyMesh/neighbour", b"1\n(0)\n");
}
#[test]
fn exact_plane_recovery_matches_python_and_rejects_ambiguity() {
    for mode in [
        "ok",
        "duplicate",
        "nan",
        "missing",
        "boundary",
        "source",
        "existing-set",
        "existing-receipt",
        "outside",
        "symlink",
    ] {
        let root = setup();
        fixture(&root);
        let mut args = Vec::new();
        match mode {
            "nan" => {
                put(
                    &root,
                    "runs/source/constant/polyMesh/points",
                    b"4\n((nan 0 0) (1 0 0) (0 1 0) (0 0 -1))\n",
                );
                put(
                    &root,
                    "runs/case1/constant/polyMesh/points",
                    b"4\n((0 0 -1) (0 1 0) (1 0 0) (nan 0 0))\n",
                );
            }
            "duplicate" => put(
                &root,
                "runs/case1/constant/polyMesh/faces",
                b"2\n(3(1 3 2) 3(2 1 3))\n",
            ),
            "missing" => put(
                &root,
                "runs/case1/constant/polyMesh/faces",
                b"1\n(3(0 3 2))\n",
            ),
            "boundary" => put(&root, "runs/case1/constant/polyMesh/neighbour", b"0\n()\n"),
            "source" => put(
                &root,
                "runs/case1/domain-extension.json",
                b"{\"source_case\":\"wrong\"}",
            ),
            "existing-set" => put(
                &root,
                "runs/case1/constant/polyMesh/sets/deviceOverflow",
                b"retained",
            ),
            "existing-receipt" => put(&root, "runs/case1/device-plane.json", b"retained"),
            "outside" => args = vec!["--source", "bin"],
            "symlink" => {
                std::os::unix::fs::symlink(root.join("bin"), root.join("runs/link")).unwrap();
                args = vec!["--source", "runs/link"];
            }
            _ => {}
        }
        let original = snapshot(&root.join("runs/case1"));
        let py = invoke(&root, true, "reference_plane", &args, "ok");
        fs::rename(root.join("runs/case1"), root.join("python-case")).unwrap();
        for (name, bytes) in original {
            put(&root, &format!("runs/case1/{name}"), bytes);
        }
        let rs = invoke(&root, false, "reference_plane", &args, "ok");
        assert_eq!(py.status.success(), mode == "ok", "{mode}: {py:?}");
        assert_eq!(rs.status.success(), py.status.success(), "{mode}: {rs:?}");
        compare_cases(&root);
        if mode == "ok" {
            assert_eq!(py.stdout, rs.stdout);
            assert!(!invoke(&root, false, "reference_plane", &[], "ok")
                .status
                .success());
        }
    }
}
