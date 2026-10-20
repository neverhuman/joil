"""Check selection and receipts must preserve coverage and fail closed."""
import importlib.util
import os
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess
import shutil

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

    def run_fake_suite(self, suite, fail, *, fail_command=None, error=None, cad=False, expected_node=None):
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
                if expected_node is not None:
                    self.assertEqual(shutil.which('node', path=kwargs['env']['PATH']), str(expected_node))
                failed = fail and (fail_command is None or command == fail_command)
                if failed and error is not None:
                    raise error
                return subprocess.CompletedProcess(command, 1 if failed else 0, stdout=b'fixture output', stderr=b'')

            argv = ['check.py'] + (['--suite', suite] if suite else []) + (['--cad'] if cad else [])
            with patch.object(check, 'ROOT', root), patch('sys.argv', argv), \
                    patch.object(check.subprocess, 'run', side_effect=run), \
                    patch.object(check.subprocess, 'check_output', return_value=b''):
                status = check.main()
            receipts = list((root / '.private/checks').glob('*/receipt.json'))
            self.assertEqual(len(receipts), 1)
            receipt = json.loads(receipts[0].read_text())
            self.assertEqual(receipt['requested_suite'], suite or 'all')
            self.assertEqual(receipt['ui_selected'], suite in (None, 'ui'))
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
        self.assertFalse(receipt['ui_attempted'])
        self.assertFalse(receipt['ui_executed'])
        self.assertFalse(receipt['ui_passed'])

    def test_early_abort_keeps_selected_ui_and_cad_unexecuted(self):
        status, receipt = self.run_fake_suite(None, fail=True, cad=True)
        self.assertEqual(status, 1)
        self.assertTrue(receipt['ui_selected'])
        self.assertTrue(receipt['cad_selected'])
        for feature in ('ui', 'cad'):
            for flag in ('attempted', 'executed', 'passed'):
                self.assertFalse(receipt[f'{feature}_{flag}'])

    def test_ui_build_selection_attempt_execution_and_success_are_distinct(self):
        build = ['npm', 'run', 'build']
        cases = [
            (False, None, 0, True, True),
            (True, None, 1, True, False),
            (True, FileNotFoundError('missing build executable'), 1, False, False),
            (True, subprocess.TimeoutExpired(build, 1100), 1, True, False),
        ]
        for fail, error, expected_status, executed, passed in cases:
            with self.subTest(fail=fail, error=error):
                status, receipt = self.run_fake_suite('ui', fail=fail, fail_command=build, error=error)
                self.assertEqual(status, expected_status)
                self.assertTrue(receipt['ui_selected'])
                self.assertTrue(receipt['ui_attempted'])
                self.assertEqual(receipt['ui_executed'], executed)
                self.assertEqual(receipt['ui_passed'], passed)
                self.assertEqual(receipt['all_requested_pass'], passed)

    def test_complete_suite_records_actual_ui_and_optional_cad_execution(self):
        status, receipt = self.run_fake_suite(None, fail=False, cad=True)
        self.assertEqual(status, 0)
        self.assertTrue(receipt['all_requested_pass'])
        for feature in ('ui', 'cad'):
            for flag in ('selected', 'attempted', 'executed', 'passed'):
                self.assertTrue(receipt[f'{feature}_{flag}'])

    def test_worktree_finds_shared_project_local_node_before_system_node(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            node = home / 'joil/.private/node-runtime/node_modules/.bin/node'
            node.parent.mkdir(parents=True)
            node.write_text('#!/bin/sh\nexit 0\n')
            node.chmod(0o700)
            with patch.object(check.Path, 'home', return_value=home):
                status, receipt = self.run_fake_suite('ui', fail=False, expected_node=node)
            self.assertEqual(status, 0)
            self.assertTrue(receipt['ui_passed'])

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
