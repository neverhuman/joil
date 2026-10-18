use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn run(root: &Path, source: bool, python: bool) -> Output {
    let mut c = if python {
        let mut c = Command::new("python3");
        c.args(["-c","import sys,json;from pathlib import Path;sys.path.insert(0,'tools');import v5_baseline_audit as m\ntry: print(json.dumps(m.load(Path(sys.argv[1]),sys.argv[2]=='true'),sort_keys=True))\nexcept Exception as e: print('FAIL: '+str(e),file=sys.stderr);sys.exit(1)"]);
        c.arg(root).arg(source.to_string());
        c
    } else {
        let mut c = Command::new(env!("CARGO_BIN_EXE_v5-baseline-audit"));
        c.arg("--root").arg(root);
        if source {
            c.arg("--source-check");
        }
        c
    };
    c.current_dir(repo()).output().unwrap()
}
fn parity(root: &Path, source: bool, success: bool, exact: bool) {
    let p = run(root, source, true);
    let r = run(root, source, false);
    assert_eq!(p.status.code(), r.status.code(), "{:?} {:?}", p, r);
    assert_eq!(r.status.success(), success, "{:?}", r);
    assert_eq!(p.stdout, r.stdout);
    if exact {
        assert_eq!(p.stderr, r.stderr);
    }
}
#[test]
fn frozen_inventory_and_negative_controls() {
    parity(&repo(), false, true, true);
    parity(&repo(), true, true, true);
    let root = repo().join(format!("target/i015-baseline-{}", std::process::id()));
    fs::create_dir_all(root.join("docs/v5-plan/execution")).unwrap();
    for p in [
        "DEFECT_MAP.json",
        "TASKS.json",
        "execution/DEFECTS.json",
        "execution/DEFECT_ACCEPTANCE_GATES.json",
        "execution/V4_DEFERRALS.json",
    ] {
        fs::copy(
            repo().join("docs/v5-plan").join(p),
            root.join("docs/v5-plan").join(p),
        )
        .unwrap();
    }
    parity(&root, false, true, true);
    for (file, key, value) in [
        (
            "DEFECT_ACCEPTANCE_GATES.json",
            "frozen_source_commit",
            json!("stale"),
        ),
        (
            "DEFECT_ACCEPTANCE_GATES.json",
            "inventory_sha256",
            json!("0".repeat(64)),
        ),
        ("DEFECT_ACCEPTANCE_GATES.json", "records", json!([])),
        ("V4_DEFERRALS.json", "deferrals", json!([])),
        ("V4_DEFERRALS.json", "source_path", json!("other")),
    ] {
        let p = root.join("docs/v5-plan/execution").join(file);
        let saved = fs::read(&p).unwrap();
        let mut v: Value = serde_json::from_slice(&saved).unwrap();
        v[key] = value;
        fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
        parity(&root, false, false, true);
        fs::write(p, saved).unwrap();
    }
    let p = root.join("docs/v5-plan/execution/DEFECT_ACCEPTANCE_GATES.json");
    let saved = fs::read(&p).unwrap();
    for (k, v) in [
        ("primary_task", json!("wrong")),
        ("baseline_disposition", json!("PASS")),
        ("source_finding_sha256", json!([])),
        ("runtime_probe", json!(null)),
        ("positive_fixture", json!("")),
        ("primary_repair_closure", json!("closed")),
        ("physical_qualification", json!(true)),
    ] {
        let mut d: Value = serde_json::from_slice(&saved).unwrap();
        d["records"][0][k] = v;
        fs::write(&p, serde_json::to_vec(&d).unwrap()).unwrap();
        parity(&root, false, false, true);
    }
    for raw in [b"{broken".as_slice(), b"{}"] {
        fs::write(&p, raw).unwrap();
        parity(&root, false, false, false);
    }
    fs::rename(&p, p.with_extension("missing")).unwrap();
    parity(&root, false, false, false);
    fs::write(p, saved).unwrap();
    parity(&root, false, true, true);
}
