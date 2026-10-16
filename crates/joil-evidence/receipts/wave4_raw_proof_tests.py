"""Synthetic functional oracles; these fixtures are never Wave-4 run evidence."""

import copy
import importlib.util
import json
import math
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('ev_raw_proof', Path(__file__).with_name('wave4_raw_proof.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


def candidate(identity, generation, cout):
    return dict(id=identity, generation_born=generation, red_team_passed=True,
                mc_robustness=dict(samples=72, cout_sample_screen_upper_mg_l=cout,
                                   grade_20um_sample_screen_lower=0),
                nominal_metrics=dict(dp_u_pa=100), constraint_violation=0)


def synthetic_runs():
    cfg = dict(master_seeds=[1, 2, 3, 4, 5], generations=1,
               population_per_generation=2, proposal_budget=4,
               quality_limit_mg_l=30, mc_samples=72)
    docs = []
    for arm in [m.B3, *m.OTHERS]:
        for seed in cfg['master_seeds']:
            gs = []
            for g, couts in enumerate([[20, 10], [5, 15]]):
                cs = [candidate(f's{seed}-g{g}-i{i}', g, c) for i, c in enumerate(couts)]
                gs.append(dict(generation=g, proposals=[dict(state='evaluated', individual=c) for c in cs],
                               active_ids=[c['id'] for c in cs], pareto_hypervolume=g+.25))
            docs.append(dict(schema='joil.wave4-raw-seed.v1', arm=arm, seed=seed,
                             campaign_id='synthetic-not-run-evidence', source_commit='synthetic-fixture',
                             objective_sha256='synthetic-not-admitted', config=cfg,
                             reference=candidate('reference', 0, 30), generation_populations=gs))
    return docs


class RawProofTests(unittest.TestCase):
    def test_known_candidate_population_oracle_and_equal_budget_counts(self):
        raw = synthetic_runs()[0]
        trace = m.seed_trace(raw)
        self.assertEqual([r['best_kpi_score'] for r in trace['generation_trace']], [20, 25])
        self.assertEqual([r['median_kpi_score'] for r in trace['generation_trace']], [15, 20])
        self.assertEqual([r['cumulative_evaluations'] for r in trace['generation_trace']], [2, 4])
        self.assertEqual(trace['budget'], 4)
        self.assertEqual(m.bootstrap([7]*5, 123)['ci95_low'], 7)
        self.assertEqual(m.bootstrap([7]*5, 123)['ci95_high'], 7)
        actual, gaps = m.recompute(synthetic_runs())
        self.assertEqual(len(actual['pairwise_tests']), 16)
        self.assertEqual(actual['pairwise_tests'][0]['mann_whitney_u'], 12.5)
        self.assertEqual(actual['pairwise_tests'][0]['cliffs_delta'], 0)
        self.assertEqual(actual['pairwise_tests'][0]['p_value_holm_bonferroni'], 1)
        self.assertIsNone(actual['ct35_equal_budget'])
        self.assertIsNone(actual['held_out_geometry_generalization'])
        self.assertTrue(any('CT35' in g for g in gaps))
        self.assertTrue(any('out-of-fold' in g for g in gaps))

    def test_rank_statistic_effect_and_holm_have_independent_hand_oracles(self):
        t = m.rank_test([3, 4], [1, 2])
        self.assertEqual(t['mann_whitney_u'], 4)
        self.assertEqual(t['cliffs_delta'], 1)
        self.assertAlmostEqual(t['z_score'], 2/math.sqrt(20/12), places=4)
        self.assertAlmostEqual(t['p_value_one_sided'], .5*math.erfc((2/math.sqrt(20/12))/math.sqrt(2)), delta=1e-7)
        tied = m.rank_test([1, 2], [2, 3])
        self.assertEqual(tied['mann_whitney_u'], .5)
        self.assertEqual(tied['cliffs_delta'], -.75)
        tests = [{'p_value_one_sided': p} for p in [.02, .001, .03]]
        m.holm(tests)
        self.assertEqual([t['p_value_holm_bonferroni'] for t in tests], [.04, .003, .04])

    def test_summary_substitution_missing_seed_and_unequal_budget_reject(self):
        with self.assertRaisesRegex(ValueError, 'summary JSON'):
            m.recompute([{'arm_outcomes': {}}])
        with self.assertRaisesRegex(ValueError, 'missing arm or seed'):
            m.recompute(synthetic_runs()[:-1])
        raw = copy.deepcopy(synthetic_runs()[0])
        raw['config']['proposal_budget'] = 3
        with self.assertRaisesRegex(ValueError, 'proposal budget'):
            m.seed_trace(raw)
        raw = copy.deepcopy(synthetic_runs()[0])
        raw['generation_populations'][0]['proposals'][0]['individual']['mc_robustness']['cout_sample_screen_upper_mg_l'] = None
        with self.assertRaisesRegex(ValueError, 'no imputation'):
            m.seed_trace(raw)

    def test_invalid_proposal_counts_budget_without_evaluating_it(self):
        raw = copy.deepcopy(synthetic_runs()[0])
        g = raw['generation_populations'][0]
        g['proposals'][0] = dict(state='invalid', individual=None)
        g['active_ids'] = [g['active_ids'][1]]
        out = m.seed_trace(raw)
        self.assertEqual(out['budget'], 4)
        self.assertEqual(out['proposals_invalid'], 1)
        self.assertEqual(out['evaluations'], 3)
        self.assertEqual(out['generation_trace'][-1]['cumulative_evaluations'], 3)

    def test_holdout_raw_residual_oracle_null_kpi_and_leakage(self):
        fold = dict(topology_class='synthetic-family', train_subject_ids=['train'],
                    evaluated_candidates=[candidate('test-a', 0, 5)], predictions=[
                        dict(subject_id='test-a', target_cout=10, predicted_cout=13, sigma_cout=1,
                             target_kpi=None, predicted_kpi=None),
                        dict(subject_id='test-b', target_cout=10, predicted_cout=14, sigma_cout=2,
                             target_kpi=None, predicted_kpi=None)])
        raw = dict(split_basis='authored_family_screen', folds=[fold])
        row = m.holdout_folds(raw, 100, 30)[0]
        self.assertAlmostEqual(row['sandbox_authored_family_rmse_cout_mg_l'], math.sqrt(12.5))
        self.assertEqual(row['sandbox_authored_family_2sigma_coverage'], .5)
        self.assertIsNone(row['sandbox_authored_family_rmse_kpi_score'])
        actual, _ = m.recompute(synthetic_runs())
        actual['held_out_geometry_generalization'] = [row]
        summary = json.loads(json.dumps(actual))
        summary['held_out_geometry_generalization'][0]['sandbox_authored_family_rmse_kpi_score'] = 0
        diff = m.diff_summary(actual, summary)['numeric_or_structural_mismatches']
        self.assertIn('/held_out_geometry_generalization/0/sandbox_authored_family_rmse_kpi_score',
                      [d['pointer'] for d in diff])
        fold['train_subject_ids'].append('test-a')
        with self.assertRaisesRegex(ValueError, 'leaking holdout'):
            m.holdout_folds(raw, 100, 30)

    def test_summary_mutations_detect_exact_mean_and_small_p_value_pointers(self):
        actual, _ = m.recompute(synthetic_runs())
        summary = json.loads(json.dumps(actual))
        for outs in summary['arm_outcomes'].values():
            for o in outs:
                for f in m.FIELDS:
                    o['final_'+f] = o['generation_trace'][-1][f]
        self.assertFalse(m.diff_summary(actual, summary)['numeric_or_structural_mismatches'])
        summary['final_summary_statistics'][m.B3]['best_kpi_score']['mean'] += 1
        summary['pairwise_tests'][0]['p_value_one_sided'] += 1e-7
        diff = m.diff_summary(actual, summary)['numeric_or_structural_mismatches']
        self.assertEqual({r['pointer'] for r in diff}, {
            '/final_summary_statistics/'+m.B3+'/best_kpi_score/mean',
            '/pairwise_tests/0/p_value_one_sided'})

    def test_missing_raw_cli_report_is_blocked_and_has_no_recomputed_numbers(self):
        root = Path(__file__).resolve().parents[3]
        (root/'.private').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root/'.private') as directory:
            raw = Path(directory)
            summary = raw/'summary.json'
            summary.write_text(json.dumps({'final_summary_statistics': {'invented': 12345}}))
            out = m.audit(raw, summary)
            self.assertEqual(out['state'], 'BLOCKED')
            self.assertIsNone(out['recomputed'])
            self.assertIsNone(out['comparison'])
            (raw/'wave4-proof-raw-summary-substitution.json').write_text(summary.read_text())
            out = m.audit(raw, summary)
            self.assertEqual(out['state'], 'BLOCKED')
            self.assertIn('summary JSON', out['gaps'][0])

    def test_extra_ct35_arm_stays_out_of_legacy_summary_diff(self):
        docs = synthetic_runs()
        original, _ = m.recompute(docs)
        extra = [copy.deepcopy(r) for r in docs if r['arm'] == m.B3]
        for r in extra:
            r['arm'] = m.B0
        actual, _ = m.recompute(docs+extra)
        summary = json.loads(json.dumps(original))
        for outs in summary['arm_outcomes'].values():
            for o in outs:
                for f in m.FIELDS:
                    o['final_'+f] = o['generation_trace'][-1][f]
        self.assertIn(m.B0, actual['generation_band_curves'])
        self.assertEqual(actual['ct35_equal_budget']['mann_whitney_u'], 12.5)
        self.assertFalse(m.diff_summary(actual, summary)['numeric_or_structural_mismatches'])

    def test_unfitted_family_is_explicit_and_holdouts_match_by_family(self):
        row = dict(topology_class='synthetic-b', train_subject_ids=['train'],
                   evaluated_candidates=[candidate('candidate-b', 0, 5)], predictions=[
                       dict(subject_id='test-b', target_cout=10, predicted_cout=11, sigma_cout=1,
                            target_kpi=None, predicted_kpi=None)])
        absent = dict(topology_class='synthetic-a', state='insufficient', train_subject_ids=[],
                      predictions=[], evaluated_candidates=[candidate('candidate-a', 0, 6)])
        folds = m.holdout_folds(dict(split_basis='authored_family_screen', folds=[row, absent]), 100, 30)
        self.assertEqual(folds[1]['sample_count'], 0)
        self.assertIsNone(folds[1]['sandbox_authored_family_rmse_cout_mg_l'])
        actual, _ = m.recompute(synthetic_runs())
        actual['held_out_geometry_generalization'] = folds
        summary = json.loads(json.dumps(actual))
        for outs in summary['arm_outcomes'].values():
            for o in outs:
                for f in m.FIELDS:
                    o['final_'+f] = o['generation_trace'][-1][f]
        summary['held_out_geometry_generalization'].reverse()
        result = m.diff_summary(actual, summary)
        self.assertFalse(result['numeric_or_structural_mismatches'])
        self.assertFalse(result['coverage_gaps'])
        summary['held_out_geometry_generalization'].append(dict(topology_class='synthetic-missing'))
        result = m.diff_summary(actual, summary)
        self.assertFalse(result['numeric_or_structural_mismatches'])
        self.assertEqual(len(result['coverage_gaps']), 1)


if __name__ == '__main__':
    unittest.main()
