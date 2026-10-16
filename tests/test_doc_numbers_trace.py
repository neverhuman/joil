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
    _check_declared,
    _lead_affinity,
    _numeral_tokens,
    _unit_scales,
    build_numbers_trace,
    check_numbers_trace,
)


def _copy_tree(dst):
    for rel in ('docs/results', 'docs/receipts', 'experiments', 'baselines/ct35',
                'crates/joil-evidence/receipts', 'crates/joil-core/tests'):
        shutil.copytree(ROOT / rel, dst / rel)
    for rel in ('docs/STATUS.json', 'docs/WHITEPAPER.md', 'docs/SCIENCE.md', 'README.md', 'docs/RESULTS.md', 'docs/PR3-DESCRIPTION.md', NUMBERS_TRACE,
                'crates/joil-core/tests/regression.rs', 'crates/joil-core/src/program.rs',
                'tools/run_elite_3grid_gci_and_rsm.py', 'crates/joil-cli/src/main.rs',
                'crates/joil-core/src/structure.rs', 'crates/joil-core/src/evolve.rs',
                'crates/joil-core/src/population.rs', 'crates/joil-core/src/redteam.rs',
                'crates/joil-ir/src/lib.rs', 'requirements/joil-hydrocyclone-v1.toml'):
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

    def test_committed_trace_has_no_declaration_errors(self):
        _, errors = build_numbers_trace(self.base)
        self.assertEqual(errors, [])

    def test_cited_receipt_binds_numerals(self):
        # 237 was the pre-refresh smoke count; it still occurs in the Wave-4 study receipt, so an
        # any-receipt search would accept it. §5.6 cites only the smoke receipt at this point.
        self._with('docs/WHITEPAPER.md', '`228` repaired', '`237` repaired')
        _, errors = build_numbers_trace(self.base)
        self.assertTrue(any("'237'" in e and 'wave4-smoke-v1.json' in e for e in errors), errors)

    def test_scoped_declaration_stays_in_its_sentence(self):
        self._with('docs/WHITEPAPER.md', '- **No silent fallbacks.**', '- **No silent fallbacks.** Gate `0.025`.')
        _, errors = build_numbers_trace(self.base)
        self.assertTrue(any("'0.025'" in e for e in errors), errors)

    def test_unused_scoped_declaration_fails(self):
        self._with('docs/WHITEPAPER.md', 'one-sided $\\alpha = 0.025$', 'one-sided $\\alpha=0.025$')
        _, errors = build_numbers_trace(self.base)
        self.assertTrue(any('0.025@one-sided' in e and 'stale declaration' in e for e in errors), errors)

    def test_results_page_is_traced(self):
        self._with('docs/RESULTS.md', '## Software stack\n', '## Software stack\n\nThe elite separates `98.765` % of oil.\n')
        _, errors = build_numbers_trace(self.base)
        self.assertTrue(any("RESULTS: numeral '98.765'" in e for e in errors), errors)

    def test_statistic_symbol_outranks_column_header(self):
        # The header word 'best' also names unrelated `best_candidate` fields; the symbol before the numeral wins.
        table = ('\n| Arm | Best score vs `B3` |\n| :--- | :--- |\n'
                 '| `B1` | $p_{\\text{Holm}} = 1.0$ ([`docs/results/wave4-evolution-proof-v1.json`]'
                 '(results/wave4-evolution-proof-v1.json)) |\n')
        self._with('docs/RESULTS.md', '## Software stack\n', table + '\n## Software stack\n')
        text, _ = build_numbers_trace(self.base)
        results = text[text.index('\n## RESULTS'):]
        rows = [l for l in results.splitlines() if l.startswith('| `1.0` |')]
        self.assertTrue(rows and all('p_value_holm_bonferroni' in l for l in rows), rows)

    def test_derived_constants_are_checked(self):
        self.assertIsNone(_check_declared('255.8', 'derived', 'ratio', '`5.60293e-5 / 2.19045e-5`', ROOT, []))
        self.assertIn('evaluates to', _check_declared('255.9', 'derived', 'ratio', '`5.60293e-5 / 2.19045e-5`', ROOT, []))
        self.assertIn('unsupported', _check_declared('1', 'derived', 'x', '`__import__`', ROOT, []))

    def test_statistic_symbols_prefer_their_fields(self):
        self.assertEqual(_lead_affinity('Whitney $U = '), ('mann_whitney_u',))
        self.assertEqual(_lead_affinity("Cliff's $\\delta = "), ('cliffs_delta',))
        self.assertEqual(_lead_affinity('$p_{\\text{Holm}} = '), ('holm',))
        self.assertEqual(_lead_affinity('budget of '), ())

    def test_units_select_scale_and_quantity(self):
        self.assertIn((1e-3, '/1000'), _unit_scales(' kPa` vs'))
        self.assertNotIn((1e-3, '/1000'), _unit_scales(' mg/L` ('))
        self.assertIn((100.0, 'x100'), _unit_scales('%`)'))
        self.assertIn('_pa', _affinity(' kPa`'))
        self.assertIn('mg_l', _affinity('\\text{ mg/L}$'))
        self.assertIn('_pa', _affinity('–19.53 kPa`'))  # unit after the far end of a range
        self.assertIn('_um', _affinity('\\ \\mu\\text{m}$, $9.74'))  # LaTeX micrometre

    def test_identifiers_dates_and_lists_are_not_numerals(self):
        tokens = _numeral_tokens('`CAD-09`, S3 P0-09, G2, CT-35, 16 October 2026, grid `[18,12,212]`, 5.6\\times 10^{-5}')
        self.assertEqual(tokens, ['18', '12', '212', '5.6e-5'])
        self.assertEqual(_numeral_tokens('$d_{50,\\text{feed}} = 38.0$, $d_{32}$ (S2 §4.7)'), ['38.0'])
        self.assertEqual(_numeral_tokens('### 4.6 Control in OpenFOAM 14 gives `18.63 kPa`'), ['18.63'])


if __name__ == '__main__':
    unittest.main()
