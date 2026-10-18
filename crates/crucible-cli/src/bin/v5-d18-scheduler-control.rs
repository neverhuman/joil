//! Bounded frozen scheduler controls; no scientific admission.
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
const FROZEN: &str = "e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0";
const HASH: &str = "62cb5c8c30186155d09fb816a0e42ad7da615738cfdc11cb27348a16f94778ab";
const PROBE: &str = r###"use joil_core::evolve::{run_campaign, EvolutionConfig};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = &args[1];
    let scratch = PathBuf::from(&args[2]);
    let mut cfg = EvolutionConfig::default();
    cfg.islands = if mode == "positive" { 1 } else { 2 };
    cfg.pop_per_island = 1;
    cfg.generations = 1;
    cfg.migration_interval = 1;
    cfg.mc_samples = 1;
    cfg.cuda_bin_path = if mode == "positive" {
        None
    } else {
        Some(scratch.join("fake_recording_evaluator"))
    };
    let output = scratch.join(if mode == "positive" { "campaign-pos" } else { "campaign-red" });
    let summary = run_campaign(&cfg, &output).unwrap_or_else(|e| panic!("run_campaign failed: {e}"));
    if mode == "positive" {
        let children = std::fs::read_to_string(format!("/proc/{}/task/{}/children", std::process::id(), std::process::id()))
            .unwrap_or_default();
        let assertion_holds = summary.total_evaluations == 3 && children.trim().is_empty();
        println!("{}", serde_json::json!({
            "mode": "positive",
            "assertion_holds": assertion_holds,
            "total_evaluations": summary.total_evaluations,
            "requested_islands": cfg.islands,
            "active_concurrent_evaluator_tasks": 1,
            "active_task_basis": "one synchronous run_campaign call; one island; serial evaluator path",
            "unleased_child_evaluator_processes": if children.trim().is_empty() { 0 } else { children.split_whitespace().count() },
            "scientific_admission": false
        }));
        if !assertion_holds { std::process::exit(2); }
    } else {
        println!("{}", serde_json::json!({
            "mode": "red",
            "assertion_holds": false,
            "total_evaluations": summary.total_evaluations,
            "requested_islands": cfg.islands,
            "max_concurrent_islands": 1,
            "evaluator_invocations_log": scratch.join("evaluator-invocations.log"),
            "red_reason": "unleased_serial_island_loop: EvolutionConfig has no lease/concurrency admission field; nested island loop is serial and the external evaluator call has no lease admission",
            "scientific_admission": false
        }));
        eprintln!("C006-RED D-18: unleased_serial_island_loop — no lease/concurrency admission; external evaluator invoked without admission");
        std::process::exit(1);
    }
}
"###;
fn facts(source: &str) -> Result<(), String> {
    let config = source
        .split_once("pub struct EvolutionConfig")
        .and_then(|(_, s)| s.split_once("{"))
        .and_then(|(_, s)| s.split_once("\n}"))
        .map(|(s, _)| s)
        .ok_or("EvolutionConfig definition not found")?;
    if ["lease", "concurr", "admission"]
        .iter()
        .any(|a| config.to_lowercase().contains(a))
    {
        return Err("unexpected lease/concurrency admission field found".into());
    }
    let anchor = "for (isl, pop) in islands.iter_mut().enumerate()";
    if !source.contains(anchor) {
        return Err("frozen serial island loop anchor not found".into());
    }
    let body = source
        .split_once("pub fn run_campaign(")
        .and_then(|(_, s)| s.split_once("\npub fn "))
        .map(|(s, _)| s)
        .ok_or("substring not found")?;
    let pos = body.find(anchor).ok_or("substring not found")?;
    if ["std::thread::scope", ".spawn(", "rayon::"]
        .iter()
        .any(|a| body.contains(a))
    {
        return Err("unexpected parallel scheduler construct in run_campaign".into());
    }
    if body
        .find("if let Some(cuda_bin) = &cfg.cuda_bin_path")
        .ok_or("substring not found")?
        < pos
    {
        return Err("external evaluator call moved before the island loop".into());
    }
    if !body.contains("attach_cuda_audit_with_vram(&mut baseline_ref, cuda_bin, Some(85.5));") {
        return Err("external evaluator call path changed".into());
    }
    let attach = source
        .split_once("pub fn attach_cuda_audit_with_vram(")
        .and_then(|(_, s)| s.split_once("\n///"))
        .map(|(s, _)| s)
        .ok_or("substring not found")?;
    if !attach.contains("Command::new(cuda_bin)") {
        return Err("external evaluator executable invocation not found".into());
    }
    Ok(())
}
fn records(path: &Path) -> Result<(), String> {
    let raw = if path.is_file() {
        fs::read_to_string(path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };
    let times = raw
        .lines()
        .map(|s| {
            s.split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<i128>().ok())
                .ok_or("fake evaluator invocation record is malformed".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if times.is_empty() || times.windows(2).any(|w| w[1] <= w[0]) {
        return Err(
            "run_campaign did not invoke the evaluator serially with monotonic records".into(),
        );
    }
    Ok(())
}
fn run(root: &Path, mode: &str, scratch: &Path, frozen: &Path) -> Result<i32, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["show", &format!("{FROZEN}:crates/joil-core/src/evolve.rs")])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("frozen source read failed".into());
    }
    let digest = format!("{:x}", Sha256::digest(&out.stdout));
    if digest != HASH {
        return Err(format!("frozen source digest mismatch: {digest}"));
    }
    facts(std::str::from_utf8(&out.stdout).map_err(|e| e.to_string())?)?;
    fs::create_dir_all(scratch).map_err(|e| e.to_string())?;
    let scratch = scratch.canonicalize().map_err(|e| e.to_string())?;
    let core = frozen.join("libjoil_core.rlib");
    let ir = frozen.join("libjoil_ir.rlib");
    for lib in [&core, &ir] {
        if !lib.is_file() {
            return Err(lib.display().to_string());
        }
    }
    let serde = fs::read_dir(frozen.join("deps"))
        .ok()
        .and_then(|entries| {
            entries.filter_map(Result::ok).map(|e| e.path()).find(|p| {
                p.file_name().is_some_and(|s| {
                    s.to_string_lossy().starts_with("libserde_json-")
                        && s.to_string_lossy().ends_with(".rlib")
                })
            })
        })
        .ok_or("libserde_json rlib not found in frozen dependency directory")?;
    let source = scratch.join("d18_probe.rs");
    let binary = scratch.join("d18_probe");
    fs::write(&source, PROBE).map_err(|e| e.to_string())?;
    let status = Command::new(std::env::var("RUSTC").unwrap_or("rustc".into()))
        .arg("--edition=2021")
        .arg(&source)
        .arg("--extern")
        .arg(format!("joil_core={}", core.display()))
        .arg("--extern")
        .arg(format!("joil_ir={}", ir.display()))
        .arg("--extern")
        .arg(format!("serde_json={}", serde.display()))
        .arg("-L")
        .arg(format!("dependency={}", frozen.join("deps").display()))
        .arg("-L")
        .arg(frozen)
        .arg("-o")
        .arg(&binary)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "probe compilation failed with exit {}",
            status.code().unwrap_or(1)
        ));
    }
    let mut cmd = Command::new(binary);
    cmd.arg(mode).arg(&scratch);
    if mode == "red" {
        let evaluator = scratch.join("fake_recording_evaluator");
        fs::write(&evaluator,"#!/bin/sh\nprintf '%s %s\\n' \"$$\" \"$(date +%s%N)\" >> \"$D18_LOG\"\nsleep 0.001\nexit 0\n").map_err(|e|e.to_string())?;
        fs::set_permissions(evaluator, fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
        cmd.env("D18_LOG", scratch.join("evaluator-invocations.log"));
    }
    let status = cmd.status().map_err(|e| e.to_string())?;
    let code = status.code().unwrap_or(2);
    if mode == "red" {
        records(&scratch.join("evaluator-invocations.log"))?;
        if code != 1 {
            return Err(format!("red control returned {code}, expected 1"));
        }
    }
    Ok(code)
}
fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_default();
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut scratch = None;
    let mut frozen = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().unwrap_or_default()),
            "--scratch" => scratch = args.next().map(PathBuf::from),
            "--frozen-rust" => frozen = args.next().map(PathBuf::from),
            _ => {
                eprintln!("invalid argument: {arg}");
                std::process::exit(2)
            }
        }
    }
    let result=match (mode.as_str(),scratch,frozen) {(m @ ("positive"|"red"),Some(s),Some(f))=>run(&root,m,&s,&f),_=>Err("usage: v5-d18-scheduler-control positive|red --scratch PATH --frozen-rust PATH [--root ROOT]".into())};
    std::process::exit(match result {
        Ok(c) => c,
        Err(e) => {
            eprintln!("D-18 control error: {e}");
            2
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_anchors_reject_tampering() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["show", &format!("{FROZEN}:crates/joil-core/src/evolve.rs")])
            .output()
            .unwrap();
        assert!(out.status.success());
        let source = String::from_utf8(out.stdout).unwrap();
        assert!(facts(&source).is_ok());
        for anchor in [
            "pub struct EvolutionConfig",
            "for (isl, pop) in islands.iter_mut().enumerate()",
            "attach_cuda_audit_with_vram(&mut baseline_ref, cuda_bin, Some(85.5));",
            "Command::new(cuda_bin)",
        ] {
            assert!(facts(&source.replace(anchor, "tampered")).is_err());
        }
    }
}
