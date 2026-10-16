"""Synthetic byte-retention tests, not an OpenFOAM or physical validation run."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from truth_ev12_capture import capture_case, digest
from analysis.case_identity import finalize_case_identity, verify_case_identity
from analysis.evidence import evidence_reference

ROOT = Path(__file__).resolve().parents[3]


def synthetic_case(parent):
    case = parent / 'wave4-synthetic-g0-capture-test'
    program = {'synthetic': True, 'z': 2, 'a': 0}
    compact = lambda sorted_keys: json.dumps(program, sort_keys=sorted_keys,
                                             separators=(',', ':')).encode()
    files = {
        'design-program.json': json.dumps(program).encode(),
        'design-ir.json': json.dumps({'program_sha256': hashlib.sha256(compact(True)).hexdigest(),
                                      'ir_sha256': 'b' * 64}).encode(),
        'constant/triSurface/ct35.stl': b'solid synthetic\nendsolid\n',
        '0/U': b'synthetic initial velocity', '0/p': b'synthetic initial pressure',
        'system/controlDict': b'synthetic solver config',
        'system/fvSchemes': b'synthetic schemes', 'system/fvSolution': b'synthetic solution',
    }
    for name in ['points', 'faces', 'owner', 'neighbour', 'boundary']:
        files['constant/polyMesh/' + name] = ('synthetic ' + name).encode()
    for name, data in files.items():
        target = case / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    bundle = {key: evidence_reference(name, 'synthetic-v1', files[name])
              for key, name in [('genome', 'design-program.json'), ('ir', 'design-ir.json'),
                                ('surface_mesh', 'constant/triSurface/ct35.stl')]}
    for key in ['compiler', 'geometry_kernel', 'compiler_config']:
        bundle[key] = evidence_reference(key, 'synthetic-v1', b'synthetic fixture')
    (case / 'input-manifest.json').write_text(json.dumps({
        'design_sha256': hashlib.sha256(compact(False)).hexdigest(),
        'design_ir_sha256': 'b' * 64, 'surface_stl_relpath': 'constant/triSurface/ct35.stl',
        'surface_stl_sha256': hashlib.sha256(files['constant/triSurface/ct35.stl']).hexdigest(),
        'subject_bundle': bundle,
    }))
    return case


class CaptureTests(unittest.TestCase):
    def setUp(self):
        (ROOT / '.private').mkdir(exist_ok=True)
        temporary = tempfile.TemporaryDirectory(dir=ROOT / '.private', prefix='truth-ev12-test-')
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.case = synthetic_case(self.directory)
        self.capture = self.directory / 'capture'

    def test_exact_capture_survives_source_pruning_and_detects_archive_tamper(self):
        identity = finalize_case_identity(self.case)
        self.assertNotEqual(identity['canonical_genome_hash'],
                            identity['canonical_ir_source_genome_hash'])
        before = {p.relative_to(self.case).as_posix(): digest(p)
                  for p in self.case.rglob('*') if p.is_file()}
        receipt = capture_case(self.case, self.capture)
        self.assertEqual(receipt['files'], before)
        self.assertEqual(receipt['chain']['mesh_hash'], identity['mesh_hash'])
        self.assertEqual(before, {name: digest(self.case / name) for name in before})
        (self.case / 'constant/polyMesh/points').unlink()
        with self.assertRaises((OSError, ValueError)):
            verify_case_identity(self.case)
        verify_case_identity(self.capture)
        (self.capture / 'constant/polyMesh/points').write_bytes(b'tampered')
        with self.assertRaises(ValueError):
            verify_case_identity(self.capture)

    def test_missing_identity_or_pruned_mesh_never_produces_capture_receipt(self):
        with self.assertRaises(FileNotFoundError):
            capture_case(self.case, self.capture)
        self.assertFalse(self.capture.exists())
        finalize_case_identity(self.case)
        (self.case / 'constant/polyMesh/points').unlink()
        with self.assertRaises(ValueError):
            capture_case(self.case, self.capture)
        self.assertFalse(self.capture.exists())

    def test_stale_input_or_symlink_never_produces_capture(self):
        finalize_case_identity(self.case)
        path = self.case / '0/U'
        original = path.read_bytes()
        path.write_bytes(original + b' changed')
        with self.assertRaises(ValueError):
            capture_case(self.case, self.capture)
        self.assertFalse(self.capture.exists())
        path.unlink()
        outside = self.directory / 'substitute'
        outside.write_bytes(original)
        path.symlink_to(outside)
        with self.assertRaises(ValueError):
            capture_case(self.case, self.capture)
        self.assertFalse(self.capture.exists())

    def test_existing_or_nested_capture_is_refused(self):
        finalize_case_identity(self.case)
        capture_case(self.case, self.capture)
        receipt = digest(self.capture / 'tr-ev12-capture-receipt.json')
        with self.assertRaises(FileExistsError):
            capture_case(self.case, self.capture)
        self.assertEqual(digest(self.capture / 'tr-ev12-capture-receipt.json'), receipt)
        with self.assertRaises(ValueError):
            capture_case(self.case, self.case / 'capture')

    def test_cli_success_and_failure_exits(self):
        command = [sys.executable, str(Path(__file__).with_name('truth_ev12_capture.py')),
                   '--case', str(self.case), '--capture', str(self.capture)]
        missing = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(missing.returncode, 1, missing.stderr)
        self.assertIn('capture refused:', missing.stderr)
        finalize_case_identity(self.case)
        passed = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(passed.returncode, 0, passed.stderr)
        self.assertEqual(json.loads(passed.stdout)['state'],
                         'exact_indexed_inputs_captured_not_validation')


if __name__ == '__main__':
    unittest.main(verbosity=2)
