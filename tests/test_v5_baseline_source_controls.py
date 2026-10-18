"""C006: executed red/positive controls for the 16 source-only V5-00 findings on frozen e1e9203 components.

Each red is a child run whose independent assertion fails on the frozen component; the parent checks
the exact failure code. Rust rows need the frozen workspace built under its own cargo profile
(`tools/v5_baseline_source_controls.py build-frozen-rust`) and run only when
JOIL_C006_FROZEN_RUST_DIR names that profile directory. Test scratch lives in the private worker
target (or the repository private test-evidence directory), never in TMPDIR.
"""
import contextlib
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
import venv

ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / 'tools/v5_baseline_source_controls.py'
RUST = os.environ.get('JOIL_C006_FROZEN_RUST_DIR')
SOURCE_ONLY = ['D-05', 'D-07', 'D-10', 'D-14', 'D-16', 'D-17', 'D-18', 'D-21', 'D-22', 'D-24', 'D-25',
               'D-27', 'D-28', 'D-29', 'D-30', 'D-31']
RED_CODES = {
    'D-05': 'sealed_pocket_not_reported_as_powder',
    'D-07': 'mass_change_unsupported_by_compiled_solid',
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


def scratch_base():
    """The absolute private worker target when set, otherwise the repository private test-evidence directory."""
    target = os.environ.get('CARGO_TARGET_DIR')
    if target and Path(target).is_absolute() and '.private' in Path(target).parts:
        return Path(target)
    return ROOT / '.private/test-evidence'


@contextlib.contextmanager
def scratch():
    base = scratch_base()
    base.mkdir(parents=True, exist_ok=True)
    path = Path(tempfile.mkdtemp(prefix='v5-test-c006-', dir=base))
    try:
        yield path
    except BaseException:
        print(f'retained failed-test scratch: {path}', file=sys.stderr)
        raise
    shutil.rmtree(path)


def control(row, mode, root, run, *extra):
    """One control child with --scratch root/run; its TMPDIR is pinned inside the same test scratch."""
    tmp = root / 'tmp'
    tmp.mkdir(exist_ok=True)
    argv = [sys.executable, '-I', str(TOOL), 'run', row, mode, '--scratch', str(root / run), *extra]
    return subprocess.run(argv, capture_output=True, text=True, cwd=ROOT, timeout=600, env={**os.environ, 'TMPDIR': str(tmp)})


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def recorded_sources(row):
    defects = json.loads((ROOT / 'docs/v5-plan/execution/DEFECTS.json').read_text())['defects']
    return {(s['path'], s['source_sha256']) for d in defects if d['id'] == row for s in d['sources']}


class ControlAssertions:
    def assert_pair(self, row, *extra):
        with scratch() as path:
            proc = control(row, 'positive', path, 'positive', *extra)
            self.assertEqual(proc.returncode, 0, proc.stderr)
            result = json.loads(proc.stdout)
            self.assertTrue(result['assertion_holds'])
            self.assertEqual(result['frozen_commit'], 'e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0')
            bound = {(b['path'], b['sha256']) for b in result['bindings'] if b['recorded']}
            self.assertTrue(bound and bound <= recorded_sources(row), result['bindings'])
            if 'frozen_rust' in result:
                self.assert_rust_provenance(path / 'positive', result)
            proc = control(row, 'red', path, 'red', *extra)
            self.assertEqual(proc.returncode, 1, proc.stderr)
            self.assertTrue(proc.stderr.strip().splitlines()[-1].startswith(f'C006-RED {row}: {RED_CODES[row]} '),
                            proc.stderr)
            result = json.loads(proc.stdout)
            self.assertFalse(result['assertion_holds'])
            self.assertTrue(result['red_reason'].startswith(RED_CODES[row] + ' '))
            self.assertEqual(sorted(p.name for p in path.iterdir()), ['positive', 'red', 'tmp'])

    def assert_rust_provenance(self, run, result):
        meta = result['frozen_rust']
        self.assertEqual(meta['profile'], 'c006frozen')
        self.assertTrue(meta['toolchain'].get('release') and meta['toolchain'].get('host'))
        for name, digest in meta['artifact_sha256'].items():
            self.assertEqual(digest, sha(Path(RUST) / name))
        if (run / 'c006-probe').exists():
            self.assertEqual(meta['probe']['sha256'], sha(run / 'c006-probe'))
            self.assertEqual(meta['probe']['source_sha256'], sha(run / 'c006-probe.rs'))
        else:
            self.assertNotIn('probe', meta)


class MatrixTests(unittest.TestCase):
    def test_interpreter_library_under_checkout_is_allowed_but_source_and_symlink_leaks_are_refused(self):
        with scratch() as path:
            checkout = path / 'checkout'
            environment = checkout / '.private/ci-python'
            venv.EnvBuilder(with_pip=False, symlinks=True).create(environment)
            python = environment / 'bin/python'
            info = subprocess.run([str(python), '-I', '-c',
                                   'import json,sysconfig; print(json.dumps(sysconfig.get_paths()))'],
                                  check=True, capture_output=True, text=True)
            libraries = json.loads(info.stdout)
            purelib = Path(libraries['purelib'])
            purelib.mkdir(parents=True, exist_ok=True)
            (purelib / 'synthetic_dependency.py').write_text('VALUE = 42\n')
            source = checkout / 'tools/current_source.py'
            source.parent.mkdir(parents=True)
            source.write_text('VALUE = 17\n')
            (purelib / 'synthetic_source_link.py').symlink_to(source)
            frozen = path / 'frozen'
            frozen.mkdir()
            cases = {
                'dependency': ('import synthetic_dependency\nVALUE = synthetic_dependency.VALUE\n', 0),
                'source': (f'import importlib.util, sys\ns = importlib.util.spec_from_file_location("leaked_source", {str(source)!r})\nm = importlib.util.module_from_spec(s)\nsys.modules["leaked_source"] = m\ns.loader.exec_module(m)\n', 2),
                'symlink': ('import synthetic_source_link\n', 2),
            }
            for name, (body, expected) in cases.items():
                with self.subTest(name=name):
                    module = frozen / (name + '.py')
                    module.write_text(body)
                    script = (f'import importlib.util, sys\nfrom pathlib import Path\n'
                              f's = importlib.util.spec_from_file_location("control_tool", {str(TOOL)!r})\n'
                              'tool = importlib.util.module_from_spec(s)\ns.loader.exec_module(tool)\n'
                              f'tool.ROOT = Path({str(checkout)!r})\n'
                              'try:\n'
                              f'    m = tool.load_module("frozen_case", Path({str(module)!r}), Path({str(frozen)!r}))\n'
                              'except tool.Setup as error:\n    print(str(error), file=sys.stderr)\n    sys.exit(2)\n'
                              'print(getattr(m, "VALUE", None))\n')
                    result = subprocess.run([str(python), '-I', '-c', script],
                                            capture_output=True, text=True, timeout=15)
                    self.assertEqual(result.returncode, expected, result.stderr)
                    if expected == 0:
                        self.assertEqual(result.stdout.strip(), '42')
                    else:
                        self.assertIn('current-checkout module leaked', result.stderr)

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
                proc = control(row, 'red', path, 'run')
                self.assertEqual(proc.returncode, 3)
                self.assertEqual(proc.stdout, '')
                self.assertTrue(proc.stderr.startswith(f'C006-BLOCKED {row}: '))
                self.assertFalse((path / 'run').exists())


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


class ScratchPlacementTests(unittest.TestCase):
    def test_tmpdir_cannot_redirect_test_scratch_outside_the_declared_target(self):
        base = scratch_base()
        base.mkdir(parents=True, exist_ok=True)
        decoy = Path(tempfile.mkdtemp(prefix='v5-test-c006-tmpdir-decoy-', dir=base))
        saved_env, saved_tempdir = os.environ.get('TMPDIR'), tempfile.tempdir
        try:
            os.environ['TMPDIR'] = str(decoy)
            tempfile.tempdir = None  # tempfile re-reads TMPDIR, so the redirect is live
            self.assertEqual(Path(tempfile.gettempdir()), decoy)
            with scratch() as path:
                self.assertEqual(path.parent, base)
                self.assertFalse(path.is_relative_to(decoy))
                proc = control('D-10', 'positive', path, 'run')
                self.assertEqual(proc.returncode, 0, proc.stderr)
            self.assertFalse(path.exists())
            self.assertEqual(list(decoy.iterdir()), [])
        finally:
            if saved_env is None:
                os.environ.pop('TMPDIR', None)
            else:
                os.environ['TMPDIR'] = saved_env
            tempfile.tempdir = saved_tempdir
            shutil.rmtree(decoy)

    def test_base_is_the_private_absolute_target_or_repository_test_evidence(self):
        saved = os.environ.get('CARGO_TARGET_DIR')
        try:
            for value, expected in (('relative/target', ROOT / '.private/test-evidence'),
                                    ('/srv/shared/target', ROOT / '.private/test-evidence'),
                                    (str(ROOT / '.private/worker/target'), ROOT / '.private/worker/target')):
                os.environ['CARGO_TARGET_DIR'] = value
                self.assertEqual(scratch_base(), expected)
            os.environ.pop('CARGO_TARGET_DIR')
            self.assertEqual(scratch_base(), ROOT / '.private/test-evidence')
        finally:
            if saved is None:
                os.environ.pop('CARGO_TARGET_DIR', None)
            else:
                os.environ['CARGO_TARGET_DIR'] = saved


class ScratchRefusalTests(unittest.TestCase):
    def test_stale_probe_in_scratch_is_setup_and_never_executed(self):
        with scratch() as path:
            run = path / 'run'
            run.mkdir()
            marker = path / 'stale-probe-executed'
            stale = run / 'c006-probe'
            stale.write_text(f'#!/bin/sh\n# SYNTHETIC stale probe\ntouch "{marker}"\necho "{{}}"\n')
            stale.chmod(0o755)
            before = stale.read_bytes()
            proc = control('D-05', 'red', path, 'run', '--frozen-rust', RUST or str(path / 'no-frozen-profile'))
            self.assertEqual(proc.returncode, 2, proc.stderr)
            self.assertIn('C006-SETUP D-05 red: scratch is not empty', proc.stderr)
            self.assertNotIn('C006-RED', proc.stderr)
            self.assertEqual(proc.stdout, '')
            self.assertFalse(marker.exists())
            self.assertEqual(stale.read_bytes(), before)
            self.assertEqual(sorted(p.name for p in run.iterdir()), ['c006-probe'])

    def test_probe_not_compiled_by_this_invocation_is_never_executed(self):
        spec = importlib.util.spec_from_file_location('c006_tool_under_test', TOOL)
        tool = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(tool)
        with scratch() as path:
            marker = path / 'stale-probe-executed'
            stale = path / 'c006-probe'
            stale.write_text(f'#!/bin/sh\ntouch "{marker}"\n')
            stale.chmod(0o755)
            with self.assertRaisesRegex(tool.Setup, 'pre-existing probe refused and never executed'):
                tool.probe(path / 'no-frozen-profile', path, 'd05', 'open')
            self.assertFalse(marker.exists())

    def test_nonempty_or_file_scratch_is_refused_for_python_rows(self):
        with scratch() as path:
            (path / 'nonempty').mkdir()
            prior = path / 'nonempty/prior-diagnostic.log'
            prior.write_bytes(b'SYNTHETIC earlier diagnostic; must survive\n')
            (path / 'file').write_bytes(b'SYNTHETIC file in place of a run directory\n')
            for run, reason in (('nonempty', 'scratch is not empty'), ('file', 'scratch is not a directory')):
                with self.subTest(run=run):
                    proc = control('D-10', 'positive', path, run)
                    self.assertEqual(proc.returncode, 2, proc.stderr)
                    self.assertIn(f'C006-SETUP D-10 positive: {reason}', proc.stderr)
                    self.assertEqual(proc.stdout, '')
            self.assertEqual(prior.read_bytes(), b'SYNTHETIC earlier diagnostic; must survive\n')
            self.assertEqual(sorted(p.name for p in (path / 'nonempty').iterdir()), ['prior-diagnostic.log'])
            self.assertEqual((path / 'file').read_bytes(), b'SYNTHETIC file in place of a run directory\n')

    def test_fresh_or_empty_owned_run_directory_is_accepted(self):
        with scratch() as path:
            (path / 'empty').mkdir()
            for run in ('empty', 'fresh/nested'):
                with self.subTest(run=run):
                    proc = control('D-10', 'positive', path, run)
                    self.assertEqual(proc.returncode, 0, proc.stderr)


class BindingFailsClosedTests(unittest.TestCase):
    def test_tampered_recorded_source_hash_is_setup_not_evidence(self):
        with scratch() as path:
            defects = json.loads((ROOT / 'docs/v5-plan/execution/DEFECTS.json').read_text())
            row = next(d for d in defects['defects'] if d['id'] == 'D-14')
            row['sources'][0]['source_sha256'] = '0' * 64
            (path / 'defects.json').write_text(json.dumps(defects))
            proc = control('D-14', 'red', path, 'run', '--defects', str(path / 'defects.json'))
            self.assertEqual(proc.returncode, 2)
            self.assertIn('C006-SETUP D-14 red: binding mismatch physics/openfoam/oil/oil_grade.py', proc.stderr)
            self.assertEqual(proc.stdout, '')

    def test_missing_or_malformed_defects_inventory_is_setup_not_traceback(self):
        defects = json.loads((ROOT / 'docs/v5-plan/execution/DEFECTS.json').read_text())
        bad_hash = json.loads(json.dumps(defects))
        next(d for d in bad_hash['defects'] if d['id'] == 'D-10')['sources'][0]['source_sha256'] = 'not-a-sha'
        cases = {'missing.json': None, 'not-json.json': b'{not json', 'not-a-list.json': b'{"defects": "x"}',
                 'row-absent.json': json.dumps({'defects': []}).encode(), 'bad-hash.json': json.dumps(bad_hash).encode()}
        with scratch() as path:
            for name, content in cases.items():
                with self.subTest(case=name):
                    if content is not None:
                        (path / name).write_bytes(content)
                    proc = control('D-10', 'positive', path, f'run-{name}', '--defects', str(path / name))
                    self.assertEqual(proc.returncode, 2, proc.stderr)
                    self.assertIn('C006-SETUP D-10 positive:', proc.stderr)
                    self.assertNotIn('Traceback', proc.stderr)
                    self.assertEqual(proc.stdout, '')

    def test_rust_row_without_frozen_build_is_setup_not_red(self):
        for extra in ([], ['--frozen-rust', 'nonexistent-c006-profile-dir']):
            with self.subTest(extra=extra), scratch() as path:
                proc = control('D-24', 'red', path, 'run', *extra)
                self.assertEqual(proc.returncode, 2)
                self.assertIn('C006-SETUP D-24 red:', proc.stderr)
                self.assertNotIn('C006-RED', proc.stderr)


@unittest.skipUnless(RUST, 'opt-in: set JOIL_C006_FROZEN_RUST_DIR to target/c006frozen built from the frozen tree')
class RustControlTests(ControlAssertions, unittest.TestCase):
    def test_d05_sealed_pocket_counts_as_fluid_but_never_powder(self):
        self.assert_pair('D-05', '--frozen-rust', RUST)

    def test_d07_mass_changes_when_compiled_solid_volume_is_unchanged(self):
        self.assert_pair('D-07', '--frozen-rust', RUST)

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
