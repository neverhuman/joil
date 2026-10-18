//! Frozen D-29 bindings and transitive blind-judge dependency control.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
const BASELINE: &str = "e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0";
const SOURCES: [(&str, &str); 2] = [
    (
        "crates/joil-core/src/evolve.rs",
        "62cb5c8c30186155d09fb816a0e42ad7da615738cfdc11cb27348a16f94778ab",
    ),
    (
        "crates/joil-ir/src/lib.rs",
        "8926748a21966751dc40d12a7ab4227b294cb7dd05b6ec9a9b7949aca1be43ec",
    ),
];
const MANIFESTS: [&str; 5] = [
    "packs/hydrocyclone/judges/Cargo.toml",
    "crates/crucible-fea/Cargo.toml",
    "crates/crucible-lbm/Cargo.toml",
    "crates/crucible-particles/Cargo.toml",
    "crates/crucible-architecture/Cargo.toml",
];
fn git(args: &[&str], capture: bool) -> Result<Vec<u8>, String> {
    let out = Command::new("git")
        .args(args)
        .stderr(Stdio::inherit())
        .stdout(if capture {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        let argv = std::iter::once("git")
            .chain(args.iter().copied())
            .map(|s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "Command '[{argv}]' returned non-zero exit status {}.",
            out.status.code().unwrap_or(1)
        ));
    }
    Ok(out.stdout)
}
const JUDGE: &str = "pack-hydrocyclone-judges";
type Graph = BTreeMap<String, BTreeSet<String>>;
fn forbidden_path(graph: &Graph) -> Option<Vec<String>> {
    let mut pending = vec![(JUDGE.to_string(), vec![JUDGE.to_string()])];
    let mut visited = BTreeSet::new();
    while let Some((package, path)) = pending.pop() {
        if ["crucible-genome", "crucible-search"].contains(&package.as_str()) {
            return Some(path);
        }
        if !visited.insert(package.clone()) {
            continue;
        }
        if let Some(children) = graph.get(&package) {
            for child in children {
                let mut next = path.clone();
                next.push(child.clone());
                pending.push((child.clone(), next));
            }
        }
    }
    None
}
fn string(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or("expected metadata string".into())
}
fn array(v: &Value) -> Result<&Vec<Value>, String> {
    v.as_array().ok_or("expected metadata array".into())
}
fn run(mode: &str) -> Result<i32, String> {
    git(
        &["cat-file", "-e", &format!("{BASELINE}^{{commit}}")],
        false,
    )?;
    for (path, expected) in SOURCES {
        let actual = format!(
            "{:x}",
            Sha256::digest(git(&["show", &format!("{BASELINE}:{path}")], true)?)
        );
        if actual != expected {
            return Err(format!(
                "frozen source digest mismatch for {path}: {actual}"
            ));
        }
    }
    for manifest in MANIFESTS {
        if !Path::new(manifest).is_file() {
            return Err(format!("missing workspace member manifest: {manifest}"));
        }
    }
    let args = [
        "metadata",
        "--locked",
        "--offline",
        "--all-features",
        "--format-version",
        "1",
    ];
    let out = Command::new("cargo")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("Command '['cargo', 'metadata', '--locked', '--offline', '--all-features', '--format-version', '1']' returned non-zero exit status {}.",out.status.code().unwrap_or(1)));
    }
    let data: Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    let mut names = BTreeMap::new();
    let mut graph = Graph::new();
    for p in array(&data["packages"])? {
        let name = string(&p["name"])?;
        names.insert(string(&p["id"])?, name);
        graph.entry(name.into()).or_default();
    }
    for node in array(&data["resolve"]["nodes"])? {
        let id = string(&node["id"])?;
        let source = names.get(id).ok_or_else(|| format!("'{id}'"))?;
        for dep in array(&node["deps"])? {
            let id = string(&dep["pkg"])?;
            let target = names.get(id).ok_or_else(|| format!("'{id}'"))?;
            graph
                .entry((*source).into())
                .or_default()
                .insert((*target).into());
        }
    }
    if !graph.contains_key(JUDGE) {
        return Err(format!(
            "blind-judge crate missing from resolved workspace: {JUDGE}"
        ));
    }
    if mode == "positive" {
        if let Some(bad) = forbidden_path(&graph) {
            return Err(format!(
                "blind judge reaches forbidden package: {}",
                bad.join(" -> ")
            ));
        }
        println!("D-29 positive PASS: {JUDGE} has no transitive path to crucible-genome, crucible-search");
        Ok(0)
    } else {
        graph
            .entry(JUDGE.into())
            .or_default()
            .insert("synthetic-d29-bridge".into());
        graph.insert(
            "synthetic-d29-bridge".into(),
            BTreeSet::from(["crucible-genome".into()]),
        );
        let bad = forbidden_path(&graph)
            .ok_or("red control failed to reject synthetic indirect dependency")?;
        eprintln!("D-29 RED reproduced: {}", bad.join(" -> "));
        Ok(1)
    }
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (mode, root) = match args.as_slice() {
        [flag, m] if flag == "--mode" => {
            (m, PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        }
        [flag, m, rflag, r] if flag == "--mode" && rflag == "--root" => (m, PathBuf::from(r)),
        _ => {
            eprintln!("usage: v5-d29-blind-judge-control --mode {{positive,red}} [--root ROOT]");
            std::process::exit(2)
        }
    };
    if !["positive", "red"].contains(&mode.as_str()) {
        eprintln!("invalid mode: {mode}");
        std::process::exit(2)
    }
    std::process::exit(
        match std::env::set_current_dir(root)
            .map_err(|e| e.to_string())
            .and_then(|()| run(mode))
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("D-29 control ERROR: {e}");
                2
            }
        },
    );
}
