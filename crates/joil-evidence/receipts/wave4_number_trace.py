"""Independent arithmetic audit of the committed Wave-4 receipt; no Joil imports.

Run from the repository root with a Python environment containing NumPy.
Writes recomputed values to stdout; it neither evaluates geometry nor edits sources.
"""

import itertools
import json
import math
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np


def bootstrap(values, seed):
    """10,000 percentile resamples, replaying the published SplitMix64 policy."""
    values = np.asarray(values, dtype=float)
    n = len(values)
    with np.errstate(over="ignore"):
        z = np.uint64(seed) + np.uint64(0x9E3779B97F4A7C15) * np.arange(
            1, 10000 * n + 1, dtype=np.uint64
        )
        z = (z ^ (z >> 30)) * np.uint64(0xBF58476D1CE4E5B9)
        z = (z ^ (z >> 27)) * np.uint64(0x94D049BB133111EB)
        indices = ((z ^ (z >> 31)) % np.uint64(n)).astype(int).reshape(10000, n)
    means = values[indices].sum(axis=1) / n
    return dict(
        n_seeds=n,
        mean=float(values.mean()),
        std_dev=float(values.std(ddof=1)),
        median=float(np.median(values)),
        iqr=float(np.quantile(values, 0.75) - np.quantile(values, 0.25)),
        ci95_low=float(np.quantile(means, 0.025)),
        ci95_high=float(np.quantile(means, 0.975)),
    )


def approx_erfc(x):
    # Published A&S 7.1.26 approximation; also report math.erfc independently.
    t = 1 / (1 + 0.3275911 * abs(x))
    tail = (((((1.061405429 * t - 1.453152027) * t + 1.421413741) * t
              - 0.284496736) * t + 0.254829592) * t * math.exp(-x * x))
    return tail if x >= 0 else 2 - tail


def rank_test(a, b, smaller=False):
    a, b = list(a), list(b)
    differences = [(-1 if smaller else 1) * (x - y) for x in a for y in b]
    wins = sum(x > 0 for x in differences)
    ties = sum(x == 0 for x in differences)
    u = wins + ties / 2
    z = (u - len(a) * len(b) / 2) / math.sqrt(len(a) * len(b) * (len(a) + len(b) + 1) / 12)
    return dict(
        mean_diff=float(np.mean(a) - np.mean(b)),
        median_diff=float(np.median(a) - np.median(b)),
        mann_whitney_u=u,
        z_score=z,
        p_value_one_sided=round(0.5 * approx_erfc(z / math.sqrt(2)), 8),
        normal_p_math_erfc=0.5 * math.erfc(z / math.sqrt(2)),
        cliffs_delta=(wins - (len(differences) - wins - ties)) / len(differences),
        ties=ties,
    )


def kpi(candidate, reference_dp):
    mc = candidate['mc_robustness']
    return (30 - mc['cout_sample_screen_upper_mg_l']
            + 25 * mc['grade_20um_sample_screen_lower']
            + 10 * (reference_dp - candidate['nominal_metrics']['dp_u_pa']) / reference_dp
            - 500 * candidate['constraint_violation'])


def audit(x):
    mismatches = []

    def compare(path, actual, expected, tolerance=0.00000101):
        if abs(actual - expected) > tolerance:
            mismatches.append(dict(path=path, recomputed=actual, recorded=expected))

    fields = ['best_kpi_score', 'median_kpi_score',
              'best_cout_sample_screen_upper_mg_l', 'median_cout_sample_screen_upper_mg_l',
              'pareto_hypervolume', 'map_elites_occupied_cells',
              'shannon_diversity_bits', 'mean_exoticness_index']
    result = dict(generation_statistics={}, final_statistics={}, tests=[],
                  lineage={}, families=[], budgets={}, candidate_kpi_checks=0)
    seeds = x['master_seeds']
    assert len(seeds) == len(set(seeds)) == x['n_seeds'] == 10
    discovery = defaultdict(set)
    for arm, outcomes in x['arm_outcomes'].items():
        assert [o['seed'] for o in outcomes] == seeds
        result['generation_statistics'][arm] = []
        result['budgets'][arm] = {
            field: sum(o[field] for o in outcomes)
            for field in ['budget', 'reference_evaluations', 'proposals_invalid',
                          'proposals_repaired', 'evaluations', 'total_evaluations', 'valid_f0_evaluations']
        }
        result['lineage'][arm] = []
        for o in outcomes:
            assert o['proposals_invalid'] + o['proposals_repaired'] + o['evaluations'] == o['budget'] == 450
            assert len(o['generation_trace']) == 25
            assert [g['cumulative_evaluations'] for g in o['generation_trace']] == list(range(18, 451, 18))
            compare(f'arm_outcomes/{arm}/{o["seed"]}/final_best_kpi_score',
                    kpi(o['best_candidate'], x['baseline_b0_ct35_dp_u_pa']), o['final_best_kpi_score'])
            result['candidate_kpi_checks'] += 1
            milestones = o['lineage_milestones']
            result['lineage'][arm].append(dict(
                seed=o['seed'], milestones=len(milestones),
                milestone_cross_family=sum(m['cross_family_jump'] for m in milestones),
                all_proposal_cross_family=o['total_cross_family_lineage_jumps'],
                final_trace_cross_family=o['generation_trace'][-1]['cumulative_lineage_jumps'],
                operator_counts=dict(Counter(m['operator_applied'] for m in milestones)),
            ))
            assert o['total_cross_family_lineage_jumps'] == o['generation_trace'][-1]['cumulative_lineage_jumps']
            if arm.startswith('B3_'):
                for m in milestones:
                    discovery[m['child_topology_class']].add(o['seed'])
                discovery[o['best_candidate']['topology_class']].add(o['seed'])
        for generation in range(25):
            stats = {}
            band = x['generation_band_curves'][arm][generation]
            for i, field in enumerate(fields):
                vals = [o['generation_trace'][generation][field] for o in outcomes]
                st = bootstrap(vals, 20269100 + i * 100 + generation)
                for key, value in st.items():
                    compare(f'generation_band_curves/{arm}/{generation}/{field}/{key}', value, band[field][key])
                stats[field] = st
            result['generation_statistics'][arm].append(stats)
        final = result['generation_statistics'][arm][-1]
        result['final_statistics'][arm] = final
        for field, recorded in x['final_summary_statistics'][arm].items():
            for key, value in recorded.items():
                compare(f'final_summary_statistics/{arm}/{field}/{key}', final[field][key], value)

    for recorded in x['pairwise_tests']:
        a = [o[recorded['metric']] for o in x['arm_outcomes'][recorded['comparator_a']]]
        b = [o[recorded['metric']] for o in x['arm_outcomes'][recorded['comparator_b']]]
        test = rank_test(a, b, 'cout' in recorded['metric'])
        test.update(metric=recorded['metric'], comparator_b=recorded['comparator_b'])
        for field in ['mean_diff', 'median_diff', 'mann_whitney_u', 'p_value_one_sided', 'cliffs_delta']:
            tolerance = 0.00000101 if field.endswith('_diff') else 1e-9
            compare(f'pairwise_tests/{len(result["tests"])}/{field}', test[field], recorded[field], tolerance)
        compare(f'pairwise_tests/{len(result["tests"])}/z_score', test['z_score'], recorded['z_score'], 0.000051)
        result['tests'].append(test)
    running = 0
    for rank, index in enumerate(sorted(range(len(result['tests'])), key=lambda i: result['tests'][i]['p_value_one_sided'])):
        test = result['tests'][index]
        running = max(running, min(1, test['p_value_one_sided'] * (len(result['tests']) - rank)))
        test['p_value_holm_bonferroni'] = round(running, 8)
        compare(f'pairwise_tests/{index}/p_value_holm_bonferroni', test['p_value_holm_bonferroni'], x['pairwise_tests'][index]['p_value_holm_bonferroni'], 1e-9)
        assert (running < 0.025) == x['pairwise_tests'][index]['statistically_significant_alpha_0025']

    # Independently enumerate the exact untied n=10 vs n=10 null rank allocation.
    null_u = Counter(sum(ranks) - 55 for ranks in itertools.combinations(range(1, 21), 10))
    for test in result['tests']:
        if not test['ties']:
            test['exact_one_sided_p_sensitivity'] = sum(n for u, n in null_u.items() if u >= test['mann_whitney_u']) / math.comb(20, 10)

    for i, family in enumerate(x['held_out_geometry_generalization']):
        name = family['topology_class']
        candidate = x['promoted_wave3_elites_by_family'][name]
        score = kpi(candidate, x['baseline_b0_ct35_dp_u_pa'])
        compare(f'held_out_geometry_generalization/{i}/held_out_best_kpi_score', score, family['held_out_best_kpi_score'])
        count = len(discovery[name])
        assert count == family['de_novo_rediscovery_seeds']
        compare(f'held_out_geometry_generalization/{i}/de_novo_rediscovery_fraction', count / 10, family['de_novo_rediscovery_fraction'])
        result['families'].append(dict(topology_class=name, kpi=score, discovery_seeds=count,
                                       sample_count=family['sample_count'],
                                       rmse_kpi=family['sandbox_authored_family_rmse_kpi_score']))
    compare('baseline_b0_ct35_kpi_score', kpi(x['baseline_b0_ct35'], x['baseline_b0_ct35_dp_u_pa']), x['baseline_b0_ct35_kpi_score'])
    compare('promoted_wave3_top_elite_kpi_score', kpi(x['promoted_wave3_top_elite'], x['baseline_b0_ct35_dp_u_pa']), x['promoted_wave3_top_elite_kpi_score'])
    result['arithmetic_mismatches'] = mismatches
    return result


if __name__ == '__main__':
    data = json.loads(Path('docs/results/wave4-evolution-proof-v1.json').read_text())
    result = audit(data)
    print(json.dumps(result, indent=2))
    raise SystemExit(bool(result['arithmetic_mismatches']))
