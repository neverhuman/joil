#!/usr/bin/env python3
"""WHITEPAPER lead figure (owner request PAGE1-WHY-MATTERS, workplan DOC-10).

Wave-4 best screen `KPI_score` against generation for evolution (`B3`) and equal-budget uniform random search
(`B1`), with the `CT-35` reference. Every plotted value and every number in the labels is read from
`docs/results/wave4-evolution-proof-v1.json`; nothing is typed in. Reuses the README hero generator's helpers,
data layout and arm colours (`tools/generate_wave3_proof_figures.py`, owned by INT and not modified here).

    python3 tools/generate_page1_figure.py            # write PNG + manifest
    python3 tools/generate_page1_figure.py --verify   # check manifest against PNG, receipt and series
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.generate_wave3_proof_figures import _b1_key, file_sha256  # noqa: E402

RECEIPT = ROOT / 'docs/results/wave4-evolution-proof-v1.json'
OUT = ROOT / 'docs/figures/wave4-page1-why-it-matters.png'
B3 = 'B3_Wave3_Exotic3D_QD_CEI'
COLOURS = {'B3': '#0284C7', 'B1': '#D97706', 'CT35': '#1E293B'}  # as in the README hero plot
TITLE = 'Wave-4 lead figure: evolution (B3) vs equal-budget uniform random search (B1) on the F1 screen'


def extract_series(w4: dict) -> dict:
    """Everything the figure draws or prints, taken from the receipt without arithmetic."""
    bands = w4['generation_band_curves']
    b1 = _b1_key(bands)
    tests = {
        (t['metric'], t['comparator_b']): t
        for t in w4['pairwise_tests']
        if t['comparator_a'] == B3
    }
    best_b1 = tests[('final_best_kpi_score', b1)]
    median_b1 = tests[('final_median_kpi_score', b1)]

    def arm(key: str) -> dict:
        return {
            'best_mean': [b['best_kpi_score']['mean'] for b in bands[key]],
            'best_ci95_low': [b['best_kpi_score']['ci95_low'] for b in bands[key]],
            'best_ci95_high': [b['best_kpi_score']['ci95_high'] for b in bands[key]],
            'median_mean': [b['median_kpi_score']['mean'] for b in bands[key]],
            'seed_best': [[s['best_kpi_score'] for s in o['generation_trace']] for o in w4['arm_outcomes'][key]],
        }

    return {
        'generation': [b['generation'] for b in bands[B3]],
        'n_seeds': w4['n_seeds'],
        'proposals_per_generation': w4['population_per_generation'],
        'proposals_per_seed_per_arm': w4['evaluations_per_seed_per_arm'],
        'ct35_kpi': w4['baseline_b0_ct35_kpi_score'],
        'B3': arm(B3),
        'B1': arm(b1),
        'best_test': {k: best_b1[k] for k in ('mann_whitney_u', 'p_value_holm_bonferroni', 'cliffs_delta')},
        'median_test': {k: median_b1[k] for k in ('mann_whitney_u', 'p_value_holm_bonferroni', 'cliffs_delta')},
    }


def series_sha256(series: dict) -> str:
    return hashlib.sha256(json.dumps(series, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def render(series: dict, out: Path) -> None:
    import matplotlib

    matplotlib.use('Agg')
    import matplotlib.pyplot as plt

    plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 10.0})
    g = series['generation']
    fig, ax = plt.subplots(figsize=(11.0, 5.6), dpi=160, facecolor='#F8FAFC')
    ax.set_facecolor('#FFFFFF')

    for name, label in (('B1', 'B1 uniform random search'), ('B3', 'B3 evolution (QD-CEI arm)')):
        arm, col = series[name], COLOURS[name]
        for seed in arm['seed_best']:
            ax.plot(g, seed, color=col, alpha=0.18, linewidth=0.8)
        ax.fill_between(g, arm['best_ci95_low'], arm['best_ci95_high'], color=col, alpha=0.18, linewidth=0)
        ax.plot(g, arm['best_mean'], color=col, linewidth=2.6,
                label=f"{label}: best so far, seed mean {arm['best_mean'][-1]:.2f} "
                      f"[{arm['best_ci95_low'][-1]:.2f}, {arm['best_ci95_high'][-1]:.2f}]")
        ax.plot(g, arm['median_mean'], color=col, linewidth=1.5, linestyle=':',
                label=f"{name} population median, seed mean {arm['median_mean'][-1]:.2f}")
    ax.axhline(series['ct35_kpi'], color=COLOURS['CT35'], linestyle='--', linewidth=1.4,
               label=f"CT-35 reference (single frozen evaluation, not a search arm) {series['ct35_kpi']:.2f}")

    bt, mt = series['best_test'], series['median_test']
    ax.text(0.985, 0.04,
            f"Best score, B3 > B1: U = {bt['mann_whitney_u']:g}, p_Holm = {bt['p_value_holm_bonferroni']:g}, "
            f"Cliff's δ = {bt['cliffs_delta']:+.2f}  (random search not beaten)\n"
            f"Population median, B3 > B1: U = {mt['mann_whitney_u']:g}, p_Holm = {mt['p_value_holm_bonferroni']:g}, "
            f"Cliff's δ = {mt['cliffs_delta']:+.2f}",
            transform=ax.transAxes, ha='right', va='bottom', fontsize=8.6,
            bbox={'boxstyle': 'round,pad=0.4', 'facecolor': '#F1F5F9', 'edgecolor': '#94A3B8'})

    ax.set_xlim(g[0], g[-1])
    ax.set_xlabel(f"Generation ({series['proposals_per_generation']} charged proposals each; "
                  f"{series['proposals_per_seed_per_arm']} per seed and arm)")
    ax.set_ylabel('Screen KPI_score (higher is better)')
    ax.set_title(f"Wave-4 on repaired operators, {series['n_seeds']} seeds per arm: uniform random search "
                 f"is not beaten on best screen score", fontsize=11.5, fontweight='bold')
    fig.text(0.5, 0.015, 'F1 prior-laden screen, sandbox evidence tier: screen values, not physical performance. '
             'Bands: 95% bootstrap intervals of the seed mean; thin lines: single seeds.',
             ha='center', fontsize=8.4, color='#334155', style='italic')
    ax.grid(True, linestyle='--', alpha=0.35)
    ax.legend(loc='center right', fontsize=8.2, framealpha=0.95)
    fig.tight_layout(rect=(0, 0.04, 1, 1))
    fig.savefig(out, metadata={'Software': None})
    plt.close(fig)


def manifest_for(out: Path, series: dict) -> dict:
    return {
        'schema_version': 1,
        'figure_path': out.relative_to(ROOT).as_posix(),
        'figure_sha256': file_sha256(out),
        'title': TITLE,
        'generator_script': 'tools/generate_page1_figure.py',
        'plotted_series_sha256': series_sha256(series),
        'source_receipts': [{'path': RECEIPT.relative_to(ROOT).as_posix(), 'sha256': file_sha256(RECEIPT)}],
    }


def verify(out: Path = OUT) -> list[str]:
    """Errors if the committed PNG, receipt or plotted series no longer match the manifest."""
    errors = []
    manifest_path = Path(str(out) + '.manifest.json')
    if not out.is_file() or not manifest_path.is_file():
        return [f'{out.relative_to(ROOT)} or its manifest is missing']
    manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
    if manifest['figure_sha256'] != file_sha256(out):
        errors.append('figure_sha256 does not match the committed PNG')
    for src in manifest['source_receipts']:
        if file_sha256(ROOT / src['path']) != src['sha256']:
            errors.append(f"source receipt {src['path']} changed since the figure was generated")
    series = extract_series(json.loads(RECEIPT.read_text(encoding='utf-8')))
    if manifest['plotted_series_sha256'] != series_sha256(series):
        errors.append('plotted series no longer match the receipt; regenerate the figure')
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--verify', action='store_true', help='check the committed figure; write nothing')
    args = parser.parse_args()
    if args.verify:
        errors = verify()
        for e in errors:
            print(f'FAIL: {e}')
        print('OK' if not errors else f'{len(errors)} error(s)')
        return 1 if errors else 0
    series = extract_series(json.loads(RECEIPT.read_text(encoding='utf-8')))
    render(series, OUT)
    manifest = manifest_for(OUT, series)
    Path(str(OUT) + '.manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'png': manifest['figure_path'], 'sha256': manifest['figure_sha256']}))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
