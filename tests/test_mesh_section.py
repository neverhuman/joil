import importlib.util
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('mesh_audit',ROOT/'physics/openfoam/mesh/audit.py')
audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)


@unittest.skipUnless(importlib.util.find_spec('numpy'), 'install project CAD dependencies to execute the optional mesh-section test')
class MeshSectionTests(unittest.TestCase):
    def test_vertex_touch_does_not_change_closed_section(self):
        import numpy as np
        # Four side quads give an exact square; an isolated point-touching
        # triangle contributes zero length/area and must not corrupt the loop.
        p=np.array([[-1,-1,-1],[1,-1,-1],[1,1,-1],[-1,1,-1],
                    [-1,-1,1],[1,-1,1],[1,1,1],[-1,1,1],
                    [4,4,0],[5,4,1],[4,5,1]],dtype=float)
        faces=[[0,1,5,4],[1,2,6,5],[2,3,7,6],[3,0,4,7],[8,9,10]]
        value=audit.planar_section(p,faces,0)
        self.assertAlmostEqual(value['area_m2'],4)
        self.assertAlmostEqual(value['perimeter_m'],8)
        with self.assertRaises(ValueError):audit.planar_section(p,faces[1:],0)
