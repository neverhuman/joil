use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn executable(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
#[test]
fn frozen_scheduler_and_negative_controls_match_python() {
    let root = repo();
    let scratch = root.join(format!("target/i016-d18-{}", std::process::id()));
    fs::create_dir_all(&scratch).unwrap();
    let source = scratch.join("frozen-source");
    fs::create_dir_all(&source).unwrap();
    let archive = scratch.join("frozen.tar");
    assert!(Command::new("git")
        .current_dir(&root)
        .args([
            "archive",
            "e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0",
            "Cargo.toml",
            "Cargo.lock",
            "crates",
            "baselines",
            "requirements"
        ])
        .arg("-o")
        .arg(&archive)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("tar")
        .arg("-xf")
        .arg(archive)
        .arg("-C")
        .arg(&source)
        .status()
        .unwrap()
        .success());
    // Isolated frozen profile shares this lane target cache, never another checkout.
    let build = Command::new("cargo")
        .current_dir(&source)
        .args([
            "build",
            "--locked",
            "--offline",
            "--config",
            "profile.i016frozen.inherits=\"dev\"",
            "--profile",
            "i016frozen",
            "-p",
            "joil-core",
            "-p",
            "joil-ir",
            "--target-dir",
        ])
        .arg(root.join("target"))
        .env("CARGO_BUILD_JOBS", "2")
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let frozen = root.join("target/i016frozen");
    for case in [
        "positive",
        "red",
        "missing",
        "empty",
        "nonmonotonic",
        "malformed",
        "anchor",
    ] {
        let mode = if case == "positive" || case == "missing" || case == "anchor" {
            "positive"
        } else {
            "red"
        };
        let mut outputs = vec![];
        for python in [true, false] {
            let work = scratch.join(format!("{case}-{python}"));
            fs::create_dir_all(&work).unwrap();
            let mut cmd = if python {
                let mut c = Command::new("python3");
                c.arg(root.join("tools/v5_d18_scheduler_control.py"));
                c
            } else {
                Command::new(env!("CARGO_BIN_EXE_v5-d18-scheduler-control"))
            };
            cmd.arg(mode)
                .arg("--scratch")
                .arg(&work)
                .arg("--frozen-rust")
                .arg(if case == "missing" { &work } else { &frozen });
            if ["empty", "nonmonotonic", "malformed"].contains(&case) {
                // Synthetic compiler double emits a probe with a controlled invalid log.
                let log = match case {
                    "empty" => "",
                    "nonmonotonic" => "1 2\\n2 2\\n",
                    _ => "broken\\n",
                };
                let compiler = work.join("rustc-double");
                executable(&compiler,&format!("#!/bin/sh\nfor arg do out=\"$arg\"; done\ncat > \"$out\" <<'EOF'\n#!/bin/sh\nprintf '{log}' > \"$D18_LOG\"\nexit 1\nEOF\nchmod +x \"$out\"\n"));
                cmd.env("RUSTC", compiler);
            }
            if case == "anchor" {
                let git = Command::new("git")
                    .current_dir(&root)
                    .args([
                        "show",
                        "e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0:crates/joil-core/src/evolve.rs",
                    ])
                    .output()
                    .unwrap();
                assert!(git.status.success());
                fs::write(
                    work.join("source"),
                    String::from_utf8(git.stdout).unwrap().replace(
                        "for (isl, pop) in islands.iter_mut().enumerate()",
                        "tampered loop",
                    ),
                )
                .unwrap();
                executable(
                    &work.join("git"),
                    "#!/bin/sh\ncat \"$(dirname \"$0\")/source\"\n",
                );
                cmd.env(
                    "PATH",
                    format!("{}:{}", work.display(), std::env::var("PATH").unwrap()),
                );
            }
            let out = cmd.output().unwrap();
            assert_eq!(
                out.status.code(),
                Some(if case == "positive" {
                    0
                } else if case == "red" {
                    1
                } else {
                    2
                }),
                "{case}/{python}: {out:?}"
            );
            let stdout = String::from_utf8(out.stdout)
                .unwrap()
                .replace(&work.display().to_string(), "SCRATCH");
            let stderr = String::from_utf8(out.stderr)
                .unwrap()
                .replace(&work.display().to_string(), "SCRATCH");
            outputs.push((stdout, stderr));
            if case == "red" && !python {
                let script = fs::read_to_string(work.join("fake_recording_evaluator")).unwrap();
                assert!(!script.contains("python"));
                assert!(script.contains("sleep 0.001"));
            }
        }
        assert_eq!(outputs[0], outputs[1], "{case}");
    }
}
