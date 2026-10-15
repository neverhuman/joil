"""Synthetic adversarial streams test extraction, never CFD performance."""
import math
from pathlib import Path
import sys
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from analysis.carrier_snapshot import scalar_rows,extract


class CarrierExtractionTests(unittest.TestCase):
    def test_incomplete_and_duplicate_rows(self):
        self.assertEqual(scalar_rows(b'# example\n1 2\n2 3'),{1.:2.})
        with self.assertRaises(ValueError):scalar_rows(b'1 2\n1 3\n')
        with self.assertRaises(ValueError):scalar_rows(b'1 nan\n')

    def test_ports_must_share_same_iteration(self):
        q={'inlet1':-.5,'inlet2':-.5,'overflow':.02,'underflow':.98}
        p={'inlet1':200000,'inlet2':200000,'overflow':-100000,'underflow':0}
        streams={n+'_flow':{10:v,11:99} for n,v in q.items()}
        streams.update({n+'_pressure':{10:v/998} for n,v in p.items()})
        r=extract(streams,998)
        self.assertEqual(r['pseudo_iteration'],10)
        self.assertEqual(r['reject_fraction'],.02)
        self.assertAlmostEqual(r['water_closure_fraction'],0)
        self.assertAlmostEqual(r['pdr'],1.5)
        self.assertEqual(r['static_tap_drop_pa'],200000)
        self.assertIsNone(r['physical_elapsed_time_s'])
        streams['overflow_flow']={10:-.01}
        self.assertFalse(extract(streams,998)['flow_orientation_valid'])
        streams['overflow_flow']={12:.02}
        with self.assertRaises(ValueError):extract(streams,998)


if __name__=='__main__':unittest.main()
