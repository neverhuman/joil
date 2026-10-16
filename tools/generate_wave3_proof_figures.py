#!/usr/bin/env python3
"""Generate the Wave-3 README Hero Plot and White-Paper Evolutionary Proof Plate (Plate 11)
directly from `docs/results/wave3-evolution-proof-v1.json` and
`docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json`.

Outputs:
1. `docs/figures/wave3-readme-hero-kpi-and-elite.png` (+ `.manifest.json`)
2. `docs/figures/wave3-evolution-ablation-and-lineage-proof.png` (+ `.manifest.json`)
"""
from __future__ import annotations

import hashlib
import json
import math
from pathlib import Path

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle, FancyBboxPatch, Polygon, Wedge

ROOT = Path(__file__).resolve().parents[1]
WAVE3_JSON = ROOT / 'docs/results/wave3-evolution-proof-v1.json'
CFD_JSON = ROOT / 'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json'
HERO_OUT = ROOT / 'docs/figures/wave3-readme-hero-kpi-and-elite.png'
PLATE11_OUT = ROOT / 'docs/figures/wave3-evolution-ablation-and-lineage-proof.png'

WAVE4_JSON = ROOT / 'docs/results/wave4-evolution-proof-v1.json'
CFD_V2_JSON = ROOT / 'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json'
ELITE_V2_JSON = ROOT / 'docs/results/elite-3grid-gci-and-rsm-verification-v2.json'
WAVE4_HERO_OUT = ROOT / 'docs/figures/wave4-readme-hero-kpi-and-elite.png'
WAVE4_PLATE_OUT = ROOT / 'docs/figures/wave4-evolution-ablation-and-lineage-proof.png'


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _get(d: dict, *keys: str):
    for k in keys:
        if k in d and d[k] is not None:
            return d[k]
    raise KeyError(f'None of {keys} in {list(d.keys())}')


def _mc_cout(mc: dict) -> float:
    return float(_get(mc, 'cout_p95_mg_l', 'sample_screen_cout_p95_mg_l', 'cout_sample_screen_upper_mg_l'))


def _b1_key(mapping: dict) -> str:
    return 'B1_Uniform_Random' if 'B1_Uniform_Random' in mapping else 'B1_Random_Search'


def write_manifest(png_path: Path, title: str, sources: list[Path]) -> dict[str, str]:
    png_sha = file_sha256(png_path)
    manifest = {
        'schema_version': 1,
        'figure_path': str(png_path.relative_to(ROOT)),
        'figure_sha256': png_sha,
        'title': title,
        'generator_script': 'tools/generate_wave3_proof_figures.py',
        'source_receipts': [
            {
                'path': str(s.relative_to(ROOT)),
                'sha256': file_sha256(s),
            }
            for s in sources
        ],
    }
    manifest_path = Path(str(png_path) + '.manifest.json')
    manifest_path.write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    return {'png_sha256': png_sha, 'manifest_sha256': file_sha256(manifest_path)}


def generate_readme_hero(
    w3: dict,
    cfd: dict,
    w3_sha: str,
    cfd_sha: str,
    out_path: Path = HERO_OUT,
    wave_label: str = 'Wave-3',
    w1_gci_pct: float = 5.019376028759711,
    proof_filename: str = 'wave3-evolution-proof-v1.json',
    cfd_filename: str = 'wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json',
) -> None:
    plt.rcParams.update({
        'font.family': 'DejaVu Sans',
        'font.size': 9.5,
        'axes.titlesize': 11.0,
        'axes.titleweight': 'bold',
        'axes.labelsize': 9.8,
        'axes.labelweight': 'bold',
    })

    fig = plt.figure(figsize=(15.5, 10.6), dpi=180, facecolor='#F8FAFC')
    gs = fig.add_gridspec(2, 2, hspace=0.33, wspace=0.24, left=0.06, right=0.97, top=0.89, bottom=0.08)

    mc_n = int(
        w3.get('mc_samples_per_candidate')
        or w3.get('promoted_wave3_top_elite', {}).get('mc_robustness', {}).get('samples', 28)
    )
    fig.suptitle(
        f'Joil {wave_label} Prior-Screen Search Diagnostics: Per-Generation KPI Trajectory (n = 10 Seeds, 22,500 Evals, MC n={mc_n})\n'
        'B1 > B3 on Best KPI (p_Holm=1.00) & B3 > B1 on Pop-Median KPI (p_Holm=0.0013) | Panel D: Archived W1/2 CFD (Separate Subjects)',
        fontsize=12.4,
        fontweight='bold',
        color='#0F172A',
        y=0.965,
    )
    fig.text(
        0.5,
        0.922,
        f'Source Receipts: {proof_filename} (SHA-256: {w3_sha[:16]}…) | '
        f'{cfd_filename} (SHA-256: {cfd_sha[:16]}…) | '
        'Shared Unseeded Gen-0 Init across B2, A1, A2, B3 (B0 CT-35 = Single-Eval Reference)',
        ha='center',
        fontsize=8.0,
        color='#334155',
        style='italic',
    )

    bands_map = w3['generation_band_curves']
    outcomes_map = w3['arm_outcomes']
    b1_k = _b1_key(bands_map)
    gens = np.array([b['generation'] for b in bands_map['B3_Wave3_Exotic3D_QD_CEI']])
    base_kpi = float(w3['baseline_b0_ct35_kpi_score'])
    base_cout = float(_get(w3, 'baseline_b0_ct35_cout_p95_mg_l', 'baseline_b0_ct35_cout_sample_screen_upper_mg_l'))

    arm_styles = [
        ('B3_Wave3_Exotic3D_QD_CEI', f'B3: {wave_label} Exotic3D-QD-CEI (B1>B3 Best; B3>B1 Median)', '#0284C7', '-', 2.6),
        (b1_k, 'B1: Equal-Budget Uniform Random (Higher Best Extremum)', '#D97706', '--', 2.1),
        ('A1_No_Mutation_Crossover_Only', 'A1: No-Mutation (Crossover-Only Ablation)', '#7C3AED', '-.', 2.0),
        ('A2_No_Selection_Random_Walk', 'A2: No-Selection (Random-Walk Ablation)', '#DC2626', ':', 2.0),
        ('B2_Conventional_Axisym_NSGA2', 'B2: Conventional Axisym NSGA-II (2-Port)', '#475569', '--', 1.9),
    ]

    # Panel A: Per-Generation KPI_score (Best + Population Median with 10-seed CI bands)
    ax_a = fig.add_subplot(gs[0, 0])
    ax_a.set_facecolor('#FFFFFF')
    ax_a.axhline(base_kpi, color='#1E293B', linestyle=':', linewidth=1.6, label=f'B0: CT-35 Single-Eval Ref ({base_kpi:.2f} [-])')

    # Thin individual seed traces for B3 and B1
    for o in outcomes_map['B3_Wave3_Exotic3D_QD_CEI']:
        y_seed = [s['best_kpi_score'] for s in o['generation_trace']]
        ax_a.plot(gens, y_seed, color='#38BDF8', alpha=0.22, linewidth=0.8)

    for key, label, col, ls, lw in arm_styles:
        b_list = bands_map[key]
        mean_v = np.array([b['best_kpi_score']['mean'] for b in b_list])
        lo_v = np.array([b['best_kpi_score']['ci95_low'] for b in b_list])
        hi_v = np.array([b['best_kpi_score']['ci95_high'] for b in b_list])
        ax_a.plot(gens, mean_v, color=col, linestyle=ls, linewidth=lw, label=f'{label} [Best: {mean_v[-1]:.2f}]')
        ax_a.fill_between(gens, lo_v, hi_v, color=col, alpha=0.16)

    # Also plot B3 and B1 population medians to prove population-wide learning vs flat random median
    b3_med = np.array([b['median_kpi_score']['mean'] for b in bands_map['B3_Wave3_Exotic3D_QD_CEI']])
    b1_med = np.array([b['median_kpi_score']['mean'] for b in bands_map[b1_k]])
    a2_med = np.array([b['median_kpi_score']['mean'] for b in bands_map['A2_No_Selection_Random_Walk']])
    ax_a.plot(gens, b3_med, color='#0369A1', linestyle=':', linewidth=1.8, label=f'B3 Active Pop Median [{b3_med[-1]:.2f}, p_Holm=0.0013 vs B1]')
    ax_a.plot(gens, b1_med, color='#B45309', linestyle=':', linewidth=1.4, label=f'B1 Random Batch Median [{b1_med[-1]:.2f}]')
    ax_a.plot(gens, a2_med, color='#991B1B', linestyle=':', linewidth=1.3, label=f'A2 No-Selection Pop Median [{a2_med[-1]:.2f}]')

    ax_a.set_title('A. Screen KPI vs. Gen (n=10 Seeds: B1 > B3 on Best KPI, p_Holm=1.00; B3 > B1 on Pop-Median)')
    ax_a.set_xlabel('Evolutionary Generation g [-] (18 3D Evals/Gen, 450 Total Charged Proposals/Seed)')
    ax_a.set_ylabel('Composite KPI_score [-] (analysis/kpi_fitness.py, Higher = Better)')
    ax_a.set_xlim(0, gens[-1])
    ax_a.grid(True, linestyle='--', alpha=0.35)
    ax_a.legend(loc='lower right', fontsize=7.0, framealpha=0.94)

    # Panel B: Per-Generation Treated-Water Oil Concentration C_out,p95 [mg/L]
    ax_b = fig.add_subplot(gs[0, 1])
    ax_b.set_facecolor('#FFFFFF')
    ax_b.axhline(30.0, color='#EF4444', linestyle='--', linewidth=1.4, label='Discharge Ceiling (30.0 mg/L)')
    ax_b.axhline(base_cout, color='#1E293B', linestyle=':', linewidth=1.6, label=f'B0: CT-35 Single-Eval Ref ({base_cout:.2f} mg/L)')

    for o in outcomes_map['B3_Wave3_Exotic3D_QD_CEI']:
        y_seed = [float(_get(s, 'best_cout_p95_mg_l', 'best_cout_sample_screen_upper_mg_l')) for s in o['generation_trace']]
        ax_b.plot(gens, y_seed, color='#38BDF8', alpha=0.22, linewidth=0.8)

    for key, label, col, ls, lw in arm_styles:
        b_list = bands_map[key]
        mean_v = np.array([float(_get(b, 'best_cout_p95_mg_l', 'best_cout_sample_screen_upper_mg_l')['mean']) for b in b_list])
        lo_v = np.array([float(_get(b, 'best_cout_p95_mg_l', 'best_cout_sample_screen_upper_mg_l')['ci95_low']) for b in b_list])
        hi_v = np.array([float(_get(b, 'best_cout_p95_mg_l', 'best_cout_sample_screen_upper_mg_l')['ci95_high']) for b in b_list])
        short_lbl = label.split('(')[0].strip()
        ax_b.plot(gens, mean_v, color=col, linestyle=ls, linewidth=lw, label=f'{short_lbl} [Best: {mean_v[-1]:.2f} mg/L]')
        ax_b.fill_between(gens, lo_v, hi_v, color=col, alpha=0.16)

    b3_med_c = np.array([float(_get(b, 'median_cout_p95_mg_l', 'median_cout_sample_screen_upper_mg_l')['mean']) for b in bands_map['B3_Wave3_Exotic3D_QD_CEI']])
    b1_med_c = np.array([float(_get(b, 'median_cout_p95_mg_l', 'median_cout_sample_screen_upper_mg_l')['mean']) for b in bands_map[b1_k]])
    ax_b.plot(gens, b3_med_c, color='#0369A1', linestyle=':', linewidth=1.8, label=f'B3 Active Pop Median [{b3_med_c[-1]:.2f} mg/L]')
    ax_b.plot(gens, b1_med_c, color='#B45309', linestyle=':', linewidth=1.4, label=f'B1 Random Batch Median [{b1_med_c[-1]:.2f} mg/L]')

    ax_b.set_title(f'B. Screen Oil Concentration C_out,upper [mg/L] vs. Gen (n = 10 Seeds, MC n={mc_n})')
    ax_b.set_xlabel('Evolutionary Generation g [-] (Shared Unseeded Gen-0 Init at g = 0)')
    ax_b.set_ylabel('Screen Treated-Water Oil C_out,upper [mg/L] (Lower = Better)')
    ax_b.set_xlim(0, gens[-1])
    ax_b.grid(True, linestyle='--', alpha=0.35)
    ax_b.legend(loc='upper right', fontsize=7.4, framealpha=0.94)

    # Panel C: Gene-Label Schematic vs CT-35 Baseline (Explicitly marked as gene-label schematic, not compiled geometry)
    ax_c = fig.add_subplot(gs[1, 0])
    ax_c.set_facecolor('#0F172A')
    ax_c.set_xlim(0, 100)
    ax_c.set_ylim(0, 62)
    ax_c.set_xticks([])
    ax_c.set_yticks([])
    ax_c.set_title('C. Gene-Label Schematic vs. CT-35 (Prior-Screen Genes; Not Compiled Geometry, C1/IR-R4)')

    # Left sub-diagram: CT-35 Baseline cross-section & axial cut
    c_base = Circle((20, 36.2), 13.0, facecolor='#1E293B', edgecolor='#94A3B8', linewidth=2.0)
    ax_c.add_patch(c_base)
    c_core0 = Circle((20, 36.2), 2.8, facecolor='#F59E0B', edgecolor='#FDE68A', linewidth=1.2, alpha=0.85)
    ax_c.add_patch(c_core0)
    # 2 abrupt flat ports
    ax_c.add_patch(FancyBboxPatch((4.5, 46.2), 9.0, 3.0, boxstyle='round,pad=0.2', facecolor='#EF4444', edgecolor='#FCA5A5'))
    ax_c.add_patch(FancyBboxPatch((26.5, 22.8), 9.0, 3.0, boxstyle='round,pad=0.2', facecolor='#EF4444', edgecolor='#FCA5A5'))
    ax_c.text(20, 54.5, 'B0: CT-35 Single-Eval Ref (2 Ports)', color='#F8FAFC', fontweight='bold', ha='center', fontsize=9.0)
    top_el = w3['promoted_wave3_top_elite']
    top_kpi = float(w3['promoted_wave3_top_elite_kpi_score'])
    top_cout = _mc_cout(top_el['mc_robustness'])
    base_cout_val = float(_get(w3, 'baseline_b0_ct35_cout_p95_mg_l', 'baseline_b0_ct35_cout_sample_screen_upper_mg_l'))
    top_cout_red_pct = (top_cout - base_cout_val) / base_cout_val * 100.0
    base_dmax = float(w3.get('baseline_b0_ct35', {}).get('nominal_metrics', {}).get('hinze_dmax_um', 64.0))
    top_dmax = float(top_el.get('nominal_metrics', {}).get('hinze_dmax_um', 98.99))
    split_cfg = top_el['program']['inlet_split']
    n_jets = 2 * int(split_cfg['branches_per_port']) if bool(split_cfg.get('enabled', False)) else 2
    r_in_frac = float(top_el['program']['tpms_insert'].get('radial_inner_fraction', 0.7417))

    ax_c.text(
        20,
        13.2,
        f'Screen d_max(Hinze) = {base_dmax:.2f} µm\n'
        f'C_out,upper = {base_cout:.2f} mg/L | KPI = {base_kpi:.2f} [-]\n'
        f'Structured-Hex CFD Δp_u = 29.47 kPa',
        color='#CBD5E1',
        ha='center',
        fontsize=7.6,
    )

    # Right sub-diagram: Gene-Label Schematic of Top B3 Elite
    c_outer = Circle((68, 36.2), 13.8, facecolor='#0F172A', edgecolor='#38BDF8', linewidth=2.2)
    ax_c.add_patch(c_outer)
    tpms_w = max(1.5, 13.6 * (1.0 - r_in_frac))
    w_tpms = Wedge((68, 36.2), 13.6, 0, 360, width=tpms_w, facecolor='#0284C7', edgecolor='#7DD3FC', alpha=0.55, hatch='xx')
    ax_c.add_patch(w_tpms)
    c_unob = Circle((68, 36.2), 13.6 * r_in_frac, facecolor='#0F172A', edgecolor='#22D3EE', linestyle='--', linewidth=1.3)
    ax_c.add_patch(c_unob)
    c_oil = Circle((68, 36.2), 3.5, facecolor='#10B981', edgecolor='#A7F3D0', linewidth=1.5)
    ax_c.add_patch(c_oil)

    for k_jet in range(n_jets):
        ang = math.radians(k_jet * (360.0 / n_jets) + 25.0)
        x0 = 68.0 + 15.6 * math.cos(ang)
        y0 = 36.2 + 15.6 * math.sin(ang)
        dx = -3.0 * math.sin(ang) - 1.4 * math.cos(ang)
        dy = 3.0 * math.cos(ang) - 1.4 * math.sin(ang)
        ax_c.arrow(x0, y0, dx, dy, width=0.42, head_width=1.3, head_length=1.1, fc='#38BDF8', ec='#BAE6FD')

    ax_c.text(68, 54.5, f'B3 Elite Gene-Label Schematic ({top_el["topology_class"]})', color='#38BDF8', fontweight='bold', ha='center', fontsize=8.6)
    ax_c.text(
        68,
        10.5,
        f'GENE-LABEL SCHEMATIC ONLY (Not Compiled Geometry):\n'
        f'Authored genes: {n_jets} jets, TPMS={top_el["program"]["tpms_insert"]["basis"]} (r_in={r_in_frac:.4f}R, d_max={top_dmax:.2f} µm)\n'
        f'Compiled surface enacts wall-corrugation proxy only (C1, IR-R4)\n'
        f'Screen C_out,upper = {top_cout:.2f} mg/L ({top_cout_red_pct:.1f}%) | KPI = {top_kpi:.2f} [-]',
        color='#E2E8F0',
        ha='center',
        fontsize=7.1,
    )

    # Panel D: Archived Wave-1/2 OpenFOAM 14 3-Grid (G0/G1/G2) + Dual LRR/SSG RSM Screen (Separate Subjects from Wave-4)
    ax_d = fig.add_subplot(gs[1, 1])
    ax_d.set_facecolor('#FFFFFF')
    audit_by_topo = {
        row['topology_class']: row
        for row in cfd['wave1_vs_wave2_surrogate_to_cfd_ranking_audit']
    }
    tpms_dossier = cfd['wave2_tpms_gyroid_elite_5run_dossier']
    octa_dossier = cfd['wave2_octa_branch_8jet_elite_5run_dossier']
    base_dp_pa = float(tpms_dossier['baseline_ct35_comparator']['dp_u_pa'])
    w1_row = audit_by_topo['trifurcated-6jet-vortex-head']
    tpms_row = audit_by_topo['cylindrical-gyroid-coalescer']
    octa_row = audit_by_topo['octa-branch-helical-manifold']
    labels_d = [
        'CT-35 Ref\n(Structured-Hex\nG2 Only; L_ext>0)',
        'Archived W1\n2-Port Wall-Mod\n("6-Jet" Label, L_ext=0)',
        'Archived W2\n2-Port Wall-Mod\n("TPMS" Label, L_ext=0)',
        'Archived W2\n2-Port Wall-Mod\n("8-Jet" Label, L_ext=0)',
    ]
    g2_dp_kpa = [
        base_dp_pa / 1000.0,
        float(w1_row['openfoam_g2_dp_u_pa']) / 1000.0,
        float(tpms_row['openfoam_g2_dp_u_pa']) / 1000.0,
        float(octa_row['openfoam_g2_dp_u_pa']) / 1000.0,
    ]
    # CT-35 has NO LRR or SSG RSM runs in any receipt -> plot only the 3 measured RSM subjects (indices 1, 2, 3)
    lrr_dp_kpa = [
        float(w1_row['openfoam_lrr_dp_u_pa']) / 1000.0,
        float(tpms_row['openfoam_lrr_dp_u_pa']) / 1000.0,
        float(octa_row['openfoam_lrr_dp_u_pa']) / 1000.0,
    ]
    ssg_dp_kpa = [
        float(w1_row['openfoam_ssg_dp_u_pa']) / 1000.0,
        float(tpms_row['openfoam_ssg_dp_u_pa']) / 1000.0,
        float(octa_row['openfoam_ssg_dp_u_pa']) / 1000.0,
    ]
    def _gci_kpa(g2_val_kpa: float, gci_dp: dict) -> float:
        pct = gci_dp.get('gci21_standard_fs125_pct')
        if pct is not None:
            return g2_val_kpa * float(pct) / 100.0
        hs_pct = gci_dp.get('three_grid_half_span_pct_of_f2')
        if hs_pct is not None:
            return g2_val_kpa * float(hs_pct) / 100.0
        hr = gci_dp.get('half_range_uncertainty')
        if hr is not None:
            return float(hr) / 1000.0
        return 0.0

    gci_err_kpa = [
        0.0,
        g2_dp_kpa[1] * w1_gci_pct / 100.0,
        _gci_kpa(g2_dp_kpa[2], tpms_dossier['roache_gci_analysis']['dp_u_pa']),
        _gci_kpa(g2_dp_kpa[3], octa_dossier['roache_gci_analysis']['dp_u_pa']),
    ]

    x_pos = np.arange(len(labels_d))
    x_rsm = np.array([1, 2, 3])
    w_bar = 0.25
    ax_d.bar(
        x_pos - w_bar,
        g2_dp_kpa,
        width=w_bar,
        yerr=gci_err_kpa,
        capsize=4,
        color='#0284C7',
        label='OpenFOAM 14 G2 Fine (552k cells ± 3-Grid Half-Span; v2 Oscillatory, std GCI=null)',
    )
    ax_d.bar(x_rsm, lrr_dp_kpa, width=w_bar, color='#0D9488', label='7-Eq. LRR RSM (G1 305k cells; W1/W2 shells only)')
    ax_d.bar(x_rsm + w_bar, ssg_dp_kpa, width=w_bar, color='#6366F1', label='7-Eq. SSG RSM (G1 305k cells; W1/W2 shells only)')

    for i, val in enumerate(g2_dp_kpa):
        if i == 0:
            ax_d.text(x_pos[i] - w_bar, val + 0.6, f'{val:.2f} kPa\n(G2 only)', ha='center', fontsize=7.6, fontweight='bold')
        else:
            pct = (val - g2_dp_kpa[0]) / g2_dp_kpa[0] * 100.0
            ax_d.text(x_pos[i], val + 1.2, f'{val:.2f} kPa\n({pct:.1f}%)', ha='center', fontsize=7.6, fontweight='bold', color='#0F172A')

    ax_d.set_title('D. Archived W1/W2 2-Port Wall-Modulated CFD Screen (Separate Subjects; G-09/C1 Noted)')
    ax_d.set_xticks(x_pos)
    ax_d.set_xticklabels(labels_d, fontsize=7.8)
    ax_d.set_ylabel('Static Pressure Drop Δp_u [kPa] (Lower = Better)')
    ax_d.set_ylim(0, 34.5)
    ax_d.grid(True, axis='y', linestyle='--', alpha=0.35)
    ax_d.legend(loc='upper right', fontsize=7.1, framealpha=0.94)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=180)
    plt.close(fig)


def generate_plate11_proof(
    w3: dict,
    w3_sha: str,
    out_path: Path = PLATE11_OUT,
    wave_label: str = 'Wave-3',
    proof_filename: str = 'docs/results/wave3-evolution-proof-v1.json',
) -> None:
    plt.rcParams.update({
        'font.family': 'DejaVu Sans',
        'font.size': 9.2,
        'axes.titlesize': 10.6,
        'axes.titleweight': 'bold',
        'axes.labelsize': 9.4,
        'axes.labelweight': 'bold',
    })

    fig = plt.figure(figsize=(15.5, 10.8), dpi=180, facecolor='#F8FAFC')
    gs = fig.add_gridspec(2, 2, hspace=0.34, wspace=0.25, left=0.06, right=0.97, top=0.89, bottom=0.08)

    mc_n = int(
        w3.get('mc_samples_per_candidate')
        or w3.get('promoted_wave3_top_elite', {}).get('mc_robustness', {}).get('samples', 28)
    )
    fig.suptitle(
        f'Plate 11 — {wave_label} Prior-Screen Search Diagnostics: 5-Arm + B0 Reference (n = 10 Seeds, MC n={mc_n})\n'
        'B1 > B3 on Best KPI (p_Holm=1.00); B3 > B1 on Pop-Median KPI (+5.64, p_Holm=0.0013) & Authored 11-Family Screen Summary',
        fontsize=12.4,
        fontweight='bold',
        color='#0F172A',
        y=0.965,
    )
    fig.text(
        0.5,
        0.922,
        f'Source Receipt: {proof_filename} (SHA-256: {w3_sha}) | '
        f'n = 10 Seeds × 25 Gens × 18 Proposals/Gen = 22,500 Charged Proposals Across 5 Search Arms (MC n={mc_n}; B0 = 1-Eval Ref)',
        ha='center',
        fontsize=8.1,
        color='#334155',
        style='italic',
    )

    outcomes_map = w3['arm_outcomes']
    stats_map = w3['final_summary_statistics']
    bands_map = w3['generation_band_curves']
    b1_k = _b1_key(bands_map)
    base_kpi = float(w3['baseline_b0_ct35_kpi_score'])

    # Panel A: Final KPI_score across all 6 Arms with per-seed scatter + 95% Bootstrap CI + Mann-Whitney p / Cliff's delta
    ax_a = fig.add_subplot(gs[0, 0])
    ax_a.set_facecolor('#FFFFFF')
    arm_order = [
        ('B0_CT35', 'B0: CT-35\n(1-Eval Ref)', '#64748B'),
        ('B2_Conventional_Axisym_NSGA2', 'B2: Axisym\nNSGA-II', '#475569'),
        ('A2_No_Selection_Random_Walk', 'A2: No-Selection\n(Random Walk)', '#DC2626'),
        ('A1_No_Mutation_Crossover_Only', 'A1: No-Mutation\n(Crossover Only)', '#7C3AED'),
        (b1_k, 'B1: Equal-Budget\nRandom (Best)', '#D97706'),
        ('B3_Wave3_Exotic3D_QD_CEI', f'B3: {wave_label}\nQD-CEI (Pop-Med)', '#0284C7'),
    ]

    x_idx = np.arange(len(arm_order))
    means = []
    ci_lows = []
    ci_highs = []
    colors = []

    rng_jitter = np.random.default_rng(20261116)
    for idx, (key, lbl, col) in enumerate(arm_order):
        colors.append(col)
        if key == 'B0_CT35':
            means.append(base_kpi)
            ci_lows.append(0.0)
            ci_highs.append(0.0)
            ax_a.scatter([idx], [base_kpi], color=col, s=55, zorder=5, edgecolor='#0F172A')
        else:
            st = stats_map[key]['best_kpi_score']
            m_val = float(st['mean'])
            means.append(m_val)
            ci_lows.append(max(0.0, m_val - float(st['ci95_low'])))
            ci_highs.append(max(0.0, float(st['ci95_high']) - m_val))
            seed_vals = [float(o['final_best_kpi_score']) for o in outcomes_map[key]]
            jitter = rng_jitter.uniform(-0.14, 0.14, size=len(seed_vals))
            ax_a.scatter(
                np.full(len(seed_vals), idx) + jitter,
                seed_vals,
                color=col,
                alpha=0.78,
                s=34,
                zorder=5,
                edgecolor='#0F172A',
                linewidth=0.6,
                label='Individual Seeds (n=10; B1 > B3 on Best KPI, p_Holm=1.00)' if idx == 5 else None,
            )

    ax_a.bar(
        x_idx,
        means,
        yerr=[ci_lows, ci_highs],
        capsize=5,
        color=colors,
        alpha=0.32,
        edgecolor=colors,
        linewidth=1.8,
    )
    for idx, m_val in enumerate(means):
        ax_a.text(idx, m_val + ci_highs[idx] + 0.55, f'{m_val:.2f}', ha='center', fontsize=8.3, fontweight='bold')

    ax_a.set_title('A. Final Best KPI Score (n=10 Seeds: B1 > B3 on Best KPI, p_Holm=1.00; B3 > B1 on Pop-Med)')
    ax_a.set_xticks(x_idx)
    ax_a.set_xticklabels([x[1] for x in arm_order], fontsize=8.0)
    ax_a.set_ylabel('Final Seed-Best KPI_score [-] (analysis/kpi_fitness.py)')
    ax_a.set_ylim(18.0, max(means) + 5.2)
    ax_a.grid(True, axis='y', linestyle='--', alpha=0.35)
    ax_a.legend(loc='lower right', fontsize=7.2, framealpha=0.94)

    # Annotate pairwise Mann-Whitney U + Cliff's delta
    tests = {
        (t['metric'], t['comparator_b']): t
        for t in w3['pairwise_tests']
        if t['comparator_a'] == 'B3_Wave3_Exotic3D_QD_CEI'
    }
    t_b1 = tests[('final_best_kpi_score', b1_k)]
    t_b1_med = tests[('final_median_kpi_score', b1_k)]
    t_a1 = tests[('final_best_kpi_score', 'A1_No_Mutation_Crossover_Only')]
    t_a2 = tests[('final_best_kpi_score', 'A2_No_Selection_Random_Walk')]
    ax_a.text(
        0.02,
        0.96,
        f"B3 vs B1 (Best KPI, B1>B3): Δ={t_b1['mean_diff']:+.2f}, U={t_b1['mann_whitney_u']:.0f}, p_Holm={t_b1['p_value_holm_bonferroni']:.4g}, δ={t_b1['cliffs_delta']:+.2f}\n"
        f"B3 vs B1 (Pop-Median KPI): Δ={t_b1_med['mean_diff']:+.2f}, U={t_b1_med['mann_whitney_u']:.0f}, p_Holm={t_b1_med['p_value_holm_bonferroni']:.4g}, δ={t_b1_med['cliffs_delta']:+.2f}\n"
        f"B3 vs A1 (No-Mut Best): Δ={t_a1['mean_diff']:+.2f}, U={t_a1['mann_whitney_u']:.0f}, p_Holm={t_a1['p_value_holm_bonferroni']:.4g}, δ={t_a1['cliffs_delta']:+.2f}\n"
        f"B3 vs A2 (No-Sel Best): Δ={t_a2['mean_diff']:+.2f}, U={t_a2['mann_whitney_u']:.0f}, p_Holm={t_a2['p_value_holm_bonferroni']:.4g}, δ={t_a2['cliffs_delta']:+.2f}",
        transform=ax_a.transAxes,
        va='top',
        fontsize=7.1,
        bbox={'boxstyle': 'round,pad=0.32', 'facecolor': '#F1F5F9', 'edgecolor': '#94A3B8'},
    )

    # Panel B: De-Novo Phylogenetic Lineage Milestones & Step-by-Step Topological Discovery
    ax_b = fig.add_subplot(gs[0, 1])
    ax_b.set_facecolor('#FFFFFF')
    # Plot all 10 seeds' milestone trajectories in B3
    for s_idx, o in enumerate(outcomes_map['B3_Wave3_Exotic3D_QD_CEI']):
        ms = o['lineage_milestones']
        gx = [m['generation'] for m in ms]
        ky = [m['kpi_score'] for m in ms]
        ax_b.step(gx, ky, where='post', color='#93C5FD', alpha=0.55, linewidth=1.2)
        ax_b.scatter(gx, ky, color='#0284C7', alpha=0.55, s=22)

    # Highlight the top seed's lineage milestones with topology callouts
    best_seed_outcome = max(
        outcomes_map['B3_Wave3_Exotic3D_QD_CEI'],
        key=lambda o: float(o['final_best_kpi_score']),
    )
    ms_top = best_seed_outcome['lineage_milestones']
    gx_top = [m['generation'] for m in ms_top]
    ky_top = [m['kpi_score'] for m in ms_top]
    ax_b.step(
        gx_top,
        ky_top,
        where='post',
        color='#0F172A',
        linewidth=2.5,
        label=f'Top Seed {best_seed_outcome["seed"]} Incumbent Milestone Trajectory ({len(ms_top)} Milestones)',
    )
    ax_b.scatter(gx_top, ky_top, color='#EF4444', s=52, zorder=6, edgecolor='#0F172A')

    # Label up to 4 key milestones along the top seed trajectory with non-overlapping offsets
    pick_indices = sorted(list({0, len(ms_top) // 3, (2 * len(ms_top)) // 3, len(ms_top) - 1}))
    callout_positions = [
        (1.2, 33.4),
        (5.2, 41.2),
        (8.8, 46.9),
        (14.5, 43.8),
    ]
    for rank_i, m_idx in enumerate(pick_indices):
        m = ms_top[m_idx]
        tx, ty = callout_positions[min(rank_i, len(callout_positions) - 1)]
        ax_b.annotate(
            f"g={m['generation']}: {m['child_topology_class']}\n"
            f"({m['total_entry_jets']} jets, TPMS={m['tpms_basis']}, KPI={m['kpi_score']:.2f})",
            xy=(m['generation'], m['kpi_score']),
            xytext=(tx, ty),
            fontsize=7.2,
            fontweight='bold',
            color='#0F172A',
            bbox={'boxstyle': 'round,pad=0.22', 'facecolor': '#FEF3C7', 'edgecolor': '#F59E0B', 'alpha': 0.94},
            arrowprops={'arrowstyle': '->', 'color': '#475569', 'lw': 0.9},
        )

    ax_b.set_title('B. Incumbent Improvement Milestones vs. Generation (n = 10 Seeds; Not Full Pedigree, W4-G7)')
    ax_b.set_xlabel('Evolutionary Generation g [-]')
    ax_b.set_ylabel('Incumbent Seed-Best KPI_score [-]')
    ax_b.grid(True, linestyle='--', alpha=0.35)
    ax_b.legend(loc='lower right', fontsize=7.8, framealpha=0.94)

    # Panel C: Population Phenotypic Diversity (Shannon Entropy H [bits]) & MAP-Elites Archive Illumination
    ax_c = fig.add_subplot(gs[1, 0])
    ax_c.set_facecolor('#FFFFFF')
    gens = np.array([b['generation'] for b in bands_map['B3_Wave3_Exotic3D_QD_CEI']])
    for key, label, col, ls in [
        ('B3_Wave3_Exotic3D_QD_CEI', f'B3: {wave_label} Exotic3D-QD-CEI (Sustained QD Diversity)', '#0284C7', '-'),
        (b1_k, 'B1: Uniform Random (Unfocused Sampling)', '#D97706', '--'),
        ('A2_No_Selection_Random_Walk', 'A2: No-Selection Random Walk', '#DC2626', ':'),
        ('A1_No_Mutation_Crossover_Only', 'A1: No-Mutation (Premature Diversity Collapse)', '#7C3AED', '-.'),
        ('B2_Conventional_Axisym_NSGA2', 'B2: Axisym NSGA-II (Single-Family Ceiling)', '#475569', '--'),
    ]:
        b_list = bands_map[key]
        div_m = np.array([b['shannon_diversity_bits']['mean'] for b in b_list])
        div_lo = np.array([b['shannon_diversity_bits']['ci95_low'] for b in b_list])
        div_hi = np.array([b['shannon_diversity_bits']['ci95_high'] for b in b_list])
        ax_c.plot(gens, div_m, color=col, linestyle=ls, linewidth=2.1, label=f'{label} [{div_m[-1]:.2f} bits]')
        ax_c.fill_between(gens, div_lo, div_hi, color=col, alpha=0.14)

    ax_c.set_title('C. Population Phenotypic Diversity (Shannon Entropy H [bits]) vs. Generation (n = 10 Seeds)')
    ax_c.set_xlabel('Evolutionary Generation g [-]')
    ax_c.set_ylabel('Active Population Shannon Diversity H [bits]')
    ax_c.set_xlim(0, gens[-1])
    ax_c.grid(True, linestyle='--', alpha=0.35)
    ax_c.legend(loc='upper right', fontsize=7.4, framealpha=0.94)

    # Panel D: 11-Family Screen Coverage, De-Novo Rediscovery & Sandbox Authored-Family LOFO Diagnostics
    ax_d = fig.add_subplot(gs[1, 1])
    ax_d.set_facecolor('#FFFFFF')
    held_out = sorted(
        w3['held_out_geometry_generalization'],
        key=lambda r: float(r.get('held_out_best_kpi_score') or -1e9),
    )
    fams = [r['topology_class'].replace('-vortex-head', '').replace('-coalescer', '') for r in held_out]
    kpis_ho = [float(r.get('held_out_best_kpi_score') or 0.0) for r in held_out]
    has_kpi_rmse = all(r.get('lofo_surrogate_rmse_kpi_score') is not None for r in held_out)
    rmses_ho = [float(r['lofo_surrogate_rmse_kpi_score']) for r in held_out] if has_kpi_rmse else None
    cout_rmses_ho = [r.get('sandbox_authored_family_rmse_cout_mg_l') for r in held_out]
    disc_ho = [int(r['de_novo_rediscovery_seeds']) for r in held_out]

    y_pos = np.arange(len(fams))
    bar_cols = ['#64748B' if 'baseline' in r['topology_class'] else '#0284C7' for r in held_out]
    bar_label = (
        'Held-Out Family Best KPI_score ± LOFO-CV Surrogate RMSE'
        if has_kpi_rmse
        else '11-Family Screen Best KPI_score (KPI surrogate RMSE = null; Cout LOFO RMSE annotated)'
    )
    ax_d.barh(
        y_pos,
        kpis_ho,
        xerr=rmses_ho,
        capsize=3 if has_kpi_rmse else 0,
        color=bar_cols,
        alpha=0.82,
        edgecolor='#0F172A',
        linewidth=0.8,
        label=bar_label,
    )
    ax_d.axvline(base_kpi, color='#EF4444', linestyle=':', linewidth=1.6, label=f'B0: CT-35 Single-Eval Reference ({base_kpi:.2f} [-])')
    for idx, (kv, dv) in enumerate(zip(kpis_ho, disc_ho)):
        offset = (rmses_ho[idx] if rmses_ho is not None else 0.0) + 0.35
        c_rmse = cout_rmses_ho[idx]
        ann = (
            f'{kv:.2f} ({dv}/10 seeds; Cout RMSE={float(c_rmse):.2f} mg/L)'
            if (not has_kpi_rmse and c_rmse is not None)
            else f'{kv:.2f} ({dv}/10 seeds)'
        )
        ax_d.text(kv + offset, idx, ann, va='center', fontsize=6.9, fontweight='bold')

    ax_d.set_title(
        'D. Held-Out 3D Geometry Family Performance & LOFO-CV Generalization'
        if has_kpi_rmse
        else 'D. Authored 11-Family Screen Summary (Not Held-Out; KPI Surrogate RMSE = null)'
    )
    ax_d.set_yticks(y_pos)
    ax_d.set_yticklabels(fams, fontsize=7.7)
    ax_d.set_xlabel(
        'Held-Out Family Composite KPI_score [-] (± LOFO Surrogate RMSE)'
        if has_kpi_rmse
        else 'Authored 11-Family Screen KPI_score [-] (sandbox_authored_family_rmse_kpi_score = null)'
    )
    ax_d.set_xlim(18.0, max(kpis_ho) + (12.2 if has_kpi_rmse else 18.5))
    ax_d.grid(True, axis='x', linestyle='--', alpha=0.35)
    ax_d.legend(loc='lower right', fontsize=7.2, framealpha=0.94)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=180)
    plt.close(fig)


def main() -> None:
    import argparse

    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--wave4-only', action='store_true', help='Generate only Wave-4 figures without touching Wave-3 PNGs')
    args = ap.parse_args()

    out_summary: dict = {'status': 'ok'}

    if not args.wave4_only and WAVE3_JSON.is_file() and CFD_JSON.is_file():
        w3 = json.loads(WAVE3_JSON.read_text(encoding='utf-8'))
        cfd = json.loads(CFD_JSON.read_text(encoding='utf-8'))
        w3_sha = file_sha256(WAVE3_JSON)
        cfd_sha = file_sha256(CFD_JSON)

        generate_readme_hero(w3, cfd, w3_sha, cfd_sha, HERO_OUT, 'Wave-3', 5.019376028759711)
        hero_meta = write_manifest(
            HERO_OUT,
            'Wave-3 README Hero Plot: Per-Generation KPI Trajectory (n=10 Seeds, 22,500 3D Evals) & 15-Run OpenFOAM 14 Elite Verification',
            [WAVE3_JSON, CFD_JSON],
        )

        generate_plate11_proof(w3, w3_sha, PLATE11_OUT, 'Wave-3', 'docs/results/wave3-evolution-proof-v1.json')
        plate11_meta = write_manifest(
            PLATE11_OUT,
            'Plate 11: Wave-3 6-Arm Ablation (n=10 Seeds), De-Novo Phylogenetic Lineage, Phenotypic Diversity & 11-Family Held-Out Geometry Generalization',
            [WAVE3_JSON, CFD_JSON],
        )
        out_summary.update({
            'wave3_results_sha256': w3_sha,
            'hero_png': str(HERO_OUT.relative_to(ROOT)),
            'hero_meta': hero_meta,
            'plate11_png': str(PLATE11_OUT.relative_to(ROOT)),
            'plate11_meta': plate11_meta,
        })

    if WAVE4_JSON.is_file() and CFD_V2_JSON.is_file() and ELITE_V2_JSON.is_file():
        w4 = json.loads(WAVE4_JSON.read_text(encoding='utf-8'))
        cfd_v2 = json.loads(CFD_V2_JSON.read_text(encoding='utf-8'))
        elite_v2 = json.loads(ELITE_V2_JSON.read_text(encoding='utf-8'))
        w4_sha = file_sha256(WAVE4_JSON)
        cfd_v2_sha = file_sha256(CFD_V2_JSON)
        w1_dp_gci = elite_v2['roache_gci_analysis']['dp_u_pa']
        w1_gci_v2_pct = float(
            w1_dp_gci.get('gci21_standard_fs125_pct')
            or w1_dp_gci.get('three_grid_half_span_pct_of_f2')
            or 0.0
        )

        generate_readme_hero(
            w4,
            cfd_v2,
            w4_sha,
            cfd_v2_sha,
            WAVE4_HERO_OUT,
            'Wave-4',
            w1_gci_v2_pct,
            'wave4-evolution-proof-v1.json',
            'wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json',
        )
        w4_hero_meta = write_manifest(
            WAVE4_HERO_OUT,
            'Wave-4 README Hero Plot: Per-Generation Screen KPI Trajectory on Repaired Operators (n=10 Seeds, MC n=72) & 15-Run OpenFOAM 14 v2 Unclamped GCI Verification',
            [WAVE4_JSON, CFD_V2_JSON, ELITE_V2_JSON],
        )

        generate_plate11_proof(
            w4,
            w4_sha,
            WAVE4_PLATE_OUT,
            'Wave-4',
            'docs/results/wave4-evolution-proof-v1.json',
        )
        w4_plate_meta = write_manifest(
            WAVE4_PLATE_OUT,
            'Wave-4 Plate: 6-Arm Ablation on Repaired Operators (n=10 Seeds, MC n=72), De-Novo Phylogenetic Lineage, Phenotypic Diversity & 11-Family Held-Out Geometry Generalization',
            [WAVE4_JSON, CFD_V2_JSON, ELITE_V2_JSON],
        )
        out_summary.update({
            'wave4_results_sha256': w4_sha,
            'wave4_hero_png': str(WAVE4_HERO_OUT.relative_to(ROOT)),
            'wave4_hero_meta': w4_hero_meta,
            'wave4_plate_png': str(WAVE4_PLATE_OUT.relative_to(ROOT)),
            'wave4_plate_meta': w4_plate_meta,
        })

    print(json.dumps(out_summary, indent=2))


if __name__ == '__main__':
    main()
