#!/usr/bin/env python3
"""Run bounded controls against the frozen D-18 joil-core library."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys


FROZEN_COMMIT = "e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0"
SOURCE_PATH = "crates/joil-core/src/evolve.rs"
SOURCE_SHA256 = "62cb5c8c30186155d09fb816a0e42ad7da615738cfdc11cb27348a16f94778ab"


def source_inventory(root: Path) -> str:
    raw = subprocess.check_output(
        ["git", "-C", str(root), "show", f"{FROZEN_COMMIT}:{SOURCE_PATH}"]
    )
    actual = hashlib.sha256(raw).hexdigest()
    if actual != SOURCE_SHA256:
        raise RuntimeError(f"frozen source digest mismatch: {actual}")
    return raw.decode("utf-8")


def verify_frozen_scheduler_facts(source: str) -> None:
    config = re.search(r"pub struct EvolutionConfig\s*\{(.*?)\n\}", source, re.S)
    if not config:
        raise RuntimeError("EvolutionConfig definition not found")
    if re.search(r"lease|concurr|admission", config.group(1), re.I):
        raise RuntimeError("unexpected lease/concurrency admission field found")
    if "for (isl, pop) in islands.iter_mut().enumerate()" not in source:
        raise RuntimeError("frozen serial island loop anchor not found")
    run_start = source.index("pub fn run_campaign(")
    run_end = source.index("\npub fn ", run_start + 1)
    run_body = source[run_start:run_end]
    loop_pos = run_body.index("for (isl, pop) in islands.iter_mut().enumerate()")
    if "std::thread::scope" in run_body or ".spawn(" in run_body or "rayon::" in run_body:
        raise RuntimeError("unexpected parallel scheduler construct in run_campaign")
    if run_body.index("if let Some(cuda_bin) = &cfg.cuda_bin_path") < loop_pos:
        raise RuntimeError("external evaluator call moved before the island loop")
    if "attach_cuda_audit_with_vram(&mut baseline_ref, cuda_bin, Some(85.5));" not in run_body:
        raise RuntimeError("external evaluator call path changed")
    attach_start = source.index("pub fn attach_cuda_audit_with_vram(")
    attach_end = source.index("\n///", attach_start + 1)
    if "Command::new(cuda_bin)" not in source[attach_start:attach_end]:
        raise RuntimeError("external evaluator executable invocation not found")


def write_probe_source(path: Path) -> None:
    path.write_text(
        r'''use joil_core::evolve::{run_campaign, EvolutionConfig};
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
''',
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("positive", "red"))
    parser.add_argument("--scratch", required=True, type=Path)
    parser.add_argument("--frozen-rust", required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    source = source_inventory(root)
    verify_frozen_scheduler_facts(source)

    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=True)
    core_rlib = args.frozen_rust / "libjoil_core.rlib"
    ir_rlib = args.frozen_rust / "libjoil_ir.rlib"
    for lib in (core_rlib, ir_rlib):
        if not lib.is_file():
            raise FileNotFoundError(lib)
    serde_json_rlib = next((args.frozen_rust / "deps").glob("libserde_json-*.rlib"), None)
    if serde_json_rlib is None:
        raise FileNotFoundError("libserde_json rlib not found in frozen dependency directory")
    rust_source = scratch / "d18_probe.rs"
    binary = scratch / "d18_probe"
    write_probe_source(rust_source)
    rustc = os.environ.get("RUSTC", "rustc")
    subprocess.run(
        [
            rustc,
            "--edition=2021",
            str(rust_source),
            "--extern",
            f"joil_core={core_rlib}",
            "--extern",
            f"joil_ir={ir_rlib}",
            "--extern",
            f"serde_json={serde_json_rlib}",
            "-L",
            f"dependency={args.frozen_rust / 'deps'}",
            "-L",
            str(args.frozen_rust),
            "-o",
            str(binary),
        ],
        check=True,
    )
    if args.mode == "red":
        evaluator = scratch / "fake_recording_evaluator"
        log = scratch / "evaluator-invocations.log"
        evaluator.write_text(
            "#!/bin/sh\n"
            "export D18_EVALUATOR_PID=$$\n"
            "python3 -I -c 'import os,time; print(os.environ[\"D18_EVALUATOR_PID\"], time.monotonic_ns(), file=open(os.environ[\"D18_LOG\"], \"a\"))'\n"
            "exit 0\n",
            encoding="utf-8",
        )
        evaluator.chmod(0o755)
        child_env = dict(os.environ, D18_LOG=str(log))
    else:
        child_env = os.environ.copy()
    result = subprocess.run([str(binary), args.mode, str(scratch)], env=child_env)
    if args.mode == "red":
        invocation_log = scratch / "evaluator-invocations.log"
        records = invocation_log.read_text(encoding="utf-8").splitlines() if invocation_log.is_file() else []
        try:
            timestamps = [int(line.split()[1]) for line in records]
        except (IndexError, ValueError) as exc:
            raise RuntimeError("fake evaluator invocation record is malformed") from exc
        if not timestamps or any(b <= a for a, b in zip(timestamps, timestamps[1:])):
            raise RuntimeError("run_campaign did not invoke the evaluator serially with monotonic records")
        if result.returncode != 1:
            raise RuntimeError(f"red control returned {result.returncode}, expected 1")
    return result.returncode


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, subprocess.CalledProcessError, RuntimeError) as exc:
        print(f"D-18 control error: {exc}", file=sys.stderr)
        sys.exit(2)
