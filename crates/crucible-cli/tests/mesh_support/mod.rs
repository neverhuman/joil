#![allow(dead_code)]
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
pub fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn put(root: &Path, name: &str, bytes: impl AsRef<[u8]>) {
    let p = root.join(name);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}
pub fn snapshot(path: &Path) -> Vec<(String, Vec<u8>)> {
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
pub fn setup() -> PathBuf {
    let root = repo().join(format!(
        "target/i019-mesh-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for name in [
        "physics/openfoam/mesh/reference_plane.py",
        "physics/openfoam/mesh/extend_outlet.py",
        "physics/openfoam/mesh/audit.py",
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
        "runs/source/constant/polyMesh/points",
        b"/* synthetic */ FoamFile { format ascii; }\n4\n(\n(0 0 0) (1 0 0) (0 1 0) (0 0 -1)\n)\n",
    );
    put(
        &root,
        "runs/source/constant/polyMesh/faces",
        b"FoamFile {format ascii;}\n2\n(\n3(0 1 3) // wall\n3(0 1 2)\n)\n",
    );
    put(
        &root,
        "runs/source/constant/polyMesh/neighbour",
        b"0\n(\n)\n",
    );
    put(&root, "runs/source/constant/polyMesh/owner", b"2\n(0 0)\n");
    put(&root,"runs/source/constant/polyMesh/boundary",b"2\n(\nwall {type wall; nFaces 1; startFace 0;}\noverflow {type patch; nFaces 1; startFace 1;}\n)\n");
    put(
        &root,
        "runs/source/system/controlDict",
        b"synthetic original control\n",
    );
    put(
        &root,
        "runs/source/mesh-audit.json",
        b"{\"pilot_mesh_pass\":false,\"patches\":{\"overflow\":{\"area_m2\":0.5}}}\n",
    );
    put(&root,"runs/source/mesh-input.json",b"{\"analytic_volume_m3\":1.0,\"patches\":{\"overflow\":{\"plane_position_m\":0.0}},\"synthetic\":true}\n");
    put(
        &root,
        "bin/docker",
        r#"#!/usr/bin/env bash
set -eu
args=("$@")
[[ ${args[0]} == run && ${args[1]} == --name && ${args[2]} == joil-* ]]
[[ ${args[3]} == --rm && ${args[4]} == --init && ${args[5]} == --network=none && ${args[6]} == --read-only ]]
[[ ${args[7]} == --cpus=4 && ${args[8]} == --cpuset-cpus=0,1,2,3 ]]
[[ ${args[9]} == --memory=12g && ${args[10]} == --memory-swap=12g && ${args[11]} == --pids-limit=256 ]]
[[ ${args[12]} == --cap-drop=ALL && ${args[13]} == --security-opt=no-new-privileges ]]
[[ ${args[14]} == --user && ${args[15]} == "$(id -u):$(id -g)" ]]
[[ ${args[16]} == --tmpfs && ${args[17]} == /tmp:rw,nosuid,nodev,size=256m,mode=1777 ]]
[[ ${args[18]} == -e && ${args[19]} == HOME=/tmp && ${args[20]} == -e && ${args[21]} == OMP_NUM_THREADS=1 ]]
[[ ${args[22]} == -e && ${args[23]} == OPENBLAS_NUM_THREADS=1 && ${args[24]} == -v && ${args[25]} == "$TEST_ROOT:/work:ro" ]]
[[ ${args[26]} == -v && ${args[27]} == "$TEST_ROOT/runs/case1:/case:rw" && ${args[28]} == -w && ${args[29]} == /case ]]
[[ ${args[30]} == sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ]]
printf '%s\n' "${args[@]:31}"
if [[ ${args[31]} == extrudeMesh ]]; then
  if [[ ${MESH_MODE:-ok} == nan ]]; then
    printf '4\n((nan 0 0) (1 0 0) (0 1 0) (0 0 -1))\n' > "$TEST_ROOT/runs/case1/constant/polyMesh/points"
  elif [[ ${MESH_MODE:-ok} == changed ]]; then
    printf '1\n((9 9 9))\n' > "$TEST_ROOT/runs/case1/constant/polyMesh/points"
  else
    printf '5\n((0 0 -1) (0 1 0) (1 0 0) (0 0 0) (0 0 -2))\n' > "$TEST_ROOT/runs/case1/constant/polyMesh/points"
  fi
fi
[[ ${MESH_MODE:-ok} != docker ]]
"#,
    );
    fs::set_permissions(root.join("bin/docker"), fs::Permissions::from_mode(0o755)).unwrap();
    root
}
pub fn invoke(root: &Path, python: bool, tool: &str, args: &[&str], mode: &str) -> Output {
    let mut cmd = if python {
        let mut c = Command::new("python3");
        c.arg(format!("physics/openfoam/mesh/{tool}.py"));
        c
    } else {
        let bin = match tool {
            "reference_plane" => env!("CARGO_BIN_EXE_mesh-reference-plane"),
            "extend_outlet" => env!("CARGO_BIN_EXE_mesh-extend-outlet"),
            _ => panic!("unknown tool"),
        };
        let mut c = Command::new(bin);
        c.arg("--root").arg(root);
        c
    };
    cmd.current_dir(root)
        .args(["--case", "runs/case1", "--source", "runs/source"])
        .args(args)
        .env(
            "PATH",
            format!(
                "{}:{}",
                root.join("bin").display(),
                std::env::var("PATH").unwrap()
            ),
        )
        .env("TEST_ROOT", root)
        .env("JOIL_CPU_SET", "0,1,2,3")
        .env("MESH_MODE", mode)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .unwrap()
}
pub fn compare_cases(root: &Path) {
    let a = snapshot(&root.join("python-case"));
    let b = snapshot(&root.join("runs/case1"));
    assert_eq!(a.len(), b.len());
    let mut delta = 0i64;
    for ((an, av), (bn, bv)) in a.iter().zip(&b) {
        assert_eq!(an, bn);
        if an.ends_with(".log.receipt.json") {
            let mut a: Value = serde_json::from_slice(av).unwrap();
            let mut b: Value = serde_json::from_slice(bv).unwrap();
            assert_eq!(
                b["max_observed_shared_output_bytes"].as_i64().unwrap()
                    - a["max_observed_shared_output_bytes"].as_i64().unwrap(),
                delta
            );
            b["max_observed_shared_output_bytes"] = a["max_observed_shared_output_bytes"].clone();
            delta += bv.len() as i64 - av.len() as i64;
            for v in [&mut a, &mut b] {
                assert!(v["wall_seconds"].as_f64().unwrap() >= 0.);
                v.as_object_mut().unwrap().remove("wall_seconds");
            }
            assert_eq!(a, b, "{an}");
        } else {
            assert_eq!(av, bv, "{an}");
        }
    }
}
