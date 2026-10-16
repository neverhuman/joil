#!/usr/bin/env python3
"""Execute 5-case OpenFOAM 14 (G0/G1/G2 Roache GCI + Dual LRR/SSG RSM) verification
and F2-Oil stage-0 Lagrangian droplet cohort checkpoint on Wave-2 TPMS and 8-Jet elites.

Uses physics/openfoam/environment/runtime.py (JOIL_CPU_SET=0,1,2,3, 4 cores, 12 GiB RAM)
and cleans transient processor* directories immediately after reconstructPar to keep
runs/ well below the 5.0 GiB shared disk cap.
"""

from __future__ import annotations

import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
from typing import Any

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "physics/openfoam/environment"))
sys.path.insert(0, str(ROOT / "tools"))

import runtime  # type: ignore
from generate_gci_rsm_report_and_figures import (  # type: ignore
    compute_roache_3grid_gci,
    extract_reynolds_stress_stats,
    load_grid_case,
    parse_checkmesh,
)
from run_elite_3grid_gci_and_rsm import evaluate_elite_gci_rsm_gate  # type: ignore


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse_internal_scalar_list(path: Path) -> np.ndarray:
    txt = path.read_text(errors="replace")
    m = re.search(
        r"internalField\s+nonuniform\s+List<scalar>\s+(\d+)\s*\(\s*(.*?)\s*\)\s*;",
        txt,
        re.S,
    )
    if m:
        return np.fromstring(m.group(2), sep=" ", dtype=np.float64)
    u_m = re.search(r"internalField\s+uniform\s+([0-9.eE+-]+)\s*;", txt)
    if u_m:
        return np.array([float(u_m.group(1))], dtype=np.float64)
    raise ValueError(f"Could not parse scalar field from {path}")


def set_blockmesh_grid(case_dir: Path, nx: int, ny: int, nz: int) -> None:
    bm_path = case_dir / "system/blockMeshDict"
    txt = bm_path.read_text(encoding="utf-8")
    txt_new, n = re.subn(
        r"blocks\s*\(\s*hex\s*\(0 1 2 3 4 5 6 7\)\s*\(\d+\s+\d+\s+\d+\)",
        f"blocks (hex (0 1 2 3 4 5 6 7) ({nx} {ny} {nz})",
        txt,
    )
    if n != 1:
        raise ValueError(f"Could not update blockMeshDict grid in {bm_path}")
    bm_path.write_text(txt_new, encoding="utf-8")
    im_path = case_dir / "input-manifest.json"
    if im_path.is_file():
        im = json.loads(im_path.read_text(encoding="utf-8"))
        if isinstance(im.get("dictionary_files_sha256"), dict):
            im["dictionary_files_sha256"]["system/blockMeshDict"] = sha256_file(bm_path)
            im_path.write_text(json.dumps(im, indent=2) + "\n", encoding="utf-8")


def prepare_and_run_sst_grid_case(
    wave2_family: str,
    case_dir: Path,
    grid_id: str,
    bg_cells: list[int],
    keep_time_150: bool,
) -> dict[str, Any]:
    if (case_dir / "hydraulic-integrals.json").is_file() and (
        case_dir / "mesh-2-checkMesh.log"
    ).is_file():
        print(f"[skip-existing] {case_dir.name} already solved", flush=True)
        return load_grid_case(case_dir, grid_id, bg_cells)

    if case_dir.exists():
        shutil.rmtree(case_dir)

    subprocess.run(
        [
            "cargo",
            "run",
            "--release",
            "-p",
            "joil-cli",
            "--",
            "prepare-cfd-case",
            "--wave2-family",
            wave2_family,
            "--case",
            str(case_dir),
            "--reject-pressure-pa",
            "-180000.0",
            "--steps",
            "150",
            "--model",
            "kOmegaSST",
        ],
        cwd=ROOT,
        check=True,
    )
    set_blockmesh_grid(case_dir, bg_cells[0], bg_cells[1], bg_cells[2])

    # 1. Meshing + checkMesh in a single container invocation
    mesh_cmd = (
        "blockMesh > mesh-0-blockMesh.log 2>&1 && "
        "decomposePar -noFields > mesh-1a-decomposePar.log 2>&1 && "
        "mpirun -np 4 snappyHexMesh -parallel -overwrite > mesh-1b-snappyHexMesh.log 2>&1 && "
        "reconstructPar -constant > mesh-1c-reconstructPar.log 2>&1 && "
        "rm -rf processor* && "
        "checkMesh > mesh-2-checkMesh.log 2>&1"
    )
    runtime.run(
        case_dir,
        ["bash", "-lc", mesh_cmd],
        case_dir / "mesh-pipeline.log",
        timeout=1800,
    )

    # 2. Steady kOmegaSST carrier (150 steps) + reconstructPar
    solve_cmd = (
        "decomposePar > water-0-decomposePar.log 2>&1 && "
        "mpirun -np 4 foamRun -parallel > water-1-mpirun.orig.log 2>&1 && "
        "cat water-1-mpirun.orig.log && rm -f water-1-mpirun.orig.log && "
        "reconstructPar -time 150 > water-2-reconstructPar-150.log 2>&1 && "
        "rm -rf processor* 75"
    )
    runtime.run(
        case_dir,
        ["bash", "-lc", solve_cmd],
        case_dir / "water-1-mpirun.log",
        timeout=1800,
    )

    # 3. Native joilHydraulicMetrics receipt
    runtime.run(
        case_dir,
        ["/work/runs/environment-capability-r4/bin/joilHydraulicMetrics", "-time", "150"],
        case_dir / "water-3-hydraulicMetrics-150.log",
        timeout=300,
    )
    if not keep_time_150:
        shutil.rmtree(case_dir / "150", ignore_errors=True)
        shutil.rmtree(case_dir / "constant/polyMesh", ignore_errors=True)

    return load_grid_case(case_dir, grid_id, bg_cells)


def prepare_and_run_rsm_continuation(
    wave2_family: str,
    g1_case_dir: Path,
    rsm_case_dir: Path,
    turbulence_model: str,
    base_dp_u: float,
    base_eloss: float,
    g1_dp_u: float,
    g1_eloss: float,
    keep_r_field: bool = False,
) -> dict[str, Any]:
    summary_cache = rsm_case_dir / "reynolds-stress-summary.json"
    if (rsm_case_dir / "hydraulic-integrals.json").is_file() and summary_cache.is_file():
        print(f"[skip-existing] {rsm_case_dir.name} already solved", flush=True)
        hyd = json.loads((rsm_case_dir / "hydraulic-integrals.json").read_text())
        r_stats = json.loads(summary_cache.read_text())
    else:
        if rsm_case_dir.exists():
            shutil.rmtree(rsm_case_dir)

        subprocess.run(
            [
                "cargo",
                "run",
                "--release",
                "-p",
                "joil-cli",
                "--",
                "prepare-cfd-case",
                "--wave2-family",
                wave2_family,
                "--case",
                str(rsm_case_dir),
                "--reject-pressure-pa",
                "-180000.0",
                "--steps",
                "60",
                "--model",
                turbulence_model,
            ],
            cwd=ROOT,
            check=True,
        )
        # Copy polyMesh and converged 150/{U,p,phi,k,omega,nut} from G1
        shutil.copytree(
            g1_case_dir / "constant/polyMesh", rsm_case_dir / "constant/polyMesh"
        )
        for fname in ("U", "p", "phi", "k", "omega", "nut"):
            src = g1_case_dir / "150" / fname
            if src.is_file():
                shutil.copy2(src, rsm_case_dir / "0" / fname)

        # Build nonuniform 0/R and 0/epsilon from G1 150/k and 150/omega
        k_arr = parse_internal_scalar_list(g1_case_dir / "150/k")
        om_arr = parse_internal_scalar_list(g1_case_dir / "150/omega")
        n_cells = len(k_arr)
        eps_arr = np.maximum(0.09 * k_arr * om_arr, 1e-6)
        r_diag_arr = np.maximum((2.0 / 3.0) * k_arr, 1e-6)

        eps_orig = (rsm_case_dir / "0/epsilon").read_text(encoding="utf-8")
        bf_eps = eps_orig[eps_orig.index("boundaryField") :]
        eps_lines = "\n".join(f"{v:.8e}" for v in eps_arr)
        (rsm_case_dir / "0/epsilon").write_text(
            f"FoamFile {{ format ascii; class volScalarField; object epsilon; }}\n"
            f"dimensions [0 2 -3 0 0 0 0]; internalField nonuniform List<scalar>\n"
            f"{n_cells}\n(\n{eps_lines}\n);\n\n{bf_eps}",
            encoding="utf-8",
        )

        r_orig = (rsm_case_dir / "0/R").read_text(encoding="utf-8")
        bf_r = r_orig[r_orig.index("boundaryField") :]
        r_lines = "\n".join(
            f"({v:.8e} 0 0 {v:.8e} 0 {v:.8e})" for v in r_diag_arr
        )
        (rsm_case_dir / "0/R").write_text(
            f"FoamFile {{ format ascii; class volSymmTensorField; object R; }}\n"
            f"dimensions [0 2 -2 0 0 0 0]; internalField nonuniform List<symmTensor>\n"
            f"{n_cells}\n(\n{r_lines}\n);\n\n{bf_r}",
            encoding="utf-8",
        )

        solve_cmd = (
            "decomposePar > water-0-decomposePar.log 2>&1 && "
            "mpirun -np 4 foamRun -parallel > water-1-mpirun.orig.log 2>&1 && "
            "cat water-1-mpirun.orig.log && rm -f water-1-mpirun.orig.log && "
            "reconstructPar -time 60 > water-2-reconstructPar-60.log 2>&1 && "
            "rm -rf processor* 30 && "
            "/work/runs/environment-capability-r4/bin/joilHydraulicMetrics -time 60 > water-3-hydraulicMetrics-60.log 2>&1"
        )
        runtime.run(
            rsm_case_dir,
            ["bash", "-lc", solve_cmd],
            rsm_case_dir / "water-1-mpirun.log",
            timeout=1800,
        )

        r_stats = extract_reynolds_stress_stats(rsm_case_dir / "60/R")
        summary_cache.write_text(json.dumps(r_stats, indent=2) + "\n", encoding="utf-8")
        hyd = json.loads((rsm_case_dir / "hydraulic-integrals.json").read_text())

        # Clean heavy fields to keep runs/ compact
        shutil.rmtree(rsm_case_dir / "constant/polyMesh", ignore_errors=True)
        for f_clean in ("U", "p", "phi", "k", "omega", "nut", "epsilon", "R"):
            (rsm_case_dir / "0" / f_clean).unlink(missing_ok=True)
            if not (keep_r_field and f_clean == "R"):
                (rsm_case_dir / "60" / f_clean).unlink(missing_ok=True)
        if not keep_r_field:
            shutil.rmtree(rsm_case_dir / "60", ignore_errors=True)

    p_in1 = float(hyd["patches"]["inlet1"]["area_average_static_pressure_pa"])
    p_in2 = float(hyd["patches"]["inlet2"]["area_average_static_pressure_pa"])
    dp_u = 0.5 * (p_in1 + p_in2) - float(
        hyd["patches"]["underflow"]["area_average_static_pressure_pa"]
    )
    eloss = float(hyd["hydraulic_loss_j_per_treated_m3"])
    return {
        "turbulence_model": turbulence_model,
        "turbulence_family": "7-equation Reynolds-Stress Transport Model (RSM)",
        "case_dir": str(rsm_case_dir.relative_to(ROOT)),
        "continuation_steps_from_g1_sst": 60,
        "feed_m3_h": float(hyd["feed_m3_h"]),
        "treated_m3_h": float(hyd["treated_m3_h"]),
        "reject_fraction": float(hyd["reject_fraction"]),
        "water_closure_fraction": float(hyd["water_closure_fraction"]),
        "dp_u_pa": dp_u,
        "hydraulic_loss_w": float(hyd["hydraulic_loss_w"]),
        "hydraulic_loss_j_per_treated_m3": eloss,
        "delta_dp_u_vs_kOmegaSST_pct": (dp_u - g1_dp_u) / g1_dp_u * 100.0,
        "delta_eloss_vs_kOmegaSST_pct": (eloss - g1_eloss) / g1_eloss * 100.0,
        "delta_dp_u_vs_ct35_baseline_pct": (dp_u - base_dp_u) / base_dp_u * 100.0,
        "delta_eloss_vs_ct35_baseline_pct": (eloss - base_eloss) / base_eloss * 100.0,
        "reynolds_stress_tensor_diagnostics": r_stats,
        "hydraulic_integrals_sha256": sha256_file(
            rsm_case_dir / "hydraulic-integrals.json"
        ),
    }


def build_candidate_5case_dossier(
    candidate_id: str,
    wave2_family: str,
    design_sha256: str,
    f1_cout_p95_mg_l: float,
    f1_dp_u_pa: float,
    prefix: str,
    keep_g1_fields: bool,
) -> dict[str, Any]:
    bench = json.loads(
        (ROOT / "docs/results/exotic-vs-baseline-cfd-benchmark-v1.json").read_text()
    )["baseline_ct35_openfoam14_reference"]
    base_dp_u = float(bench["static_tap_drop_pa"])
    base_eloss = float(bench["device_hydraulic_loss_j_m3"])
    base_split = float(bench["reject_fraction"])

    g0_dir = ROOT / f"runs/{prefix}-g0-coarse"
    g1_dir = ROOT / f"runs/{prefix}-g1-medium"
    g2_dir = ROOT / f"runs/{prefix}-g2-fine"
    lrr_dir = ROOT / f"runs/{prefix}-g1-rsm-lrr"
    ssg_dir = ROOT / f"runs/{prefix}-g1-rsm-ssg"

    g0 = prepare_and_run_sst_grid_case(
        wave2_family, g0_dir, "G0_coarse", [18, 12, 212], keep_time_150=False
    )
    g1 = prepare_and_run_sst_grid_case(
        wave2_family, g1_dir, "G1_medium", [24, 16, 282], keep_time_150=True
    )
    g2 = prepare_and_run_sst_grid_case(
        wave2_family, g2_dir, "G2_fine", [30, 20, 352], keep_time_150=False
    )

    lrr = prepare_and_run_rsm_continuation(
        wave2_family,
        g1_dir,
        lrr_dir,
        "LRR",
        base_dp_u,
        base_eloss,
        g1["dp_u_pa"],
        g1["hydraulic_loss_j_per_treated_m3"],
        keep_r_field=False,
    )
    ssg = prepare_and_run_rsm_continuation(
        wave2_family,
        g1_dir,
        ssg_dir,
        "SSG",
        base_dp_u,
        base_eloss,
        g1["dp_u_pa"],
        g1["hydraulic_loss_j_per_treated_m3"],
        keep_r_field=False,
    )

    if not keep_g1_fields:
        shutil.rmtree(g1_dir / "150", ignore_errors=True)
        shutil.rmtree(g1_dir / "constant/polyMesh", ignore_errors=True)

    gci_dp_u = compute_roache_3grid_gci(
        g0["dp_u_pa"],
        g1["dp_u_pa"],
        g2["dp_u_pa"],
        g0["effective_cell_size_mm"],
        g1["effective_cell_size_mm"],
        g2["effective_cell_size_mm"],
    )
    gci_eloss = compute_roache_3grid_gci(
        g0["hydraulic_loss_j_per_treated_m3"],
        g1["hydraulic_loss_j_per_treated_m3"],
        g2["hydraulic_loss_j_per_treated_m3"],
        g0["effective_cell_size_mm"],
        g1["effective_cell_size_mm"],
        g2["effective_cell_size_mm"],
    )
    gci_split = compute_roache_3grid_gci(
        g0["reject_fraction"],
        g1["reject_fraction"],
        g2["reject_fraction"],
        g0["effective_cell_size_mm"],
        g1["effective_cell_size_mm"],
        g2["effective_cell_size_mm"],
    )

    dp_saving_pa = base_dp_u - g2["dp_u_pa"]
    dp_saving_pct = (g2["dp_u_pa"] - base_dp_u) / base_dp_u * 100.0
    eloss_saving = base_eloss - g2["hydraulic_loss_j_per_treated_m3"]
    eloss_saving_pct = (
        (g2["hydraulic_loss_j_per_treated_m3"] - base_eloss) / base_eloss * 100.0
    )

    dossier: dict[str, Any] = {
        "schema_version": 1,
        "kind": "openfoam14_3grid_roache_gci_and_rsm_turbulence_verification_not_physical_validation",
        "subject_candidate_id": candidate_id,
        "subject_topology_class": wave2_family,
        "subject_design_sha256": design_sha256,
        "f1_surrogate_screen_metrics": {
            "cout_p95_mg_l": f1_cout_p95_mg_l,
            "dp_u_pa": f1_dp_u_pa,
        },
        "openfoam_image_id": "sha256:8866eda29be2bcbc05e0fa9536cacb3c63ff25c97179e058959ffbeb90da7199",
        "baseline_ct35_comparator": {
            "case_dir": "runs/ct35-sst-extended-000",
            "cells": 334768,
            "dp_u_pa": base_dp_u,
            "hydraulic_loss_j_per_treated_m3": base_eloss,
            "reject_fraction": base_split,
            "water_closure_fraction": 1.63e-05,
        },
        "grids": [g0, g1, g2],
        "roache_gci_analysis": {
            "dp_u_pa": gci_dp_u,
            "hydraulic_loss_j_per_treated_m3": gci_eloss,
            "reject_fraction": gci_split,
            "signal_to_uncertainty_margin": {
                "fine_grid_dp_u_saving_pa": dp_saving_pa,
                "fine_grid_dp_u_saving_pct": dp_saving_pct,
                "three_grid_dp_u_span_pa": gci_dp_u["three_grid_extremum_span"],
                "saving_to_3grid_span_ratio": dp_saving_pa
                / max(gci_dp_u["three_grid_extremum_span"], 1e-9),
                "fine_grid_eloss_saving_j_m3": eloss_saving,
                "fine_grid_eloss_saving_pct": eloss_saving_pct,
                "saving_to_3grid_eloss_span_ratio": eloss_saving
                / max(gci_eloss["three_grid_extremum_span"], 1e-9),
            },
        },
        "rsm_turbulence_closure_crosscheck": [lrr, ssg],
    }
    prereg = json.loads((ROOT / "experiments/wave2-preregistration-v1.json").read_text())
    gate_eval = evaluate_elite_gci_rsm_gate(dossier, prereg)
    dossier["mandatory_promotion_gate"] = gate_eval
    return dossier


def run_wave2_tpms_f2_oil_stage0_checkpoint(g1_case_dir: Path) -> dict[str, Any]:
    """Run a 0.01 s (200 steps at dt=5e-5 s) 10-bin Lagrangian droplet cohort stage-0 checkpoint
    on the converged Wave-2 TPMS elite G1 carrier (time=150) and record exact wall-clock throughput
    plus the 8-stage resumable continuation plan to reach T >= 4.0 s (< 0.1% retained oil).
    """
    sys.path.insert(0, str(ROOT / "physics/openfoam/oil"))
    import oil_grade  # type: ignore

    # Ensure hydraulic-integrals-t150.json and metrics-binary-t150.json exist for strict provenance check
    hyd_src = g1_case_dir / "hydraulic-integrals.json"
    hyd_t150 = g1_case_dir / "hydraulic-integrals-t150.json"
    if hyd_src.is_file() and not hyd_t150.is_file():
        shutil.copy2(hyd_src, hyd_t150)

    bin_path = ROOT / "runs/environment-capability-r4/bin/joilHydraulicMetrics"
    bin_meta = g1_case_dir / "metrics-binary-t150.json"
    if not bin_meta.is_file():
        bin_meta.write_text(
            json.dumps(
                {
                    "binary": str(bin_path.relative_to(ROOT)),
                    "sha256": sha256_file(bin_path),
                    "time_step": 150,
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )

    strict_gate_reason = None
    try:
        oil_grade.validate_carrier_case_eligibility(
            g1_case_dir,
            time_step=150,
            allow_diagnostic_on_failed_seed_mesh=True,
            allow_unextended_overflow_backflow_diagnostic=False,
        )
    except ValueError as exc:
        strict_gate_reason = str(exc)

    existing_pilots = []
    solved_unscored = []
    for d in sorted((ROOT / "runs").glob("gemini-ct35-full-liner-inlet-t150-dt5e-05-*")):
        res_f = d / "result-review-v2.json"
        if res_f.is_file():
            r_obj = json.loads(res_f.read_text(encoding="utf-8"))
            if "wave2-tpms-gyroid-g1-medium" in str(r_obj.get("carrier_case", "")):
                existing_pilots.append((d, r_obj))
        elif (d / "foamRun.log.receipt.json").is_file() and (d / "0.01").is_dir():
            solved_unscored.append(d)

    if existing_pilots:
        pilot_dir, pilot_res = existing_pilots[-1]
    elif solved_unscored:
        pilot_dir = solved_unscored[-1]
        pilot_res = oil_grade.evaluate_full_liner_pilot_case(
            case=pilot_dir,
            carrier_case=g1_case_dir,
            time_step=150,
            dt=5e-5,
            end_time=0.01,
            commands=None,
            injection_mode="flux_weighted_inlet",
            allow_unextended_overflow_backflow_diagnostic=True,
        )
        for p_sub in pilot_dir.glob("processor*"):
            shutil.rmtree(p_sub, ignore_errors=True)
        shutil.rmtree(pilot_dir / "constant/polyMesh", ignore_errors=True)
        for t_sub in ("0", "0.002", "0.004", "0.006", "0.008", "0.01"):
            shutil.rmtree(pilot_dir / t_sub, ignore_errors=True)
    else:
        pilot_res, _ = oil_grade.run_full_liner_frozen_carrier_pilot(
            g1_case_dir,
            time_step=150,
            dt=5e-5,
            end_time=0.01,
            cpuset="0,1,2,3",
            seed_cache=None,
            injection_mode="flux_weighted_inlet",
            allow_unextended_overflow_backflow_diagnostic=True,
        )
        pilot_dir = ROOT / Path(pilot_res["result_file"]).parent
        for p_sub in pilot_dir.glob("processor*"):
            shutil.rmtree(p_sub, ignore_errors=True)
        shutil.rmtree(pilot_dir / "constant/polyMesh", ignore_errors=True)
        for t_sub in ("0", "0.002", "0.004", "0.006", "0.008", "0.01"):
            shutil.rmtree(pilot_dir / t_sub, ignore_errors=True)

    wall_s = float(pilot_res["foamRun_wall_seconds"])
    est_wall_per_1s = wall_s * 100.0
    est_wall_4s_hours = (wall_s * 400.0) / 3600.0

    return {
        "status": "stage0_checkpoint_executed_resumable_plan_bound_not_completed_4s_f2_oil",
        "qualified_for_f2_oil_grade": False,
        "qualified_multiobjective_vector": None,
        "strict_unextended_overflow_backflow_gate_rejected": strict_gate_reason is not None,
        "strict_unextended_overflow_backflow_gate_reason": strict_gate_reason,
        "carrier_case": str(g1_case_dir.relative_to(ROOT)),
        "carrier_time_step": 150,
        "stage0_pilot_case_dir": str(pilot_dir.relative_to(ROOT)),
        "stage0_result_file": pilot_res["result_file"],
        "stage0_result_sha256": pilot_res["result_sha256"],
        "dt_s": 5e-5,
        "stage0_end_time_s": 0.01,
        "stage0_steps": 200,
        "stage0_foamRun_wall_seconds": wall_s,
        "stage0_inventory_closure_passed": bool(pilot_res["inventory_closure_passed"]),
        "stage0_total_discrepancy_count": sum(
            int(c["discrepancy_count"]) for c in pilot_res["cohorts"]
        ),
        "stage0_remaining_in_domain_fraction_mean": float(
            np.mean([float(c["remaining_in_domain_fraction"]) for c in pilot_res["cohorts"]])
        ),
        "measured_throughput": {
            "wall_seconds_per_0.01s_physical": wall_s,
            "estimated_wall_seconds_per_1.0s_physical": est_wall_per_1s,
            "estimated_wall_hours_for_4.0s_full_residence_4cpus": est_wall_4s_hours,
        },
        "resumable_8stage_continuation_plan": {
            "target_physical_time_s": 4.0,
            "target_max_retained_mass_fraction": 0.001,
            "time_step_dt_s": 5e-5,
            "total_steps_required": 80000,
            "chunk_size_physical_s": 0.50,
            "chunk_steps": 10000,
            "num_chunks": 8,
            "controlDict_continuation_settings": {
                "startFrom": "latestTime",
                "stopAt": "endTime",
                "writeControl": "timeStep",
                "writeInterval": 10000,
                "purgeWrite": 2,
            },
            "stages": [
                {
                    "stage_index": k,
                    "t_start_s": round((k - 1) * 0.50, 2),
                    "t_end_s": round(k * 0.50, 2),
                    "steps": 10000,
                    "estimated_wall_minutes_4cpus": round((wall_s * 50.0) / 60.0, 2),
                }
                for k in range(1, 9)
            ],
            "resume_command": (
                "JOIL_CPU_SET=0,1,2,3 python3 physics/openfoam/oil/oil_grade.py "
                "--carrier-case runs/wave2-tpms-gyroid-g1-medium --time-step 150 "
                "--run-inlet-cohort-pilot --pilot-dt 5e-5 --pilot-end-time 0.50 --cpuset 0,1,2,3"
            ),
        },
    }


def main() -> int:
    os.environ["JOIL_CPU_SET"] = "0,1,2,3"
    w2_ablation = json.loads(
        (ROOT / "docs/results/wave2-multiseed-ablation-and-holdout-v1.json").read_text()
    )
    fams = w2_ablation["promoted_wave2_elites_by_family"]

    tpms_ind = fams["cylindrical-gyroid-coalescer"]
    octa_ind = fams["octa-branch-helical-manifold"]

    print("=== Running 5-case OpenFOAM 14 GCI+RSM on Wave-2 TPMS Elite (cylindrical-gyroid-coalescer) ===", flush=True)
    tpms_dossier = build_candidate_5case_dossier(
        candidate_id=tpms_ind["id"],
        wave2_family="cylindrical-gyroid-coalescer",
        design_sha256=tpms_ind["genome_sha256"],
        f1_cout_p95_mg_l=float(tpms_ind["mc_robustness"]["cout_p95_mg_l"]),
        f1_dp_u_pa=float(tpms_ind["nominal_metrics"]["dp_u_pa"]),
        prefix="wave2-tpms-gyroid",
        keep_g1_fields=True,
    )

    print("=== Running 5-case OpenFOAM 14 GCI+RSM on Wave-2 8-Jet Elite (octa-branch-helical-manifold) ===", flush=True)
    octa_dossier = build_candidate_5case_dossier(
        candidate_id=octa_ind["id"],
        wave2_family="octa-branch-helical-manifold",
        design_sha256=octa_ind["genome_sha256"],
        f1_cout_p95_mg_l=float(octa_ind["mc_robustness"]["cout_p95_mg_l"]),
        f1_dp_u_pa=float(octa_ind["nominal_metrics"]["dp_u_pa"]),
        prefix="wave2-octa8jet",
        keep_g1_fields=False,
    )

    print("=== Running Stage-0 F2-Oil 10-Bin Lagrangian Checkpoint on Wave-2 TPMS Elite ===", flush=True)
    f2_oil_checkpoint = run_wave2_tpms_f2_oil_stage0_checkpoint(
        ROOT / "runs/wave2-tpms-gyroid-g1-medium"
    )

    w1_dossier = json.loads(
        (ROOT / "docs/results/elite-3grid-gci-and-rsm-verification-v1.json").read_text()
    )
    prereg = json.loads((ROOT / "experiments/wave2-preregistration-v1.json").read_text())
    w1_gate = evaluate_elite_gci_rsm_gate(w1_dossier, prereg)

    # Compare surrogate/F1 ranking vs OpenFOAM 14 CFD outcomes across Wave-1 and Wave-2 elites
    ranking_comparison = [
        {
            "candidate_id": tpms_dossier["subject_candidate_id"],
            "wave": "Wave-2",
            "topology_class": tpms_dossier["subject_topology_class"],
            "design_sha256": tpms_dossier["subject_design_sha256"],
            "f1_cout_p95_mg_l": tpms_dossier["f1_surrogate_screen_metrics"]["cout_p95_mg_l"],
            "f1_dp_u_pa": tpms_dossier["f1_surrogate_screen_metrics"]["dp_u_pa"],
            "openfoam_g2_dp_u_pa": tpms_dossier["grids"][2]["dp_u_pa"],
            "openfoam_g2_eloss_j_m3": tpms_dossier["grids"][2]["hydraulic_loss_j_per_treated_m3"],
            "openfoam_lrr_dp_u_pa": tpms_dossier["rsm_turbulence_closure_crosscheck"][0]["dp_u_pa"],
            "openfoam_ssg_dp_u_pa": tpms_dossier["rsm_turbulence_closure_crosscheck"][1]["dp_u_pa"],
            "snr_span_dp_u": tpms_dossier["roache_gci_analysis"]["signal_to_uncertainty_margin"]["saving_to_3grid_span_ratio"],
            "snr_span_eloss": tpms_dossier["roache_gci_analysis"]["signal_to_uncertainty_margin"]["saving_to_3grid_eloss_span_ratio"],
            "inter_rsm_dp_u_delta_pct": tpms_dossier["mandatory_promotion_gate"]["checks"]["inter_rsm_dp_u_delta_pct"],
            "mandatory_5run_gate_passed": tpms_dossier["mandatory_promotion_gate"]["gate_passed"],
            "gate_failures": tpms_dossier["mandatory_promotion_gate"]["failures"],
        },
        {
            "candidate_id": w1_dossier["subject_candidate_id"],
            "wave": "Wave-1",
            "topology_class": w1_dossier["subject_topology_class"],
            "design_sha256": w1_dossier["subject_design_sha256"],
            "f1_cout_p95_mg_l": 7.4991,
            "f1_dp_u_pa": 28697.71,
            "openfoam_g2_dp_u_pa": w1_dossier["grids"][2]["dp_u_pa"],
            "openfoam_g2_eloss_j_m3": w1_dossier["grids"][2]["hydraulic_loss_j_per_treated_m3"],
            "openfoam_lrr_dp_u_pa": w1_dossier["rsm_turbulence_closure_crosscheck"][0]["dp_u_pa"],
            "openfoam_ssg_dp_u_pa": w1_dossier["rsm_turbulence_closure_crosscheck"][1]["dp_u_pa"],
            "snr_span_dp_u": w1_dossier["roache_gci_analysis"]["signal_to_uncertainty_margin"]["saving_to_3grid_span_ratio"],
            "snr_span_eloss": w1_dossier["roache_gci_analysis"]["signal_to_uncertainty_margin"]["saving_to_3grid_eloss_span_ratio"],
            "inter_rsm_dp_u_delta_pct": w1_gate["checks"]["inter_rsm_dp_u_delta_pct"],
            "mandatory_5run_gate_passed": w1_gate["gate_passed"],
            "gate_failures": w1_gate["failures"],
        },
        {
            "candidate_id": octa_dossier["subject_candidate_id"],
            "wave": "Wave-2",
            "topology_class": octa_dossier["subject_topology_class"],
            "design_sha256": octa_dossier["subject_design_sha256"],
            "f1_cout_p95_mg_l": octa_dossier["f1_surrogate_screen_metrics"]["cout_p95_mg_l"],
            "f1_dp_u_pa": octa_dossier["f1_surrogate_screen_metrics"]["dp_u_pa"],
            "openfoam_g2_dp_u_pa": octa_dossier["grids"][2]["dp_u_pa"],
            "openfoam_g2_eloss_j_m3": octa_dossier["grids"][2]["hydraulic_loss_j_per_treated_m3"],
            "openfoam_lrr_dp_u_pa": octa_dossier["rsm_turbulence_closure_crosscheck"][0]["dp_u_pa"],
            "openfoam_ssg_dp_u_pa": octa_dossier["rsm_turbulence_closure_crosscheck"][1]["dp_u_pa"],
            "snr_span_dp_u": octa_dossier["roache_gci_analysis"]["signal_to_uncertainty_margin"]["saving_to_3grid_span_ratio"],
            "snr_span_eloss": octa_dossier["roache_gci_analysis"]["signal_to_uncertainty_margin"]["saving_to_3grid_eloss_span_ratio"],
            "inter_rsm_dp_u_delta_pct": octa_dossier["mandatory_promotion_gate"]["checks"]["inter_rsm_dp_u_delta_pct"],
            "mandatory_5run_gate_passed": octa_dossier["mandatory_promotion_gate"]["gate_passed"],
            "gate_failures": octa_dossier["mandatory_promotion_gate"]["failures"],
        },
    ]

    out_bundle = {
        "schema_version": 1,
        "kind": "wave2_elites_openfoam14_5run_gci_rsm_and_f2_oil_checkpoint_dossier",
        "claim_state": "openfoam14_3grid_gci_and_dual_rsm_carrier_verified_f2_oil_4s_full_residence_and_f4_rig_still_open",
        "qualified_multiobjective_vector": None,
        "qualified_capacity_gain": None,
        "wave2_tpms_gyroid_elite_5run_dossier": tpms_dossier,
        "wave2_octa_branch_8jet_elite_5run_dossier": octa_dossier,
        "wave1_vs_wave2_surrogate_to_cfd_ranking_audit": ranking_comparison,
        "f2_oil_transient_droplet_closure_checkpoint_and_resumable_plan": f2_oil_checkpoint,
    }

    out_path = ROOT / "docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json"
    out_path.write_text(json.dumps(out_bundle, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {out_path} (SHA-256: {sha256_file(out_path)})", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
