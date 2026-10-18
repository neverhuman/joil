#!/usr/bin/env python3
"""Manifest-first Blender probe and restartable plate coordinator (stdlib only)."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

def digest(data):
    return hashlib.sha256(data).hexdigest()

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--probe", action="store_true")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--bundle", type=Path)
    p.add_argument("--recipe", type=Path)
    p.add_argument("--output", type=Path)
    p.add_argument("--build-id")
    p.add_argument("--frames", type=int, default=1)
    p.add_argument("--blender")
    a = p.parse_args()
    executable = a.blender or shutil.which("blender")
    if a.probe:
        if not executable:
            print(json.dumps({"available": False, "reason": "blender executable not found"}))
            return 0
        proc = subprocess.run([executable, "--version"], text=True, capture_output=True, check=False)
        print(json.dumps({"available": proc.returncode == 0, "executable": executable,
                          "version_output": proc.stdout.splitlines()[:2], "exit_code": proc.returncode}))
        return proc.returncode
    if not a.bundle or not a.recipe or not a.output or a.frames < 1:
        p.error("--bundle, --recipe, --output and positive --frames required")
    bundle_bytes, recipe_bytes = a.bundle.read_bytes(), a.recipe.read_bytes()
    bundle, recipe = json.loads(bundle_bytes), json.loads(recipe_bytes)
    if not a.build_id and not a.dry_run:
        p.error("real render requires exact --build-id from an availability probe")
    build_id = a.build_id or "DRY_RUN_NO_BLENDER_BUILD"
    key = digest(json.dumps([digest(bundle_bytes), digest(recipe_bytes), build_id,
                             digest(Path(__file__).with_name("profile.json").read_bytes())],
                            separators=(",", ":")).encode())
    a.output.mkdir(parents=True, exist_ok=True)
    manifest_path = a.output / (key + ".manifest.json")
    manifest = json.loads(manifest_path.read_text()) if manifest_path.exists() else {
        "schema_version": 1, "cache_key": key, "bundle_sha256": digest(bundle_bytes),
        "recipe_sha256": digest(recipe_bytes), "blender_build": build_id,
        "evidence_class": bundle.get("evidence_class", "unknown"),
        "observable": bundle.get("observable", "unspecified"), "unit": bundle.get("unit", "unspecified"),
        "scale": bundle.get("scale"), "motion_is_simulated": bundle.get("motion_is_simulated", False),
        "limitations": bundle.get("limitations", ["rendering does not qualify evidence"]),
        "completed_frames": []}
    if a.dry_run:
        print(json.dumps({"dry_run": True, "cache_key": key, "completed_frames": manifest["completed_frames"],
                          "frames_requested": a.frames, "blender_available": bool(executable)}))
        return 0
    if not executable:
        p.error("Blender unavailable; use --dry-run or install a tested build")
    probe = subprocess.run([executable, "--version"], text=True, capture_output=True, check=False)
    version_text = probe.stdout + probe.stderr
    if probe.returncode or build_id not in version_text:
        p.error("installed Blender version does not match supplied --build-id")
    if bundle.get("evidence_class") == "unavailable" and bundle.get("render_kind") != "unavailable_card":
        p.error("unavailable evidence may only render an unavailable card")
    # The adapter consumes a canonical GLB only after semantic validation in crucible-render.
    if not bundle.get("validated_scene_bundle") or not bundle.get("glb_path"):
        p.error("bundle lacks validated_scene_bundle and glb_path")
    if bundle.get("private_geometry") and not bundle.get("private_export_approved"):
        p.error("private geometry export is not approved")
    manifest["status"] = "rendering"
    manifest["layout_review"] = "pending_after_semantic_validation"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    glb = Path(bundle["glb_path"])
    if not glb.is_file():
        p.error("canonical GLB file is missing")
    proc = subprocess.run([executable, "--background", "--python", str(Path(__file__).with_name("render_job.py")),
                           "--", "--glb", str(glb.resolve()), "--output", str(a.output.resolve()),
                           "--manifest", str(manifest_path.resolve()), "--frames", str(a.frames)],
                          check=False)
    if proc.returncode:
        manifest["status"] = "interrupted"
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
        return proc.returncode
    manifest = json.loads(manifest_path.read_text())
    print(json.dumps({"manifest": str(manifest_path), "status": manifest["status"],
                      "completed_frames": manifest["completed_frames"]}))
    return 0

if __name__ == "__main__":
    sys.exit(main())
