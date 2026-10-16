#!/usr/bin/env python3
"""Extract 3D OpenFOAM 14 cell-center fields (r, z, U_theta, U_z, p, k) and render vortex physics comparison plate."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
from typing import Any

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[1]


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse_foam_points(points_path: Path) -> np.ndarray:
    txt = points_path.read_text(errors="replace")
    m = re.search(r"\n(\d+)\s*\(\s*(.*?)\s*\)\s*(?:;|\n\s*//)", txt, re.S)
    if not m:
        raise ValueError(f"Could not parse points from {points_path}")
    pts = re.findall(r"\(\s*([0-9.eE+-]+)\s+([0-9.eE+-]+)\s+([0-9.eE+-]+)\s*\)", m.group(2))
    return np.array([[float(a), float(b), float(c)] for a, b, c in pts], dtype=np.float64)


def parse_foam_faces_first_vertex(faces_path: Path) -> np.ndarray:
    txt = faces_path.read_text(errors="replace")
    m = re.search(r"\n(\d+)\s*\(\s*(.*?)\s*\)\s*(?:;|\n\s*//)", txt, re.S)
    if not m:
        raise ValueError(f"Could not parse faces from {faces_path}")
    first_v = re.findall(r"\d+\(\s*(\d+)", m.group(2))
    return np.array([int(v) for v in first_v], dtype=np.int64)


def parse_foam_owner(owner_path: Path) -> tuple[int, np.ndarray]:
    txt = owner_path.read_text(errors="replace")
    nc_m = re.search(r"nCells:\s*(\d+)", txt)
    m = re.search(r"\n(\d+)\s*\(\s*(.*?)\s*\)\s*(?:;|\n\s*//)", txt, re.S)
    if not m:
        raise ValueError(f"Could not parse owner from {owner_path}")
    owners = np.fromstring(m.group(2), sep=" ", dtype=np.int64)
    n_cells = int(nc_m.group(1)) if nc_m else int(owners.max() + 1)
    return n_cells, owners


def compute_cell_centers(case_dir: Path) -> np.ndarray:
    cache = case_dir / "cell-centers-summary.npz"
    if cache.exists():
        return np.load(cache)["centers"]
    pm = case_dir / "constant/polyMesh"
    pts = parse_foam_points(pm / "points")
    f_v0 = parse_foam_faces_first_vertex(pm / "faces")
    n_cells, owners = parse_foam_owner(pm / "owner")
    n_faces = min(len(f_v0), len(owners))
    f_pts = pts[f_v0[:n_faces]]
    owners = owners[:n_faces]
    sums = np.zeros((n_cells, 3), dtype=np.float64)
    counts = np.zeros(n_cells, dtype=np.float64)
    np.add.at(sums, owners, f_pts)
    np.add.at(counts, owners, 1.0)
    centers = sums / np.maximum(counts[:, None], 1.0)
    np.savez_compressed(cache, centers=centers)
    return centers


def parse_internal_vectors(u_path: Path) -> np.ndarray:
    txt = u_path.read_text(errors="replace")
    m = re.search(r"internalField\s+nonuniform\s+List<vector>\s+(\d+)\s*\(\s*(.*?)\s*\)\s*;", txt, re.S)
    if not m:
        raise ValueError(f"Could not parse vectors from {u_path}")
    vecs = re.findall(r"\(\s*([0-9.eE+-]+)\s+([0-9.eE+-]+)\s+([0-9.eE+-]+)\s*\)", m.group(2))
    return np.array([[float(a), float(b), float(c)] for a, b, c in vecs], dtype=np.float64)


def parse_internal_scalars(s_path: Path) -> np.ndarray:
    txt = s_path.read_text(errors="replace")
    m = re.search(r"internalField\s+nonuniform\s+List<scalar>\s+(\d+)\s*\(\s*(.*?)\s*\)\s*;", txt, re.S)
    if not m:
        raise ValueError(f"Could not parse scalars from {s_path}")
    return np.fromstring(m.group(2), sep=" ", dtype=np.float64)


def extract_case_profiles(case_dir: Path, time_str: str) -> dict[str, Any]:
    centers = compute_cell_centers(case_dir)
    u = parse_internal_vectors(case_dir / time_str / "U")
    p_kin = parse_internal_scalars(case_dir / time_str / "p")
    k_turb = parse_internal_scalars(case_dir / time_str / "k")
    n = min(len(centers), len(u), len(p_kin), len(k_turb))
    centers, u, p_kin, k_turb = centers[:n], u[:n], p_kin[:n], k_turb[:n]

    x, y, z = centers[:, 0], centers[:, 1], centers[:, 2]
    r = np.hypot(x, y)
    r_safe = np.maximum(r, 1e-6)
    u_theta = np.abs(x * u[:, 1] - y * u[:, 0]) / r_safe
    u_z = u[:, 2]
    p_kpa = p_kin * 998.0 / 1e3
    accel_g = (u_theta**2 / r_safe) / 9.80665

    def radial_bins(z_min: float, z_max: float, r_max_mm: float, n_bins: int = 16) -> dict[str, list[float]]:
        mask = (z >= z_min) & (z <= z_max) & (r * 1e3 <= r_max_mm)
        edges = np.linspace(1.0, r_max_mm, n_bins + 1)
        r_mid, ut_m, uz_m, p_m, g_m = [], [], [], [], []
        for i in range(n_bins):
            bm = mask & (r * 1e3 >= edges[i]) & (r * 1e3 < edges[i + 1])
            if np.sum(bm) >= 5:
                r_mid.append(float(0.5 * (edges[i] + edges[i + 1])))
                ut_m.append(float(np.percentile(u_theta[bm], 90)))
                uz_m.append(float(np.mean(u_z[bm])))
                p_m.append(float(np.mean(p_kpa[bm])))
                g_m.append(float(np.percentile(accel_g[bm], 90)))
        return {"r_mm": r_mid, "u_theta_p75": ut_m, "u_z_mean": uz_m, "p_kpa_mean": p_m, "accel_g_p75": g_m}

    # Axial evolution along primary swirl head and reducing cone z in [-20, 220] mm
    z_edges = np.linspace(-0.020, 0.220, 25)
    z_mid, ut_ax, k_ax, core_uz_ax = [], [], [], []
    for i in range(len(z_edges) - 1):
        zm = (z >= z_edges[i]) & (z < z_edges[i + 1])
        if np.sum(zm) >= 10:
            z_mid.append(float(0.5 * (z_edges[i] + z_edges[i + 1]) * 1e3))
            ut_ax.append(float(np.percentile(u_theta[zm], 95)))
            k_ax.append(float(np.mean(k_turb[zm])))
            core_m = zm & (r * 1e3 <= 4.0)
            core_uz_ax.append(float(np.min(u_z[core_m])) if np.sum(core_m) >= 2 else float(np.min(u_z[zm])))

    return {
        "cell_count": int(n),
        "swirl_chamber_z20_90mm": radial_bins(0.002, 0.055, 34.5, 16),
        "taper_cone_z150_350mm": radial_bins(0.055, 0.115, 28.0, 14),
        "axial_profile": {
            "z_mm": z_mid,
            "u_theta_p90_m_s": ut_ax,
            "tke_mean_m2_s2": k_ax,
            "core_min_uz_m_s": core_uz_ax,
        },
        "peak_centrifugal_g_p99": float(np.percentile(accel_g[(z >= 0.0) & (z <= 0.12)], 99)),
        "mean_swirl_velocity_chamber_m_s": float(np.mean(u_theta[(z >= 0.0) & (z <= 0.10)])),
    }


def main() -> None:
    elite_dir = ROOT / "runs/rust-cfd-elite-gen024-isl2-ind014"
    base_dir = ROOT / "runs/ct35-sst-extended-000"
    elite_prof = extract_case_profiles(elite_dir, "150")
    base_prof = extract_case_profiles(base_dir, "600")

    fig = plt.figure(figsize=(16, 11), dpi=220, facecolor="#0b1021")
    gs = fig.add_gridspec(2, 2, hspace=0.32, wspace=0.25, left=0.07, right=0.96, top=0.90, bottom=0.08)

    fig.suptitle(
        "3D OpenFOAM 14 Swirl-Head & Primary Reducing-Cone Vortex Physics Comparison\n"
        "Promoted 6-Jet Helical Elite (xbabe2-gen043-isl10-ind030, 305,522 Cells) vs. Colman–Thew CT-35 Baseline (334,768 Cells)",
        fontsize=14, fontweight="bold", color="#f8fafc", y=0.965
    )

    def style_ax(ax: plt.Axes, title: str, xlabel: str, ylabel: str) -> None:
        ax.set_facecolor("#111827")
        ax.set_title(title, fontsize=11.5, fontweight="bold", color="#e2e8f0", pad=10)
        ax.set_xlabel(xlabel, fontsize=10, color="#cbd5e1")
        ax.set_ylabel(ylabel, fontsize=10, color="#cbd5e1")
        ax.tick_params(colors="#94a3b8", labelsize=9)
        ax.grid(True, linestyle="--", alpha=0.25, color="#475569")
        for spine in ax.spines.values():
            spine.set_color("#334155")

    # Panel A: Radial Swirl Velocity U_theta(r) in Swirl Head & Primary Cone
    ax1 = fig.add_subplot(gs[0, 0])
    style_ax(
        ax1,
        "A. Radial Swirl Velocity $U_{\\theta,p90}(r)$ in Entry Swirl Chamber & Primary S-Cone",
        "Radial Coordinate $r = \\sqrt{x^2 + y^2}$ [mm]",
        "Tangential Swirl Velocity $U_{\\theta,p90}$ [m/s]"
    )
    eb_ch = elite_prof["swirl_chamber_z20_90mm"]
    bb_ch = base_prof["swirl_chamber_z20_90mm"]
    eb_tp = elite_prof["taper_cone_z150_350mm"]
    bb_tp = base_prof["taper_cone_z150_350mm"]

    ax1.plot(bb_ch["r_mm"], bb_ch["u_theta_p75"], "--o", color="#ef4444", linewidth=2.0, markersize=5.5, label="CT-35 Swirl Chamber ($z \\in [2, 55]$ mm)")
    ax1.plot(eb_ch["r_mm"], eb_ch["u_theta_p75"], "-s", color="#38bdf8", linewidth=2.4, markersize=6, label="6-Jet Elite Swirl Chamber ($z \\in [2, 55]$ mm)")
    ax1.plot(bb_tp["r_mm"], bb_tp["u_theta_p75"], ":^", color="#f97316", linewidth=2.0, markersize=5.5, label="CT-35 Primary Cone ($z \\in [55, 115]$ mm)")
    ax1.plot(eb_tp["r_mm"], eb_tp["u_theta_p75"], "-D", color="#34d399", linewidth=2.4, markersize=6, label="6-Jet Elite Inflected S-Cone ($z \\in [55, 115]$ mm)")
    ax1.legend(loc="upper left", fontsize=8.3, facecolor="#0f172a", edgecolor="#334155", labelcolor="#e2e8f0")

    # Panel B: Radial Static Pressure Profile p(r) & Core Depression
    ax2 = fig.add_subplot(gs[0, 1])
    style_ax(
        ax2,
        "B. Radial Static Pressure Profile $p(r)$ & Wall-to-Core Pressure Saving",
        "Radial Coordinate $r = \\sqrt{x^2 + y^2}$ [mm]",
        "Area/Cell-Averaged Static Gauge Pressure $p$ [kPa]"
    )
    ax2.plot(bb_ch["r_mm"], bb_ch["p_kpa_mean"], "--o", color="#ef4444", linewidth=2.2, markersize=6, label="CT-35 Swirl Chamber $p(r)$ (Wall: 29.47 kPa)")
    ax2.plot(eb_ch["r_mm"], eb_ch["p_kpa_mean"], "-s", color="#38bdf8", linewidth=2.5, markersize=6.5, label="6-Jet Elite Swirl Chamber $p(r)$ (Wall: 18.70 kPa, -36.5%)")
    ax2.plot(bb_tp["r_mm"], bb_tp["p_kpa_mean"], ":^", color="#f97316", linewidth=2.0, markersize=5.5, label="CT-35 Primary Cone $p(r)$")
    ax2.plot(eb_tp["r_mm"], eb_tp["p_kpa_mean"], "-D", color="#34d399", linewidth=2.3, markersize=6, label="6-Jet Elite Inflected S-Cone $p(r)$")
    ax2.legend(loc="lower right", fontsize=8.3, facecolor="#0f172a", edgecolor="#334155", labelcolor="#e2e8f0")

    # Panel C: Centrifugal Acceleration Field U_theta^2 / (r g) Driving Oil Migration
    ax3 = fig.add_subplot(gs[1, 0])
    style_ax(
        ax3,
        "C. Radial Centrifugal Acceleration $a_{c,p90}(r) = U_\\theta^2 / (r g)$ [g-Units]",
        "Radial Coordinate $r = \\sqrt{x^2 + y^2}$ [mm]",
        "Centripetal Buoyancy Acceleration $a_c / g$ [-]"
    )
    ax3.plot(bb_ch["r_mm"], bb_ch["accel_g_p75"], "--o", color="#ef4444", linewidth=2.1, markersize=6, label="CT-35 Swirl Chamber $a_c(r)$")
    ax3.plot(eb_ch["r_mm"], eb_ch["accel_g_p75"], "-s", color="#38bdf8", linewidth=2.5, markersize=6.5, label="6-Jet Elite Swirl Chamber $a_c(r)$")
    ax3.plot(bb_tp["r_mm"], bb_tp["accel_g_p75"], ":^", color="#f97316", linewidth=1.9, markersize=5.5, label="CT-35 Primary Cone $a_c(r)$")
    ax3.plot(eb_tp["r_mm"], eb_tp["accel_g_p75"], "-D", color="#34d399", linewidth=2.2, markersize=6, label="6-Jet Elite Inflected S-Cone $a_c(r)$")
    ax3.legend(loc="upper left", fontsize=8.3, facecolor="#0f172a", edgecolor="#334155", labelcolor="#e2e8f0")

    # Panel D: Axial Evolution Across Swirl Head & Vortex Finder (z in [-20, 220] mm)
    ax4 = fig.add_subplot(gs[1, 1])
    style_ax(
        ax4,
        "D. Swirl-Head Axial Evolution of Peak Swirl $U_{\\theta,p95}(z)$ & TKE $k(z)$",
        "Axial Coordinate $z$ Across Swirl Head & Primary Cone [mm]",
        "95th-Percentile Swirl Velocity $U_{\\theta,p95}(z)$ [m/s]"
    )
    ea = elite_prof["axial_profile"]
    ba = base_prof["axial_profile"]
    ax4.plot(ba["z_mm"], ba["u_theta_p90_m_s"], "--o", color="#ef4444", linewidth=2.1, markersize=5.5, label="CT-35 $U_{\\theta,p95}(z)$ [m/s]")
    ax4.plot(ea["z_mm"], ea["u_theta_p90_m_s"], "-s", color="#38bdf8", linewidth=2.4, markersize=6, label="6-Jet Elite $U_{\\theta,p95}(z)$ [m/s]")

    ax4_r = ax4.twinx()
    ax4_r.plot(ba["z_mm"], ba["tke_mean_m2_s2"], ":^", color="#fbbf24", linewidth=1.9, markersize=5, label="CT-35 Mean TKE $k(z)$ [m$^2$/s$^2$]")
    ax4_r.plot(ea["z_mm"], ea["tke_mean_m2_s2"], "-D", color="#a855f7", linewidth=2.1, markersize=5.5, label="6-Jet Elite Mean TKE $k(z)$ [m$^2$/s$^2$]")
    ax4_r.set_ylabel("Section-Mean Turbulent Kinetic Energy $k(z)$ [m$^2$/s$^2$]", fontsize=9.5, color="#c084fc")
    ax4_r.tick_params(colors="#c084fc", labelsize=8.5)
    ax4.legend(loc="upper right", fontsize=8.2, facecolor="#0f172a", edgecolor="#334155", labelcolor="#e2e8f0")
    ax4_r.legend(loc="center right", fontsize=8.2, facecolor="#0f172a", edgecolor="#334155", labelcolor="#e2e8f0")

    fig_path = ROOT / "docs/figures/openfoam14-3d-vortex-physics-comparison.png"
    fig.savefig(fig_path, dpi=220, facecolor=fig.get_facecolor())
    plt.close(fig)
    fig_sha = sha256_file(fig_path)

    manifest_path = ROOT / "docs/figures/openfoam14-3d-vortex-physics-comparison.png.manifest.json"
    manifest_path.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "kind": "publication_figure_manifest_not_physical_validation",
                "figure_path": "docs/figures/openfoam14-3d-vortex-physics-comparison.png",
                "figure_sha256": fig_sha,
                "figure_bytes": fig_path.stat().st_size,
                "generator_script": "tools/generate_openfoam14_flowfield_plate.py",
                "generator_sha256": sha256_file(Path(__file__).resolve()),
                "elite_case_dir": "runs/rust-cfd-elite-gen024-isl2-ind014",
                "baseline_case_dir": "runs/ct35-sst-extended-000",
                "extracted_summary": {
                    "elite": {
                        "cell_count": elite_prof["cell_count"],
                        "peak_centrifugal_g_p99": elite_prof["peak_centrifugal_g_p99"],
                        "mean_swirl_velocity_chamber_m_s": elite_prof["mean_swirl_velocity_chamber_m_s"],
                    },
                    "baseline": {
                        "cell_count": base_prof["cell_count"],
                        "peak_centrifugal_g_p99": base_prof["peak_centrifugal_g_p99"],
                        "mean_swirl_velocity_chamber_m_s": base_prof["mean_swirl_velocity_chamber_m_s"],
                    },
                },
            },
            indent=2,
        )
        + "\n"
    )
    print(f"Wrote {fig_path} (SHA-256: {fig_sha})")


if __name__ == "__main__":
    main()
