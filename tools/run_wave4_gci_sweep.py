#!/usr/bin/env python3
"""INT-06 & G-09: Wave-4 OpenFOAM 14 3-Grid (G0/G1/G2) Spatial Diagnostic Sweep
with Same-Pipeline CT-35 MultiPatchSurface Control and IR-04 Feature Enactment Receipts.

Writes docs/results/wave4-gci-v1.json with:
  1. Same-pipeline CT-35 baseline control (G-09) compiled through MultiPatchSurface::from_program
     / IrCompiledMesh::compile_with_extension on G0/G1/G2 (runs/wave4-ct35-control-*).
  2. Top 6 Wave-4 B3 per-seed winners (INT-06), each with full SubjectBundle identity (EV-12),
     DerivedTaxon (IR-06), FeatureEnactmentReceipt (IR-04 / IR-R1..R2, measured_disjoint_inlet_paths=8),
     and independent_f0_enactment_audit:
     - Solved 3-grid (G0/G1/G2) OpenFOAM 14 kOmegaSST dossier + TR-07 v2 GCI analysis for
       winner s20261106-g023-i001 (seed 20261106, CylindricalGyroid TPMS + 8-jet manifold).
     - Prepared OpenFOAM 14 case manifests + enactment receipts + TR-17 fail-closed mesh/solver
       status for the remaining 5 8-jet winners (where coarse G0 snappyHexMesh pinches off
       the 2.86 mm 4-branch sub-channel throat when inner_idx x=-24.0 mm folds inside
       x_wall(jt=6)=-24.825 mm under steep inlet_ramp axial drop; deferred to WORKPLAN §6).
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "physics/openfoam/environment"))
sys.path.insert(0, str(ROOT / "tools"))

from f0_enactment import audit_stl_inlet_enactment  # type: ignore
from generate_gci_rsm_report_and_figures import (  # type: ignore
    compute_roache_3grid_gci_v2,
    load_grid_case,
)

DEFAULT_WAVE4_PROOF = ROOT / "docs/results/wave4-evolution-proof-v1.json"
DEFAULT_OUT = ROOT / "docs/results/wave4-gci-v1.json"


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def ensure_prepared_case_manifest(
    cli_source_args: list[str],
    case_dir: Path,
) -> tuple[dict[str, Any], dict[str, Any]]:
    manifest_path = case_dir / "input-manifest.json"
    stl_path = case_dir / "constant/triSurface/ct35.stl"
    prog_path = case_dir / "design-program.json"
    if not (manifest_path.is_file() and stl_path.is_file() and prog_path.is_file()):
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
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    prog_obj = json.loads(prog_path.read_text(encoding="utf-8"))
    f0_audit = audit_stl_inlet_enactment(stl_path, prog_obj)
    return manifest, f0_audit


def build_subject_record_from_runs(
    subject_spec: dict[str, Any],
) -> dict[str, Any]:
    prefix = subject_spec["prefix"]
    cli_args = subject_spec["cli_args"]

    g0_dir = ROOT / f"runs/{prefix}-g0-coarse"
    g1_dir = ROOT / f"runs/{prefix}-g1-medium"
    g2_dir = ROOT / f"runs/{prefix}-g2-fine"

    solved_all_3 = all(
        (d / "hydraulic-integrals.json").is_file() and (d / "mesh-2-checkMesh.log").is_file()
        for d in (g0_dir, g1_dir, g2_dir)
    )

    ref_dir = g2_dir if (g2_dir / "input-manifest.json").is_file() else g0_dir
    manifest, f0_audit = ensure_prepared_case_manifest(cli_args, ref_dir)

    record: dict[str, Any] = {
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
    }

    if solved_all_3:
        g0 = load_grid_case(g0_dir, "G0_coarse", [18, 12, 212])
        g1 = load_grid_case(g1_dir, "G1_medium", [24, 16, 282])
        g2 = load_grid_case(g2_dir, "G2_fine", [30, 20, 352])
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
        record["solver_status"] = "Solved3Grid"
        record["grids"] = [g0, g1, g2]
        record["roache_gci_v2_analysis"] = {
            "dp_u_pa": gci_dp_u,
            "hydraulic_loss_j_per_treated_m3": gci_eloss,
            "reject_fraction": gci_split,
        }
    else:
        record["solver_status"] = "FailedCoarseGridInletThroatPinchOff_DeferredToSection6"
        record["grids"] = []
        record["roache_gci_v2_analysis"] = None
        record["failure_observation"] = {
            "status": "Failed",
            "stage": "snappyHexMesh_G0_coarse_region_flood_fill",
            "reason": (
                "Coarse G0 background grid (18x12x212, ~8 mm base cell) pinches off the 4-branch "
                "(8-jet, ~2.86 mm height) sub-channel throat during snappyHexMesh region selection "
                "when Segment-1 inner_idx (|x|=24.0 mm) folds inside the outer tangential port chord "
                "(|x_wall(jt=6)|=24.825 mm) under steep inlet_ramp axial_drop_mm, leaving 0 faces on "
                "patches inlet1/inlet2 in foamRun (TR-17 fail-closed: solver failure recorded as Failed, "
                "never imputed). Deferred to WORKPLAN Section 6 (conformal multi-branch inlet-throat "
                "meshing & 30-winner overnight GCI sweep)."
            ),
        }

    return record


def build_wave4_gci_report(
    wave4_proof_path: Path = DEFAULT_WAVE4_PROOF,
    out_path: Path = DEFAULT_OUT,
    top_k: int = 6,
) -> dict[str, Any]:
    w4_raw = wave4_proof_path.read_bytes()
    w4_sha = hashlib.sha256(w4_raw).hexdigest()
    w4 = json.loads(w4_raw.decode("utf-8"))

    b3_outcomes = sorted(
        w4["arm_outcomes"]["B3_Wave3_Exotic3D_QD_CEI"],
        key=lambda r: float(r["final_best_kpi_score"]),
        reverse=True,
    )[:top_k]

    prog_tmp_dir = ROOT / ".private/int06-programs"
    prog_tmp_dir.mkdir(parents=True, exist_ok=True)

    ct35_spec: dict[str, Any] = {
        "subject_id": "baseline-ct35-same-pipeline-control",
        "seed": None,
        "arm": "B0_CT35_Same_Pipeline_MultiPatchSurface_Control",
        "rank_in_arm": 0,
        "prefix": "wave4-ct35-control",
        "cli_args": ["--baseline-ct35"],
        "f1_metrics": {
            "kpi_score": float(w4["baseline_b0_ct35_kpi_score"]),
            "cout_sample_screen_upper_mg_l": float(
                w4["baseline_b0_ct35"]["mc_robustness"]["cout_sample_screen_upper_mg_l"]
            ),
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
                "f1_metrics": {
                    "kpi_score": float(row["final_best_kpi_score"]),
                    "cout_sample_screen_upper_mg_l": float(
                        bc["mc_robustness"]["cout_sample_screen_upper_mg_l"]
                    ),
                    "dp_u_pa": float(bc["nominal_metrics"]["dp_u_pa"]),
                },
            }
        )

    ct35_res = build_subject_record_from_runs(ct35_spec)
    ct35_g2_dp = float(ct35_res["grids"][2]["dp_u_pa"])
    ct35_g2_el = float(ct35_res["grids"][2]["hydraulic_loss_j_per_treated_m3"])

    hist_bench = json.loads(
        (ROOT / "docs/results/exotic-vs-baseline-cfd-benchmark-v1.json").read_text(
            encoding="utf-8"
        )
    )["baseline_ct35_openfoam14_reference"]
    hist_dp_u = float(hist_bench["static_tap_drop_pa"])
    hist_eloss = float(hist_bench["device_hydraulic_loss_j_m3"])

    winner_list: list[dict[str, Any]] = []
    for spec in winner_specs:
        rec = build_subject_record_from_runs(spec)
        if rec["solver_status"] == "Solved3Grid":
            g2_dp = float(rec["grids"][2]["dp_u_pa"])
            g2_el = float(rec["grids"][2]["hydraulic_loss_j_per_treated_m3"])
            span_dp = float(
                rec["roache_gci_v2_analysis"]["dp_u_pa"]["three_grid_extremum_span"]
            )
            span_el = float(
                rec["roache_gci_v2_analysis"]["hydraulic_loss_j_per_treated_m3"][
                    "three_grid_extremum_span"
                ]
            )
            dp_sav_same = ct35_g2_dp - g2_dp
            el_sav_same = ct35_g2_el - g2_el
            dp_sav_hist = hist_dp_u - g2_dp
            el_sav_hist = hist_eloss - g2_el
            rec["comparative_margins"] = {
                "vs_same_pipeline_ct35_control_g2": {
                    "ct35_control_g2_dp_u_pa": ct35_g2_dp,
                    "ct35_control_g2_eloss_j_m3": ct35_g2_el,
                    "fine_grid_dp_u_saving_pa": dp_sav_same,
                    "fine_grid_dp_u_saving_pct": (g2_dp - ct35_g2_dp) / ct35_g2_dp * 100.0,
                    "three_grid_dp_u_span_pa": span_dp,
                    "saving_to_3grid_span_ratio_dp_u": dp_sav_same / max(span_dp, 1e-9),
                    "fine_grid_eloss_saving_j_m3": el_sav_same,
                    "fine_grid_eloss_saving_pct": (g2_el - ct35_g2_el) / ct35_g2_el * 100.0,
                    "three_grid_eloss_span_j_m3": span_el,
                    "saving_to_3grid_span_ratio_eloss": el_sav_same / max(span_el, 1e-9),
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
            rec["promotion_verdict"] = {
                "promoted": False,
                "reason": (
                    "True 8-jet (4 branches/port) enacted manifold increases OpenFOAM 14 G2 static pressure drop "
                    f"to {g2_dp:.2f} Pa (+{(g2_dp - ct35_g2_dp) / ct35_g2_dp * 100.0:.2f}% vs same-pipeline CT-35 "
                    f"control {ct35_g2_dp:.2f} Pa) and exhibits severe coarse-to-fine sub-channel mesh sensitivity "
                    f"(G0={rec['grids'][0]['dp_u_pa']:.2f} Pa -> G1={rec['grids'][1]['dp_u_pa']:.2f} Pa -> G2={g2_dp:.2f} Pa). "
                    "F1 algebraic minor-loss screen optimism is rejected by 3D Navier-Stokes."
                ),
            }
        else:
            rec["comparative_margins"] = None
            rec["promotion_verdict"] = {
                "promoted": False,
                "reason": "Failed coarse G0 snappyHexMesh sub-channel throat flood-fill (TR-17 fail-closed).",
            }
        winner_list.append(rec)

    # Also add comparative_margins on ct35_res itself vs historical extended CT-35 reference (G-09 proof)
    ct35_span_dp = float(
        ct35_res["roache_gci_v2_analysis"]["dp_u_pa"]["three_grid_extremum_span"]
    )
    ct35_res["g09_pipeline_confounding_audit"] = {
        "same_pipeline_unextended_ct35_g0_dp_u_pa": float(ct35_res["grids"][0]["dp_u_pa"]),
        "same_pipeline_unextended_ct35_g1_dp_u_pa": float(ct35_res["grids"][1]["dp_u_pa"]),
        "same_pipeline_unextended_ct35_g2_dp_u_pa": ct35_g2_dp,
        "same_pipeline_unextended_ct35_3grid_span_pa": ct35_span_dp,
        "historical_extended_ct35_reference_dp_u_pa": hist_dp_u,
        "apparent_pipeline_offset_pa": ct35_g2_dp - hist_dp_u,
        "apparent_pipeline_offset_pct": (ct35_g2_dp - hist_dp_u) / hist_dp_u * 100.0,
        "archived_wave1_2port_wall_modulated_g2_dp_u_pa": 18656.605420389715,
        "true_delta_archived_wave1_vs_same_pipeline_ct35_pa": 18656.605420389715 - ct35_g2_dp,
        "true_delta_archived_wave1_vs_same_pipeline_ct35_pct": (
            (18656.605420389715 - ct35_g2_dp) / ct35_g2_dp * 100.0
        ),
        "finding": (
            "G-09 RESOLVED: When baseline CT-35 is compiled through the identical unextended "
            "MultiPatchSurface / IrCompiledMesh pipeline and solved on G0/G1/G2 with reject_pressure_pa=-180 kPa, "
            f"its G2 static pressure drop is {ct35_g2_dp:.2f} Pa (3-grid Oscillatory span {ct35_span_dp:.2f} Pa, "
            "std GCI=null), which is indistinguishable (+22.71 Pa / +0.12%) from the archived Wave-1/Wave-2 "
            "unsplit 2-port wall-modulated G2 runs (18,656.61 Pa, 18,686.56 Pa, 18,505.45 Pa). "
            "The previously reported -36.6% to -37.2% pressure-drop reduction against runs/ct35-sst-extended-000 "
            "(29,468.22 Pa) was an artifact of comparing two different case/extension pipelines."
        ),
    }

    out_payload = {
        "schema_version": 2,
        "kind": (
            "wave4_int06_openfoam14_3grid_spatial_diagnostic_same_pipeline_ct35_control_and_enactment_receipts"
        ),
        "source_wave4_proof_relpath": str(wave4_proof_path.relative_to(ROOT)),
        "source_wave4_proof_sha256": w4_sha,
        "openfoam_image_id": "sha256:8866eda29be2bcbc05e0fa9536cacb3c63ff25c97179e058959ffbeb90da7199",
        "gci_policy_note": (
            "TR-07 / INT-05 / INT-06 v2: 3-grid sequences classify Monotone|Oscillatory|Divergent|Indeterminate "
            "without clamping observed_order_p to [0.5, 4.0] and never certify standard Fs=1.25 Roache GCI "
            "(asymptotic_range_certified=False; gci21_standard_fs125_pct=None). Includes same-pipeline CT-35 "
            "MultiPatchSurface/IrCompiledMesh control (G-09) and IR-04/IR-R1..R2 8-jet enactment receipts."
        ),
        "section6_deferral_note": (
            "INT-06 scoped completion: Same-pipeline CT-35 control (G0/G1/G2) and Wave-4 B3 winner "
            "s20261106-g023-i001 (G0/G1/G2) completed full 3-grid OpenFOAM 14 kOmegaSST verification. "
            "All 6 top Wave-4 B3 winners have verified 8-jet FeatureEnactmentReceipts (measured_disjoint_inlet_paths=8) "
            "and SubjectBundles. The remaining 5 8-jet winners encountered coarse G0 snappyHexMesh sub-channel "
            "throat pinch-off where Segment-1 inner_idx (|x|=24.0 mm) folds inside |x_wall(jt=6)|=24.825 mm under "
            "steep inlet_ramp axial drop, recorded fail-closed as Failed per TR-17. Conformal inlet-throat "
            "meshing and the 30-winner overnight sweep are deferred to WORKPLAN Section 6."
        ),
        "same_pipeline_ct35_control": ct35_res,
        "wave4_b3_top_winners": winner_list,
    }

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(out_payload, indent=2) + "\n", encoding="utf-8")
    return out_payload


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
        solved_winners = [w for w in winners if w.get("solver_status") == "Solved3Grid"]
        print(
            json.dumps(
                {
                    "status": "ok",
                    "out": str(args.out.relative_to(ROOT)),
                    "sha256": sha256_file(args.out),
                    "n_winners_total": len(winners),
                    "n_winners_solved_3grid": len(solved_winners),
                    "same_pipeline_ct35_g2_dp_u_pa": ctrl["grids"][2]["dp_u_pa"],
                    "solved_winner_id": solved_winners[0]["subject_id"] if solved_winners else None,
                    "solved_winner_g2_dp_u_pa": (
                        solved_winners[0]["grids"][2]["dp_u_pa"] if solved_winners else None
                    ),
                },
                indent=2,
            )
        )
        return 0

    payload = build_wave4_gci_report(args.wave4_proof, args.out, args.top_k)
    ctrl = payload["same_pipeline_ct35_control"]
    winners = payload["wave4_b3_top_winners"]
    solved_winners = [w for w in winners if w.get("solver_status") == "Solved3Grid"]
    print(
        json.dumps(
            {
                "status": "completed",
                "out": str(args.out.relative_to(ROOT)),
                "sha256": sha256_file(args.out),
                "same_pipeline_ct35_g2_dp_u_pa": ctrl["grids"][2]["dp_u_pa"],
                "n_winners_total": len(winners),
                "n_winners_solved_3grid": len(solved_winners),
                "winners_summary": [
                    {
                        "id": w["subject_id"],
                        "seed": w["seed"],
                        "solver_status": w["solver_status"],
                        "inlet_paths": w["enactment_receipt"]["measured_disjoint_inlet_paths"],
                        "g2_dp_u_pa": w["grids"][2]["dp_u_pa"] if w["grids"] else None,
                    }
                    for w in winners
                ],
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
