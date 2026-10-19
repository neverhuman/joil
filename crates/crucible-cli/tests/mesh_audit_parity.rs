mod mesh_support;
use mesh_support::*;
use serde_json::{json, Value};
use std::{fs, path::Path, process::Command};
// Synthetic polygonal cylinder with an area within 2% of the analytic bore.
fn fixture(root: &Path) {
    let n = 32;
    let mut points = Vec::new();
    for z in [-0.002, 0.] {
        for i in 0..n {
            let a = 2. * std::f64::consts::PI * i as f64 / n as f64;
            points.push(format!("({} {} {z})", 0.0007 * a.cos(), 0.0007 * a.sin()));
        }
    }
    put(
        root,
        "runs/audit/constant/polyMesh/points",
        format!("{}\n(\n{}\n)\n", 2 * n, points.join("\n")),
    );
    let mut faces = Vec::new();
    for i in 0..n {
        faces.push(format!(
            "4({i} {} {} {})",
            (i + 1) % n,
            (i + 1) % n + n,
            i + n
        ));
    }
    faces.push(format!(
        "{n}({})",
        (n..2 * n)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    ));
    put(
        root,
        "runs/audit/constant/polyMesh/faces",
        format!("{}\n(\n{}\n)\n", faces.len(), faces.join("\n")),
    );
    put(
        root,
        "runs/audit/constant/polyMesh/boundary",
        format!("2\n(wall {{nFaces {n}; startFace 0;}} outlet {{nFaces 1; startFace {n};}})"),
    );
    let area = n as f64 * 0.5 * 0.0007f64.powi(2) * (2. * std::f64::consts::PI / n as f64).sin();
    put(root,"runs/audit/mesh-input.json",json!({"analytic_volume_m3":1.,"patches":{"outlet":{"area_m2":area,"normal":[0,0,1],"plane_axis":2,"plane_position_m":0.,"plane_tolerance_m":1e-9}}}).to_string());
    put(root,"runs/audit/mesh-2-checkMesh.log","cells: 100\nTotal volume = 1\nMesh OK.\nMesh non-orthogonality Max: 65\nMax skewness = 4\nCell determinant minimum: 0.001\nFace interpolation weight minimum: 0.05\nFace volume ratio minimum: 0.01\n");
}
fn close(a: &Value, b: &Value) {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let a = a.as_f64().unwrap();
            let b = b.as_f64().unwrap();
            assert!(
                (a - b).abs() <= 1e-15 + 1e-12 * a.abs().max(b.abs()),
                "{a} != {b}"
            )
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len());
            for (k, v) in a {
                close(v, &b[k])
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                close(a, b)
            }
        }
        _ => assert_eq!(a, b),
    }
}
#[test]
fn audit_positive_and_negative_oracle_parity() {
    for no_site in [false, true] {
        for mode in [
            "ok",
            "quality",
            "missing-quality",
            "cells",
            "volume",
            "normal",
            "plane",
            "area",
            "missing-patch",
            "open-loop",
            "point-count",
            "existing",
            "output-path",
        ] {
            let root = setup();
            fixture(&root);
            let case = root.join("runs/audit");
            match mode {
                "quality" | "missing-quality" | "cells" | "volume" => {
                    let p = case.join("mesh-2-checkMesh.log");
                    let s = fs::read_to_string(&p).unwrap();
                    let s = match mode {
                        "quality" => s.replace("Max: 65", "Max: 65.01"),
                        "missing-quality" => {
                            s.replace("Max skewness = 4", "no skewness measurement")
                        }
                        "cells" => s.replace("cells: 100", "cells: 1200001"),
                        _ => s.replace("volume = 1", "volume = 1.021"),
                    };
                    fs::write(p, s).unwrap()
                }
                "normal" | "plane" | "area" | "missing-patch" => {
                    let p = case.join("mesh-input.json");
                    let mut v: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
                    match mode {
                        "normal" => v["patches"]["outlet"]["normal"] = json!([0, 0, -1]),
                        "plane" => v["patches"]["outlet"]["plane_position_m"] = json!(0.1),
                        "area" => v["patches"]["outlet"]["area_m2"] = json!(1.),
                        _ => v["patches"]["missing"] = v["patches"]["outlet"].clone(),
                    }
                    fs::write(p, v.to_string()).unwrap()
                }
                "open-loop" => {
                    let p = case.join("constant/polyMesh/boundary");
                    fs::write(
                        &p,
                        fs::read_to_string(&p)
                            .unwrap()
                            .replace("nFaces 32", "nFaces 31"),
                    )
                    .unwrap()
                }
                "point-count" => {
                    let p = case.join("constant/polyMesh/points");
                    fs::write(&p, fs::read_to_string(&p).unwrap().replacen("64", "65", 1)).unwrap()
                }
                "existing" => put(&root, "runs/audit/mesh-audit.json", "retained"),
                _ => {}
            }
            let name = if mode == "output-path" {
                "../escape.json"
            } else {
                "mesh-audit.json"
            };
            let mut py = Command::new("python3");
            if no_site {
                py.arg("-S");
            }
            // Explicit shim for no-site replay, otherwise prefer installed NumPy.
            if no_site
                || std::env::var_os("JOIL_MESH_FORCE_PYSHIM").is_some()
                || !Command::new("python3")
                    .args(["-c", "import numpy"])
                    .output()
                    .unwrap()
                    .status
                    .success()
            {
                py.env("PYTHONPATH", root.join("pyshim"));
            }
            let py = py
                .current_dir(&root)
                .arg("physics/openfoam/mesh/audit.py")
                .args(["--case", "runs/audit", "--output-name", name])
                .env("PYTHONDONTWRITEBYTECODE", "1")
                .output()
                .unwrap();
            let pyfile = fs::read(case.join("mesh-audit.json")).ok();
            if mode != "existing" && pyfile.is_some() {
                fs::rename(case.join("mesh-audit.json"), case.join("python-audit.json")).unwrap();
            }
            let rs = Command::new(env!("CARGO_BIN_EXE_mesh-audit"))
                .current_dir(&root)
                .args(["--case", "runs/audit", "--output-name", name])
                .output()
                .unwrap();
            assert_eq!(
                py.status.success(),
                mode == "ok",
                "{mode} no_site={no_site}: {py:?}"
            );
            assert_eq!(rs.status.code(), py.status.code(), "{mode}: {rs:?}");
            if let Some(bytes) = pyfile {
                if mode == "existing" {
                    assert_eq!(fs::read(case.join("mesh-audit.json")).unwrap(), bytes)
                } else {
                    let a: Value = serde_json::from_slice(&bytes).unwrap();
                    let b: Value =
                        serde_json::from_slice(&fs::read(case.join("mesh-audit.json")).unwrap())
                            .unwrap();
                    close(&a, &b);
                    close(&a, &serde_json::from_slice(&rs.stdout).unwrap());
                }
            } else {
                assert!(!case.join("mesh-audit.json").exists());
            }
        }
    }
}
