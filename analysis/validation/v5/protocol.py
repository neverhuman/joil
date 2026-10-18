"""Strict fixture-grade physical confirmation analysis.

Input channels use SI units except concentration (mg/L) and DSD-bin fractions.
Each row's timestamp is the instrument timestamp; corrected process time is
``time_s - lag_s``. Uncertainty fields are one-standard-deviation values.
"""

from __future__ import annotations

from copy import deepcopy
from datetime import datetime
import hashlib
import json
import math
import os
from pathlib import Path
import re
from statistics import mean, stdev
from typing import Any


class AnalysisError(ValueError):
    """A record cannot support the requested reduction or claim."""


CHANNELS = {
    "c_in": "mg/L", "c_treated": "mg/L", "c_reject": "mg/L",
    "dsd_in": "fraction", "dsd_treated": "fraction", "dsd_reject": "fraction",
    "q_feed": "L/s", "q_treated": "L/s", "q_reject": "L/s",
    "p_inlet": "Pa", "p_treated": "Pa", "p_reject": "Pa",
    "temperature": "K", "inventory_oil": "mg", "inventory_water": "L",
}


def required_channels() -> dict[str, str]:
    return dict(CHANNELS)


def _finite(x: Any, label: str) -> float:
    if isinstance(x, bool) or not isinstance(x, (int, float)) or not math.isfinite(x):
        raise AnalysisError(f"{label}: finite_number_required")
    return float(x)


def _digest(value: Any) -> str:
    raw = json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()
    return hashlib.sha256(raw).hexdigest()


def _is_sha256(value: Any) -> bool:
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def _corrected(channel: dict[str, Any], name: str) -> list[tuple[float, Any, Any, bool]]:
    if set(channel) != {"unit", "lag_s", "lag_evidence_sha256", "calibration_sha256", "u_systematic", "rows"}:
        raise AnalysisError(f"{name}: unknown_or_missing_channel_fields")
    if not _is_sha256(channel["lag_evidence_sha256"]) or not _is_sha256(channel["calibration_sha256"]):
        raise AnalysisError(f"{name}: lag_and_calibration_evidence_hashes_required")
    if channel["unit"] != CHANNELS[name]:
        raise AnalysisError(f"{name}: unit_mismatch")
    lag = _finite(channel["lag_s"], f"{name}.lag_s")
    if lag < 0:
        raise AnalysisError(f"{name}: negative_lag")
    systematic = channel["u_systematic"]
    if name.startswith("dsd_"):
        if not isinstance(systematic, list) or not systematic:
            raise AnalysisError(f"{name}: systematic_uncertainty_vector_required")
        systematic = [_finite(x, f"{name}.u_systematic") for x in systematic]
        if any(x < 0 for x in systematic):
            raise AnalysisError(f"{name}: negative_systematic_uncertainty")
    else:
        systematic = _finite(systematic, f"{name}.u_systematic")
        if systematic < 0:
            raise AnalysisError(f"{name}: negative_systematic_uncertainty")
    rows = channel["rows"]
    if not isinstance(rows, list) or len(rows) < 2:
        raise AnalysisError(f"{name}: at_least_two_samples_required")
    result = []
    last = -math.inf
    for row in rows:
        if set(row) != {"time_s", "value", "u", "censored"}:
            raise AnalysisError(f"{name}: malformed_sample")
        t = _finite(row["time_s"], f"{name}.time_s") - lag
        if name.startswith("dsd_"):
            if not isinstance(row["value"], list) or not isinstance(row["u"], list) or len(row["value"]) != len(row["u"]) or not row["value"]:
                raise AnalysisError(f"{name}: dsd_value_and_uncertainty_vectors_required")
            v = [_finite(x, f"{name}.value") for x in row["value"]]
            u = [_finite(x, f"{name}.u") for x in row["u"]]
        else:
            v = _finite(row["value"], f"{name}.value")
            u = _finite(row["u"], f"{name}.u")
        if t <= last or (any(x < 0 for x in u) if isinstance(u, list) else u < 0):
            raise AnalysisError(f"{name}: unordered_time_or_negative_uncertainty")
        censored = row["censored"]
        if not isinstance(censored, bool):
            raise AnalysisError(f"{name}: censor_flag_must_be_boolean")
        if censored:
            # A censored concentration is a bound, not a point observation.
            if name.startswith("c_") and v < 0:
                raise AnalysisError(f"{name}: invalid_censor_bound")
        result.append((t, v, u, censored))
        last = t
    return result


def _interpolate(rows: list[tuple[float, Any, Any, bool]], t: float, name: str) -> tuple[Any, Any, bool]:
    if t < rows[0][0] or t > rows[-1][0]:
        raise AnalysisError(f"{name}: extrapolation_forbidden")
    for left, right in zip(rows, rows[1:]):
        if left[0] <= t <= right[0]:
            if left[3] or right[3]:
                raise AnalysisError(f"{name}: censored_sample_intersects_window")
            a = (t - left[0]) / (right[0] - left[0])
            if isinstance(left[1], list):
                if len(left[1]) != len(right[1]):
                    raise AnalysisError(f"{name}: changing_dsd_bin_count")
                value = [(1 - a) * x + a * y for x, y in zip(left[1], right[1])]
                uncertainty = [math.hypot((1 - a) * x, a * y) for x, y in zip(left[2], right[2])]
            else:
                value = (1 - a) * left[1] + a * right[1]
                uncertainty = math.hypot((1 - a) * left[2], a * right[2])
            return value, uncertainty, False
    raise AnalysisError(f"{name}: interpolation_gap")


def _integrate(rows: list[tuple[float, float, float, bool]], start: float, end: float, name: str) -> tuple[float, float]:
    points = [start] + [r[0] for r in rows if start < r[0] < end] + [end]
    vals = [_interpolate(rows, t, name) for t in points]
    weights = [0.0] * len(points)
    for i, (a, b) in enumerate(zip(points, points[1:])):
        dt = b - a
        weights[i] += dt / 2
        weights[i + 1] += dt / 2
    if isinstance(vals[0][0], list):
        integral = [sum(w * item[0][j] for w, item in zip(weights, vals)) for j in range(len(vals[0][0]))]
        u = [math.sqrt(sum((w * item[1][j]) ** 2 for w, item in zip(weights, vals))) for j in range(len(vals[0][0]))]
    else:
        integral = sum(w * item[0] for w, item in zip(weights, vals))
        u = math.sqrt(sum((w * item[1]) ** 2 for w, item in zip(weights, vals)))
    return integral, u


def _product_integral(a_rows: list, b_rows: list, start: float, end: float, a_name: str, b_name: str,
                      a_systematic: float, b_systematic: float) -> tuple[float, float, float, float]:
    """Exactly integrate piecewise-linear C(t)Q(t); propagate original-sample gradients.

    Original observations are independent within and across channels in this fixture
    model. Shared interpolation endpoints retain their covariance through gradients.
    Cross-sensor covariance still needs an independently admitted measurement model.
    """
    points = sorted({start, end, *(r[0] for r in a_rows if start < r[0] < end),
                     *(r[0] for r in b_rows if start < r[0] < end)})
    def at(rows, t, name):
        value, _, _ = _interpolate(rows, t, name)
        for i, (left, right) in enumerate(zip(rows, rows[1:])):
            if left[0] <= t <= right[0]:
                fraction = (t-left[0])/(right[0]-left[0])
                return value, ((i, 1-fraction), (i+1, fraction))
        raise AnalysisError(f"{name}: interpolation_gap")
    ga, gb = [0.0]*len(a_rows), [0.0]*len(b_rows)
    integral = 0.0
    for left, right in zip(points, points[1:]):
        a0, wa0 = at(a_rows,left,a_name); a1, wa1 = at(a_rows,right,a_name)
        b0, wb0 = at(b_rows,left,b_name); b1, wb1 = at(b_rows,right,b_name)
        scale = (right-left)/6.0
        integral += scale*(2*a0*b0+a0*b1+a1*b0+2*a1*b1)
        for weights, gradient, coefficient in (
            (wa0,ga,scale*(2*b0+b1)), (wa1,ga,scale*(b0+2*b1)),
            (wb0,gb,scale*(2*a0+a1)), (wb1,gb,scale*(a0+2*a1))):
            for index, weight in weights:
                gradient[index] += coefficient*weight
    u_random = math.hypot(*(g*row[2] for g,row in zip(ga,a_rows)),
                          *(g*row[2] for g,row in zip(gb,b_rows)))
    u_systematic = math.hypot(sum(ga)*a_systematic, sum(gb)*b_systematic)
    result = (integral, u_random, u_systematic, math.hypot(u_random,u_systematic))
    if not all(math.isfinite(value) for value in result):
        raise AnalysisError("product_integral: unrepresentable_result")
    return result


def reduce_measurements(record: dict[str, Any]) -> dict[str, Any]:
    """Lag-correct channels, reduce common windows and propagate uncertainty.

    Every channel is sampled at process time after explicit lag compensation.
    A window is rejected if any channel needs extrapolation or crosses a
    censored observation. Concentration and flow mass rates are paired on the
    common time grid; their independent uncertainties are propagated by first
    order products. Cross-sensor covariance is not assumed away implicitly.
    """
    if set(record) != {"schema_version", "record_id", "channels", "windows", "synchronization", "clock_sync_sha256",
                       "recovery_control_sha256", "dsd_basis", "dsd_bin_edges_um"}:
        raise AnalysisError("record: unknown_or_missing_top_level_fields")
    if record["schema_version"] != 1 or not record["record_id"]:
        raise AnalysisError("record: schema_or_id")
    if (record["synchronization"] != "common_clock_with_documented_sensor_and_sample_line_lags"
            or not _is_sha256(record["clock_sync_sha256"]) or not _is_sha256(record["recovery_control_sha256"])):
        raise AnalysisError("unsynchronized_records_cannot_establish_treatment_capacity")
    edges = [_finite(x, "dsd_bin_edges_um") for x in record["dsd_bin_edges_um"]]
    if record["dsd_basis"] != "oil_volume_fraction" or len(edges) < 2 or any(b <= a for a, b in zip(edges, edges[1:])):
        raise AnalysisError("dsd_requires_volume_basis_and_strict_bin_edges")
    if set(record["channels"]) != set(CHANNELS):
        raise AnalysisError("record: required_channel_inventory_mismatch")
    if not record["windows"] or len({w.get("id") for w in record["windows"]}) != len(record["windows"]):
        raise AnalysisError("record: windows_must_be_nonempty_and_uniquely_identified")
    ordered_windows = sorted(record["windows"], key=lambda w: _finite(w["start_s"], "start_s"))
    if any(_finite(left["end_s"], "end_s") > _finite(right["start_s"], "start_s")
           for left, right in zip(ordered_windows, ordered_windows[1:])):
        raise AnalysisError("record: overlapping_windows_cannot_be_counted_twice")
    channels = {k: _corrected(record["channels"][k], k) for k in CHANNELS}
    systematics = {k: record["channels"][k]["u_systematic"] for k in CHANNELS}
    for name in ("dsd_in", "dsd_treated", "dsd_reject"):
        if len(systematics[name]) != len(edges) - 1 or any(len(row[1]) != len(edges) - 1 for row in channels[name]):
            raise AnalysisError(f"{name}: bin_edges_do_not_match_dsd_vectors")
    results = []
    for window in record["windows"]:
        if set(window) != {"id", "start_s", "end_s"}:
            raise AnalysisError("window: malformed")
        start, end = _finite(window["start_s"], "start_s"), _finite(window["end_s"], "end_s")
        if not start < end:
            raise AnalysisError("window: nonpositive_duration")
        means: dict[str, tuple[float, float]] = {}
        integrals: dict[str, tuple[float, float]] = {}
        for name, rows in channels.items():
            integral, u_integral = _integrate(rows, start, end, name)
            integrals[name] = (integral, u_integral)
            # Time-average all continuous channels, retaining units per second.
            duration = end - start
            if isinstance(integral, list):
                means[name] = ([x / duration for x in integral], [x / duration for x in u_integral])
            else:
                means[name] = (integral / duration, u_integral / duration)
        mass = {}
        for label, concentration, flow in (
            ("feed", "c_in", "q_feed"), ("treated", "c_treated", "q_treated"),
            ("reject", "c_reject", "q_reject"),
        ):
            integrated_mass, u_mass_random, u_mass_systematic, u_mass = _product_integral(
                channels[concentration], channels[flow], start, end, concentration, flow,
                systematics[concentration], systematics[flow])
            duration = end - start
            rate = integrated_mass / duration
            u_rate = u_mass / duration
            mass[label] = {"rate_mg_s": rate, "u_rate_mg_s": u_rate,
                           "window_mass_mg": integrated_mass,
                           "u_window_mass_mg": u_mass, "u_random_mg": u_mass_random,
                           "u_systematic_mg": u_mass_systematic}
        dsd = {}
        for name in ("dsd_in", "dsd_treated", "dsd_reject"):
            v, u = means[name]
            if any(not -1e-12 <= x <= 1 + 1e-12 for x in v) or abs(sum(v) - 1.0) > 1e-6:
                raise AnalysisError(f"{name}: bins_must_be_fractions_summing_to_one")
            dsd[name] = {"fractions": v, "u_random": u, "u_systematic": systematics[name]}
        inventories = {}
        for name in ("inventory_oil", "inventory_water"):
            first, u_first, _ = _interpolate(channels[name], start, name)
            last, u_last, _ = _interpolate(channels[name], end, name)
            inventories[name] = {"start": first, "end": last, "change": last - first,
                                 "u_change": math.hypot(u_first, u_last)}
        oil_residual = mass["feed"]["window_mass_mg"] - mass["treated"]["window_mass_mg"] - mass["reject"]["window_mass_mg"] - inventories["inventory_oil"]["change"]
        oil_residual_u = math.sqrt(sum(mass[k]["u_window_mass_mg"] ** 2 for k in ("feed", "treated", "reject")) + inventories["inventory_oil"]["u_change"] ** 2)
        q_feed = integrals["q_feed"][0]
        q_closure = q_feed - integrals["q_treated"][0] - integrals["q_reject"][0] - inventories["inventory_water"]["change"]
        duration = end - start
        q_closure_u = math.sqrt(sum(integrals[k][1] ** 2 + (systematics[k] * duration) ** 2
                                    for k in ("q_feed", "q_treated", "q_reject")) + inventories["inventory_water"]["u_change"] ** 2)
        results.append({
            "window_id": window["id"], "start_s": start, "end_s": end,
            "duration_s": end - start,
            "channels": {k: {"mean": v, "u_random": u, "u_systematic": systematics[k], "unit": CHANNELS[k]}
                         for k, (v, u) in means.items()},
            "mass": mass, "dsd": dsd, "inventories": inventories,
            "water_closure_fraction": q_closure / q_feed if q_feed > 0 else None,
            "u_water_closure_volume_L": q_closure_u,
            "oil_mass_balance": {"residual_mg": oil_residual, "u_residual_mg": oil_residual_u},
        })
    return {"schema_version": 1, "record_id": record["record_id"],
            "input_sha256": _digest(record), "reduction": "lag_corrected_linear_product_common_window_v2",
            "clock_sync_sha256": record["clock_sync_sha256"],
            "recovery_control_sha256": record["recovery_control_sha256"],
            "dsd_basis": record["dsd_basis"], "dsd_bin_edges_um": edges, "results": results,
            "limitations": ["cross-sensor covariance not modeled", "censored-window values abstain", "software fixture reduction is not physical validation"]}


def sample_size_from_pilot(pilot_pairs: list[dict[str, Any]], *, useful_effect: float = 10.0,
                           alpha: float = 0.05, power: float = 0.8) -> dict[str, Any]:
    """Conservative normal planning approximation for paired percentage effects."""
    required = {"unit_id", "build_a", "build_b", "fluid_batch_id", "effect_pct"}
    if len(pilot_pairs) < 3 or any(not isinstance(p, dict) or set(p) != required for p in pilot_pairs):
        raise AnalysisError("pilot_requires_three_strict_independent_pair_records")
    validate_independent_pairs([{"unit_id": p["unit_id"], "build_a": p["build_a"],
                                 "build_b": p["build_b"], "fluid_batch_id": p["fluid_batch_id"]}
                                for p in pilot_pairs])
    effects = [_finite(p["effect_pct"], "pilot_effect_pct") for p in pilot_pairs]
    useful_effect = _finite(useful_effect, "useful_effect")
    alpha = _finite(alpha, "alpha")
    power = _finite(power, "power")
    if useful_effect <= 0 or not 0 < alpha < 1 or not 0 < power < 1:
        raise AnalysisError("invalid_power_inputs")
    sd = stdev(effects)
    # Conventional fixed quantiles avoid inversion in the common case;
    # the planning result is rounded upward and never substitutes for review.
    z_alpha = 1.959963984540054 if alpha == 0.05 else _normal_quantile(1 - alpha / 2)
    z_power = 0.8416212335729143 if power == 0.8 else _normal_quantile(power)
    n = max(3, math.ceil(((z_alpha + z_power) * sd / useful_effect) ** 2))
    return {"independent_pairs": n, "pilot_sd_percentage_points": sd,
            "useful_effect_percentage_points": useful_effect, "alpha": alpha, "power": power,
            "method": "paired_normal_approximation_ceiling_no_small_sample_correction"}


def _normal_quantile(p: float) -> float:
    # Bisection on the standard-normal CDF; adequate for transparent planning.
    lo, hi = -10.0, 10.0
    for _ in range(100):
        mid = (lo + hi) / 2
        cdf = 0.5 * (1 + math.erf(mid / math.sqrt(2)))
        if cdf < p: lo = mid
        else: hi = mid
    return (lo + hi) / 2


def _beta_cf(a: float, b: float, x: float) -> float:
    # Continued fraction from the standard incomplete-beta evaluation.
    qab, qap, qam = a + b, a + 1, a - 1
    c, d = 1.0, 1.0 - qab * x / qap
    d = 1e-300 if abs(d) < 1e-300 else 1 / d
    h = d
    for m in range(1, 201):
        m2 = 2 * m
        aa = m * (b - m) * x / ((qam + m2) * (a + m2))
        d = 1 + aa * d; d = 1e-300 if abs(d) < 1e-300 else d
        c = 1 + aa / c; c = 1e-300 if abs(c) < 1e-300 else c
        d = 1 / d; h *= d * c
        aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2))
        d = 1 + aa * d; d = 1e-300 if abs(d) < 1e-300 else d
        c = 1 + aa / c; c = 1e-300 if abs(c) < 1e-300 else c
        d = 1 / d; delta = d * c; h *= delta
        if abs(delta - 1) < 3e-14: break
    return h


def _ibeta(x: float, a: float, b: float) -> float:
    if x <= 0: return 0.0
    if x >= 1: return 1.0
    bt = math.exp(math.lgamma(a + b) - math.lgamma(a) - math.lgamma(b) + a * math.log(x) + b * math.log1p(-x))
    if x < (a + 1) / (a + b + 2):
        return bt * _beta_cf(a, b, x) / a
    return 1 - bt * _beta_cf(b, a, 1 - x) / b


def _t_critical(confidence: float, df: int) -> float:
    target = confidence
    lo, hi = 0.0, 100.0
    for _ in range(100):
        t = (lo + hi) / 2
        x = df / (df + t * t)
        cdf = 1 - 0.5 * _ibeta(x, df / 2, 0.5)
        if cdf < (1 + target) / 2: lo = t
        else: hi = t
    return (lo + hi) / 2


def analyze_confirmation(protocol: dict[str, Any], pairs: list[dict[str, Any]]) -> dict[str, Any]:
    """Analyze paired independent units; blocked inputs return no claim."""
    required = {"decision_profile_sha256", "requirements_sha256", "duty_id", "band_id",
                "pressure_policy_sha256", "required_scenarios", "claim_threshold_pct",
                "confidence", "independent_unit", "drift_concentration_limit_pct", "drift_pressure_limit_pct"}
    if set(protocol) != required:
        raise AnalysisError("confirmation_profile: unknown_or_missing_fields")
    confidence = _finite(protocol["confidence"], "confidence")
    drift_capacity = _finite(protocol["drift_concentration_limit_pct"], "drift_concentration_limit_pct")
    drift_pressure = _finite(protocol["drift_pressure_limit_pct"], "drift_pressure_limit_pct")
    if (not _is_sha256(protocol["decision_profile_sha256"]) or not _is_sha256(protocol["requirements_sha256"])
            or not _is_sha256(protocol["pressure_policy_sha256"])
            or not isinstance(protocol["required_scenarios"], list) or not protocol["required_scenarios"]
            or any(not isinstance(x, str) or not x for x in protocol["required_scenarios"])
            or len(set(protocol["required_scenarios"])) != len(protocol["required_scenarios"])
            or not 0 < confidence < 1 or drift_capacity < 0 or drift_pressure < 0):
        raise AnalysisError("confirmation_profile: invalid_identity_scenario_or_confidence")
    if protocol["independent_unit"] not in {"independent build/preparation pair", "build/preparation"}:
        raise AnalysisError("confirmation_profile: unsupported_independent_unit")
    if len(pairs) < 3:
        raise AnalysisError("confirmation_requires_at_least_three_independent_pairs")
    unit_ids = [p.get("unit_id") for p in pairs]
    if len(set(unit_ids)) != len(unit_ids) or any(not x for x in unit_ids):
        raise AnalysisError("repeated_or_missing_independent_unit")
    effects = []
    failed = []
    builds, batches = [], []
    for pair in pairs:
        expected = {"unit_id", "build_a_id", "build_b_id", "fluid_batch_id", "decision_profile_sha256",
                    "requirements_sha256", "a1_capacity", "b_capacity", "a2_capacity", "a1_treated_conc_mg_l",
                    "a2_treated_conc_mg_l", "a1_pressure_pa", "a2_pressure_pa", "duty_id", "band_id",
                    "pressure_policy_sha256", "scenarios"}
        if set(pair) != expected:
            raise AnalysisError("pair: unknown_or_missing_fields")
        if any(not isinstance(pair[s], str) or not pair[s] for s in ("unit_id", "build_a_id", "build_b_id", "fluid_batch_id")):
            raise AnalysisError("pair: missing_independent_identity")
        if any(not isinstance(s, dict) or set(s) != {"scenario_id", "required_pass"}
               or not isinstance(s["required_pass"], bool) for s in pair["scenarios"]):
            raise AnalysisError("scenario_result: strict_schema_failure")
        builds.extend([pair["build_a_id"], pair["build_b_id"]])
        batches.append(pair["fluid_batch_id"])
        if pair["decision_profile_sha256"] != protocol["decision_profile_sha256"] or pair["requirements_sha256"] != protocol["requirements_sha256"]:
            raise AnalysisError("decision_profile_or_requirements_identity_mismatch")
        if pair["duty_id"] != protocol["duty_id"] or pair["band_id"] != protocol["band_id"] or pair["pressure_policy_sha256"] != protocol["pressure_policy_sha256"]:
            raise AnalysisError("paired_duty_band_or_pressure_policy_mismatch")
        scenario_ids = {s["scenario_id"] for s in pair["scenarios"]}
        if scenario_ids != set(protocol["required_scenarios"]):
            raise AnalysisError("required_scenario_missing_or_extra")
        for s in pair["scenarios"]:
            if not s["required_pass"]:
                failed.append((pair["unit_id"], s["scenario_id"]))
        a1, b, a2 = (_finite(pair[k], k) for k in ("a1_capacity", "b_capacity", "a2_capacity"))
        c1, c2 = (_finite(pair[k], k) for k in ("a1_treated_conc_mg_l", "a2_treated_conc_mg_l"))
        p1, p2 = (_finite(pair[k], k) for k in ("a1_pressure_pa", "a2_pressure_pa"))
        if min(a1, b, a2) <= 0 or min(c1, c2) < 0 or min(p1, p2) <= 0:
            raise AnalysisError("baseline_capacity_must_be_positive")
        concentration_drift = abs(c2 - c1) / c1 * 100 if c1 > 0 else (0.0 if c2 == 0 else math.inf)
        pressure_drift = abs(p2 - p1) / p1 * 100
        if concentration_drift > protocol["drift_concentration_limit_pct"] or pressure_drift > protocol["drift_pressure_limit_pct"]:
            failed.append((pair["unit_id"], "ab_a_drift_gate"))
        a = (a1 + a2) / 2
        effects.append(100 * (b - a) / a)
    if len(set(builds)) != len(builds) or len(set(batches)) != len(batches):
        raise AnalysisError("nominal_repeats_of_build_or_fluid_batch_are_not_independent_units")
    avg = mean(effects)
    half = _t_critical(confidence, len(effects) - 1) * stdev(effects) / math.sqrt(len(effects))
    lower, upper = avg - half, avg + half
    threshold = _finite(protocol["claim_threshold_pct"], "claim_threshold_pct")
    eligible = not failed and lower >= threshold
    return {"schema_version": 1, "decision_profile_sha256": protocol["decision_profile_sha256"],
            "estimand": "paired_relative_treated_capacity_change_pct", "independent_unit": protocol["independent_unit"],
            "n_independent_pairs": len(effects), "pair_effects_pct": effects,
            "mean_pct": avg, "confidence": confidence, "lower_bound_pct": lower,
            "upper_bound_pct": upper, "claim_threshold_pct": threshold,
            "supported_finding": "no_claim_failed_required_scenario" if failed else ("improvement_at_or_above_threshold" if eligible else ("positive_below_threshold" if lower > 0 else ("estimate_positive_but_improvement_uncertain" if avg > 0 else "no_positive_improvement_supported"))),
            "software_bound_reaches_10pct": eligible if threshold >= 10 else False,
            "claim_10pct_qualified": False,
            "failed_required_scenarios": failed, "qualified_candidate": False,
            "qualification_blockers": ["this software result is not canonical DecisionProfile admission", "as-built/material/model/service evidence must be independently admitted"]}


def validate_specimen_passport(passport: dict[str, Any]) -> dict[str, Any]:
    fields = {"schema_version", "specimen_id", "build_id", "build_family_id", "fluid_batch_id", "process", "alloy",
              "orientation", "post_process", "coupon_data", "ct_scan_sha256", "roughness", "orifice_measurement",
              "proof_test", "tap_results", "as_built_subject_sha256", "computational_subject_sha256", "parent_geometry_sha256"}
    if set(passport) != fields or passport["schema_version"] != 1:
        raise AnalysisError("passport: strict_schema_failure")
    digest_fields = ("build_id", "ct_scan_sha256", "as_built_subject_sha256", "computational_subject_sha256", "parent_geometry_sha256")
    if any(not _is_sha256(passport[field]) for field in digest_fields):
        raise AnalysisError("passport: exact_build_and_subject_hashes_required")
    if (not passport["specimen_id"] or not passport["build_family_id"] or not passport["fluid_batch_id"]
            or not passport["process"] or not passport["alloy"] or not passport["coupon_data"]
            or not passport["proof_test"] or not passport["tap_results"]):
        raise AnalysisError("passport: process_coupon_ct_proof_and_taps_required")
    if (not isinstance(passport["roughness"], dict) or set(passport["roughness"]) != {"Ra_um", "Rz_um"}
            or not isinstance(passport["orifice_measurement"], dict)
            or set(passport["orifice_measurement"]) != {"diameter_mm", "u_mm"}):
        raise AnalysisError("passport: roughness_and_orifice_units_required")
    for key, value in passport["roughness"].items():
        if _finite(value, f"roughness.{key}") < 0:
            raise AnalysisError("passport: roughness_value_invalid")
    if _finite(passport["orifice_measurement"]["diameter_mm"], "orifice_diameter_mm") <= 0 or _finite(passport["orifice_measurement"]["u_mm"], "orifice_u_mm") < 0:
        raise AnalysisError("passport: orifice_measurement_invalid")
    if passport["as_built_subject_sha256"] != passport["computational_subject_sha256"]:
        return {"valid": True, "reevaluation_required": True, "reason": "as_built_geometry_differs_from_computational_subject"}
    return {"valid": True, "reevaluation_required": False, "reason": "subject_hashes_match"}


def validate_independent_pairs(pairs: list[dict[str, Any]]) -> dict[str, Any]:
    """Reject repeated builds/preparations masquerading as independent units."""
    if len(pairs) < 3:
        raise AnalysisError("power_or_confirmation_requires_three_independent_pairs_minimum")
    build_ids, batch_ids, unit_ids = [], [], []
    for pair in pairs:
        if set(pair) != {"unit_id", "build_a", "build_b", "fluid_batch_id"}:
            raise AnalysisError("independent_pair: strict_schema_failure")
        unit_ids.append(pair["unit_id"])
        if any(not isinstance(pair[k], str) or not pair[k] for k in ("unit_id", "build_a", "build_b", "fluid_batch_id")):
            raise AnalysisError("independent_pair: missing_identity")
        build_ids.extend([pair["build_a"], pair["build_b"]])
        batch_ids.append(pair["fluid_batch_id"])
    if len(set(unit_ids)) != len(unit_ids) or len(set(build_ids)) != len(build_ids) or len(set(batch_ids)) != len(batch_ids):
        raise AnalysisError("nominal_repeats_of_build_or_fluid_batch_are_not_independent_units")
    return {"independent_pairs": len(pairs), "independent_builds": len(build_ids), "independent_fluid_batches": len(batch_ids)}


def validate_holdout_partition(calibration: dict[str, Any], holdout: dict[str, Any]) -> dict[str, Any]:
    """Check strict partition identity and prevent specimen/preparation leakage."""
    required = {"schema_version", "partition_id", "status", "purpose", "records"}
    allowed = required | {"forbidden_inputs", "opening_condition"}
    for label, manifest in (("calibration", calibration), ("holdout", holdout)):
        if not required <= set(manifest) or not set(manifest) <= allowed or manifest["schema_version"] != 1:
            raise AnalysisError(f"{label}: partition_manifest_schema")
        for row in manifest["records"]:
            if set(row) != {"record_id", "subject_sha256", "fluid_batch_id"}:
                raise AnalysisError(f"{label}: record_identity_schema")
            if not row["record_id"] or not row["fluid_batch_id"] or not _is_sha256(row["subject_sha256"]):
                raise AnalysisError(f"{label}: record_identity_invalid")
    if not isinstance(holdout["status"], str) or not holdout["status"].startswith("sealed"):
        raise AnalysisError("holdout_must_remain_sealed")
    c_subjects = {x["subject_sha256"] for x in calibration["records"]}
    h_subjects = {x["subject_sha256"] for x in holdout["records"]}
    c_batches = {x["fluid_batch_id"] for x in calibration["records"]}
    h_batches = {x["fluid_batch_id"] for x in holdout["records"]}
    if c_subjects & h_subjects or c_batches & h_batches:
        raise AnalysisError("calibration_holdout_identity_leakage")
    return {"calibration_records": len(calibration["records"]), "holdout_records": len(holdout["records"]),
            "independent": True, "holdout_status": holdout["status"]}


PRIVATE_FIELDS = {"schema_version", "lineage", "first_seen_utc", "subject_sha256", "variant_id", "evidence",
                  "controls", "interventions", "human_decisions", "classification", "industrial_installation_authorized"}
PUBLIC_FIELDS = {"schema_version", "subject_sha256", "variant_id", "evidence_summary", "review_id", "reviewed_by_role", "reviewed_utc"}
SENSITIVE = re.compile(r"customer|facility|site_id|plant_id|account_id|address|contact|serial_number", re.I)


def build_private_dossier(data: dict[str, Any]) -> dict[str, Any]:
    if set(data) != PRIVATE_FIELDS or data["schema_version"] != 1:
        raise AnalysisError("private_dossier: strict_schema_failure")
    if data["classification"] != "private" or data["industrial_installation_authorized"] is not False:
        raise AnalysisError("dossier: private_classification_and_no_installation_authority_required")
    try:
        first_seen = datetime.fromisoformat(data["first_seen_utc"].replace("Z", "+00:00"))
        if first_seen.utcoffset() is None:
            raise ValueError("UTC offset required")
    except (AttributeError, TypeError, ValueError) as exc:
        raise AnalysisError("dossier: first_seen_must_be_iso8601") from exc
    if (not isinstance(data["lineage"], list) or not isinstance(data["evidence"], list)
            or not isinstance(data["controls"], list) or not isinstance(data["interventions"], list)
            or not isinstance(data["human_decisions"], list)
            or not data["first_seen_utc"] or not isinstance(data["subject_sha256"], str)
            or not re.fullmatch(r"[0-9a-f]{64}", data["subject_sha256"])
            or not data["variant_id"] or not data["lineage"] or not data["evidence"]):
        raise AnalysisError("dossier: lineage_time_subject_and_evidence_required")
    return deepcopy(data)


def store_private_dossier(data: dict[str, Any], private_root: str | Path, filename: str) -> Path:
    """Persist dossier with restrictive permissions under an explicit private root."""
    dossier = build_private_dossier(data)
    root = Path(private_root).resolve()
    if ".private" not in root.parts or Path(filename).name != filename or filename in {"", ".", ".."}:
        raise AnalysisError("private_storage_root_or_filename_rejected")
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    os.chmod(root, 0o700)
    target = root / filename
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    fd = os.open(target, flags, 0o600)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as stream:
            json.dump(dossier, stream, sort_keys=True, indent=2, allow_nan=False)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
    except Exception:
        target.unlink(missing_ok=True)
        raise
    return target


def export_reviewed_public(private: dict[str, Any], review: dict[str, Any]) -> dict[str, Any]:
    """Return only exact allowlisted public fields after affirmative human review."""
    private = build_private_dossier(private)
    review_fields = {"approved", "review_id", "reviewed_by_role", "reviewed_utc", "allowlist", "evidence_summary"}
    if set(review) != review_fields or review["approved"] is not True:
        raise AnalysisError("public_export_requires_explicit_human_review")
    if (not isinstance(review["allowlist"], list) or len(review["allowlist"]) != len(PUBLIC_FIELDS)
            or set(review["allowlist"]) != PUBLIC_FIELDS):
        raise AnalysisError("public_export_allowlist_must_equal_reviewed_schema")
    if (not review["review_id"] or not review["reviewed_by_role"] or not isinstance(review["evidence_summary"], list)):
        raise AnalysisError("public_export_review_fields_invalid")
    try:
        reviewed = datetime.fromisoformat(review["reviewed_utc"].replace("Z", "+00:00"))
        if reviewed.utcoffset() is None:
            raise ValueError("UTC offset required")
    except (AttributeError, TypeError, ValueError) as exc:
        raise AnalysisError("public_export_review_time_invalid") from exc
    serialized = json.dumps(private, sort_keys=True) + json.dumps(review["evidence_summary"], sort_keys=True)
    if SENSITIVE.search(serialized):
        raise AnalysisError("private_customer_or_facility_identifier_rejected")
    if private.get("industrial_installation_authorized") is not False:
        raise AnalysisError("research_or_software_evidence_cannot_authorize_installation")
    output = {"schema_version": 1, "subject_sha256": private["subject_sha256"],
              "variant_id": private["variant_id"], "evidence_summary": review["evidence_summary"],
              "review_id": review["review_id"], "reviewed_by_role": review["reviewed_by_role"],
              "reviewed_utc": review["reviewed_utc"]}
    if set(output) != PUBLIC_FIELDS:
        raise AnalysisError("public_export_schema_mismatch")
    return output
