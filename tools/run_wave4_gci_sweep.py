#!/usr/bin/env python3
"""INT-06 & G-09: Wave-4 OpenFOAM 14 3-Grid (G0/G1/G2) + Dual 7-Eq. LRR/SSG RSM Sweep
with Same-Pipeline CT-35 MultiPatchSurface Control and IR-04 Feature Enactment Receipts.

Executes OpenFOAM 14 G0/G1/G2 verification (plus LRR/SSG 7-equation RSM on the
same-pipeline CT-35 control and top Wave-4 winner) across up to 4 parallel 4-core
container slots (16 cores total on cpus 0..15 per docs/WORKPLAN-2026-10-15.md).

Writes docs/results/wave4-gci-v1.json with:
  - Same-pipeline CT-35 baseline control (G-09) compiled through MultiPatchSurface::from_program
    / IrCompiledMesh::compile_with_extension on G0/G1/G2 + LRR/SSG.
  - Top 6 Wave-4 B3 per-seed winners (INT-06), each with full SubjectBundle identity (EV-12),
    DerivedTaxon (IR-06), FeatureEnactmentReceipt (IR-04 / IR-R1..R2, measured_disjoint_inlet_paths=8),
    and TR-07 v2 3-grid spatial diagnostic (Monotone|Oscillatory|Divergent|Indeterminate).
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "physics/openfoam/environment"))
sys.path.insert(0, str(ROOT / "tools"))

import runtime  # type: ignore
from f0_enactment import audit_stl_inlet_enactment  # type: ignore
from generate_gci_rsm_report_and_figures import (  # type: ignore
    compute_roache_3grid_gci_v2,
    extract_reynolds_stress_stats,
    load_grid_case,
)
from run_wave2_elites_openfoam14 import (  # type: ignore
    parse_internal_scalar_list,
    set_blockmesh_grid,
)

DEFAULT_WAVE4_PROOF = ROOT / "docs/results/wave4-evolution-proof-v1.json"
DEFAULT_OUT = ROOT / "docs/results/wave4-gci-v1.json"

CPU_SLOTS = [
    "0,1,2,3",
    "4,5,6,7",
    "8,9,10,11",
    "12,13,14,15",
]

GRIDS_SPEC = [
    ("G0_coarse", "g0-coarse", [18, 12, 212]),
    ("G1_medium", "g1-medium", [24, 16, 282]),
    ("G2_fine", "g2-fine", [30, 20, 352]),
]


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare_and_solve_sst_case(
    cli_source_args: list[str],
    case_dir: Path,
    grid_id: str,
    bg_cells: list[int],
    keep_time_150: bool,
    cpu_set: str,
) -> dict[str, Any]:
    if (
        (case_dir / "hydraulic-integrals.json").is_file()
        and (case_dir / "mesh-2-checkMesh.log").is_file()
        and (case_dir / "input-manifest.json").is_file()
    ):
        print(f"[skip-existing] {case_dir.name} already solved", flush=True)
        return load_grid_case(case_dir, grid_id, bg_cells)

    if case_dir.exists():
        shutil.rmtree(case_dir)

    joil_cli = ROOT / "target/release/joil-cli"
    cmd_prep = (
        [str(joil_cli)]
        if joil_cli.is_file()
        else ["cargo", "run", "--release", "-p", "joil-cli", "--"]
    )
    subprocess.run(
        [
            *cmd_prep,
            "prepare-cfd-case",
            *cli_source_args,
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
        stdout=subprocess.DEVNULL,
    )
    set_blockmesh_grid(case_dir, bg_cells[0], bg_cells[1], bg_cells[2])

    old_cpu = os.environ.get("JOIL_CPU_SET")
    os.environ["JOIL_CPU_SET"] = cpu_set
    try:
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

        runtime.run(
            case_dir,
            ["/work/runs/environment-capability-r4/bin/joilHydraulicMetrics", "-time", "150"],
            case_dir / "water-3-hydraulicMetrics-150.log",
            timeout=300,
        )
    finally:
        if old_cpu is None:
            os.environ.pop("JOIL_CPU_SET", None)
        else:
            os.environ["JOIL_CPU_SET"] = old_cpu

    if not keep_time_150:
        shutil.rmtree(case_dir / "150", ignore_errors=True)
        shutil.rmtree(case_dir / "constant/polyMesh", ignore_errors=True)

    return load_grid_case(case_dir, grid_id, bg_cells)


def prepare_and_solve_rsm_case(
    cli_source_args: list[str],
    g1_case_dir: Path,
    rsm_case_dir: Path,
    turbulence_model: str,
    base_dp_u: float,
    base_eloss: float,
    g1_dp_u: float,
    g1_eloss: float,
    cpu_set: str,
) -> dict[str, Any]:
    summary_cache = rsm_case_dir / "reynolds-stress-summary.json"
    if (rsm_case_dir / "hydraulic-integrals.json").is_file() and summary_cache.is_file():
        print(f"[skip-existing] {rsm_case_dir.name} already solved", flush=True)
        hyd = json.loads((rsm_case_dir / "hydraulic-integrals.json").read_text())
        r_stats = json.loads(summary_cache.read_text())
    else:
        if rsm_case_dir.exists():
            shutil.rmtree(rsm_case_dir)

        joil_cli = ROOT / "target/release/joil-cli"
        cmd_prep = (
            [str(joil_cli)]
            if joil_cli.is_file()
            else ["cargo", "run", "--release", "-p", "joil-cli", "--"]
        )
        subprocess.run(
            [
                *cmd_prep,
                "prepare-cfd-case",
                *cli_source_args,
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
            stdout=subprocess.DEVNULL,
        )
        shutil.copytree(
            g1_case_dir / "constant/polyMesh", rsm_case_dir / "constant/polyMesh"
        )
        for fname in ("U", "p", "phi", "k", "omega", "nut"):
            src = g1_case_dir / "150" / fname
            if src.is_file():
                shutil.copy2(src, rsm_case_dir / "0" / fname)

        k_arr = parse_internal_scalar_list(g1_case_dir / "150/k")
        om_arr = parse_internal_scalar_list(g1_case_dir / "150/omega")
        n_cells = len(k_arr)
        eps_arr = (0.09 * k_arr * om_arr).clip(min=1e-6)
        r_diag_arr = ((2.0 / 3.0) * k_arr).clip(min=1e-6)

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
        r_lines = "\n".join(f"({v:.8e} 0 0 {v:.8e} 0 {v:.8e})" for v in r_diag_arr)
        (rsm_case_dir / "0/R").write_text(
            f"FoamFile {{ format ascii; class volSymmTensorField; object R; }}\n"
            f"dimensions [0 2 -2 0 0 0 0]; internalField nonuniform List<symmTensor>\n"
            f"{n_cells}\n(\n{r_lines}\n);\n\n{bf_r}",
            encoding="utf-8",
        )

        old_cpu = os.environ.get("JOIL_CPU_SET")
        os.environ["JOIL_CPU_SET"] = cpu_set
        try:
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
        finally:
            if old_cpu is None:
                os.environ.pop("JOIL_CPU_SET", None)
            else:
                os.environ["JOIL_CPU_SET"] = old_cpu

        r_stats = extract_reynolds_stress_stats(rsm_case_dir / "60/R")
        summary_cache.write_text(json.dumps(r_stats, indent=2) + "\n", encoding="utf-8")
        hyd = json.loads((rsm_case_dir / "hydraulic-integrals.json").read_text())

        shutil.rmtree(rsm_case_dir / "constant/polyMesh", ignore_errors=True)
        for f_clean in ("U", "p", "phi", "k", "omega", "nut", "epsilon", "R"):
            (rsm_case_dir / "0" / f_clean).unlink(missing_ok=True)
            (rsm_case_dir / "60" / f_clean).unlink(missing_ok=True)
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
        "delta_dp_u_vs_same_pipeline_ct35_g1_pct": (dp_u - base_dp_u) / base_dp_u * 100.0,
        "delta_eloss_vs_same_pipeline_ct35_g1_pct": (eloss - base_eloss) / base_eloss * 100.0,
        "reynolds_stress_tensor_diagnostics": r_stats,
        "hydraulic_integrals_sha256": sha256_file(
            rsm_case_dir / "hydraulic-integrals.json"
        ),
    }


def run_subject_3grid(
    subject_spec: dict[str, Any],
    cpu_set: str,
) -> dict[str, Any]:
    prefix = subject_spec["prefix"]
    cli_args = subject_spec["cli_args"]
    run_rsm = bool(subject_spec.get("run_rsm", False))

    g0_dir = ROOT / f"runs/{prefix}-g0-coarse"
    g1_dir = ROOT / f"runs/{prefix}-g1-medium"
    g2_dir = ROOT / f"runs/{prefix}-g2-fine"

    g0 = prepare_and_solve_sst_case(
        cli_args, g0_dir, "G0_coarse", [18, 12, 212], keep_time_150=False, cpu_set=cpu_set
    )
    g1 = prepare_and_solve_sst_case(
        cli_args, g1_dir, "G1_medium", [24, 16, 282], keep_time_150=run_rsm, cpu_set=cpu_set
    )
    g2 = prepare_and_solve_sst_case(
        cli_args, g2_dir, "G2_fine", [30, 20, 352], keep_time_150=False, cpu_set=cpu_set
    )

    manifest = json.loads((g2_dir / "input-manifest.json").read_text(encoding="utf-8"))
    prog_obj = json.loads((g2_dir / "design-program.json").read_text(encoding="utf-8"))
    f0_audit = audit_stl_inlet_enactment(
        g2_dir / "constant/triSurface/ct35.stl", prog_obj
    )

    gci_dp_u = compute_roache_3grid_gci_v2(
        float(g0["dp_u_pa"]),
        float(g1["dp_u_pa"]),
        float(g2["dp_u_pa"]),
        float(g0["effective_cell_size_mm"]),
        float(g1["effective_cell_size_mm"]),
        float(g2["effective_cell_size_mm"]),
    )
    gci_eloss = compute_roache_3grid_gci_v2(
        float(g0["hydraulic_loss_j_per_treated_m3"]),
        float(g1["hydraulic_loss_j_per_treated_m3"]),
        float(g2["hydraulic_loss_j_per_treated_m3"]),
        float(g0["effective_cell_size_mm"]),
        float(g1["effective_cell_size_mm"]),
        float(g2["effective_cell_size_mm"]),
    )
    gci_split = compute_roache_3grid_gci_v2(
        float(g0["reject_fraction"]),
        float(g1["reject_fraction"]),
        float(g2["reject_fraction"]),
        float(g0["effective_cell_size_mm"]),
        float(g1["effective_cell_size_mm"]),
        float(g2["effective_cell_size_mm"]),
    )

    return {
        "subject_id": subject_spec["subject_id"],
        "seed": subject_spec.get("seed"),
        "arm": subject_spec.get("arm"),
        "rank_in_arm": subject_spec.get("rank_in_arm"),
        "prefix": prefix,
        "design_sha256": manifest["design_sha256"],
        "design_ir_sha256": manifest.get("design_ir_sha256"),
        "surface_stl_sha256": manifest["surface_stl_sha256"],
        "derived_taxon": manifest.get("derived_taxon"),
        "enactment_receipt": manifest.get("enactment_receipt"),
        "independent_f0_enactment_audit": f0_audit,
        "subject_bundle": manifest.get("subject_bundle"),
        "f1_sandbox_screen_metrics": subject_spec.get("f1_metrics"),
        "grids": [g0, g1, g2],
        "roache_gci_v2_analysis": {
            "dp_u_pa": gci_dp_u,
            "hydraulic_loss_j_per_treated_m3": gci_eloss,
            "reject_fraction": gci_split,
        },
        "_run_rsm": run_rsm,
        "_cli_args": cli_args,
    }


def _worker_entry(task: tuple[dict[str, Any], str]) -> dict[str, Any]:
    spec, cpu_set = task
    return run_subject_3grid(spec, cpu_set)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wave4-proof", type=Path, default=DEFAULT_WAVE4_PROOF)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--top-k", type=int, default=6)
    parser.add_argument("--verify-existing", action="store_true")
    args = parser.parse_args()

    if args.verify_existing:
        if not args.out.is_file():
            print(f"ERROR: missing {args.out}", file=sys.stderr)
            return 1
        data = json.loads(args.out.read_text(encoding="utf-8"))
        ctrl = data.get("same_pipeline_ct35_control", {})
        winners = data.get("wave4_b3_top_winners", [])
        if len(ctrl.get("grids", [])) != 3 or len(winners) < 6:
            print("ERROR: incomplete wave4-gci-v1.json", file=sys.stderr)
            return 1
        for w in winners:
            rec = w.get("enactment_receipt") or {}
            if not rec.get("all_active_features_enacted") or int(rec.get("measured_disjoint_inlet_paths", 0)) < 2:
                print(f"ERROR: enactment check failed for {w.get('subject_id')}", file=sys.stderr)
                return 1
        print(
            json.dumps(
                {
                    "status": "ok",
                    "out": str(args.out.relative_to(ROOT)),
                    "sha256": sha256_file(args.out),
                    "n_winners": len(winners),
                    "same_pipeline_ct35_g2_dp_u_pa": ctrl["grids"][2]["dp_u_pa"],
                    "top_winner_id": winners[0]["subject_id"],
                    "top_winner_g2_dp_u_pa": winners[0]["grids"][2]["dp_u_pa"],
                },
                indent=2,
            )
        )
        return 0

    w4_raw = args.wave4_proof.read_bytes()
    w4_sha = hashlib.sha256(w4_raw).hexdigest()
    w4 = json.loads(w4_raw.decode("utf-8"))

    b3_outcomes = sorted(
        w4["arm_outcomes"]["B3_Wave3_Exotic3D_QD_CEI"],
        key=lambda r: float(r["final_best_kpi_score"]),
        reverse=True,
    )[: args.top_k]

    prog_tmp_dir = ROOT / ".private/int06-programs"
    prog_tmp_dir.mkdir(parents=True, exist_ok=True)

    ct35_spec: dict[str, Any] = {
        "subject_id": "baseline-ct35-same-pipeline-control",
        "seed": None,
        "arm": "B0_CT35_Same_Pipeline_MultiPatchSurface_Control",
        "rank_in_arm": 0,
        "prefix": "wave4-ct35-control",
        "cli_args": ["--baseline-ct35"],
        "run_rsm": True,
        "f1_metrics": {
            "kpi_score": float(w4["baseline_b0_ct35_kpi_score"]),
            "cout_sample_screen_upper_mg_l": float(w4["baseline_b0_ct35_cout_p95_mg_l"]),
            "dp_u_pa": float(w4["baseline_b0_ct35"]["nominal_metrics"]["dp_u_pa"]),
        },
    }

    winner_specs: list[dict[str, Any]] = []
    for idx, row in enumerate(b3_outcomes):
        bc = row["best_candidate"]
        seed = int(row["seed"])
        cid = str(bc["id"])
        prog_path = prog_tmp_dir / f"wave4-b3-s{seed}-{cid}.json"
        prog_path.write_text(json.dumps(bc["program"], indent=2) + "\n", encoding="utf-8")
        winner_specs.append(
            {
                "subject_id": cid,
                "seed": seed,
                "arm": "B3_Wave3_Exotic3D_QD_CEI",
                "rank_in_arm": idx + 1,
                "prefix": f"wave4-b3-s{seed}",
                "cli_args": ["--program-json", str(prog_path)],
                "run_rsm": (idx == 0),
                "f1_metrics": {
                    "kpi_score": float(row["final_best_kpi_score"]),
                    "cout_sample_screen_upper_mg_l": float(
                        bc["mc_robustness"]["cout_sample_screen_upper_mg_l"]
                    ),
                    "dp_u_pa": float(bc["nominal_metrics"]["dp_u_pa"]),
                },
            }
        )

    all_specs = [ct35_spec, *winner_specs]
    results_by_prefix: dict[str, dict[str, Any]] = {}

    # Execute in batches of up to 4 subjects in parallel (4 x 4 cores = 16 cores max)
    for batch_start in range(0, len(all_specs), len(CPU_SLOTS)):
        batch = all_specs[batch_start : batch_start + len(CPU_SLOTS)]
        tasks = [(spec, CPU_SLOTS[i]) for i, spec in enumerate(batch)]
        with concurrent.futures.ProcessPoolExecutor(max_workers=len(tasks)) as pool:
            for res in pool.map(_worker_entry, tasks):
                results_by_prefix[res["prefix"]] = res

    ct35_res = results_by_prefix[ct35_spec["prefix"]]
    ct35_g1_dp = float(ct35_res["grids"][1]["dp_u_pa"])
    ct35_g1_el = float(ct35_res["grids"][1]["hydraulic_loss_j_per_treated_m3"])
    ct35_g2_dp = float(ct35_res["grids"][2]["dp_u_pa"])
    ct35_g2_el = float(ct35_res["grids"][2]["hydraulic_loss_j_per_treated_m3"])

    # Run dual 7-equation LRR + SSG RSM continuation on G1 for CT-35 control & #1 Wave-4 winner
    rsm_subjects = [s for s in all_specs if s.get("run_rsm")]
    for spec in rsm_subjects:
        pfx = spec["prefix"]
        subj = results_by_prefix[pfx]
        g1_dir = ROOT / f"runs/{pfx}-g1-medium"
        lrr_dir = ROOT / f"runs/{pfx}-g1-rsm-lrr"
        ssg_dir = ROOT / f"runs/{pfx}-g1-rsm-ssg"
        g1_dp = float(subj["grids"][1]["dp_u_pa"])
        g1_el = float(subj["grids"][1]["hydraulic_loss_j_per_treated_m3"])
        lrr = prepare_and_solve_rsm_case(
            spec["cli_args"],
            g1_dir,
            lrr_dir,
            "LRR",
            ct35_g1_dp,
            ct35_g1_el,
            g1_dp,
            g1_el,
            CPU_SLOTS[0],
        )
        ssg = prepare_and_solve_rsm_case(
            spec["cli_args"],
            g1_dir,
            ssg_dir,
            "SSG",
            ct35_g1_dp,
            ct35_g1_el,
            g1_dp,
            g1_el,
            CPU_SLOTS[1],
        )
        subj["rsm_turbulence_closure_crosscheck"] = [lrr, ssg]
        shutil.rmtree(g1_dir / "150", ignore_errors=True)
        shutil.rmtree(g1_dir / "constant/polyMesh", ignore_errors=True)

    # Compute comparative deltas against BOTH same-pipeline CT-35 (G-09) and historical extended CT-35
    hist_bench = json.loads(
        (ROOT / "docs/results/exotic-vs-baseline-cfd-benchmark-v1.json").read_text(
            encoding="utf-8"
        )
    )["baseline_ct35_openfoam14_reference"]
    hist_dp_u = float(hist_bench["static_tap_drop_pa"])
    hist_eloss = float(hist_bench["device_hydraulic_loss_j_m3"])

    for pfx, subj in results_by_prefix.items():
        subj.pop("_run_rsm", None)
        subj.pop("_cli_args", None)
        g2_dp = float(subj["grids"][2]["dp_u_pa"])
        g2_el = float(subj["grids"][2]["hydraulic_loss_j_per_treated_m3"])
        span_dp = float(
            subj["roache_gci_v2_analysis"]["dp_u_pa"]["three_grid_extremum_span"]
        )
        span_el = float(
            subj["roache_gci_v2_analysis"]["hydraulic_loss_j_per_treated_m3"][
                "three_grid_extremum_span"
            ]
        )
        dp_sav_same = ct35_g2_dp - g2_dp
        el_sav_same = ct35_g2_el - g2_el
        dp_sav_hist = hist_dp_u - g2_dp
        el_sav_hist = hist_eloss - g2_el
        subj["comparative_margins"] = {
            "vs_same_pipeline_ct35_control_g2": {
                "ct35_control_g2_dp_u_pa": ct35_g2_dp,
                "ct35_control_g2_eloss_j_m3": ct35_g2_el,
                "fine_grid_dp_u_saving_pa": dp_sav_same,
                "fine_grid_dp_u_saving_pct": (g2_dp - ct35_g2_dp) / ct35_g2_dp * 100.0,
                "three_grid_dp_u_span_pa": span_dp,
                "saving_to_3grid_span_ratio_dp_u": abs(dp_sav_same) / max(span_dp, 1e-9),
                "fine_grid_eloss_saving_j_m3": el_sav_same,
                "fine_grid_eloss_saving_pct": (g2_el - ct35_g2_el) / ct35_g2_el * 100.0,
                "three_grid_eloss_span_j_m3": span_el,
                "saving_to_3grid_span_ratio_eloss": abs(el_sav_same) / max(span_el, 1e-9),
            },
            "vs_historical_extended_ct35_reference": {
                "historical_ct35_dp_u_pa": hist_dp_u,
                "historical_ct35_eloss_j_m3": hist_eloss,
                "fine_grid_dp_u_saving_pa": dp_sav_hist,
                "fine_grid_dp_u_saving_pct": (g2_dp - hist_dp_u) / hist_dp_u * 100.0,
                "saving_to_3grid_span_ratio_dp_u": dp_sav_hist / max(span_dp, 1e-9),
                "fine_grid_eloss_saving_j_m3": el_sav_hist,
                "fine_grid_eloss_saving_pct": (g2_el - hist_eloss) / hist_eloss * 100.0,
                "saving_to_3grid_span_ratio_eloss": el_sav_hist / max(span_el, 1e-9),
            },
        }

    winner_list = [results_by_prefix[s["prefix"]] for s in winner_specs]

    out_payload = {
        "schema_version": 2,
        "kind": (
            "wave4_int06_openfoam14_3grid_spatial_diagnostic_same_pipeline_ct35_control_and_enactment_receipts"
        ),
        "source_wave4_proof_relpath": str(args.wave4_proof.relative_to(ROOT)),
        "source_wave4_proof_sha256": w4_sha,
        "openfoam_image_id": "sha256:8866eda29be2bcbc05e0fa9536cacb3c63ff25c97179e058959ffbeb90da7199",
        "gci_policy_note": (
            "TR-07 / INT-05 / INT-06 v2: 3-grid sequences classify Monotone|Oscillatory|Divergent|Indeterminate "
            "without clamping observed_order_p to [0.5, 4.0] and never certify standard Fs=1.25 Roache GCI "
            "(asymptotic_range_certified=False; gci21_standard_fs125_pct=None). Includes same-pipeline CT-35 "
            "MultiPatchSurface/IrCompiledMesh control (G-09) and IR-04/IR-R1..R2 8-jet enactment receipts."
        ),
        "same_pipeline_ct35_control": ct35_res,
        "wave4_b3_top_winners": winner_list,
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(out_payload, indent=2) + "\n", encoding="utf-8")
    print(
        json.dumps(
            {
                "status": "completed",
                "out": str(args.out.relative_to(ROOT)),
                "sha256": sha256_file(args.out),
                "same_pipeline_ct35_g2_dp_u_pa": ct35_g2_dp,
                "n_winners": len(winner_list),
                "winner_g2_dp_u_pa": [
                    {
                        "id": w["subject_id"],
                        "seed": w["seed"],
                        "inlet_paths": w["enactment_receipt"]["measured_disjoint_inlet_paths"],
                        "g2_dp_u_pa": w["grids"][2]["dp_u_pa"],
                        "gci_state_dp_u": w["roache_gci_v2_analysis"]["dp_u_pa"][
                            "gci_validity_state"
                        ],
                    }
                    for w in winner_list
                ],
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
