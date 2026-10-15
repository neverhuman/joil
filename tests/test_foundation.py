import importlib.util
import json
import math
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[1]
def module(name,path):
    spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
G=module('geometry',ROOT/'cad/ct35/geometry.py')
P=module('public_check',ROOT/'tools/check_public.py')


class FoundationTests(unittest.TestCase):
    def test_analytic_fluid_reference(self):
        _,d=G.load()
        self.assertAlmostEqual(d['analytic_fluid_volume_mm3'],1105802.5227442954,places=6)
        self.assertAlmostEqual(d['ze'],1639.2474318433099,places=9)
        self.assertAlmostEqual(d['zh'],186.74743184330993,places=9)
    def test_inlet_area(self):
        _,d=G.load()
        self.assertAlmostEqual(d['w']*d['h'],120.26409377023427,places=10)
        self.assertAlmostEqual(d['h']/d['w'],2)
    def test_invalid_geometry(self):
        obj,_=G.load();p=obj['parameters'].copy();p['d2_mm']=math.nan
        with self.assertRaises(ValueError):G.derive(p)
    def test_publication_guard(self):
        self.assertTrue(P.violations('.private/raw.md',b'raw'))
        self.assertTrue(P.violations('source.md',b'private customer', [b'private customer']))
        self.assertTrue(P.violations('design.step',b'geometry'))
        self.assertFalse(P.violations('README.md',b'public research'))
    def test_packet_dag_and_scopes(self):
        packets=json.loads((ROOT/'docs/WORK_PACKETS.json').read_text())['packets'];seen=set()
        for p in packets:
            self.assertFalse(set(p['dependencies'])-seen,p['id']);self.assertNotIn(p['id'],seen);seen.add(p['id'])
        self.assertEqual(len(seen),11)
    def test_all_inputs_have_disposition(self):
        text=(ROOT/'docs/SOURCE_REVIEW.md').read_text()
        for n in ['ti1.md','ti2.md','ti3.md','ti4.md','ti5.md','ti6.md','ti7.md','ct35_fluid_domain.stl','ct35_head_cartridge.stl','ct35_baseline_rev1 (1).zip','HC35_Simulation_HPC_Engineering_Spec.html']:
            self.assertIn(n,text)


if __name__=='__main__':unittest.main()
