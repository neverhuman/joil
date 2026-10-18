"""Receipt-gate boundary tests; synthetic receipt metadata never validates physics."""
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('v5_plan_closure', ROOT / 'tools/v5_plan.py')
plan = importlib.util.module_from_spec(spec)
spec.loader.exec_module(plan)


class DefectClosure(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.real_is_tracked = plan.is_tracked
        tracker = patch.object(plan, 'is_tracked', return_value=True)
        self.tracker = tracker.start()
        self.addCleanup(tracker.stop)
        shutil.copytree(ROOT / 'docs/v5-plan/execution', self.root / 'docs/v5-plan/execution')
        self.task = 'V5-09-03'
        self.ref = {'path': 'synthetic-receipt.json', 'sha256': 'a' * 64}
        fixture = self.root / 'fixture.py'
        fixture.write_text('# synthetic receipt-validator fixture, not an executed repair\n')
        self.artifact = {'path': 'fixture.py', 'sha256': plan.digest(fixture.read_bytes())}
        source = next(r for r in plan.read_json(
            self.root / 'docs/v5-plan/execution/DEFECTS.json')['defects'] if r['id'] == 'D-05')['sources'][0]
        self.positive = dict(argv=['synthetic-positive'], exit_code=0, log_sha256='b' * 64)
        self.receipt = dict(artifacts=[self.artifact], commands=[self.positive], defect_controls=[{
            'defect_id': 'D-05', 'fixture': self.artifact,
            'source': dict(commit='e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0',
                           path=source['path'], symbol=source['symbol'], sha256=source['source_sha256']),
            'positive': dict(self.positive, expected_result='independent positive assertion'),
            'red': dict(argv=['synthetic-red'], exit_code=101, log_sha256='c' * 64,
                        expected_result='independent assertion fails on prior source'),
        }])

    def check(self):
        plan.validate_defect_controls(self.root, self.ref, self.task, self.receipt)

    def test_complete_metadata_is_accepted_without_claiming_execution(self):
        self.check()

    def test_missing_and_duplicate_controls_refuse(self):
        control = self.receipt['defect_controls'][0]
        for controls in ([], [control, control]):
            self.receipt['defect_controls'] = controls
            with self.assertRaisesRegex(ValueError, 'missing/duplicate'):
                self.check()

    def test_inventory_and_zero_exit_red_cannot_close_a_primary_repair(self):
        self.receipt['defect_controls'][0]['red']['exit_code'] = 0
        with self.assertRaisesRegex(ValueError, 'red defect control'):
            self.check()

    def test_every_source_only_primary_requires_its_own_control(self):
        gates = plan.read_json(self.root / 'docs/v5-plan/execution/DEFECT_ACCEPTANCE_GATES.json')
        self.receipt['defect_controls'] = []
        for row in gates['records']:
            if row['inventory_evidence_kind'] != 'source_only_inventory':
                continue
            with self.subTest(row['id']):
                self.task = row['primary_task']
                with self.assertRaisesRegex(ValueError, 'executed defect controls'):
                    self.check()

    def test_unrecorded_positive_and_missing_red_hash_refuse(self):
        original = copy.deepcopy(self.receipt)
        self.receipt['defect_controls'][0]['positive']['argv'] = ['unrecorded']
        with self.assertRaisesRegex(ValueError, 'not executed'):
            self.check()
        self.receipt = original
        self.receipt['defect_controls'][0]['red']['log_sha256'] = ''
        with self.assertRaisesRegex(ValueError, 'red defect control'):
            self.check()

    def test_stale_fixture_unbound_artifact_and_source_drift_refuse(self):
        original = copy.deepcopy(self.receipt)
        self.receipt['artifacts'] = []
        with self.assertRaisesRegex(ValueError, 'receipt artifact'):
            self.check()
        self.receipt = copy.deepcopy(original)
        self.receipt['defect_controls'][0]['source']['sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'source binding'):
            self.check()
        self.receipt = original
        (self.root / 'fixture.py').write_text('changed')
        with self.assertRaisesRegex(ValueError, 'stale defect fixture'):
            self.check()

    def test_untracked_fixture_cannot_close_primary_task(self):
        self.tracker.return_value = False
        with self.assertRaisesRegex(ValueError, 'stale defect fixture'):
            self.check()

    def test_real_repository_tracking_boundary(self):
        self.assertTrue(self.real_is_tracked(ROOT, 'docs/v5-plan/execution/DEFECTS.json'))
        self.assertFalse(self.real_is_tracked(ROOT, '.private/untracked-control.py'))

    def test_prior_acceptance_is_exact_receipt_only(self):
        gates = plan.read_json(self.root / 'docs/v5-plan/execution/DEFECT_ACCEPTANCE_GATES.json')
        for prior in gates['accepted_prior_primary_receipts']:
            self.task = prior['task_id']
            self.ref = prior['receipt']
            self.receipt['defect_controls'] = []
            self.check()
            self.ref = dict(self.ref, sha256='0' * 64)
            with self.assertRaisesRegex(ValueError, 'executed defect controls'):
                self.check()


if __name__ == '__main__':
    unittest.main()
