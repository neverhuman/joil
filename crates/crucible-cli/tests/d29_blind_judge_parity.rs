use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
#[test]
fn frozen_sources_and_blind_judge_match_python() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let scratch = root.join(format!("target/i014-d29-{}", std::process::id()));
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
            py.arg(root.join("tools/v5_d29_blind_judge_control.py"));
            let mut rs = Command::new(env!("CARGO_BIN_EXE_v5-d29-blind-judge-control"));
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
fn resolved_graph_adversarial_controls_match_python() {
    use serde_json::json;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let scratch = root.join(format!("target/i014-d29-graph-{}", std::process::id()));
    fs::create_dir_all(&scratch).unwrap();
    let cargo = scratch.join("cargo");
    for case in [
        "clean",
        "indirect",
        "direct",
        "cycle",
        "missing-judge",
        "missing-id",
        "cargo-failure",
    ] {
        let mut packages = vec![];
        let mut nodes = vec![];
        for name in [
            "pack-hydrocyclone-judges",
            "bridge",
            "crucible-genome",
            "crucible-search",
        ] {
            if case == "missing-judge" && name == "pack-hydrocyclone-judges" {
                continue;
            }
            let deps = match (case, name) {
                ("indirect" | "cycle", "pack-hydrocyclone-judges") => vec!["bridge"],
                ("indirect", "bridge") => vec!["crucible-genome"],
                ("cycle", "bridge") => vec!["pack-hydrocyclone-judges"],
                ("direct", "pack-hydrocyclone-judges") => {
                    vec!["crucible-search", "crucible-genome"]
                }
                ("missing-id", "bridge") => vec!["absent"],
                _ => vec![],
            };
            packages.push(json!({"id":name,"name":name}));
            nodes.push(
                json!({"id":name,"deps":deps.iter().map(|n|json!({"pkg":n})).collect::<Vec<_>>()}),
            );
        }
        let data = json!({"packages":packages,"resolve":{"nodes":nodes}});
        fs::write(
            &cargo,
            if case == "cargo-failure" {
                "#!/bin/sh\nexit 101\n".to_string()
            } else {
                format!("#!/bin/sh\ncat <<'JSON'\n{data}\nJSON\n")
            },
        )
        .unwrap();
        fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();
        for mode in ["positive", "red"] {
            let mut py = Command::new("python3");
            py.arg(root.join("tools/v5_d29_blind_judge_control.py"));
            let mut rs = Command::new(env!("CARGO_BIN_EXE_v5-d29-blind-judge-control"));
            for c in [&mut py, &mut rs] {
                c.args(["--mode", mode]).current_dir(&root).env(
                    "PATH",
                    format!("{}:{}", scratch.display(), std::env::var("PATH").unwrap()),
                );
            }
            let py = py.output().unwrap();
            let rs = rs.output().unwrap();
            assert_eq!(rs.status.code(), py.status.code(), "{case}/{mode}");
            assert_eq!(rs.stdout, py.stdout, "{case}/{mode}");
            assert_eq!(rs.stderr, py.stderr, "{case}/{mode}");
            assert_eq!(
                rs.status.code(),
                Some(
                    if ["missing-id", "missing-judge", "cargo-failure"].contains(&case)
                        || (mode == "positive" && ["direct", "indirect"].contains(&case))
                    {
                        2
                    } else if mode == "red" {
                        1
                    } else {
                        0
                    }
                )
            );
        }
    }
}
