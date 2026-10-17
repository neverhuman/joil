"""Plan integrity boundaries; no physical qualification is asserted."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('v5_plan', ROOT / 'tools/v5_plan.py')
plan = importlib.util.module_from_spec(spec)
spec.loader.exec_module(plan)


class PlanIntegrity(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        shutil.copytree(ROOT / 'docs/v5-plan', self.root / 'docs/v5-plan')
        (self.root / 'tools').mkdir()
        shutil.copyfile(ROOT / 'tools/v5_dashboard.html', self.root / 'tools/v5_dashboard.html')
        shutil.copyfile(ROOT / 'docs/TEST_CATALOG.json', self.root / 'docs/TEST_CATALOG.json')

    def mutate(self, name, fn):
        path = self.root / 'docs/v5-plan' / name
        data = json.loads(path.read_text())
        fn(data)
        path.write_text(json.dumps(data))

    def receipt(self, task='V5-00-01'):
        path = self.root / 'docs/v5-plan/receipts/test.json'
        path.write_text(json.dumps(dict(task_ids=[task], tested_commit='a' * 40,
            changed_paths=['docs/v5-plan/execution/BASELINE.json'],
            commands=[dict(argv=['fixture-check'], exit_code=0, log_sha256='b' * 64)],
            positive_checks=['known source'], negative_checks=['changed source rejected'],
            artifacts=[dict(path='baseline', sha256='c' * 64)],
            resource_allocation={'gpu': False}, limitations=['synthetic test receipt'], next_dependency='V5-00-02')))
        return str(path.relative_to(self.root))

    def test_current_package_is_consistent(self):
        data = plan.build(self.root, check=True)
        self.assertEqual(sum(len(p['tasks']) for p in data['packets']), 160)
        self.assertFalse(any(t['status'] == 'done' for p in data['packets'] for t in p['tasks']))

    def test_cycle_rejected(self):
        self.mutate('TASKS.json', lambda d: d['packets'][0]['dependencies'].append('V5-01'))
        with self.assertRaisesRegex(ValueError, 'cycle'):
            plan.load(self.root)

    def test_unknown_dependency_rejected(self):
        self.mutate('TASKS.json', lambda d: d['packets'][0]['dependencies'].append('V5-99'))
        with self.assertRaisesRegex(ValueError, 'dependency'):
            plan.load(self.root)

    def test_duplicate_task_rejected(self):
        self.mutate('TASKS.json', lambda d: d['packets'][0]['tasks'].append(d['packets'][0]['tasks'][0]))
        with self.assertRaisesRegex(ValueError, 'unique tasks'):
            plan.load(self.root)

    def test_unknown_progress_and_missing_task_rejected(self):
        self.mutate('progress/V5-00.json', lambda d: d['tasks'].pop('V5-00-01'))
        with self.assertRaisesRegex(ValueError, 'mismatch'):
            plan.load(self.root)

    def test_unowned_active_task_rejected_and_update_rolled_back(self):
        path = self.root / 'docs/v5-plan/progress/V5-00.json'
        before = path.read_bytes()
        with self.assertRaisesRegex(ValueError, 'owner'):
            plan.update(self.root, 'V5-00-01', 'in_progress', None, None, None)
        self.assertEqual(before, path.read_bytes())

    def test_ordered_tasks_and_dependencies_gate_activation(self):
        with self.assertRaisesRegex(ValueError, 'incomplete'):
            plan.update(self.root, 'V5-00-02', 'in_progress', 'tester', None, None)
        with self.assertRaisesRegex(ValueError, 'incomplete'):
            plan.update(self.root, 'V5-01-01', 'in_progress', 'tester', None, None)

    def test_blocker_needs_reason(self):
        with self.assertRaisesRegex(ValueError, 'reason'):
            plan.update(self.root, 'V5-00-01', 'blocked', 'tester', '', None)

    def test_done_requires_receipt(self):
        with self.assertRaisesRegex(ValueError, 'receipt'):
            plan.update(self.root, 'V5-00-01', 'done', 'tester', None, None)

    def test_hashed_receipt_updates_only_progress_and_generated_views(self):
        original_registry = (self.root / 'docs/v5-plan/TASKS.json').read_bytes()
        plan.update(self.root, 'V5-00-01', 'done', 'tester', 'Fixture only', self.receipt())
        data = plan.build(self.root)
        self.assertEqual(data['packets'][0]['tasks'][0]['status'], 'done')
        self.assertEqual(original_registry, (self.root / 'docs/v5-plan/TASKS.json').read_bytes())
        self.assertIn('progress is 1/160', (self.root / 'docs/v5-plan/TODO.md').read_text())
        plan.build(self.root, check=True)

    def test_stale_receipt_rejected(self):
        rel = self.receipt()
        plan.update(self.root, 'V5-00-01', 'done', 'tester', None, rel)
        (self.root / rel).write_text('{}')
        with self.assertRaisesRegex(ValueError, 'stale receipt'):
            plan.load(self.root)

    def test_incomplete_receipt_rejected(self):
        rel = self.receipt()
        p = self.root / rel
        data = json.loads(p.read_text())
        data.pop('negative_checks')
        p.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'incomplete receipt'):
            plan.update(self.root, 'V5-00-01', 'done', 'tester', None, rel)

    def test_path_escape_and_symlink_escape_rejected(self):
        with self.assertRaisesRegex(ValueError, 'escapes'):
            plan.inside(self.root, '../private.json')
        link = self.root / 'escape'
        link.symlink_to(self.root.parent, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'escapes'):
            plan.inside(self.root, 'escape/private.json')

    def test_catalogue_omission_rejected(self):
        self.mutate('CATALOGUE_MAP.json', lambda d: d['families'].pop())
        with self.assertRaisesRegex(ValueError, 'catalogue'):
            plan.load(self.root)

    def test_defect_omission_rejected(self):
        self.mutate('DEFECT_MAP.json', lambda d: d['defects'].pop())
        with self.assertRaisesRegex(ValueError, 'defect'):
            plan.load(self.root)

    def test_duplicate_json_and_nonfinite_rejected(self):
        path = self.root / 'bad.json'
        for text in ('{"id":1,"id":2}', '{"x":NaN}', '{"x":Infinity}'):
            path.write_text(text)
            with self.assertRaises(ValueError):
                plan.read_json(path)

    def test_stale_generated_view_rejected(self):
        (self.root / 'docs/v5-plan/dashboard.html').write_text('stale')
        with self.assertRaisesRegex(ValueError, 'stale generated'):
            plan.build(self.root, check=True)

    def test_script_embedding_cannot_end_data_tag(self):
        bad = '</script><script>alert("injected")</script>'
        self.mutate('TASKS.json', lambda d: d['packets'][0].update(title=bad))
        _, files = plan.generated(self.root)
        html = files[Path('docs/v5-plan/dashboard.html')]
        self.assertNotIn(bad, html)
        self.assertIn('\\u003c/script\\u003e', html)


if __name__ == '__main__':
    unittest.main()
