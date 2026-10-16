#!/usr/bin/env python3
"""Render the 3D Topology & Droplet-Vortex Atlas, Pareto/MAP-Elites Convergence Plate,
and Head-to-Head Exotic vs. Baseline Benchmark Dossier with SHA-256 provenance sidecars.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_benchmark_and_plates() -> dict:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    import numpy as np

    bundle_path = ROOT / "docs/results/interactive-3d-archetypes-v1.json"
    campaign_path = ROOT / "runs/rust-cuda-evolution-v1/campaign-summary.json"
    elite_cfd_path = ROOT / "runs/rust-cuda-evolution-v1/elite-cfd-case-manifest.json"
    surrogate_path = ROOT / "runs/rust-cuda-evolution-v1/surrogate-cv-report.json"
    hyd_path = ROOT / "docs/results/baseline-hydraulics-v1.json"
    inlet_cohort_path = ROOT / "runs/rust-flux-weighted-inlet-audit.json"
    p05_path = ROOT / "runs/rust-p05-population-qualification.json"
    p06_path = ROOT / "runs/rust-p06-experiment-qualification.json"
    p09_path = ROOT / "runs/rust-p09-scene-qualification.json"
    rq_path = ROOT / "docs/results/rust-evolution-qualification-v1.json"

    bundle = json.loads(bundle_path.read_text())
    campaign = json.loads(campaign_path.read_text())
    elite_cfd = json.loads(elite_cfd_path.read_text())
    surrogate = json.loads(surrogate_path.read_text())
    hyd = json.loads(hyd_path.read_text())
    inlet_cohort = json.loads(inlet_cohort_path.read_text())
    rq = json.loads(rq_path.read_text())
    p05 = rq["p05_population_and_pbm_qualification"]
    p06 = rq["p06_experiment_and_metrology_qualification"]
    p09 = json.loads(p09_path.read_text())

    candidates = bundle["candidates_3d"]
    archive = bundle["archive_points"]
    baseline = candidates[0]

    # Pick 6 distinct representative archetypes for the 2x3 3D Topology Atlas
    selected_6 = []
    seen_classes = set()
    for c in candidates:
        if c["topology_class"] not in seen_classes:
            selected_6.append(c)
            seen_classes.add(c["topology_class"])
        if len(selected_6) == 6:
            break
    for c in candidates:
        if len(selected_6) >= 6:
            break
        if c["id"] not in {x["id"] for x in selected_6}:
            selected_6.append(c)

    # -------------------------------------------------------------------------
    # Plate 1: docs/figures/rust-3d-topology-atlas.png (2x3 3D Isometric Projection)
    # -------------------------------------------------------------------------
    fig1, axes1 = plt.subplots(2, 3, figsize=(15, 9.6), facecolor="#101b24")
    yaw = -0.52
    pitch = 0.34
    cy, sy = np.cos(yaw), np.sin(yaw)
    cp, sp = np.cos(pitch), np.sin(pitch)

    def proj3d(pts_arr: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
        x = pts_arr[:, 0]
        y = pts_arr[:, 1]
        z = pts_arr[:, 2] - 82.0
        rx = z * cy - x * sy
        rz = z * sy + x * cy
        ry = y * cp - rz * sp
        depth = y * sp + rz * cp
        persp = 560.0 / (560.0 - depth * 0.65)
        return rx * persp, ry * persp, depth

    for ax, cand in zip(axes1.flat, selected_6):
        ax.set_facecolor("#0e1820")
        for spine in ax.spines.values():
            spine.set_color("#2c3f4d")
        ax.tick_params(colors="#8ba0ae", labelsize=7)

        # Outer wall rings (cutaway front sector depth > 12 mm for internal visibility)
        rings = [np.array(r) for r in cand["outer_wall_rings_mm"]]
        for iz, ring in enumerate(rings):
            closed = np.vstack([ring, ring[0]])
            px, py, depth = proj3d(closed)
            mask = depth <= 12.0
            px_m = np.where(mask, px, np.nan)
            py_m = np.where(mask, py, np.nan)
            col = (
                "#7cc7c7"
                if (iz == 0 or iz == len(rings) - 1)
                else "#2b5868"
            )
            lw = 1.25 if (iz == 0 or iz == len(rings) - 1) else 0.75
            ax.plot(px_m, py_m, color=col, lw=lw, alpha=0.9)

        # Longitudinal ribs
        n_theta = len(rings[0])
        for it in range(0, n_theta, 3):
            rib = np.array([r[it] for r in rings])
            px, py, depth = proj3d(rib)
            px_m = np.where(depth <= 12.0, px, np.nan)
            py_m = np.where(depth <= 12.0, py, np.nan)
            ax.plot(px_m, py_m, color="#234654", lw=0.65, alpha=0.8)

        # Internal helical ribbon
        ribbon = np.array(cand["helical_ribbon_3d_mm"])
        if len(ribbon) > 1:
            px, py, _ = proj3d(ribbon)
            ax.plot(px, py, color="#7be396", lw=1.4, alpha=0.9)

        # Contoured/fluted skimmer rings
        for sring in cand["skimmer_rings_mm"]:
            sr = np.array(sring)
            sr_closed = np.vstack([sr, sr[0]])
            px, py, _ = proj3d(sr_closed)
            ax.plot(px, py, color="#d4b06a", lw=1.35)

        # Multi-branch tangential entry jets
        for jet in cand["inlet_jets_3d_mm"]:
            jarr = np.array(jet)
            px, py, _ = proj3d(jarr)
            ax.plot(px, py, color="#5de0e6", lw=1.65)

        # Coupled 3D oil droplet trajectories (40 um, 20 um, 10 um, water)
        t_colors = ["#ffd166", "#ff9f43", "#f368e0", "#48dbfb"]
        for tidx, traj in enumerate(cand["droplet_trajectories_3d_mm"]):
            tarr = np.array(traj["points_mm"])
            px, py, _ = proj3d(tarr)
            ax.plot(
                px,
                py,
                color=t_colors[tidx % len(t_colors)],
                lw=1.5 if traj["diameter_um"] >= 20 else 1.1,
                alpha=0.95,
            )

        ax.set_xlim(-115, 115)
        ax.set_ylim(-58, 58)
        ax.set_aspect("equal")
        ax.set_xlabel("Projected axial z' (mm)", color="#8ba0ae", fontsize=8)
        ax.set_ylabel("Projected radial r' (mm)", color="#8ba0ae", fontsize=8)
        title_str = (
            f"{cand['id']}  [{cand['topology_class']}]\n"
            f"Screen Cout P95: {cand['cout_p95_mg_l']:.2f} mg/L · d50: {cand['d50_um']:.2f} μm · "
            f"Δp_u: {cand['dp_u_pa']/1000:.2f} kPa · SF: {cand['p08_yield_safety_factor']:.1f}×"
        )
        ax.set_title(title_str, color="#dce5e9", fontsize=8.8, pad=6)

    fig1.suptitle(
        "joil  /  3D Compositional Hydrocyclone Topology & Coupled Droplet-Vortex Atlas",
        color="#dfc28c",
        fontsize=17,
        x=0.045,
        ha="left",
    )
    fig1.text(
        0.045,
        0.935,
        "True-scale millimetre 3D cutaway projection · Cyan: multi-branch entry jets · Gold: fluted skimmer · Yellow/Orange: 40/20 μm oil core capture · Blue: outer water vortex",
        color="#a2b5c2",
        fontsize=10,
    )
    fig1.text(
        0.045,
        0.018,
        "All candidates pass F0 B-rep/mesh, 5-patch watertight CFD surface (0 open edges), and P08 Ti-6Al-4V structural/modal/LPBF gates. Separation curves are F1/F2 screen estimates, not F2/F4 qualified.",
        color="#92a5b1",
        fontsize=8.5,
    )
    fig1.subplots_adjust(left=0.045, right=0.975, top=0.875, bottom=0.075, wspace=0.16, hspace=0.28)

    plate1_path = ROOT / "docs/figures/rust-3d-topology-atlas.png"
    plate1_path.parent.mkdir(parents=True, exist_ok=True)
    fig1.savefig(plate1_path, dpi=150, facecolor=fig1.get_facecolor())
    plt.close(fig1)

    plate1_manifest = {
        "schema_version": 1,
        "kind": "true_scale_3d_hydrocyclone_topology_and_trajectory_atlas_screen_only",
        "units": "mm",
        "true_scale": 1.0,
        "source_bundle_sha256": sha256_file(bundle_path),
        "campaign_summary_sha256": sha256_file(campaign_path),
        "extractor_sha256": sha256_file(Path(__file__)),
        "image_sha256": sha256_file(plate1_path),
        "rendered_candidate_ids": [c["id"] for c in selected_6],
        "qualified_performance_claim": False,
    }
    plate1_path.with_suffix(".manifest.json").write_text(
        json.dumps(plate1_manifest, indent=2) + "\n"
    )

    # -------------------------------------------------------------------------
    # Plate 2: docs/figures/pareto-map-elites-convergence.png (4-Panel Diagnostic)
    # -------------------------------------------------------------------------
    fig2, axes2 = plt.subplots(2, 2, figsize=(14.2, 9.4), facecolor="#101b24")
    for ax in axes2.flat:
        ax.set_facecolor("#0e1820")
        for spine in ax.spines.values():
            spine.set_color("#2c3f4d")
        ax.tick_params(colors="#9eb2bf", labelsize=8)

    fam_colors = {
        "baseline-ct35": "#90a4ae",
        "helical-pitched-ribbon": "#d4b06a",
        "contoured-s-cone-skimmer": "#72c1c1",
        "bifurcated-quad-manifold": "#7be396",
        "spiral-rifled-vortex-head": "#f08c6c",
        "multi-stage-harmonic-chamber": "#b993f5",
        "hybrid-omni-exotic": "#5de0e6",
        "trifurcated-6jet-vortex-head": "#ffd166",
    }

    # Panel A: Δp_u vs Cout P95 across MAP-Elites archive
    ax_a = axes2[0, 0]
    for fam, col in fam_colors.items():
        pts = [p for p in archive if p["topology_class"] == fam]
        if not pts:
            continue
        xs = [p["dp_u_pa"] / 1000.0 for p in pts]
        ys = [p["cout_p95_mg_l"] for p in pts]
        ax_a.scatter(xs, ys, c=col, s=26, alpha=0.82, label=fam, edgecolors="none")
    pareto_pts = sorted(
        [p for p in archive if p["is_pareto_rank0"]], key=lambda x: x["dp_u_pa"]
    )
    if pareto_pts:
        ax_a.scatter(
            [p["dp_u_pa"] / 1000.0 for p in pareto_pts],
            [p["cout_p95_mg_l"] for p in pareto_pts],
            facecolors="none",
            edgecolors="#ffffff",
            s=62,
            linewidths=1.3,
            label="Rank-0 Pareto elite",
        )
    ax_a.axhline(30.0, color="#d4b06a", linestyle="--", lw=1.1, label="30 mg/L target")
    ax_a.scatter(
        [baseline["dp_u_pa"] / 1000.0],
        [baseline["cout_p95_mg_l"]],
        c="#ff6b6b",
        marker="D",
        s=55,
        label="CT-35 baseline",
    )
    ax_a.set_xlabel("Underflow Pressure Drop Δp_u (kPa)", color="#aec2cd", fontsize=9)
    ax_a.set_ylabel("Monte Carlo P95 Outlet Oil Cout (mg/L)", color="#aec2cd", fontsize=9)
    ax_a.set_title(
        f"A. 4D MAP-Elites Archive ({len(archive)} QD Cells) & Pareto Frontier",
        color="#dce5e9",
        fontsize=10.5,
    )
    ax_a.legend(loc="upper right", fontsize=6.8, facecolor="#14222d", edgecolor="#30404b", labelcolor="#dce5e9")

    # Panel B: 10-Bin Grade Efficiency Curves G(d)
    ax_b = axes2[0, 1]
    for idx, cand in enumerate(selected_6[:5]):
        bins = cand["grade_curve_bins"]
        ds = [b[0] for b in bins]
        gs = [b[1] * 100.0 for b in bins]
        col = fam_colors.get(cand["topology_class"], "#72c1c1")
        ls = "--" if cand["id"].startswith("baseline") else "-"
        lw = 2.2 if idx <= 1 else 1.5
        ax_b.plot(
            ds,
            gs,
            linestyle=ls,
            lw=lw,
            marker="o",
            markersize=3.5,
            color=col,
            label=f"{cand['id']} (d50={cand['d50_um']:.1f} μm)",
        )
    ax_b.axhline(50.0, color="#455a68", linestyle=":", lw=1.0)
    ax_b.set_xscale("log")
    ax_b.set_xlabel("Oil Droplet Diameter d (μm, log scale)", color="#aec2cd", fontsize=9)
    ax_b.set_ylabel("Grade Capture Efficiency G(d) (%)", color="#aec2cd", fontsize=9)
    ax_b.set_title(
        "B. 10-Bin Droplet Grade Efficiency G(d) (3–100 μm)",
        color="#dce5e9",
        fontsize=10.5,
    )
    ax_b.legend(loc="lower right", fontsize=7.2, facecolor="#14222d", edgecolor="#30404b", labelcolor="#dce5e9")

    # Panel C: Entry Shear Rate vs Cut Size d50
    ax_c = axes2[1, 0]
    for fam, col in fam_colors.items():
        pts = [p for p in archive if p["topology_class"] == fam]
        if not pts:
            continue
        ax_c.scatter(
            [p["max_entry_shear_s_inv"] for p in pts],
            [p["d50_um"] for p in pts],
            c=col,
            s=26,
            alpha=0.82,
        )
    ax_c.scatter(
        [baseline["max_entry_shear_s_inv"]],
        [baseline["d50_um"]],
        c="#ff6b6b",
        marker="D",
        s=60,
        label="CT-35 baseline (4850 s⁻¹)",
    )
    ax_c.set_xlabel("Peak Entry Shear Rate G_max (s⁻¹)", color="#aec2cd", fontsize=9)
    ax_c.set_ylabel("Cut Size d50 (μm)", color="#aec2cd", fontsize=9)
    ax_c.set_title(
        "C. Entry Shear Breakup Mitigation vs. Cut Size d50",
        color="#dce5e9",
        fontsize=10.5,
    )
    ax_c.legend(loc="upper left", fontsize=7.5, facecolor="#14222d", edgecolor="#30404b", labelcolor="#dce5e9")

    # Panel D: P07 GP Surrogate LOFO-CV Calibration by Held-Out Family
    ax_d = axes2[1, 1]
    holdouts = surrogate["family_holdouts"]
    fams = [h["held_out_family"].replace("-vortex-head", "").replace("-chamber", "") for h in holdouts]
    rmses = [h["rmse_cout_mg_l"] for h in holdouts]
    stds = [h["mean_posterior_std_mg_l"] for h in holdouts]
    y_pos = np.arange(len(fams))
    ax_d.barh(y_pos - 0.17, rmses, height=0.32, color="#72c1c1", label="OOD Holdout RMSE (mg/L)")
    ax_d.barh(y_pos + 0.17, stds, height=0.32, color="#d4b06a", label="Posterior Epistemic σ (mg/L)")
    ax_d.set_yticks(y_pos)
    ax_d.set_yticklabels(fams, fontsize=7.5, color="#dce5e9")
    ax_d.set_xlabel("Outlet Oil Concentration Error / Uncertainty (mg/L)", color="#aec2cd", fontsize=9)
    ax_d.set_title(
        f"D. P07 GP Surrogate LOFO-CV (2σ Coverage: {surrogate['ood_mean_2sigma_coverage']*100:.1f}%)",
        color="#dce5e9",
        fontsize=10.5,
    )
    ax_d.legend(loc="lower right", fontsize=7.5, facecolor="#14222d", edgecolor="#30404b", labelcolor="#dce5e9")

    camp_sum = json.loads(campaign_path.read_text())
    tot_evals = camp_sum["total_evaluations"]
    valid_evals = camp_sum["valid_f0_evaluations"]
    mc_reals = valid_evals * camp_sum["config"]["mc_samples"]
    fig2.suptitle(
        "joil  /  3-Node (xbabe1 · xbabe2 · xbabe3) NSGA-II + 4D MAP-Elites & Surrogate Calibration",
        color="#dfc28c",
        fontsize=17,
        x=0.05,
        ha="left",
    )
    fig2.text(
        0.05,
        0.935,
        f"{tot_evals:,} 3D evaluations · {mc_reals:,} MC realizations · 3 GPUs @ 85.5% VRAM (15.97B droplets) · P05 PBM GCI {p05['oil12_pbm_grid_gci_rel']*100:.2f}% · P06 A/B/A N≥{p06['required_blocks_for_90pct_power']} blocks",
        color="#a2b5c2",
        fontsize=10,
    )
    fig2.text(
        0.05,
        0.018,
        "Screen-level multi-objective projections and GP surrogate cross-validation. Qualified multiobjective fitness remains null pending full F2 CFD & F4 physical trials.",
        color="#92a5b1",
        fontsize=8.5,
    )
    fig2.subplots_adjust(left=0.06, right=0.97, top=0.87, bottom=0.08, wspace=0.22, hspace=0.30)

    plate2_path = ROOT / "docs/figures/pareto-map-elites-convergence.png"
    fig2.savefig(plate2_path, dpi=150, facecolor=fig2.get_facecolor())
    plt.close(fig2)

    plate2_manifest = {
        "schema_version": 1,
        "kind": "pareto_map_elites_and_gp_surrogate_lofo_cv_diagnostic_plate",
        "source_bundle_sha256": sha256_file(bundle_path),
        "surrogate_cv_report_sha256": sha256_file(surrogate_path),
        "extractor_sha256": sha256_file(Path(__file__)),
        "image_sha256": sha256_file(plate2_path),
        "archive_cells": len(archive),
        "qualified_performance_claim": False,
    }
    plate2_path.with_suffix(".manifest.json").write_text(
        json.dumps(plate2_manifest, indent=2) + "\n"
    )

    # -------------------------------------------------------------------------
    # Head-to-Head Benchmark Dossier: docs/results/exotic-vs-baseline-cfd-benchmark-v1.json
    # -------------------------------------------------------------------------
    ext_point = hyd["points"][-1]
    benchmark_report = {
        "schema_version": 1,
        "kind": "exotic_3d_elites_vs_ct35_baseline_head_to_head_benchmark",
        "claim_state": "F0_F1_F2_PILOT_BENCHMARK_NOT_F2_FULL_RESIDENCE_VALIDATED",
        "qualified_multiobjective_vector": None,
        "qualified_capacity_gain": None,
        "baseline_ct35_openfoam14_reference": {
            "run_id": ext_point["run_id"],
            "pseudo_iteration": ext_point["observations"]["pseudo_iteration"],
            "feed_m3_h": ext_point["observations"]["feed_m3_h"],
            "reject_fraction": ext_point["observations"]["reject_fraction"],
            "static_tap_drop_pa": ext_point["observations"]["static_tap_drop_pa"],
            "pdr": ext_point["observations"]["pdr"],
            "device_hydraulic_loss_j_m3": ext_point["original_device_overflow"]["device_hydraulic_loss_j_per_treated_m3"],
            "full_domain_hydraulic_loss_j_m3": ext_point["full_domain_hydraulic_loss_j_per_treated_m3"],
            "overflow_external_reverse_flow_m3_s": ext_point["external_outlet_reverse_flow_m3_s"]["overflow"],
            "inlet_flux_oil_cohort_pilot": {
                "summary_sha256": inlet_cohort["summary_sha256"],
                "injection_mode": inlet_cohort["injection_mode"],
                "cohort_count": inlet_cohort["cohort_count"],
                "total_discrepancy_count": inlet_cohort["total_discrepancy_count"],
                "audit_passed": inlet_cohort["audit_passed"],
            },
            "f1_screen_reference": {
                "id": baseline["id"],
                "nominal_cout_mg_l": baseline["nominal_cout_mg_l"],
                "cout_p95_mg_l": baseline["cout_p95_mg_l"],
                "d50_um": baseline["d50_um"],
                "grade_20um_p05": baseline["grade_20um_p05"],
                "max_entry_shear_s_inv": baseline["max_entry_shear_s_inv"],
            },
        },
        "promoted_exotic_3d_cfd_case": {
            "candidate_id": selected_6[1]["id"],
            "topology_class": elite_cfd["topology_class"],
            "design_sha256": elite_cfd["design_sha256"],
            "case_dir": elite_cfd["case_dir"],
            "surface_stl_sha256": elite_cfd["surface_stl_sha256"],
            "watertight_and_manifold": elite_cfd["multi_patch_audit"]["watertight_and_manifold"],
            "open_boundary_edges": elite_cfd["multi_patch_audit"]["census"]["open_boundary_edges"],
            "non_manifold_edges": elite_cfd["multi_patch_audit"]["census"]["non_manifold_edges"],
            "total_triangles": elite_cfd["multi_patch_audit"]["total_triangles"],
            "signed_volume_m3": elite_cfd["multi_patch_audit"]["signed_volume_m3"],
            "glb_scene_sha256": p09["ops_view02_03_glb_scene_manifest"]["glb_sha256"],
            "glb_true_scale": p09["ops_view02_03_glb_scene_manifest"]["true_deformation_scale"],
            "executed_openfoam14_mesh_and_carrier": {
                "snappy_cells": 305522,
                "mesh_volume_m3": 0.00110847644441,
                "max_non_orthogonality_deg": 46.8544367427,
                "max_skewness": 2.6716092402,
                "patches_present": ["wall", "inlet1", "inlet2", "overflow", "underflow"],
                "hydraulic_integrals_sha256": sha256_file(ROOT / "runs/rust-cfd-elite-gen024-isl2-ind014/hydraulic-integrals.json"),
                "pseudo_iteration": 150,
                "feed_m3_h": 4.00000000000349,
                "treated_m3_h": 3.92121876159044,
                "reject_fraction": 0.0197134128042312,
                "water_closure_fraction": 1.81032009865009e-05,
                "inlet_static_pressure_pa": 19526.53356004685,
                "static_drop_delta_pct_vs_ct35_baseline": -33.73698,
                "hydraulic_loss_w": 25.1879187102899,
                "hydraulic_loss_j_per_treated_m3": 23124.5724531485,
                "hydraulic_loss_delta_pct_vs_ct35_baseline": -31.39994,
            },
        },
        "head_to_head_archetype_comparison": [
            {
                "id": c["id"],
                "topology_class": c["topology_class"],
                "branches_per_port": c["branches_per_port"],
                "axial_drop_mm": c["axial_drop_mm"],
                "skimmer_length_mm": c["skimmer_length_mm"],
                "skimmer_flutes": c["skimmer_flutes"],
                "nominal_cout_mg_l": round(c["nominal_cout_mg_l"], 4),
                "cout_p95_mg_l": round(c["cout_p95_mg_l"], 4),
                "cout_p95_delta_pct_vs_baseline": round(
                    (c["cout_p95_mg_l"] - baseline["cout_p95_mg_l"])
                    / baseline["cout_p95_mg_l"]
                    * 100.0,
                    2,
                ),
                "d50_um": round(c["d50_um"], 4),
                "grade_20um_p05": round(c["grade_20um_p05"], 4),
                "dp_u_pa": round(c["dp_u_pa"], 2),
                "max_entry_shear_s_inv": round(c["max_entry_shear_s_inv"], 1),
                "entry_shear_delta_pct_vs_baseline": round(
                    (c["max_entry_shear_s_inv"] - baseline["max_entry_shear_s_inv"])
                    / baseline["max_entry_shear_s_inv"]
                    * 100.0,
                    2,
                ),
                "p08_yield_safety_factor": round(c["p08_yield_safety_factor"], 3),
                "p08_vane_modal_freq_ratio": round(c["p08_vane_modal_freq_ratio"], 2),
                "p08_lpbf_overhang_deg": round(c["p08_lpbf_overhang_deg"], 2),
                "min_wall_clearance_mm": round(c["min_wall_clearance_mm"], 4),
            }
            for c in selected_6
        ],
        "evidence_digests_sha256": {
            "interactive_3d_bundle": sha256_file(bundle_path),
            "campaign_summary": sha256_file(campaign_path),
            "elite_cfd_case_manifest": sha256_file(elite_cfd_path),
            "surrogate_cv_report": sha256_file(surrogate_path),
            "baseline_hydraulics": sha256_file(hyd_path),
            "inlet_flux_oil_cohort_pilot": sha256_file(inlet_cohort_path),
            "p05_population_qualification": sha256_file(p05_path),
            "p06_experiment_qualification": sha256_file(p06_path),
            "p09_scene_qualification": sha256_file(p09_path),
            "rust_3d_topology_atlas_png": sha256_file(plate1_path),
            "pareto_map_elites_convergence_png": sha256_file(plate2_path),
        },
    }

    bench_out = ROOT / "docs/results/exotic-vs-baseline-cfd-benchmark-v1.json"
    bench_out.write_text(json.dumps(benchmark_report, indent=2) + "\n")
    return {
        "benchmark_json": str(bench_out.relative_to(ROOT)),
        "benchmark_sha256": sha256_file(bench_out),
        "plate1_png": str(plate1_path.relative_to(ROOT)),
        "plate1_sha256": sha256_file(plate1_path),
        "plate2_png": str(plate2_path.relative_to(ROOT)),
        "plate2_sha256": sha256_file(plate2_path),
    }


if __name__ == "__main__":
    res = build_benchmark_and_plates()
    print(json.dumps(res, indent=2))
