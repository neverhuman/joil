"""DOC-01/DOC-02: review inputs stay verbatim and every review P0 traces to a workplan item."""
from pathlib import Path
import re
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
    ledger_from_board,
    refresh_review_statuses,
    sync_ledger,
    verify_review_input_checksums,
)

LIVE_SYNTHESIS = (ROOT / REVIEW_SYNTHESIS).read_text(encoding='utf-8')
WORKPLAN = (ROOT / REVIEW_WORKPLAN).read_text(encoding='utf-8')
_tr09 = next(line for line in LIVE_SYNTHESIS.splitlines() if line.startswith('| TR-09 |'))
# Fixture independent of live progress: TR-09 forced OPEN, statuses re-derived.
SYNTHESIS = refresh_review_statuses(LIVE_SYNTHESIS.replace(_tr09, '| TR-09 | TR | OPEN | - |'), 'fixture')


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
        errors, rows = check_review_traceability(LIVE_SYNTHESIS, WORKPLAN)
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
        norm = lambda text: re.sub(r'\*\*Status as of [^*]+\.\*\*', '', text)
        self.assertEqual(norm(refresh_review_statuses(LIVE_SYNTHESIS, stamp)), norm(LIVE_SYNTHESIS))
        ledger_row = next(line for line in SYNTHESIS.splitlines() if line.startswith('| TR-09 |'))
        changed = SYNTHESIS.replace(ledger_row, '| TR-09 | TR | MERGED | `abc1234` |')
        errors, _ = check_review_traceability(changed, WORKPLAN)
        self.assertTrue(any(e.startswith('finding S3-P0-07: status') for e in errors), errors)
        refreshed = refresh_review_statuses(changed, stamp)
        self.assertEqual(check_review_traceability(refreshed, WORKPLAN)[0], [])
        row = next(line for line in refreshed.splitlines() if line.startswith('| S3-P0-07 |'))
        self.assertTrue(row.endswith('| MERGED |'), row)

    def test_ledger_sync_from_board_lines(self):
        board = '\n'.join([
            'HB 2026-10-16T05:00:00Z TR TR-09 START - beginning',
            'DONE TR-09 1eeff1000c2328dc8c81df8e736758dc2f944957',
            'MERGED TR-09 04415616b94bb892ff6b719bb040eeef9800dbec',
            'HB 2026-10-16T05:01:00Z TR TR-12/TR-15 BLOCKED 9409804 needs scope decision',
            'HB 2026-10-16T05:02:00Z IR IR-03 WIP - parity work',
            'DONE XX-99 abcdef1',
        ])
        items = {'TR-09', 'TR-12', 'TR-15', 'IR-03'}
        ledger = ledger_from_board(board, items)
        self.assertEqual(ledger['TR-09'][0], 'MERGED')
        self.assertIn('0441561', ledger['TR-09'][1])
        self.assertEqual(ledger['TR-12'][0], 'BLOCKED')
        self.assertEqual(ledger['TR-15'][0], 'BLOCKED')
        self.assertEqual(ledger['IR-03'][0], 'WIP')
        synced = sync_ledger(SYNTHESIS, board)
        row = next(line for line in synced.splitlines() if line.startswith('| TR-09 |'))
        self.assertIn('| MERGED |', row)
        refreshed = refresh_review_statuses(synced, 'T')
        self.assertEqual(check_review_traceability(refreshed, WORKPLAN)[0], [])
        # A board that only says START never downgrades an item the ledger already records as MERGED.
        regressed = sync_ledger(synced, 'HB 2026-10-16T06:00:00Z TR TR-09 START - again')
        self.assertIn('| MERGED |', next(l for l in regressed.splitlines() if l.startswith('| TR-09 |')))

    def test_merge_without_done_is_not_completion(self):
        board = '\n'.join([
            'HB 2026-10-16T05:00:00Z DOC DOC-04 WIP abc1234 section pending',
            'MERGED DOC-04 0ef2b88',
            'HB 2026-10-16T05:01:00Z TR TR-16 BLOCKED dcaf087 physical admission needs sources',
            'MERGED TR-16 ceaa03d',
        ])
        ledger = ledger_from_board(board, {'DOC-04', 'TR-16'})
        self.assertEqual(ledger['DOC-04'][0], 'WIP')
        self.assertIn('without board `DONE DOC-04`', ledger['DOC-04'][1])
        self.assertEqual(ledger['TR-16'][0], 'BLOCKED')
        done = ledger_from_board(board + '\nDONE DOC-04 978b23a', {'DOC-04'})
        self.assertEqual(done['DOC-04'][0], 'MERGED')

    def test_gap_rows_must_cite_a_gap_entry(self):
        row = next(line for line in SYNTHESIS.splitlines() if line.startswith('| S2-4.09 |'))
        errors, _ = check_review_traceability(SYNTHESIS.replace(row, row.replace('(§5 G-01)', '(no owner)')), WORKPLAN)
        self.assertIn('finding S2-4.09: GAP coverage must cite a §5 gap row', errors)


if __name__ == '__main__':
    unittest.main()
