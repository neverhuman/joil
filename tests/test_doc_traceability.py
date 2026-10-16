"""DOC-01/DOC-02: review inputs stay verbatim and every review P0 traces to a workplan item."""
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.claims_audit import (
    REQUIRED_REVIEW_FINDINGS,
    REVIEW_INPUT_DIR,
    REVIEW_SYNTHESIS,
    REVIEW_WORKPLAN,
    check_review_traceability,
    refresh_review_statuses,
    verify_review_input_checksums,
)

SYNTHESIS = (ROOT / REVIEW_SYNTHESIS).read_text(encoding='utf-8')
WORKPLAN = (ROOT / REVIEW_WORKPLAN).read_text(encoding='utf-8')


class ReviewInputTests(unittest.TestCase):
    def test_committed_inputs_match_checksums(self):
        errors, count = verify_review_input_checksums(ROOT / REVIEW_INPUT_DIR)
        self.assertEqual(errors, [])
        self.assertEqual(count, 6)

    def test_tampered_or_unlisted_input_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            copy = Path(tmp) / 'inputs'
            shutil.copytree(ROOT / REVIEW_INPUT_DIR, copy)
            victim = sorted(copy.glob('S3-*.md'))[0]
            victim.write_bytes(victim.read_bytes() + b'\n')
            (copy / 'S7-unlisted.md').write_text('extra', encoding='utf-8')
            errors, _ = verify_review_input_checksums(copy)
        self.assertTrue(any(victim.name in e and 'sha256' in e for e in errors), errors)
        self.assertTrue(any('S7-unlisted.md' in e for e in errors), errors)


class TraceabilityTests(unittest.TestCase):
    def test_committed_synthesis_traces_every_required_finding(self):
        errors, rows = check_review_traceability(SYNTHESIS, WORKPLAN)
        self.assertEqual(errors, [])
        self.assertGreaterEqual(rows, len(REQUIRED_REVIEW_FINDINGS))

    def test_missing_required_finding_fails(self):
        row = next(line for line in SYNTHESIS.splitlines() if line.startswith('| S3-P0-07 |'))
        errors, _ = check_review_traceability(SYNTHESIS.replace(row + '\n', ''), WORKPLAN)
        self.assertIn('finding S3-P0-07: required review finding not traced', errors)

    def test_unknown_workplan_item_fails(self):
        row = next(line for line in SYNTHESIS.splitlines() if line.startswith('| S3-P0-07 |'))
        errors, _ = check_review_traceability(SYNTHESIS.replace(row, row.replace('| TR-09 |', '| TR-99 |')), WORKPLAN)
        self.assertTrue(any('S3-P0-07' in e and 'TR-99' in e for e in errors), errors)

    def test_status_cannot_run_ahead_of_ledger(self):
        row = next(line for line in SYNTHESIS.splitlines() if line.startswith('| S3-P0-07 |'))
        self.assertTrue(row.endswith('| OPEN |'), row)
        errors, _ = check_review_traceability(SYNTHESIS.replace(row, row[: -len('OPEN |')] + 'MERGED |'), WORKPLAN)
        self.assertIn("finding S3-P0-07: status 'MERGED' but ledger derives 'OPEN'", errors)

    def test_ledger_done_requires_sha_and_drives_derived_status(self):
        open_row = '| TR-09 | TR | OPEN | - |'
        self.assertIn(open_row, SYNTHESIS)
        errors, _ = check_review_traceability(SYNTHESIS.replace(open_row, '| TR-09 | TR | DONE | - |'), WORKPLAN)
        self.assertIn('ledger: TR-09 is DONE without a commit sha', errors)
        # With a sha, the single-item finding S3-P0-07 must now be reported as DONE, not OPEN.
        errors, _ = check_review_traceability(SYNTHESIS.replace(open_row, '| TR-09 | TR | DONE | `abc1234` |'), WORKPLAN)
        self.assertIn("finding S3-P0-07: status 'OPEN' but ledger derives 'DONE'", errors)

    def test_blocked_item_dominates_and_needs_reason(self):
        open_row = '| TR-09 | TR | OPEN | - |'
        errors, _ = check_review_traceability(SYNTHESIS.replace(open_row, '| TR-09 | TR | BLOCKED | - |'), WORKPLAN)
        self.assertIn('ledger: TR-09 is BLOCKED without a reason', errors)
        self.assertIn("finding S3-P0-07: status 'OPEN' but ledger derives 'BLOCKED'", errors)

    def test_refresh_derives_statuses_from_ledger(self):
        stamp = '2026-10-16T05:23Z'
        self.assertEqual(refresh_review_statuses(SYNTHESIS, stamp).replace(stamp, ''),
                         SYNTHESIS.replace(SYNTHESIS.split('**Status as of ')[1].split('.**')[0], ''))
        ledger_row = next(line for line in SYNTHESIS.splitlines() if line.startswith('| TR-09 |'))
        changed = SYNTHESIS.replace(ledger_row, '| TR-09 | TR | MERGED | `abc1234` |')
        errors, _ = check_review_traceability(changed, WORKPLAN)
        self.assertTrue(any(e.startswith('finding S3-P0-07: status') for e in errors), errors)
        refreshed = refresh_review_statuses(changed, stamp)
        self.assertEqual(check_review_traceability(refreshed, WORKPLAN)[0], [])
        row = next(line for line in refreshed.splitlines() if line.startswith('| S3-P0-07 |'))
        self.assertTrue(row.endswith('| MERGED |'), row)

    def test_gap_rows_must_cite_a_gap_entry(self):
        row = next(line for line in SYNTHESIS.splitlines() if line.startswith('| S2-4.09 |'))
        errors, _ = check_review_traceability(SYNTHESIS.replace(row, row.replace('(§5 G-01)', '(no owner)')), WORKPLAN)
        self.assertIn('finding S2-4.09: GAP coverage must cite a §5 gap row', errors)


if __name__ == '__main__':
    unittest.main()
