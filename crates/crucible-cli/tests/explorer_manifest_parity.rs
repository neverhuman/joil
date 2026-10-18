use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn copy_json(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let dest = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_json(&path, &dest);
        } else if path.extension().is_some_and(|s| s == "json") {
            fs::copy(path, dest).unwrap();
        }
    }
}
fn run(root: &Path, command: &str, python: bool) -> Output {
    let mut cmd = if python {
        let mut c = Command::new("python3");
        c.args(["-c", "import sys; from pathlib import Path; from tools import v5_explorer_manifest as m\ntry:\n root=Path(sys.argv[1]); m.apply(m.generated(root),root,check=sys.argv[2]=='check'); print('PASS: canonical software-progress explorer '+sys.argv[2]+'; physical observations absent')\nexcept (ValueError,OSError,KeyError) as e:\n print('FAIL: '+str(e),file=sys.stderr);sys.exit(1)"]);
        c.arg(root).arg(command).current_dir(self::root());
        c
    } else {
        let mut c = Command::new(env!("CARGO_BIN_EXE_v5-explorer-manifest"));
        c.arg(command).arg("--root").arg(root);
        c
    };
    cmd.output().unwrap()
}
fn parity(root: &Path, command: &str, success: bool, exact_error: bool) {
    let py = run(root, command, true);
    let rs = run(root, command, false);
    assert_eq!(
        rs.status.code(),
        py.status.code(),
        "{command}: python={} rust={}",
        String::from_utf8_lossy(&py.stderr),
        String::from_utf8_lossy(&rs.stderr)
    );
    assert_eq!(
        rs.status.success(),
        success,
        "{}",
        String::from_utf8_lossy(&rs.stderr)
    );
    assert_eq!(rs.stdout, py.stdout);
    if exact_error {
        assert_eq!(
            String::from_utf8_lossy(&rs.stderr),
            String::from_utf8_lossy(&py.stderr)
        );
    }
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
#[test]
fn generated_bytes_and_validation_controls_match_python() {
    let repo = root();
    let scratch = repo.join(format!("target/i013-explorer-{}", std::process::id()));
    fs::create_dir_all(&scratch).unwrap();
    copy_json(&repo.join("docs"), &scratch.join("docs"));
    // Defect fixtures can be Rust/text files; copy each source-bound fixture too.
    let paths = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(&repo)
        .output()
        .unwrap();
    for name in paths.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let name = std::str::from_utf8(name).unwrap();
        if !name.ends_with(".png") && !name.ends_with(".svg") {
            let p = scratch.join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::copy(repo.join(name), p).unwrap();
        }
    }
    assert!(Command::new("git")
        .args(["init", "-q"])
        .arg(&scratch)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["add", "."])
        .current_dir(&scratch)
        .status()
        .unwrap()
        .success());
    let py = run(&scratch, "build", true);
    assert!(
        py.status.success(),
        "{}",
        String::from_utf8_lossy(&py.stderr)
    );
    let manifest = scratch.join("apps/explorer/public/evidence-manifest.json");
    let expected = fs::read(&manifest).unwrap();
    let source = scratch.join(format!(
        "apps/explorer/public/v5/source-{}.json",
        read(&manifest)["revision"].as_str().unwrap()
    ));
    let raw = fs::read(&source).unwrap();
    // Remove only generated files whose bytes were just captured, force native generation.
    fs::remove_file(&manifest).unwrap();
    fs::remove_file(&source).unwrap();
    let rs = run(&scratch, "build", false);
    assert!(
        rs.status.success(),
        "{}",
        String::from_utf8_lossy(&rs.stderr)
    );
    assert_eq!(rs.stdout, py.stdout);
    assert_eq!(rs.stderr, py.stderr);
    assert_eq!(fs::read(&manifest).unwrap(), expected);
    assert_eq!(fs::read(&source).unwrap(), raw);
    parity(&scratch, "check", true, true);
    for path in [&manifest, &source] {
        let saved = fs::read(path).unwrap();
        fs::write(path, b"stale").unwrap();
        parity(&scratch, "check", false, true);
        if path == &source {
            parity(&scratch, "build", false, true);
        }
        fs::write(path, &saved).unwrap();
        fs::remove_file(path).unwrap();
        std::os::unix::fs::symlink(scratch.join("docs/STATUS.json"), path).unwrap();
        parity(&scratch, "check", false, true);
        parity(&scratch, "build", false, true);
        fs::remove_file(path).unwrap();
        fs::write(path, saved).unwrap();
    }
    let tasks = scratch.join("docs/v5-plan/TASKS.json");
    let original = fs::read(&tasks).unwrap();
    for case in 0..9 {
        let mut data: Value = serde_json::from_slice(&original).unwrap();
        match case {
            0 => data["schema_version"] = json!(2),
            1 => data["packets"][0]["id"] = data["packets"][1]["id"].clone(),
            2 => data["packets"][0]["dependencies"] = json!(["V5-00"]),
            3 => data["packets"][0]["write_scope"] = json!(["../escape"]),
            4 => data["packets"][0]["write_scope"] = json!([".private/secret"]),
            5 => data["packets"][0]["owner_role"] = json!(""),
            6 => data["packets"][0]["tasks"][0]["acceptance"] = json!([]),
            7 => data["packets"][0]["dependencies"] = json!(["V5-01", "V5-01"]),
            _ => data["packets"][0]["dependencies"] = json!(["V5-01"]),
        }
        write(&tasks, &data);
        parity(&scratch, "check", false, true);
    }
    fs::write(&tasks, &original).unwrap();
    let progress = scratch.join("docs/v5-plan/progress/V5-00.json");
    let saved = fs::read(&progress).unwrap();
    for case in 0..8 {
        let mut p: Value = serde_json::from_slice(&saved).unwrap();
        let row = &mut p["tasks"]["V5-00-01"];
        match case {
            0 => row["status"] = json!("invented"),
            1 => row["owner"] = Value::Null,
            2 => row["evidence"] = json!([]),
            3 => row["evidence"][0]["sha256"] = json!("0".repeat(64)),
            4 => row["note"] = json!(42),
            5 => {
                row["status"] = json!("blocked");
                row["note"] = json!(" ");
            }
            6 => p["schema_version"] = json!(2),
            _ => {
                p["tasks"].as_object_mut().unwrap().remove("V5-00-01");
            }
        }
        write(&progress, &p);
        parity(&scratch, "check", false, true);
    }
    fs::write(&progress, saved).unwrap();
    for (name, key, value) in [
        ("SOURCE_MANIFEST.json", "source_count", json!(9)),
        ("CATALOGUE_MAP.json", "families", json!([])),
        ("DEFECT_MAP.json", "defects", json!([])),
    ] {
        let path = scratch.join("docs/v5-plan").join(name);
        let saved = fs::read(&path).unwrap();
        let mut v = read(&path);
        v[key] = value;
        write(&path, &v);
        parity(&scratch, "check", false, true);
        fs::write(path, saved).unwrap();
    }
    // Rebind the mutated receipt digest so checks reach its semantic controls.
    let progress31 = scratch.join("docs/v5-plan/progress/V5-31.json");
    let saved31 = fs::read(&progress31).unwrap();
    let initial31 = read(&progress31);
    let reference = &initial31["tasks"]["V5-31-02"]["evidence"][0];
    let receipt_path = scratch.join(reference["path"].as_str().unwrap());
    let receipt_bytes = fs::read(&receipt_path).unwrap();
    for case in 0..7 {
        let mut receipt: Value = serde_json::from_slice(&receipt_bytes).unwrap();
        match case {
            0 => receipt["commands"][0]["exit_code"] = json!(1),
            1 => receipt["tested_commit"] = json!("stale"),
            2 => receipt["defect_controls"][0]["source"]["commit"] = json!("0".repeat(40)),
            3 => receipt["defect_controls"][0]["red"]["exit_code"] = json!(0),
            4 => receipt["defect_controls"] = json!([]),
            5 => receipt["defect_controls"][0]["fixture"]["sha256"] = json!("0".repeat(64)),
            _ => receipt["commands"][0]["log_sha256"] = json!("missing"),
        }
        write(&receipt_path, &receipt);
        use sha2::{Digest, Sha256};
        let mut p = initial31.clone();
        p["tasks"]["V5-31-02"]["evidence"][0]["sha256"] = json!(format!(
            "{:x}",
            Sha256::digest(fs::read(&receipt_path).unwrap())
        ));
        write(&progress31, &p);
        parity(&scratch, "check", false, true);
    }
    fs::write(receipt_path, receipt_bytes).unwrap();
    fs::write(progress31, saved31).unwrap();
    // Syntax errors use language-specific parser diagnostics, but both fail closed.
    for bytes in [
        b"{broken".as_slice(),
        b"{\"schema_version\":1,\"schema_version\":1}",
        b"{\"schema_version\":NaN}",
    ] {
        fs::write(&tasks, bytes).unwrap();
        parity(&scratch, "check", false, bytes.contains(&b','));
    }
    fs::write(tasks, original).unwrap();
    parity(&scratch, "check", true, true);
}
