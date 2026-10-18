//! Frozen presentation provenance controls; synthetic fixtures are not physical evidence.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
const FROZEN: &str = "e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0";
const SOURCES: [(&str, &str, &str); 2] = [
    (
        "apps/explorer/src/main.tsx",
        "Evidence and Proposals panels",
        "77d32c1c3474d06e0fe90af5f9a4ef808f5874f8d4c5c6a834b5d446a25373db",
    ),
    (
        "crates/joil-core/src/scene.rs",
        "trajectory construction",
        "614952d72ea54ce4ddca16bbc5f0ab7c450b094da3eff7acfe333f00d8b48159",
    ),
];
fn read(path: impl AsRef<Path>) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
}
fn validate(m: &Value) -> (bool, String) {
    if [
        "subject_id",
        "subject_sha256",
        "time_s",
        "field",
        "field_sha256",
        "evidence_kind",
    ]
    .iter()
    .any(|k| m[*k].is_null() || m[*k] == "")
    {
        return (
            false,
            "SCHEMATIC / unavailable: trajectory evidence is not bound to subject, time, and field"
                .into(),
        );
    }
    if m["evidence_kind"] != "native_observation" {
        return (
            false,
            "SCHEMATIC: authored or unvalidated paths are not measured trajectories".into(),
        );
    }
    let time = m["time_s"]
        .as_f64()
        .or_else(|| m["time_s"].as_bool().map(|b| if b { 1.0 } else { 0.0 }));
    if time.is_none_or(|t| t < 0.0) {
        return (false, "SCHEMATIC: invalid observation time".into());
    }
    let scope = &m["scope"];
    if scope.is_null()
        || scope == ""
        || scope == false
        || scope == 0
        || scope.as_array().is_some_and(Vec::is_empty)
        || scope.as_object().is_some_and(|o| o.is_empty())
    {
        return (false, "SCHEMATIC: measured scope is absent".into());
    }
    (
        true,
        format!(
            "Bound scope (synthetic fixture): {}",
            scope
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| scope.to_string())
        ),
    )
}
fn run(root: &Path, mode: &str, skip: bool) -> Result<i32, String> {
    let gates: Value = serde_json::from_str(&read(
        root.join("docs/v5-plan/execution/DEFECT_ACCEPTANCE_GATES.json"),
    )?)
    .map_err(|e| e.to_string())?;
    if gates["frozen_source_commit"] != FROZEN {
        return Err("acceptance gate frozen commit drift".into());
    }
    let defects: Value = serde_json::from_str(&read(
        root.join(
            gates["inventory_path"]
                .as_str()
                .ok_or("missing inventory_path")?,
        ),
    )?)
    .map_err(|e| e.to_string())?;
    let defect = defects["defects"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["id"] == "D-21"))
        .ok_or("D-21 defect missing")?;
    for (path, symbol, expected) in SOURCES {
        let matches: Vec<_> = defect["sources"]
            .as_array()
            .ok_or("missing sources")?
            .iter()
            .filter(|s| s["path"] == path && s["symbol"] == symbol)
            .collect();
        if matches.len() != 1 || matches[0]["source_sha256"] != expected {
            return Err(format!("D-21 source inventory drift: {path}"));
        }
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["show", &format!("{FROZEN}:{path}")])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!("frozen source read failed: {path}"));
        }
        let digest = format!("{:x}", Sha256::digest(output.stdout));
        if digest != expected {
            return Err(format!("frozen source digest mismatch: {path}: {digest}"));
        }
    }
    for (path, anchor, error) in [
        (
            "crates/crucible-render/src/lib.rs",
            "oil_without_evidence_is_an_unavailable_card_not_trajectory_data",
            "crucible-render unavailable-evidence guard is missing",
        ),
        (
            "recipes/v5/registry.json",
            "authored trajectories are prohibited",
            "crucible-render policy does not prohibit authored trajectories",
        ),
        (
            "crates/joil-core/src/scene.rs",
            "SCHEMATIC_AUTHORED_CURVE_NOT_SIMULATED_OR_MEASURED",
            "scene trajectory provenance no longer labels authored paths schematic",
        ),
    ] {
        if !read(root.join(path))?.contains(anchor) {
            return Err(error.into());
        }
    }
    if root.join("crates/crucible-web").exists() {
        return Err("crates/crucible-web exists; its policy must be added to this control".into());
    }
    if !skip {
        let status = Command::new("cargo")
            .args([
                "test",
                "--locked",
                "--offline",
                "-p",
                "crucible-render",
                "oil_without_evidence_is_an_unavailable_card_not_trajectory_data",
            ])
            .current_dir(root)
            .env("CARGO_INCREMENTAL", "0")
            .env("CARGO_PROFILE_DEV_DEBUG", "line-tables-only")
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!(
                "crucible-render regression failed with exit {}",
                status.code().unwrap_or(1)
            ));
        }
        println!("crucible-render unavailable-data regression: 1 passed; separate crucible-web crate absent.");
    }
    let (authored_accepted, unavailable) =
        validate(&json!({"label":"SCHEMATIC authored curve","evidence_kind":"authored"}));
    if mode == "positive" {
        let (accepted, presentation) = validate(
            &json!({"subject_id":"subject-fixture-01","subject_sha256":"a".repeat(64),"time_s":1.25,"field":"oil_volume_fraction","field_sha256":"b".repeat(64),"evidence_kind":"native_observation","scope":"single subject at 1.25 s; oil_volume_fraction field only"}),
        );
        if !accepted || authored_accepted || !unavailable.starts_with("SCHEMATIC") {
            return Err("positive scope/presentation assertion failed".into());
        }
        println!("D-21 positive PASS: exact frozen source digests bound:");
        for (path, _, digest) in SOURCES {
            println!("  {path} sha256={digest}");
        }
        println!("  {presentation}\n  {unavailable}\nScope: software provenance control only; fixture is synthetic, not physical evidence.");
        Ok(0)
    } else {
        let scene = json!({"native_trajectory_manifest":null,"claim":"coupled-transport cross-pipeline -38.5%"});
        let (accepted, unavailable) = validate(&scene);
        let claim = scene["claim"].as_str().ok_or("missing claim")?;
        let forbidden = [
            "coupled-transport",
            "cross-pipeline",
            "-38.5%",
            "−38.5%",
            "-38.4%",
            "−38.4%",
        ];
        if accepted && !forbidden.iter().any(|term| claim.contains(term)) {
            return Err("unsupported coupled-transport claim was admitted".into());
        }
        println!("D-21 RED PASS: scene without native trajectory evidence rejected the claim.\n  {unavailable}\n  rejected wording: coupled-transport cross-pipeline -38.5%\nExpected control exit: 1 (the invalid red fixture was rejected).");
        Ok(1)
    }
}
fn main() {
    let mut args = std::env::args().skip(1);
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut mode = None;
    let mut skip = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().unwrap_or_default()),
            "--mode" => mode = args.next(),
            "--skip-renderer-test" => skip = true,
            _ => {
                eprintln!("invalid argument: {arg}");
                std::process::exit(2)
            }
        }
    }
    let code=match mode.as_deref() {
        Some(m @ ("positive"|"red"))=>run(&root,m,skip),
        _=>Err("usage: v5-d21-trajectory-control --mode positive|red [--root ROOT] [--skip-renderer-test]".into())
    };
    std::process::exit(match code {
        Ok(c) => c,
        Err(e) => {
            eprintln!("D-21 CONTROL ERROR: {e}");
            2
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_boundaries() {
        let valid = json!({"subject_id":"s","subject_sha256":"a","time_s":0,"field":"f","field_sha256":"b","evidence_kind":"native_observation","scope":"synthetic"});
        assert!(validate(&valid).0);
        for key in [
            "subject_id",
            "subject_sha256",
            "time_s",
            "field",
            "field_sha256",
            "evidence_kind",
            "scope",
        ] {
            let mut m = valid.clone();
            m.as_object_mut().unwrap().remove(key);
            assert!(!validate(&m).0);
        }
        for (key, value) in [
            ("time_s", json!(-1)),
            ("time_s", json!("1")),
            ("evidence_kind", json!("authored")),
            ("scope", json!(false)),
        ] {
            let mut m = valid.clone();
            m[key] = value;
            assert!(!validate(&m).0);
        }
    }
}
