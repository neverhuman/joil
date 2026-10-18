"""C006: executed red/positive controls for the 16 source-only V5-00 findings on frozen e1e9203 components.

Each red is a child run whose independent assertion fails on the frozen component; the parent checks
the exact failure code. Rust rows need the frozen workspace built under its own cargo profile
(`tools/v5_baseline_source_controls.py build-frozen-rust`) and run only when
JOIL_C006_FROZEN_RUST_DIR names that profile directory.
"""
import contextlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / 'tools/v5_baseline_source_controls.py'
RUST = os.environ.get('JOIL_C006_FROZEN_RUST_DIR')
SOURCE_ONLY = ['D-05', 'D-07', 'D-10', 'D-14', 'D-16', 'D-17', 'D-18', 'D-21', 'D-22', 'D-24', 'D-25',
               'D-27', 'D-28', 'D-29', 'D-30', 'D-31']
RED_CODES = {
    'D-05': 'sealed_pocket_not_reported_as_powder',
    'D-10': 'requirement_sources_disagree',
    'D-14': 'seed_share_ignores_inward_flux',
    'D-16': 'wall_authorities_disagree',
    'D-17': 'gpu_child_launched_without_lease',
    'D-22': 'modal_series_without_evidence',
    'D-24': 'existing_destination_overwritten',
    'D-25': 'policy_admits_agent_proposals_in_search_loop',
    'D-27': 'candidate_without_disclosure_class_admitted',
    'D-30': 'explicit_state_disagrees',
    'D-31': 'revoked_claim_still_accepted',
}


@contextlib.contextmanager
def scratch():
    path = Path(tempfile.mkdtemp(prefix='c006-'))
    try:
        yield path
    except BaseException:
        print(f'C006 diagnostics retained at {path}', file=sys.stderr)
        raise
    shutil.rmtree(path)


def control(row, mode, path, *extra):
    argv = [sys.executable, '-I', str(TOOL), 'run', row, mode, '--scratch', str(path), *extra]
    return subprocess.run(argv, capture_output=True, text=True, cwd=ROOT, timeout=600)


def recorded_sources(row):
    defects = json.loads((ROOT / 'docs/v5-plan/execution/DEFECTS.json').read_text())['defects']
    return {(s['path'], s['source_sha256']) for d in defects if d['id'] == row for s in d['sources']}


class ControlAssertions:
    def assert_pair(self, row, *extra):
        with scratch() as path:
            proc = control(row, 'positive', path / 'positive', *extra)
            self.assertEqual(proc.returncode, 0, proc.stderr)
            result = json.loads(proc.stdout)
            self.assertTrue(result['assertion_holds'])
            self.assertEqual(result['frozen_commit'], 'e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0')
            bound = {(b['path'], b['sha256']) for b in result['bindings'] if b['recorded']}
            self.assertTrue(bound and bound <= recorded_sources(row), result['bindings'])
            proc = control(row, 'red', path / 'red', *extra)
            self.assertEqual(proc.returncode, 1, proc.stderr)
            self.assertTrue(proc.stderr.strip().splitlines()[-1].startswith(f'C006-RED {row}: {RED_CODES[row]} '),
                            proc.stderr)
            result = json.loads(proc.stdout)
            self.assertFalse(result['assertion_holds'])
            self.assertTrue(result['red_reason'].startswith(RED_CODES[row] + ' '))


class MatrixTests(unittest.TestCase):
    def test_every_source_only_row_appears_once_with_an_honest_kind(self):
        matrix = json.loads(subprocess.run([sys.executable, '-I', str(TOOL), 'matrix'], check=True,
                                           capture_output=True, text=True).stdout)
        gates = json.loads((ROOT / 'docs/v5-plan/execution/DEFECT_ACCEPTANCE_GATES.json').read_text())
        source_only = sorted(r['id'] for r in gates['records'] if r['inventory_evidence_kind'] == 'source_only_inventory')
        self.assertEqual(source_only, SOURCE_ONLY)
        self.assertEqual(sorted(matrix['rows']), SOURCE_ONLY)
        for row, spec in matrix['rows'].items():
            self.assertIn(spec['kind'], {'runtime', 'static', 'blocked'})
            if spec['kind'] == 'blocked':
                self.assertTrue(spec['blocker'].strip())
                self.assertNotIn(row, RED_CODES)
            else:
                self.assertTrue(spec['assertion'] and spec['positive'] and spec['red'])
                self.assertEqual(spec['assertion'].startswith('STATIC'), spec['kind'] == 'static')
                self.assertIn(row, RED_CODES)

    def test_blocked_rows_refuse_to_produce_evidence(self):
        matrix = json.loads(subprocess.run([sys.executable, '-I', str(TOOL), 'matrix'], check=True,
                                           capture_output=True, text=True).stdout)
        for row in [r for r, s in matrix['rows'].items() if s['kind'] == 'blocked']:
            with self.subTest(row=row), scratch() as path:
                proc = control(row, 'red', path)
                self.assertEqual(proc.returncode, 3)
                self.assertEqual(proc.stdout, '')
                self.assertTrue(proc.stderr.startswith(f'C006-BLOCKED {row}: '))


class PythonControlTests(ControlAssertions, unittest.TestCase):
    def test_d10_static_requirement_sources_agree_on_quality_but_not_pdr(self):
        self.assert_pair('D-10')

    def test_d14_seed_share_follows_speed_not_inward_flux(self):
        self.assert_pair('D-14')

    def test_d22_modal_curve_is_plotted_without_modal_evidence(self):
        self.assert_pair('D-22')

    def test_d25_static_policy_admits_llm_proposals_in_search_loop(self):
        self.assert_pair('D-25')

    def test_d27_publication_gate_admits_candidate_without_disclosure_class(self):
        self.assert_pair('D-27')

    def test_d31_revoked_claim_survives_on_an_unrelated_numeric_match(self):
        self.assert_pair('D-31')


class BindingFailsClosedTests(unittest.TestCase):
    def test_tampered_recorded_source_hash_is_setup_not_evidence(self):
        with scratch() as path:
            defects = json.loads((ROOT / 'docs/v5-plan/execution/DEFECTS.json').read_text())
            row = next(d for d in defects['defects'] if d['id'] == 'D-14')
            row['sources'][0]['source_sha256'] = '0' * 64
            (path / 'defects.json').write_text(json.dumps(defects))
            proc = control('D-14', 'red', path / 'run', '--defects', str(path / 'defects.json'))
            self.assertEqual(proc.returncode, 2)
            self.assertIn('C006-SETUP D-14 red: binding mismatch physics/openfoam/oil/oil_grade.py', proc.stderr)
            self.assertEqual(proc.stdout, '')

    def test_rust_row_without_frozen_build_is_setup_not_red(self):
        for extra in ([], ['--frozen-rust', 'nonexistent-c006-profile-dir']):
            with self.subTest(extra=extra), scratch() as path:
                proc = control('D-24', 'red', path, *extra)
                self.assertEqual(proc.returncode, 2)
                self.assertIn('C006-SETUP D-24 red:', proc.stderr)
                self.assertNotIn('C006-RED', proc.stderr)

    def test_existing_frozen_extract_is_refused(self):
        with scratch() as path:
            (path / 'frozen').mkdir()
            proc = control('D-10', 'positive', path)
            self.assertEqual(proc.returncode, 2)
            self.assertIn('scratch already holds a frozen extract', proc.stderr)


@unittest.skipUnless(RUST, 'opt-in: set JOIL_C006_FROZEN_RUST_DIR to target/c006frozen built from the frozen tree')
class RustControlTests(ControlAssertions, unittest.TestCase):
    def test_d05_sealed_pocket_counts_as_fluid_but_never_powder(self):
        self.assert_pair('D-05', '--frozen-rust', RUST)

    def test_d16_wall_authorities_disagree_on_a_recorded_tpms_elite(self):
        self.assert_pair('D-16', '--frozen-rust', RUST)

    def test_d17_gpu_child_launches_without_lease_admission(self):
        self.assert_pair('D-17', '--frozen-rust', RUST)

    def test_d24_existing_output_is_overwritten(self):
        self.assert_pair('D-24', '--frozen-rust', RUST)

    def test_d30_kpi_copies_disagree_on_overflow_state(self):
        self.assert_pair('D-30', '--frozen-rust', RUST)


if __name__ == '__main__':
    unittest.main()
