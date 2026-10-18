import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "render/blender"))
from plate import verified_completed_frames

PLATE = ROOT / "render/blender/plate.py"
FIXTURES = ROOT / "tests/fixtures/v5-scenes"
RECIPE = ROOT / "recipes/v5/registry.json"


class PlatePreflightTests(unittest.TestCase):
    def run_plate(self, bundle, output, *extra, dry_run=True):
        env = os.environ.copy()
        env.update({"CARGO_INCREMENTAL": "0", "RAYON_NUM_THREADS": "1",
                    "OMP_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1",
                    "MKL_NUM_THREADS": "1"})
        command = ["python3", str(PLATE), "--bundle", str(bundle), "--recipe", str(RECIPE),
                   "--output", str(output), "--frames", "1", *extra]
        if dry_run:
            command.append("--dry-run")
        return subprocess.run(command, cwd=ROOT, env=env, capture_output=True,
                              text=True, check=False)

    def test_unavailable_card_is_an_honest_dry_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = self.run_plate(FIXTURES / "unavailable-oil-card.json", Path(tmp))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('"valid": true', result.stdout)
        self.assertIn('"evidence_class": "Unavailable"', result.stdout)

    def test_semantic_mutations_fail_in_dry_run(self):
        invalid = ("wrong-unit.json", "wrong-subject.json", "wrong-time.json",
                   "wrong-transform.json", "private-geometry-unapproved.json")
        with tempfile.TemporaryDirectory() as tmp:
            for name in invalid:
                with self.subTest(name=name):
                    result = self.run_plate(FIXTURES / name, Path(tmp) / name)
                    self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_invalid_bundle_never_launches_renderer(self):
        with tempfile.TemporaryDirectory() as tmp:
            marker = Path(tmp) / "launched"
            fake = Path(tmp) / "fake-blender"
            fake.write_text("#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'fake blender 1'; else touch '" +
                            str(marker) + "'; fi\n")
            fake.chmod(0o755)
            subject = Path(tmp) / "subject.bin"
            subject.write_bytes(b"unrelated subject")
            result = self.run_plate(FIXTURES / "wrong-unit.json", Path(tmp) / "out",
                                    "--blender", str(fake), "--build-id", "fake blender 1",
                                    "--subject", str(subject), dry_run=False)
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertFalse(marker.exists())

    def test_resume_requires_existing_frame_and_matching_digest(self):
        from hashlib import sha256
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp)
            frame = output / "frame-000000.png"
            frame.write_bytes(b"frame bytes")
            manifest = {"completed_frames": [0, 1], "frame_sha256": {
                "0": sha256(b"frame bytes").hexdigest(), "1": sha256(b"missing").hexdigest()}}
            self.assertEqual(verified_completed_frames(manifest, output), [0])
            frame.write_bytes(b"tampered")
            self.assertEqual(verified_completed_frames(manifest, output), [])


if __name__ == "__main__":
    unittest.main()
