mod mesh_support;
use mesh_support::*;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn fixture() -> PathBuf {
    let root = repo().join(format!(
        "runs/i021-snapshot-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    put(&root,"case/mesh-input.json",json!({"genome_file_sha256":sha(&fs::read(repo().join("baselines/ct35/genome.json")).unwrap())}).to_string());
    put(&root, "case/0/p", "dimensions [0 2 -2 0 0 0 0];\n");
    put(&root, "case/mesh-audit.json", r#"{"pilot_mesh_pass":true}"#);
    for (n, q, p) in [
        ("inlet1", -0.0005, 40.),
        ("inlet2", -0.0005, 42.),
        ("overflow", 0.00002, -10.),
        ("underflow", 0.00098, 0.),
    ] {
        for (s, v) in [("flow", q), ("pressure", p)] {
            put(
                &root,
                &format!("case/postProcessing/{n}_{s}/0/data.dat"),
                format!("# synthetic scalar stream\n1 {v}\n2 {v}\n3 "),
            );
        }
    }
    bind(&root);
    root
}
fn bind(root: &Path) {
    let case = root.join("case");
    let mut manifest = serde_json::Map::new();
    for (name, bytes) in snapshot(&case) {
        if !name.starts_with("postProcessing/") && name != "input-manifest.json" {
            manifest.insert(name, json!(sha(&bytes)));
        }
    }
    put(
        &case,
        "input-manifest.json",
        Value::Object(manifest).to_string(),
    );
}
fn invoke(root: &Path, python: bool, out: &str) -> std::process::Output {
    let mut c = if python {
        let mut c = Command::new("python3");
        c.arg("-S").arg(repo().join("analysis/carrier_snapshot.py"));
        c
    } else {
        Command::new(env!("CARGO_BIN_EXE_carrier-snapshot"))
    };
    c.arg("--case")
        .arg(root.join("case"))
        .arg("--out")
        .arg(root.join(out))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .unwrap()
}
fn compare(a: &Value, b: &Value) {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            assert!(
                (a - b).abs() <= 1e-13 * a.abs().max(b.abs()).max(1.),
                "{a} != {b}"
            );
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len());
            for (k, v) in a {
                compare(v, &b[k]);
            }
        }
        _ => assert_eq!(a, b),
    }
}
#[test]
fn positive_snapshot_parity() {
    for variant in [
        "kinematic",
        "dynamic",
        "failed_mesh",
        "missing_mesh",
        "reverse",
        "split",
        "model",
    ] {
        let root = fixture();
        match variant {
            "dynamic" => put(&root, "case/0/p", "dimensions [1 -1 -2 0 0 0 0];\n"),
            "failed_mesh" => put(
                &root,
                "case/mesh-audit.json",
                r#"{"pilot_mesh_pass":false}"#,
            ),
            "missing_mesh" => {
                fs::rename(
                    root.join("case/mesh-audit.json"),
                    root.join("saved-audit.json"),
                )
                .unwrap();
            }
            "reverse" => put(
                &root,
                "case/postProcessing/overflow_flow/0/data.dat",
                "1 -0.00001\n2 -0.00001\n",
            ),
            "split" => put(
                &root,
                "case/postProcessing/overflow_flow/0/data.dat",
                "1 0.0001\n2 0.0001\n",
            ),
            "model" => put(&root, "case/carrier-input.json", r#"{"model":"kOmegaSST"}"#),
            _ => {}
        }
        bind(&root);
        let a = invoke(&root, true, "python");
        let b = invoke(&root, false, "rust");
        assert!(a.status.success(), "{}", String::from_utf8_lossy(&a.stderr));
        assert!(b.status.success(), "{}", String::from_utf8_lossy(&b.stderr));
        let mut a: Value = serde_json::from_slice(&a.stdout).unwrap();
        let mut b: Value = serde_json::from_slice(&b.stdout).unwrap();
        a["created_utc"] = Value::Null;
        b["created_utc"] = Value::Null;
        compare(&a, &b);
        assert_eq!(
            snapshot(&root.join("python/source-data")),
            snapshot(&root.join("rust/source-data"))
        );
        assert!(!invoke(&root, false, "rust").status.success());
    }
}
#[test]
fn negative_snapshot_parity() {
    for variant in [
        "tamper",
        "escape",
        "duplicate",
        "no_common",
        "nonfinite",
        "empty",
        "genome",
        "pressure",
        "outside",
    ] {
        let root = fixture();
        match variant {
            "tamper" => put(&root, "case/0/p", "tampered"),
            "escape" => {
                put(&root, "outside", "synthetic");
                put(
                    &root,
                    "case/input-manifest.json",
                    json!({"../outside":sha(b"synthetic")}).to_string(),
                );
            }
            "duplicate" => put(
                &root,
                "case/postProcessing/inlet1_flow/0/data.dat",
                "1 -0.1\n1 -0.1\n",
            ),
            "no_common" => put(
                &root,
                "case/postProcessing/inlet1_flow/0/data.dat",
                "9 -0.1\n",
            ),
            "nonfinite" => put(
                &root,
                "case/postProcessing/inlet1_flow/0/data.dat",
                "1 NaN\n",
            ),
            "empty" => put(
                &root,
                "case/postProcessing/inlet1_flow/0/data.dat",
                "1 -0.1",
            ),
            "genome" => {
                put(
                    &root,
                    "case/mesh-input.json",
                    r#"{"genome_file_sha256":"wrong"}"#,
                );
                bind(&root);
            }
            "pressure" => {
                put(&root, "case/0/p", "dimensions [1 1 1 1 1 1 1];");
                bind(&root);
            }
            "outside" => {
                let outside = setup();
                for py in [true, false] {
                    let out = invoke(&root, py, outside.to_str().unwrap());
                    assert!(!out.status.success());
                }
                continue;
            }
            _ => {}
        }
        for py in [true, false] {
            let out = invoke(&root, py, if py { "python" } else { "rust" });
            assert!(!out.status.success(), "accepted {variant}");
        }
    }
}
