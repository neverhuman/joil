#!/usr/bin/env python3
"""Runtime controls for the D-21 presentation and trajectory claim boundary.

These controls exercise software provenance rules only. They do not validate a
physical trajectory or any separator-performance claim.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FROZEN = "e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0"
SOURCES = {
    "apps/explorer/src/main.tsx": (
        "Evidence and Proposals panels",
        "77d32c1c3474d06e0fe90af5f9a4ef808f5874f8d4c5c6a834b5d446a25373db",
    ),
    "crates/joil-core/src/scene.rs": (
        "trajectory construction",
        "614952d72ea54ce4ddca16bbc5f0ab7c450b094da3eff7acfe333f00d8b48159",
    ),
}
FORBIDDEN = ("coupled-transport", "cross-pipeline", "-38.5%", "−38.5%", "-38.4%", "−38.4%")


class ControlError(Exception):
    pass


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def frozen_bytes(path: str) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(ROOT), "show", f"{FROZEN}:{path}"],
        check=True,
        capture_output=True,
    )
    return result.stdout


def bind_frozen_sources() -> dict[str, str]:
    gates = json.loads((ROOT / "docs/v5-plan/execution/DEFECT_ACCEPTANCE_GATES.json").read_text())
    if gates.get("frozen_source_commit") != FROZEN:
        raise ControlError("acceptance gate frozen commit drift")
    defects = json.loads((ROOT / gates["inventory_path"]).read_text())
    defect = next(row for row in defects["defects"] if row["id"] == "D-21")
    actual = {}
    for path, (symbol, expected) in SOURCES.items():
        recorded = [s for s in defect["sources"] if s["path"] == path and s["symbol"] == symbol]
        if len(recorded) != 1 or recorded[0]["source_sha256"] != expected:
            raise ControlError(f"D-21 source inventory drift: {path}")
        source = frozen_bytes(path)
        digest = sha256(source)
        if digest != expected:
            raise ControlError(f"frozen source digest mismatch: {path}: {digest}")
        actual[path] = digest
    return actual


def validate_trajectory_manifest(manifest: dict) -> tuple[bool, str]:
    """Accept only native, exactly bound trajectory observations."""
    required = ("subject_id", "subject_sha256", "time_s", "field", "field_sha256", "evidence_kind")
    if not all(manifest.get(key) not in (None, "") for key in required):
        return False, "SCHEMATIC / unavailable: trajectory evidence is not bound to subject, time, and field"
    if manifest["evidence_kind"] != "native_observation":
        return False, "SCHEMATIC: authored or unvalidated paths are not measured trajectories"
    if not isinstance(manifest["time_s"], (int, float)) or manifest["time_s"] < 0:
        return False, "SCHEMATIC: invalid observation time"
    if not manifest.get("scope"):
        return False, "SCHEMATIC: measured scope is absent"
    return True, f"Bound scope (synthetic fixture): {manifest['scope']}"


def check_renderer_contract() -> None:
    render = (ROOT / "crates/crucible-render/src/lib.rs").read_text()
    registry = (ROOT / "recipes/v5/registry.json").read_text()
    scene = (ROOT / "crates/joil-core/src/scene.rs").read_text()
    if "oil_without_evidence_is_an_unavailable_card_not_trajectory_data" not in render:
        raise ControlError("crucible-render unavailable-evidence guard is missing")
    if "authored trajectories are prohibited" not in registry:
        raise ControlError("crucible-render policy does not prohibit authored trajectories")
    if "SCHEMATIC_AUTHORED_CURVE_NOT_SIMULATED_OR_MEASURED" not in scene:
        raise ControlError("scene trajectory provenance no longer labels authored paths schematic")
    web = ROOT / "crates/crucible-web"
    if web.exists():
        raise ControlError("crates/crucible-web exists; its policy must be added to this control")


def run_renderer_regression() -> None:
    env = os.environ.copy()
    env["CARGO_INCREMENTAL"] = "0"
    env["CARGO_PROFILE_DEV_DEBUG"] = "line-tables-only"
    command = [
        "cargo", "test", "--locked", "--offline", "-p", "crucible-render",
        "oil_without_evidence_is_an_unavailable_card_not_trajectory_data",
    ]
    result = subprocess.run(command, cwd=ROOT, env=env)
    if result.returncode:
        raise ControlError(f"crucible-render regression failed with exit {result.returncode}")
    print("crucible-render unavailable-data regression: 1 passed; separate crucible-web crate absent.")


def positive() -> None:
    sources = bind_frozen_sources()
    check_renderer_contract()
    run_renderer_regression()
    fixture = {
        "subject_id": "subject-fixture-01",
        "subject_sha256": "a" * 64,
        "time_s": 1.25,
        "field": "oil_volume_fraction",
        "field_sha256": "b" * 64,
        "evidence_kind": "native_observation",
        "scope": "single subject at 1.25 s; oil_volume_fraction field only",
    }
    accepted, presentation = validate_trajectory_manifest(fixture)
    authored = {"label": "SCHEMATIC authored curve", "evidence_kind": "authored"}
    authored_accepted, authored_presentation = validate_trajectory_manifest(authored)
    if not accepted or authored_accepted or not authored_presentation.startswith("SCHEMATIC"):
        raise ControlError("positive scope/presentation assertion failed")
    print("D-21 positive PASS: exact frozen source digests bound:")
    for path, digest in sources.items():
        print(f"  {path} sha256={digest}")
    print(f"  {presentation}")
    print(f"  {authored_presentation}")
    print("Scope: software provenance control only; fixture is synthetic, not physical evidence.")


def red() -> None:
    bind_frozen_sources()
    check_renderer_contract()
    run_renderer_regression()
    scene = {
        "native_trajectory_manifest": None,
        "claim": "coupled-transport cross-pipeline -38.5%",
    }
    accepted, presentation = validate_trajectory_manifest(scene)
    claim_allowed = accepted and not any(term in scene["claim"] for term in FORBIDDEN)
    if claim_allowed:
        raise ControlError("unsupported coupled-transport claim was admitted")
    print("D-21 RED PASS: scene without native trajectory evidence rejected the claim.")
    print(f"  {presentation}")
    print(f"  rejected wording: {scene['claim']}")
    print("Expected control exit: 1 (the invalid red fixture was rejected).")
    raise SystemExit(1)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("positive", "red"), required=True)
    args = parser.parse_args()
    try:
        if args.mode == "positive":
            positive()
            return 0
        red()
    except (ControlError, subprocess.CalledProcessError, OSError, KeyError, ValueError) as exc:
        print(f"D-21 CONTROL ERROR: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
