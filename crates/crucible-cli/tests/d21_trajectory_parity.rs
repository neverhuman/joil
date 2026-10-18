use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
#[test]
fn trajectory_controls_match_python() {
    let repo = repo();
    let root = repo.join(format!("target/i016-d21-{}", std::process::id()));
    for file in [
        "docs/v5-plan/execution/DEFECT_ACCEPTANCE_GATES.json",
        "docs/v5-plan/execution/DEFECTS.json",
        "crates/crucible-render/src/lib.rs",
        "recipes/v5/registry.json",
        "crates/joil-core/src/scene.rs",
    ] {
        let dest = root.join(file);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::copy(repo.join(file), dest).unwrap();
    }
    // Git discovers the enclosing repository; only current contract files are replayed.
    for case in [
        "clean",
        "commit",
        "inventory",
        "renderer",
        "registry",
        "scene",
        "web",
    ] {
        let file = match case {
            "commit" => "docs/v5-plan/execution/DEFECT_ACCEPTANCE_GATES.json",
            "inventory" => "docs/v5-plan/execution/DEFECTS.json",
            "renderer" => "crates/crucible-render/src/lib.rs",
            "registry" => "recipes/v5/registry.json",
            _ => "crates/joil-core/src/scene.rs",
        };
        let path = root.join(file);
        let saved = fs::read(&path).unwrap();
        match case {
            "commit" | "inventory" => {
                let mut data: Value = serde_json::from_slice(&saved).unwrap();
                if case == "commit" {
                    data["frozen_source_commit"] = json!("drift");
                } else {
                    let defect = data["defects"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|d| d["id"] == "D-21")
                        .unwrap();
                    defect["sources"][0]["source_sha256"] = json!("drift");
                }
                fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
            }
            "renderer" | "registry" | "scene" => fs::write(&path, "").unwrap(),
            "web" => fs::create_dir_all(root.join("crates/crucible-web")).unwrap(),
            _ => {}
        }
        for mode in ["positive", "red"] {
            let py=Command::new("python3").current_dir(&repo).args(["-c","import sys;from pathlib import Path;from tools import v5_d21_trajectory_control as m;m.ROOT=Path(sys.argv.pop(1));m.run_renderer_regression=lambda:None;sys.exit(m.main())"]).arg(&root).args(["--mode",mode]).output().unwrap();
            let rs = Command::new(env!("CARGO_BIN_EXE_v5-d21-trajectory-control"))
                .args(["--mode", mode, "--skip-renderer-test", "--root"])
                .arg(&root)
                .output()
                .unwrap();
            assert_eq!(
                rs.status.code(),
                Some(if case != "clean" {
                    2
                } else if mode == "red" {
                    1
                } else {
                    0
                }),
                "{case}: {rs:?}"
            );
            assert_eq!(
                (rs.status.code(), rs.stdout, rs.stderr),
                (py.status.code(), py.stdout, py.stderr),
                "{case}/{mode}"
            );
        }
        fs::write(path, saved).unwrap();
    }
}
#[test]
fn actual_renderer_runs_by_default() {
    for mode in ["positive", "red"] {
        let py = Command::new("python3")
            .arg(repo().join("tools/v5_d21_trajectory_control.py"))
            .args(["--mode", mode])
            .output()
            .unwrap();
        let rs = Command::new(env!("CARGO_BIN_EXE_v5-d21-trajectory-control"))
            .args(["--mode", mode])
            .output()
            .unwrap();
        assert_eq!(
            rs.status.code(),
            Some(if mode == "positive" { 0 } else { 1 }),
            "{rs:?}"
        );
        assert_eq!(py.status.code(), rs.status.code(), "{py:?}");
        // Cargo elapsed time/build notices vary; control output after the child test is exact.
        let p = String::from_utf8(py.stdout).unwrap();
        let r = String::from_utf8(rs.stdout).unwrap();
        let marker = "crucible-render unavailable-data regression:";
        assert_eq!(
            p.split_once(marker).unwrap().1,
            r.split_once(marker).unwrap().1
        );
        assert!(p.contains("1 passed"));
        assert!(r.contains("1 passed"));
    }
}
