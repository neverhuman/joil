import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('wall_field',ROOT/'cad/program/wall_field.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)


class DesignContractTests(unittest.TestCase):
    def setUp(self):
        self.program=json.loads((ROOT/'experiments/search/proposals-v1.json').read_text())['programmes'][1]

    def test_proposal_cannot_change_baseline_identity(self):
        module.validate(self.program)
        p=copy.deepcopy(self.program);p['baseline_file_sha256']='0'*64
        with self.assertRaises(ValueError):module.validate(p)
        p=copy.deepcopy(self.program);p['inlet_diameter_mm']=9
        with self.assertRaises(ValueError):module.validate(p)

    def test_invalid_gene_never_repairs_to_baseline(self):
        for value in [float('nan'),float('inf'),True]:
            p=copy.deepcopy(self.program);p['modes'][0]['cos_mm']=value
            with self.assertRaises(ValueError):module.validate(p)
        p=copy.deepcopy(self.program);p['modes'][0]['cos_mm']=3
        with self.assertRaises(ValueError):module.validate(p)
