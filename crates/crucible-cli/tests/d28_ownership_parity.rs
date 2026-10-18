use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
#[test]
fn frozen_sources_and_rejected_allocation_match_python() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let scratch = root.join(format!("target/i013-d28-{}", std::process::id()));
    fs::create_dir_all(&scratch).unwrap();
    for case in ["actual", "digest", "missing"] {
        let git = scratch.join("git");
        fs::write(
            &git,
            if case == "digest" {
                "#!/bin/sh\nif [ \"$1\" = cat-file ]; then exit 0; fi\nprintf synthetic\n"
            } else {
                "#!/bin/sh\nexit 128\n"
            },
        )
        .unwrap();
        fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
        for mode in ["positive", "red"] {
            let mut py = Command::new("python3");
            py.arg(root.join("tools/v5_d28_ownership_control.py"));
            let mut rs = Command::new(env!("CARGO_BIN_EXE_v5-d28-ownership-control"));
            for cmd in [&mut py, &mut rs] {
                cmd.args(["--mode", mode]).current_dir(&root);
                if case != "actual" {
                    cmd.env(
                        "PATH",
                        format!("{}:{}", scratch.display(), std::env::var("PATH").unwrap()),
                    );
                }
            }
            let py = py.output().unwrap();
            let rs = rs.output().unwrap();
            assert_eq!(rs.status.code(), py.status.code(), "{case}/{mode}");
            assert_eq!(rs.stdout, py.stdout, "{case}/{mode}");
            assert_eq!(rs.stderr, py.stderr, "{case}/{mode}");
            assert_eq!(
                rs.status.code(),
                Some(if case != "actual" {
                    2
                } else if mode == "red" {
                    1
                } else {
                    0
                })
            );
        }
    }
}
#[test]
fn allocation_policy_positive_and_negative_controls_match_python() {
    // Exercise the Rust policy's unit test directly in its binary test target;
    // this table also pins the Python oracle for the same overlap/lease cases.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out=Command::new("python3").args(["-c", "from tools.v5_d28_ownership_control import allocation_passes as f\nfor ack,lease,paths,others,expected in [(True,True,['a/b'],['a/c'],True),(False,True,['a/b'],['c'],False),(True,False,['a/b'],['c'],False),(True,True,[],['c'],False),(True,True,['a/b'],['a/b'],False),(True,True,['a/b'],['a/'],False),(True,True,['a/'],['a/b'],False)]:\n assert f(dict(target_worker_acknowledged=ack,lease_active=lease,write_scope=paths,other_active_write_scopes=[others])) is expected"]).current_dir(root).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
