"""Recompute Wave-4 from retained candidate/population and prediction records.

No Joil evaluator or summary-audit imports. The summary is parsed only for the
final diff. This consumer's raw capture contract is NEW, not evidence that the
original runner retained these files. An owner may adapt an existing raw export
to it while preserving the original files and recording the adapter provenance.

--raw-root contains wave4-proof-raw-*.json files with these schemas:
  joil.wave4-raw-seed.v1: arm, seed, campaign_id, source_commit, objective_sha256,
    config {master_seeds, generations, population_per_generation, proposal_budget,
    quality_limit_mg_l, mc_samples}, reference (original Individual), generation_populations.
    Each population has generation, proposals [{state: invalid|repaired|evaluated,
    individual: null|original Individual}], active_ids, pareto_hypervolume.
    Retain all evaluated Individuals, including failures, and actual survival IDs.
  joil.wave4-raw-reference.v1: candidate (the one original B0 evaluation).
  joil.wave4-raw-holdouts.v1: split_basis, folds [{topology_class, train_subject_ids,
    predictions [{subject_id, target_cout, predicted_cout, sigma_cout,
    target_kpi, predicted_kpi}], evaluated_candidates: [original Individual]}].
    Null KPI predictions remain null. Authored-family folds are not blind geometry
    validation. Keep train/test geometry grouping and independent labels separately.

Record exact source hashes, model/config identities and raw MC observations in
the retained export. This script starts from emitted candidate quantiles, not MC
sample generation. Archive-volume telemetry is used only for the four HV tests
in the published 16-test Holm family; it is not independently re-integrated here.
Exit 0: supplied raw inputs recomputed and requested coverage complete, no diff.
Exit 1: numeric/structural mismatch. Exit 2: missing/invalid raw data or coverage.
"""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
from pathlib import Path
import statistics

import numpy as np

B3 = 'B3_Wave3_Exotic3D_QD_CEI'
B1 = 'B1_Random_Search'
B0 = 'B0_CT35_Equal_Budget'
OTHERS = [B1, 'B2_Conventional_Axisym_NSGA2',
          'A1_No_Mutation_Crossover_Only', 'A2_No_Selection_Random_Walk']
FIELDS = ['best_kpi_score', 'median_kpi_score', 'best_cout_sample_screen_upper_mg_l',
          'median_cout_sample_screen_upper_mg_l', 'pareto_hypervolume']
FORBIDDEN = {'arm_outcomes', 'generation_band_curves', 'final_summary_statistics', 'pairwise_tests'}


def number(x):
    if isinstance(x, bool) or not isinstance(x, (int, float)) or not math.isfinite(x):
        raise ValueError('missing/nonfinite numeric input; no imputation')
    return float(x)


def rounded(x, digits=6):
    scale = 10**digits
    fraction, integer = math.modf(abs(x)*scale)
    return math.copysign((integer+(fraction >= .5))/scale, x)


def ordered_sum(values):
    """The declared Rust statistic accumulates f64 values left to right."""
    total = 0.0
    for value in values:
        total += float(value)
    return total


def percentile_sorted(values, q):
    if not len(values):
        raise ValueError('empty percentile sample; no imputation')
    position = q*(len(values)-1)
    lo, hi = math.floor(position), math.ceil(position)
    fraction = position-lo
    # Preserve both multiplications: NumPy's lerp differs at rounding boundaries.
    return float(values[lo])*(1.0-fraction)+float(values[hi])*fraction


def kpi(ind, dp_ref, limit):
    mc = ind['mc_robustness']
    return rounded(limit-number(mc['cout_sample_screen_upper_mg_l'])
                   + 25*number(mc['grade_20um_sample_screen_lower'])
                   + 10*(dp_ref-number(ind['nominal_metrics']['dp_u_pa']))/dp_ref
                   - 500*number(ind['constraint_violation']))


def bootstrap(values, seed):
    if not values:
        raise ValueError('empty seed sample; cannot estimate CI')
    a = np.array([number(x) for x in values])
    n = len(a)
    with np.errstate(over='ignore'):
        z = np.uint64(seed)+np.uint64(0x9E3779B97F4A7C15)*np.arange(1, 10000*n+1, dtype=np.uint64)
        z = (z ^ (z >> 30))*np.uint64(0xBF58476D1CE4E5B9)
        z = (z ^ (z >> 27))*np.uint64(0x94D049BB133111EB)
        idx = ((z ^ (z >> 31)) % np.uint64(n)).astype(int).reshape(10000, n)
    means = np.zeros(10000)
    for column in range(n):
        means += a[idx[:, column]]
    means.sort()
    means /= n
    mean = ordered_sum(a)/n
    sorted_a = sorted(a)
    return dict(n_seeds=n, mean=rounded(mean),
                std_dev=rounded(math.sqrt(ordered_sum((float(x)-mean)**2 for x in a)/(n-1))) if n > 1 else 0.0,
                median=rounded(percentile_sorted(sorted_a, .5)),
                iqr=rounded(percentile_sorted(sorted_a, .75)-percentile_sorted(sorted_a, .25)),
                ci95_low=rounded(percentile_sorted(means, .025)),
                ci95_high=rounded(percentile_sorted(means, .975)))


def rank_test(a, b, smaller=False):
    if not a or not b:
        raise ValueError('rank test requires two nonempty independent samples')
    differences = [(-1 if smaller else 1)*(number(x)-number(y)) for x in a for y in b]
    wins, ties = sum(d > 0 for d in differences), sum(d == 0 for d in differences)
    u = wins+ties/2
    z = (u-len(a)*len(b)/2)/math.sqrt(len(a)*len(b)*(len(a)+len(b)+1)/12)
    # Reproduce the declared normal approximation, without continuity/tie correction.
    t = 1/(1+.3275911*abs(z/math.sqrt(2)))
    tail = (((((1.061405429*t-1.453152027)*t+1.421413741)*t-.284496736)*t
             + .254829592)*t*math.exp(-z*z/2))
    return dict(mean_diff=rounded(ordered_sum(a)/len(a)-ordered_sum(b)/len(b)),
                median_diff=rounded(percentile_sorted(sorted(a), .5)-percentile_sorted(sorted(b), .5)),
                mann_whitney_u=u, z_score=rounded(z, 4),
                p_value_one_sided=rounded(.5*(tail if z >= 0 else 2-tail), 8),
                cliffs_delta=rounded((2*wins+ties-len(differences))/len(differences), 4))


def holm(tests):
    running = 0.0
    for rank, i in enumerate(sorted(range(len(tests)), key=lambda j: tests[j]['p_value_one_sided'])):
        running = max(running, min(1, tests[i]['p_value_one_sided']*(len(tests)-rank)))
        tests[i]['p_value_holm_bonferroni'] = rounded(running, 8)
        tests[i]['statistically_significant_alpha_0025'] = running < .025


def seed_trace(raw):
    """Scores come from raw Individuals; active membership comes from the run log."""
    cfg, arm = raw['config'], raw['arm']
    reference = raw['reference']
    if reference['mc_robustness']['samples'] != cfg['mc_samples']:
        raise ValueError('reference MC configuration differs')
    limit = number(cfg['quality_limit_mg_l'])
    dp_ref = max(1., number(reference['nominal_metrics']['dp_u_pa']))
    best = reference
    subjects = {reference['id']: reference}
    counts = Counter()
    trace = []
    generations = raw['generation_populations']
    if [g['generation'] for g in generations] != list(range(cfg['generations']+1)):
        raise ValueError('missing, duplicate or unordered generation populations')
    for g in generations:
        if len(g['proposals']) != cfg['population_per_generation']:
            raise ValueError('missing charged proposal records')
        for p in g['proposals']:
            state, candidate = p['state'], p['individual']
            if state not in {'invalid', 'repaired', 'evaluated'}:
                raise ValueError('unrecognized proposal state')
            counts[state] += 1
            if state == 'invalid':
                if candidate is not None:
                    raise ValueError('invalid proposal has evaluated subject')
                continue
            if candidate is None or candidate['id'] in subjects:
                raise ValueError('missing or duplicate evaluated subject')
            if candidate['generation_born'] != g['generation']:
                raise ValueError('candidate belongs to another generation')
            if not isinstance(candidate['red_team_passed'], bool):
                raise ValueError('missing typed red-team result')
            if candidate['mc_robustness']['samples'] != cfg['mc_samples']:
                raise ValueError('candidate MC configuration differs')
            kpi(candidate, dp_ref, limit)  # Reject incomplete observations, including failed ones.
            subjects[candidate['id']] = candidate
            if candidate['red_team_passed'] and kpi(candidate, dp_ref, limit) > kpi(best, dp_ref, limit)+1e-9:
                best = candidate
        ids = g['active_ids']
        if not ids or len(ids) != len(set(ids)):
            raise ValueError('missing or duplicated active population membership')
        active = [subjects[i] for i in ids]
        if arm == 'A2_No_Selection_Random_Walk' and g['generation'] > 0:
            valid = [i for i in active if i['red_team_passed']]
            if valid:
                best = max(valid, key=lambda i: kpi(i, dp_ref, limit))
        trace.append(dict(
            generation=g['generation'], cumulative_evaluations=counts['evaluated']+counts['repaired'],
            best_kpi_score=kpi(best, dp_ref, limit),
            median_kpi_score=rounded(statistics.median(kpi(i, dp_ref, limit) for i in active)),
            best_cout_sample_screen_upper_mg_l=number(best['mc_robustness']['cout_sample_screen_upper_mg_l']),
            median_cout_sample_screen_upper_mg_l=rounded(statistics.median(
                number(i['mc_robustness']['cout_sample_screen_upper_mg_l']) for i in active)),
            pareto_hypervolume=number(g['pareto_hypervolume'])))
    if sum(counts.values()) != cfg['proposal_budget']:
        raise ValueError('unequal/incomplete charged proposal budget')
    return dict(seed=raw['seed'], generation_trace=trace, budget=sum(counts.values()),
                proposals_invalid=counts['invalid'], proposals_repaired=counts['repaired'],
                evaluations=counts['evaluated'])


def holdout_folds(raw, dp_ref, limit):
    rows = []
    if raw['split_basis'] not in {'authored_family_screen', 'observed_geometry_holdout'}:
        raise ValueError('unknown holdout grouping basis')
    for f in raw['folds']:
        predictions = f['predictions']
        if f.get('state') == 'insufficient':
            if predictions:
                raise ValueError('insufficient fold must not invent prediction rows')
            pool = f['evaluated_candidates']
            rows.append(dict(topology_class=f['topology_class'], sample_count=0,
                             sandbox_authored_family_rmse_cout_mg_l=None,
                             sandbox_authored_family_rmse_kpi_score=None,
                             sandbox_authored_family_2sigma_coverage=None,
                             held_out_best_kpi_score=max((kpi(i, dp_ref, limit) for i in pool), default=None)))
            continue
        ids = [p['subject_id'] for p in predictions]
        if not ids or len(ids) != len(set(ids)) or set(ids) & set(f['train_subject_ids']):
            raise ValueError('empty, duplicate or leaking holdout predictions')
        if raw['split_basis'] == 'observed_geometry_holdout':
            train = f['train_geometry_hashes']
            test = [p['geometry_hash'] for p in predictions]
            if not train or not all(train+test) or set(train) & set(test):
                raise ValueError('missing/leaking geometry holdout groups')
        residuals = [number(p['predicted_cout'])-number(p['target_cout']) for p in predictions]
        sigmas = [number(p['sigma_cout']) for p in predictions]
        if any(s < 0 for s in sigmas):
            raise ValueError('negative predictive uncertainty')
        pairs = [(p['predicted_kpi'], p['target_kpi']) for p in predictions]
        if all(x is None and y is None for x, y in pairs):
            kpi_rmse = None
        else:
            kpi_rmse = math.sqrt(statistics.mean((number(x)-number(y))**2 for x, y in pairs))
        if not f['evaluated_candidates']:
            raise ValueError('missing evaluated family candidate pool')
        rows.append(dict(topology_class=f['topology_class'], sample_count=len(ids),
                         sandbox_authored_family_rmse_cout_mg_l=math.sqrt(statistics.mean(x*x for x in residuals)),
                         sandbox_authored_family_rmse_kpi_score=kpi_rmse,
                         sandbox_authored_family_2sigma_coverage=sum(abs(e) <= 2*s for e, s in zip(residuals, sigmas))/len(ids),
                         held_out_best_kpi_score=max(kpi(i, dp_ref, limit) for i in f['evaluated_candidates'])))
    return rows


def recompute(documents):
    runs, reference, holdouts = [], None, None
    for doc in documents:
        if not isinstance(doc, dict):
            raise ValueError('raw document must be a JSON object')
        if FORBIDDEN & doc.keys():
            raise ValueError('summary JSON is not raw-independent input')
        schema = doc['schema']
        if schema == 'joil.wave4-raw-seed.v1':
            runs.append(doc)
        elif schema == 'joil.wave4-raw-reference.v1' and reference is None:
            reference = doc['candidate']
        elif schema == 'joil.wave4-raw-holdouts.v1' and holdouts is None:
            holdouts = doc
        else:
            raise ValueError('unknown/duplicate raw input schema')
    if not runs:
        raise ValueError('no retained raw Wave-4 candidate/population exports supplied')
    cfg = runs[0]['config']
    for key in ['generations', 'population_per_generation', 'proposal_budget', 'mc_samples']:
        if isinstance(cfg[key], bool) or not isinstance(cfg[key], int) or cfg[key] < (0 if key == 'generations' else 1):
            raise ValueError('invalid raw integer configuration')
    if cfg['proposal_budget'] != (cfg['generations']+1)*cfg['population_per_generation']:
        raise ValueError('configured proposal budget does not cover every generation')
    seeds = cfg['master_seeds']
    if len(seeds) < 5 or len(seeds) != len(set(seeds)):
        raise ValueError('missing/duplicate independent master seeds')
    grouped = defaultdict(dict)
    for raw in runs:
        if raw['config'] != cfg or raw['seed'] not in seeds:
            raise ValueError('run configuration or seed differs')
        for key in ['campaign_id', 'source_commit', 'objective_sha256']:
            if not raw[key] or raw[key] != runs[0][key]:
                raise ValueError('mixed/missing run provenance')
        arm = raw['arm']
        if arm not in [B3, B0, *OTHERS] or raw['seed'] in grouped[arm]:
            raise ValueError('unknown arm or duplicated seed run')
        grouped[arm][raw['seed']] = seed_trace(raw)
    if not {B3, *OTHERS} <= grouped.keys() or any(set(v) != set(seeds) for v in grouped.values()):
        raise ValueError('missing arm or seed; cannot reproduce 16-test Holm family')
    outcomes = {a: [v[s] for s in seeds] for a, v in grouped.items()}
    bands = {}
    for arm, outs in outcomes.items():
        bands[arm] = []
        for g in range(cfg['generations']+1):
            rows = [o['generation_trace'][g] for o in outs]
            if len({r['cumulative_evaluations'] for r in rows}) != 1:
                raise ValueError('different evaluated budgets across seed bands; cannot use one x-axis')
            bands[arm].append({
                'generation': g, 'cumulative_evaluations': rows[0]['cumulative_evaluations'],
                **{field: bootstrap([r[field] for r in rows], 20269100+100*j+g)
                   for j, field in enumerate(FIELDS)}})
    tests = []
    metrics = ['final_best_kpi_score', 'final_median_kpi_score',
               'final_best_cout_sample_screen_upper_mg_l', 'final_pareto_hypervolume']
    for other in OTHERS:
        for metric in metrics:
            field = metric.removeprefix('final_')
            a = [o['generation_trace'][-1][field] for o in outcomes[B3]]
            b = [o['generation_trace'][-1][field] for o in outcomes[other]]
            tests.append(dict(metric=metric, comparator_a=B3, comparator_b=other,
                              **rank_test(a, b, smaller='cout' in field)))
    holm(tests)
    gaps = []
    result = dict(arm_outcomes=outcomes, generation_band_curves=bands,
                  source_provenance={k: runs[0][k] for k in ['campaign_id', 'source_commit', 'objective_sha256']},
                  raw_configuration=cfg,
                  final_summary_statistics={a: {k: b[-1][k] for k in FIELDS} for a, b in bands.items()},
                  pairwise_tests=tests, held_out_geometry_generalization=None,
                  baseline_b0_ct35_kpi_score=None, ct35_equal_budget=None)
    if B0 not in outcomes:
        gaps.append('No independent equal-budget CT35 seed arm; reference cannot supply CI/test.')
    else:
        a = [o['generation_trace'][-1]['best_kpi_score'] for o in outcomes[B3]]
        b = [o['generation_trace'][-1]['best_kpi_score'] for o in outcomes[B0]]
        result['ct35_equal_budget'] = rank_test(a, b)
    if reference is None:
        gaps.append('Original B0 reference evaluation absent from raw export.')
    else:
        dp_ref = max(1., number(reference['nominal_metrics']['dp_u_pa']))
        result['baseline_b0_ct35_kpi_score'] = kpi(reference, dp_ref, number(cfg['quality_limit_mg_l']))
        if holdouts is not None:
            result['held_out_geometry_generalization'] = holdout_folds(
                holdouts, dp_ref, number(cfg['quality_limit_mg_l']))
            for f in holdouts['folds']:
                if f.get('state') == 'insufficient':
                    gaps.append('Insufficient raw fold predictions: '+f['topology_class'])
    if holdouts is None:
        gaps.append('Raw out-of-fold targets/predictions absent; no held-out RMSE can be recomputed.')
    elif holdouts['split_basis'] != 'observed_geometry_holdout':
        gaps.append('Provided folds are authored-family screen diagnostics, not independent geometry holdouts.')
    return result, gaps


def diff_summary(actual, summary):
    """Compare only independently recomputed coverage, with exact JSON pointers."""
    diffs, compared, coverage_gaps = [], 0, []
    def visit(a, b, pointer):
        nonlocal compared
        if isinstance(a, dict):
            for k, v in a.items():
                visit(v, b.get(k) if isinstance(b, dict) else None, pointer+'/'+k)
        elif isinstance(a, list):
            if pointer == '/held_out_geometry_generalization' and isinstance(b, list):
                actual_families = {r['topology_class'] for r in a}
                expected = {r['topology_class']: (i, r) for i, r in enumerate(b)}
                if len(actual_families) != len(a) or len(expected) != len(b):
                    raise ValueError('duplicated holdout family rows')
                for family in expected.keys()-actual_families:
                    coverage_gaps.append(pointer+'/'+str(expected[family][0])+' missing raw family '+family)
                for row in a:
                    family = row['topology_class']
                    if family not in expected:
                        diffs.append(dict(pointer=pointer, unexpected_raw_family=family))
                    else:
                        i, recorded = expected[family]
                        visit(row, recorded, pointer+'/'+str(i))
                return
            if not isinstance(b, list) or len(a) != len(b):
                diffs.append(dict(pointer=pointer, recomputed_length=len(a), recorded_length=len(b) if isinstance(b, list) else None))
            else:
                for i, v in enumerate(a): visit(v, b[i], pointer+'/'+str(i))
        else:
            compared += 1
            if isinstance(a, (int, float)) and not isinstance(a, bool):
                tolerance = 5.1e-9 if '/p_value_' in pointer else 5.1e-7
                match = isinstance(b, (int, float)) and not isinstance(b, bool) and math.isclose(a, b, rel_tol=1e-10, abs_tol=tolerance)
            else:
                match = a == b
            if not match: diffs.append(dict(pointer=pointer, recomputed=a, recorded=b))
    # Run outputs do not contain the summary's final_* aliases; compare them too.
    # An unavailable section has no comparison. A recomputed null KPI RMSE is
    # compared as null, so a summary's invented zero is detected.
    projection = {k: v for k, v in actual.items()
                  if k not in {'ct35_equal_budget', 'source_provenance', 'raw_configuration'} and v is not None}
    for key in ['generation_band_curves', 'final_summary_statistics']:
        projection[key] = {a: v for a, v in projection[key].items() if a != B0}
    projection['arm_outcomes'] = {}
    for arm, outs in actual['arm_outcomes'].items():
        if arm == B0: continue
        projection['arm_outcomes'][arm] = [dict(
            o, **{'final_'+f: o['generation_trace'][-1][f] for f in FIELDS
                  if f != 'median_cout_sample_screen_upper_mg_l'}) for o in outs]
    # Match raw-seed ordering by identity, not by incidental file order.
    target = dict(summary)
    target['arm_outcomes'] = {a: sorted(v, key=lambda o: [x['seed'] for x in projection['arm_outcomes'][a]].index(o['seed']))
                              for a, v in summary['arm_outcomes'].items() if a in projection['arm_outcomes']}
    visit(projection, target, '')
    return dict(compared_leaves=compared, numeric_or_structural_mismatches=diffs,
                coverage_gaps=sorted(coverage_gaps))


def audit(raw_root, summary_path):
    raw_root, summary_path = Path(raw_root), Path(summary_path)
    report = dict(kind='raw_independent_wave4_audit_not_physical_validation', state='BLOCKED',
                  raw_sources=[], summary_sha256=None,
                  recomputed=None, comparison=None, gaps=[])
    try:
        summary_bytes = summary_path.read_bytes()
        report['summary_sha256'] = hashlib.sha256(summary_bytes).hexdigest()
        if not raw_root.is_dir(): raise ValueError('raw root is not a directory')
        docs = []
        for path in sorted(raw_root.rglob('wave4-proof-raw-*.json')):
            if path.is_symlink(): raise ValueError('raw source symlink rejected')
            data = path.read_bytes()
            report['raw_sources'].append(dict(path=str(path.relative_to(raw_root)), sha256=hashlib.sha256(data).hexdigest()))
            docs.append(json.loads(data))
        actual, gaps = recompute(docs)
        report['recomputed'], report['gaps'] = actual, gaps
        summary = json.loads(summary_bytes)
        if not isinstance(summary, dict): raise ValueError('summary must be a JSON object')
        report['comparison'] = diff_summary(actual, summary)
        report['gaps'].extend(report['comparison']['coverage_gaps'])
        report['state'] = 'MISMATCH' if report['comparison']['numeric_or_structural_mismatches'] else ('BLOCKED' if gaps else 'PASS')
    except (ValueError, KeyError, TypeError, OSError) as exc:
        report['gaps'].append(str(exc))
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--raw-root', type=Path, required=True)
    parser.add_argument('--summary', type=Path, default=Path('docs/results/wave4-evolution-proof-v1.json'))
    args = parser.parse_args()
    result = audit(args.raw_root, args.summary)
    print(json.dumps(result, indent=2, allow_nan=False))
    raise SystemExit({'PASS': 0, 'MISMATCH': 1, 'BLOCKED': 2}[result['state']])
