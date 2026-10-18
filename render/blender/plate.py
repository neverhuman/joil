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


ROOT = Path(__file__).resolve().parents[2]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def atomic_json(path, value):
    tmp = path.with_suffix(path.suffix + ".tmp")
    with tmp.open("w", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(tmp, path)


def semantic_preflight(bundle, glb=None, subject=None):
    command = ["cargo", "run", "--offline", "--locked", "-q", "-p", "crucible-render",
               "--bin", "preflight", "--", str(bundle)]
    if glb:
        command.extend(["--glb", str(glb), "--subject", str(subject)])
    env = os.environ.copy()
    env.update({"CARGO_INCREMENTAL": "0", "RAYON_NUM_THREADS": "1",
                "OMP_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1", "MKL_NUM_THREADS": "1"})
    proc = subprocess.run(command, cwd=ROOT, env=env, text=True, capture_output=True)
    if proc.returncode:
        raise ValueError(proc.stderr.strip() or "scene semantic validation failed")
    return json.loads(proc.stdout)


def frame_digest(path):
    return digest(path.read_bytes())


def verified_completed_frames(manifest, output):
    frame_hashes = manifest.get("frame_sha256", {})
    completed = []
    for frame in manifest.get("completed_frames", []):
        path = output / f"frame-{frame:06d}.png"
        expected = frame_hashes.get(str(frame))
        if path.is_file() and expected and frame_digest(path) == expected:
            completed.append(frame)
    return completed


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
    p.add_argument("--subject", type=Path, help="source subject bytes for real render")
    a = p.parse_args()
    executable = a.blender or shutil.which("blender")
    if a.probe:
        if not executable:
            print(json.dumps({"available": False, "reason": "blender executable not found"}))
            return 0
        proc = subprocess.run([executable, "--version"], text=True, capture_output=True, check=False)
        version = (proc.stdout + proc.stderr).strip()
        print(json.dumps({"available": proc.returncode == 0, "executable": str(Path(executable).resolve()),
                          "build_id": version, "exit_code": proc.returncode}))
        return proc.returncode
    if not a.bundle or not a.recipe or not a.output or a.frames < 1:
        p.error("--bundle, --recipe, --output and positive --frames required")

    bundle_bytes, recipe_bytes = a.bundle.read_bytes(), a.recipe.read_bytes()
    bundle, recipe = json.loads(bundle_bytes), json.loads(recipe_bytes)
    profile_path = Path(__file__).with_name("profile.json")
    profile_bytes = profile_path.read_bytes()
    if a.dry_run:
        semantic = semantic_preflight(a.bundle)
        build_identity = "DRY_RUN_NO_BLENDER_BUILD"
    else:
        if not a.build_id or not a.subject:
            p.error("real render requires --build-id from --probe and --subject source bytes")
        if not executable:
            p.error("Blender unavailable; use --dry-run or install a tested build")
        probe = subprocess.run([executable, "--version"], text=True, capture_output=True, check=False)
        build_identity = (probe.stdout + probe.stderr).strip()
        if probe.returncode or a.build_id != build_identity:
            p.error("installed Blender build does not exactly match the probed --build-id")
        semantic = semantic_preflight(a.bundle, bundle.get("glb_path"), a.subject)

    key_payload = [digest(bundle_bytes), digest(recipe_bytes), build_identity,
                   digest(profile_bytes), Path(executable).resolve().as_posix() if executable else None]
    key = digest(json.dumps(key_payload, separators=(",", ":")).encode())
    run_output = a.output / key
    run_output.mkdir(parents=True, exist_ok=True)
    manifest_path = run_output / "manifest.json"
    if manifest_path.exists():
        manifest = json.loads(manifest_path.read_text())
        if manifest.get("cache_key") != key:
            p.error("manifest cache identity mismatch")
    else:
        manifest = {"schema_version": 1, "cache_key": key,
                    "bundle_sha256": digest(bundle_bytes), "recipe_sha256": digest(recipe_bytes),
                    "blender_build": build_identity, "profile_sha256": digest(profile_bytes),
                    "evidence_class": bundle.get("evidence_class", "unknown"),
                    "observable": bundle.get("observable", "unspecified"),
                    "unit": bundle.get("unit", "unspecified"), "scale": bundle.get("scale"),
                    "motion_is_simulated": bundle.get("motion_is_simulated", False),
                    "limitations": bundle.get("limitations", ["rendering does not qualify evidence"]),
                    "completed_frames": [], "frame_sha256": {}}
    valid_frames = verified_completed_frames(manifest, run_output)
    manifest["completed_frames"] = valid_frames
    manifest["frame_sha256"] = {str(frame): manifest["frame_sha256"][str(frame)] for frame in valid_frames}

    if a.dry_run:
        print(json.dumps({"dry_run": True, "cache_key": key,
                          "completed_frames": manifest["completed_frames"],
                          "frames_requested": a.frames, "semantic_preflight": semantic,
                          "blender_available": bool(executable)}))
        return 0
    glb = Path(bundle["glb_path"]).resolve()
    manifest["status"] = "rendering"
    atomic_json(manifest_path, manifest)
    env = os.environ.copy()
    env.update({"RAYON_NUM_THREADS": "1", "OMP_NUM_THREADS": "1",
                "OPENBLAS_NUM_THREADS": "1", "MKL_NUM_THREADS": "1"})
    proc = subprocess.run([executable, "--background", "--python", str(Path(__file__).with_name("render_job.py")),
                           "--", "--glb", str(glb), "--output", str(run_output),
                           "--manifest", str(manifest_path), "--frames", str(a.frames)],
                          check=False, env=env)
    if proc.returncode:
        manifest["status"] = "interrupted"
        atomic_json(manifest_path, manifest)
        return proc.returncode
    manifest = json.loads(manifest_path.read_text())
    print(json.dumps({"manifest": str(manifest_path), "status": manifest["status"],
                      "completed_frames": manifest["completed_frames"]}))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"preflight failed: {error}", file=sys.stderr)
        sys.exit(1)
