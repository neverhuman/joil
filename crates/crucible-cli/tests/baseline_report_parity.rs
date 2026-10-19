use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn put(root: &Path, name: &str, bytes: impl AsRef<[u8]>) {
    let p = root.join(name);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}
fn write(root: &Path, name: &str, value: Value) {
    put(root, name, serde_json::to_vec(&value).unwrap());
}
fn read(root: &Path, name: &str) -> Value {
    serde_json::from_slice(&fs::read(root.join(name)).unwrap()).unwrap()
}
fn hash(root: &Path, name: &str) -> String {
    sha(&fs::read(root.join(name)).unwrap())
}
fn setup() -> PathBuf {
    let root = repo().join(format!(
        "target/i018-baseline-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for name in ["analysis/baseline_report.py", "analysis/evidence.py"] {
        put(&root, name, fs::read(repo().join(name)).unwrap());
    }
    write(
        &root,
        "requirements/RS-joil-research-v1.json",
        json!({"limits":{"pdr_draft_range":[1.,10.]}}),
    );
    write(
        &root,
        "baselines/ct35/genome.json",
        json!({"synthetic":true}),
    );
    put(
        &root,
        "runs/utility/joilHydraulicMetrics",
        b"synthetic native utility",
    );
    put(&root, "runs/case1/input.dat", b"synthetic input");
    write(
        &root,
        "runs/case1/input-manifest.json",
        json!({"input.dat":sha(b"synthetic input")}),
    );
    put(
        &root,
        "runs/case1/snapshot/source-data/flow.dat",
        b"synthetic flow stream",
    );
    write(
        &root,
        "runs/case1/snapshot/snapshot.json",
        json!({"observations":{"pseudo_iteration":100,"port_flow_m3_s":{"inlet":-0.001,"overflow":0.00002,"underflow":0.00098},"feed_m3_h":3.6,"reject_fraction":0.02,"pdr":3.},"evidence":{"requirements_file_sha256":hash(&root,"requirements/RS-joil-research-v1.json"),"input_manifest_sha256":hash(&root,"runs/case1/input-manifest.json"),"captured_streams":{"flow":{"sha256":sha(b"synthetic flow stream")}}},"fitness":{"remaining_qualification_reasons":["synthetic_unqualified","mesh","mesh"]}}),
    );
    write(
        &root,
        "runs/case1/hydraulic-integrals-t100.json",
        json!({"time":100,"patches":{"inlet":{"signed_flow_m3_s":-0.001},"overflow":{"signed_flow_m3_s":0.00002,"reverse_flow_m3_s":0.0},"underflow":{"signed_flow_m3_s":0.00098,"reverse_flow_m3_s":0.0}},"rho_kg_m3":998,"gravity_m_s2":[0,0,0],"hydraulic_loss_w":10.0,"hydraulic_loss_j_per_treated_m3":10000.0}),
    );
    write(
        &root,
        "runs/case1/metrics-binary-t100.json",
        json!({"path":"runs/utility/joilHydraulicMetrics","sha256":sha(b"synthetic native utility")}),
    );
    for name in ["water-1-mpirun", "metrics-t100-0-joilHydraulicMetrics"] {
        put(
            &root,
            &format!("runs/case1/{name}.log"),
            b"synthetic execution\n",
        );
        write(
            &root,
            &format!("runs/case1/{name}.log.receipt.json"),
            json!({"exit_code":0,"log_sha256":sha(b"synthetic execution\n"),"wall_seconds":1.25,"image_id":format!("sha256:{}","a".repeat(64))}),
        );
    }
    write(
        &root,
        "runs/case1/carrier-input.json",
        json!({"model":"synthetic_SST"}),
    );
    root
}
fn invoke(root: &Path, python: bool) -> Output {
    let mut c = if python {
        let mut c = Command::new("python3");
        c.arg("analysis/baseline_report.py");
        c
    } else {
        let mut c = Command::new(env!("CARGO_BIN_EXE_baseline-report"));
        c.arg("--root").arg(root);
        c
    };
    c.current_dir(root)
        .args([
            "--point",
            "runs/case1",
            "snapshot",
            "--point",
            "runs/case1",
            "snapshot",
            "--out",
            "report.json",
        ])
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .unwrap()
}
fn mutate(root: &Path, name: &str, f: impl FnOnce(&mut Value)) {
    let mut v = read(root, name);
    f(&mut v);
    write(root, name, v);
}
#[test]
fn report_stdout_and_json_match_python_with_only_clock_normalized() {
    for variant in ["clean", "backflow", "pdr", "extension"] {
        let root = setup();
        match variant {
            "backflow" => mutate(&root, "runs/case1/hydraulic-integrals-t100.json", |v| {
                v["patches"]["overflow"]["reverse_flow_m3_s"] = json!(0.0001)
            }),
            "pdr" => mutate(&root, "runs/case1/snapshot/snapshot.json", |v| {
                v["observations"]["pdr"] = Value::Null
            }),
            "extension" => {
                write(
                    &root,
                    "runs/case1/domain-extension.json",
                    json!({"length_m":0.12}),
                );
                mutate(&root, "runs/case1/hydraulic-integrals-t100.json", |v| {
                    v["original_device_overflow"] = json!({"synthetic":true})
                });
            }
            _ => {}
        }
        let py = invoke(&root, true);
        assert!(py.status.success(), "{py:?}");
        let mut a = read(&root, "report.json");
        fs::rename(root.join("report.json"), root.join("python-report.json")).unwrap();
        let rs = invoke(&root, false);
        assert!(rs.status.success(), "{rs:?}");
        assert_eq!(py.stdout, rs.stdout);
        let mut b = read(&root, "report.json");
        for v in [&mut a, &mut b] {
            assert!(v["created_utc"].as_str().unwrap().ends_with("+00:00"));
            v.as_object_mut().unwrap().remove("created_utc");
        }
        assert_eq!(a, b);
        assert_eq!(b["qualified_baseline"], false);
        assert_eq!(b["points"][0]["fitness"]["sampled_quality"], "unknown");
        let bytes = fs::read(root.join("report.json")).unwrap();
        assert!(!invoke(&root, false).status.success());
        assert_eq!(fs::read(root.join("report.json")).unwrap(), bytes);
    }
}
#[test]
fn tampered_missing_and_nonphysical_evidence_is_rejected() {
    for mode in [
        "requirements",
        "manifest",
        "input",
        "escape",
        "stream",
        "time",
        "flux",
        "energy",
        "rho",
        "gravity",
        "binary",
        "water_log",
        "metrics_log",
        "failed",
        "duty",
        "split",
        "missing",
        "existing",
    ] {
        let root = setup();
        match mode {
            "requirements" => put(&root, "requirements/RS-joil-research-v1.json", b"{}"),
            "manifest" => put(&root, "runs/case1/input-manifest.json", b"{}"),
            "input" => put(&root, "runs/case1/input.dat", b"changed"),
            "escape" => {
                write(
                    &root,
                    "runs/case1/input-manifest.json",
                    json!({"../utility/joilHydraulicMetrics":sha(b"synthetic native utility")}),
                );
                mutate(&root, "runs/case1/snapshot/snapshot.json", |v| {
                    v["evidence"]["input_manifest_sha256"] =
                        json!(hash(&root, "runs/case1/input-manifest.json"))
                });
            }
            "stream" => put(
                &root,
                "runs/case1/snapshot/source-data/flow.dat",
                b"changed",
            ),
            "time" => mutate(&root, "runs/case1/hydraulic-integrals-t100.json", |v| {
                v["time"] = json!(101)
            }),
            "flux" => mutate(&root, "runs/case1/hydraulic-integrals-t100.json", |v| {
                v["patches"]["inlet"]["signed_flow_m3_s"] = json!(0.001)
            }),
            "energy" => mutate(&root, "runs/case1/hydraulic-integrals-t100.json", |v| {
                v["hydraulic_loss_j_per_treated_m3"] = json!(-1.)
            }),
            "rho" => mutate(&root, "runs/case1/hydraulic-integrals-t100.json", |v| {
                v["rho_kg_m3"] = json!(1000)
            }),
            "gravity" => mutate(&root, "runs/case1/hydraulic-integrals-t100.json", |v| {
                v["gravity_m_s2"] = json!([0, 0, -9.81])
            }),
            "binary" => put(&root, "runs/utility/joilHydraulicMetrics", b"changed"),
            "water_log" => put(&root, "runs/case1/water-1-mpirun.log", b"changed"),
            "metrics_log" => put(
                &root,
                "runs/case1/metrics-t100-0-joilHydraulicMetrics.log",
                b"changed",
            ),
            "failed" => mutate(&root, "runs/case1/water-1-mpirun.log.receipt.json", |v| {
                v["exit_code"] = json!(1)
            }),
            "duty" => mutate(&root, "runs/case1/snapshot/snapshot.json", |v| {
                v["observations"]["feed_m3_h"] = json!(0.)
            }),
            "split" => mutate(&root, "runs/case1/snapshot/snapshot.json", |v| {
                v["observations"]["reject_fraction"] = json!(1.)
            }),
            "missing" => {
                fs::rename(
                    root.join("runs/case1/carrier-input.json"),
                    root.join("retained-carrier.json"),
                )
                .unwrap();
            }
            "existing" => put(&root, "report.json", b"retained"),
            _ => unreachable!(),
        }
        let py = invoke(&root, true);
        let rs = invoke(&root, false);
        assert!(!py.status.success(), "{mode}: {py:?}");
        assert!(!rs.status.success(), "{mode}: {rs:?}");
        if mode == "existing" {
            assert_eq!(fs::read(root.join("report.json")).unwrap(), b"retained");
        } else {
            assert!(!root.join("report.json").exists());
        }
    }
}

#[test]
fn unicode_and_dot_output_path_stdout_matches_python() {
    let root = setup();
    let mut outputs = Vec::new();
    for python in [true, false] {
        let mut cmd = if python {
            let mut c = Command::new("python3");
            c.arg("analysis/baseline_report.py");
            c
        } else {
            let mut c = Command::new(env!("CARGO_BIN_EXE_baseline-report"));
            c.arg("--root").arg(&root);
            c
        };
        let result = cmd
            .current_dir(&root)
            .args([
                "--point",
                "runs/case1",
                "snapshot",
                "--out",
                "./rêport-🌊.json",
            ])
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
        outputs.push(result.stdout);
        fs::rename(
            root.join("rêport-🌊.json"),
            root.join(if python {
                "python-unicode.json"
            } else {
                "rust-unicode.json"
            }),
        )
        .unwrap();
    }
    assert_eq!(outputs[0], outputs[1]);
}
