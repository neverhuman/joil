#!/usr/bin/env python3
"""WHITEPAPER §4.6 figure (IR NEED-DOC-06, workplan DOC-10): same-pipeline CT-35 control vs archived shells and
the first Wave-4 design with enacted split ducts, from committed receipts only.

Panel A: static pressure drop against effective cell size on G0/G1/G2 for the same-pipeline CT-35 control
(docs/results/wave4-gci-v1.json) and the three archived two-duct shells (v2 dossiers), with the recorded
different-pipeline CT-35 case as a reference line. Panel B: the solved 8-jet Wave-4 design against the control,
on a log axis. No number is typed in; the plotted series and their digest are recorded in the manifest.

    python3 tools/generate_wave4_gci_figure.py            # write PNG + manifest
    python3 tools/generate_wave4_gci_figure.py --verify   # check manifest against PNG, receipts and series
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.generate_wave3_proof_figures import file_sha256  # noqa: E402

GCI = ROOT / 'docs/results/wave4-gci-v1.json'
ELITE_V2 = ROOT / 'docs/results/elite-3grid-gci-and-rsm-verification-v2.json'
WAVE2_V2 = ROOT / 'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json'
SOURCES = (GCI, ELITE_V2, WAVE2_V2)
OUT = ROOT / 'docs/figures/wave4-gci-same-pipeline-control.png'
TITLE = 'Wave-4 section 4.6: same-pipeline CT-35 control, archived two-duct shells and the enacted 8-jet design'


def _grid_series(grids: list[dict]) -> dict:
    return {
        'h_mm': [g['effective_cell_size_mm'] for g in grids],
        'dp_u_kpa': [g['dp_u_pa'] / 1000.0 for g in grids],
        'cells': [g['cells'] for g in grids],
    }


def extract_series(gci: dict, elite_v2: dict, wave2_v2: dict) -> dict:
    control = gci['same_pipeline_ct35_control']
    solved = [w for w in gci['wave4_b3_top_winners'] if w['solver_status'] == 'Solved3Grid']
    return {
        'control': _grid_series(control['grids']),
        'control_state': control['roache_gci_v2_analysis']['dp_u_pa']['gci_validity_state'],
        'recorded_ct35_kpa': control['g09_pipeline_confounding_audit']['historical_extended_ct35_reference_dp_u_pa'] / 1000.0,
        'archived': {
            'Wave-1 shell (label 6-jet)': _grid_series(elite_v2['grids']),
            'Wave-2 shell (label gyroid)': _grid_series(wave2_v2['wave2_tpms_gyroid_elite_5run_dossier']['grids']),
            'Wave-2 shell (label 8-jet)': _grid_series(wave2_v2['wave2_octa_branch_8jet_elite_5run_dossier']['grids']),
        },
        'winners': {
            w['subject_id']: {
                **_grid_series(w['grids']),
                'state': w['roache_gci_v2_analysis']['dp_u_pa']['gci_validity_state'],
                'inlet_paths': w['enactment_receipt']['measured_disjoint_inlet_paths'],
            }
            for w in solved
        },
        'unsolved_winners': len(gci['wave4_b3_top_winners']) - len(solved),
    }


def series_sha256(series: dict) -> str:
    return hashlib.sha256(json.dumps(series, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def render(series: dict, out: Path) -> None:
    import matplotlib

    matplotlib.use('Agg')
    import matplotlib.pyplot as plt

    plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 9.8})
    fig, (ax_a, ax_b) = plt.subplots(1, 2, figsize=(12.0, 5.2), dpi=160, facecolor='#F8FAFC')
    ctl = series['control']
    for ax in (ax_a, ax_b):
        ax.set_facecolor('#FFFFFF')
        ax.plot(ctl['h_mm'], ctl['dp_u_kpa'], color='#111827', marker='s', linewidth=2.4,
                label=f"CT-35, same pipeline ({series['control_state']})")
        ax.invert_xaxis()
        ax.grid(True, linestyle='--', alpha=0.35)
        ax.set_xlabel('Effective cell size h [mm] (G0 → G1 → G2)')

    colours = ('#0284C7', '#16A34A', '#7C3AED')
    for (name, s), col in zip(series['archived'].items(), colours):
        ax_a.plot(s['h_mm'], s['dp_u_kpa'], color=col, marker='o', linewidth=1.6, label=name)
    ax_a.axhline(series['recorded_ct35_kpa'], color='#DC2626', linestyle='--', linewidth=1.4,
                 label=f"recorded CT-35 case, different pipeline ({series['recorded_ct35_kpa']:.2f} kPa)")
    ax_a.set_ylabel('Static pressure drop Δp_u [kPa]')
    ax_a.set_title('A. Archived shells vs CT-35 built by the same pipeline\n(Wave-1/2 two-duct surfaces)', fontweight='bold')
    ax_a.legend(loc='center left', fontsize=7.8, framealpha=0.95)

    for name, s in series['winners'].items():
        ax_b.plot(s['h_mm'], s['dp_u_kpa'], color='#D97706', marker='^', linewidth=2.2,
                  label=f"{name}: {s['inlet_paths']} inlet paths ({s['state']}, not asymptotic)")
    ax_b.set_yscale('log')
    ax_b.set_ylabel('Static pressure drop Δp_u [kPa] (log scale)')
    ax_b.set_title(f"B. Wave-4 design with enacted split ducts\n({series['unsolved_winners']} other top designs not solved)",
                   fontweight='bold')
    ticks = [t for t in (10, 20, 50, 100, 200, 500, 1000)
             if min(ctl['dp_u_kpa']) / 1.5 <= t <= max(max(s['dp_u_kpa']) for s in series['winners'].values()) * 1.5]
    ax_b.set_yticks(ticks)
    ax_b.set_yticklabels([f'{t:g}' for t in ticks])
    ax_b.minorticks_off()
    ax_b.legend(loc='upper right', fontsize=7.8, framealpha=0.95)

    fig.suptitle('OpenFOAM 14 kOmegaSST carrier CFD (numerical only)\nThe archived shells match the same-pipeline '
                 'CT-35 control; the 8-jet design is not grid-converged', fontsize=10.6, fontweight='bold')
    fig.tight_layout(rect=(0, 0, 1, 0.92))
    fig.savefig(out, metadata={'Software': None})
    plt.close(fig)


def _load() -> tuple[dict, dict, dict]:
    return tuple(json.loads(p.read_text(encoding='utf-8')) for p in SOURCES)  # type: ignore[return-value]


def manifest_for(out: Path, series: dict) -> dict:
    return {
        'schema_version': 1,
        'figure_path': out.relative_to(ROOT).as_posix(),
        'figure_sha256': file_sha256(out),
        'title': TITLE,
        'generator_script': 'tools/generate_wave4_gci_figure.py',
        'plotted_series_sha256': series_sha256(series),
        'source_receipts': [{'path': p.relative_to(ROOT).as_posix(), 'sha256': file_sha256(p)} for p in SOURCES],
    }


def verify(out: Path = OUT) -> list[str]:
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
    if manifest['plotted_series_sha256'] != series_sha256(extract_series(*_load())):
        errors.append('plotted series no longer match the receipts; regenerate the figure')
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
    series = extract_series(*_load())
    render(series, OUT)
    manifest = manifest_for(OUT, series)
    Path(str(OUT) + '.manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'png': manifest['figure_path'], 'sha256': manifest['figure_sha256']}))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
