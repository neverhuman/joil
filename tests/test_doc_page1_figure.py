"""PAGE1-WHY-MATTERS: the WHITEPAPER lead figure is generated from the Wave-4 receipt and stays bound to it."""
from pathlib import Path
import copy
import json
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.generate_page1_figure import B3, OUT, RECEIPT, extract_series, series_sha256, verify


class Page1FigureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.w4 = json.loads(RECEIPT.read_text(encoding='utf-8'))
        cls.series = extract_series(cls.w4)

    def test_committed_figure_matches_manifest_receipt_and_series(self):
        self.assertEqual(verify(), [])

    def test_whitepaper_shows_figure_first_under_read_this_first(self):
        text = (ROOT / 'docs/WHITEPAPER.md').read_text(encoding='utf-8')
        section = text.split('## Read this first', 1)[1].split('\n## Abstract', 1)[0]
        first_image = section.find('![')
        self.assertGreaterEqual(first_image, 0)
        self.assertIn('figures/' + OUT.name, section[first_image:first_image + 400])
        self.assertLess(first_image, section.find('Six external reviews'))

    def test_series_agree_with_final_statistics_and_tests(self):
        final = self.w4['final_summary_statistics']
        self.assertEqual(self.series['B3']['best_mean'][-1], final[B3]['best_kpi_score']['mean'])
        self.assertEqual(self.series['B1']['best_mean'][-1], final['B1_Random_Search']['best_kpi_score']['mean'])
        self.assertEqual(len(self.series['B3']['seed_best']), self.series['n_seeds'])
        self.assertGreater(self.series['best_test']['p_value_holm_bonferroni'], 0.025)  # random search not beaten
        self.assertLess(self.series['median_test']['p_value_holm_bonferroni'], 0.025)

    def test_changed_receipt_value_changes_series_digest(self):
        mutated = copy.deepcopy(self.w4)
        mutated['generation_band_curves'][B3][-1]['best_kpi_score']['mean'] += 1e-6
        self.assertNotEqual(series_sha256(extract_series(mutated)), series_sha256(self.series))


if __name__ == '__main__':
    unittest.main()
