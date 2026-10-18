"""Release-scope and parity checks for the V5-31 software handoff.

These checks verify software/release contracts only. They do not qualify a
physical design or promote scientific evidence.
"""
import json
from pathlib import Path
import re
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools import check_public  # noqa: E402


class V5ReleaseScopeTests(unittest.TestCase):
    def test_v5_31_01_to_05_release_contracts_and_truthful_status(self):
        status = json.loads((ROOT / "docs/STATUS.json").read_text(encoding="utf-8"))
        for claim in ("qualified_baseline", "structural_qualification", "physical_validation", "oil_grade_evaluated"):
            self.assertIs(status[claim], False, f"{claim} must remain false without physical evidence")
            self.assertEqual(status["claims"][claim]["state"], "incomplete")
        self.assertIsNone(status["qualified_fitness"])
        self.assertIsNone(status["qualified_capacity_gain"])
        self.assertTrue(status["legacy_status_is_historical_not_authoritative"])

        generated = subprocess.run(
            [sys.executable, str(ROOT / "tools/gen_status.py"), "--check"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(generated.returncode, 0, generated.stdout + generated.stderr)

        workflow = (ROOT / ".github/workflows/foundation.yml").read_text(encoding="utf-8")
        matrix = re.search(r"(?m)^\s*suite:\s*\[([^\]]+)\]", workflow)
        self.assertIsNotNone(matrix, "foundation workflow must define its suite matrix")
        suites = {item.strip() for item in matrix.group(1).split(",")}
        self.assertEqual(
            suites,
            {"fmt", "clippy", "rust-tests", "python-tests", "baseline", "ui", "claims-audit", "public-check"},
        )

    def test_publication_guard_rejects_private_paths_and_deny_terms(self):
        self.assertIn("private/generated path", check_public.violations(".private/input.json", b"public text"))
        self.assertIn(
            "private publication deny rule",
            check_public.violations("docs/release-note.md", b"synthetic fixture", (b"synthetic fixture",)),
        )
        self.assertEqual(check_public.violations("docs/release-note.md", b"ordinary public text", (b"private phrase",)), [])


if __name__ == "__main__":
    unittest.main()
