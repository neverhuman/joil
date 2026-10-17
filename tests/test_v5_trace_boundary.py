"""Operational task receipts must not become uncited scientific provenance."""
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'tools'))
import claims_audit as audit


class OperationalTraceBoundary(unittest.TestCase):
    def test_current_scientific_trace_is_unchanged_by_operational_receipts(self):
        existing = (ROOT/audit.NUMBERS_TRACE).read_text()
        rendered, errors = audit.build_numbers_trace(ROOT, existing)
        self.assertEqual(errors, [])
        self.assertEqual(rendered, existing)

    def test_planted_matching_worker_allocation_needs_explicit_citation(self):
        existing = (ROOT/audit.NUMBERS_TRACE).read_text()
        planted_path = 'docs/receipts/v5-00-synthetic-worker-allocation.json'
        values = [(planted_path, '/physical_cores', 2.0)] + audit._trace_values(ROOT)
        with patch.object(audit, '_trace_values', return_value=values):
            rendered, errors = audit.build_numbers_trace(ROOT, existing)
            self.assertEqual(errors, [])
            self.assertEqual(rendered, existing)
            # This counterfactual removes the production guard. The planted
            # allocation then steals provenance for an unrelated paper numeral.
            old_policy = tuple(p for p in audit.TRACE_CITE_ONLY if not p.startswith('docs/receipts/'))
            with patch.object(audit, 'TRACE_CITE_ONLY', old_policy):
                unguarded, _ = audit.build_numbers_trace(ROOT, existing)
                self.assertIn(planted_path, unguarded)
                self.assertNotEqual(unguarded, existing)


if __name__ == '__main__':
    unittest.main()
