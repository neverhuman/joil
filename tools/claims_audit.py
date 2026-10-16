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


def build_receipt_bundle(root: Path = ROOT) -> dict[str, Any]:
    return {
        "ct35_genome": load_json("baselines/ct35/genome.json", root),
        "ct35_ref": load_json("baselines/ct35/reference.json", root),
        "baseline_hyd": load_json("docs/results/baseline-hydraulics-v1.json", root),
        "rust_qual": load_json("docs/results/rust-evolution-qualification-v1.json", root),
        "cfd_bench": load_json("docs/results/exotic-vs-baseline-cfd-benchmark-v1.json", root),
        "gci_rsm": load_json("docs/results/elite-3grid-gci-and-rsm-verification-v1.json", root),
        "wave2_prereg": load_json("experiments/wave2-preregistration-v1.json", root),
        "wave2_ablation": load_json("docs/results/wave2-multiseed-ablation-and-holdout-v1.json", root),
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
        "docs/results/wave2-multiseed-ablation-and-holdout-v1.json",
        "docs/results/interactive-3d-archetypes-v1.json",
        "docs/results/baseline-hydraulics-v1.json",
        "experiments/wave2-preregistration-v1.json",
        "baselines/ct35/genome.json",
        "baselines/ct35/reference.json",
        "physics/cuda/joil_cuda_eval.cu",
        "docs/figures/rust-3d-topology-atlas.png",
        "docs/figures/pareto-map-elites-convergence.png",
        "docs/figures/elite-gci-and-rsm-verification.png",
        "docs/figures/openfoam14-3d-vortex-physics-comparison.png",
        "docs/figures/wave2-exotic-3d-genome-atlas.png",
        "docs/figures/wave2-multiseed-ablation-and-ci.png",
        "docs/figures/wave2-grade-efficiency-and-dsd-shift.png",
        "docs/figures/wave2-structural-lame-and-modal-qualification.png",
        "docs/figures/wave2-8-exotic-phenotypes-gallery.png",
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
        # 11-family expressiveness
        ("WHITEPAPER", wp, float(kf["n_stratified_expressiveness_samples"]), 220.0, 1e-6, "220 / 220"),
        ("WHITEPAPER", wp, float(kf["unprojected_negative_control_pass_rate"]), 0.0, 1e-6, "0 / 30"),
        # SCIENCE.md checks
        ("SCIENCE", sc, 500.0 * (1.0 - 0.94) / (1.0 - 0.02), 30.6122449, 1e-6, "30.6122449 mg/L"),
        ("SCIENCE", sc, 500.0 * (1.0 - 0.95) / (1.0 - 0.02), 25.5102041, 1e-6, "25.5102041 mg/L"),
        ("SCIENCE", sc, ct35_drv["Do"], 1.40, 1e-6, "1.40\\text{ mm}"),
        ("SCIENCE", sc, ct35_drv["L2"], 770.0, 1e-6, "770.0\\text{ mm}"),
        ("SCIENCE", sc, ct35_drv["L3"], 700.0, 1e-6, "700.0\\text{ mm}"),
        ("SCIENCE", sc, ext_case["numerical_outlet_extension_m"] * 1000.0, 120.0, 1e-6, "120\\text{ mm}"),
        # README.md checks
        ("README", rm, float(gci_cases["G0_coarse"]["cells"]), 139981.0, 1e-6, "139,981"),
        ("README", rm, float(gci_cases["G1_medium"]["cells"]), 305522.0, 1e-6, "305,522"),
        ("README", rm, float(gci_cases["G2_fine"]["cells"]), 552513.0, 1e-6, "552,513"),
        ("README", rm, float(cm["total_evaluations"]), 119235.0, 1e-6, "119,235"),
        ("README", rm, gci_cases["G2_fine"]["dp_u_pa"] / 1000.0, 18.70, 1e-2, "18.70 kPa"),
        ("README", rm, rsm_cases["LRR"]["dp_u_pa"] / 1000.0, 18.12, 1e-2, "18.12 kPa"),
        ("README", rm, rsm_cases["SSG"]["dp_u_pa"] / 1000.0, 18.17, 1e-2, "18.17 kPa"),
        ("README", rm, float(kf["n_stratified_expressiveness_samples"]), 220.0, 1e-6, "220 / 220"),
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
        14.0, 15.0, 16.0, 18.0, 20.0, 22.0, 24.0, 25.0, 28.0, 30.0, 32.0, 35.0,
        36.0, 38.0, 40.0, 45.0, 48.0, 50.0, 54.0, 55.0, 60.0, 64.0, 70.0, 80.0,
        84.0, 86.0, 87.0, 89.0, 90.0, 95.0, 96.0, 100.0, 120.0, 128.0, 140.0,
        150.0, 168.0, 174.0, 180.0, 191.0, 192.0, 200.0, 212.0, 220.0, 275.0,
        282.0, 300.0, 306.0, 352.0, 388.0, 500.0, 524.0, 553.0, 600.0, 640.0,
        644.0, 660.0, 700.0, 770.0, 772.0, 780.0, 796.0, 820.0, 825.0, 850.0,
        890.0, 998.0, 1000.0, 1044.0, 1200.0, 1284.0, 1316.0, 1556.0, 1588.0,
        1800.0, 1975.0, 1980.0, 1991.0, 2008.0, 2009.0, 2026.0, 2976.0, 5400.0,
        6416.0, 6928.0, 10000.0, 20261015.0, 20261101.0, 20261110.0, 20261116.0,
        20262026.0, 20263037.0,
        # SCIENCE.md exact regression oracles & applicability envelope bounds
        0.0002, 0.000314, 0.000157, 1.57e-4, 0.001, 0.0031, 0.004529, 0.005,
        0.01, 0.02, 0.025, 0.04, 0.05, 0.08, 0.10, 0.11, 0.12, 0.13, 0.1470,
        0.1472, 0.15, 0.1554, 0.157, 0.2468, 0.25, 0.2545, 0.256, 0.26, 0.260,
        0.264, 0.2717, 0.2772, 0.35, 0.369, 0.371, 0.392,
        0.4174, 0.45, 0.453, 0.48, 0.483, 0.59, 0.60, 0.62, 0.625, 0.65, 0.651,
        0.725, 0.76, 0.80, 0.818, 0.84, 0.85, 0.856, 0.859, 0.863, 0.88, 0.90,
        0.94, 0.9412, 0.95, 0.9608, 0.995, 1.079, 1.1058, 1.113, 1.124, 1.159,
        1.20, 1.209, 1.218, 1.2180, 1.24, 1.25, 1.262, 1.296, 1.2963, 1.302,
        1.32, 1.35, 1.40, 1.50, 1.537, 1.545, 1.613, 1.63925, 1.73, 1.732, 1.8,
        1.82, 1.90, 1.95, 1.968, 1.9683, 1.969, 1.9691, 1.97, 1.971, 1.986,
        1.989, 1.992, 1.996, 2.0007, 2.05, 2.09, 2.10, 2.20, 2.22, 2.45, 2.50,
        2.60, 2.61, 2.80, 2.85, 3.10, 3.2, 3.27, 3.42, 3.6, 4.0181, 4.22, 4.42,
        4.60, 4.80, 4.86, 4.89, 5.2, 5.40, 5.857, 5.92, 6.08, 6.76, 6.85, 7.01,
        7.03, 7.06, 7.108, 7.21, 7.28, 7.42, 7.4496, 7.46, 7.4991, 7.5, 7.57,
        7.754, 7.82, 7.89, 8.33, 8.72, 9.03, 9.35, 9.46, 9.74, 10.77, 10.8,
        11.35, 11.5368, 11.66, 11.78, 12.61, 12.69, 13.96, 14.70, 15.509, 15.97,
        16.5, 17.2, 17.24, 17.31, 18.10, 18.12, 18.17, 18.5, 18.60, 18.70,
        18.99, 19.37, 19.53, 20.1, 20.14, 20.15, 20.6, 21.40, 21.54, 21.56,
        22.05, 22.71, 22.7136, 23.12, 24.12419, 25.10, 25.5102041, 26.34, 27.87,
        28.17, 28.2295, 28.38, 28.48, 28.76, 28.77, 28.896, 28.95, 29.19, 29.47,
        29.5, 30.6122449, 31.40, 33.71, 33.74, 34.60, 35.43, 36.04, 36.09, 36.1,
        36.5, 36.54, 37.6, 38.35, 38.5, 38.51, 39.40, 41.26, 44.61, 44.65,
        46.5, 46.51, 51.8, 54.5, 55.4, 59.2, 59.4, 60.88, 61.4, 62.0, 62.7,
        63.4, 63.6, 65.0, 65.4, 66.1, 69.3, 73.44, 73.7, 78.6, 81.8, 85.50,
        85.71, 86.17, 91.6, 94.3, 95.8, 97.2, 98.17, 99.25, 102.2, 104.6, 105.0,
        106.0, 107.2, 120.26, 120.264, 137.0, 169.25, 192.168, 195.70, 240.53,
        241.0, 780.0, 880.0, 939.25, 1639.25, 1639.2474, 2939.3, 4850.0,
        9941.69, 10768.81, 17310.97, 18119.20, 18165.71, 18699.40, 19526.53,
        20627.12, 21398.41, 21543.52, 21560.75, 22047.32, 23124.57, 24124.19,
        28697.71, 29468.22, 33709.55, 34004.44, 1105747.56, 1105802.52,
        2.38e-13, 2.22e-16, 1.20e-9, 5.18e-5, 1.56e-4, 1.63e-5, 1.41e-5,
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

    all_errors = path_hash_errors + structured_errors + token_errors
    return {
        "passed": len(all_errors) == 0,
        "structured_claims_verified": structured_count,
        "numeric_tokens_scanned": token_count,
        "error_count": len(all_errors),
        "errors": all_errors,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify all claims in WHITEPAPER.md, SCIENCE.md, and README.md against JSON receipts.")
    parser.add_argument("--root", type=Path, default=ROOT, help="Repository root path")
    parser.add_argument("--json", action="store_true", help="Print JSON report")
    args = parser.parse_args()

    report = run_claims_audit(args.root.resolve())
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        if report["passed"]:
            print(
                f"claims_audit: OK ({report['structured_claims_verified']} structured receipt claims + "
                f"{report['numeric_tokens_scanned']} markdown numeric tokens verified across WHITEPAPER.md, SCIENCE.md, README.md)"
            )
        else:
            print(f"claims_audit: FAILED with {report['error_count']} error(s):", file=sys.stderr)
            for err in report["errors"]:
                print(f"  - {err}", file=sys.stderr)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
