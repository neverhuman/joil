use serde_json::Value;
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
fn git(root: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    String::from_utf8(o.stdout).unwrap().trim().into()
}
fn setup(name: &str) -> (PathBuf, String) {
    let root = repo().join(format!("target/i017-smoke-{}-{name}", std::process::id()));
    fs::create_dir_all(root.join("checkout/campaigns")).unwrap();
    fs::create_dir_all(root.join(".private/cache")).unwrap();
    fs::create_dir_all(root.join("public-cache")).unwrap();
    fs::create_dir_all(root.join("bin")).unwrap();
    let checkout = root.join("checkout");
    fs::write(
        checkout.join("campaigns/fixture-v5.toml"),
        "workers = 2\nkind = 'synthetic_integer_fixture'\n",
    )
    .unwrap();
    git(&checkout, &["init", "-q"]);
    git(&checkout, &["add", "campaigns"]);
    git(
        &checkout,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "synthetic fixture",
        ],
    );
    git(
        &checkout,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/neverhuman/joil.git",
        ],
    );
    let commit = git(&checkout, &["rev-parse", "HEAD"]);
    let cargo = root.join("bin/cargo");
    fs::write(&cargo,r#"#!/usr/bin/env python3
import json,sys
from pathlib import Path
args=sys.argv[1:]
assert args[:5]==['run','--locked','-p','crucible-cli','--']
a=args[5:]
if a[0]=='run':
    text=Path(a[1]).read_text()
    if 'physical_solver' in text:
        assert a[2]=='--root'
        print('bounded_software_campaign_admission',file=sys.stderr)
        sys.exit(64)
    assert a[1].endswith('one-worker.toml') or a[1]=='campaigns/fixture-v5.toml'
    partial='--until' in a
    print(json.dumps(dict(root='synthetic-root',epochs=1 if partial else 2,accounting=dict(charges=2 if partial else 4,exceeded=False),logical_digest='synthetic-logical',actual_cost_digest='one' if 'workers = 1' in text else 'two')))
elif a==['checkpoint','verify','synthetic-root']:
    print(json.dumps(dict(logical_digest='synthetic-logical',offline=True,physical_eligibility='Unsupported')))
elif a==['status','synthetic-root']:
    print(json.dumps(dict(state='synthetic-complete')))
else: sys.exit(9)
"#).unwrap();
    fs::set_permissions(cargo, fs::Permissions::from_mode(0o755)).unwrap();
    (root, commit)
}
fn run(root: &Path, commit: &str, case: &str, python: bool) -> Output {
    // Root is known non-private and exists; rejection must happen before mkdir.
    let evidence = if case == "evidence" {
        PathBuf::from("/")
    } else {
        root.join(".private/evidence")
    };
    let target = if case == "target" {
        PathBuf::from("/")
    } else {
        root.join(".private/cache")
    };
    let mut c = if python {
        let mut c = Command::new("python3");
        c.arg(repo().join("tools/v5_fixture_smoke.py"));
        c
    } else {
        Command::new(env!("CARGO_BIN_EXE_v5-fixture-smoke"))
    };
    c.arg("--checkout")
        .arg(root.join("checkout"))
        .arg("--evidence")
        .arg(evidence)
        .args([
            "--expected-commit",
            if case == "commit" { "wrong" } else { commit },
        ])
        .env("CARGO_TARGET_DIR", target)
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
fn validation_gates_match_python() {
    for case in [
        "evidence",
        "target",
        "commit",
        "dirty",
        "alternates",
        "origin",
    ] {
        let (root, commit) = setup(case);
        let checkout = root.join("checkout");
        match case {
            "dirty" => fs::write(checkout.join("dirty"), "dirty").unwrap(),
            "alternates" => fs::write(checkout.join(".git/objects/info/alternates"), "").unwrap(),
            "origin" => {
                git(
                    &checkout,
                    &[
                        "remote",
                        "set-url",
                        "origin",
                        "https://example.invalid/other.git",
                    ],
                );
            }
            _ => {}
        }
        let py = run(&root, &commit, case, true);
        let evidence = root.join(".private/evidence");
        if evidence.exists() {
            fs::rename(&evidence, root.join(".private/python-evidence")).unwrap();
        }
        let rs = run(&root, &commit, case, false);
        assert!(!py.status.success(), "{case}: {py:?}");
        assert!(!rs.status.success(), "{case}: {rs:?}");
        let message = String::from_utf8(rs.stderr).unwrap();
        assert!(
            String::from_utf8(py.stderr)
                .unwrap()
                .contains(message.trim()),
            "{case}: {message}"
        );
        assert!(!evidence.join("report.json").exists());
    }
}
fn normalized_inventory(dir: &Path) -> Vec<u8> {
    // Only measured command durations vary. Recompute their one bound inventory entry.
    let mut commands: Value =
        serde_json::from_slice(&fs::read(dir.join("commands.json")).unwrap()).unwrap();
    for c in commands.as_array_mut().unwrap() {
        assert!(c["wall_seconds"].as_f64().unwrap() >= 0.0);
        c["wall_seconds"] = 0.into();
    }
    let raw = serde_json::to_vec(&commands).unwrap();
    let mut encoded = fs::read_to_string(dir.join("inventory.json")).unwrap();
    let inventory: Value = serde_json::from_str(&encoded).unwrap();
    for row in inventory.as_array().unwrap() {
        let file = fs::read(dir.join(row["path"].as_str().unwrap())).unwrap();
        assert_eq!(row["sha256"], format!("{:x}", Sha256::digest(&file)));
        assert_eq!(row["bytes"], file.len());
        if row["path"] == "commands.json" {
            let start = encoded.find("\"path\": \"commands.json\"").unwrap();
            let end = start + encoded[start..].find('}').unwrap();
            let normalized = encoded[start..end]
                .replace(
                    row["sha256"].as_str().unwrap(),
                    &format!("{:x}", Sha256::digest(&raw)),
                )
                .replace(
                    &format!("\"bytes\": {}", file.len()),
                    &format!("\"bytes\": {}", raw.len()),
                );
            encoded.replace_range(start..end, &normalized);
        }
    }
    encoded.into_bytes()
}
#[test]
fn bounded_synthetic_cargo_parity() {
    let (root, commit) = setup("positive");
    let py = run(&root, &commit, "clean", true);
    assert!(py.status.success(), "{py:?}");
    let saved = root.join(".private/python-evidence");
    let evidence = root.join(".private/evidence");
    fs::rename(&evidence, &saved).unwrap();
    let rs = run(&root, &commit, "clean", false);
    assert!(rs.status.success(), "{rs:?}");
    for file in [
        "source.json",
        "worker-capability.json",
        "report.json",
        "one-worker.toml",
        "invalid-physical.toml",
    ] {
        assert_eq!(
            fs::read(saved.join(file)).unwrap(),
            fs::read(evidence.join(file)).unwrap(),
            "{file}"
        );
    }
    assert_eq!(
        normalized_inventory(&saved),
        normalized_inventory(&evidence)
    );
    let mut py_json: Value = serde_json::from_slice(&py.stdout).unwrap();
    let mut rs_json: Value = serde_json::from_slice(&rs.stdout).unwrap();
    for (v, dir) in [(&mut py_json, &saved), (&mut rs_json, &evidence)] {
        assert_eq!(
            v["inventory_sha256"],
            format!(
                "{:x}",
                Sha256::digest(fs::read(dir.join("inventory.json")).unwrap())
            )
        );
        v["inventory_sha256"] = "measured-command-times-normalized".into();
    }
    let py_stdout = String::from_utf8(py.stdout).unwrap();
    let rs_stdout = String::from_utf8(rs.stdout).unwrap();
    let py_hash = format!(
        "{:x}",
        Sha256::digest(fs::read(saved.join("inventory.json")).unwrap())
    );
    let rs_hash = format!(
        "{:x}",
        Sha256::digest(fs::read(evidence.join("inventory.json")).unwrap())
    );
    assert_eq!(
        py_stdout.replace(&py_hash, "normalized-inventory"),
        rs_stdout.replace(&rs_hash, "normalized-inventory")
    );
    assert_eq!(py_json, rs_json);
    assert_eq!(py_json["commands"], 13);
    assert_eq!(py_json["physical_qualification"], false);
}
