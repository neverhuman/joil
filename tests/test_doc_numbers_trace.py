"""DOC-04: every numeral in README and WHITEPAPER "Read this first", Abstract and §5 traces to a receipt or a checked declaration."""
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.claims_audit import (
    NUMBERS_TRACE,
    _affinity,
    _numeral_tokens,
    _unit_scales,
    build_numbers_trace,
    check_numbers_trace,
)


def _copy_tree(dst):
    for rel in ('docs/results', 'docs/receipts', 'experiments', 'baselines/ct35'):
        shutil.copytree(ROOT / rel, dst / rel)
    for rel in ('docs/STATUS.json', 'docs/WHITEPAPER.md', 'docs/SCIENCE.md', 'README.md', NUMBERS_TRACE,
                'crates/joil-core/tests/regression.rs'):
        (dst / rel).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(ROOT / rel, dst / rel)


class NumbersTraceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.base = Path(cls.tmp.name)
        _copy_tree(cls.base)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def _with(self, rel, old, new):
        path = self.base / rel
        original = path.read_text(encoding='utf-8')
        self.assertIn(old, original)
        path.write_text(original.replace(old, new, 1), encoding='utf-8')
        self.addCleanup(path.write_text, original, encoding='utf-8')

    def test_committed_trace_is_current_and_complete(self):
        self.assertEqual(check_numbers_trace(ROOT), [])
        self.assertNotIn('UNTRACED', (ROOT / NUMBERS_TRACE).read_text(encoding='utf-8'))

    def test_planted_untraced_number_fails(self):
        self._with('docs/WHITEPAPER.md', '## Abstract\n', '## Abstract\n\nThe elite separates `98.765` % of oil.\n')
        _, errors = build_numbers_trace(self.base)
        self.assertTrue(any("'98.765'" in e for e in errors), errors)

    def test_wrong_derived_declaration_fails(self):
        self._with(NUMBERS_TRACE, '| `+20.517319` | derived |', '| `+20.517000` | derived |')
        _, errors = build_numbers_trace(self.base)
        self.assertTrue(any('derived +20.517000' in e for e in errors), errors)

    def test_missing_literal_fails(self):
        self._with('crates/joil-core/tests/regression.rs', 'operator_chain_validity(20, 20261116)',
                   'operator_chain_validity(20, 20261117)')
        _, errors = build_numbers_trace(self.base)
        self.assertTrue(any('declared literal for 20261116' in e for e in errors), errors)

    def test_stale_trace_is_reported(self):
        self._with('docs/WHITEPAPER.md', '## Abstract\n', '## Abstract\n\nScreen evaluations: `119,235`.\n')
        self.assertTrue(any('stale' in e for e in check_numbers_trace(self.base)))

    def test_units_select_scale_and_quantity(self):
        self.assertIn((1e-3, '/1000'), _unit_scales(' kPa` vs'))
        self.assertNotIn((1e-3, '/1000'), _unit_scales(' mg/L` ('))
        self.assertIn((100.0, 'x100'), _unit_scales('%`)'))
        self.assertIn('_pa', _affinity(' kPa`'))
        self.assertIn('mg_l', _affinity('\\text{ mg/L}$'))
        self.assertIn('_pa', _affinity('–19.53 kPa`'))  # unit after the far end of a range

    def test_identifiers_dates_and_lists_are_not_numerals(self):
        tokens = _numeral_tokens('`CAD-09`, S3 P0-09, G2, CT-35, 16 October 2026, grid `[18,12,212]`, 5.6\\times 10^{-5}')
        self.assertEqual(tokens, ['18', '12', '212', '5.6e-5'])


if __name__ == '__main__':
    unittest.main()
