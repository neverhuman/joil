"""V5-01 truth repairs: production-path regressions for corrected claim semantics.

Every mutation below is a deep copy labelled SYNTHETIC. No test asserts an improvement
threshold or a physical result; negatives assert their specific failure reason.
"""
import copy
import hashlib
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools import run_elite_3grid_gci_and_rsm as gate  # noqa: E402

GATE_TOOL = ROOT / 'tools/run_elite_3grid_gci_and_rsm.py'
PREREG = ROOT / 'experiments/wave2-preregistration-v1.json'
W1_V1 = ROOT / 'docs/results/elite-3grid-gci-and-rsm-verification-v1.json'
W1_V2 = ROOT / 'docs/results/elite-3grid-gci-and-rsm-verification-v2.json'
W2_V1 = ROOT / 'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json'
W2_V2 = ROOT / 'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json'
W4 = ROOT / 'docs/results/wave4-gci-v1.json'
W2_KEYS = ('wave2_tpms_gyroid_elite_5run_dossier', 'wave2_octa_branch_8jet_elite_5run_dossier')


def load(path):
    return json.loads(path.read_text(encoding='utf-8'))


def sha(data):
    return hashlib.sha256(data).hexdigest()


def legacy_dossiers():
    w2v1, w2v2 = load(W2_V1), load(W2_V2)
    return [('w1_v1', load(W1_V1)), ('w1_v2', load(W1_V2))] + \
        [(f'w2_v1_{k}', w2v1[k]) for k in W2_KEYS] + [(f'w2_v2_{k}', w2v2[k]) for k in W2_KEYS]


def synthetic_same_pipeline(offset_pa=400.0):
    """SYNTHETIC candidate/comparator pair from the v2 Wave-1 template with one bound pipeline."""
    d = copy.deepcopy(load(W1_V2))
    ident = {k: sha(f'SYNTHETIC-{k}'.encode()) for k in gate.PIPELINE_IDENTITY_KEYS}
    subject, control = sha(b'SYNTHETIC-candidate'), sha(b'SYNTHETIC-ct35-control')
    d['synthetic_fixture'] = 'SYNTHETIC V5-01-01 same-pipeline positive control; not a physical result'
    d.pop('mandatory_promotion_gate')
    d['subject_design_sha256'] = subject
    d['openfoam_version'] = 'Foundation-14'
    d['pipeline_identity'] = dict(ident)
    del d['roache_gci_analysis']['signal_to_uncertainty_margin']
    cmp_grids = []
    for g, dp, el in zip(d['grids'], (18600.0, 18700.0, 18650.0), (21000.0, 21100.0, 21050.0)):
        g.update(dp_u_pa=dp, hydraulic_loss_j_per_treated_m3=el, subject_design_sha256=subject)
        cmp_grids.append({'grid_id': g['grid_id'], 'background_hex_grid': list(g['background_hex_grid']),
                          'subject_design_sha256': control, 'reject_fraction': g['reject_fraction'],
                          'water_closure_fraction': g['water_closure_fraction'],
                          'dp_u_pa': dp + offset_pa, 'hydraulic_loss_j_per_treated_m3': el + offset_pa})
    d['baseline_ct35_comparator'] = {'synthetic_fixture': 'SYNTHETIC same-pipeline CT-35 control',
                                     'subject_design_sha256': control, 'openfoam_version': 'Foundation-14',
                                     'pipeline_identity': dict(ident), 'grids': cmp_grids}
    for e in d['rsm_turbulence_closure_crosscheck']:
        e['delta_dp_u_vs_ct35_baseline_pct'] = -2.0
    return d


def run_cli(*args):
    return subprocess.run([sys.executable, '-I', str(GATE_TOOL), *args], capture_output=True, text=True, cwd=str(ROOT))


class EliteCfdGateConfigTruth(unittest.TestCase):
    def setUp(self):
        self.prereg = load(PREREG)

    def assertConfigError(self, prereg, needle):
        with self.assertRaises(gate.GateConfigError) as ctx:
            gate.load_elite_cfd_gate_config(prereg)
        self.assertIn(needle, str(ctx.exception))

    def test_registered_block_values_are_read(self):
        cfg = gate.load_elite_cfd_gate_config(self.prereg)
        block = self.prereg['mandatory_elite_cfd_gates']
        self.assertEqual(cfg['min_signal_to_3grid_span_ratio'], block['min_signal_to_3grid_span_ratio'])
        self.assertEqual(cfg['rsm_closures_required'], block['rsm_closures_required'])
        self.assertEqual(cfg['max_inter_rsm_dp_discrepancy_pct'], block['max_inter_rsm_dp_discrepancy_pct'])
        self.assertEqual(cfg['reject_split_band'], self.prereg['hard_feasibility_gates']['reject_split_band'])
        self.assertEqual(cfg['provenance']['protocol_id'], 'joil-wave2-preregistration-v1')
        self.assertEqual(cfg['provenance']['config_key'], 'mandatory_elite_cfd_gates')
        canonical = json.dumps(block, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
        self.assertEqual(cfg['provenance']['block_sha256'], sha(canonical))
        # SYNTHETIC: an impossible registered threshold now changes the outcome of a passing control.
        strict = copy.deepcopy(self.prereg)
        strict['mandatory_elite_cfd_gates']['min_signal_to_3grid_span_ratio'] = 1.0e6
        res = gate.evaluate_elite_gci_rsm_gate(synthetic_same_pipeline(), strict)
        self.assertEqual(res['admission'], 'not_admitted')
        self.assertTrue(any('1000000.000' in f and 'dp_u signal-to-span' in f for f in res['failures']), res['failures'])

    def test_missing_or_misspelled_config_fails_closed(self):
        for prereg in (None, {}, [], 'mandatory_elite_cfd_gates'):
            self.assertConfigError(prereg, 'mandatory_elite_cfd_gates')
        misspelled = copy.deepcopy(self.prereg)
        misspelled['mandatory_elite_cfd_gate'] = misspelled.pop('mandatory_elite_cfd_gates')
        self.assertConfigError(misspelled, 'mandatory_elite_cfd_gates')
        old_code_key = copy.deepcopy(self.prereg)
        old_code_key['mandatory_elite_gci_and_rsm_gate'] = old_code_key.pop('mandatory_elite_cfd_gates')
        self.assertConfigError(old_code_key, 'mandatory_elite_cfd_gates')
        with self.assertRaises(gate.GateConfigError):
            gate.evaluate_elite_gci_rsm_gate(load(W1_V1), misspelled)
        no_protocol = copy.deepcopy(self.prereg)
        del no_protocol['protocol_id']
        self.assertConfigError(no_protocol, 'protocol_id')

    def test_each_required_key_missing_or_unknown_key_fails(self):
        for key in sorted(gate.REQUIRED_GATE_KEYS):
            bad = copy.deepcopy(self.prereg)
            del bad['mandatory_elite_cfd_gates'][key]
            self.assertConfigError(bad, key)
        unknown = copy.deepcopy(self.prereg)
        unknown['mandatory_elite_cfd_gates']['synthetic_unknown_gate'] = 1.0
        self.assertConfigError(unknown, 'synthetic_unknown_gate')
        no_band = copy.deepcopy(self.prereg)
        del no_band['hard_feasibility_gates']['reject_split_band']
        self.assertConfigError(no_band, 'reject_split_band')

    def test_malformed_config_values_fail(self):
        block = 'mandatory_elite_cfd_gates'
        cases = [
            ('min_signal_to_3grid_span_ratio', math.nan), ('min_signal_to_3grid_span_ratio', math.inf),
            ('min_signal_to_3grid_span_ratio', -math.inf), ('min_signal_to_3grid_span_ratio', True),
            ('min_signal_to_3grid_span_ratio', '3.0'), ('min_signal_to_3grid_span_ratio', 0.0),
            ('min_signal_to_3grid_span_ratio', -3.0),
            ('min_rsm_normal_stress_realizability', 2.0), ('min_rsm_normal_stress_realizability', 0.0),
            ('max_inter_rsm_dp_discrepancy_pct', -1.0), ('max_inter_rsm_dp_discrepancy_pct', None),
            ('rsm_closures_required', []), ('rsm_closures_required', ['LRR', 'LRR']),
            ('rsm_closures_required', 'LRR'), ('rsm_closures_required', ['LRR', '']),
            ('openfoam_version', ''), ('openfoam_version', 14),
            ('three_grid_levels', ['G0_coarse_140k', 'G1_medium_305k']),
            ('three_grid_levels', ['G0_coarse_140k', 'G0_medium_305k', 'G2_fine_550k']),
            ('three_grid_levels', ['coarse', 'medium', 'fine']),
        ]
        for key, value in cases:
            bad = copy.deepcopy(self.prereg)
            bad[block][key] = value
            with self.subTest(key=key, value=value):
                self.assertConfigError(bad, key)
        for band in ([0.0215, 0.0185], [0.02, 0.02], [-0.1, 0.02], [0.0185, 1.0], [0.0185], [0.0185, math.nan],
                     [0.0185, True], '0.0185,0.0215'):
            bad = copy.deepcopy(self.prereg)
            bad['hard_feasibility_gates']['reject_split_band'] = band
            with self.subTest(band=band):
                self.assertConfigError(bad, 'reject_split_band')


class EliteCfdGateAdmissionTruth(unittest.TestCase):
    def setUp(self):
        self.prereg = load(PREREG)

    def evaluate(self, dossier, **kw):
        return gate.evaluate_elite_gci_rsm_gate(dossier, self.prereg, **kw)

    def test_same_pipeline_positive_control_admitted_without_improvement_threshold(self):
        d = synthetic_same_pipeline()
        res = self.evaluate(d)
        self.assertEqual(res['failures'], [])
        self.assertEqual(res['admission'], 'admitted')
        self.assertTrue(res['gate_passed'])
        self.assertAlmostEqual(res['checks']['saving_to_3grid_span_ratio_dp_u'], 4.0, places=12)
        self.assertAlmostEqual(res['checks']['saving_to_3grid_span_ratio_eloss'], 4.0, places=12)
        self.assertEqual(res['checks']['comparator_minus_candidate_finest_dp_u_pa'], 400.0)
        self.assertEqual(res['gate_config']['protocol_id'], 'joil-wave2-preregistration-v1')
        self.assertIn('not an improvement', res['interpretation'])
        no_delta = copy.deepcopy(d)
        for e in no_delta['rsm_turbulence_closure_crosscheck']:
            del e['delta_dp_u_vs_ct35_baseline_pct']
        self.assertEqual(self.evaluate(no_delta)['admission'], 'admitted')
        # A candidate with higher pressure drop than the control is an equally resolved difference.
        reversed_sign = synthetic_same_pipeline(offset_pa=-400.0)
        res = self.evaluate(reversed_sign)
        self.assertEqual(res['admission'], 'admitted', res['failures'])
        self.assertEqual(res['checks']['comparator_minus_candidate_finest_dp_u_pa'], -400.0)
        # Recorded margin that reproduces from grids is accepted.
        recorded = copy.deepcopy(d)
        recorded['roache_gci_analysis']['signal_to_uncertainty_margin'] = {
            'saving_to_3grid_span_ratio': 4.0, 'saving_to_3grid_eloss_span_ratio': 4.0}
        self.assertEqual(self.evaluate(recorded)['admission'], 'admitted')

    def test_signal_to_span_boundary(self):
        self.assertEqual(self.evaluate(synthetic_same_pipeline(300.0))['admission'], 'admitted')
        res = self.evaluate(synthetic_same_pipeline(299.0))
        self.assertEqual(res['admission'], 'not_admitted')
        self.assertTrue(any('dp_u signal-to-span ratio 2.990 < required 3.000' in f for f in res['failures']),
                        res['failures'])

    def test_legacy_different_pipeline_ct35_baseline_not_admitted(self):
        for name, dossier in legacy_dossiers():
            before = json.dumps(dossier, sort_keys=True)
            res = self.evaluate(dossier)
            with self.subTest(dossier=name):
                self.assertEqual(res['admission'], 'not_admitted')
                self.assertFalse(res['gate_passed'])
                self.assertTrue(any('pipeline identity not bound' in f for f in res['failures']), res['failures'])
                self.assertTrue(any('comparator 3-grid ladder not bound' in f for f in res['failures']),
                                res['failures'])
                self.assertTrue(any('solver version not bound' in f for f in res['failures']), res['failures'])
                self.assertFalse(any('25' in f and 'reduction' in f for f in res['failures']), res['failures'])
                self.assertEqual(json.dumps(dossier, sort_keys=True), before)

    def test_real_same_pipeline_numbers_are_not_a_resolved_difference(self):
        # Wave-1 v2 grids against INT-06 same-pipeline control grids, with SYNTHETIC equal identities.
        d = synthetic_same_pipeline()
        real = load(W1_V2)
        for g, r in zip(d['grids'], real['grids']):
            g['dp_u_pa'] = r['dp_u_pa']
            g['hydraulic_loss_j_per_treated_m3'] = r['hydraulic_loss_j_per_treated_m3']
        ctl = load(W4)['same_pipeline_ct35_control']['grids']
        for g, c in zip(d['baseline_ct35_comparator']['grids'], ctl):
            g['dp_u_pa'] = c['dp_u_pa']
            g['hydraulic_loss_j_per_treated_m3'] = c['hydraulic_loss_j_per_treated_m3']
        res = self.evaluate(d)
        self.assertEqual(res['admission'], 'not_admitted')
        span = max(g['dp_u_pa'] for g in real['grids']) - min(g['dp_u_pa'] for g in real['grids'])
        expected = abs(ctl[2]['dp_u_pa'] - real['grids'][2]['dp_u_pa']) / span
        self.assertAlmostEqual(res['checks']['saving_to_3grid_span_ratio_dp_u'], expected, places=12)
        self.assertLess(expected, 0.05)
        self.assertTrue(any('dp_u signal-to-span ratio' in f for f in res['failures']), res['failures'])

    def test_identity_ladder_and_subject_mismatch_not_admitted(self):
        cases = []
        d = synthetic_same_pipeline()
        d['baseline_ct35_comparator']['pipeline_identity']['mesher'] = sha(b'SYNTHETIC-other-mesher')
        cases.append((d, 'pipeline identity differs'))
        d = synthetic_same_pipeline()
        d['pipeline_identity']['arbitrary'] = sha(b'x')
        d['baseline_ct35_comparator']['pipeline_identity']['arbitrary'] = sha(b'x')
        cases.append((d, 'pipeline identity not bound'))
        d = synthetic_same_pipeline()
        d['pipeline_identity'] = d['baseline_ct35_comparator']['pipeline_identity'] = {'name': 'same'}
        cases.append((d, 'pipeline identity not bound'))
        d = synthetic_same_pipeline()
        d['baseline_ct35_comparator']['grids'][1]['background_hex_grid'] = [25, 16, 282]
        cases.append((d, 'background_hex_grid'))
        d = synthetic_same_pipeline()
        del d['baseline_ct35_comparator']['grids']
        cases.append((d, 'comparator 3-grid ladder not bound'))
        d = synthetic_same_pipeline()
        d['grids'][1]['subject_design_sha256'] = sha(b'SYNTHETIC-prior-elite')
        cases.append((d, 'grid G1_medium subject identity'))
        d = synthetic_same_pipeline()
        d['baseline_ct35_comparator']['grids'][2]['subject_design_sha256'] = sha(b'SYNTHETIC-other')
        cases.append((d, 'comparator grid G2_fine subject identity'))
        d = synthetic_same_pipeline()
        d['openfoam_version'] = 'v2606'
        cases.append((d, 'solver version'))
        d = synthetic_same_pipeline()
        d['grids'][0]['grid_id'] = 'G1_coarse'
        cases.append((d, 'grid ladder'))
        d = synthetic_same_pipeline()
        d['baseline_ct35_comparator']['grids'][0]['reject_fraction'] = 0.03
        cases.append((d, 'comparator grid G0_coarse reject_fraction'))
        for i, (dossier, needle) in enumerate(cases):
            res = self.evaluate(dossier)
            with self.subTest(case=i, needle=needle):
                self.assertEqual(res['admission'], 'not_admitted')
                self.assertTrue(any(needle in f for f in res['failures']), res['failures'])

    def test_control_argument_replaces_comparator(self):
        d = synthetic_same_pipeline()
        control = copy.deepcopy(d['baseline_ct35_comparator'])
        del d['baseline_ct35_comparator']
        self.assertEqual(self.evaluate(d, control=control)['admission'], 'admitted')
        res = self.evaluate(d)
        self.assertEqual(res['admission'], 'not_admitted')
        self.assertTrue(any('comparator' in f and 'missing' in f for f in res['failures']), res['failures'])

    def test_recorded_ratio_must_reproduce(self):
        d = synthetic_same_pipeline()
        d['roache_gci_analysis']['signal_to_uncertainty_margin'] = {
            'saving_to_3grid_span_ratio': 4.86, 'saving_to_3grid_eloss_span_ratio': 4.0}
        res = self.evaluate(d)
        self.assertEqual(res['admission'], 'not_admitted')
        self.assertTrue(any('recorded saving_to_3grid_span_ratio' in f and 'does not reproduce' in f
                            for f in res['failures']), res['failures'])

    def test_non_numeric_non_finite_and_missing_observations_not_admitted(self):
        def mutate(fn, needle):
            d = synthetic_same_pipeline()
            fn(d)
            res = self.evaluate(d)
            self.assertEqual(res['admission'], 'not_admitted', needle)
            self.assertTrue(any(needle in f for f in res['failures']), (needle, res['failures']))

        def rsm(d):
            return d['rsm_turbulence_closure_crosscheck']
        mutate(lambda d: d['grids'][0].__setitem__('water_closure_fraction', math.nan), 'non-finite')
        mutate(lambda d: d['grids'][2].__setitem__('dp_u_pa', math.inf), 'non-finite')
        mutate(lambda d: d['grids'][1].__setitem__('reject_fraction', '0.0197'), 'not a number')
        mutate(lambda d: d['grids'][1].__setitem__('cells', True), 'not a number')
        mutate(lambda d: d['grids'][0].pop('water_closure_fraction'), 'missing')
        mutate(lambda d: rsm(d)[0]['reynolds_stress_tensor_diagnostics'].__setitem__(
            'positive_definite_normal_stress_fraction', math.nan), 'non-finite')
        mutate(lambda d: rsm(d)[0]['reynolds_stress_tensor_diagnostics'].__setitem__(
            'positive_definite_normal_stress_fraction', 0.98), 'realizability fraction 0.980000 < 1.000000')
        mutate(lambda d: rsm(d)[1].pop('dp_u_pa'), 'missing')
        mutate(lambda d: [e.__setitem__('dp_u_pa', -1.0) for e in rsm(d)], 'non-positive mean')
        mutate(lambda d: rsm(d).pop(), 'missing required 7-equation RSM closure: SSG')
        mutate(lambda d: d['roache_gci_analysis'].__setitem__('signal_to_uncertainty_margin', {
            'saving_to_3grid_span_ratio': True}), 'not a number')
        mutate(lambda d: [g.__setitem__('dp_u_pa', 18650.0) for g in d['grids']], '3-grid dp_u_pa span')
        mutate(lambda d: d['grids'][0].__setitem__('water_closure_fraction', 2e-3),
               'runner policy (not preregistered)')

    def test_wave2_runner_import_contract_preserved(self):
        res = self.evaluate(load(W1_V1))
        for key in ('subject_candidate_id', 'subject_topology_class', 'gate_passed', 'checks', 'failures'):
            self.assertIn(key, res)
        self.assertIn('runner_policy_constraints_not_preregistered', res)


class EliteCfdGateCliTruth(unittest.TestCase):
    def test_missing_prereg_is_an_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = run_cli('--prereg', str(Path(tmp) / 'missing.json'))
            self.assertNotEqual(p.returncode, 0)
            self.assertNotIn('PASS', p.stdout)
            self.assertFalse(p.stdout.startswith('ADMITTED'))
            self.assertIn('preregistration', p.stderr)

    def test_misspelled_prereg_key_is_an_error(self):
        prereg = load(PREREG)
        prereg['mandatory_elite_cfd_gate'] = prereg.pop('mandatory_elite_cfd_gates')
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'prereg.json'
            path.write_text(json.dumps(prereg), encoding='utf-8')
            p = run_cli('--prereg', str(path))
            self.assertEqual(p.returncode, 2)
            self.assertIn('mandatory_elite_cfd_gates', p.stderr)

    def test_nonfinite_or_duplicate_json_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            text = PREREG.read_text(encoding='utf-8').replace(
                '"min_signal_to_3grid_span_ratio": 3.0', '"min_signal_to_3grid_span_ratio": NaN')
            path = Path(tmp) / 'nan.json'
            path.write_text(text, encoding='utf-8')
            self.assertEqual(run_cli('--prereg', str(path)).returncode, 2)
            dup = PREREG.read_text(encoding='utf-8').replace(
                '"openfoam_version": "Foundation-14",',
                '"openfoam_version": "Foundation-14", "openfoam_version": "Foundation-14",')
            path = Path(tmp) / 'dup.json'
            path.write_text(dup, encoding='utf-8')
            self.assertEqual(run_cli('--prereg', str(path)).returncode, 2)

    def test_committed_dossier_not_admitted_at_cli(self):
        p = run_cli()
        self.assertEqual(p.returncode, 1)
        self.assertIn('NOT ADMITTED', p.stdout)
        self.assertNotIn('certified', p.stdout)
        self.assertNotIn('PASS', p.stdout)

    def test_verify_existing_missing_ablation_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = run_cli('--verify-existing', '--wave2-ablation', str(Path(tmp) / 'missing.json'))
            self.assertNotEqual(p.returncode, 0)
            self.assertIn('missing', p.stdout + p.stderr)

    def test_interpret_existing_report(self):
        sources = {p: sha(p.read_bytes()) for p in (W1_V1, W1_V2, W2_V1, W2_V2, PREREG)}
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'interpretation.json'
            p = run_cli('--interpret-existing', '--out', str(out))
            self.assertEqual(p.returncode, 0, p.stderr)
            report = json.loads(out.read_text(encoding='utf-8'))
            stdout_report = json.loads(run_cli('--interpret-existing').stdout)
        self.assertEqual(report, stdout_report)
        self.assertEqual(report['schema'], 'joil-elite-cfd-gate-interpretation/1')
        self.assertEqual(report['gate_tool_sha256'], sha(GATE_TOOL.read_bytes()))
        self.assertEqual(report['preregistration']['sha256'], sources[PREREG])
        self.assertEqual(len(report['dossiers']), 6)
        recorded = 0
        for row in report['dossiers']:
            self.assertEqual(row['corrected_admission'], 'not_admitted')
            self.assertEqual(row['source_sha256'], sources[ROOT / row['source_path']])
            self.assertTrue(row['failures'])
            if row['recorded_gate_passed'] is not None:
                recorded += 1
                self.assertTrue(row['recorded_gate_passed'])
            diag = row['diagnostic_vs_int06_same_pipeline_control']
            self.assertEqual(diag['status'], 'diagnostic_only_pipeline_identity_unbound_not_admission')
            self.assertLess(diag['signal_to_3grid_span_ratio_dp_u'], 3.0)
        self.assertEqual(recorded, 5)
        self.assertEqual({p: sha(p.read_bytes()) for p in sources}, sources)

    def test_interpret_existing_refuses_existing_out(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'existing.json'
            out.write_bytes(b'keep')
            p = run_cli('--interpret-existing', '--out', str(out))
            self.assertEqual(p.returncode, 2)
            self.assertEqual(out.read_bytes(), b'keep')

    def test_generate_v2_requires_fresh_out_dir(self):
        committed = {p: sha(p.read_bytes()) for p in (W1_V2, W2_V2)}
        self.assertEqual(run_cli('--generate-v2').returncode, 2)
        with tempfile.TemporaryDirectory() as tmp:
            p = run_cli('--generate-v2', '--out-dir', tmp)
            self.assertEqual(p.returncode, 2)
            self.assertEqual(list(Path(tmp).iterdir()), [])
        self.assertEqual({p: sha(p.read_bytes()) for p in committed}, committed)


class GeometryMeasurementTruth(unittest.TestCase):
    """V5-01-02: copied, assigned or star-shaped geometry values are proxies, never measurements."""

    @classmethod
    def setUpClass(cls):
        sys.path.insert(0, str(ROOT / 'tools'))
        import f0_enactment
        import f0_voxel
        cls.enact, cls.voxel = f0_enactment, f0_voxel

    def ramp_program(self, drop_mm):
        p = self.enact.exotic_graph.default_exotic_program()
        p['inlet_ramp']['axial_drop_mm'] = drop_mm
        return p

    def test_enactment_receipt_does_not_copy_authored_delta(self):
        for drop in (4.0, 6.0):
            r = self.enact.build_enactment_receipt(self.ramp_program(drop))
            ramp = r['active_mechanisms'][0]
            self.assertEqual(ramp['gene_name'], 'inlet_ramp')
            self.assertIsNone(ramp['measured_geometric_delta_mm'])
            self.assertIsNone(ramp['enacted_surface_area_m2'])
            self.assertIsNone(ramp['enacted_triangle_count'])
            self.assertEqual(ramp['boundary_face_ids'], [])
            self.assertEqual(ramp['measurement_status'], 'not_measured')
            self.assertEqual(ramp['legacy_proxy']['geometric_delta_mm'], drop)
            self.assertIsNone(r['measured_disjoint_inlet_paths'])
            self.assertIsNone(r['all_active_features_enacted'])
            self.assertIsNone(r['actual_simulated_geometry_label'])
            self.assertTrue(r['legacy_proxy']['all_active_features_enacted'])
            self.assertEqual(r['kind'], 'authored_enactment_manifest_not_geometry_measurement')
        with self.assertRaises(self.enact.EnactmentVerificationError):
            self.enact.build_enactment_receipt(self.ramp_program(4.0), dropped_features=('inlet_ramp',))

    def test_program_observer_returns_no_measured_cavity_zero(self):
        for p in (self.enact.exotic_graph.default_exotic_program(), self.ramp_program(4.0)):
            o = self.voxel.observe_voxel_sdf(p)
            self.assertIsNone(o['observed_artifact'])
            for key in ('trapped_powder_cavities', 'trapped_fluid_cavities', 'trapped_cavities_total',
                        'connected_fluid_components', 'frozen_collar_and_tail_diff_mm3',
                        'terminal_inlet1_area_mm2', 'terminal_overflow_diameter_mm', 'derived_taxon',
                        'fluid_volume_mm3', 'wetted_area_mm2'):
                self.assertIsNone(o[key], key)
                self.assertTrue(o['measurement_status'][key], key)
            self.assertEqual(o['legacy_proxy']['trapped_powder_cavities'], 0)
            self.assertEqual(o['legacy_proxy']['frozen_collar_and_tail_diff_mm3'], 0.0)
            self.assertIn('star-shaped', o['measurement_status']['trapped_fluid_cavities'])

    def test_proxy_or_missing_values_cannot_satisfy_geometry_gate(self):
        fixture = load(ROOT / 'tests/fixtures/negative/neg_01_open_boundary_edge.json')
        fixture['open_boundary_edges'] = 0
        self.assertEqual(self.voxel.audit_negative_fixture(fixture), [])  # complete declared facts pass
        for value in ('missing', None, True, '0'):
            bad = copy.deepcopy(fixture)
            if value == 'missing':
                del bad['trapped_powder_cavities']
            else:
                bad['trapped_powder_cavities'] = value
            with self.subTest(value=value):
                self.assertEqual(self.voxel.audit_negative_fixture(bad), ['geometry_fact_not_measured'])
        nan = copy.deepcopy(fixture)
        nan['minimum_shell_thickness_mm'] = math.nan
        self.assertEqual(self.voxel.audit_negative_fixture(nan), ['shell_thickness_below_minimum'])
        observed = self.voxel.observe_voxel_sdf(self.ramp_program(4.0))
        observed.update(declared_units='m', coordinate_span_m=1.662)
        self.assertIn('geometry_fact_not_measured', self.voxel.audit_negative_fixture(observed))
        self.assertEqual(self.voxel.audit_negative_corpus_dir()['fixture_count'], 22)


class ExplorerClaimTruth(unittest.TestCase):
    """V5-01-04: explorer strings match current evidence; authored paths are labelled SCHEMATIC."""

    def setUp(self):
        self.src = (ROOT / 'apps/explorer/src/main.tsx').read_text(encoding='utf-8')

    def test_retracted_pressure_reduction_strings_removed(self):
        for stale in ('−38.5% / −38.4%', '100% realizable)', 'pass F0 B-rep/mesh', 'oil-droplet core capture',
                      'coupled droplet trajectories'):
            self.assertNotIn(stale, self.src)
        self.assertIn('retracted comparison, not a reduction claim', self.src)
        self.assertIn('corrected gate: not admitted', self.src)
        # Historical numbers are bound to the committed dossier fields, not typed into the UI.
        self.assertIn('rsm[0].delta_dp_u_vs_ct35_baseline_pct', self.src)

    def test_three_d_view_is_labelled_schematic(self):
        view = self.src[self.src.index("tab === '3D Explorer'"):]
        view = view[:view.index('<Viewport3D />')]
        self.assertIn('SCHEMATIC paths.', view)
        self.assertIn('No simulated or measured trajectory is supplied', view)


if __name__ == '__main__':
    unittest.main()
