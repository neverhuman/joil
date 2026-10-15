"""Regression tests for hardened KPI provenance, Exotic CAD canonicalization, and Lagrangian parsers."""
from __future__ import annotations

import copy
import json
from pathlib import Path
import shutil
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "analysis"))
sys.path.insert(0, str(ROOT / "cad/program"))
sys.path.insert(0, str(ROOT / "physics/openfoam/oil"))

import kpi_fitness
import exotic_graph
import oil_grade


class TestHardenedPipeline(unittest.TestCase):
    def test_kpi_fitness_rejects_escaped_paths_and_failure_markers(self) -> None:
        tmp_case = ROOT / "runs" / "test-kpi-prov-tmp"
        if tmp_case.exists():
            shutil.rmtree(tmp_case)
        tmp_case.mkdir(parents=True, exist_ok=True)
        try:
            (tmp_case / "input-manifest.json").write_text(
                json.dumps({"../outside.txt": "0" * 64, "/etc/passwd": "0" * 64})
            )
            (tmp_case / "numerical-failure.json").write_text("{}")
            prov = kpi_fitness.verify_carrier_case_provenance(tmp_case, 600)
            self.assertFalse(prov["input_manifest_verified"])
            self.assertTrue(prov["failed_marker_present"])
            self.assertEqual(len(prov["mismatched_inputs"]), 2)
        finally:
            if tmp_case.exists():
                shutil.rmtree(tmp_case)

    def test_exotic_graph_rejects_unknown_keys_and_canonicalizes_inactive_genes(self) -> None:
        base = exotic_graph.default_exotic_program()
        bad = copy.deepcopy(base)
        bad["unknown_top_key"] = 123
        with self.assertRaises(ValueError):
            exotic_graph.normalize_program(bad)

        inactive = copy.deepcopy(base)
        inactive["inlet_split"]["enabled"] = False
        inactive["inlet_split"]["vane_thickness_mm"] = 1.75
        inactive["inlet_split"]["branches_per_port"] = 3
        inactive["skimmer"]["length_mm"] = 0.0
        inactive["skimmer"]["outer_radius_mm"] = 3.6
        inactive["chamber_modes"] = [
            {"azimuthal_order": 3, "cos_mm": 0.0, "sin_mm": 0.0, "twist_rad": -2.5}
        ]
        norm = exotic_graph.normalize_program(inactive)
        self.assertTrue(exotic_graph.is_exact_baseline(norm))
        self.assertEqual(norm["inlet_split"]["vane_thickness_mm"], 1.2)
        self.assertEqual(norm["inlet_split"]["branches_per_port"], 1)
        self.assertEqual(norm["skimmer"]["outer_radius_mm"], 2.4)
        self.assertEqual(norm["chamber_modes"], [])

    def test_oil_grade_strict_parsers(self) -> None:
        tmp = ROOT / "runs" / "test-parser-tmp.txt"
        tmp.parent.mkdir(parents=True, exist_ok=True)
        try:
            tmp.write_text("FoamFile { object origId; }\n2 ( 10 20 ) extra_garbage\n")
            with self.assertRaises(ValueError):
                oil_grade.parse_labels(tmp)

            tmp.write_text(
                "FoamFile { object U; }\ninternalField nonuniform List<vector> 1 ( (1 2 3) trailing );\n"
            )
            with self.assertRaises(ValueError):
                oil_grade.parse_vectors(tmp)

            tmp.write_text("# Time cloudA\n600.002 100.0\n600.001 100.0\n")
            with self.assertRaises(ValueError):
                oil_grade.parse_surface_field_value_sums(tmp, ["cloudA"], 5e-5)
        finally:
            if tmp.exists():
                tmp.unlink()


if __name__ == "__main__":
    unittest.main()
