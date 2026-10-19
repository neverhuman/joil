use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
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
fn setup() -> PathBuf {
    let root = repo().join(format!(
        "target/i018-bench-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for name in [
        "tools/bench_cfd.py",
        "physics/openfoam/environment/runtime.py",
    ] {
        put(&root, name, fs::read(repo().join(name)).unwrap());
    }
    put(
        &root,
        "physics/openfoam/environment/lock.json",
        json!({"image_id":format!("sha256:{}","a".repeat(64))}).to_string(),
    );
    put(
        &root,
        "runs/case1/constant/polyMesh/boundary",
        b"wall\n{\n type patch;\n}\ninlet { type patch; }\n",
    );
    put(
        &root,
        "runs/utility/joilHydraulicMetrics",
        b"synthetic utility",
    );
    put(
        &root,
        "bin/docker",
        r#"#!/usr/bin/env bash
set -eu
args=("$@")
[[ ${args[0]} == run && ${args[1]} == --name && ${args[2]} == joil-* ]]
[[ ${args[3]} == --rm && ${args[4]} == --init && ${args[5]} == --network=none && ${args[6]} == --read-only ]]
[[ ${args[7]} == --cpus=4 && ${args[8]} == --cpuset-cpus="$JOIL_CPU_SET" ]]
[[ ${args[9]} == --memory=12g && ${args[10]} == --memory-swap=12g && ${args[11]} == --pids-limit=256 ]]
[[ ${args[12]} == --cap-drop=ALL && ${args[13]} == --security-opt=no-new-privileges ]]
[[ ${args[14]} == --user && ${args[15]} == "$(id -u):$(id -g)" ]]
[[ ${args[16]} == --tmpfs && ${args[17]} == /tmp:rw,nosuid,nodev,size=256m,mode=1777 ]]
[[ ${args[18]} == -e && ${args[19]} == HOME=/tmp && ${args[20]} == -e && ${args[21]} == OMP_NUM_THREADS=1 ]]
[[ ${args[22]} == -e && ${args[23]} == OPENBLAS_NUM_THREADS=1 && ${args[24]} == -v && ${args[25]} == "$TEST_ROOT:/work:ro" ]]
[[ ${args[26]} == -v && ${args[27]} == "$TEST_ROOT/runs/case1:/case:rw" && ${args[28]} == -w && ${args[29]} == /case ]]
[[ ${args[30]} == sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ]]
printf '%s\n' "${args[@]:31}"
if [[ ${args[31]} == */joilHydraulicMetrics ]]; then
  printf '{"time": %s}\n' "${NATIVE_TIME:-100}" > "$TEST_ROOT/runs/case1/hydraulic-integrals.json"
fi
[[ ${FAIL_DOCKER:-0} == 0 ]]
"#,
    );
    fs::set_permissions(root.join("bin/docker"), fs::Permissions::from_mode(0o755)).unwrap();
    root
}
fn invoke(root: &Path, python: bool, args: &[&str], cpus: &str, fail: bool) -> Output {
    let mut cmd = if python {
        let mut c = Command::new("python3");
        c.arg("tools/bench_cfd.py");
        c
    } else {
        let mut c = Command::new(env!("CARGO_BIN_EXE_bench-cfd"));
        c.arg(args[0]).arg("--root").arg(root);
        c
    };
    cmd.current_dir(root)
        .args(if python { args } else { &args[1..] })
        .args(["--case", "runs/case1"])
        .env(
            "PATH",
            format!(
                "{}:{}",
                root.join("bin").display(),
                std::env::var("PATH").unwrap()
            ),
        )
        .env("TEST_ROOT", root)
        .env("JOIL_CPU_SET", cpus)
        .env("FAIL_DOCKER", if fail { "1" } else { "0" })
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .unwrap()
}
fn snapshot(path: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                out.push((
                    p.strip_prefix(base).unwrap().to_string_lossy().into_owned(),
                    fs::read(p).unwrap(),
                ));
            }
        }
    }
    let mut out = Vec::new();
    walk(path, path, &mut out);
    out.sort();
    out
}
#[test]
fn stage_logs_receipts_and_boundary_match_python() {
    for args in [
        vec!["centres", "--time", "100"],
        vec!["tet"],
        vec!["mesh"],
        vec!["water"],
        vec!["reconstruct", "--time", "0.000001234567"],
        vec!["build-metrics"],
        vec![
            "metrics",
            "--time",
            "100",
            "--metrics-bin",
            "runs/utility/joilHydraulicMetrics",
        ],
    ] {
        let root = setup();
        if args[0] == "metrics" {
            put(
                &root,
                "runs/case1/constant/polyMesh/sets/deviceOverflow",
                b"faces",
            );
            put(
                &root,
                "runs/case1/device-plane.json",
                json!({"face_set_sha256":sha(b"faces")}).to_string(),
            );
        }
        let original = snapshot(&root.join("runs/case1"));
        let py = invoke(&root, true, &args, "0,1,2,3", false);
        assert!(py.status.success(), "{args:?}: {py:?}");
        fs::rename(root.join("runs/case1"), root.join("python-case")).unwrap();
        for (name, bytes) in &original {
            put(&root, &format!("runs/case1/{name}"), bytes);
        }
        let rs = invoke(&root, false, &args, "0,1,2,3", false);
        assert!(rs.status.success(), "{args:?}: {rs:?}");
        let a = snapshot(&root.join("python-case"));
        let b = snapshot(&root.join("runs/case1"));
        assert_eq!(a.len(), b.len());
        let mut receipt_byte_delta = 0i64;
        // Execution order matters for measured shared size (mesh and water have multiple stages).
        let mut receipts = Vec::new();
        for ((an, av), (bn, bv)) in a.iter().zip(&b) {
            assert_eq!(an, bn);
            if an.ends_with(".log.receipt.json") {
                receipts.push((an, av, bv));
            } else {
                assert_eq!(av, bv, "{an}");
            }
        }
        receipts.sort_by_key(|(n, _, _)| {
            if n.starts_with("tet-") {
                "0".to_string()
            } else if n == &"mesh-2-checkMesh.log.receipt.json" && args[0] == "tet" {
                "1".to_string()
            } else {
                (*n).clone()
            }
        });
        for (name, av, bv) in receipts {
            let mut a: Value = serde_json::from_slice(av).unwrap();
            let mut b: Value = serde_json::from_slice(bv).unwrap();
            assert_eq!(
                b["max_observed_shared_output_bytes"].as_i64().unwrap()
                    - a["max_observed_shared_output_bytes"].as_i64().unwrap(),
                receipt_byte_delta,
                "{name}"
            );
            // Only preceding wall_seconds serialization changes the measured byte count.
            b["max_observed_shared_output_bytes"] = a["max_observed_shared_output_bytes"].clone();
            receipt_byte_delta += bv.len() as i64 - av.len() as i64;
            for v in [&mut a, &mut b] {
                assert!(v["wall_seconds"].as_f64().unwrap() >= 0.);
                v.as_object_mut().unwrap().remove("wall_seconds");
            }
            assert_eq!(a, b, "{name}");
        }
        let again = invoke(&root, false, &args, "0,1,2,3", false);
        assert!(!again.status.success());
    }
}
#[test]
fn rejected_inputs_and_failed_execution_match_python() {
    for mode in [
        "image",
        "cpu",
        "expired",
        "unallocated",
        "cap",
        "binary",
        "plane",
        "boundary",
        "docker",
        "existing",
        "time",
    ] {
        let root = setup();
        let mut args = vec!["centres", "--time", "100"];
        let mut cpus = "0,1,2,3";
        match mode {
            "image" => put(
                &root,
                "physics/openfoam/environment/lock.json",
                b"{\"image_id\":\"mutable\"}",
            ),
            "cpu" => cpus = "0,0,1,2",
            "expired" | "unallocated" => {
                cpus = "4,5,6,7";
                put(&root,".private/coordination/resource-lease.json",json!({"expires_utc":if mode=="expired" {"2000-01-01T00:00:00+00:00"} else {"2999-01-01T00:00:00+00:00"},"leases":[]}).to_string());
            }
            "cap" => {
                let f = fs::File::create(root.join("runs/large-sparse")).unwrap();
                f.set_len(5 * 1024u64.pow(3) + 1).unwrap();
            }
            "binary" => args = vec!["metrics", "--metrics-bin", "bin/docker"],
            "plane" | "time" => {
                args = vec![
                    "metrics",
                    "--time",
                    "101",
                    "--metrics-bin",
                    "runs/utility/joilHydraulicMetrics",
                ];
                if mode == "plane" {
                    put(
                        &root,
                        "runs/case1/device-plane.json",
                        b"{\"face_set_sha256\":\"wrong\"}",
                    );
                    put(
                        &root,
                        "runs/case1/constant/polyMesh/sets/deviceOverflow",
                        b"faces",
                    );
                }
            }
            "boundary" => {
                args = vec!["tet"];
                put(
                    &root,
                    "runs/case1/constant/polyMesh/boundary",
                    b"inlet { type patch; }",
                );
            }
            "existing" => put(
                &root,
                "runs/case1/centres-t100-0-foamPostProcess.log",
                b"retained",
            ),
            _ => {}
        }
        let original = snapshot(&root.join("runs/case1"));
        let py = invoke(&root, true, &args, cpus, mode == "docker");
        assert!(!py.status.success(), "{mode}");
        fs::rename(root.join("runs/case1"), root.join("python-case")).unwrap();
        for (name, bytes) in original {
            put(&root, &format!("runs/case1/{name}"), bytes);
        }
        let rs = invoke(&root, false, &args, cpus, mode == "docker");
        assert!(!rs.status.success(), "{mode}: {rs:?}");
        if mode == "docker" {
            let receipt: Value = serde_json::from_slice(
                &fs::read(root.join("runs/case1/centres-t100-0-foamPostProcess.log.receipt.json"))
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(receipt["exit_code"], 1);
        }
    }
}

#[test]
fn explicit_live_lease_and_immutable_metrics_collisions() {
    for mode in [
        "lease",
        "metadata",
        "native",
        "target",
        "two_walls",
        "symlink",
    ] {
        let root = setup();
        let mut args = vec![
            "metrics",
            "--metrics-bin",
            "runs/utility/joilHydraulicMetrics",
            "--time",
            "100",
        ];
        let mut cpus = "0,1,2,3";
        match mode {
            "lease" => {
                args = vec!["centres"];
                cpus = "4,5,6,7";
                put(&root, ".private/coordination/resource-lease.json", json!({"expires_utc":"2999-01-01T00:00:00+00:00","leases":[{"cpus":[4,5,6,7]}]}).to_string());
            }
            "metadata" => put(&root, "runs/case1/metrics-binary-t100.json", b"retained"),
            "native" => put(&root, "runs/case1/hydraulic-integrals.json", b"retained"),
            "target" => put(
                &root,
                "runs/case1/hydraulic-integrals-t100.json",
                b"retained",
            ),
            "two_walls" => {
                args = vec!["tet"];
                put(
                    &root,
                    "runs/case1/constant/polyMesh/boundary",
                    b"wall { type patch; } wall { type patch; }",
                );
            }
            "symlink" => {
                std::os::unix::fs::symlink(root.join("bin/docker"), root.join("runs/utility/link"))
                    .unwrap();
                args = vec!["metrics", "--metrics-bin", "runs/utility/link"];
            }
            _ => unreachable!(),
        }
        let original = snapshot(&root.join("runs/case1"));
        let py = invoke(&root, true, &args, cpus, false);
        fs::rename(root.join("runs/case1"), root.join("python-case")).unwrap();
        for (name, bytes) in original {
            put(&root, &format!("runs/case1/{name}"), bytes);
        }
        let rs = invoke(&root, false, &args, cpus, false);
        assert_eq!(py.status.success(), mode == "lease", "{mode}: {py:?}");
        assert_eq!(rs.status.success(), py.status.success(), "{mode}: {rs:?}");
        if mode == "lease" {
            let name = "centres-0-foamPostProcess.log.receipt.json";
            let mut a: Value =
                serde_json::from_slice(&fs::read(root.join("python-case").join(name)).unwrap())
                    .unwrap();
            let mut b: Value =
                serde_json::from_slice(&fs::read(root.join("runs/case1").join(name)).unwrap())
                    .unwrap();
            for v in [&mut a, &mut b] {
                v.as_object_mut().unwrap().remove("wall_seconds");
            }
            assert_eq!(a, b);
            assert_eq!(
                a["resource_lease_id"],
                sha(&fs::read(root.join(".private/coordination/resource-lease.json")).unwrap())
            );
        }
    }
}
