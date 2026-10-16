#!/usr/bin/env python3
"""INT-03: Wave-4 smoke campaign after TR-01..04 + TR-13 to verify zero silent baseline fallbacks
and report invalid-proposal and repaired-proposal rates honestly across variation operators.

Writes `docs/results/wave4-smoke-v1.json`.
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
OUT_PATH = ROOT / 'docs/results/wave4-smoke-v1.json'
SMOKE_SEEDS = [20261201, 20261202, 20261203]


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def verify_no_baseline_ct35_fallbacks() -> dict[str, Any]:
    """Verify no silent unwrap_or / fallback to baseline_ct35() exists in crates/ and run truth_no_baseline_fallback."""
    proc = subprocess.run(
        ['rg', r'unwrap_or.*baseline_ct35|ok\(\).*baseline_ct35|Err.*baseline_ct35', 'crates'],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    disallowed_lines = [ln.strip() for ln in proc.stdout.splitlines() if ln.strip()]
    test_proc = subprocess.run(
        ['cargo', 'test', '-p', 'joil-core', '--test', 'truth_variation', '--', 'truth_no_baseline_fallback'],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    return {
        'passed': len(disallowed_lines) == 0 and test_proc.returncode == 0,
        'disallowed_fallback_matches': len(disallowed_lines),
        'disallowed_lines': disallowed_lines,
        'rust_truth_no_baseline_fallback_test_passed': test_proc.returncode == 0,
    }


def run_wave4_smoke(
    seeds: list[int] | None = None,
    islands: int = 2,
    pop_per_island: int = 9,
    generations: int = 6,
    mc_samples: int = 12,
    out_path: Path = OUT_PATH,
) -> dict[str, Any]:
    if seeds is None:
        seeds = list(SMOKE_SEEDS)

    fallback_audit = verify_no_baseline_ct35_fallbacks()
    if not fallback_audit['passed']:
        raise RuntimeError(
            f"Silent baseline_ct35() fallback detected in crates/: {fallback_audit['disallowed_lines']}"
        )

    # Ensure joil-cli is built in debug/test or release mode
    subprocess.run(
        ['cargo', 'build', '-p', 'joil-cli'],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    cli_bin = ROOT / 'target/debug/joil-cli'

    total_budget = 0
    total_ref_evals = 0
    total_invalid = 0
    total_repaired = 0
    total_unrepaired_evaluated = 0
    agg_ops: dict[str, dict[str, int]] = {}
    seed_summaries: list[dict[str, Any]] = []
    failure_reasons_histogram: dict[str, int] = {}
    repair_fields_histogram: dict[str, int] = {}

    with tempfile.TemporaryDirectory(prefix='joil-wave4-smoke-') as tmp:
        tmp_root = Path(tmp)
        for seed in seeds:
            seed_dir = tmp_root / f'seed_{seed}'
            subprocess.run(
                [
                    str(cli_bin),
                    'evolve',
                    '--seed',
                    str(seed),
                    '--islands',
                    str(islands),
                    '--pop',
                    str(pop_per_island),
                    '--generations',
                    str(generations),
                    '--mc',
                    str(mc_samples),
                    '--out',
                    str(seed_dir),
                ],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            summary_file = seed_dir / 'campaign-summary.json'
            raw_bytes = summary_file.read_bytes()
            summary = json.loads(raw_bytes.decode('utf-8'))

            b_total = int(summary['budget'])
            b_ref = int(summary.get('reference_evaluations', 0))
            b_inv = int(summary['proposals_invalid'])
            b_rep = int(summary['proposals_repaired'])
            b_eval = int(summary['evaluations'])
            if b_total != b_inv + b_rep + b_eval:
                raise RuntimeError(
                    f'TR-03 budget accounting mismatch on seed {seed}: '
                    f'{b_total} != {b_inv} + {b_rep} + {b_eval}'
                )

            # Verify TR-13 lineage & EV-04 evidence tier on top Pareto elites
            for elite in summary.get('top_pareto_elites', []):
                if elite.get('lineage') is None:
                    raise RuntimeError(f"Missing TR-13 lineage on elite {elite.get('id')}")
                if elite.get('evidence_tier') != 'software_contract':
                    raise RuntimeError(
                        f"Unexpected evidence_tier on prior-screen elite {elite.get('id')}: {elite.get('evidence_tier')}"
                    )

            total_budget += b_total
            total_ref_evals += b_ref
            total_invalid += b_inv
            total_repaired += b_rep
            total_unrepaired_evaluated += b_eval

            for op_id, stats in summary.get('operator_statistics', {}).items():
                slot = agg_ops.setdefault(
                    op_id,
                    {'attempts': 0, 'invalid': 0, 'repaired': 0, 'evaluated': 0},
                )
                slot['attempts'] += int(stats['attempts'])
                slot['invalid'] += int(stats['invalid'])
                slot['repaired'] += int(stats['repaired'])
                slot['evaluated'] += int(stats['evaluated'])

            for rec in summary.get('proposal_receipts', []):
                if rec.get('failure'):
                    reason = str(rec['failure']).split(':', 1)[0].strip()
                    failure_reasons_histogram[reason] = failure_reasons_histogram.get(reason, 0) + 1
                for rep in rec.get('repairs', []):
                    for ed in rep.get('edits', []):
                        fld = str(ed.get('path', rep.get('rule', 'unknown')))
                        repair_fields_histogram[fld] = repair_fields_histogram.get(fld, 0) + 1

            seed_summaries.append(
                {
                    'seed': seed,
                    'campaign_summary_sha256': _sha256_bytes(raw_bytes),
                    'budget': b_total,
                    'reference_evaluations': b_ref,
                    'proposals_invalid': b_inv,
                    'proposals_repaired': b_rep,
                    'evaluations_unrepaired': b_eval,
                    'total_f0_f1_evaluated': b_rep + b_eval,
                    'invalid_proposal_rate': round(b_inv / b_total, 6) if b_total > 0 else 0.0,
                    'repaired_proposal_rate': round(b_rep / b_total, 6) if b_total > 0 else 0.0,
                    'unrepaired_evaluation_rate': round(b_eval / b_total, 6) if b_total > 0 else 0.0,
                    'map_elites_occupied_cells': int(summary['map_elites_occupied_cells']),
                    'pareto_front_size': int(summary['pareto_front_size']),
                }
            )

        # Also run a 5-seed, 3-generation multi-arm wave2-benchmark smoke check to capture per-arm invalid/repaired rates
        proof_smoke_file = tmp_root / 'wave4_proof_smoke.json'
        proof_seeds = [20261201, 20261202, 20261203, 20261204, 20261205]
        subprocess.run(
            [
                str(cli_bin),
                'wave2-benchmark',
                '--seeds',
                ','.join(str(s) for s in proof_seeds),
                '--islands',
                '2',
                '--pop',
                '6',
                '--generations',
                '3',
                '--mc',
                '8',
                '--out',
                str(proof_smoke_file),
            ],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        proof_doc = json.loads(proof_smoke_file.read_text(encoding='utf-8'))
        arm_budget_summaries: dict[str, Any] = {}
        outcomes_map = proof_doc.get('arm_outcomes') or proof_doc.get('per_seed_outcomes') or {}
        for arm_name, outcomes in outcomes_map.items():
            arm_b = sum(int(o['budget']) for o in outcomes)
            arm_inv = sum(int(o['proposals_invalid']) for o in outcomes)
            arm_rep = sum(int(o['proposals_repaired']) for o in outcomes)
            arm_ev = sum(int(o['evaluations']) for o in outcomes)
            if arm_b != arm_inv + arm_rep + arm_ev:
                raise RuntimeError(
                    f'Arm {arm_name} budget mismatch: {arm_b} != {arm_inv}+{arm_rep}+{arm_ev}'
                )
            arm_budget_summaries[arm_name] = {
                'budget': arm_b,
                'proposals_invalid': arm_inv,
                'proposals_repaired': arm_rep,
                'evaluations_unrepaired': arm_ev,
                'total_f0_f1_evaluated': arm_rep + arm_ev,
                'invalid_proposal_rate': round(arm_inv / arm_b, 6) if arm_b > 0 else 0.0,
                'repaired_proposal_rate': round(arm_rep / arm_b, 6) if arm_b > 0 else 0.0,
                'unrepaired_evaluation_rate': round(arm_ev / arm_b, 6) if arm_b > 0 else 0.0,
            }

    operator_table: dict[str, Any] = {}
    for op_id, s in sorted(agg_ops.items()):
        att = s['attempts']
        inv = s['invalid']
        rep = s['repaired']
        ev = s['evaluated']
        if att != inv + rep + ev:
            raise RuntimeError(f'Operator {op_id} accounting mismatch: {att} != {inv}+{rep}+{ev}')
        operator_table[op_id] = {
            'attempts': att,
            'invalid': inv,
            'repaired': rep,
            'evaluated_unrepaired': ev,
            'invalid_rate': round(inv / att, 6) if att > 0 else 0.0,
            'repaired_rate': round(rep / att, 6) if att > 0 else 0.0,
            'evaluated_unrepaired_rate': round(ev / att, 6) if att > 0 else 0.0,
        }

    report = {
        'schema_version': '0.4.0',
        'workplan_item': 'INT-03',
        'protocol_id': 'JOIL-WAVE4-SMOKE-NO-FALLBACK-V1',
        'claim_state': 'f0_f1_prior_laden_reduced_order_screen_smoke_audit_not_f2_or_f4_qualified',
        'evidence_tier': 'software_contract',
        'dependencies_verified': ['TR-01', 'TR-02', 'TR-03', 'TR-04', 'TR-11', 'TR-13', 'EV-08'],
        'config': {
            'seeds': seeds,
            'n_seeds': len(seeds),
            'islands': islands,
            'pop_per_island': pop_per_island,
            'generations': generations,
            'mc_samples': mc_samples,
        },
        'zero_silent_baseline_fallbacks_audit': fallback_audit,
        'aggregate_proposal_budget': {
            'budget': total_budget,
            'reference_evaluations': total_ref_evals,
            'proposals_invalid': total_invalid,
            'proposals_repaired': total_repaired,
            'evaluations_unrepaired': total_unrepaired_evaluated,
            'total_f0_f1_evaluated': total_repaired + total_unrepaired_evaluated,
            'exclusive_partition_verified': total_budget
            == (total_invalid + total_repaired + total_unrepaired_evaluated),
            'invalid_proposal_rate': round(total_invalid / total_budget, 6)
            if total_budget > 0
            else 0.0,
            'repaired_proposal_rate': round(total_repaired / total_budget, 6)
            if total_budget > 0
            else 0.0,
            'unrepaired_evaluation_rate': round(total_unrepaired_evaluated / total_budget, 6)
            if total_budget > 0
            else 0.0,
        },
        'operator_statistics': operator_table,
        'six_arm_proof_smoke_budget': arm_budget_summaries,
        'failure_reasons_histogram': dict(sorted(failure_reasons_histogram.items())),
        'repair_fields_histogram': dict(sorted(repair_fields_histogram.items())),
        'per_seed_summaries': seed_summaries,
    }

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    return report


if __name__ == '__main__':
    rep = run_wave4_smoke()
    print(
        json.dumps(
            {
                'workplan_item': rep['workplan_item'],
                'out_path': str(OUT_PATH.relative_to(ROOT)),
                'aggregate_proposal_budget': rep['aggregate_proposal_budget'],
                'zero_silent_baseline_fallbacks_passed': rep[
                    'zero_silent_baseline_fallbacks_audit'
                ]['passed'],
            },
            indent=2,
        )
    )
