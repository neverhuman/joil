#!/usr/bin/env python3
"""Mandatory 3-Grid (G0/G1/G2) + Dual 7-Eq. LRR/SSG RSM elite CFD admission gate.

Historical v1 description (docs/WAVE2_PLAN.md Section 3.5): five OpenFOAM 14 runs with
monotonic refinement, water continuity closure < 1e-3, and matched reject split r in [1.90%, 2.10%].

Corrected behaviour (V5-01-01). Configuration comes only from the frozen preregistration block
``mandatory_elite_cfd_gates`` plus ``hard_feasibility_gates.reject_split_band``; a missing block,
missing or unknown key, or malformed value raises GateConfigError and never falls through to
defaults. Observations that are missing, non-numeric or non-finite are failures. Admission needs
a same-pipeline CT-35 control with its own three grids on the candidate's grid ladder, an equal
pipeline identity (compiler, mesher, boundary/controller, taps, solver build, extractor digests),
per-grid subject binding and the preregistered solver version. The signal-to-span ratio is
recomputed from grid values:

    SNR_span = |dp_u(control, finest) - dp_u(candidate, finest)| / span(candidate G0, G1, G2)

and a recorded ratio must reproduce it. An admitted result is a resolved same-pipeline numerical
difference, not an improvement, ranking or physical qualification. No improvement threshold is
applied. The historical runner constraints (water closure, split band, e_loss checks) are kept
only as named non-preregistered policy that can add failures.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import re
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_GCI_RSM_DOSSIER = ROOT / "docs/results/elite-3grid-gci-and-rsm-verification-v1.json"
DEFAULT_PREREG = ROOT / "experiments/wave2-preregistration-v1.json"
DEFAULT_WAVE2_ABLATION = ROOT / "docs/results/wave2-multiseed-ablation-and-holdout-v1.json"
W1_V1 = "docs/results/elite-3grid-gci-and-rsm-verification-v1.json"
W1_V2 = "docs/results/elite-3grid-gci-and-rsm-verification-v2.json"
W2_V1 = "docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json"
W2_V2 = "docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json"
INT06_CONTROL = "docs/results/wave4-gci-v1.json"
W2_SUBDOSSIERS = ("wave2_tpms_gyroid_elite_5run_dossier", "wave2_octa_branch_8jet_elite_5run_dossier")

GATE_CONFIG_KEY = "mandatory_elite_cfd_gates"
REQUIRED_GATE_KEYS = frozenset({
    "openfoam_version",
    "three_grid_levels",
    "min_signal_to_3grid_span_ratio",
    "rsm_closures_required",
    "min_rsm_normal_stress_realizability",
    "max_inter_rsm_dp_discrepancy_pct",
})
PIPELINE_IDENTITY_KEYS = ("compiler", "mesher", "boundary_controller", "taps", "solver_build", "extractor")
RUNNER_POLICY_NOT_PREREGISTERED = {
    "max_water_closure_residual": 1.0e-3,
    "matched_reject_split_bounds": [0.019, 0.021],
    "eloss_signal_to_span_and_inter_rsm_reuse_preregistered_dp_thresholds": True,
}
INTERPRETATION = (
    "admitted means a resolved same-pipeline numerical difference; not an improvement, "
    "ranking or physical qualification"
)
SHA256 = re.compile(r"[0-9a-f]{64}")
GRID_ORDINAL = re.compile(r"G\d+")


class GateConfigError(ValueError):
    """The preregistered gate configuration is missing or malformed."""


def _canonical_sha256(value: Any) -> str:
    text = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def _is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def _ordinal(label: Any) -> str | None:
    if not isinstance(label, str):
        return None
    token = label.split("_", 1)[0]
    return token if GRID_ORDINAL.fullmatch(token) else None


def load_elite_cfd_gate_config(prereg: Any) -> dict[str, Any]:
    """Return the normalized preregistered gate configuration or raise GateConfigError."""
    if not isinstance(prereg, dict) or not isinstance(prereg.get(GATE_CONFIG_KEY), dict):
        raise GateConfigError(f"preregistration lacks the {GATE_CONFIG_KEY} block")
    protocol_id = prereg.get("protocol_id")
    if not isinstance(protocol_id, str) or not protocol_id:
        raise GateConfigError("preregistration lacks protocol_id")
    block = prereg[GATE_CONFIG_KEY]
    missing = sorted(REQUIRED_GATE_KEYS - set(block))
    unknown = sorted(set(block) - REQUIRED_GATE_KEYS)
    if missing:
        raise GateConfigError(f"{GATE_CONFIG_KEY} missing required keys: {missing}")
    if unknown:
        raise GateConfigError(f"{GATE_CONFIG_KEY} has unknown keys: {unknown}")

    version = block["openfoam_version"]
    if not isinstance(version, str) or not version:
        raise GateConfigError("openfoam_version must be a non-empty string")
    levels = block["three_grid_levels"]
    ordinals = [_ordinal(x) for x in levels] if isinstance(levels, list) else []
    if len(ordinals) != 3 or None in ordinals or len(set(ordinals)) != 3:
        raise GateConfigError(f"three_grid_levels must be three labels with unique G<n> ordinals: {levels!r}")
    snr = block["min_signal_to_3grid_span_ratio"]
    if not _is_number(snr) or snr <= 0:
        raise GateConfigError(f"min_signal_to_3grid_span_ratio must be a finite number > 0: {snr!r}")
    realizability = block["min_rsm_normal_stress_realizability"]
    if not _is_number(realizability) or not 0 < realizability <= 1:
        raise GateConfigError(
            f"min_rsm_normal_stress_realizability must be a finite number in (0, 1]: {realizability!r}"
        )
    inter_rsm = block["max_inter_rsm_dp_discrepancy_pct"]
    if not _is_number(inter_rsm) or inter_rsm < 0:
        raise GateConfigError(f"max_inter_rsm_dp_discrepancy_pct must be a finite number >= 0: {inter_rsm!r}")
    closures = block["rsm_closures_required"]
    if (not isinstance(closures, list) or not closures
            or any(not isinstance(c, str) or not c for c in closures) or len(set(closures)) != len(closures)):
        raise GateConfigError(f"rsm_closures_required must be a non-empty list of unique names: {closures!r}")

    feasibility = prereg.get("hard_feasibility_gates")
    band = feasibility.get("reject_split_band") if isinstance(feasibility, dict) else None
    if (not isinstance(band, list) or len(band) != 2 or not all(_is_number(x) for x in band)
            or not 0 <= band[0] < band[1] < 1):
        raise GateConfigError(f"hard_feasibility_gates.reject_split_band must be [lo, hi], 0 <= lo < hi < 1: {band!r}")

    return {
        "openfoam_version": version,
        "three_grid_levels": list(levels),
        "grid_ordinals": ordinals,
        "min_signal_to_3grid_span_ratio": float(snr),
        "rsm_closures_required": list(closures),
        "min_rsm_normal_stress_realizability": float(realizability),
        "max_inter_rsm_dp_discrepancy_pct": float(inter_rsm),
        "reject_split_band": [float(band[0]), float(band[1])],
        "provenance": {
            "protocol_id": protocol_id,
            "config_key": GATE_CONFIG_KEY,
            "block_sha256": _canonical_sha256(block),
            "split_band_source": "hard_feasibility_gates.reject_split_band",
        },
    }


def _pipeline_identity_ok(identity: Any) -> bool:
    return (isinstance(identity, dict) and set(identity) == set(PIPELINE_IDENTITY_KEYS)
            and all(isinstance(v, str) and SHA256.fullmatch(v) for v in identity.values()))


def evaluate_elite_gci_rsm_gate(
    dossier: dict[str, Any],
    prereg: dict[str, Any] | None = None,
    control: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Evaluate admission of one elite dossier against its same-pipeline CT-35 control.

    ``control`` replaces ``dossier["baseline_ct35_comparator"]`` when given. Raises
    GateConfigError for missing/malformed preregistration; configuration is not an observation.
    """
    cfg = load_elite_cfd_gate_config(prereg)
    failures: list[str] = []
    checks: dict[str, Any] = {}
    policy = RUNNER_POLICY_NOT_PREREGISTERED
    lo, hi = cfg["reject_split_band"]
    policy_lo, policy_hi = policy["matched_reject_split_bounds"]
    max_water_closure = policy["max_water_closure_residual"]
    min_snr_span = cfg["min_signal_to_3grid_span_ratio"]
    min_realizability = cfg["min_rsm_normal_stress_realizability"]
    max_rsm_delta_pct = cfg["max_inter_rsm_dp_discrepancy_pct"]

    def num(label: str, obj: Any, key: str) -> float | None:
        if not isinstance(obj, dict) or key not in obj:
            failures.append(f"{label} {key} missing")
            return None
        value = obj[key]
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            failures.append(f"{label} {key} is not a number: {value!r}")
            return None
        if not math.isfinite(value):
            failures.append(f"{label} {key} is non-finite: {value!r}")
            return None
        return float(value)

    def check_flow_quality(label: str, entry: Any) -> None:
        wc = num(label, entry, "water_closure_fraction")
        rf = num(label, entry, "reject_fraction")
        if rf is not None and not lo <= rf <= hi:
            failures.append(f"{label} reject_fraction {rf:.6f} outside preregistered [{lo:.4f}, {hi:.4f}]")
        if rf is not None and not policy_lo <= rf <= policy_hi:
            failures.append(
                f"{label} reject_fraction {rf:.6f} outside runner policy (not preregistered) "
                f"[{policy_lo:.4f}, {policy_hi:.4f}]"
            )
        if wc is not None and wc > max_water_closure:
            failures.append(
                f"{label} water_closure_fraction {wc:.3e} > runner policy (not preregistered) {max_water_closure:.3e}"
            )

    def check_ladder(label: str, grids: Any, subject: Any) -> bool:
        if not isinstance(grids, list) or len(grids) != 3 or not all(isinstance(g, dict) for g in grids):
            count = len(grids) if isinstance(grids, list) else 0
            failures.append(f"{label} expected exactly 3 grids {cfg['grid_ordinals']}, found {count}")
            return False
        found = [_ordinal(g.get("grid_id")) for g in grids]
        if found != cfg["grid_ordinals"]:
            failures.append(f"{label} grid ladder {found} does not match preregistered {cfg['grid_ordinals']}")
        subject_ok = isinstance(subject, str) and SHA256.fullmatch(subject) is not None
        for g in grids:
            if not subject_ok or g.get("subject_design_sha256") != subject:
                prefix = "comparator grid" if label == "comparator" else "grid"
                failures.append(f"{prefix} {g.get('grid_id', 'unknown')} subject identity not bound to {label} subject")
        return True

    subject = dossier.get("subject_design_sha256")
    grids = dossier.get("grids")
    ladder_ok = check_ladder("candidate", grids, subject)
    dp: list[float | None] = [None, None, None]
    el: list[float | None] = [None, None, None]
    if ladder_ok:
        cells = [num(f"grid {g.get('grid_id', 'unknown')}", g, "cells") for g in grids]
        h_mm = [num(f"grid {g.get('grid_id', 'unknown')}", g, "effective_cell_size_mm") for g in grids]
        checks["grid_cells"] = cells
        checks["effective_cell_size_mm"] = h_mm
        if None not in cells and not cells[0] < cells[1] < cells[2]:
            failures.append(f"non-monotonic grid cell counts: {cells}")
        if None not in h_mm and not h_mm[0] > h_mm[1] > h_mm[2] > 0.0:
            failures.append(f"non-monotonic effective cell sizes (mm): {h_mm}")
        for i, g in enumerate(grids):
            label = f"grid {g.get('grid_id', 'unknown')}"
            check_flow_quality(label, g)
            dp[i] = num(label, g, "dp_u_pa")
            el[i] = num(label, g, "hydraulic_loss_j_per_treated_m3")

    if dossier.get("openfoam_version") != cfg["openfoam_version"]:
        failures.append(
            f"solver version not bound: dossier openfoam_version {dossier.get('openfoam_version')!r} "
            f"!= preregistered {cfg['openfoam_version']!r}"
        )
    if not _pipeline_identity_ok(dossier.get("pipeline_identity")):
        failures.append(f"candidate pipeline identity not bound: need exact digests {list(PIPELINE_IDENTITY_KEYS)}")

    comparator = control if control is not None else dossier.get("baseline_ct35_comparator")
    checks["comparator_source"] = "control_argument" if control is not None else "baseline_ct35_comparator"
    cmp_dp: float | None = None
    cmp_el: float | None = None
    if not isinstance(comparator, dict):
        failures.append("comparator missing: no same-pipeline CT-35 control supplied")
    else:
        if not _pipeline_identity_ok(comparator.get("pipeline_identity")):
            failures.append(f"comparator pipeline identity not bound: need exact digests {list(PIPELINE_IDENTITY_KEYS)}")
        elif comparator["pipeline_identity"] != dossier.get("pipeline_identity"):
            differing = sorted(k for k in PIPELINE_IDENTITY_KEYS
                               if comparator["pipeline_identity"].get(k)
                               != (dossier.get("pipeline_identity") or {}).get(k))
            failures.append(f"comparator pipeline identity differs from candidate: {differing}")
        if comparator.get("openfoam_version") != cfg["openfoam_version"]:
            failures.append(
                f"solver version not bound: comparator openfoam_version {comparator.get('openfoam_version')!r}"
            )
        cmp_grids = comparator.get("grids")
        if cmp_grids is None:
            failures.append("comparator 3-grid ladder not bound: comparator has no grids (single-point reference)")
        elif check_ladder("comparator", cmp_grids, comparator.get("subject_design_sha256")):
            if ladder_ok:
                for g, c in zip(grids, cmp_grids):
                    if g.get("background_hex_grid") != c.get("background_hex_grid"):
                        failures.append(
                            f"comparator grid {c.get('grid_id', 'unknown')} background_hex_grid "
                            f"{c.get('background_hex_grid')!r} != candidate {g.get('background_hex_grid')!r}"
                        )
            for c in cmp_grids:
                check_flow_quality(f"comparator grid {c.get('grid_id', 'unknown')}", c)
            finest = cmp_grids[-1]
            cmp_dp = num(f"comparator grid {finest.get('grid_id', 'unknown')}", finest, "dp_u_pa")
            cmp_el = num(f"comparator grid {finest.get('grid_id', 'unknown')}", finest, "hydraulic_loss_j_per_treated_m3")

    gci = dossier.get("roache_gci_analysis")
    gci = gci if isinstance(gci, dict) else {}
    margin = gci.get("signal_to_uncertainty_margin")
    for metric, values, ref, ratio_key, recorded_key, preregistered in (
        ("dp_u", dp, cmp_dp, "saving_to_3grid_span_ratio_dp_u", "saving_to_3grid_span_ratio", True),
        ("e_loss", el, cmp_el, "saving_to_3grid_span_ratio_eloss", "saving_to_3grid_eloss_span_ratio", False),
    ):
        ratio = None
        if None not in values:
            span = max(values) - min(values)
            checks[f"candidate_3grid_{metric}_span"] = span
            if not span > 0.0:
                field = "dp_u_pa" if metric == "dp_u" else "hydraulic_loss_j_per_treated_m3"
                failures.append(f"3-grid {field} span {span!r} is not positive; signal-to-span undefined")
            elif ref is not None:
                difference = ref - values[-1]
                checks[f"comparator_minus_candidate_finest_{metric}_{'pa' if metric == 'dp_u' else 'j_per_m3'}"] = difference
                ratio = abs(difference) / span
        checks[ratio_key] = ratio
        if ratio is not None and ratio < min_snr_span:
            source = "preregistered" if preregistered else "runner policy (not preregistered) reusing preregistered"
            failures.append(f"{metric} signal-to-span ratio {ratio:.3f} < required {min_snr_span:.3f} ({source})")
        if isinstance(margin, dict) and recorded_key in margin:
            recorded = num("recorded signal_to_uncertainty_margin", margin, recorded_key)
            if recorded is not None and (ratio is None or not math.isclose(recorded, ratio, rel_tol=1e-9, abs_tol=0.0)):
                failures.append(
                    f"recorded {recorded_key} {recorded!r} does not reproduce from grids and comparator ({ratio!r})"
                )

    valid_states = {"Monotone", "Oscillatory", "Divergent", "Asymptotic", "Indeterminate"}
    schema_version = dossier.get("schema_version", 1)
    if isinstance(schema_version, bool) or not isinstance(schema_version, int):
        failures.append(f"schema_version is not an integer: {schema_version!r}")
        schema_version = 1
    for metric_key in ("dp_u_pa", "hydraulic_loss_j_per_treated_m3", "reject_fraction"):
        m_block = gci.get(metric_key, {})
        if isinstance(m_block, dict) and "gci_validity_state" in m_block:
            st = m_block.get("gci_validity_state")
            checks[f"gci_validity_state_{metric_key}"] = st
            if st not in valid_states:
                failures.append(f"invalid gci_validity_state for {metric_key}: {st}")
            if ladder_ok and bool(m_block.get("asymptotic_range_certified", False)):
                failures.append(
                    f"3-grid sequence for {metric_key} cannot certify asymptotic_range_certified=True"
                )
            if schema_version >= 2:
                if ladder_ok and m_block.get("gci21_standard_fs125_pct") is not None:
                    failures.append(
                        f"v2 3-grid sequence for {metric_key} must have gci21_standard_fs125_pct=None"
                    )
                if st in {"Oscillatory", "Indeterminate"} and m_block.get("observed_order_p") is not None:
                    failures.append(
                        f"v2 {st} sequence for {metric_key} must have observed_order_p=None"
                    )

    rsm_list = dossier.get("rsm_turbulence_closure_crosscheck")
    rsm_list = rsm_list if isinstance(rsm_list, list) else []
    by_model = {str(item.get("turbulence_model")): item for item in rsm_list if isinstance(item, dict)}
    rsm_dp: dict[str, float] = {}
    rsm_el: dict[str, float] = {}
    for model_name in cfg["rsm_closures_required"]:
        if model_name not in by_model:
            failures.append(f"missing required 7-equation RSM closure: {model_name}")
            continue
        entry = by_model[model_name]
        label = f"RSM {model_name}"
        check_flow_quality(label, entry)
        pos_frac = num(label, entry.get("reynolds_stress_tensor_diagnostics"), "positive_definite_normal_stress_fraction")
        if pos_frac is not None and pos_frac < min_realizability:
            failures.append(f"{label} realizability fraction {pos_frac:.6f} < {min_realizability:.6f}")
        value = num(label, entry, "dp_u_pa")
        if value is not None:
            rsm_dp[model_name] = value
        value = num(label, entry, "hydraulic_loss_j_per_treated_m3")
        if value is not None:
            rsm_el[model_name] = value

    closures = cfg["rsm_closures_required"]
    pairs = [(a, b) for i, a in enumerate(closures) for b in closures[i + 1:]]
    for metric, values, check_key, preregistered in (
        ("dp_u", rsm_dp, "inter_rsm_dp_u_delta_pct", True),
        ("e_loss", rsm_el, "inter_rsm_eloss_delta_pct", False),
    ):
        worst = None
        for a, b in pairs:
            if a not in values or b not in values:
                continue
            mean = 0.5 * (values[a] + values[b])
            if not mean > 0.0:
                failures.append(f"inter-RSM ({a} vs {b}) {metric} non-positive mean {mean!r}; discrepancy undefined")
                continue
            pct = 100.0 * abs(values[a] - values[b]) / mean
            checks[f"inter_rsm_{metric}_delta_pct_{a}_{b}"] = pct
            worst = pct if worst is None else max(worst, pct)
            if pct > max_rsm_delta_pct:
                source = "preregistered" if preregistered else "runner policy (not preregistered)"
                failures.append(f"inter-RSM ({a} vs {b}) {metric} delta {pct:.3f}% > {max_rsm_delta_pct:.3f}% ({source})")
        checks[check_key] = worst

    admitted = not failures
    return {
        "subject_candidate_id": dossier.get("subject_candidate_id"),
        "subject_topology_class": dossier.get("subject_topology_class"),
        "admission": "admitted" if admitted else "not_admitted",
        "gate_passed": admitted,
        "interpretation": INTERPRETATION,
        "gate_config": cfg["provenance"],
        "runner_policy_constraints_not_preregistered": copy.deepcopy(policy),
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


def generate_int05_v2_reports(prereg: dict[str, Any] | None, out_dir: Path) -> dict[str, str]:
    """Generate INT-05 v2 GCI reports over the existing 15-run Wave-1/Wave-2 OpenFOAM 14 set.

    Writes into a fresh ``out_dir`` only; the committed docs/results v2 reports are historical
    records and are never rewritten.
    """
    load_elite_cfd_gate_config(prereg)
    out_dir = Path(out_dir)
    if out_dir.exists():
        raise FileExistsError(f"refusing existing output directory: {out_dir}")

    v1_w1_path = ROOT / W1_V1
    v2_w1_path = out_dir / Path(W1_V2).name
    v1_w1 = json.loads(v1_w1_path.read_text(encoding="utf-8"))
    v2_w1 = recompute_dossier_gci_v2(v1_w1, prereg)
    v2_w1["mandatory_promotion_gate"] = evaluate_elite_gci_rsm_gate(v2_w1, prereg)
    out_dir.mkdir(parents=True)
    with v2_w1_path.open("x", encoding="utf-8") as fh:
        fh.write(json.dumps(v2_w1, indent=2) + "\n")

    v1_w2_path = ROOT / W2_V1
    v2_w2_path = out_dir / Path(W2_V2).name
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

    with v2_w2_path.open("x", encoding="utf-8") as fh:
        fh.write(json.dumps(v2_w2, indent=2) + "\n")
    return {
        str(v2_w1_path): hashlib.sha256(v2_w1_path.read_bytes()).hexdigest(),
        str(v2_w2_path): hashlib.sha256(v2_w2_path.read_bytes()).hexdigest(),
    }


def _diagnostic_vs_int06(dossier: dict[str, Any], control: dict[str, Any]) -> dict[str, Any]:
    """Arithmetic-only signal-to-span of a legacy dossier against the INT-06 same-pipeline G2."""
    dp = [g.get("dp_u_pa") for g in dossier.get("grids", [])]
    ctl = control["grids"][-1].get("dp_u_pa")
    row: dict[str, Any] = {
        "status": "diagnostic_only_pipeline_identity_unbound_not_admission",
        "control_source": f"{INT06_CONTROL}#/same_pipeline_ct35_control",
        "control_finest_dp_u_pa": ctl,
        "candidate_finest_dp_u_pa": None,
        "candidate_3grid_dp_u_span_pa": None,
        "signal_to_3grid_span_ratio_dp_u": None,
    }
    if len(dp) == 3 and all(_is_number(x) for x in dp + [ctl]) and max(dp) > min(dp):
        row["candidate_finest_dp_u_pa"] = dp[-1]
        row["candidate_3grid_dp_u_span_pa"] = max(dp) - min(dp)
        row["signal_to_3grid_span_ratio_dp_u"] = abs(ctl - dp[-1]) / (max(dp) - min(dp))
    return row


def interpret_existing(prereg: dict[str, Any], prereg_path: Path) -> dict[str, Any]:
    """New-identity interpretation of committed elite dossiers under the corrected gate."""
    cfg = load_elite_cfd_gate_config(prereg)
    control = _read_json(ROOT / INT06_CONTROL)["same_pipeline_ct35_control"]
    sources: list[tuple[str, str]] = [(W1_V1, ""), (W1_V2, "")]
    sources += [(path, f"/{key}") for path in (W2_V1, W2_V2) for key in W2_SUBDOSSIERS]
    rows = []
    for rel, pointer in sources:
        path = ROOT / rel
        data = _read_json(path)
        dossier = data[pointer[1:]] if pointer else data
        recorded = dossier.get("mandatory_promotion_gate")
        result = evaluate_elite_gci_rsm_gate(dossier, prereg)
        rows.append({
            "source_path": rel,
            "json_pointer": pointer,
            "source_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "subject_candidate_id": dossier.get("subject_candidate_id"),
            "recorded_gate_passed": recorded.get("gate_passed") if isinstance(recorded, dict) else None,
            "recorded_basis": (
                "historical runner read the unregistered key mandatory_elite_gci_and_rsm_gate (code defaults), "
                "applied a non-preregistered 25% reduction rule and a stored ratio against the "
                "different-pipeline single-point CT-35 reference"
            ),
            "corrected_admission": result["admission"],
            "failures": result["failures"],
            "diagnostic_vs_int06_same_pipeline_control": _diagnostic_vs_int06(dossier, control),
        })
    return {
        "schema": "joil-elite-cfd-gate-interpretation/1",
        "kind": "interpretation_of_historical_records_not_physical_validation",
        "gate_tool": "tools/run_elite_3grid_gci_and_rsm.py",
        "gate_tool_sha256": hashlib.sha256(Path(__file__).resolve().read_bytes()).hexdigest(),
        "preregistration": {
            "path": str(prereg_path.resolve().relative_to(ROOT)) if prereg_path.resolve().is_relative_to(ROOT)
            else str(prereg_path),
            "sha256": hashlib.sha256(prereg_path.read_bytes()).hexdigest(),
            "gate_config": cfg["provenance"],
        },
        "runner_policy_constraints_not_preregistered": copy.deepcopy(RUNNER_POLICY_NOT_PREREGISTERED),
        "interpretation": INTERPRETATION,
        "historical_records_unchanged": "source files are read only; recorded gate values are reported, not rewritten",
        "dossiers": rows,
    }


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for key, value in pairs:
        if key in out:
            raise ValueError(f"duplicate JSON key: {key}")
        out[key] = value
    return out


def _reject_constant(value: str) -> Any:
    raise ValueError(f"non-finite JSON constant: {value}")


def _read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_unique_object,
                      parse_constant=_reject_constant)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Evaluate the preregistered 3-grid + dual LRR/SSG RSM elite CFD admission gate."
    )
    parser.add_argument(
        "--verify-existing",
        action="store_true",
        help="Evaluate every committed elite dossier and the Wave-2 rejected-cohort audit; missing inputs fail.",
    )
    parser.add_argument(
        "--interpret-existing",
        action="store_true",
        help="Print (or write with --out) a new-identity interpretation of the committed dossiers; exit 0 when produced.",
    )
    parser.add_argument(
        "--out",
        type=Path,
        help="Write the --interpret-existing report to this new file; an existing path is refused.",
    )
    parser.add_argument(
        "--generate-v2",
        action="store_true",
        help="Regenerate INT-05 / TR-07 v2 GCI reports into a fresh --out-dir (never into docs/results).",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        help="Fresh output directory for --generate-v2; an existing path is refused.",
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

    if not args.prereg.is_file():
        print(f"FAIL: missing preregistration: {args.prereg}", file=sys.stderr)
        return 2
    try:
        prereg = _read_json(args.prereg)
        load_elite_cfd_gate_config(prereg)
    except (ValueError, GateConfigError) as exc:
        print(f"FAIL: invalid preregistration {args.prereg}: {exc}", file=sys.stderr)
        return 2

    if args.out is not None and not args.interpret_existing:
        print("FAIL: --out requires --interpret-existing", file=sys.stderr)
        return 2
    if args.generate_v2:
        if args.out_dir is None:
            print("FAIL: --generate-v2 requires a fresh --out-dir", file=sys.stderr)
            return 2
        try:
            digests = generate_int05_v2_reports(prereg, args.out_dir)
        except FileExistsError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 2
        print(json.dumps({"generated_v2_reports": digests}, indent=2))
        return 0
    if args.interpret_existing:
        if args.out is not None and (args.out.exists() or args.out.is_symlink()):
            print(f"FAIL: refusing existing output: {args.out}", file=sys.stderr)
            return 2
        text = json.dumps(interpret_existing(prereg, args.prereg), indent=2) + "\n"
        if args.out is None:
            print(text, end="")
        else:
            with args.out.open("x", encoding="utf-8") as fh:
                fh.write(text)
            print(f"interpretation report: {args.out} sha256={hashlib.sha256(text.encode('utf-8')).hexdigest()}")
        return 0

    if not args.dossier.is_file():
        print(f"FAIL: missing GCI/RSM dossier: {args.dossier}", file=sys.stderr)
        return 1
    result = evaluate_elite_gci_rsm_gate(_read_json(args.dossier), prereg)

    if args.verify_existing:
        if not args.wave2_ablation.is_file():
            result["failures"].append(f"missing Wave-2 ablation dossier: {args.wave2_ablation}")
        else:
            ablation = _read_json(args.wave2_ablation)
            audit = ablation.get("rejected_cohort_10pct_audit", {})
            if audit.get("audit_passed") is not True:
                result["failures"].append("Wave-2 10% rejected-cohort audit did not pass")
            result["checks"]["wave2_rejected_cohort_audit_passed"] = audit.get("audit_passed") is True
            result["checks"]["wave2_dossier_sha256"] = hashlib.sha256(
                args.wave2_ablation.read_bytes()
            ).hexdigest()
        for rel in (W2_V1, W2_V2):
            w2_path = ROOT / rel
            if not w2_path.is_file():
                result["failures"].append(f"missing Wave-2 elite bundle: {rel}")
                continue
            w2_bundle = _read_json(w2_path)
            for key in W2_SUBDOSSIERS:
                sub_d = w2_bundle.get(key)
                if not isinstance(sub_d, dict):
                    result["failures"].append(f"missing {key} in {w2_path.name}")
                    continue
                sub_res = evaluate_elite_gci_rsm_gate(sub_d, prereg)
                result["checks"][f"{w2_path.stem}_{key}_admission"] = sub_res["admission"]
                result["failures"].extend(f"{w2_path.name} {key}: {f}" for f in sub_res["failures"])
        v2_w1_path = ROOT / W1_V2
        if not v2_w1_path.is_file():
            result["failures"].append(f"missing Wave-1 v2 dossier: {W1_V2}")
        else:
            v2_w1_res = evaluate_elite_gci_rsm_gate(_read_json(v2_w1_path), prereg)
            result["checks"]["v2_wave1_elite_admission"] = v2_w1_res["admission"]
            result["failures"].extend(f"{v2_w1_path.name}: {f}" for f in v2_w1_res["failures"])
        admitted = not result["failures"]
        result["admission"] = "admitted" if admitted else "not_admitted"
        result["gate_passed"] = admitted

    if result["admission"] != "admitted":
        print(json.dumps(result, indent=2))
        print(f"NOT ADMITTED: {len(result['failures'])} failures for {result['subject_candidate_id']}")
        return 1

    print(
        f"ADMITTED: same-pipeline resolved difference for {result['subject_candidate_id']} "
        f"(SNR_dp={result['checks']['saving_to_3grid_span_ratio_dp_u']:.2f}x, "
        f"SNR_eloss={result['checks']['saving_to_3grid_span_ratio_eloss']:.2f}x); "
        "not an improvement or physical qualification"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

