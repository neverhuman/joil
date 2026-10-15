import json
import math
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from analysis.evidence import manifest, verify, research_fitness, save_new
from analysis.inventory import bounds, concentration_bounds
from analysis.units import pressure_scale


class EvidenceTests(unittest.TestCase):
    def test_changed_evidence_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'p';p.write_bytes(b'pressure')
            m=manifest(d,['p']);verify(d,m)
            p.write_bytes(b'Pressure')
            with self.assertRaises(ValueError):verify(d,m)
            with self.assertRaises(ValueError):manifest(d,['../outside'])

    def test_receipts_cannot_be_overwritten(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'receipt.json';save_new(p,{'a':1})
            with self.assertRaises(FileExistsError):save_new(p,{'a':2})

    def test_pressure_dimensions(self):
        self.assertEqual(pressure_scale('dimensions [0 2 -2 0 0 0 0];',998),998)
        self.assertEqual(pressure_scale('dimensions [1 -1 -2 0 0 0 0];',998),1)
        with self.assertRaises(ValueError):pressure_scale('dimensions [0 1 -1 0 0 0 0];',998)
        with self.assertRaises(ValueError):pressure_scale('p 100;',998)

    def test_tracer_is_not_oil_separation(self):
        self.assertEqual(concentration_bounds(500,[.02,.02],.02),[500,500])

    def test_inventory_wall_and_remaining_are_not_known_capture(self):
        b=bounds(1,.8,.1,.05,.05)
        self.assertAlmostEqual(b['capture_bounds'][0],.8)
        self.assertAlmostEqual(b['capture_bounds'][1],.9)
        with self.assertRaises(ValueError):bounds(1,.9,.2,0,0)

    def test_threshold_and_unknown_fitness(self):
        kwargs=dict(flow_m3_h=4,reject=.02,hydraulic_loss_j_m3=10000,
                    eligibility_reasons=['no validation'])
        unknown=research_fitness(cout_bounds=None,**kwargs)
        self.assertIsNone(unknown['minimize_screen_vector'])
        self.assertIsNone(unknown['qualified_multiobjective_vector'])
        self.assertEqual(unknown['sampled_quality'],'unknown')
        fail=research_fitness(cout_bounds=concentration_bounds(500,[.94,.94],.02),**kwargs)
        self.assertEqual(fail['sampled_quality'],'fail_at_sampled_point')
        self.assertAlmostEqual(fail['minimize_screen_vector'][0],30.6122448979592)
        self.assertIsNone(fail['qualified_capacity_gain'])
        with self.assertRaises(ValueError):research_fitness(cout_bounds=[math.nan,1],**kwargs)


if __name__=='__main__':unittest.main()
