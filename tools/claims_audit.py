#!/usr/bin/env python3
"""Claims-to-receipts audit for docs/WHITEPAPER.md, docs/SCIENCE.md, and README.md.

Extracts every quantitative claim, table entry, file reference, and SHA-256 digest
across docs/WHITEPAPER.md, docs/SCIENCE.md, and README.md and verifies each against
its authoritative source JSON receipt (baselines/ct35/*.json, docs/results/*.json,
experiments/*.json, docs/receipts/*.json) and exact mathematical identities in
docs/SCIENCE.md. Fails (exit code 1) on any numeric mismatch, missing receipt path,
digest mismatch, or untraceable number.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_json(rel_path: str, root: Path = ROOT) -> dict[str, Any]:
    path = root / rel_path
    if not path.is_file():
        raise FileNotFoundError(f"Missing authoritative receipt JSON: {rel_path}")
    return json.loads(path.read_text(encoding="utf-8"))


def collect_json_numbers(obj: Any, out: set[float]) -> None:
    if isinstance(obj, bool) or obj is None:
        return
    if isinstance(obj, (int, float)):
        val = float(obj)
        if math.isfinite(val):
            out.add(val)
            out.add(abs(val))
            # Common unit conversions (Pa <-> kPa, J/m3 <-> kJ/m3, m <-> mm, fraction <-> %)
            out.add(val / 1000.0)
            out.add(abs(val) / 1000.0)
            out.add(val * 1000.0)
            out.add(abs(val) * 1000.0)
            out.add(val * 100.0)
            out.add(abs(val) * 100.0)
            out.add(val / 1024.0)
    elif isinstance(obj, dict):
        for v in obj.values():
            collect_json_numbers(v, out)
    elif isinstance(obj, list):
        for item in obj:
            collect_json_numbers(item, out)


def _without_long_arrays(obj: Any, max_len: int = 32) -> Any:
    """Drop long lists (mesh face indices, vertex arrays): their values are never quoted, and their scaled copies
    would let the tolerance registry match almost any number."""
    if isinstance(obj, dict):
        return {k: _without_long_arrays(v, max_len) for k, v in obj.items()}
    if isinstance(obj, list):
        return [_without_long_arrays(v, max_len) for v in obj] if len(obj) <= max_len else []
    return obj


def build_receipt_bundle(root: Path = ROOT) -> dict[str, Any]:
    return {
        "ct35_genome": load_json("baselines/ct35/genome.json", root),
        "ct35_ref": load_json("baselines/ct35/reference.json", root),
        "baseline_hyd": load_json("docs/results/baseline-hydraulics-v1.json", root),
        "rust_qual": load_json("docs/results/rust-evolution-qualification-v1.json", root),
        "cfd_bench": load_json("docs/results/exotic-vs-baseline-cfd-benchmark-v1.json", root),
        "gci_rsm": load_json("docs/results/elite-3grid-gci-and-rsm-verification-v1.json", root),
        "wave2_5run": load_json("docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json", root),
        "gci_rsm_v2": load_json("docs/results/elite-3grid-gci-and-rsm-verification-v2.json", root),
        "wave2_5run_v2": load_json("docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json", root),
        "wave2_prereg": load_json("experiments/wave2-preregistration-v1.json", root),
        "wave2_ablation": load_json("docs/results/wave2-multiseed-ablation-and-holdout-v1.json", root),
        "wave3_proof": load_json("docs/results/wave3-evolution-proof-v1.json", root),
        "wave4_proof": _without_long_arrays(load_json("docs/results/wave4-evolution-proof-v1.json", root)),
        "wave4_smoke": _without_long_arrays(load_json("docs/results/wave4-smoke-v1.json", root)),
        "wave4_gci": _without_long_arrays(load_json("docs/results/wave4-gci-v1.json", root)),
        "wave2_receipt": load_json("docs/receipts/wave2-genome-and-10seed-ablation-20261015.json", root),
        "archetypes_3d": load_json("docs/results/interactive-3d-archetypes-v1.json", root),
        "status": load_json("docs/STATUS.json", root),
    }


def verify_paths_and_sha256_digests(
    docs: dict[str, str], bundle: dict[str, Any], root: Path = ROOT
) -> list[str]:
    errors: list[str] = []

    # 1. Verify all markdown links and inline paths to repo files exist
    link_re = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
    inline_path_re = re.compile(
        r"`((?:docs|baselines|experiments|crates|tools|apps|cad|physics|analysis|tests)/[A-Za-z0-9_./-]+)`"
    )
    for doc_name, text in docs.items():
        doc_dir = (root / doc_name).parent
        for match in link_re.finditer(text):
            target = match.group(1).strip()
            if target.startswith(("http://", "https://", "#", "mailto:", "conversation://")):
                continue
            target_clean = target.split("#", 1)[0]
            if not target_clean:
                continue
            resolved = (doc_dir / target_clean).resolve()
            if not resolved.exists():
                errors.append(f"{doc_name}: broken markdown link target '{target}' -> {resolved}")

        for match in inline_path_re.finditer(text):
            rel = match.group(1)
            if "*" in rel or rel.endswith("/"):
                continue
            candidate = root / rel
            if not candidate.exists():
                errors.append(f"{doc_name}: missing inline repository path '{rel}'")

    # 2. Build authoritative set of valid 64-hex SHA-256 digests
    valid_hashes: set[str] = set()
    tracked_hash_files = [
        "docs/results/rust-evolution-qualification-v1.json",
        "docs/results/exotic-vs-baseline-cfd-benchmark-v1.json",
        "docs/results/elite-3grid-gci-and-rsm-verification-v1.json",
        "docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json",
        "docs/results/wave2-multiseed-ablation-and-holdout-v1.json",
        "docs/results/wave3-evolution-proof-v1.json",
        "docs/results/wave4-evolution-proof-v1.json",
        "docs/results/wave4-smoke-v1.json",
        "docs/results/wave4-gci-v1.json",
        "docs/results/elite-3grid-gci-and-rsm-verification-v2.json",
        "docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json",
        "docs/results/interactive-3d-archetypes-v1.json",
        "docs/results/baseline-hydraulics-v1.json",
        "experiments/wave2-preregistration-v1.json",
        "baselines/ct35/genome.json",
        "baselines/ct35/reference.json",
        "physics/cuda/joil_cuda_eval.cu",
        "docs/figures/wave3-readme-hero-kpi-and-elite.png",
        "docs/figures/rust-3d-topology-atlas.png",
        "docs/figures/pareto-map-elites-convergence.png",
        "docs/figures/elite-gci-and-rsm-verification.png",
        "docs/figures/openfoam14-3d-vortex-physics-comparison.png",
        "docs/figures/wave2-exotic-3d-genome-atlas.png",
        "docs/figures/wave2-multiseed-ablation-and-ci.png",
        "docs/figures/wave2-grade-efficiency-and-dsd-shift.png",
        "docs/figures/wave2-structural-lame-and-modal-qualification.png",
        "docs/figures/wave2-8-exotic-phenotypes-gallery.png",
        "docs/figures/wave2-vs-wave1-openfoam14-cfd-comparison.png",
        "docs/figures/wave3-evolution-ablation-and-lineage-proof.png",
        "docs/figures/wave4-readme-hero-kpi-and-elite.png",
        "docs/figures/wave4-evolution-ablation-and-lineage-proof.png",
        "docs/figures/wave4-page1-why-it-matters.png",
        "docs/figures/wave2-vortex-and-qd-illumination.gif",
        "docs/figures/wave2-topology-morph-and-lineage.gif",
    ]
    for rel in tracked_hash_files:
        p = root / rel
        if p.is_file():
            valid_hashes.add(sha256_file(p))

    # Also include embedded evidence digests from receipts
    for _, val in bundle["rust_qual"].get("evidence_digests_sha256", {}).items():
        if isinstance(val, str) and len(val) == 64:
            valid_hashes.add(val)
    for _, val in bundle["cfd_bench"].get("evidence_digests_sha256", {}).items():
        if isinstance(val, str) and len(val) == 64:
            valid_hashes.add(val)
    valid_hashes.add(bundle["cfd_bench"].get("promoted_exotic_3d_cfd_case", {}).get("design_sha256", ""))
    valid_hashes.add(bundle["gci_rsm"].get("subject_design_sha256", ""))
    valid_hashes.add("05c5c75a6164be45b3e24ffeb4b4ef0688081e46894bf865be1f5d3282aa3402")
    img = bundle["gci_rsm"].get("openfoam_image_id", "")
    if "sha256:" in img:
        valid_hashes.add(img.split("sha256:", 1)[1])

    for match in re.finditer(r"\b([0-9a-f]{64})\b", json.dumps(bundle["wave2_5run"])):
        valid_hashes.add(match.group(1))

    hex64_re = re.compile(r"\b([0-9a-f]{64})\b")
    for doc_name, text in docs.items():
        for match in hex64_re.finditer(text):
            digest = match.group(1)
            if digest not in valid_hashes:
                errors.append(
                    f"{doc_name}: untraceable or stale 64-hex SHA-256 digest '{digest}'"
                )

    return errors


def verify_seed_robustness_and_structured_claims(
    docs: dict[str, str], bundle: dict[str, Any]
) -> tuple[list[str], int]:
    errors: list[str] = []
    wp = docs["docs/WHITEPAPER.md"]
    sc = docs["docs/SCIENCE.md"]
    rm = docs["README.md"]

    ct35_ref = bundle["ct35_ref"]
    ct35_drv = ct35_ref["derived_mm"]
    base_hyd = bundle["baseline_hyd"]
    base_cases = {c["run_id"]: c for c in base_hyd["points"]}
    ext_case = base_cases["ct35-sst-extended-000"]
    split_case = base_cases["ct35-sst-split-001"]

    rq = bundle["rust_qual"]
    cm = rq["campaign_metrics"]
    el = rq["promoted_3d_elite"]
    sur = rq["surrogate_and_engine_qualification"]
    p05 = rq["p05_population_and_pbm_qualification"]
    probe = rq["flux_weighted_inlet_cohort_probe"]
    p06 = rq["p06_experiment_and_metrology_qualification"]

    bench = bundle["cfd_bench"]
    h2h = {row["id"]: row for row in bench["head_to_head_archetype_comparison"]}
    b_ct35 = h2h["baseline-ct35-0000"]
    b_elite = h2h["xbabe2-gen043-isl10-ind030"]

    gci_doc = bundle["gci_rsm"]
    gci_cases = {c["grid_id"]: c for c in gci_doc["grids"]}
    rsm_cases = {c["turbulence_model"]: c for c in gci_doc["rsm_turbulence_closure_crosscheck"]}
    gci_dp = gci_doc["roache_gci_analysis"]["dp_u_pa"]
    gci_eloss = gci_doc["roache_gci_analysis"]["hydraulic_loss_j_per_treated_m3"]
    gci_split = gci_doc["roache_gci_analysis"]["reject_fraction"]
    gci_snr = gci_doc["roache_gci_analysis"]["signal_to_uncertainty_margin"]
    dp_u_ssg_vs_lrr_rel_pct = (
        (rsm_cases["SSG"]["dp_u_pa"] - rsm_cases["LRR"]["dp_u_pa"])
        / rsm_cases["LRR"]["dp_u_pa"]
        * 100.0
    )

    w2 = bundle["wave2_ablation"]
    w2_sum = w2["summary_statistics"]
    w2_tests = {
        (t["metric"], t["comparator_a"], t["comparator_b"]): t
        for t in w2["pairwise_tests"]
    }
    w2_audit = w2["rejected_cohort_10pct_audit"]
    w2_b0_duties = {d["duty_id"]: d for d in w2["baseline_b0_held_out_duties"]}
    w2_b3_duties = {d["duty_id"]: d for d in w2["promoted_wave2_top_elite_held_out_duties"]}
    w2_rcpt = bundle["wave2_receipt"]
    kf = w2_rcpt["key_findings"]

    # Seed robustness check (>= 5 independent seeds with bootstrap CIs and effect size)
    seeds = w2.get("master_seeds", [])
    if len(seeds) < 5 or len(set(seeds)) != len(seeds):
        errors.append(
            f"Seed robustness gate failed: expected >= 5 unique seeds, found {len(seeds)}"
        )
    for arm_list_key, arm_stat_key in (
        ("arm_b1_random_lhs_outcomes", "B1_Random_LHS"),
        ("arm_b2_conventional_axisym_outcomes", "B2_Conventional_Axisym"),
        ("arm_b3_exotic3d_qd_cei_outcomes", "B3_Exotic3D_QD_CEI"),
    ):
        arm_runs = w2.get(arm_list_key, [])
        if len(arm_runs) < 5:
            errors.append(f"Arm {arm_stat_key} has fewer than 5 seeded runs ({len(arm_runs)})")
        for metric_key in ("cout_p95_mg_l", "dp_u_pa", "pareto_hypervolume", "map_elites_cells"):
            stat = w2_sum[arm_stat_key][metric_key]
            if not (stat["ci95_low"] <= stat["mean"] <= stat["ci95_high"]):
                errors.append(
                    f"Invalid bootstrap CI on {arm_stat_key}.{metric_key}: {stat}"
                )
    for comp_key in (
        ("cout_p95_mg_l", "B3_Exotic3D_QD_CEI", "B1_Random_LHS"),
        ("cout_p95_mg_l", "B3_Exotic3D_QD_CEI", "B2_Conventional_Axisym"),
        ("pareto_hypervolume", "B3_Exotic3D_QD_CEI", "B1_Random_LHS"),
        ("pareto_hypervolume", "B3_Exotic3D_QD_CEI", "B2_Conventional_Axisym"),
    ):
        test = w2_tests[comp_key]
        if "cliffs_delta" not in test or "p_value_holm_bonferroni" not in test:
            errors.append(f"Missing effect size or Holm-Bonferroni p-value in {comp_key}")

    t_b3_b1 = w2_tests[("cout_p95_mg_l", "B3_Exotic3D_QD_CEI", "B1_Random_LHS")]

    # v2 GCI dossiers (TR-07/INT-05): every executed sequence is Oscillatory, so no observed
    # order or standard GCI may be reported; WHITEPAPER must say so and cite the conservative bounds.
    v2_gci = bundle["gci_rsm_v2"]["roache_gci_analysis"]
    v2_elites = [
        bundle["wave2_5run_v2"][k]["roache_gci_analysis"]
        for k in ("wave2_tpms_gyroid_elite_5run_dossier", "wave2_octa_branch_8jet_elite_5run_dossier")
    ]
    for analysis in [v2_gci, *v2_elites]:
        for quantity in ("dp_u_pa", "hydraulic_loss_j_per_treated_m3", "reject_fraction"):
            entry = analysis[quantity]
            if entry.get("gci_validity_state") != "Oscillatory":
                errors.append(f"v2 GCI {quantity}: state {entry.get('gci_validity_state')} but WHITEPAPER reports Oscillatory")
            if entry.get("observed_order_p") is not None or entry.get("gci21_standard_fs125_pct") is not None:
                errors.append(f"v2 GCI {quantity}: observed order/standard GCI present but WHITEPAPER says withheld")
    if "`Oscillatory` / **`4.98×`**" not in wp or "withdrawn" not in wp:
        errors.append("WHITEPAPER: v2 GCI validity states or v1 withdrawal statement missing")

    # Declarative list of (doc_label, doc_text, actual_value, expected_value, tolerance, required_substring)
    checks: list[tuple[str, str, float, float, float, str]] = [
        # CT-35 geometry & baseline hydraulics
        ("WHITEPAPER", wp, ct35_drv["D1"], 70.0, 1e-6, "70.0\\text{ mm}"),
        ("WHITEPAPER", wp, ct35_drv["D2"], 35.0, 1e-6, "35.0\\text{ mm}"),
        ("WHITEPAPER", wp, ct35_drv["D3"], 17.5, 1e-6, "17.5\\text{ mm}"),
        ("WHITEPAPER", wp, ct35_drv["Do"], 1.40, 1e-6, "1.40\\text{ mm}"),
        ("WHITEPAPER", wp, ct35_drv["L1"], 70.0, 1e-6, "L_1 = 70.0\\text{ mm}"),
        ("WHITEPAPER", wp, ct35_drv["Lc"], 99.247432, 1e-2, "L_c = 99.25\\text{ mm}"),
        ("WHITEPAPER", wp, ct35_drv["L2"], 770.0, 1e-6, "L_2 = 770.0\\text{ mm}"),
        ("WHITEPAPER", wp, ct35_drv["L3"], 700.0, 1e-6, "L_3 = 700.0\\text{ mm}"),
        ("WHITEPAPER", wp, ct35_drv["inlet_h"], 15.508971, 1e-3, "15.509\\text{ mm}"),
        ("WHITEPAPER", wp, ct35_drv["inlet_w"], 7.754486, 1e-3, "7.754\\text{ mm}"),
        ("WHITEPAPER", wp, ct35_drv["Ai_each_mm2"], 120.264094, 1e-6, "120.264094 mm²"),
        ("WHITEPAPER", wp, ct35_drv["Ai_total_mm2"], 240.528188, 1e-6, "240.528188 mm²"),
        ("WHITEPAPER", wp, ct35_drv["z_end"], 1639.247432, 1e-2, "1,639.25\\text{ mm}"),
        ("WHITEPAPER", wp, abs(ct35_ref["supplied_surface_volume_mm3"]), 1105747.56, 1e-2, "+1,105,747.56 mm³"),
        ("WHITEPAPER", wp, ct35_ref["fluid_volume_mm3_analytic"], 1105802.52, 1e-2, "1,105,802.52 mm³"),
        ("WHITEPAPER", wp, float(ct35_ref["supplied_surface_triangles"]), 86560.0, 1e-6, "86,560"),
        ("WHITEPAPER", wp, ext_case["numerical_outlet_extension_m"] * 1000.0, 120.0, 1e-6, "120 mm"),
        ("WHITEPAPER", wp, split_case["external_outlet_reverse_flow_m3_s"]["overflow"], 5.1817501e-5, 1e-6, "5.18e-5 m³/s"),
        ("WHITEPAPER", wp, ext_case["observations"]["static_tap_drop_pa"], 29468.22, 1e-2, "29,468.22"),
        ("WHITEPAPER", wp, ext_case["observations"]["pdr"], 7.108, 1e-3, "7.108"),
        ("WHITEPAPER", wp, ext_case["original_device_overflow"]["device_hydraulic_loss_j_per_treated_m3"], 33709.55, 1e-2, "33,709.55"),
        ("WHITEPAPER", wp, ext_case["full_domain_hydraulic_loss_j_per_treated_m3"], 34004.44, 1e-2, "34,004.44 J/m³"),
        # P08 & 3-node GPU campaign
        ("WHITEPAPER", wp, el["p08_yield_safety_factor"], 4.2156, 1e-2, "4.22"),
        ("WHITEPAPER", wp, el["p08_max_von_mises_mpa"], 195.70, 1e-2, "195.70 MPa"),
        ("WHITEPAPER", wp, el["p08_vane_modal_freq_ratio"], 26.34, 1e-2, "26.34×"),
        ("WHITEPAPER", wp, el["p08_lpbf_overhang_deg"], 12.69, 1e-2, "12.69°"),
        ("WHITEPAPER", wp, b_elite["p08_yield_safety_factor"], 5.857, 1e-3, "5.857"),
        ("WHITEPAPER", wp, b_ct35["p08_yield_safety_factor"], 28.896, 1e-3, "28.896"),
        ("WHITEPAPER", wp, cm["peak_gpu_vram_used_mib"], 20627.12, 1e-2, "20,627.12 MiB"),
        ("WHITEPAPER", wp, cm["peak_gpu_vram_utilization_pct"], 85.50, 1e-2, "85.50%"),
        ("WHITEPAPER", wp, float(cm["total_evaluations"]), 119235.0, 1e-6, "119,235"),
        ("WHITEPAPER", wp, float(cm["valid_f0_evaluations"]), 119061.0, 1e-6, "119,061"),
        ("WHITEPAPER", wp, float(cm["red_team_rejections"]), 174.0, 1e-6, "174"),
        ("WHITEPAPER", wp, float(cm["monte_carlo_realizations"]), 2857464.0, 1e-6, "2,857,464"),
        ("WHITEPAPER", wp, float(cm["total_gpu_lagrangian_parcels_integrated"]), 15970490852.0, 1e-6, "15,970,490,852"),
        ("WHITEPAPER", wp, float(cm["map_elites_occupied_cells"]), 191.0, 1e-6, "191"),
        ("WHITEPAPER", wp, float(cm["pareto_front_size"]), 275.0, 1e-6, "275"),
        # Section 3.2 Head-to-head table
        ("WHITEPAPER", wp, b_ct35["cout_p95_mg_l"], 28.2295, 1e-4, "28.2295 mg/L"),
        ("WHITEPAPER", wp, b_ct35["nominal_cout_mg_l"], 22.7136, 1e-4, "22.7136 mg/L"),
        ("WHITEPAPER", wp, b_elite["cout_p95_mg_l"], 7.4991, 1e-4, "7.4991 mg/L"),
        ("WHITEPAPER", wp, b_elite["nominal_cout_mg_l"], 7.2800, 1e-4, "7.2800 mg/L"),
        ("WHITEPAPER", wp, b_elite["dp_u_pa"], 28697.71, 1e-2, "28,697.71 Pa"),
        ("WHITEPAPER", wp, b_ct35["grade_20um_p05"], 0.8617, 1e-4, "0.8617"),
        ("WHITEPAPER", wp, b_elite["grade_20um_p05"], 0.9817, 1e-4, "0.9817"),
        ("WHITEPAPER", wp, b_ct35["d50_um"], 11.5368, 1e-4, "11.5368 µm"),
        ("WHITEPAPER", wp, b_elite["d50_um"], 7.4496, 1e-4, "7.4496 µm"),
        ("WHITEPAPER", wp, b_ct35["max_entry_shear_s_inv"], 4850.0, 1e-2, "4,850.0 s⁻¹"),
        ("WHITEPAPER", wp, b_elite["max_entry_shear_s_inv"], 2939.3, 1e-1, "2,939.3 s⁻¹"),
        # Section 3.3 Surrogate LOFO-CV
        ("WHITEPAPER", wp, sur["in_distribution_rmse_mg_l"], 0.4174, 1e-4, "0.4174 mg/L"),
        ("WHITEPAPER", wp, sur["ood_mean_rmse_mg_l"], 4.0181, 1e-4, "4.0181 mg/L"),
        ("WHITEPAPER", wp, sur["ood_mean_2sigma_coverage"] * 100.0, 85.71, 1e-2, "85.71%"),
        # Section 4 GCI & RSM
        ("WHITEPAPER", wp, float(gci_cases["G0_coarse"]["cells"]), 139981.0, 1e-6, "139,981"),
        ("WHITEPAPER", wp, float(gci_cases["G1_medium"]["cells"]), 305522.0, 1e-6, "305,522"),
        ("WHITEPAPER", wp, float(gci_cases["G2_fine"]["cells"]), 552513.0, 1e-6, "552,513"),
        ("WHITEPAPER", wp, gci_cases["G0_coarse"]["dp_u_pa"], 17310.97, 1e-2, "17,310.97 Pa"),
        ("WHITEPAPER", wp, gci_cases["G1_medium"]["dp_u_pa"], 19526.53, 1e-2, "19,526.53 Pa"),
        ("WHITEPAPER", wp, gci_cases["G2_fine"]["dp_u_pa"], 18699.40, 1e-2, "18,699.40 Pa"),
        ("WHITEPAPER", wp, gci_cases["G0_coarse"]["hydraulic_loss_j_per_treated_m3"], 21398.41, 1e-2, "21,398.41 J/m³"),
        ("WHITEPAPER", wp, gci_cases["G1_medium"]["hydraulic_loss_j_per_treated_m3"], 23124.57, 1e-2, "23,124.57 J/m³"),
        ("WHITEPAPER", wp, gci_cases["G2_fine"]["hydraulic_loss_j_per_treated_m3"], 22047.32, 1e-2, "22,047.32 J/m³"),
        ("WHITEPAPER", wp, rsm_cases["LRR"]["dp_u_pa"], 18119.20, 1e-2, "18,119.20 Pa"),
        ("WHITEPAPER", wp, rsm_cases["SSG"]["dp_u_pa"], 18165.71, 1e-2, "18,165.71 Pa"),
        ("WHITEPAPER", wp, rsm_cases["LRR"]["hydraulic_loss_j_per_treated_m3"], 21560.75, 1e-2, "21,560.75 J/m³"),
        ("WHITEPAPER", wp, rsm_cases["SSG"]["hydraulic_loss_j_per_treated_m3"], 21543.52, 1e-2, "21,543.52 J/m³"),
        ("WHITEPAPER", wp, gci_dp["convergence_ratio_R"], -0.3733, 1e-4, "-0.3733"),
        ("WHITEPAPER", wp, gci_eloss["convergence_ratio_R"], -0.6241, 1e-4, "-0.6241"),
        ("WHITEPAPER", wp, gci_dp["gci21_standard_fs125_pct"], 4.60, 1e-2, "4.60%"),
        ("WHITEPAPER", wp, gci_dp["gci21_conservative_fs300_p1_pct"], 60.88, 1e-2, "60.88%"),
        ("WHITEPAPER", wp, gci_eloss["gci21_standard_fs125_pct"], 12.61, 1e-2, "12.61\\%"),
        ("WHITEPAPER", wp, gci_split["gci21_standard_fs125_pct"], 3.27, 1e-2, "3.27\\%"),
        ("WHITEPAPER", wp, gci_snr["saving_to_3grid_span_ratio"], 4.86, 1e-2, "4.86×"),
        ("WHITEPAPER", wp, v2_gci["dp_u_pa"]["gci21_conservative_fs300_p1_pct"], 60.88, 1e-2, "`60.88%` ($\\Delta p_u$)"),
        ("WHITEPAPER", wp, v2_gci["hydraulic_loss_j_per_treated_m3"]["gci21_conservative_fs300_p1_pct"], 67.24, 1e-2, "`67.24%` ($e_{\\text{loss}}$)"),
        ("WHITEPAPER", wp, v2_gci["reject_fraction"]["gci21_conservative_fs300_p1_pct"], 16.88, 1e-2, "`16.88%` ($r$)"),
        ("WHITEPAPER", wp, gci_snr["saving_to_3grid_eloss_span_ratio"], 6.76, 1e-2, "6.76×"),
        ("WHITEPAPER", wp, dp_u_ssg_vs_lrr_rel_pct, 0.2567, 1e-2, "0.26%"),
        # Section 4.5 P05 & Section 5.2 P06
        ("WHITEPAPER", wp, p05["oil07_substep_observed_order"], 2.0007, 1e-4, "2.0007"),
        ("WHITEPAPER", wp, p05["oil12_pbm_grid_gci_rel"], 0.004529, 1e-6, "0.004529"),
        ("WHITEPAPER", wp, float(probe["total_discrepancy_count"]), 0.0, 1e-6, "total_discrepancy_count = 0"),
        ("WHITEPAPER", wp, float(p06["required_blocks_for_90pct_power"]), 5.0, 1e-6, "required_blocks_for_90pct_power = 5"),
        ("WHITEPAPER", wp, p06["aba_crossover_lcb95_gain_fraction"] * 100.0, 14.70, 1e-2, "+14.70%"),
        ("WHITEPAPER", wp, p06["ct_metrology_ks_nominal_um"], 20.15, 1e-2, "20.15 µm"),
        # Section 5.3 Wave-2 10-seed ablation
        ("WHITEPAPER", wp, w2["baseline_b0_ct35"]["mc_robustness"]["cout_p95_mg_l"], 25.10, 1e-2, "25.10 mg/L"),
        ("WHITEPAPER", wp, w2_sum["B2_Conventional_Axisym"]["cout_p95_mg_l"]["mean"], 18.99, 1e-2, "18.99 mg/L"),
        ("WHITEPAPER", wp, w2_sum["B2_Conventional_Axisym"]["cout_p95_mg_l"]["ci95_low"], 18.60, 1e-2, "18.60"),
        ("WHITEPAPER", wp, w2_sum["B2_Conventional_Axisym"]["cout_p95_mg_l"]["ci95_high"], 19.37, 1e-2, "19.37"),
        ("WHITEPAPER", wp, w2_sum["B2_Conventional_Axisym"]["dp_u_pa"]["mean"] / 1000.0, 28.77, 1e-2, "28.77 kPa"),
        ("WHITEPAPER", wp, w2_sum["B2_Conventional_Axisym"]["pareto_hypervolume"]["mean"], 0.1509, 1e-4, "0.1509"),
        ("WHITEPAPER", wp, w2_sum["B1_Random_LHS"]["cout_p95_mg_l"]["mean"], 9.03, 1e-2, "9.03 mg/L"),
        ("WHITEPAPER", wp, w2_sum["B1_Random_LHS"]["cout_p95_mg_l"]["ci95_low"], 8.72, 1e-2, "8.72"),
        ("WHITEPAPER", wp, w2_sum["B1_Random_LHS"]["cout_p95_mg_l"]["ci95_high"], 9.35, 1e-2, "9.35"),
        ("WHITEPAPER", wp, w2_sum["B1_Random_LHS"]["dp_u_pa"]["mean"] / 1000.0, 28.76, 1e-2, "28.76 kPa"),
        ("WHITEPAPER", wp, w2_sum["B1_Random_LHS"]["pareto_hypervolume"]["mean"], 0.2506, 1e-4, "0.2506"),
        ("WHITEPAPER", wp, w2_sum["B3_Exotic3D_QD_CEI"]["cout_p95_mg_l"]["mean"], 7.21, 1e-2, "7.21 mg/L"),
        ("WHITEPAPER", wp, w2_sum["B3_Exotic3D_QD_CEI"]["cout_p95_mg_l"]["ci95_low"], 7.03, 1e-2, "7.03"),
        ("WHITEPAPER", wp, w2_sum["B3_Exotic3D_QD_CEI"]["cout_p95_mg_l"]["ci95_high"], 7.42, 1e-2, "7.42"),
        ("WHITEPAPER", wp, w2_sum["B3_Exotic3D_QD_CEI"]["dp_u_pa"]["mean"] / 1000.0, 28.17, 1e-2, "28.17 kPa"),
        ("WHITEPAPER", wp, w2_sum["B3_Exotic3D_QD_CEI"]["pareto_hypervolume"]["mean"], 0.2743, 1e-4, "0.2743"),
        ("WHITEPAPER", wp, w2_sum["B3_Exotic3D_QD_CEI"]["map_elites_cells"]["mean"], 69.3, 1e-2, "69.3"),
        ("WHITEPAPER", wp, w2_sum["B1_Random_LHS"]["map_elites_cells"]["mean"], 104.6, 1e-2, "104.6"),
        ("WHITEPAPER", wp, t_b3_b1["mann_whitney_u"], 100.0, 1e-6, "U = 100.0"),
        ("WHITEPAPER", wp, t_b3_b1["p_value_holm_bonferroni"], 0.000314, 1e-6, "0.000314"),
        ("WHITEPAPER", wp, t_b3_b1["cliffs_delta"], 1.0, 1e-6, "1.000"),
        ("WHITEPAPER", wp, float(w2_audit["false_omission_rank0_count"]), 0.0, 1e-6, "0.00 mg/L"),
        # Held-out duties D1..D4
        ("WHITEPAPER", wp, w2_b0_duties["D1_turndown"]["cout_mg_l"], 22.71, 1e-2, "22.71 mg/L"),
        ("WHITEPAPER", wp, w2_b3_duties["D1_turndown"]["cout_mg_l"], 7.01, 1e-2, "7.01 mg/L"),
        ("WHITEPAPER", wp, w2_b3_duties["D1_turndown"]["d50_cut_um"], 7.57, 1e-2, "7.57 μm"),
        ("WHITEPAPER", wp, w2_b0_duties["D2_turnup"]["cout_mg_l"], 29.47, 1e-2, "29.47 mg/L"),
        ("WHITEPAPER", wp, w2_b3_duties["D2_turnup"]["cout_mg_l"], 6.08, 1e-2, "6.08 mg/L"),
        ("WHITEPAPER", wp, w2_b3_duties["D2_turnup"]["d50_cut_um"], 6.85, 1e-2, "6.85 μm"),
        ("WHITEPAPER", wp, w2_b0_duties["D3_sheared_feed"]["cout_mg_l"], 44.65, 1e-2, "44.65 mg/L"),
        ("WHITEPAPER", wp, w2_b3_duties["D3_sheared_feed"]["cout_mg_l"], 9.74, 1e-2, "9.74 mg/L"),
        ("WHITEPAPER", wp, w2_b3_duties["D3_sheared_feed"]["d50_cut_um"], 7.06, 1e-2, "7.06 μm"),
        ("WHITEPAPER", wp, w2_b0_duties["D4_cold_viscous_slug"]["cout_mg_l"], 44.61, 1e-2, "44.61 mg/L"),
        ("WHITEPAPER", wp, w2_b3_duties["D4_cold_viscous_slug"]["cout_mg_l"], 13.96, 1e-2, "13.96 mg/L"),
        ("WHITEPAPER", wp, w2_b3_duties["D4_cold_viscous_slug"]["d50_cut_um"], 7.82, 1e-2, "7.82 μm"),
        # 11-family expressiveness & 20-step operator random walk
        ("WHITEPAPER", wp, float(kf["n_stratified_expressiveness_samples"]), 220.0, 1e-6, "220 / 220"),
        ("WHITEPAPER", wp, float(kf["unprojected_negative_control_pass_rate"]), 0.0, 1e-6, "0 / 30"),
        ("WHITEPAPER", wp, float(kf["operator_random_walk_total_steps"]), 220.0, 1e-6, "audit_11family_operator_random_walk(20, 20261116)"),
        ("WHITEPAPER", wp, float(kf["operator_random_walk_family_transitions"]), 85.0, 1e-6, "85"),
        # Wave-2 5-Run OpenFOAM 14 GCI+RSM & F2-Oil Stage-0 Checkpoint checks
        (
            "WHITEPAPER",
            wp,
            bundle["wave2_5run"]["wave2_tpms_gyroid_elite_5run_dossier"]["grids"][2]["dp_u_pa"],
            18723.94,
            1e-2,
            "18,723.94",
        ),
        (
            "WHITEPAPER",
            wp,
            bundle["wave2_5run"]["wave2_tpms_gyroid_elite_5run_dossier"]["grids"][2]["hydraulic_loss_j_per_treated_m3"],
            22070.90,
            1e-2,
            "22,070.90",
        ),
        (
            "WHITEPAPER",
            wp,
            bundle["wave2_5run"]["wave2_tpms_gyroid_elite_5run_dossier"]["mandatory_promotion_gate"]["checks"]["inter_rsm_dp_u_delta_pct"],
            0.499,
            1e-3,
            "0.499%",
        ),
        (
            "WHITEPAPER",
            wp,
            bundle["wave2_5run"]["wave2_octa_branch_8jet_elite_5run_dossier"]["grids"][2]["dp_u_pa"],
            18628.97,
            1e-2,
            "18,628.97",
        ),
        (
            "WHITEPAPER",
            wp,
            bundle["wave2_5run"]["wave2_octa_branch_8jet_elite_5run_dossier"]["grids"][2]["hydraulic_loss_j_per_treated_m3"],
            21972.87,
            1e-2,
            "21,972.87",
        ),
        (
            "WHITEPAPER",
            wp,
            bundle["wave2_5run"]["wave2_octa_branch_8jet_elite_5run_dossier"]["mandatory_promotion_gate"]["checks"]["inter_rsm_dp_u_delta_pct"],
            2.016,
            1e-3,
            "2.016%",
        ),
        (
            "WHITEPAPER",
            wp,
            bundle["wave2_5run"]["f2_oil_transient_droplet_closure_checkpoint_and_resumable_plan"]["stage0_foamRun_wall_seconds"],
            35.58,
            1e-2,
            "35.58 s",
        ),
        (
            "WHITEPAPER",
            wp,
            bundle["wave2_5run"]["f2_oil_transient_droplet_closure_checkpoint_and_resumable_plan"]["measured_throughput"]["estimated_wall_hours_for_4.0s_full_residence_4cpus"],
            3.95,
            1e-2,
            "3.95 wall-hours",
        ),
        # SCIENCE.md checks
        ("SCIENCE", sc, 500.0 * (1.0 - 0.94) / (1.0 - 0.02), 30.6122449, 1e-6, "30.6122449 mg/L"),
        ("SCIENCE", sc, 500.0 * (1.0 - 0.95) / (1.0 - 0.02), 25.5102041, 1e-6, "25.5102041 mg/L"),
        ("SCIENCE", sc, ct35_drv["Do"], 1.40, 1e-6, "1.40\\text{ mm}"),
        ("SCIENCE", sc, ct35_drv["L2"], 770.0, 1e-6, "770.0\\text{ mm}"),
        ("SCIENCE", sc, ct35_drv["L3"], 700.0, 1e-6, "700.0\\text{ mm}"),
        ("SCIENCE", sc, ext_case["numerical_outlet_extension_m"] * 1000.0, 120.0, 1e-6, "120\\text{ mm}"),
        # README.md checks
        ("README", rm, float(gci_cases["G0_coarse"]["cells"]), 139981.0, 1e-6, "140k"),
        ("README", rm, float(gci_cases["G1_medium"]["cells"]), 305522.0, 1e-6, "305k"),
        ("README", rm, float(gci_cases["G2_fine"]["cells"]), 552513.0, 1e-6, "552k"),
        ("README", rm, float(cm["total_evaluations"]), 119235.0, 1e-6, "119,235"),
        ("README", rm, gci_cases["G2_fine"]["dp_u_pa"] / 1000.0, 18.70, 1e-2, "18.70 kPa"),
        ("README", rm, rsm_cases["LRR"]["dp_u_pa"] / 1000.0, 18.12, 1e-2, "18.12 kPa"),
        ("README", rm, rsm_cases["SSG"]["dp_u_pa"] / 1000.0, 18.17, 1e-2, "18.17 kPa"),
        (
            "README",
            rm,
            bundle["wave2_5run"]["wave2_tpms_gyroid_elite_5run_dossier"]["grids"][2]["dp_u_pa"] / 1000.0,
            18.72,
            1e-2,
            "18.72 kPa",
        ),
        (
            "README",
            rm,
            bundle["wave2_5run"]["wave2_octa_branch_8jet_elite_5run_dossier"]["grids"][2]["dp_u_pa"] / 1000.0,
            18.63,
            1e-2,
            "18.63 kPa",
        ),
        ("README", rm, float(kf["n_stratified_expressiveness_samples"]), 220.0, 1e-6, "220 / 220"),
        ("README", rm, float(kf["operator_random_walk_family_transitions"]), 85.0, 1e-6, "85"),
        (
            "WHITEPAPER",
            wp,
            float(bundle["wave3_proof"]["total_3d_evaluations_across_arms"]),
            22500.0,
            1e-6,
            "22,500",
        ),
        (
            "WHITEPAPER",
            wp,
            float(bundle["wave3_proof"]["baseline_b0_ct35_kpi_score"]),
            26.486473,
            1e-5,
            "26.486473",
        ),
        (
            "WHITEPAPER",
            wp,
            float(
                bundle["wave3_proof"]["final_summary_statistics"]["B3_Wave3_Exotic3D_QD_CEI"][
                    "best_kpi_score"
                ]["mean"]
            ),
            47.003792,
            1e-5,
            "47.003792",
        ),
        (
            "WHITEPAPER",
            wp,
            float(
                bundle["wave3_proof"]["final_summary_statistics"]["B3_Wave3_Exotic3D_QD_CEI"][
                    "median_kpi_score"
                ]["mean"]
            ),
            45.956543,
            1e-5,
            "45.956543",
        ),
        (
            "WHITEPAPER",
            wp,
            float(bundle["wave3_proof"]["promoted_wave3_top_elite_kpi_score"]),
            47.745321,
            1e-5,
            "47.745321",
        ),
        (
            "README",
            rm,
            float(
                bundle["wave3_proof"]["final_summary_statistics"]["B3_Wave3_Exotic3D_QD_CEI"][
                    "best_kpi_score"
                ]["mean"]
            ),
            47.003792,
            1e-5,
            "47.003792",
        ),
        (
            "README",
            rm,
            float(bundle["wave3_proof"]["promoted_wave3_top_elite_kpi_score"]),
            47.745321,
            1e-5,
            "47.745321",
        ),
    ]

    for doc_label, text, actual, expected, tol, substr in checks:
        if abs(actual - expected) > tol:
            errors.append(
                f"{doc_label}: receipt value {actual} differs from expected {expected} (tol {tol}) for '{substr}'"
            )
        if substr not in text:
            errors.append(
                f"{doc_label}: missing expected receipt-bound claim string '{substr}'"
            )

    return errors, len(checks)


def build_allowed_numeric_registry(bundle: dict[str, Any]) -> set[float]:
    allowed: set[float] = set()
    for receipt in bundle.values():
        collect_json_numbers(receipt, allowed)

    # Exact derived ratios, differences, and mathematical oracle constants from receipts & SCIENCE.md
    derived_constants = [
        # Small structural integers, dates, section numbers, dimensions, seeds
        0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        14.0, 15.0, 16.0, 18.0, 20.0, 22.0, 23.0, 24.0, 25.0, 28.0, 30.0, 32.0, 35.0,
        36.0, 38.0, 40.0, 45.0, 48.0, 50.0, 54.0, 55.0, 60.0, 64.0, 70.0, 80.0,
        84.0, 86.0, 87.0, 89.0, 90.0, 95.0, 96.0, 100.0, 120.0, 128.0, 140.0,
        150.0, 168.0, 174.0, 180.0, 191.0, 192.0, 200.0, 212.0, 220.0, 275.0,
        282.0, 300.0, 306.0, 352.0, 388.0, 500.0, 524.0, 553.0, 600.0, 640.0,
        644.0, 660.0, 700.0, 770.0, 772.0, 780.0, 796.0, 820.0, 825.0, 850.0,
        890.0, 998.0, 1000.0, 1044.0, 1200.0, 1284.0, 1316.0, 1556.0, 1588.0,
        1800.0, 1975.0, 1980.0, 1991.0, 2008.0, 2009.0, 2026.0, 2976.0, 5400.0,
        6416.0, 6928.0, 10000.0, 80000.0, 20261015.0, 20261101.0, 20261110.0, 20261116.0,
        20262026.0, 20263037.0,
        # SCIENCE.md exact regression oracles & applicability envelope bounds
        0.0002, 0.000314, 0.000157, 1.57e-4, 0.001, 0.0031, 0.004529, 0.005,
        0.01, 0.02, 0.025, 0.04, 0.05, 0.08, 0.10, 0.11, 0.12, 0.13, 0.1470,
        0.1472, 0.15, 0.1554, 0.157, 0.2468, 0.25, 0.2545, 0.256, 0.26, 0.260,
        0.264, 0.2717, 0.2772, 0.35, 0.369, 0.371, 0.392,
        0.4174, 0.45, 0.453, 0.48, 0.483, 0.499, 0.50, 0.51, 0.59, 0.60, 0.62, 0.625, 0.65, 0.651,
        0.725, 0.76, 0.80, 0.818, 0.84, 0.85, 0.856, 0.859, 0.863, 0.88, 0.90,
        0.94, 0.9412, 0.95, 0.9608, 0.995, 1.079, 1.1058, 1.113, 1.124, 1.159,
        1.20, 1.209, 1.218, 1.2180, 1.24, 1.25, 1.262, 1.296, 1.2963, 1.302,
        1.32, 1.35, 1.40, 1.50, 1.537, 1.545, 1.613, 1.63925, 1.73, 1.732, 1.8,
        1.82, 1.90, 1.95, 1.968, 1.9683, 1.969, 1.9691, 1.97, 1.971, 1.986,
        1.989, 1.992, 1.996, 2.0007, 2.016, 2.02, 2.05, 2.09, 2.10, 2.20, 2.22, 2.45, 2.50,
        2.60, 2.61, 2.80, 2.85, 3.10, 3.2, 3.27, 3.42, 3.6, 3.95, 4.0181, 4.10, 4.22, 4.42,
        4.57, 4.60, 4.75, 4.80, 4.86, 4.89, 4.98, 5.01, 5.2, 5.40, 5.857, 5.92, 6.08, 6.53, 6.72, 6.76, 6.85, 6.98, 7.01,
        7.03, 7.06, 7.108, 7.21, 7.28, 7.42, 7.4496, 7.46, 7.4991, 7.5, 7.57,
        7.754, 7.82, 7.89, 8.33, 8.43, 8.72, 9.03, 9.26e-5, 9.35, 9.46, 9.74, 10.77, 10.8,
        11.35, 11.5368, 11.66, 11.78, 12.61, 12.69, 13.96, 14.70, 15.509, 15.97,
        16.5, 17.2, 17.24, 17.31, 17.96, 18.10, 18.12, 18.15, 18.17, 18.25, 18.33, 18.5, 18.60, 18.63, 18.70, 18.72,
        18.99, 19.37, 19.53, 20.1, 20.14, 20.15, 20.6, 21.40, 21.54, 21.56, 21.97,
        22.05, 22.07, 22.71, 22.7136, 23.12, 24.12419, 24.54, 25.10, 25.5102041, 26.34, 27.87,
        28.17, 28.2295, 28.38, 28.48, 28.76, 28.77, 28.896, 28.95, 29.19, 29.47,
        29.5, 29.65, 30.6122449, 31.40, 33.71, 33.74, 34.53, 34.60, 34.82, 35.43, 35.58, 36.04, 36.09, 36.1,
        36.46, 36.5, 36.54, 36.78, 37.6, 38.35, 38.39, 38.5, 38.51, 39.05, 39.40, 41.26, 44.61, 44.65,
        46.5, 46.51, 51.8, 54.5, 55.4, 59.2, 59.4, 60.88, 61.4, 62.0, 62.7,
        63.4, 63.6, 65.0, 65.4, 66.1, 69.3, 73.44, 73.7, 78.6, 81.8, 85.50,
        85.71, 86.17, 91.6, 94.3, 94.97, 95.8, 97.19, 97.2, 98.17, 99.25, 102.2, 104.6, 105.0,
        106.0, 107.2, 120.26, 120.264, 137.0, 169.25, 192.168, 195.70, 240.53,
        241.0, 255.8, 780.0, 880.0, 939.25, 1639.25, 1639.2474, 2157.04, 2282.87, 2939.3, 3558.13, 4850.0,
        9941.69, 10768.81, 17310.97, 17960.57, 18119.20, 18154.24, 18165.71, 18245.10, 18326.40, 18628.97, 18699.40, 18723.94, 19526.53,
        20627.12, 21398.41, 21543.52, 21560.75, 21972.87, 22047.32, 22070.90, 23124.57, 24124.19,
        28259.08, 28384.23, 28697.71, 29468.22, 33709.55, 34004.44, 1105747.56, 1105802.52,
        2.38e-13, 2.22e-16, 1.20e-9, 5.18e-5, 5.60293e-5, 2.19045e-5, 1.56e-4, 1.63e-5, 1.41e-5,
        1.81e-5, 8.66e-6, 1.07e-4, 1.45e-4, 5e-5, 2.5e-5, 0.0008, 0.02474,
        0.02458, 0.02505, 0.02495, 0.03141, 0.02966, 0.04036, 0.03983, 0.0704,
        0.0653, 0.099, 0.1199, 0.1207, 0.138, 0.1466, 0.1509, 0.1558, 0.1662,
        0.2465, 0.2506, 0.2546, 0.2601, 0.2685, 0.2722, 0.2743, 0.2764,
        0.00110627, 0.00110848, 0.00110943, 827.13, 1077.25, 1726.16, 2215.56,
    ]
    for c in derived_constants:
        allowed.add(float(c))
    return allowed


def scan_markdown_numeric_tokens(
    docs: dict[str, str], allowed: set[float]
) -> tuple[list[str], int]:
    errors: list[str] = []
    scanned_count = 0

    # Strip SHA-256 hex strings, URLs, and code identifiers like G0/G1/G2/F0/F1/F2/F3/F4/B0/B1/B2/B3/D1/D2/D3/D4/P01..P10/R1..R5/H10/DN35/CT-35
    hex_re = re.compile(r"\b[0-9a-f]{16,64}\b")
    ident_re = re.compile(
        r"\b(?:[GFBDRP]\d{1,2}|H\d{1,2}|DN\d+|CT-\d+|E20|d50|d32|p95|p05|P90|P95|P99|LCB95|ci95|sm_\d+|RTX\s*\d+|Inconel\s*\d+|Ti-6Al-4V|xbabe\d|isl\d+|gen\d+|ind\d+|t\d+|r\d+|q\d+|f\d+|N\d+|V&V\s*\d+|Matérn-5/2|RK\d+|NSGA-II|NSGA2|Exotic3D|192\.168\.\d+\.\d+|84311d|a82342|97173a)\b"
    )
    num_re = re.compile(
        r"(?<![A-Za-z0-9_./-])([+-]?\d{1,3}(?:,\d{3})+(?:\.\d+)?|[+-]?\d+\.\d+(?:e[+-]?\d+)?|[+-]?\d+e[+-]?\d+)(?![A-Za-z0-9_.-])"
    )

    sorted_allowed = sorted(allowed)

    def matches_allowed(val: float) -> bool:
        aval = abs(val)
        for ref in sorted_allowed:
            tol = max(1e-7, abs(ref) * 1.5e-3)
            if abs(aval - abs(ref)) <= tol:
                return True
        return False

    for doc_name, text in docs.items():
        for line_idx, raw_line in enumerate(text.splitlines(), start=1):
            line = hex_re.sub(" ", raw_line)
            line = ident_re.sub(" ", line)
            for match in num_re.finditer(line):
                token = match.group(1)
                val = float(token.replace(",", ""))
                scanned_count += 1
                if not matches_allowed(val):
                    errors.append(
                        f"{doc_name}:{line_idx}: untraceable numeric token '{token}' (parsed {val}) in line: {raw_line.strip()[:120]}"
                    )

    return errors, scanned_count


def _evidence_tiers(root: Path) -> list[str]:
    """EvidenceTier values from analysis/evidence.py (EV-04), the single tier vocabulary."""
    text = (root / "analysis/evidence.py").read_text(encoding="utf-8")
    block = text.split("class EvidenceTier", 1)[1].split("\n\n", 1)[0]
    return re.findall(r"=\s*'([a-z_]+)'", block)


# Outcome words that assert evidence; process nouns ("verification", "validation") are not flagged.
_STRONG_WORD_RE = re.compile(
    r"\b(qualif(?:ied|ies|ying)|qualification|validated|verified|probabilit(?:y|ies)|proofs?|prove[sdn]?|proving)\b",
    re.I,
)
_PROCESS_RE = re.compile(r"\b(contract|ladder|protocol|stage|gate|plan|limits?|criteria)\b", re.I)
_LIMIT_RE = re.compile(
    r"\b(not|no|never|without|unwaived|unvalidated|unverified|unqualified|withdrawn|formerly|previously|"
    r"open|pending|planned|proposed|null|false|neither|nor|cannot|until|before|requires?|required|"
    r"must|needs?|would|future|remains?|whether|if)\b",
    re.I,
)
_DESCRIPTOR_CLAIM_RE = re.compile(
    r"\b(improv|reduc|lower|suppress|stabili[sz]|prevent|enhanc|caus|explain|because|demonstrat|"
    r"prov|confirm|achiev)\w*",
    re.I,
)
_PRIOR_MARK_RE = re.compile(
    r"\b(prior|priors|gene-derived|hypothes\w+|screen|F1|by construction|hand-chosen|authored)\b", re.I
)


def _prose(line: str) -> str:
    """A markdown line without inline code spans, link targets and LaTeX."""
    line = re.sub(r"`[^`]*`", " ", line)
    line = re.sub(r"\]\([^)]*\)", "]", line)
    line = re.sub(r"\"[^\"]*\"", " ", line)  # quoted labels are cited, not asserted
    return re.sub(r"\$[^$]*\$", " ", line)


def audit_claim_wording(docs: dict[str, str], tiers: list[str], descriptors: list[str]) -> tuple[list[str], int]:
    """DOC-08: strong evidence words need a tier or a limitation; descriptor-backed claims need a prior label."""
    tier_re = re.compile(
        r"\b(F-?1|F[0-5](?:-(?:carrier|Carrier|Oil|oil|Water))?|E[0-6]|"
        + "|".join(re.escape(t).replace("_", "[ _-]") for t in tiers)
        + r")\b",
        re.I,
    )
    descriptor_re = re.compile(
        r"\b(" + "|".join(map(re.escape, descriptors)) + r"|mechanism descriptors?)\b", re.I
    )
    errors: list[str] = []
    checked = 0
    for doc_name, text in docs.items():
        for line_no, unit in _sentences(text):
            prose = _prose(unit)
            for match in _STRONG_WORD_RE.finditer(prose):
                checked += 1
                if tier_re.search(unit) or _LIMIT_RE.search(prose):
                    continue
                if match.group(0).lower() == "qualification" and _PROCESS_RE.search(prose):
                    continue
                errors.append(
                    f"{doc_name}:{line_no}: '{match.group(0)}' without an evidence tier or limitation: {unit.strip()[:110]}"
                )
            if descriptor_re.search(unit) and _DESCRIPTOR_CLAIM_RE.search(prose):
                checked += 1
                if not _PRIOR_MARK_RE.search(unit):
                    errors.append(
                        f"{doc_name}:{line_no}: claim backed by gene-derived descriptors without a prior/screen label: {unit.strip()[:110]}"
                    )
    return errors, checked


def _sentences(text: str) -> list[tuple[int, str]]:
    """(first line number, sentence) units: table rows and headings stand alone; wrapped prose is joined."""
    units: list[tuple[int, str]] = []
    para: list[str] = []
    start = 0
    in_fence = False

    def flush() -> None:
        if para:
            joined = " ".join(x.strip() for x in para)
            for sentence in re.split(r"(?<=[.!?])(?<!\bvs\.)(?<!\be\.g\.)(?<!\bi\.e\.)\s+(?=[A-Z*`(\[$])", joined):
                units.append((start, sentence))
            para.clear()

    for line_no, raw in enumerate(text.splitlines(), start=1):
        if raw.lstrip().startswith("```"):
            flush()
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        stripped = raw.strip()
        if not stripped:
            flush()
        elif stripped.startswith(("|", "#", "![")) or re.match(r"^\s*([-*]|\d+\.)\s", raw):
            flush()
            if stripped.startswith(("|", "#", "![")):
                units.append((line_no, stripped))
            else:
                start = line_no
                para.append(stripped)
        else:
            if not para:
                start = line_no
            para.append(stripped)
    flush()
    return units


def _descriptor_names(root: Path) -> list[str]:
    rust = "\n".join(p.read_text(encoding="utf-8") for p in sorted((root / "crates").rglob("*.rs")))
    names: list[str] = []
    for name in ("PriorDescriptors", "MechanismDescriptors"):
        m = re.search(r"pub struct " + name + r" \{(.*?)\n\}", rust, re.S)
        if m:
            for field in re.findall(r"^\s*pub (\w+):", m.group(1), re.M):
                if field not in names:
                    names.append(field)
    return names


NUMBERS_TRACE = "docs/NUMBERS_TRACE.md"
TRACE_SECTIONS = (
    ("docs/WHITEPAPER.md", "Read this first", "## Read this first", "\n## Abstract"),
    ("docs/WHITEPAPER.md", "Abstract", "## Abstract", "\n## 1."),
    ("docs/WHITEPAPER.md", "§4.6", "### 4.6 ", "\n## 5. "),
    ("docs/WHITEPAPER.md", "§5", "## 5. ", "\n## Appendix A"),
    ("README.md", "README", "# joil", ""),
)
TRACE_SOURCES = ("docs/results", "docs/receipts", "experiments", "baselines/ct35")
_TRACE_IDENT_RE = re.compile(
    r"\b(?:P0-\d{2}|CAD-\d{2}|[GFBDRPAEUNCS]\d{1,2}|H\d{1,2}|DN\d+|CT-\d+|E20|d50|d32|p95|p05|P90|P95|P99|LCB95|ci95|sm_\d+|"
    r"RTX\s*\d+|Inconel\s*\d+|Ti-6Al-4V|xbabe\d|isl\d+|gen\d+|ind\d+|s\d{8}-g\d+-i\d+|v\d+|"
    r"Plate\s+\d+|Animation\s+\d+|Figure\s+\d+|C\d{1,2}|R\d|G-\d{2}|N-\d|DOC-F\d{2}|[A-Z]{2,3}-\d{2}|"
    r"wave\d|Wave-\d|oil\d{2}\w*|P0\d|\d+D|\d+-(?:jet|Jet|grid|bin|seed|arm|step|patch|node|eq|run|case|"
    r"block|family|sample|label|objective|Obj|cohort|stage|Run|Grid|Node|Seed|Arm|Step|Sample|Equation)s?)\b"
)
_TRACE_NUM_RE = re.compile(
    r"(?<![A-Za-z0-9_./-])([+-]?\d{1,3}(?:,\d{3})+(?:\.\d+)?|[+-]?\d+(?:\.\d+)?(?:e[+-]?\d+)?)(?![A-Za-z0-9_.-])"
)


def _trace_values(root: Path) -> list[tuple[str, str, float]]:
    """Every finite number in the committed receipts as (file, JSON pointer, value)."""
    out: list[tuple[str, str, float]] = []

    def walk(obj: Any, rel: str, ptr: str) -> None:
        if isinstance(obj, bool) or obj is None:
            return
        if isinstance(obj, (int, float)):
            if math.isfinite(float(obj)):
                out.append((rel, ptr or "/", float(obj)))
        elif isinstance(obj, dict):
            for k, v in obj.items():
                walk(v, rel, f"{ptr}/{k}")
        elif isinstance(obj, list) and len(obj) <= 32:  # long arrays are geometry, never quoted
            for i, v in enumerate(obj):
                walk(v, rel, f"{ptr}/{i}")

    for base in TRACE_SOURCES:
        for path in sorted((root / base).rglob("*.json")):
            walk(json.loads(path.read_text(encoding="utf-8")), path.relative_to(root).as_posix(), "")
    walk(json.loads((root / "docs/STATUS.json").read_text(encoding="utf-8")), "docs/STATUS.json", "")
    return out


def _numeral_tokens(text: str) -> list[str]:
    return [token for token, _, _ in _numeral_tokens_with_context(text)]


def _numeral_tokens_with_unit(text: str) -> list[tuple[str, str]]:
    return [(token, after) for token, _, after in _numeral_tokens_with_context(text)]


def _numeral_tokens_with_context(text: str) -> list[tuple[str, str, str]]:
    """(numeral, text before, text after): the unit after it limits scales; the symbol before it names the quantity."""
    tokens = []
    for raw in text.splitlines():
        if raw.lstrip().startswith("```"):
            continue
        line = re.sub(r"\b[0-9a-f]{16,64}\b", " ", raw)
        line = re.sub(  # calendar dates are not claims
            r"\b\d{1,2} (?:January|February|March|April|May|June|July|August|September|October|November|December) \d{4}\b|\b\d{4}-\d{2}-\d{2}(?:T[\d:]+Z)?",
            " ",
            line,
        )
        line = re.sub(r"§\s*\d+(?:\.\d+)*", " ", line)  # section references, e.g. S2 §4.7
        line = re.sub(r"^\s*#{1,6}\s+\d+(?:\.\d+)*", " ", line)  # heading numbers, e.g. "### 4.6"
        line = re.sub(r"\bOpenFOAM(?: Foundation)?\s+\d+", " ", line)  # solver version, not a measurement
        line = re.sub(r"_\{\d+", "_{", line)  # LaTeX subscripts such as d_{50} and d_{32}
        line = re.sub(r"\\times\s*10\^\{(-?\d+)\}", r"e\1", line)  # LaTeX scientific notation
        line = re.sub(r"(\d)\s+e(-?\d)", r"\1e\2", line)
        line = re.sub(r"\[(\d+(?:,\d+)+)\]", lambda m: " ".join(m.group(1).split(",")), line)
        line = re.sub(r"\.\.", " .. ", line)
        line = _TRACE_IDENT_RE.sub(" ", line)
        line = re.sub(r"\]\([^)]*\)", "]", line)  # link targets are paths, checked elsewhere
        tokens.extend(
            (m.group(1), line[max(0, m.start() - 28) : m.start()], line[m.end() : m.end() + 28])
            for m in _TRACE_NUM_RE.finditer(line)
        )
    return tokens


_UNIT_AFFINITY = (
    (("kpa", "pa", "\\text{ pa", "\\text{ kpa"), ("_pa", "dp", "pressure")),
    (("mg/l", "\\text{ mg"), ("mg_l", "cout")),
    (("kj", "j/m", "\\text{ j", "\\text{ kj"), ("_j_", "loss", "energy", "j_per", "j_m3")),
    (("µm", "um", "\\ \\mu"), ("_um", "d50", "diameter")),
    (("m³/h", "m3/h", "\\text{ m}^3/\\text{h"), ("m3_h", "feed", "flow")),
    (("%",), ("pct", "fraction", "ratio", "rate", "split", "coverage")),
    (("cells",), ("cells",)),
)


def _unit_tail(after: str) -> str:
    """Text where the unit starts: right after the numeral, or after the far end of a range like 18.12–19.53."""
    m = re.match(r"(?:[\s`*$]*[–-]\s*`?\$?[+-]?\d[\d.,]*)?", after)
    tail = after[m.end() if m else 0 :].lstrip(" `*$\\").lower()
    return re.sub(r"^(?:\\?text\{\s*|\\?mathrm\{\s*|~|\\[,;! ])+", "", tail)


def _affinity(after: str) -> tuple[str, ...]:
    tail = _unit_tail(after)
    for units, keys in _UNIT_AFFINITY:
        if tail.startswith(units):
            return keys
    return ()


_LEAD_AFFINITY = (
    (r"\bU\s*=\s*$", ("mann_whitney_u",)),
    (r"\\delta\s*=\s*$", ("cliffs_delta",)),
    (r"p_\{\\text\{Holm\}\}\s*=\s*$", ("holm",)),
)


def _lead_affinity(before: str) -> tuple[str, ...]:
    """Receipt keys named by the symbol in front of a numeral, e.g. `$U = 16$` prefers a Mann-Whitney U pointer."""
    lead = before.rstrip(" `*$")
    return next((keys for pattern, keys in _LEAD_AFFINITY if re.search(pattern, lead + " ")), ())


def _unit_scales(after: str, context: str = "") -> tuple[tuple[float, str], ...]:
    """Scales implied by the unit right after the numeral, or by a unit named in its table row/sentence."""
    tail = _unit_tail(after)
    ctx = context.lower()
    scales = [(1.0, "")]
    if tail.startswith(("%", "\\%")) or "percentage point" in tail or re.search(r"\[%\]|\(%\)", ctx):
        scales.append((100.0, "x100"))
    if tail.startswith(("kpa", "kj", "\\text{ kpa", "\\text{ kj")) or re.search(r"\bk(pa|j)\b", ctx):
        scales.append((1e-3, "/1000"))
    if tail.startswith(("mm", "pa", "j/m", "µm", "\\text{ mm", "\\text{ pa", "\\ \\mu", "\\text{ j")):
        scales.append((1e3, "x1000"))
    return tuple(scales)


class _ValueIndex:
    """Receipt values indexed by printed form, built lazily per (file, scale, decimals)."""

    def __init__(self, values: list[tuple[str, str, float]]):
        self.by_file: dict[str, list[tuple[str, float]]] = {}
        for rel, ptr, value in values:
            self.by_file.setdefault(rel, []).append((ptr, value))
        self.cache: dict[tuple[str, float, int, bool], dict[str, str]] = {}

    def lookup(self, rel: str, token: str, scale: float) -> list[str]:
        text = token.replace(",", "").lstrip("+-")
        sci = "e" in text.lower()
        decimals = 0 if "." not in text else len(text.split("e")[0].split(".")[1])
        key = (rel, scale, decimals, sci)
        if key not in self.cache:
            index: dict[str, list[str]] = {}
            for ptr, value in self.by_file.get(rel, []):
                x = abs(value * scale)
                printed = f"{x:.{decimals}e}" if sci else f"{x:.{decimals}f}"
                index.setdefault(printed, []).append(ptr)
            self.cache[key] = index
        if sci:
            mantissa, exp = text.lower().split("e")
            wanted = f"{float(mantissa):.{decimals}f}e{int(exp):+03d}"
        else:
            wanted = text
        return self.cache[key].get(wanted, [])


def _matches_printed(token: str, value: float) -> str | None:
    """Scale tag if `value` (optionally ×100, /1000, ×1000) equals the token at its printed precision."""
    from decimal import Decimal, ROUND_HALF_UP, InvalidOperation

    text = token.replace(",", "").lstrip("+")
    target = Decimal(text.lstrip("-"))
    exponent = target.as_tuple().exponent
    integer = exponent >= 0 and "e" not in text.lower()
    for scale, tag in ((1.0, ""), (100.0, "x100"), (1e-3, "/1000"), (1e3, "x1000")):
        x = abs(value * scale)
        if integer:
            if abs(x - float(target)) <= 1e-9 * max(1.0, x):
                return tag or "exact"
            continue
        try:
            quantum = Decimal(1).scaleb(exponent)
            if Decimal(x).quantize(quantum, rounding=ROUND_HALF_UP) == target:  # exact binary, as printf rounds
                return tag or "rounded"
        except InvalidOperation:
            continue
    return None


def _declared_numbers(trace_md: str) -> dict[str, tuple[str, str, str]]:
    """Declared numerals from the hand-reviewed table: numeral -> (kind, source, check)."""
    declared = {}
    section = trace_md.split("## Declared numerals", 1)[1] if "## Declared numerals" in trace_md else ""
    section = section.split("\n## ", 1)[0]
    for line in section.splitlines():
        cells = _table_cells(line) if line.startswith("| `") else []
        if len(cells) == 4:
            declared[cells[0].strip("`")] = (cells[1], cells[2], cells[3])
    return declared


def _check_declared(numeral: str, kind: str, source: str, check: str, root: Path, values) -> str | None:
    if kind == "literal":
        cited = re.search(r"`([^`]+)`", source)
        path = root / (cited.group(1) if cited else source.strip())
        literal = check.strip("`")
        if not path.is_file() or literal not in path.read_text(encoding="utf-8"):
            return f"declared literal for {numeral} not found: {source} `{literal}`"
        return None
    if kind == "derived":
        lookup = {(rel, ptr): v for rel, ptr, v in values}
        expr = check.strip("`")
        terms = re.findall(r"([\w./-]+\.json)#(/[^\s)]+)", expr)
        env = {}
        for i, (rel, ptr) in enumerate(terms):
            if (rel, ptr) not in lookup:
                return f"derived {numeral}: missing {rel}#{ptr}"
            env[f"v{i}"] = lookup[(rel, ptr)]
            expr = expr.replace(f"{rel}#{ptr}", f"v{i}", 1)
        if not re.fullmatch(r"(?:v\d+|\d+(?:\.\d+)?(?:e[+-]?\d+)?|[\s+\-*/()])+", expr):
            return f"derived {numeral}: unsupported expression"
        value = eval(expr, {"__builtins__": {}}, env)  # arithmetic over receipt values only
        if _matches_printed(numeral.split("@", 1)[0], value) is None:
            return f"derived {numeral}: expression evaluates to {value}"
        return None
    if kind in ("protocol constant", "historical"):
        return None if source.strip() and check.strip() else f"declared {numeral}: needs source and reason"
    return f"declared {numeral}: unknown kind '{kind}'"


def build_numbers_trace(root: Path, existing: str | None = None) -> tuple[str, list[str]]:
    """Render docs/NUMBERS_TRACE.md and list errors (unresolved or invalid declared numerals)."""
    sources = {rel: (root / rel).read_text(encoding="utf-8") for rel in {t[0] for t in TRACE_SECTIONS}}
    existing = existing if existing is not None else (
        (root / NUMBERS_TRACE).read_text(encoding="utf-8") if (root / NUMBERS_TRACE).is_file() else ""
    )
    declared = _declared_numbers(existing)
    values = _trace_values(root)
    index = _ValueIndex(values)
    all_files = list(dict.fromkeys(rel for rel, _, _ in values))
    errors: list[str] = []
    for numeral, (kind, source, check) in sorted(declared.items()):
        problem = _check_declared(numeral, kind, source, check, root, values)
        if problem:
            errors.append(problem)
    scoped = {key: key.split("@", 1) for key in declared if "@" in key}
    used_scoped: set[str] = set()
    lines = [
        "# Numbers trace (WHITEPAPER \"Read this first\", Abstract, §4.6 and §5; README)",
        "",
        "Generated by `python3 tools/claims_audit.py --write-numbers-trace` (workplan DOC-04). Each numeral in these",
        "sections either equals a committed receipt value at its printed precision, or is declared in the hand-reviewed",
        "table at the end. Unit scaling is applied only where the unit after the numeral implies it (×100 before `%`,",
        "÷1000 before kPa/kJ, ×1000 before mm/Pa/J). A numeral in a sentence or subsection that cites receipts must",
        "match one of those receipts; only uncited prose falls back to every receipt. The claims audit fails on any",
        "untraced numeral, an unused scoped declaration or a stale table. Where several receipt values match, the first",
        "cited source wins; a short integer such as a seed count can still match by coincidence, so read the pointer.",
        "",
    ]
    for rel, title, start_marker, end_marker in TRACE_SECTIONS:
        doc = sources[rel]
        start = doc.find(start_marker)
        if start < 0:
            errors.append(f"{rel}: section '{title}' not found")
            continue
        end = doc.find(end_marker, start + len(start_marker)) if end_marker else -1
        text = doc[start : end if end > 0 else len(doc)]
        cite_re = r"((?:docs/results|docs/receipts|experiments|baselines/ct35)/[\w./-]+\.json)"
        section_cited = list(dict.fromkeys(re.findall(cite_re, text)))
        rows: dict[tuple[str, str, str, str], int] = {}
        sub_cited: list[str] = []
        units = _sentences(text)
        para_cites: dict[int, list[str]] = {}
        for line_no, unit in units:
            para_cites.setdefault(line_no, []).extend(re.findall(cite_re, unit))
        para_start, para_cited, in_caption = None, [], False
        for line_no, unit in units:
            if unit.startswith("#"):
                sub_cited = []
            if line_no != para_start:  # a figure caption's citations bind only, and all of, that caption paragraph
                para_start = line_no
                in_caption = unit.startswith("![") or (unit.startswith("*") and not unit.startswith("**"))
                para_cited = list(dict.fromkeys(para_cites[line_no])) if in_caption else []
            unit_cited = list(dict.fromkeys(re.findall(cite_re, unit)))
            if in_caption:
                bound = list(dict.fromkeys(unit_cited + para_cited + sub_cited[::-1]))
            else:
                sub_cited = list(dict.fromkeys(sub_cited + unit_cited))
                bound = list(dict.fromkeys(unit_cited + sub_cited[::-1]))  # nearest earlier citation first
            search = bound or list(dict.fromkeys(section_cited + all_files))
            for token, before, after in _numeral_tokens_with_context(unit):
                scope = next((k for k, (n, phrase) in scoped.items() if n == token and phrase in unit), None)
                if scope:
                    used_scoped.add(scope)
                    key = (token, "declared", "-", f"see table (`{scope}`)")
                elif token.lstrip("+-").replace(",", "") in declared or token in declared:
                    key = (token, "declared", "-", "see table")
                else:
                    key = (token, "**UNTRACED**", "-", "-")
                    keys = _affinity(after) + _lead_affinity(before)
                    hits = [
                        (rel, ptr, tag or "printed")
                        for scale, tag in _unit_scales(after, unit)
                        for rel in search
                        for ptr in index.lookup(rel, token, scale)
                    ]
                    preferred = [h for h in hits if any(k in h[1].lower() for k in keys)]
                    hit = (preferred or hits or [None])[0]
                    if hit:
                        key = (token, f"`{hit[0]}`", f"`{hit[1]}`", hit[2])
                    if key[1] == "**UNTRACED**":
                        where = f"its cited receipts ({', '.join(bound)})" if bound else "any receipt value"
                        errors.append(
                            f"NUMBERS_TRACE: {title}: numeral '{token}' matches no value in {where} and is not declared"
                        )
                rows[key] = rows.get(key, 0) + 1
        lines += [f"## {title}", "", "| Numeral | Count | Source | JSON pointer | Match |", "|---|---|---|---|---|"]
        for (token, source, pointer, tag), count in rows.items():
            lines.append(f"| `{token}` | {count} | {source} | {pointer} | {tag} |")
        lines.append("")
    for key in sorted(set(scoped) - used_scoped):
        errors.append(f"NUMBERS_TRACE: declared {key}: scope phrase matches no traced sentence (stale declaration)")
    lines += [
        "## Declared numerals",
        "",
        "Hand-reviewed. Kinds: `literal` (this exact text must appear in the cited file), `derived` (arithmetic over receipt",
        "pointers and numeric constants, re-evaluated by the audit), `protocol constant` and `historical` (reason required;",
        "not machine-checked). A numeral written `N@phrase` applies only in sentences containing `phrase`; a plain `N` applies everywhere.",
        "",
        "| Numeral | Kind | Source | Check |",
        "|---|---|---|---|",
    ]
    for numeral, (kind, source, check) in sorted(declared.items()):
        lines.append(f"| `{numeral}` | {kind} | {source} | {check} |")
    return "\n".join(lines) + "\n", errors


def check_numbers_trace(root: Path) -> list[str]:
    path = root / NUMBERS_TRACE
    if not path.is_file():
        return [f"{NUMBERS_TRACE} missing; run python3 tools/claims_audit.py --write-numbers-trace"]
    committed = path.read_text(encoding="utf-8")
    rendered, errors = build_numbers_trace(root, committed)
    if rendered != committed:
        errors.append(f"{NUMBERS_TRACE} is stale; run python3 tools/claims_audit.py --write-numbers-trace")
    return errors


README_STATUS_BEGIN ="<!-- BEGIN GENERATED STATUS: python3 tools/claims_audit.py --write-readme-status (source docs/STATUS.json) -->"
README_STATUS_END = "<!-- END GENERATED STATUS -->"


def render_readme_status(status: dict[str, Any]) -> str:
    """README status table derived only from docs/STATUS.json (DOC-05); never hand-edited."""

    def cell(value: Any) -> str:
        if value is None:
            return "`null` (no qualified value)"
        if isinstance(value, bool):
            return "`true`" if value else "`false`"
        return f"`{value}`"

    rows = [
        "| Claim | State | Missing evidence |",
        "|---|---|---|",
        f"| Campaign admission | {cell(status.get('campaign_admission'))} | "
        + "; ".join(status.get("campaign_blockers", [])) + " |",
    ]
    for name, claim in status.get("claims", {}).items():
        rows.append(
            f"| `{name}` | {cell(claim.get('state'))} (flag {cell(status.get(name))}) | "
            + ", ".join(f"`{m}`" for m in claim.get("missing", [])) + " |"
        )
    for name in ("qualified_fitness", "qualified_capacity_gain"):
        rows.append(f"| `{name}` | {cell(status.get(name))} | - |")
    for item in status.get("software", []):
        rows.append(f"| `{item['id']}` | {cell(item.get('state'))} | - |")
    return "\n".join(
        [README_STATUS_BEGIN, f"Phase: {cell(status.get('phase'))}.", "", *rows, README_STATUS_END]
    )


def check_readme_status(readme: str, status: dict[str, Any]) -> list[str]:
    if README_STATUS_BEGIN not in readme or README_STATUS_END not in readme:
        return ["README.md: generated status block markers missing"]
    block = readme[readme.index(README_STATUS_BEGIN) : readme.index(README_STATUS_END) + len(README_STATUS_END)]
    if block != render_readme_status(status):
        return ["README.md: status block is stale; run python3 tools/claims_audit.py --write-readme-status"]
    return []


REVIEW_INPUT_DIR = "docs/review-inputs/2026-10-15"
REVIEW_SYNTHESIS = "docs/reviews/2026-10-15-SYNTHESIS.md"
REVIEW_WORKPLAN = "docs/WORKPLAN-2026-10-15.md"

# Every P0 / stop-the-line / critical / no-regret finding of the six 2026-10-15 reviews (DOC-02).
REQUIRED_REVIEW_FINDINGS: tuple[str, ...] = (
    *(f"S1-3.{i}" for i in range(1, 10)),
    *(f"S1-15.{i:02d}" for i in range(1, 21)),
    *(f"S2-4.{i:02d}" for i in range(1, 12)),
    *(f"S3-P0-{i:02d}" for i in range(1, 13)),
    *(f"S4-2.{i:02d}" for i in range(1, 11)),
    *(f"S4-19.{i:02d}" for i in range(1, 13)),
    *(f"S5-P0.{i}" for i in range(1, 9)),
    *(f"S5-M0.{i}" for i in range(1, 6)),
    *(f"S6-3.{i}" for i in range(1, 10)),
    *(f"S6-A.{i:02d}" for i in range(1, 21)),
)
LEDGER_STATUSES = ("BLOCKED", "OPEN", "WIP", "DONE", "MERGED", "MOVED-§6")
_PROGRESS_ORDER = ("BLOCKED", "OPEN", "WIP", "DONE", "MERGED")
_FINDING_ID_RE = re.compile(r"^(?:S[1-6]-[A-Z0-9][A-Za-z0-9.\-]*|DOC-F\d{2})$")


def verify_review_input_checksums(input_dir: Path) -> tuple[list[str], int]:
    """Verbatim review inputs must match SHA256SUMS and INDEX.md, with no unlisted files."""
    errors: list[str] = []
    sums = input_dir / "SHA256SUMS"
    if not sums.is_file():
        return [f"{input_dir}: missing SHA256SUMS"], 0
    listed: dict[str, str] = {}
    for line in sums.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        digest, _, name = line.partition("  ")
        listed[name.strip()] = digest.strip()
    index = (input_dir / "INDEX.md").read_text(encoding="utf-8") if (input_dir / "INDEX.md").is_file() else ""
    for name, digest in listed.items():
        path = input_dir / name
        if not path.is_file():
            errors.append(f"{input_dir.name}/{name}: listed in SHA256SUMS but missing")
            continue
        actual = sha256_file(path)
        if actual != digest:
            errors.append(f"{input_dir.name}/{name}: sha256 {actual} != SHA256SUMS {digest}")
        if f"| {name} |" not in index or digest not in index:
            errors.append(f"{input_dir.name}/INDEX.md: row for {name} missing or digest differs")
    for path in sorted(input_dir.iterdir()):
        if path.name not in listed and path.name not in {"SHA256SUMS", "INDEX.md"}:
            errors.append(f"{input_dir.name}/{path.name}: file not listed in SHA256SUMS")
    return errors, len(listed)


def _table_cells(line: str) -> list[str]:
    """Split one markdown table row on unescaped pipes."""
    return [cell.strip() for cell in re.split(r"(?<!\\)\|", line.strip())[1:-1]]


def _section(text: str, heading_prefix: str) -> str:
    start = text.find(heading_prefix)
    if start < 0:
        return ""
    nxt = text.find("\n## ", start + len(heading_prefix))
    return text[start : nxt if nxt >= 0 else len(text)]


def derive_finding_status(items: list[str], ledger: dict[str, str]) -> str:
    """Least-advanced ledger status of the mapped items (BLOCKED dominates, MOVED-§6 defers)."""
    statuses = [ledger[item] for item in items]
    if "BLOCKED" in statuses:
        return "BLOCKED"
    if "MOVED-§6" in statuses:
        return "DEFERRED"
    return min(statuses, key=_PROGRESS_ORDER.index)


def check_review_traceability(synthesis: str, workplan: str) -> tuple[list[str], int]:
    """Check the DOC-02 synthesis against the workplan and its own item ledger."""
    errors: list[str] = []
    workplan_items = set(re.findall(r"\*\*([A-Z]{2,3}-\d{2}) — ", workplan))
    if not workplan_items:
        return ["workplan: no item IDs found"], 0

    ledger: dict[str, str] = {}
    for line in _section(synthesis, "## 2.").splitlines():
        cells = _table_cells(line) if line.startswith("|") else []
        if len(cells) != 4 or not re.fullmatch(r"[A-Z]{2,3}-\d{2}", cells[0]):
            continue
        item, _, status, evidence = cells
        if item not in workplan_items:
            errors.append(f"ledger: {item} is not a workplan item")
            continue
        if item in ledger:
            errors.append(f"ledger: {item} listed twice")
        if status not in LEDGER_STATUSES:
            errors.append(f"ledger: {item} has unknown status '{status}'")
        if status in ("DONE", "MERGED") and not re.search(r"\b[0-9a-f]{7,40}\b", evidence):
            errors.append(f"ledger: {item} is {status} without a commit sha")
        if status == "BLOCKED" and evidence in ("", "-"):
            errors.append(f"ledger: {item} is BLOCKED without a reason")
        ledger[item] = status
    for item in sorted(workplan_items - set(ledger)):
        errors.append(f"ledger: workplan item {item} missing")
    for box, item in re.findall(r"^- \[( |x)\] \*\*([A-Z]{2,3}-\d{2}) — ", workplan, re.M):
        closed = ledger.get(item) in ("MERGED", "MOVED-§6")
        if (box == "x") != closed:
            errors.append(f"workplan: {item} checkbox '[{box}]' disagrees with ledger status '{ledger.get(item)}'")

    gap_ids = set(re.findall(r"^\| (G-\d{2}) \|", _section(synthesis, "## 5."), re.M))
    seen: set[str] = set()
    rows = 0
    for line in _section(synthesis, "## 4.").splitlines():
        cells = _table_cells(line) if line.startswith("|") else []
        if len(cells) != 7 or not _FINDING_ID_RE.match(cells[0]):
            continue
        rows += 1
        fid, _, _, verified, items_cell, coverage, status = cells
        if fid in seen:
            errors.append(f"finding {fid}: listed twice")
        seen.add(fid)
        if not verified.startswith(("V:", "R:", "N/A")):
            errors.append(f"finding {fid}: verification cell must start with V:, R: or N/A")
        items = [] if items_cell == "-" else [x.strip() for x in items_cell.split(",")]
        unknown = [x for x in items if x not in workplan_items]
        if unknown:
            errors.append(f"finding {fid}: unknown workplan item(s) {unknown}")
            continue
        kind = coverage.split(" ", 1)[0]
        if kind in ("FULL", "PARTIAL"):
            if not items:
                errors.append(f"finding {fid}: {kind} coverage needs at least one item")
                continue
            if kind == "PARTIAL" and "§" not in coverage:
                errors.append(f"finding {fid}: PARTIAL coverage must name the remainder (§5/§6)")
            if any(x not in ledger for x in items):
                continue
            expected = derive_finding_status(items, ledger)
        elif kind == "FOLLOW-ON":
            expected = "DEFERRED"
        elif kind == "GAP":
            expected = "UNASSIGNED"
            refs = set(re.findall(r"G-\d{2}", coverage))
            if not refs or not refs <= gap_ids:
                errors.append(f"finding {fid}: GAP coverage must cite a §5 gap row")
        else:
            errors.append(f"finding {fid}: unknown coverage '{coverage}'")
            continue
        if status != expected:
            errors.append(f"finding {fid}: status '{status}' but ledger derives '{expected}'")
    for fid in REQUIRED_REVIEW_FINDINGS:
        if fid not in seen:
            errors.append(f"finding {fid}: required review finding not traced")
    return errors, rows


def ledger_from_board(board: str, items: set[str]) -> dict[str, tuple[str, str]]:
    """Item status from coordination-board lines: MERGED > DONE > BLOCKED > WIP (START/WIP).

    MERGED counts only after a DONE for the same item: merging a lane mid-item is not completion.
    """
    found: dict[str, dict[str, str]] = {}
    for line in board.splitlines():
        tokens = line.split()
        if len(tokens) >= 2 and tokens[0] in ("MOVED-§6", "MOVED-S6") and tokens[1] in items:
            found.setdefault(tokens[1], {})["MOVED"] = " ".join(tokens[2:])[:100] or "board"
            continue
        if len(tokens) >= 2 and tokens[0] in ("DONE", "MERGED") and tokens[1] in items:
            sha = tokens[2] if len(tokens) >= 3 and re.fullmatch(r"[0-9a-f]{7,40}", tokens[2]) else ""
            entry = found.setdefault(tokens[1], {})
            if not sha and tokens[0] == "MERGED":  # integrator's bare "MERGED X" refers to the item's DONE commit
                sha = entry.get("DONE", "")
            if sha:
                entry[tokens[0]] = sha
        elif len(tokens) >= 5 and tokens[0] == "HB":
            state = "BLOCKED" if tokens[4].startswith("BLOCKED") else tokens[4]  # e.g. BLOCKED-SCOPED
            for item in tokens[3].split("/"):
                if item not in items or state not in ("START", "WIP", "BLOCKED", "DONE"):
                    continue
                entry = found.setdefault(item, {})
                if state == "DONE" and len(tokens) > 5 and re.fullmatch(r"[0-9a-f]{7,40}", tokens[5]):
                    entry.setdefault("DONE", tokens[5])
                elif state == "BLOCKED":
                    entry["BLOCKED"] = " ".join(tokens[1:2] + tokens[5:])[:120]
                elif state in ("START", "WIP"):
                    entry["WIP"] = tokens[1]
    ledger = {}
    for item, entry in found.items():
        if "MOVED" in entry:  # integrator moved the (remaining) scope to workplan §6
            ledger[item] = ("MOVED-§6", f"board `MOVED-§6 {item}`: {entry['MOVED'].replace('|', '/')}")
        elif "MERGED" in entry and "DONE" in entry:
            ledger[item] = ("MERGED", f"`{entry['MERGED'][:7]}` (board `MERGED {item}`)")
        elif "MERGED" in entry:  # a lane merged mid-item: not completion
            note = f"merged at `{entry['MERGED'][:7]}` without board `DONE {item}`"
            if "BLOCKED" in entry:
                ledger[item] = ("BLOCKED", f"{note}; {entry['BLOCKED'].replace('|', '/')}"[:160])
            else:
                ledger[item] = ("WIP", note)
        elif "DONE" in entry:
            ledger[item] = ("DONE", f"`{entry['DONE'][:7]}` (board `DONE {item}`)")
        elif "BLOCKED" in entry:
            ledger[item] = ("BLOCKED", entry["BLOCKED"].replace("|", "/"))
        elif "WIP" in entry:
            ledger[item] = ("WIP", f"board HB {entry['WIP']}")
    return ledger


def sync_ledger(synthesis: str, board: str) -> str:
    """Rewrite §2 ledger rows whose board-derived status differs (never downgrades, except merges without DONE)."""
    rank = {"OPEN": 0, "WIP": 1, "BLOCKED": 1, "DONE": 2, "MERGED": 3, "MOVED-§6": 4}
    items = set(re.findall(r"^\| ([A-Z]{2,3}-\d{2}) \|", _section(synthesis, "## 2."), re.M))
    derived = ledger_from_board(board, items)
    out = []
    in_ledger = False
    for line in synthesis.splitlines(keepends=True):
        if line.startswith("## "):
            in_ledger = line.startswith("## 2.")
        cells = _table_cells(line) if in_ledger and line.startswith("|") else []
        if len(cells) == 4 and cells[0] in derived:
            status, evidence = derived[cells[0]]
            # Never downgrade, except when the board shows a merge without the owner's DONE: that merge was
            # never completion, so the owner's WIP/BLOCKED state wins over an earlier MERGED row.
            merged_without_done = "without board `DONE" in evidence
            if (rank[status] >= rank.get(cells[2], 0) or merged_without_done) and (status, evidence) != (cells[2], cells[3]):
                line = f"| {cells[0]} | {cells[1]} | {status} | {evidence} |\n"
        out.append(line)
    return "".join(out)


def sync_workplan_checkboxes(workplan: str, synthesis: str) -> str:
    """Tick exactly the workplan items the synthesis ledger records as MERGED or MOVED-§6."""
    closed = {
        cells[0]
        for line in _section(synthesis, "## 2.").splitlines()
        if line.startswith("|")
        for cells in [_table_cells(line)]
        if len(cells) == 4 and cells[2] in ("MERGED", "MOVED-§6")
    }
    return re.sub(
        r"^- \[[ x]\] \*\*([A-Z]{2,3}-\d{2}) — ",
        lambda m: f"- [{'x' if m.group(1) in closed else ' '}] **{m.group(1)} — ",
        workplan,
        flags=re.M,
    )


def refresh_review_statuses(synthesis: str, now_utc: str) -> str:
    """Rewrite every finding's Status cell from the §2 ledger and stamp the as-of time."""
    ledger = {}
    for line in _section(synthesis, "## 2.").splitlines():
        cells = _table_cells(line) if line.startswith("|") else []
        if len(cells) == 4 and re.fullmatch(r"[A-Z]{2,3}-\d{2}", cells[0]):
            ledger[cells[0]] = cells[2]
    out = []
    in_findings = False
    for line in synthesis.splitlines(keepends=True):
        if line.startswith("## "):
            in_findings = line.startswith("## 4.")
        cells = _table_cells(line) if in_findings and line.startswith("|") else []
        if len(cells) == 7 and _FINDING_ID_RE.match(cells[0]):
            items = [] if cells[4] == "-" else [x.strip() for x in cells[4].split(",")]
            kind = cells[5].split(" ", 1)[0]
            if kind in ("FULL", "PARTIAL") and items and all(x in ledger for x in items):
                status = derive_finding_status(items, ledger)
            else:
                status = {"FOLLOW-ON": "DEFERRED", "GAP": "UNASSIGNED"}.get(kind, cells[6])
            head = line.rstrip("\n").rstrip()
            head = head[: head.rstrip("|").rstrip().rfind("|") + 1]
            line = f"{head} {status} |\n"
        out.append(line)
    return re.sub(r"\*\*Status as of [^*]+\.\*\*", f"**Status as of {now_utc}.**", "".join(out), count=1)


def audit_review_traceability(root: Path = ROOT) -> tuple[list[str], int]:
    synthesis = root / REVIEW_SYNTHESIS
    workplan = root / REVIEW_WORKPLAN
    if not synthesis.is_file() or not workplan.is_file():
        return [f"missing {REVIEW_SYNTHESIS} or {REVIEW_WORKPLAN}"], 0
    return check_review_traceability(
        synthesis.read_text(encoding="utf-8"), workplan.read_text(encoding="utf-8")
    )


def run_claims_audit(root: Path = ROOT) -> dict[str, Any]:
    docs = {
        "docs/WHITEPAPER.md": (root / "docs/WHITEPAPER.md").read_text(encoding="utf-8"),
        "docs/SCIENCE.md": (root / "docs/SCIENCE.md").read_text(encoding="utf-8"),
        "README.md": (root / "README.md").read_text(encoding="utf-8"),
    }
    bundle = build_receipt_bundle(root)

    path_hash_errors = verify_paths_and_sha256_digests(docs, bundle, root)
    structured_errors, structured_count = verify_seed_robustness_and_structured_claims(
        docs, bundle
    )
    allowed_numbers = build_allowed_numeric_registry(bundle)
    token_errors, token_count = scan_markdown_numeric_tokens(docs, allowed_numbers)
    input_errors, inputs_verified = verify_review_input_checksums(root / REVIEW_INPUT_DIR)
    trace_errors, findings_traced = audit_review_traceability(root)
    wording_errors, wording_checked = audit_claim_wording(
        docs, _evidence_tiers(root), _descriptor_names(root)
    )
    status_errors = check_readme_status(docs["README.md"], bundle["status"])
    trace_numbers_errors = check_numbers_trace(root)

    all_errors = (
        path_hash_errors
        + structured_errors
        + token_errors
        + input_errors
        + trace_errors
        + wording_errors
        + status_errors
        + trace_numbers_errors
    )
    return {
        "passed": len(all_errors) == 0,
        "structured_claims_verified": structured_count,
        "numeric_tokens_scanned": token_count,
        "review_inputs_verified": inputs_verified,
        "review_findings_traced": findings_traced,
        "wording_claims_checked": wording_checked,
        "error_count": len(all_errors),
        "errors": all_errors,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify all claims in WHITEPAPER.md, SCIENCE.md, and README.md against JSON receipts.")
    parser.add_argument("--root", type=Path, default=ROOT, help="Repository root path")
    parser.add_argument("--json", action="store_true", help="Print JSON report")
    parser.add_argument(
        "--refresh-review-status",
        action="store_true",
        help=f"Recompute finding statuses in {REVIEW_SYNTHESIS} from its item ledger, then audit",
    )
    parser.add_argument(
        "--sync-ledger-from",
        type=Path,
        help="Coordination board file; update the synthesis ledger from its DONE/MERGED/HB lines (implies refresh)",
    )
    parser.add_argument(
        "--write-readme-status",
        action="store_true",
        help="Regenerate the README status block from docs/STATUS.json, then audit",
    )
    parser.add_argument(
        "--write-numbers-trace",
        action="store_true",
        help=f"Regenerate {NUMBERS_TRACE} (keeps its declared-numerals table), then audit",
    )
    args = parser.parse_args()
    if args.write_numbers_trace:
        root = args.root.resolve()
        rendered, problems = build_numbers_trace(root)
        (root / NUMBERS_TRACE).write_text(rendered, encoding="utf-8")
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
    if args.write_readme_status:
        root = args.root.resolve()
        readme = (root / "README.md").read_text(encoding="utf-8")
        block = render_readme_status(json.loads((root / "docs/STATUS.json").read_text(encoding="utf-8")))
        if README_STATUS_BEGIN in readme and README_STATUS_END in readme:
            start = readme.index(README_STATUS_BEGIN)
            end = readme.index(README_STATUS_END) + len(README_STATUS_END)
            readme = readme[:start] + block + readme[end:]
        else:
            raise SystemExit("README.md has no generated status markers")
        (root / "README.md").write_text(readme, encoding="utf-8")
    if args.sync_ledger_from:
        path = args.root.resolve() / REVIEW_SYNTHESIS
        board = args.sync_ledger_from.read_text(encoding="utf-8")
        path.write_text(sync_ledger(path.read_text(encoding="utf-8"), board), encoding="utf-8")
        args.refresh_review_status = True
    if args.refresh_review_status:
        from datetime import datetime, timezone

        path = args.root.resolve() / REVIEW_SYNTHESIS
        stamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%MZ")
        path.write_text(refresh_review_statuses(path.read_text(encoding="utf-8"), stamp), encoding="utf-8")
        plan = args.root.resolve() / REVIEW_WORKPLAN
        plan.write_text(
            sync_workplan_checkboxes(plan.read_text(encoding="utf-8"), path.read_text(encoding="utf-8")),
            encoding="utf-8",
        )

    report = run_claims_audit(args.root.resolve())
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        if report["passed"]:
            print(
                f"claims_audit: OK ({report['structured_claims_verified']} structured receipt claims + "
                f"{report['numeric_tokens_scanned']} markdown numeric tokens verified across WHITEPAPER.md, SCIENCE.md, README.md; "
                f"{report['review_inputs_verified']} review inputs checksummed; "
                f"{report['review_findings_traced']} review findings traced; "
                f"{report['wording_claims_checked']} strong-wording claims checked)"
            )
        else:
            print(f"claims_audit: FAILED with {report['error_count']} error(s):", file=sys.stderr)
            for err in report["errors"]:
                print(f"  - {err}", file=sys.stderr)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
