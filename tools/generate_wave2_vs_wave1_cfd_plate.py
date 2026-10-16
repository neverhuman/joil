#!/usr/bin/env python3
"""Generate Plate 10: Side-by-Side Wave-1 vs. Wave-2 Elite OpenFOAM 14 3D CFD Field Comparison
and 5-Run G0/G1/G2 Roache GCI + Dual LRR/SSG RSM Survival Audit.

Compares:
  - Wave-1 6-Jet Helical Elite (xbabe2-gen043-isl10-ind030, runs/rust-cfd-elite-gen024-isl2-ind014)
  - Wave-2 TPMS Cylindrical Gyroid Elite (s20261103-g008-i012, runs/wave2-tpms-gyroid-g1-medium)
  - Wave-2 8-Jet Octa-Branch Manifold Elite (wave2-arch-octa-branch-helical-manifold, 5-case GCI/RSM)
  - Colman-Thew CT-35 Baseline (runs/ct35-sst-extended-000)

All side-by-side field panels enforce strictly identical color scales (vmin, vmax), spatial limits
(z in [-20, 220] mm, r in [0, 35] mm), and SI/engineering units.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.colors import Normalize
import numpy as np

ROOT = Path(__file__).resolve().parents[1]

import sys
sys.path.insert(0, str(ROOT / "tools"))
from generate_openfoam14_flowfield_plate import (  # type: ignore
    compute_cell_centers,
    extract_case_profiles,
    parse_internal_scalars,
    parse_internal_vectors,
    sha256_file,
)


def extract_meridional_rz_grid(
    case_dir: Path,
    time_str: str,
    z_min_mm: float = -20.0,
    z_max_mm: float = 220.0,
    r_max_mm: float = 35.0,
    nz: int = 48,
    nr: int = 28,
) -> dict[str, Any]:
    """Bin 3D OpenFOAM 14 cell-center fields onto a structured (z, r) meridional grid."""
    centers = compute_cell_centers(case_dir)
    u = parse_internal_vectors(case_dir / time_str / "U")
    p_kin = parse_internal_scalars(case_dir / time_str / "p")
    k_turb = parse_internal_scalars(case_dir / time_str / "k")
    n = min(len(centers), len(u), len(p_kin), len(k_turb))
    centers, u, p_kin, k_turb = centers[:n], u[:n], p_kin[:n], k_turb[:n]

    x, y, z = centers[:, 0], centers[:, 1], centers[:, 2]
    r_mm = np.hypot(x, y) * 1e3
    z_mm = z * 1e3
    r_safe = np.maximum(np.hypot(x, y), 1e-6)
    u_theta = np.abs(x * u[:, 1] - y * u[:, 0]) / r_safe
    u_z = u[:, 2]
    p_kpa = p_kin * 998.0 / 1e3

    z_edges = np.linspace(z_min_mm, z_max_mm, nz + 1)
    r_edges = np.linspace(0.0, r_max_mm, nr + 1)
    z_centers = 0.5 * (z_edges[:-1] + z_edges[1:])
    r_centers = 0.5 * (r_edges[:-1] + r_edges[1:])

    ut_grid = np.full((nr, nz), np.nan, dtype=np.float64)
    p_grid = np.full((nr, nz), np.nan, dtype=np.float64)
    uz_grid = np.full((nr, nz), np.nan, dtype=np.float64)
    r_wall_mm = np.full(nz, np.nan, dtype=np.float64)

    in_window = (z_mm >= z_min_mm) & (z_mm <= z_max_mm) & (r_mm <= r_max_mm)
    iz = np.clip(np.digitize(z_mm[in_window], z_edges) - 1, 0, nz - 1)
    ir = np.clip(np.digitize(r_mm[in_window], r_edges) - 1, 0, nr - 1)
    ut_w = u_theta[in_window]
    p_w = p_kpa[in_window]
    uz_w = u_z[in_window]
    r_w = r_mm[in_window]

    for j in range(nz):
        zm = iz == j
        if np.any(zm):
            r_wall_mm[j] = float(np.percentile(r_w[zm], 99.0))
        for i in range(nr):
            m = zm & (ir == i)
            if np.sum(m) >= 2:
                ut_grid[i, j] = float(np.percentile(ut_w[m], 90.0))
                p_grid[i, j] = float(np.mean(p_w[m]))
                uz_grid[i, j] = float(np.mean(uz_w[m]))

    return {
        "z_edges_mm": z_edges,
        "r_edges_mm": r_edges,
        "z_centers_mm": z_centers,
        "r_centers_mm": r_centers,
        "u_theta_p90_grid": ut_grid,
        "p_kpa_mean_grid": p_grid,
        "u_z_mean_grid": uz_grid,
        "r_wall_mm": r_wall_mm,
        "cell_count": int(n),
    }


def main() -> None:
    w1_case = ROOT / "runs/rust-cfd-elite-gen024-isl2-ind014"
    w2_case = ROOT / "runs/wave2-tpms-gyroid-g1-medium"
    base_case = ROOT / "runs/ct35-sst-extended-000"

    w1_dossier_path = ROOT / "docs/results/elite-3grid-gci-and-rsm-verification-v2.json"
    w2_dossier_path = ROOT / "docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json"

    w1_dossier = json.loads(w1_dossier_path.read_text(encoding="utf-8"))
    w2_dossier = json.loads(w2_dossier_path.read_text(encoding="utf-8"))

    w1_grid = extract_meridional_rz_grid(w1_case, "150")
    w2_grid = extract_meridional_rz_grid(w2_case, "150")
    w1_prof = extract_case_profiles(w1_case, "150")
    w2_prof = extract_case_profiles(w2_case, "150")
    base_prof = extract_case_profiles(base_case, "600")

    # Shared color scales across Wave-1 and Wave-2 meridional CFD maps
    ut_valid = np.concatenate([
        w1_grid["u_theta_p90_grid"][~np.isnan(w1_grid["u_theta_p90_grid"])],
        w2_grid["u_theta_p90_grid"][~np.isnan(w2_grid["u_theta_p90_grid"])],
    ])
    z_pos_mask = w1_grid["z_centers_mm"] >= 0.0
    p_ch_w1 = w1_grid["p_kpa_mean_grid"][:, z_pos_mask]
    p_ch_w2 = w2_grid["p_kpa_mean_grid"][:, z_pos_mask]
    p_valid = np.concatenate([
        p_ch_w1[~np.isnan(p_ch_w1)],
        p_ch_w2[~np.isnan(p_ch_w2)],
    ])
    ut_vmin, ut_vmax = 0.0, float(np.ceil(np.percentile(ut_valid, 98.5) * 2.0) / 2.0)
    p_vmin = float(np.floor(np.percentile(p_valid, 2.0)))
    p_vmax = float(np.ceil(np.percentile(p_valid, 99.0)))

    fig = plt.figure(figsize=(17.5, 13.5), dpi=180, facecolor="#0b1021")
    gs = fig.add_gridspec(
        3,
        2,
        height_ratios=[1.0, 1.0, 1.12],
        hspace=0.36,
        wspace=0.24,
        left=0.065,
        right=0.92,
        top=0.905,
        bottom=0.065,
    )

    tpms_d = w2_dossier["wave2_tpms_gyroid_elite_5run_dossier"]
    octa_d = w2_dossier["wave2_octa_branch_8jet_elite_5run_dossier"]
    w1_g1 = w1_dossier["grids"][1]
    tpms_g1 = tpms_d["grids"][1]
    octa_g1 = octa_d["grids"][1]

    fig.suptitle(
        "Plate 10 — Side-by-Side Wave-1 vs. Wave-2 Archived 2-Port Wall-Modulated OpenFOAM 14 CFD Fields & 5-Run Screen\n"
        f"W1 \"6-Jet\" Label ({w1_g1['cells']:,} Cells) vs. W2 \"TPMS\" Label ({tpms_g1['cells']:,} Cells) & \"8-Jet\" Label ({octa_g1['cells']:,} Cells) | v2 Oscillatory R < 0 (std GCI = null)",
        fontsize=12.8,
        fontweight="bold",
        color="#f8fafc",
        y=0.968,
    )

    def style_ax(ax: plt.Axes, title: str, xlabel: str, ylabel: str) -> None:
        ax.set_facecolor("#111827")
        ax.set_title(title, fontsize=10.6, fontweight="bold", color="#e2e8f0", pad=8)
        ax.set_xlabel(xlabel, fontsize=9.5, color="#cbd5e1")
        ax.set_ylabel(ylabel, fontsize=9.5, color="#cbd5e1")
        ax.tick_params(colors="#94a3b8", labelsize=8.5)
        ax.grid(True, linestyle="--", alpha=0.22, color="#475569")
        for spine in ax.spines.values():
            spine.set_color("#334155")

    cmap_ut = plt.get_cmap("viridis").copy()
    cmap_ut.set_bad(color="#0f172a")
    norm_ut = Normalize(vmin=ut_vmin, vmax=ut_vmax)

    # Row 1: Side-by-side Meridional U_theta(r, z) [m/s]
    ax_a1 = fig.add_subplot(gs[0, 0])
    style_ax(
        ax_a1,
        f"A1. Wave-1 6-Jet Helical Elite $U_{{\\theta,p90}}(r, z)$ [m/s] (G1: {w1_g1['cells']:,} Cells, $\\Delta p_u = {w1_g1['dp_u_pa']/1e3:.2f}$ kPa)",
        "Axial Coordinate $z$ [mm]",
        "Radial Coordinate $r$ [mm]",
    )
    im_a1 = ax_a1.pcolormesh(
        w1_grid["z_edges_mm"],
        w1_grid["r_edges_mm"],
        w1_grid["u_theta_p90_grid"],
        cmap=cmap_ut,
        norm=norm_ut,
        shading="flat",
    )
    ax_a1.plot(w1_grid["z_centers_mm"], w1_grid["r_wall_mm"], color="#38bdf8", lw=1.8, ls="--", label="Outer Wall $R(z)$")
    ax_a1.set_xlim(-20.0, 220.0)
    ax_a1.set_ylim(0.0, 35.0)
    ax_a1.legend(loc="upper right", fontsize=8, facecolor="#0f172a", edgecolor="#334155", labelcolor="#e2e8f0")

    ax_a2 = fig.add_subplot(gs[0, 1])
    style_ax(
        ax_a2,
        f"A2. Wave-2 TPMS Gyroid Elite $U_{{\\theta,p90}}(r, z)$ [m/s] (G1: {tpms_g1['cells']:,} Cells, $\\Delta p_u = {tpms_g1['dp_u_pa']/1e3:.2f}$ kPa)",
        "Axial Coordinate $z$ [mm]",
        "Radial Coordinate $r$ [mm]",
    )
    im_a2 = ax_a2.pcolormesh(
        w2_grid["z_edges_mm"],
        w2_grid["r_edges_mm"],
        w2_grid["u_theta_p90_grid"],
        cmap=cmap_ut,
        norm=norm_ut,
        shading="flat",
    )
    ax_a2.plot(w2_grid["z_centers_mm"], w2_grid["r_wall_mm"], color="#34d399", lw=1.8, ls="--", label="Outer Wall $R(z)$ + TPMS Wall-Mod Zone")
    ax_a2.axvspan(108.0, 180.0, color="#a855f7", alpha=0.14, label="Wall-Modulated Proxy Zone (C1)")
    ax_a2.set_xlim(-20.0, 220.0)
    ax_a2.set_ylim(0.0, 35.0)
    ax_a2.legend(loc="upper right", fontsize=8, facecolor="#0f172a", edgecolor="#334155", labelcolor="#e2e8f0")

    cax_ut = fig.add_axes([0.932, 0.665, 0.014, 0.24])
    cb_ut = fig.colorbar(im_a2, cax=cax_ut)
    cb_ut.set_label("Swirl Velocity $U_{\\theta,p90}$ [m/s]\n(Shared Scale)", fontsize=9, color="#e2e8f0")
    cb_ut.ax.tick_params(colors="#94a3b8", labelsize=8)

    # Row 2: Side-by-side Meridional Static Gauge Pressure p(r, z) [kPa]
    cmap_p = plt.get_cmap("coolwarm").copy()
    cmap_p.set_bad(color="#0f172a")
    norm_p = Normalize(vmin=p_vmin, vmax=p_vmax)

    ax_b1 = fig.add_subplot(gs[1, 0])
    style_ax(
        ax_b1,
        f"B1. Wave-1 6-Jet Helical Elite Static Pressure $p(r, z)$ [kPa] ($E_{{loss}} = {w1_g1['hydraulic_loss_j_per_treated_m3']/1e3:.2f}$ kJ/m$^3$)",
        "Axial Coordinate $z$ [mm]",
        "Radial Coordinate $r$ [mm]",
    )
    im_b1 = ax_b1.pcolormesh(
        w1_grid["z_edges_mm"],
        w1_grid["r_edges_mm"],
        w1_grid["p_kpa_mean_grid"],
        cmap=cmap_p,
        norm=norm_p,
        shading="flat",
    )
    ax_b1.plot(w1_grid["z_centers_mm"], w1_grid["r_wall_mm"], color="#f8fafc", lw=1.5, ls="--")
    ax_b1.set_xlim(-20.0, 220.0)
    ax_b1.set_ylim(0.0, 35.0)

    ax_b2 = fig.add_subplot(gs[1, 1])
    style_ax(
        ax_b2,
        f"B2. Wave-2 TPMS Gyroid Elite Static Pressure $p(r, z)$ [kPa] ($E_{{loss}} = {tpms_g1['hydraulic_loss_j_per_treated_m3']/1e3:.2f}$ kJ/m$^3$)",
        "Axial Coordinate $z$ [mm]",
        "Radial Coordinate $r$ [mm]",
    )
    im_b2 = ax_b2.pcolormesh(
        w2_grid["z_edges_mm"],
        w2_grid["r_edges_mm"],
        w2_grid["p_kpa_mean_grid"],
        cmap=cmap_p,
        norm=norm_p,
        shading="flat",
    )
    ax_b2.plot(w2_grid["z_centers_mm"], w2_grid["r_wall_mm"], color="#f8fafc", lw=1.5, ls="--")
    ax_b2.axvspan(108.0, 180.0, color="#a855f7", alpha=0.14)
    ax_b2.set_xlim(-20.0, 220.0)
    ax_b2.set_ylim(0.0, 35.0)

    cax_p = fig.add_axes([0.932, 0.375, 0.014, 0.24])
    cb_p = fig.colorbar(im_b2, cax=cax_p)
    cb_p.set_label("Static Pressure $p$ [kPa]\n(Shared Scale)", fontsize=9, color="#e2e8f0")
    cb_p.ax.tick_params(colors="#94a3b8", labelsize=8)

    # Row 3 Left (Panel C): Direct Radial Profile Overlay U_theta(r) and p(r) in Swirl Chamber
    ax_c = fig.add_subplot(gs[2, 0])
    style_ax(
        ax_c,
        "C. Radial Swirl $U_{\\theta,p90}(r)$ [m/s] & Static Pressure $p(r)$ [kPa] ($z \\in [2, 55]$ mm)",
        "Radial Coordinate $r = \\sqrt{x^2 + y^2}$ [mm]",
        "Swirl Velocity $U_{\\theta,p90}(r)$ [m/s]",
    )
    bc = base_prof["swirl_chamber_z20_90mm"]
    w1c = w1_prof["swirl_chamber_z20_90mm"]
    w2c = w2_prof["swirl_chamber_z20_90mm"]

    ax_c.plot(bc["r_mm"], bc["u_theta_p75"], "--o", color="#ef4444", lw=2.0, ms=5, label="CT-35 Baseline $U_\\theta(r)$ [m/s]")
    ax_c.plot(w1c["r_mm"], w1c["u_theta_p75"], "-s", color="#38bdf8", lw=2.3, ms=5.5, label="Wave-1 6-Jet Elite $U_\\theta(r)$ [m/s]")
    ax_c.plot(w2c["r_mm"], w2c["u_theta_p75"], "-D", color="#34d399", lw=2.3, ms=5.5, label="Wave-2 TPMS Gyroid Elite $U_\\theta(r)$ [m/s]")
    ax_c.set_xlim(0.0, 35.0)

    ax_c_r = ax_c.twinx()
    ax_c_r.plot(bc["r_mm"], bc["p_kpa_mean"], ":^", color="#fca5a5", lw=1.7, ms=4.5, label="CT-35 $p(r)$ [kPa]")
    ax_c_r.plot(w1c["r_mm"], w1c["p_kpa_mean"], ":s", color="#93c5fd", lw=1.8, ms=4.5, label="Wave-1 6-Jet $p(r)$ [kPa]")
    ax_c_r.plot(w2c["r_mm"], w2c["p_kpa_mean"], ":D", color="#6ee7b7", lw=1.8, ms=4.5, label="Wave-2 TPMS $p(r)$ [kPa]")
    ax_c_r.set_ylabel("Mean Static Gauge Pressure $p(r)$ [kPa]", fontsize=9, color="#cbd5e1")
    ax_c_r.tick_params(colors="#94a3b8", labelsize=8)
    ax_c.legend(loc="upper left", fontsize=7.8, facecolor="#0f172a", edgecolor="#334155", labelcolor="#e2e8f0")
    ax_c_r.legend(loc="lower right", fontsize=7.8, facecolor="#0f172a", edgecolor="#334155", labelcolor="#e2e8f0")

    # Row 3 Right (Panel D): 5-Case OpenFOAM 14 Verification Across G0, G1 SST, G2, G1 LRR, G1 SSG
    ax_d = fig.add_subplot(gs[2, 1])
    style_ax(
        ax_d,
        "D. 5-Run OpenFOAM 14 G0/G1/G2 + LRR/SSG RSM $\\Delta p_u$ [kPa] (v2 Oscillatory R < 0; std GCI = null)",
        "OpenFOAM 14 Verification Case (3 Grids + 2 Second-Moment RSM Closures)",
        "Static Underflow Pressure Drop $\\Delta p_u$ [kPa]",
    )
    case_labels = ["G0 Coarse\n(kOmegaSST)", "G1 Medium\n(kOmegaSST)", "G2 Fine\n(kOmegaSST)", "G1 LRR\n(7-Eq RSM)", "G1 SSG\n(7-Eq RSM)"]
    x_idx = np.arange(len(case_labels))
    width = 0.25

    def get_5case_dpu_kpa(dossier_obj: dict[str, Any]) -> list[float]:
        grids = dossier_obj["grids"]
        rsms = dossier_obj["rsm_turbulence_closure_crosscheck"]
        return [
            grids[0]["dp_u_pa"] / 1e3,
            grids[1]["dp_u_pa"] / 1e3,
            grids[2]["dp_u_pa"] / 1e3,
            rsms[0]["dp_u_pa"] / 1e3,
            rsms[1]["dp_u_pa"] / 1e3,
        ]

    w1_vals = get_5case_dpu_kpa(w1_dossier)
    tpms_vals = get_5case_dpu_kpa(tpms_d)
    octa_vals = get_5case_dpu_kpa(octa_d)
    base_dpu_kpa = float(w1_dossier["baseline_ct35_comparator"]["dp_u_pa"]) / 1e3

    w1_hs = float(w1_dossier["roache_gci_analysis"]["dp_u_pa"]["three_grid_half_span_pct_of_f2"])
    tpms_hs = float(tpms_d["roache_gci_analysis"]["dp_u_pa"]["three_grid_half_span_pct_of_f2"])
    octa_hs = float(octa_d["roache_gci_analysis"]["dp_u_pa"]["three_grid_half_span_pct_of_f2"])

    ax_d.axhline(base_dpu_kpa, color="#ef4444", ls="--", lw=1.8, label=f"CT-35 Structured-Hex Ref ({base_dpu_kpa:.2f} kPa; Diff Pipeline)")
    ax_d.bar(
        x_idx - width,
        w1_vals,
        width=width,
        color="#38bdf8",
        edgecolor="#0284c7",
        label=f"W1 \"6-Jet\" Shell (Oscillatory R=-0.373, Half-Span=±{w1_hs:.2f}%, std GCI=null)",
    )
    ax_d.bar(
        x_idx,
        tpms_vals,
        width=width,
        color="#34d399",
        edgecolor="#059669",
        label=f"W2 \"TPMS\" Shell (Oscillatory R=-0.379, Half-Span=±{tpms_hs:.2f}%, std GCI=null)",
    )
    ax_d.bar(
        x_idx + width,
        octa_vals,
        width=width,
        color="#a855f7",
        edgecolor="#7e22ce",
        label=f"W2 \"8-Jet\" Shell (Oscillatory R=-0.369, Half-Span=±{octa_hs:.2f}%, std GCI=null)",
    )
    ax_d.set_xticks(x_idx)
    ax_d.set_xticklabels(case_labels, fontsize=8.2)
    ax_d.set_ylim(0.0, max(base_dpu_kpa * 1.18, max(w1_vals + tpms_vals + octa_vals) * 1.22))
    ax_d.legend(loc="upper right", fontsize=7.1, facecolor="#0f172a", edgecolor="#334155", labelcolor="#e2e8f0")

    fig_path = ROOT / "docs/figures/wave2-vs-wave1-openfoam14-cfd-comparison.png"
    fig.savefig(fig_path, dpi=180, facecolor=fig.get_facecolor())
    plt.close(fig)

    fig_sha = sha256_file(fig_path)
    manifest = {
        "schema_version": 2,
        "kind": "publication_figure_manifest_not_physical_validation",
        "figure_path": "docs/figures/wave2-vs-wave1-openfoam14-cfd-comparison.png",
        "figure_sha256": fig_sha,
        "generator_script": "tools/generate_wave2_vs_wave1_cfd_plate.py",
        "matched_color_scales": {
            "u_theta_p90_m_s": [ut_vmin, ut_vmax],
            "static_pressure_kpa": [p_vmin, p_vmax],
            "meridional_z_limits_mm": [-20.0, 220.0],
            "meridional_r_limits_mm": [0.0, 35.0],
        },
        "source_receipts": {
            "wave1_gci_rsm_dossier": "docs/results/elite-3grid-gci-and-rsm-verification-v2.json",
            "wave1_gci_rsm_dossier_sha256": sha256_file(w1_dossier_path),
            "wave2_gci_rsm_f2_dossier": "docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json",
            "wave2_gci_rsm_f2_dossier_sha256": sha256_file(w2_dossier_path),
            "wave1_g1_integrals_sha256": sha256_file(w1_case / "hydraulic-integrals.json"),
            "wave2_tpms_g1_integrals_sha256": sha256_file(w2_case / "hydraulic-integrals.json"),
            "baseline_ct35_integrals_sha256": sha256_file(base_case / "hydraulic-integrals-t600.json"),
        },
    }
    manifest_path = ROOT / "docs/figures/wave2-vs-wave1-openfoam14-cfd-comparison.png.manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {fig_path} (SHA-256: {fig_sha}, {fig_path.stat().st_size / 1024:.1f} KiB)")


if __name__ == "__main__":
    main()
