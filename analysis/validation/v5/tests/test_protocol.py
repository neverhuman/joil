import copy
import json
import tempfile
import unittest
from pathlib import Path
import json as json_module

from analysis.validation.v5.protocol import (
    AnalysisError, analyze_confirmation, build_private_dossier, export_reviewed_public,
    reduce_measurements, sample_size_from_pilot, store_private_dossier,
    validate_independent_pairs, validate_specimen_passport,
    validate_holdout_partition,
)


def measurement_record():
    channels = {}
    values = {
        "c_in": ("mg/L", 10.0), "c_treated": ("mg/L", 2.0), "c_reject": ("mg/L", 402.0),
        "dsd_in": ("fraction", [0.6, 0.4]), "dsd_treated": ("fraction", [0.7, 0.3]), "dsd_reject": ("fraction", [0.2, 0.8]),
        "q_feed": ("L/s", 10.0), "q_treated": ("L/s", 9.8), "q_reject": ("L/s", 0.2),
        "p_inlet": ("Pa", 300000.0), "p_treated": ("Pa", 250000.0), "p_reject": ("Pa", 260000.0),
        "temperature": ("K", 298.15), "inventory_oil": ("mg", 100.0), "inventory_water": ("L", 5.0),
    }
    for name, (unit, value) in values.items():
        lag = 2.0 if name in {"c_in", "c_treated", "c_reject"} else 0.0
        channels[name] = {"unit": unit, "lag_s": lag, "lag_evidence_sha256": "d" * 64,
                          "calibration_sha256": "e" * 64,
                          "u_systematic": [0.005, 0.005] if isinstance(value, list) else 0.05, "rows": [
            {"time_s": 2.0 if lag else 0.0, "value": value, "u": [0.01, 0.01] if isinstance(value, list) else 0.1, "censored": False},
            {"time_s": 10.0 if lag else 8.0, "value": value, "u": [0.01, 0.01] if isinstance(value, list) else 0.1, "censored": False},
        ]}
    return {"schema_version": 1, "record_id": "synthetic-delay-recovery", "channels": channels,
            "windows": [{"id": "steady", "start_s": 0.0, "end_s": 8.0}],
            "synchronization": "common_clock_with_documented_sensor_and_sample_line_lags",
            "clock_sync_sha256": "f" * 64, "recovery_control_sha256": "0" * 64,
            "dsd_basis": "oil_volume_fraction", "dsd_bin_edges_um": [3.0, 10.0, 100.0]}


class ProtocolTests(unittest.TestCase):
    def test_g009_product_of_linear_signals_uses_independent_integral(self):
        from analysis.validation.v5.protocol import _product_integral
        rows = [(0.0, 1.0, 0.0, False), (1.0, 2.0, 0.0, False)]
        result = _product_integral(rows, rows, 0.0, 1.0, "c", "q", 0.0, 0.0)
        # Integral (1+t)^2 from0to1 =7/3, not endpoint trapezoid2.5.
        self.assertAlmostEqual(result[0], 7.0/3.0, places=12)

    def test_g009_water_closure_includes_recorded_inventory_change(self):
        record=measurement_record()
        record['channels']['inventory_water']['rows'][1]['value'] += 1.0
        row=reduce_measurements(record)['results'][0]
        self.assertAlmostEqual(row['water_closure_fraction'], -1.0/80.0)

    def test_lag_compensation_common_window_mass_and_uncertainty(self):
        row = reduce_measurements(measurement_record())["results"][0]
        self.assertAlmostEqual(row["mass"]["feed"]["window_mass_mg"], 800.0)
        self.assertAlmostEqual(row["mass"]["treated"]["window_mass_mg"], 156.8)
        self.assertGreater(row["mass"]["feed"]["u_window_mass_mg"], 0)
        self.assertGreater(row["mass"]["feed"]["u_systematic_mg"], 0)
        self.assertEqual(len(row["dsd"]["dsd_in"]["fractions"]), 2)
        self.assertEqual(row["oil_mass_balance"]["residual_mg"], 0.0)
        self.assertAlmostEqual(row["inventories"]["inventory_oil"]["change"], 0.0)

    def test_unsynchronized_and_censored_measurements_rejected(self):
        record = measurement_record()
        record["synchronization"] = "unsynchronized"
        with self.assertRaisesRegex(AnalysisError, "unsynchronized_records"):
            reduce_measurements(record)
        record = measurement_record()
        record["channels"]["c_in"]["rows"][1]["censored"] = True
        with self.assertRaisesRegex(AnalysisError, "censored_sample"):
            reduce_measurements(record)

    def test_no_extrapolation_and_bad_dsd_rejected(self):
        record = measurement_record()
        record["windows"][0]["end_s"] = 9.0
        with self.assertRaisesRegex(AnalysisError, "extrapolation_forbidden"):
            reduce_measurements(record)
        record = measurement_record()
        record["channels"]["dsd_in"]["rows"][0]["value"] = [0.2, 0.2]
        with self.assertRaisesRegex(AnalysisError, "fractions_summing_to_one"):
            reduce_measurements(record)

    def test_power_uses_pilot_variability_and_independent_units(self):
        pilots = [{"unit_id": f"u{i}", "build_a": f"pa{i}", "build_b": f"pb{i}",
                   "fluid_batch_id": f"pf{i}", "effect_pct": value} for i, value in enumerate([1, 4, 8])]
        self.assertGreaterEqual(sample_size_from_pilot(pilots)["independent_pairs"], 3)
        pilots[2]["build_b"] = "pb0"
        with self.assertRaisesRegex(AnalysisError, "not_independent_units"):
            sample_size_from_pilot(pilots)
        independent = [{"unit_id": f"u{i}", "build_a": f"a{i}", "build_b": f"b{i}", "fluid_batch_id": f"f{i}"} for i in range(3)]
        self.assertEqual(validate_independent_pairs(independent)["independent_builds"], 6)
        repeated = copy.deepcopy(independent)
        repeated[2]["build_b"] = "b0"
        with self.assertRaisesRegex(AnalysisError, "not_independent_units"):
            validate_independent_pairs(repeated)

    def test_confirmation_lower_bound_not_favorable_mean_decides_ten_percent_claim(self):
        profile = {"decision_profile_sha256": "a" * 64, "requirements_sha256": "b" * 64,
                   "duty_id": "duty", "band_id": "band", "pressure_policy_sha256": "c" * 64,
                   "required_scenarios": ["nominal", "upset"], "claim_threshold_pct": 10.0,
                   "confidence": 0.95, "independent_unit": "independent build/preparation pair"}
        pairs = [{"unit_id": f"u{i}", "build_a_id": f"a{i}", "build_b_id": f"b{i}", "fluid_batch_id": f"f{i}",
                  "decision_profile_sha256": "a" * 64, "requirements_sha256": "b" * 64,
                  "a1_capacity": 100, "b_capacity": 112, "a2_capacity": 100,
                  "a1_treated_conc_mg_l": 20, "a2_treated_conc_mg_l": 20,
                  "a1_pressure_pa": 100000, "a2_pressure_pa": 100000,
                  "duty_id": "duty", "band_id": "band", "pressure_policy_sha256": "c" * 64,
                  "scenarios": [{"scenario_id": s, "required_pass": True} for s in ["nominal", "upset"]]} for i in range(3)]
        profile.update({"drift_concentration_limit_pct": 10, "drift_pressure_limit_pct": 5})
        forged=copy.deepcopy(profile);forged['independent_unit']='parcel'
        with self.subTest(boundary='replication_unit'):
            with self.assertRaisesRegex(AnalysisError,'independent_unit'):
                analyze_confirmation(forged,pairs)
        with self.subTest(boundary='physical_authority'):
            self.assertFalse(analyze_confirmation(profile,pairs)['claim_10pct_qualified'])
        self.assertTrue(analyze_confirmation(profile,pairs)["software_bound_reaches_10pct"])

        pairs[2]["b_capacity"] = 150
        result = analyze_confirmation(profile, pairs)
        self.assertGreater(result["mean_pct"], 10)
        self.assertLess(result["lower_bound_pct"], 10)
        self.assertFalse(result["claim_10pct_qualified"])
        pairs[2]["decision_profile_sha256"] = "d" * 64
        with self.assertRaisesRegex(AnalysisError, "profile_or_requirements_identity_mismatch"):
            analyze_confirmation(profile, pairs)
        pairs[2]["decision_profile_sha256"] = "a" * 64
        pairs[2]["build_b_id"] = pairs[0]["build_b_id"]
        with self.assertRaisesRegex(AnalysisError, "not_independent_units"):
            analyze_confirmation(profile, pairs)
        pairs[2]["build_b_id"] = "b2"
        pairs[0]["scenarios"][1]["required_pass"] = False
        self.assertFalse(analyze_confirmation(profile, pairs)["qualified_candidate"])

    def test_same_duty_band_pressure_and_scenario_family_are_mandatory(self):
        profile = {"decision_profile_sha256": "a" * 64, "requirements_sha256": "b" * 64,
                   "duty_id": "d", "band_id": "b", "pressure_policy_sha256": "c" * 64,
                   "required_scenarios": ["s"], "claim_threshold_pct": 10.0,
                   "confidence": 0.95, "independent_unit": "build/preparation", "drift_concentration_limit_pct": 10,
                   "drift_pressure_limit_pct": 5}
        pairs = [{"unit_id": f"u{i}", "build_a_id": f"a{i}", "build_b_id": f"b{i}", "fluid_batch_id": f"f{i}",
                  "decision_profile_sha256": "a" * 64, "requirements_sha256": "b" * 64,
                  "a1_capacity": 100, "b_capacity": 112, "a2_capacity": 100,
                  "a1_treated_conc_mg_l": 20, "a2_treated_conc_mg_l": 20,
                  "a1_pressure_pa": 100000, "a2_pressure_pa": 100000, "duty_id": "d", "band_id": "b",
                  "pressure_policy_sha256": "c" * 64, "scenarios": [{"scenario_id": "s", "required_pass": True}]} for i in range(3)]
        pairs[1]["pressure_policy_sha256"] = "d" * 64
        with self.assertRaisesRegex(AnalysisError, "pressure_policy_mismatch"):
            analyze_confirmation(profile, pairs)

    def test_holdout_subject_or_fluid_batch_leakage_rejected(self):
        calibration = {"schema_version": 1, "partition_id": "cal", "status": "calibration", "purpose": "controls",
                       "records": [{"record_id": "c1", "subject_sha256": "a" * 64, "fluid_batch_id": "batch-a"}]}
        holdout = {"schema_version": 1, "partition_id": "holdout", "status": "sealed", "purpose": "confirmation",
                   "records": [{"record_id": "h1", "subject_sha256": "b" * 64, "fluid_batch_id": "batch-b"}]}
        self.assertTrue(validate_holdout_partition(calibration, holdout)["independent"])
        holdout["records"][0]["fluid_batch_id"] = "batch-a"
        with self.assertRaisesRegex(AnalysisError, "identity_leakage"):
            validate_holdout_partition(calibration, holdout)

    def test_committed_empty_manifests_validate_as_separate_partitions(self):
        root = Path(__file__).parents[4]
        calibration = json_module.loads((root / "experiments/rig/v5/calibration_manifest.json").read_text())
        holdout = json_module.loads((root / "experiments/holdout/v5/manifest.json").read_text())
        result = validate_holdout_partition(calibration, holdout)
        self.assertEqual(result["calibration_records"], 0)
        self.assertEqual(result["holdout_records"], 0)

    def test_passport_geometry_change_requires_reevaluation(self):
        passport = {"schema_version": 1, "specimen_id": "spec-1", "build_id": "a" * 64,
                    "build_family_id": "family", "fluid_batch_id": "batch", "process": {"machine": "M", "parameters_sha256": "b" * 64},
                    "alloy": "alloy-id", "orientation": "z-up", "post_process": [], "coupon_data": [{"sha256": "c" * 64}],
                    "ct_scan_sha256": "d" * 64, "roughness": {"Ra_um": 4.0, "Rz_um": 8.0},
                    "orifice_measurement": {"diameter_mm": 1.4, "u_mm": 0.02},
                    "proof_test": {"evidence_sha256": "e" * 64}, "tap_results": [{"tap": "inlet", "evidence_sha256": "f" * 64}],
                    "as_built_subject_sha256": "1" * 64, "computational_subject_sha256": "2" * 64, "parent_geometry_sha256": "3" * 64}
        self.assertTrue(validate_specimen_passport(passport)["reevaluation_required"])

    def test_private_dossier_storage_and_reviewed_export_gate(self):
        private = {"schema_version": 1, "lineage": ["parent-hash"], "first_seen_utc": "2026-10-18T00:00:00Z",
                   "subject_sha256": "a" * 64, "variant_id": "variant-1", "evidence": [{"sha256": "b" * 64}],
                   "controls": ["negative-control"], "interventions": [], "human_decisions": [],
                   "classification": "private", "industrial_installation_authorized": False}
        private = build_private_dossier(private)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / ".private" / "dossiers"
            stored = store_private_dossier(private, root, "case.json")
            self.assertEqual(stored.stat().st_mode & 0o777, 0o600)
            self.assertEqual(json.loads(stored.read_text()), private)
        review = {"approved": True, "review_id": "review-1", "reviewed_by_role": "disclosure-reviewer",
                  "reviewed_utc": "2026-10-18T00:00:00Z",
                  "evidence_summary": [{"finding": "synthetic_fixture_only"}],
                  "allowlist": ["schema_version", "subject_sha256", "variant_id", "evidence_summary", "review_id", "reviewed_by_role", "reviewed_utc"]}
        self.assertEqual(set(export_reviewed_public(private, review)), set(review["allowlist"]))
        private["controls"] = ["cust" + "omer_facility_id:secret"]
        with self.assertRaisesRegex(AnalysisError, "identifier_rejected"):
            export_reviewed_public(private, review)
        review["approved"] = False
        with self.assertRaisesRegex(AnalysisError, "explicit_human_review"):
            export_reviewed_public(private, review)


if __name__ == "__main__":
    unittest.main()
