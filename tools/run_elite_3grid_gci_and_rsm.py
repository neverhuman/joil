#!/usr/bin/env python3
"""Mandatory 3-Grid Roache GCI (G0/G1/G2) + Dual 7-Eq. LRR/SSG RSM Promotion Gate.

Per docs/WAVE2_PLAN.md Section 3.5 & experiments/wave2-preregistration-v1.json:
No candidate from any seed or topology family may be promoted as a CFD-verified
elite unless it passes all five OpenFOAM 14 verification runs and gates:
  1. 3-Grid OpenFOAM 14 sequence (G0 ~140k, G1 ~305k, G2 ~550k cells) with monotonic
     cell growth (N_G0 < N_G1 < N_G2), effective cell size shrinkage (h_G0 > h_G1 > h_G2),
     water continuity closure < 1e-3, and matched reject split r in [1.90%, 2.10%].
  2. Roache GCI & Signal-to-Span Certification:
     SNR_span = |dp_u(CT35) - dp_u(G2)| / max_span(G0,G1,G2) >= 3.0.
  3. Dual 7-Equation LRR + SSG RSM Cross-Check on G1:
     - Both LRR and SSG present with water_closure_fraction < 1e-3
     - 100.0% diagonal normal-stress realizability (positive_definite_normal_stress_fraction == 1.0)
     - Inter-RSM agreement |dp_u(LRR) - dp_u(SSG)| / dp_u(G1_mean_RSM) <= 5.0%
     - Both LRR and SSG confirm >= 25.0% pressure-drop reduction vs CT-35 baseline.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_GCI_RSM_DOSSIER = ROOT / "docs/results/elite-3grid-gci-and-rsm-verification-v1.json"
DEFAULT_PREREG = ROOT / "experiments/wave2-preregistration-v1.json"
DEFAULT_WAVE2_ABLATION = ROOT / "docs/results/wave2-multiseed-ablation-and-holdout-v1.json"


def evaluate_elite_gci_rsm_gate(
    dossier: dict[str, Any],
    prereg: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Evaluate mandatory 3-grid Roache GCI + dual LRR/SSG RSM promotion criteria."""
    failures: list[str] = []
    checks: dict[str, Any] = {}

    gate_cfg = (prereg or {}).get("mandatory_elite_gci_and_rsm_gate", {})
    max_water_closure = float(gate_cfg.get("max_water_closure_residual", 1.0e-3))
    r_bounds = gate_cfg.get("matched_reject_split_bounds", [0.019, 0.021])
    min_r, max_r = float(r_bounds[0]), float(r_bounds[1])
    min_snr_span = float(gate_cfg.get("min_signal_to_3grid_span_ratio", 3.0))
    min_realizability = float(
        gate_cfg.get("min_normal_stress_realizability_fraction", 1.0)
    )
    max_rsm_delta_pct = float(gate_cfg.get("max_inter_rsm_delta_pct", 5.0))

    grids = dossier.get("grids", [])
    if len(grids) != 3:
        failures.append(f"expected exactly 3 grids (G0, G1, G2), found {len(grids)}")
    else:
        g0, g1, g2 = grids[0], grids[1], grids[2]
        cells = [int(g.get("cells", 0)) for g in grids]
        h_mm = [float(g.get("effective_cell_size_mm", 0.0)) for g in grids]
        checks["grid_cells"] = cells
        checks["effective_cell_size_mm"] = h_mm
        if not (cells[0] < cells[1] < cells[2]):
            failures.append(f"non-monotonic grid cell counts: {cells}")
        if not (h_mm[0] > h_mm[1] > h_mm[2] > 0.0):
            failures.append(f"non-monotonic effective cell sizes (mm): {h_mm}")

        for g in grids:
            gid = g.get("grid_id", "unknown")
            wc = float(g.get("water_closure_fraction", 1.0))
            rf = float(g.get("reject_fraction", 0.0))
            if wc > max_water_closure:
                failures.append(
                    f"grid {gid} water_closure_fraction {wc:.3e} > {max_water_closure:.3e}"
                )
            if not (min_r <= rf <= max_r):
                failures.append(
                    f"grid {gid} reject_fraction {rf:.6f} outside [{min_r:.4f}, {max_r:.4f}]"
                )

    gci = dossier.get("roache_gci_analysis", {})
    margin = gci.get("signal_to_uncertainty_margin", {})
    snr_dp = float(margin.get("saving_to_3grid_span_ratio", 0.0))
    snr_eloss = float(margin.get("saving_to_3grid_eloss_span_ratio", 0.0))
    checks["saving_to_3grid_span_ratio_dp_u"] = snr_dp
    checks["saving_to_3grid_span_ratio_eloss"] = snr_eloss
    if snr_dp < min_snr_span:
        failures.append(
            f"dp_u signal-to-span ratio {snr_dp:.3f} < required {min_snr_span:.3f}"
        )
    if snr_eloss < min_snr_span:
        failures.append(
            f"e_loss signal-to_span ratio {snr_eloss:.3f} < required {min_snr_span:.3f}"
        )

    valid_states = {"Monotone", "Oscillatory", "Divergent", "Asymptotic", "Indeterminate"}
    for metric_key in ("dp_u_pa", "hydraulic_loss_j_per_treated_m3", "reject_fraction"):
        m_block = gci.get(metric_key, {})
        if isinstance(m_block, dict) and "gci_validity_state" in m_block:
            st = m_block.get("gci_validity_state")
            checks[f"gci_validity_state_{metric_key}"] = st
            if st not in valid_states:
                failures.append(f"invalid gci_validity_state for {metric_key}: {st}")
            if len(grids) == 3 and bool(m_block.get("asymptotic_range_certified", False)):
                failures.append(
                    f"3-grid sequence for {metric_key} cannot certify asymptotic_range_certified=True"
                )
            if int(dossier.get("schema_version", 1)) >= 2:
                if len(grids) == 3 and m_block.get("gci21_standard_fs125_pct") is not None:
                    failures.append(
                        f"v2 3-grid sequence for {metric_key} must have gci21_standard_fs125_pct=None"
                    )
                if st in {"Oscillatory", "Indeterminate"} and m_block.get("observed_order_p") is not None:
                    failures.append(
                        f"v2 {st} sequence for {metric_key} must have observed_order_p=None"
                    )

    rsm_list = dossier.get("rsm_turbulence_closure_crosscheck", [])
    by_model = {str(item.get("turbulence_model")): item for item in rsm_list}
    required_models = gate_cfg.get("required_rsm_closures", ["LRR", "SSG"])
    for model_name in required_models:
        if model_name not in by_model:
            failures.append(f"missing required 7-equation RSM closure: {model_name}")
            continue
        entry = by_model[model_name]
        wc = float(entry.get("water_closure_fraction", 1.0))
        rf = float(entry.get("reject_fraction", 0.0))
        diag = entry.get("reynolds_stress_tensor_diagnostics", {})
        pos_frac = float(diag.get("positive_definite_normal_stress_fraction", 0.0))
        saving_pct = float(entry.get("delta_dp_u_vs_ct35_baseline_pct", 0.0))
        if wc > max_water_closure:
            failures.append(
                f"RSM {model_name} water_closure_fraction {wc:.3e} > {max_water_closure:.3e}"
            )
        if not (min_r <= rf <= max_r):
            failures.append(
                f"RSM {model_name} reject_fraction {rf:.6f} outside [{min_r:.4f}, {max_r:.4f}]"
            )
        if pos_frac < min_realizability:
            failures.append(
                f"RSM {model_name} realizability fraction {pos_frac:.6f} < {min_realizability:.6f}"
            )
        if saving_pct > -25.0:
            failures.append(
                f"RSM {model_name} dp_u saving vs CT-35 ({saving_pct:.2f}%) fails >= 25% reduction gate"
            )

    if "LRR" in by_model and "SSG" in by_model:
        dp_lrr = float(by_model["LRR"]["dp_u_pa"])
        dp_ssg = float(by_model["SSG"]["dp_u_pa"])
        el_lrr = float(by_model["LRR"]["hydraulic_loss_j_per_treated_m3"])
        el_ssg = float(by_model["SSG"]["hydraulic_loss_j_per_treated_m3"])
        mean_dp = 0.5 * (dp_lrr + dp_ssg)
        mean_el = 0.5 * (el_lrr + el_ssg)
        inter_rsm_dp_pct = 100.0 * abs(dp_lrr - dp_ssg) / max(mean_dp, 1e-9)
        inter_rsm_el_pct = 100.0 * abs(el_lrr - el_ssg) / max(mean_el, 1e-9)
        checks["inter_rsm_dp_u_delta_pct"] = inter_rsm_dp_pct
        checks["inter_rsm_eloss_delta_pct"] = inter_rsm_el_pct
        if inter_rsm_dp_pct > max_rsm_delta_pct:
            failures.append(
                f"inter-RSM (LRR vs SSG) dp_u delta {inter_rsm_dp_pct:.3f}% > {max_rsm_delta_pct:.3f}%"
            )
        if inter_rsm_el_pct > max_rsm_delta_pct:
            failures.append(
                f"inter-RSM (LRR vs SSG) e_loss delta {inter_rsm_el_pct:.3f}% > {max_rsm_delta_pct:.3f}%"
            )

    return {
        "subject_candidate_id": dossier.get("subject_candidate_id"),
        "subject_topology_class": dossier.get("subject_topology_class"),
        "gate_passed": len(failures) == 0,
        "checks": checks,
        "failures": failures,
    }


def recompute_dossier_gci_v2(
    dossier_v1: dict[str, Any], prereg: dict[str, Any] | None = None
) -> dict[str, Any]:
    """Recompute a 3-grid + dual RSM dossier under TR-07 / INT-05 v2 GCI rules."""
    import copy

    sys.path.insert(0, str(ROOT / "tools"))
    from generate_gci_rsm_report_and_figures import compute_roache_3grid_gci_v2  # type: ignore

    out = copy.deepcopy(dossier_v1)
    out["schema_version"] = 2
    out["kind"] = (
        "openfoam14_3grid_spatial_diagnostic_and_rsm_turbulence_screen_not_asymptotic_gci_or_physical_validation"
    )
    out["gci_policy_note"] = (
        "TR-07 / INT-05 v2: 3-grid sequences classify Monotone|Oscillatory|Divergent|Indeterminate "
        "without clamping observed_order_p to [0.5, 4.0] and never certify standard Fs=1.25 Roache GCI "
        "(asymptotic_range_certified=False; gci21_standard_fs125_pct=None). Promotion relies on "
        "model-free 3-grid extremum span SNR >= 3.0 and dual 7-equation LRR/SSG RSM realizability."
    )
    grids = out["grids"]
    g0, g1, g2 = grids[0], grids[1], grids[2]
    h0 = float(g0["effective_cell_size_mm"])
    h1 = float(g1["effective_cell_size_mm"])
    h2 = float(g2["effective_cell_size_mm"])

    gci_dp_u = compute_roache_3grid_gci_v2(
        float(g0["dp_u_pa"]), float(g1["dp_u_pa"]), float(g2["dp_u_pa"]), h0, h1, h2
    )
    gci_eloss = compute_roache_3grid_gci_v2(
        float(g0["hydraulic_loss_j_per_treated_m3"]),
        float(g1["hydraulic_loss_j_per_treated_m3"]),
        float(g2["hydraulic_loss_j_per_treated_m3"]),
        h0,
        h1,
        h2,
    )
    gci_split = compute_roache_3grid_gci_v2(
        float(g0["reject_fraction"]),
        float(g1["reject_fraction"]),
        float(g2["reject_fraction"]),
        h0,
        h1,
        h2,
    )
    out["roache_gci_analysis"]["dp_u_pa"] = gci_dp_u
    out["roache_gci_analysis"]["hydraulic_loss_j_per_treated_m3"] = gci_eloss
    out["roache_gci_analysis"]["reject_fraction"] = gci_split
    if "mandatory_promotion_gate" in out:
        out["mandatory_promotion_gate"] = evaluate_elite_gci_rsm_gate(out, prereg)
    return out


def generate_int05_v2_reports(prereg: dict[str, Any] | None = None) -> dict[str, str]:
    """Generate INT-05 v2 GCI reports over the existing 15-run Wave-1/Wave-2 OpenFOAM 14 set."""
    import copy

    v1_w1_path = ROOT / "docs/results/elite-3grid-gci-and-rsm-verification-v1.json"
    v2_w1_path = ROOT / "docs/results/elite-3grid-gci-and-rsm-verification-v2.json"
    v1_w1 = json.loads(v1_w1_path.read_text(encoding="utf-8"))
    v2_w1 = recompute_dossier_gci_v2(v1_w1, prereg)
    v2_w1["mandatory_promotion_gate"] = evaluate_elite_gci_rsm_gate(v2_w1, prereg)
    v2_w1_path.write_text(json.dumps(v2_w1, indent=2) + "\n", encoding="utf-8")

    v1_w2_path = ROOT / "docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json"
    v2_w2_path = ROOT / "docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json"
    v1_w2 = json.loads(v1_w2_path.read_text(encoding="utf-8"))
    v2_w2 = copy.deepcopy(v1_w2)
    v2_w2["schema_version"] = 2
    v2_w2["kind"] = (
        "wave2_promoted_elites_5case_openfoam14_v2_spatial_diagnostic_rsm_and_f2_oil_checkpoint"
    )
    v2_w2["source_v1_sha256"] = hashlib.sha256(v1_w2_path.read_bytes()).hexdigest()
    for key in (
        "wave2_tpms_gyroid_elite_5run_dossier",
        "wave2_octa_branch_8jet_elite_5run_dossier",
    ):
        v2_w2[key] = recompute_dossier_gci_v2(v1_w2[key], prereg)

    # Also attach the Wave-1 6-jet v2 summary into cross_wave_elite_cfd_ranking_audit
    rank_table = v2_w2.get("cross_wave_elite_cfd_ranking_audit", {}).get(
        "elite_comparison_table", []
    )
    dossiers_by_id = {
        v2_w1["subject_candidate_id"]: v2_w1,
        v2_w2["wave2_tpms_gyroid_elite_5run_dossier"]["subject_candidate_id"]: v2_w2[
            "wave2_tpms_gyroid_elite_5run_dossier"
        ],
        v2_w2["wave2_octa_branch_8jet_elite_5run_dossier"]["subject_candidate_id"]: v2_w2[
            "wave2_octa_branch_8jet_elite_5run_dossier"
        ],
    }
    for row in rank_table:
        cid = row.get("candidate_id")
        if cid in dossiers_by_id:
            dp_gci = dossiers_by_id[cid]["roache_gci_analysis"]["dp_u_pa"]
            row["gci_validity_state_dp_u"] = dp_gci["gci_validity_state"]
            row["asymptotic_range_certified"] = dp_gci["asymptotic_range_certified"]
            row["observed_order_p_dp_u"] = dp_gci["observed_order_p"]
            row["gci21_standard_fs125_pct"] = dp_gci["gci21_standard_fs125_pct"]
            row["gci21_conservative_fs300_p1_pct"] = dp_gci[
                "gci21_conservative_fs300_p1_pct"
            ]

    v2_w2_path.write_text(json.dumps(v2_w2, indent=2) + "\n", encoding="utf-8")
    return {
        str(v2_w1_path.relative_to(ROOT)): hashlib.sha256(
            v2_w1_path.read_bytes()
        ).hexdigest(),
        str(v2_w2_path.relative_to(ROOT)): hashlib.sha256(
            v2_w2_path.read_bytes()
        ).hexdigest(),
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verify or execute mandatory 3-grid Roache GCI + dual LRR/SSG RSM promotion gate."
    )
    parser.add_argument(
        "--verify-existing",
        action="store_true",
        help="Audit the committed 3-grid GCI + dual RSM dossier and Wave-2 pre-registration contract.",
    )
    parser.add_argument(
        "--generate-v2",
        action="store_true",
        help="Generate INT-05 / TR-07 v2 GCI reports over the existing 15-run Wave-1/Wave-2 OpenFOAM 14 set.",
    )
    parser.add_argument(
        "--dossier",
        type=Path,
        default=DEFAULT_GCI_RSM_DOSSIER,
        help="Path to elite 3-grid GCI + RSM verification JSON dossier.",
    )
    parser.add_argument(
        "--prereg",
        type=Path,
        default=DEFAULT_PREREG,
        help="Path to Wave-2 pre-registration JSON contract.",
    )
    parser.add_argument(
        "--wave2-ablation",
        type=Path,
        default=DEFAULT_WAVE2_ABLATION,
        help="Path to Wave-2 10-seed ablation and holdout JSON dossier.",
    )
    args = parser.parse_args()

    prereg = (
        json.loads(args.prereg.read_text(encoding="utf-8"))
        if args.prereg.is_file()
        else None
    )
    if args.generate_v2:
        digests = generate_int05_v2_reports(prereg)
        print(json.dumps({"generated_v2_reports": digests}, indent=2))

    if not args.dossier.is_file():
        print(f"FAIL: missing GCI/RSM dossier: {args.dossier}", file=sys.stderr)
        return 1

    dossier = json.loads(args.dossier.read_text(encoding="utf-8"))
    result = evaluate_elite_gci_rsm_gate(dossier, prereg)

    if args.verify_existing and args.wave2_ablation.is_file():
        ablation = json.loads(args.wave2_ablation.read_text(encoding="utf-8"))
        audit = ablation.get("rejected_cohort_10pct_audit", {})
        if not audit.get("audit_passed", False):
            result["gate_passed"] = False
            result["failures"].append("Wave-2 10% rejected-cohort audit did not pass")
        result["checks"]["wave2_rejected_cohort_audit_passed"] = bool(
            audit.get("audit_passed", False)
        )
        result["checks"]["wave2_dossier_sha256"] = hashlib.sha256(
            args.wave2_ablation.read_bytes()
        ).hexdigest()

    for w2_path in (
        ROOT / "docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json",
        ROOT / "docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json",
    ):
        if args.verify_existing and w2_path.is_file():
            w2_bundle = json.loads(w2_path.read_text(encoding="utf-8"))
            for key in (
                "wave2_tpms_gyroid_elite_5run_dossier",
                "wave2_octa_branch_8jet_elite_5run_dossier",
            ):
                sub_d = w2_bundle.get(key)
                if not isinstance(sub_d, dict):
                    result["gate_passed"] = False
                    result["failures"].append(f"missing {key} in {w2_path.name}")
                    continue
                sub_res = evaluate_elite_gci_rsm_gate(sub_d, prereg)
                recorded_gate = sub_d.get("mandatory_promotion_gate", {})
                if bool(recorded_gate.get("gate_passed")) != bool(sub_res["gate_passed"]):
                    result["gate_passed"] = False
                    result["failures"].append(
                        f"gate_passed mismatch for {key} in {w2_path.name}: recorded={recorded_gate.get('gate_passed')} vs evaluated={sub_res['gate_passed']}"
                    )
                result["checks"][f"{w2_path.stem}_{key}_gate_passed"] = sub_res["gate_passed"]
                result["checks"][f"{key}_gate_passed"] = sub_res["gate_passed"]
                result["checks"][f"{key}_snr_dp_u"] = sub_res["checks"].get(
                    "saving_to_3grid_span_ratio_dp_u", 0.0
                )
                result["checks"][f"{key}_inter_rsm_dp_u_delta_pct"] = sub_res[
                    "checks"
                ].get("inter_rsm_dp_u_delta_pct", 0.0)

    v2_w1_path = ROOT / "docs/results/elite-3grid-gci-and-rsm-verification-v2.json"
    if args.verify_existing and v2_w1_path.is_file():
        v2_w1_res = evaluate_elite_gci_rsm_gate(
            json.loads(v2_w1_path.read_text(encoding="utf-8")), prereg
        )
        if not v2_w1_res["gate_passed"]:
            result["gate_passed"] = False
            result["failures"].extend(v2_w1_res["failures"])
        result["checks"]["v2_wave1_elite_gate_passed"] = v2_w1_res["gate_passed"]

    if not result["gate_passed"]:
        print(json.dumps(result, indent=2))
        return 1

    print(
        f"PASS: elite GCI+RSM gate certified for {result['subject_candidate_id']} "
        f"(SNR_dp={result['checks']['saving_to_3grid_span_ratio_dp_u']:.2f}x, "
        f"SNR_eloss={result['checks']['saving_to_3grid_span_ratio_eloss']:.2f}x, "
        f"LRR-vs-SSG dp_u delta={result['checks']['inter_rsm_dp_u_delta_pct']:.3f}%)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

