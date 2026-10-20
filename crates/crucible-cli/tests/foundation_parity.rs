use serde_json::Value;
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{fs, path::Path, process::Command};
fn receipt(stdout: &[u8]) -> Value {
    let output = String::from_utf8_lossy(stdout);
    let path = output
        .lines()
        .find_map(|l| l.strip_prefix("receipt: "))
        .expect("receipt path");
    let value: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    for step in value["steps"].as_array().unwrap() {
        let bytes = fs::read(
            Path::new(path)
                .parent()
                .unwrap()
                .join(format!("{}.log", step["name"].as_str().unwrap())),
        )
        .unwrap();
        assert_eq!(step["log_sha256"], format!("{:x}", Sha256::digest(&bytes)));
        assert_eq!(step["log_bytes"], bytes.len());
    }
    value
}
#[test]
#[cfg(unix)]
fn all_suite_receipts_and_failures_match_python() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let root = repo.join("target/i012-foundation-parity");
    fs::create_dir_all(root.join("tools")).unwrap();
    fs::create_dir_all(root.join("apps/explorer")).unwrap();
    fs::create_dir_all(root.join(".private/node-runtime/bin")).unwrap();
    fs::write(root.join(".gitignore"), ".private/\n").unwrap();
    fs::copy(repo.join("tools/check.py"), root.join("tools/check.py")).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .arg(&root)
        .status()
        .unwrap()
        .success());
    for name in ["cargo", "npm", "python3"] {
        let path = root.join(".private/node-runtime/bin").join(name);
        fs::write(&path,"#!/bin/sh\nprintf 'synthetic stdout\\n'\nprintf 'synthetic stderr\\n' >&2\nexit \"${PARITY_EXIT:-0}\"\n").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    for fail in [false, true] {
        for suite in [
            "all",
            "skip-ui",
            "fmt",
            "clippy",
            "rust-tests",
            "baseline",
            "python-tests",
            "public-check",
            "claims-audit",
            "ui",
        ] {
            let args = match suite {
                "all" => vec![],
                "skip-ui" => vec!["--skip-ui"],
                _ => vec!["--suite", suite],
            };
            let py = Command::new("python3")
                .arg("tools/check.py")
                .args(&args)
                .env("PARITY_EXIT", if fail { "7" } else { "0" })
                .current_dir(&root)
                .output()
                .unwrap();
            let rs = Command::new(env!("CARGO_BIN_EXE_check-foundation"))
                .args(&args)
                .env("PARITY_EXIT", if fail { "7" } else { "0" })
                .current_dir(&root)
                .output()
                .unwrap();
            assert_eq!(rs.status.code(), py.status.code(), "{suite} {rs:?}");
            assert_eq!(rs.status.success(), !fail);
            let mut p = receipt(&py.stdout);
            let mut r = receipt(&rs.stdout);
            for v in [&mut p, &mut r] {
                v.as_object_mut().unwrap().remove("run_id");
                for step in v["steps"].as_array_mut().unwrap() {
                    step.as_object_mut().unwrap().remove("wall_seconds");
                }
            }
            assert_eq!(r, p, "{suite} failed={fail}");
        }
    }
    for args in [
        vec!["--suite", "unknown"],
        vec!["--suite", "fmt", "--cad"],
        vec!["--suite", "ui", "--skip-ui"],
    ] {
        let py = Command::new("python3")
            .arg("tools/check.py")
            .args(&args)
            .current_dir(&root)
            .output()
            .unwrap();
        let rs = Command::new(env!("CARGO_BIN_EXE_check-foundation"))
            .args(&args)
            .current_dir(&root)
            .output()
            .unwrap();
        assert_eq!(rs.status.code(), py.status.code());
        assert!(!rs.status.success());
    }
    // CAD uses an absolute project-local executable, so this missing producer
    // cannot silently fall back to a system command. An attempt is not execution.
    assert!(!root.join(".venv/bin/python").exists());
    let py = Command::new("python3")
        .args(["tools/check.py", "--skip-ui", "--cad"])
        .current_dir(&root)
        .output()
        .unwrap();
    let rs = Command::new(env!("CARGO_BIN_EXE_check-foundation"))
        .args(["--skip-ui", "--cad"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert_eq!(py.status.code(), Some(1));
    assert_eq!(rs.status.code(), Some(1));
    for value in [receipt(&py.stdout), receipt(&rs.stdout)] {
        assert_eq!(value["cad_selected"], true);
        assert_eq!(value["cad_attempted"], true);
        assert_eq!(value["cad_executed"], false);
        assert_eq!(value["cad_passed"], false);
        assert_eq!(value["ui_selected"], false);
        assert_eq!(value["ui_executed"], false);
        assert_eq!(value["all_requested_pass"], false);
        assert_eq!(
            value["steps"].as_array().unwrap().last().unwrap()["executed"],
            false
        );
    }
}
