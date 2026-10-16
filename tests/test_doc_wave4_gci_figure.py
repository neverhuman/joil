"""IR NEED-DOC-06: the WHITEPAPER §4.6 figure is generated from committed CFD receipts and stays bound to them."""
from pathlib import Path
import copy
import json
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.generate_wave4_gci_figure import OUT, SOURCES, extract_series, series_sha256, verify


class Wave4GciFigureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.receipts = [json.loads(p.read_text(encoding='utf-8')) for p in SOURCES]
        cls.series = extract_series(*cls.receipts)

    def test_committed_figure_matches_manifest_receipts_and_series(self):
        self.assertEqual(verify(), [])

    def test_whitepaper_section_4_6_embeds_the_figure(self):
        text = (ROOT / 'docs/WHITEPAPER.md').read_text(encoding='utf-8')
        section = text.split('### 4.6 ', 1)[1].split('\n## 5. ', 1)[0]
        self.assertIn('figures/' + OUT.name, section)

    def test_series_are_receipt_values(self):
        gci = self.receipts[0]
        self.assertEqual(self.series['control']['dp_u_kpa'][2] * 1000.0,
                         gci['same_pipeline_ct35_control']['grids'][2]['dp_u_pa'])
        self.assertEqual(list(self.series['winners']), ['s20261106-g023-i001'])
        self.assertEqual(self.series['unsolved_winners'], 5)

    def test_changed_receipt_value_changes_series_digest(self):
        mutated = copy.deepcopy(self.receipts)
        mutated[0]['same_pipeline_ct35_control']['grids'][2]['dp_u_pa'] += 1e-6
        self.assertNotEqual(series_sha256(extract_series(*mutated)), series_sha256(self.series))


if __name__ == '__main__':
    unittest.main()
