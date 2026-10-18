"""Independent audit mutations: inventory is never physical or unexecuted PASS."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('baseline_audit', ROOT / 'tools/v5_baseline_audit.py')
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class BaselineAudit(unittest.TestCase):
    def setUp(self):
        def read(name):
            return json.loads((ROOT / name).read_text())
        self.raw = (ROOT / audit.EXECUTION / 'DEFECTS.json').read_bytes()
        self.mapping = read('docs/v5-plan/DEFECT_MAP.json')
        self.gates = read(audit.EXECUTION / 'DEFECT_ACCEPTANCE_GATES.json')
        self.deferrals = read(audit.EXECUTION / 'V4_DEFERRALS.json')
        self.packets = [p['id'] for p in read('docs/v5-plan/TASKS.json')['packets']]

    def validate(self):
        return audit.validate(self.raw, self.mapping, self.gates, self.deferrals, self.packets)

    def test_retained_inventory_is_explicitly_unqualified(self):
        result = self.validate()
        self.assertEqual((result['recorded_behavior_probes'], result['source_only_inventory'],
                          result['retained_deferrals']), (15, 16, 12))
        self.assertFalse(result['physical_qualification'])
        self.assertFalse(result['frozen_source_bytes_checked'])

    def test_every_unexecuted_challenge_refuses_runtime_promotion(self):
        original = copy.deepcopy(self.gates)
        for row in original['records']:
            if row['runtime_probe'] is not None:
                continue
            with self.subTest(row['id']):
                self.gates = copy.deepcopy(original)
                changed = next(r for r in self.gates['records'] if r['id'] == row['id'])
                changed.update(inventory_evidence_kind='behavior_probe_recorded',
                               behavior_probe_recorded_at_baseline=True)
                with self.assertRaisesRegex(ValueError, 'unexecuted'):
                    self.validate()

    def test_every_v4_remainder_is_required(self):
        original = copy.deepcopy(self.deferrals)
        # Expectations are enumerated independently of the implementation constant.
        expected = ['TR-12', 'G-01', 'G-04', 'G-05', 'INT-06', 'F2-Oil', 'F4',
                    'TR-16', 'TR-16-threshold-audit', 'EV-09', 'EV-12',
                    'V4-follow-on-program']
        self.assertEqual({r['id'] for r in original['deferrals']}, set(expected))
        for name in expected:
            with self.subTest(name):
                self.deferrals = copy.deepcopy(original)
                self.deferrals['deferrals'] = [r for r in original['deferrals'] if r['id'] != name]
                with self.assertRaisesRegex(ValueError, 'coverage'):
                    self.validate()

    def test_no_remainder_can_be_declared_physical_pass_or_done(self):
        original = copy.deepcopy(self.deferrals)
        for i in range(12):
            for change in ({'physical_pass': True}, {'current_state': 'done'}):
                with self.subTest(i=i, change=change):
                    self.deferrals = copy.deepcopy(original)
                    self.deferrals['deferrals'][i].update(change)
                    with self.assertRaisesRegex(ValueError, 'promotion'):
                        self.validate()

    def test_missing_owner_dependency_and_unknown_target_refused(self):
        for change in ({'owner': ''}, {'next_dependency': ''}, {'source_location': ''},
                       {'target_packets': ['V5-99']}):
            with self.subTest(change=change):
                original = copy.deepcopy(self.deferrals)
                self.deferrals['deferrals'][0].update(change)
                with self.assertRaises(ValueError):
                    self.validate()
                self.deferrals = original

    def test_repair_mapping_and_challenge_remain_bound(self):
        for change in ({'primary_task': 'V5-00-01'}, {'negative_fixture': ''},
                       {'source_finding_sha256': ['0' * 64]},
                       {'physical_qualification': True}, {'primary_repair_closure': ''}):
            with self.subTest(change=change):
                original = copy.deepcopy(self.gates)
                self.gates['records'][0].update(change)
                with self.assertRaises(ValueError):
                    self.validate()
                self.gates = original

    def test_whole_source_and_anchor_binding_reject_line_only_reuse(self):
        raw = b'prefix\ndef target():\n    return 7\n'
        source = {'symbol': 'target', 'anchor': 'def target():', 'current_lines': [2]}
        audit.source_bound(raw, audit.sha(raw), source)
        for mutant in (b'\n' + raw, raw.replace(b'7', b'8')):
            with self.assertRaisesRegex(ValueError, 'source changed'):
                audit.source_bound(mutant, audit.sha(raw), source)
        source['current_lines'] = [1]
        with self.assertRaisesRegex(ValueError, 'anchor/line'):
            audit.source_bound(raw, audit.sha(raw), source)

    def test_frozen_and_inventory_identity_cannot_drift(self):
        self.gates['inventory_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'identity'):
            self.validate()
        self.gates['inventory_sha256'] = audit.sha(self.raw)
        self.deferrals['frozen_source_commit'] = '0' * 40
        with self.assertRaisesRegex(ValueError, 'source drift'):
            self.validate()


if __name__ == '__main__':
    unittest.main()
