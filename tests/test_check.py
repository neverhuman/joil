"""Check selection and receipts must preserve coverage and fail closed."""
import importlib.util
import os
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('foundation_check', ROOT / 'tools/check.py')
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


class CheckRunnerTests(unittest.TestCase):
    def test_partition_covers_all_default_steps_once(self):
        default = check.check_steps(ROOT)
        partition = [step for suite in check.SUITES for step in check.check_steps(ROOT, suite=suite)]
        self.assertCountEqual(partition, default)
        self.assertEqual(len({step[0] for step in partition}), len(default))
        self.assertEqual([step[0] for step in check.check_steps(ROOT, suite='ui')], ['npm-ci', 'ui-build'])

    def test_invalid_or_conflicting_selection_rejected(self):
        for kwargs in ({'suite': 'typo'}, {'suite': 'ui', 'skip_ui': True}, {'suite': 'fmt', 'cad': True}):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                check.check_steps(ROOT, **kwargs)
        self.assertEqual(len(check.check_steps(ROOT, skip_ui=True)), 8)
        self.assertEqual(check.check_steps(ROOT, cad=True)[-1][0], 'cad')

    def run_fake_suite(self, suite, fail):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            def run(command, **kwargs):
                if command[0] == 'git':
                    return subprocess.CompletedProcess(command, 0, stdout='f' * 40, stderr='')
                self.assertEqual(kwargs['env']['CARGO_BUILD_JOBS'], '2')
                self.assertEqual(kwargs['env']['CARGO_INCREMENTAL'], os.environ.get('CARGO_INCREMENTAL', '0'))
                self.assertEqual(kwargs['env']['CARGO_PROFILE_DEV_DEBUG'], os.environ.get('CARGO_PROFILE_DEV_DEBUG', 'line-tables-only'))
                for key in ('OMP_NUM_THREADS', 'OPENBLAS_NUM_THREADS', 'MKL_NUM_THREADS', 'RAYON_NUM_THREADS'):
                    self.assertEqual(kwargs['env'][key], '1')
                return subprocess.CompletedProcess(command, 1 if fail else 0, stdout=b'fixture output', stderr=b'')

            with patch.object(check, 'ROOT', root), patch('sys.argv', ['check.py', '--suite', suite]), \
                    patch.object(check.subprocess, 'run', side_effect=run), \
                    patch.object(check.subprocess, 'check_output', return_value=b''):
                status = check.main()
            receipts = list((root / '.private/checks').glob('*/receipt.json'))
            self.assertEqual(len(receipts), 1)
            receipt = json.loads(receipts[0].read_text())
            self.assertEqual(receipt['requested_suite'], suite)
            self.assertEqual(receipt['ui_executed'], suite == 'ui')
            self.assertFalse(receipt['cfd_executed'])
            for step in receipt['steps']:
                log = receipts[0].parent / (step['name'] + '.log')
                self.assertEqual(step['log_sha256'], check.hashlib.sha256(log.read_bytes()).hexdigest())
            return status, receipt

    def test_suite_failure_propagates_and_stops_before_ui_build(self):
        status, receipt = self.run_fake_suite('ui', fail=True)
        self.assertEqual(status, 1)
        self.assertFalse(receipt['all_requested_pass'])
        self.assertEqual([step['name'] for step in receipt['steps']], ['npm-ci'])

    def test_clean_ci_build_defaults_and_explicit_profile_override(self):
        from contextlib import redirect_stdout
        import io
        for override, expected in (({}, 'line-tables-only'), ({'CARGO_PROFILE_DEV_DEBUG': '0'}, '0')):
            with tempfile.TemporaryDirectory() as directory:
                def run(command, **kwargs):
                    if command[0] == 'git':
                        return subprocess.CompletedProcess(command, 0, stdout='f' * 40, stderr='')
                    self.assertEqual(kwargs['env']['CARGO_PROFILE_DEV_DEBUG'], expected)
                    self.assertEqual(kwargs['env']['CARGO_INCREMENTAL'], '0')
                    return subprocess.CompletedProcess(command, 0, stdout=b'', stderr=b'')
                with patch.dict(check.os.environ, {'PATH': os.defpath, **override}, clear=True), \
                        patch.object(check, 'ROOT', Path(directory)), patch('sys.argv', ['check.py', '--suite', 'fmt']), \
                        patch.object(check.subprocess, 'run', side_effect=run), \
                        patch.object(check.subprocess, 'check_output', return_value=b''), redirect_stdout(io.StringIO()):
                    self.assertEqual(check.main(), 0)

    def test_failure_stdout_is_visible_despite_long_compiler_stderr(self):
        import io
        from contextlib import redirect_stdout
        with tempfile.TemporaryDirectory() as directory:
            def run(command, **kwargs):
                if command[0] == 'git':
                    return subprocess.CompletedProcess(command, 0, stdout='f' * 40, stderr='')
                return subprocess.CompletedProcess(command, 101, stdout=b'SPECIFIC_FIXTURE_FAILURE', stderr=b'compiler noise\n' * 1000)
            captured = io.StringIO()
            with patch.object(check, 'ROOT', Path(directory)), patch('sys.argv', ['check.py', '--suite', 'rust-tests']), \
                    patch.object(check.subprocess, 'run', side_effect=run), \
                    patch.object(check.subprocess, 'check_output', return_value=b''), redirect_stdout(captured):
                self.assertEqual(check.main(), 1)
            self.assertIn('SPECIFIC_FIXTURE_FAILURE', captured.getvalue())

    def test_partial_success_is_identified_as_one_suite(self):
        status, receipt = self.run_fake_suite('fmt', fail=False)
        self.assertEqual(status, 0)
        self.assertTrue(receipt['all_requested_pass'])
        self.assertEqual([step['name'] for step in receipt['steps']], ['fmt'])


if __name__ == '__main__':
    unittest.main()
