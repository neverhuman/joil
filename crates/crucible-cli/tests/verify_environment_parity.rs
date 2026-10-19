use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn digest(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn setup(case: &str) -> PathBuf {
    let root = repo().join(format!(
        "target/i017-environment-{}-{case}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("bin")).unwrap();
    let env = root.join("physics/openfoam/environment");
    fs::create_dir_all(&env).unwrap();
    for name in ["Dockerfile", "openfoam.asc"] {
        fs::write(env.join(name), name).unwrap();
    }
    fs::copy(
        repo().join("physics/openfoam/environment/runtime.py"),
        env.join("runtime.py"),
    )
    .unwrap();
    let lock = json!({"image_id":format!("sha256:{}","a".repeat(64)),"build":"14-test","package_version":"14.0","dockerfile_sha256":digest(b"Dockerfile"),"repository_key_sha256":digest(b"openfoam.asc")});
    fs::write(env.join("lock.json"), serde_json::to_vec(&lock).unwrap()).unwrap();
    let docker = root.join("bin/docker");
    fs::write(&docker,r#"#!/usr/bin/env python3
import json,os,sys
from pathlib import Path
args=sys.argv[1:]
image='sha256:'+'a'*64
case=os.environ['CONTROL']
if args[:2]==['image','inspect']:
    assert args==['image','inspect',image]
    print(json.dumps([dict(Id=image,Architecture='arm64' if case=='architecture' else 'amd64',Os='linux')]))
else:
    root=os.environ['TEST_ROOT']
    prefix=['run','--name',args[2],'--rm','--init','--network=none','--read-only','--cpus=4','--cpuset-cpus=0,1,2,3','--memory=12g','--memory-swap=12g','--pids-limit=256','--cap-drop=ALL','--security-opt=no-new-privileges','--user',str(os.getuid())+':'+str(os.getgid()),'--tmpfs','/tmp:rw,nosuid,nodev,size=256m,mode=1777','-e','HOME=/tmp','-e','OMP_NUM_THREADS=1','-e','OPENBLAS_NUM_THREADS=1','-v',root+':/work:ro','-v',root+'/runs/case:/case:rw','-w','/case',image]
    assert args[:len(prefix)]==prefix,args
    probe=args[len(prefix):]
    if probe==['foamRun','-help']: print('wrong' if case=='build' else '14-test')
    elif probe==['g++','--version']: print('g++ synthetic 1.0')
    elif probe==['mpirun','--version']: print('mpirun synthetic 1.0')
    elif probe==['dpkg-query','-W','-f=${Package}\t${Version}\n']: print('openfoam14\t'+('wrong' if case=='package' else '14.0'))
    else: sys.exit(9)
"#).unwrap();
    fs::set_permissions(docker, fs::Permissions::from_mode(0o755)).unwrap();
    root
}
fn command(root: &Path, case: &str, python: bool) -> Output {
    let path = if case == "outside" {
        root.join("outside")
    } else {
        root.join("runs/case")
    };
    let mut cmd = if python {
        let mut c = Command::new("python3");
        c.current_dir(repo()).args(["-c","import sys;from pathlib import Path;from tools import verify_environment as m;import runtime;m.ROOT=Path(sys.argv[1]);m.ENV=m.ROOT/'physics/openfoam/environment';runtime.ROOT=m.ROOT;runtime.LOCK=m.ENV/'lock.json';m.doctor(sys.argv[2])"]);
        c.arg(root).arg(&path);
        c
    } else {
        let mut c = Command::new(env!("CARGO_BIN_EXE_verify-environment"));
        c.arg("--root")
            .arg(root)
            .arg("--case")
            .arg(&path)
            .arg("--doctor-source")
            .arg(repo().join("tools/verify_environment.py"));
        c
    };
    cmd.env("CONTROL", case)
        .env("TEST_ROOT", root)
        .env("JOIL_CPU_SET", "0,1,2,3")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env(
            "PATH",
            format!(
                "{}:{}",
                root.join("bin").display(),
                std::env::var("PATH").unwrap()
            ),
        )
        .output()
        .unwrap()
}
#[test]
fn environment_positive_and_tamper_parity() {
    for case in [
        "clean",
        "hash",
        "architecture",
        "outside",
        "existing",
        "build",
        "package",
    ] {
        let root = setup(case);
        if case == "hash" {
            fs::write(
                root.join("physics/openfoam/environment/Dockerfile"),
                "tamper",
            )
            .unwrap();
        }
        if case == "existing" {
            fs::create_dir_all(root.join("runs/case")).unwrap();
        }
        let py = command(&root, case, true);
        if root.join("runs/case").exists() && case != "existing" {
            fs::rename(root.join("runs/case"), root.join("python-case")).unwrap();
        }
        let rs = command(&root, case, false);
        assert_eq!(py.status.success(), case == "clean", "{case}: {py:?}");
        assert_eq!(rs.status.success(), py.status.success(), "{case}: {rs:?}");
        if case == "clean" {
            for (output, directory) in [(&py, "python-case"), (&rs, "runs/case")] {
                let stdout = std::str::from_utf8(&output.stdout).unwrap();
                let json_start = stdout.find("{\n").unwrap();
                assert_eq!(
                    &stdout.as_bytes()[json_start..],
                    fs::read(root.join(directory).join("capability.json")).unwrap()
                );
            }
            let mut py: Value = serde_json::from_slice(
                &fs::read(root.join("python-case/capability.json")).unwrap(),
            )
            .unwrap();
            let mut rs: Value =
                serde_json::from_slice(&fs::read(root.join("runs/case/capability.json")).unwrap())
                    .unwrap();
            for v in [&mut py, &mut rs] {
                for c in v["commands"].as_array_mut().unwrap() {
                    assert!(c["wall_seconds"].as_f64().unwrap() >= 0.0);
                    assert!(c["max_observed_shared_output_bytes"].is_u64());
                    c.as_object_mut().unwrap().remove("wall_seconds");
                    c.as_object_mut()
                        .unwrap()
                        .remove("max_observed_shared_output_bytes");
                }
            }
            assert_eq!(py, rs);
            for name in ["solver", "compiler", "mpi", "packages"] {
                assert_eq!(
                    fs::read(root.join(format!("python-case/{name}.log"))).unwrap(),
                    fs::read(root.join(format!("runs/case/{name}.log"))).unwrap()
                );
            }
        } else {
            assert!(!root.join("runs/case/capability.json").exists());
        }
    }
}
