mod mesh_support;
use mesh_support::*;
use serde_json::{json, Value};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn setup_carrier() -> std::path::PathBuf {
    let root = setup();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in [
        "physics/openfoam/carrier/configure.py",
        "physics/openfoam/carrier/point.py",
        "physics/openfoam/mesh/prepare.py",
        "cad/ct35/geometry.py",
        "baselines/ct35/genome.json",
    ] {
        put(&root, name, fs::read(repo.join(name)).unwrap());
    }
    root
}
fn invoke(root: &Path, python: bool, point: bool, args: &[&str]) -> Output {
    let mut c = if python {
        let mut c = Command::new("python3");
        c.arg("-S").arg(if point {
            "physics/openfoam/carrier/point.py"
        } else {
            "physics/openfoam/carrier/configure.py"
        });
        c
    } else {
        let mut c = Command::new(if point {
            env!("CARGO_BIN_EXE_carrier-point")
        } else {
            env!("CARGO_BIN_EXE_carrier-configure")
        });
        c.arg("--root").arg(root);
        c
    };
    c.current_dir(root)
        .args(["--case", "runs/case1"])
        .args(args)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .unwrap()
}
fn numeric_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut token = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() || "_+-.".contains(c) {
            token.push(c)
        } else {
            if !token.is_empty() {
                out.push(std::mem::take(&mut token));
            }
            if !c.is_whitespace() {
                out.push(c.to_string());
            }
        }
    }
    if !token.is_empty() {
        out.push(token)
    }
    out
}
fn compare_json(a: &Value, b: &Value) {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let a = a.as_f64().unwrap();
            let b = b.as_f64().unwrap();
            assert!(
                (a - b).abs() <= 2e-14 * a.abs().max(b.abs()).max(1e-30),
                "{a} != {b}"
            )
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len());
            for (k, v) in a {
                compare_json(v, &b[k])
            }
        }
        _ => assert_eq!(a, b),
    }
}
// Dictionary numeric values compare at 2e-14 relative tolerance: libm pow and
// shortest float formatting can differ by ULP. All nonnumeric tokens are exact.
fn compare(root: &Path) {
    let a = snapshot(&root.join("python-case"));
    let b = snapshot(&root.join("runs/case1"));
    assert_eq!(a.len(), b.len());
    for ((an, av), (bn, bv)) in a.iter().zip(&b) {
        assert_eq!(an, bn);
        if an == "input-manifest.json" {
            for (folder, bytes) in [("python-case", av), ("runs/case1", bv)] {
                let v: Value = serde_json::from_slice(bytes).unwrap();
                for (p, h) in v.as_object().unwrap() {
                    assert_eq!(
                        h,
                        &json!(sha(&fs::read(root.join(folder).join(p)).unwrap()))
                    );
                }
            }
            let a: Value = serde_json::from_slice(av).unwrap();
            let b: Value = serde_json::from_slice(bv).unwrap();
            assert_eq!(
                a.as_object().unwrap().keys().collect::<Vec<_>>(),
                b.as_object().unwrap().keys().collect::<Vec<_>>()
            );
        } else if an.ends_with(".json") {
            compare_json(
                &serde_json::from_slice(av).unwrap(),
                &serde_json::from_slice(bv).unwrap(),
            )
        } else {
            let a = numeric_tokens(std::str::from_utf8(av).unwrap());
            let b = numeric_tokens(std::str::from_utf8(bv).unwrap());
            assert_eq!(a.len(), b.len(), "{an}");
            for (a, b) in a.iter().zip(&b) {
                match (a.parse::<f64>(), b.parse::<f64>()) {
                    (Ok(a), Ok(b)) => assert!(
                        (a - b).abs() <= 2e-14 * a.abs().max(b.abs()).max(1e-30),
                        "{an}: {a} != {b}"
                    ),
                    _ => assert_eq!(a, b, "{an}"),
                }
            }
        }
    }
}
#[test]
fn configure_models_and_rejection_parity() {
    for mode in [
        "SSG",
        "LRR",
        "kOmegaSST",
        "flow-zero",
        "flow-nan",
        "pressure-inf",
        "steps-zero",
        "model",
        "bound",
        "outside",
        "symlink",
        "genome",
    ] {
        let root = setup_carrier();
        let mut args = vec![];
        match mode {
            "SSG" | "LRR" | "kOmegaSST" => {
                args = vec![
                    "--model",
                    mode,
                    "--flow",
                    "5.4",
                    "--steps",
                    "7",
                    "--reject-pressure-pa",
                    "-12345",
                ]
            }
            "flow-zero" => args = vec!["--flow", "0"],
            "flow-nan" => args = vec!["--flow", "nan"],
            "pressure-inf" => args = vec!["--reject-pressure-pa", "inf"],
            "steps-zero" => args = vec!["--steps", "0"],
            "model" => args = vec!["--model", "invalid"],
            "bound" => put(&root, "runs/case1/input-manifest.json", "{}"),
            "outside" => args = vec!["--case", "outside"],
            "symlink" => {
                std::os::unix::fs::symlink(root.join("bin"), root.join("runs/link")).unwrap();
                args = vec!["--case", "runs/link/case"]
            }
            "genome" => {
                let p = root.join("baselines/ct35/genome.json");
                let mut v: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
                v["parameters"]["n_inlets"] = json!(3);
                fs::write(p, v.to_string()).unwrap()
            }
            _ => {}
        }
        let py = invoke(&root, true, false, &args);
        let original = if root.join("runs/case1").exists() {
            fs::rename(root.join("runs/case1"), root.join("python-case")).unwrap();
            true
        } else {
            false
        };
        if mode == "bound" {
            put(&root, "runs/case1/input-manifest.json", "{}");
        }
        let rs = invoke(&root, false, false, &args);
        let success = ["SSG", "LRR", "kOmegaSST"].contains(&mode);
        assert_eq!(py.status.success(), success, "{mode}: {py:?}");
        assert_eq!(rs.status.success(), success, "{mode}: {rs:?}");
        if success || mode == "bound" {
            compare(&root)
        } else {
            assert!(!original);
            assert!(!root.join("runs/case1").exists());
        }
    }
}
fn warm(root: &Path) {
    for (name, bytes) in snapshot(&root.join("runs/source/constant/polyMesh")) {
        put(root, &format!("runs/warm/constant/polyMesh/{name}"), bytes);
    }
    put(
        root,
        "runs/warm/carrier-input.json",
        json!({"model":"kOmegaSST"}).to_string(),
    );
    put(
        root,
        "runs/warm/water-1-mpirun.log.receipt.json",
        json!({"exit_code":0}).to_string(),
    );
    for name in ["U", "p", "phi", "k", "omega", "epsilon", "R", "nut"] {
        put(
            root,
            &format!("runs/warm/10/{name}"),
            if name == "p" {
                "internalField uniform 123; boundaryField { overflow {type fixedValue; value uniform -2.0;} }"
            } else {
                "synthetic field"
            },
        );
    }
}
#[test]
fn point_and_warm_start_positive_negative_parity() {
    for mode in [
        "pilot",
        "diagnostic",
        "mesh-fail",
        "existing",
        "outside",
        "warm",
        "warm-failed",
        "warm-model",
        "warm-exit",
        "warm-mesh",
        "warm-missing-field",
        "warm-pressure",
        "warm-time",
        "warm-outside",
    ] {
        let root = setup_carrier();
        let mut args = vec![
            "--mesh-case",
            "runs/source",
            "--reject-pressure-pa",
            "-4567",
        ];
        if mode != "mesh-fail" && mode != "diagnostic" {
            put(
                &root,
                "runs/source/mesh-audit.json",
                json!({"pilot_mesh_pass":true}).to_string(),
            );
        }
        if mode == "diagnostic" {
            args.push("--diagnostic");
        }
        if mode == "existing" {
            put(&root, "runs/case1/retained", "retained");
        }
        if mode == "outside" {
            args.extend(["--case", "outside"]);
        }
        if mode.starts_with("warm") {
            warm(&root);
            args.extend(["--warm-case", "runs/warm"]);
            if mode != "warm-time" {
                args.extend(["--warm-time", "10"]);
            }
            match mode {
                "warm-failed" => put(&root, "runs/warm/numerical-failure.json", "{}"),
                "warm-model" => put(
                    &root,
                    "runs/warm/carrier-input.json",
                    json!({"model":"SSG"}).to_string(),
                ),
                "warm-exit" => put(
                    &root,
                    "runs/warm/water-1-mpirun.log.receipt.json",
                    json!({"exit_code":1}).to_string(),
                ),
                "warm-mesh" => put(&root, "runs/warm/constant/polyMesh/owner", "changed"),
                "warm-missing-field" => fs::rename(
                    root.join("runs/warm/10/phi"),
                    root.join("runs/warm/phi-retained"),
                )
                .unwrap(),
                "warm-pressure" => put(&root, "runs/warm/10/p", "no overflow boundary"),
                "warm-outside" => args.extend(["--warm-case", "bin"]),
                _ => {}
            }
        }
        let py = invoke(&root, true, true, &args);
        let exists = root.join("runs/case1").exists();
        if exists {
            fs::rename(root.join("runs/case1"), root.join("python-case")).unwrap();
        }
        if mode == "existing" {
            put(&root, "runs/case1/retained", "retained");
        }
        let rs = invoke(&root, false, true, &args);
        let success = ["pilot", "diagnostic", "warm"].contains(&mode);
        assert_eq!(py.status.success(), success, "{mode}: {py:?}");
        assert_eq!(rs.status.success(), success, "{mode}: {rs:?}");
        if exists {
            compare(&root)
        } else {
            assert!(!root.join("runs/case1").exists());
        }
        if success {
            assert_eq!(py.stdout, rs.stdout);
        } else {
            assert!(!root.join("runs/case1/input-manifest.json").exists());
        }
    }
}
