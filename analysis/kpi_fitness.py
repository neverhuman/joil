#!/usr/bin/env python3
"""Source-bound KPI, 10-bin light-droplet grade curve, and multi-objective fitness evaluator.

Strictly follows AGENTS.md, docs/SCIENCE.md, requirements/RS-joil-research-v1.json, and COORD.md:
1. Primary evaluator (`evaluate_measured_cfd_kpis_and_fitness`) reads ONLY executed OpenFOAM
   boundary-integral files (`hydraulic-integrals-t*.json`), mesh audits (`mesh.json`), and
   executed Lagrangian cohort files.
2. Never fabricates water closure, pressure drop, or oil capture. If a carrier case has
   terminal backflow (e.g. short overflow plenum recirculation), failed mesh checks, or no
   executed oil cohort data, all oil-grade, concentration, compliance, and capacity fields
   remain strictly `null` (`eligible_for_oil_grade: false`, `qualified_multiobjective_vector: null`,
   `qualified_capacity_gain: null`).
3. Never clamps negative `E_reduced(d) = (E(d) - r) / (1 - r)` to zero.
4. Provides an explicitly isolated `evaluate_synthetic_algorithm_smoke_fixture` helper solely
   for unit-testing NSGA-II / MAP-Elites algorithm mechanics (`synthetic: true`), which never
   claims CFD anchoring or physical compliance (`passes_research_30_mg_l: null`).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
STANDARD_BINS_UM = [3.0, 5.0, 8.0, 10.0, 15.0, 20.0, 25.0, 40.0, 60.0, 100.0]
EXCLUDED_DIVERGED_CASES = {'ct35-hydraulic-diagnostic-001'}


def canonical_digest(obj: Any) -> str:
    """Deterministic SHA-256 over canonical JSON (rejects NaN/Inf)."""
    encoded = json.dumps(obj, sort_keys=True, separators=(',', ':'), allow_nan=False).encode('utf-8')
    return hashlib.sha256(encoded).hexdigest()


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def lognormal_cdf(d_um: float, d50_um: float, gsd: float) -> float:
    if d_um <= 0.0:
        return 0.0
    sigma = math.log(gsd)
    z = math.log(d_um / d50_um) / (sigma * math.sqrt(2.0))
    return 0.5 * (1.0 + math.erf(z))


def lognormal_bin_weights(
    bins_um: list[float] | None = None,
    d50_um: float = 15.0,
    gsd: float = 1.6,
) -> list[float]:
    """Compute exact normalized oil-volume/mass bin weights for a lognormal DSD."""
    if bins_um is None:
        bins_um = STANDARD_BINS_UM
    if d50_um <= 0.0 or gsd <= 1.0 or not math.isfinite(d50_um) or not math.isfinite(gsd):
        raise ValueError('invalid lognormal DSD parameters')
    n = len(bins_um)
    if n < 2 or any(bins_um[i] >= bins_um[i + 1] or bins_um[i] <= 0 for i in range(n - 1)):
        raise ValueError('strictly increasing positive diameter bins required')
    edges = [0.0]
    for i in range(n - 1):
        edges.append(math.sqrt(bins_um[i] * bins_um[i + 1]))
    weights = []
    for i in range(n):
        hi_cdf = 1.0 if i == n - 1 else lognormal_cdf(edges[i + 1], d50_um, gsd)
        lo_cdf = 0.0 if i == 0 else lognormal_cdf(edges[i], d50_um, gsd)
        weights.append(max(0.0, hi_cdf - lo_cdf))
    total = sum(weights)
    if abs(total - 1.0) > 1e-12:
        weights = [w / total for w in weights]
    return weights


def concentration_mg_l(cin_mg_l: float, capture: float, r: float) -> float:
    """Split-corrected treated-water dispersed oil concentration: Cu = Ci * (1 - E) / (1 - r)."""
    if not (math.isfinite(cin_mg_l) and cin_mg_l >= 0.0):
        raise ValueError('invalid cin_mg_l')
    if not (math.isfinite(capture) and 0.0 <= capture <= 1.0):
        raise ValueError(f'invalid capture {capture}')
    if not (math.isfinite(r) and 0.0 <= r < 1.0):
        raise ValueError(f'invalid split {r}')
    return cin_mg_l * (1.0 - capture) / (1.0 - r)


def reduced_efficiency(capture: float, r: float) -> float:
    """Split-corrected reduced separation efficiency: E_reduced = (E - r) / (1 - r). Never clamped at 0."""
    if not (math.isfinite(capture) and 0.0 <= capture <= 1.0):
        raise ValueError('invalid capture')
    if not (math.isfinite(r) and 0.0 <= r < 1.0):
        raise ValueError('invalid split')
    return (capture - r) / (1.0 - r)


def pdr(p_in_pa: float, p_o_pa: float, p_u_pa: float) -> float:
    if any(not math.isfinite(x) for x in [p_in_pa, p_o_pa, p_u_pa]) or p_in_pa <= p_u_pa or p_in_pa < p_o_pa:
        raise ValueError('invalid pressure ordering for PDR')
    return (p_in_pa - p_o_pa) / (p_in_pa - p_u_pa)


def verify_passive_tracer_split_oracle(cin_mg_l: float = 200.0, r: float = 0.02) -> dict[str, Any]:
    """Arithmetic passive-tracer split oracle: when E(d) == r for all bins, E_reduced == 0 and Cu == Ci."""
    cu_tr = concentration_mg_l(cin_mg_l, r, r)
    e_red = reduced_efficiency(r, r)
    err = max(abs(cu_tr - cin_mg_l), abs(e_red))
    return {
        'passed': err < 1e-12,
        'kind': 'arithmetic_split_formula_oracle_not_native_cfd_tracer',
        'cout_tracer_mg_l': cu_tr,
        'e_reduced_tracer': e_red,
        'max_abs_error': err,
    }


def close_cohort_inventory(
    injected_kg: float,
    reject_kg: float,
    treated_kg: float,
    remaining_kg: float = 0.0,
    wall_kg: float = 0.0,
    max_closure: float = 0.01,
) -> dict[str, float]:
    """Verify cohort mass conservation and return adversarial capture bounds (wall/remaining never counted as captured in lower bound)."""
    vals = [injected_kg, reject_kg, treated_kg, remaining_kg, wall_kg]
    if any(not math.isfinite(v) or v < 0.0 for v in vals) or injected_kg <= 0.0:
        raise ValueError('invalid cohort oil inventory')
    total = reject_kg + treated_kg + remaining_kg + wall_kg
    residual = abs(injected_kg - total) / injected_kg
    if residual > max_closure:
        raise ValueError(f'oil cohort closure {residual:.6g} exceeds limit {max_closure}')
    raw_capture = reject_kg / injected_kg
    if raw_capture > 1.0 + 1e-12:
        raise ValueError('reject mass exceeds injected mass')
    raw_capture = min(1.0, raw_capture)
    unresolved = (remaining_kg + wall_kg) / injected_kg
    capture_lo = max(0.0, raw_capture - residual)
    capture_hi = min(1.0, raw_capture + unresolved + residual)
    return {
        'injected_kg': injected_kg,
        'reject_kg': reject_kg,
        'treated_kg': treated_kg,
        'remaining_kg': remaining_kg,
        'wall_kg': wall_kg,
        'residual_fraction': residual,
        'unresolved_fraction': unresolved,
        'capture_point': raw_capture,
        'capture_lower': capture_lo,
        'capture_upper': capture_hi,
    }


def build_breakup_transfer_matrix(
    bins_um: list[float],
    eps_inlet_m2_s3: float,
    residence_shear_s: float,
    rho_c: float = 998.0,
    sigma_n_m: float = 0.025,
) -> tuple[list[list[float]], dict[str, float]]:
    """Construct a mass-conserving Kolmogorov-Hinze droplet breakup transfer matrix T[i][j] (rows sum to 1.0)."""
    n = len(bins_um)
    eps = max(1e-6, eps_inlet_m2_s3)
    d_hinze_m = 0.725 * ((sigma_n_m / rho_c) ** 0.6) * (eps ** (-0.4))
    d_hinze_um = d_hinze_m * 1e6
    we_crit = 0.725 ** (5.0 / 3.0)

    matrix: list[list[float]] = []
    breakup_fractions: list[float] = []
    for i, d_um in enumerate(bins_um):
        d_m = d_um * 1e-6
        we = rho_c * (eps ** (2.0 / 3.0)) * (d_m ** (5.0 / 3.0)) / sigma_n_m
        if we <= we_crit or i == 0:
            row = [0.0] * n
            row[i] = 1.0
            matrix.append(row)
            breakup_fractions.append(0.0)
            continue
        tau_b = max(1e-5, 0.8 * ((rho_c * (d_m ** 2) / (eps ** (2.0 / 3.0))) ** 0.5) / max(0.05, we / we_crit - 1.0))
        p_break = min(0.85, 1.0 - math.exp(-residence_shear_s / tau_b))
        daughter_weights = []
        for j in range(i):
            dj = bins_um[j]
            dist = math.log(dj / min(d_um * 0.65, max(bins_um[0], d_hinze_um * 0.85)))
            daughter_weights.append(math.exp(-1.5 * (dist ** 2)))
        w_sum = sum(daughter_weights)
        row = [0.0] * n
        for j in range(i):
            row[j] = p_break * (daughter_weights[j] / w_sum)
        row[i] = 1.0 - p_break
        row_sum = sum(row)
        row = [x / row_sum for x in row]
        matrix.append(row)
        breakup_fractions.append(p_break)

    for row in matrix:
        if abs(sum(row) - 1.0) > 1e-12 or any(x < -1e-15 for x in row):
            raise ValueError('breakup transfer matrix row failed mass conservation')
    return matrix, {
        'd_hinze_um': d_hinze_um,
        'eps_inlet_m2_s3': eps,
        'residence_shear_s': residence_shear_s,
        'max_bin_breakup_fraction': max(breakup_fractions) if breakup_fractions else 0.0,
    }


def verify_carrier_case_provenance(case_dir: Path, time_step: int) -> dict[str, Any]:
    """Verify input-manifest.json (rejecting path traversal), actual joilHydraulicMetrics ELF binary, receipt, and failure markers."""
    case_dir_resolved = case_dir.resolve()
    root_resolved = ROOT.resolve()
    manifest_path = case_dir_resolved / 'input-manifest.json'
    manifest_verified = False
    manifest_sha256 = None
    mismatched_inputs: list[str] = []
    if manifest_path.exists():
        manifest_sha256 = file_sha256(manifest_path)
        manifest_data = json.loads(manifest_path.read_text())
        if not isinstance(manifest_data, dict) or not manifest_data:
            mismatched_inputs.append('<empty_or_non_dict_input_manifest>')
        else:
            for rel_file, expected_hash in manifest_data.items():
                rel_p = Path(rel_file)
                if rel_p.is_absolute() or '..' in rel_p.parts:
                    mismatched_inputs.append(f'escaped_path:{rel_file}')
                    continue
                target = (case_dir_resolved / rel_p).resolve()
                if not target.is_relative_to(case_dir_resolved):
                    mismatched_inputs.append(f'escaped_symlink:{rel_file}')
                    continue
                if not target.is_file() or file_sha256(target) != expected_hash:
                    mismatched_inputs.append(rel_file)
            manifest_verified = len(mismatched_inputs) == 0

    metrics_bin_meta_path = case_dir_resolved / f'metrics-binary-t{time_step}.json'
    metrics_bin_meta_sha256 = None
    metrics_bin_exec_sha256 = None
    metrics_bin_verified = False
    if metrics_bin_meta_path.is_file():
        metrics_bin_meta_sha256 = file_sha256(metrics_bin_meta_path)
        try:
            bin_meta = json.loads(metrics_bin_meta_path.read_text())
            bin_rel = str(bin_meta.get('path') or bin_meta.get('binary') or '')
            expected_bin_sha = str(bin_meta.get('sha256', ''))
            bin_p = Path(bin_rel)
            if bin_rel and not bin_p.is_absolute() and '..' not in bin_p.parts:
                bin_target = (root_resolved / bin_p).resolve()
                if bin_target.is_relative_to(root_resolved) and bin_target.is_file():
                    metrics_bin_exec_sha256 = file_sha256(bin_target)
                    metrics_bin_verified = (
                        len(expected_bin_sha) == 64 and metrics_bin_exec_sha256 == expected_bin_sha
                    )
        except Exception:
            metrics_bin_verified = False

    metrics_receipt_path = case_dir_resolved / f'metrics-t{time_step}-0-joilHydraulicMetrics.log.receipt.json'
    metrics_log_path = case_dir_resolved / f'metrics-t{time_step}-0-joilHydraulicMetrics.log'
    receipt_verified = False
    receipt_sha256 = None
    if metrics_receipt_path.is_file() and metrics_log_path.is_file():
        receipt_sha256 = file_sha256(metrics_receipt_path)
        rec = json.loads(metrics_receipt_path.read_text())
        if (
            rec.get('exit_code') == 0
            and rec.get('stop_reason') is None
            and rec.get('log_sha256') == file_sha256(metrics_log_path)
        ):
            receipt_verified = True

    failed_marker = any(
        (case_dir_resolved / marker).exists()
        for marker in ('FAILED', 'failed.json', 'numerical-failure.json')
    )
    return {
        'input_manifest_sha256': manifest_sha256,
        'input_manifest_verified': manifest_verified,
        'mismatched_inputs': mismatched_inputs,
        'metrics_binary_metadata_sha256': metrics_bin_meta_sha256,
        'metrics_binary_sha256': metrics_bin_exec_sha256,
        'metrics_binary_verified': metrics_bin_verified,
        'metrics_receipt_sha256': receipt_sha256,
        'metrics_receipt_verified': receipt_verified,
        'failed_marker_present': failed_marker,
    }


def evaluate_measured_cfd_kpis_and_fitness(
    integrals_path: Path,
    expected_time_step: int | None = None,
    oil_cohorts_path: Path | None = None,
    max_terminal_backflow_ratio: float = 0.01,
) -> dict[str, Any]:
    """Evaluate source-bound hydraulic and (when eligible) oil KPIs from an executed OpenFOAM case.

    Never fabricates oil capture, water closure, or pressure drop.
    """
    integrals_path = integrals_path.resolve()
    case_dir = integrals_path.parent
    if case_dir.name in EXCLUDED_DIVERGED_CASES:
        raise ValueError(f'Refusing diverged case {case_dir.name}')

    req_path = ROOT / 'requirements/RS-joil-research-v1.json'
    req = json.loads(req_path.read_text())
    integrals = json.loads(integrals_path.read_text())

    if 'time' not in integrals or not isinstance(integrals['time'], (int, float)):
        raise ValueError(f'Missing or non-numeric time in {integrals_path}')
    time_step = int(integrals['time'])
    if expected_time_step is not None and time_step != int(expected_time_step):
        raise ValueError(f'Integrals time {time_step} does not match expected time_step {expected_time_step}')

    prov = verify_carrier_case_provenance(case_dir, time_step)
    if prov['failed_marker_present']:
        raise ValueError(f'Refusing case {case_dir.name}: failed-run / numerical-failure marker present')
    if prov['mismatched_inputs'] or not prov['input_manifest_verified']:
        raise ValueError(f'Refusing case {case_dir.name}: invalid or modified inputs: {prov["mismatched_inputs"]}')

    status_path = case_dir / 'research-status.json'
    status_doc = json.loads(status_path.read_text()) if status_path.exists() else {}

    patches = integrals['patches']
    for required_patch in ('inlet1', 'inlet2', 'overflow', 'underflow'):
        if required_patch not in patches:
            raise ValueError(f'Missing required patch {required_patch} in {integrals_path}')
        for req_key in ('signed_flow_m3_s', 'reverse_flow_m3_s', 'area_average_static_pressure_pa'):
            if req_key not in patches[required_patch]:
                raise ValueError(f'Missing {req_key} on patch {required_patch} in {integrals_path}')
            val = float(patches[required_patch][req_key])
            if not math.isfinite(val):
                raise ValueError(f'Non-finite {req_key} on patch {required_patch} in {integrals_path}')

    p_in1 = float(patches['inlet1']['area_average_static_pressure_pa'])
    p_in2 = float(patches['inlet2']['area_average_static_pressure_pa'])
    p_in = 0.5 * (p_in1 + p_in2)
    p_u = float(patches['underflow']['area_average_static_pressure_pa'])
    p_o = float(patches['overflow']['area_average_static_pressure_pa'])

    dp_u = p_in - p_u
    dp_o = p_in - p_o
    pdr_val = pdr(p_in, p_o, p_u)

    q_feed_m3_h = float(integrals['feed_m3_h'])
    q_treated_m3_h = float(integrals['treated_m3_h'])
    r_act = float(integrals['reject_fraction'])
    water_closure = float(integrals['water_closure_fraction'])
    loss_w = float(integrals['hydraulic_loss_w'])
    loss_j_m3 = float(integrals['hydraulic_loss_j_per_treated_m3'])

    ov_net = float(patches['overflow']['signed_flow_m3_s'])
    ov_rev = float(patches['overflow']['reverse_flow_m3_s'])
    un_net = float(patches['underflow']['signed_flow_m3_s'])
    un_rev = float(patches['underflow']['reverse_flow_m3_s'])
    if ov_net <= 0.0 or un_net <= 0.0 or ov_rev < 0.0 or un_rev < 0.0:
        raise ValueError(
            f'Invalid outlet flow sign in {integrals_path}: '
            f'overflow_signed={ov_net}, underflow_signed={un_net}, ov_rev={ov_rev}, un_rev={un_rev}'
        )
    ov_rev_ratio = ov_rev / ov_net
    un_rev_ratio = un_rev / un_net

    orig_dev = integrals.get('original_device_overflow')
    device_loss_j_m3 = float(orig_dev['device_hydraulic_loss_j_per_treated_m3']) if orig_dev else None
    device_loss_w = float(orig_dev['device_hydraulic_loss_w']) if orig_dev else None
    device_ov_rev = float(orig_dev['reverse_flow_m3_s']) if orig_dev else None

    ineligibility_reasons: list[str] = []
    if not prov['input_manifest_verified']:
        ineligibility_reasons.append('input_manifest_missing_or_unverified')
    if not prov['metrics_binary_verified']:
        ineligibility_reasons.append('native_metrics_binary_missing_or_unverified')
    if not prov['metrics_receipt_verified']:
        ineligibility_reasons.append('native_metrics_receipt_missing_or_unverified')
    if not math.isfinite(water_closure) or water_closure > 0.001:
        ineligibility_reasons.append(f'water_closure_exceeds_0.001 ({water_closure:g})')
    if abs(r_act - 0.02) > 0.0002:
        ineligibility_reasons.append(f'reject_split_outside_tight_nominal_band ({r_act:.6f} vs 0.02 +/- 0.0002)')
    if ov_rev_ratio > max_terminal_backflow_ratio:
        ineligibility_reasons.append(
            f'overflow_terminal_reverse_flow_ratio_{ov_rev_ratio:.3f}_exceeds_{max_terminal_backflow_ratio}_'
            f'(reverse={ov_rev:.6g}_m3_s_vs_net={ov_net:.6g}_m3_s)'
        )
    if un_rev_ratio > max_terminal_backflow_ratio:
        ineligibility_reasons.append(f'underflow_terminal_reverse_flow_ratio_{un_rev_ratio:.3f}_exceeds_{max_terminal_backflow_ratio}')
    if not bool(status_doc.get('mesh_pilot_pass', False)):
        ineligibility_reasons.append('mesh_pilot_pass_is_false_in_research_status')
    if not bool(status_doc.get('campaign_admission', False)):
        ineligibility_reasons.append('case_not_campaign_admitted_in_research_status')

    verified_oil_cohort = False
    if oil_cohorts_path is None or not oil_cohorts_path.is_file():
        ineligibility_reasons.append('no_executed_full_liner_oil_cohorts_bound')
    else:
        try:
            oil_doc = json.loads(oil_cohorts_path.read_text())
            if (
                oil_doc.get('is_qualified_grade_curve') is True
                and oil_doc.get('inventory_closure_passed') is True
                and isinstance(oil_doc.get('cohorts'), list)
                and len(oil_doc['cohorts']) == len(STANDARD_BINS_UM)
            ):
                for c_item in oil_doc['cohorts']:
                    close_cohort_inventory(
                        float(c_item['injected_kg']),
                        float(c_item['reject_kg']),
                        float(c_item['treated_kg']),
                        float(c_item.get('remaining_kg', 0.0)),
                        float(c_item.get('wall_kg', 0.0)),
                    )
                verified_oil_cohort = True
            else:
                ineligibility_reasons.append('oil_cohort_file_is_capability_probe_or_unqualified')
        except Exception as exc:
            ineligibility_reasons.append(f'invalid_oil_cohort_file:{exc}')

    hydraulics_and_terminal_backflow_pass = (
        prov['input_manifest_verified']
        and prov['metrics_binary_verified']
        and prov['metrics_receipt_verified']
        and math.isfinite(water_closure)
        and water_closure <= 0.001
        and abs(r_act - 0.02) <= 0.0002
        and ov_rev_ratio <= max_terminal_backflow_ratio
        and un_rev_ratio <= max_terminal_backflow_ratio
    )
    eligible_for_oil = (len(ineligibility_reasons) == 0) and verified_oil_cohort

    return {
        'schema_version': 2,
        'kind': 'source_bound_cfd_kpi_and_eligibility_report',
        'is_synthetic_smoke_fixture': False,
        'case_name': case_dir.name,
        'time_step': time_step,
        'integrals_file': str(integrals_path.relative_to(ROOT)),
        'integrals_sha256': file_sha256(integrals_path),
        'research_status_sha256': file_sha256(status_path) if status_path.exists() else None,
        'provenance': prov,
        'requirements_id': req['id'],
        'requirements_sha256': file_sha256(req_path),
        'qualification_tier': 'executed_f1_hydraulic_diagnostic_unqualified_for_oil_grade',
        'hydraulics_and_terminal_backflow_pass_for_diagnostic_pilot': hydraulics_and_terminal_backflow_pass,
        'eligible_for_oil_grade': eligible_for_oil,
        'eligible_for_campaign_fitness': False,
        'ineligibility_reasons': ineligibility_reasons,
        'qualified_multiobjective_vector': None,
        'qualified_capacity_gain': None,
        'measured_hydraulics': {
            'feed_flow_m3_h': q_feed_m3_h,
            'treated_flow_m3_h': q_treated_m3_h,
            'reject_fraction': r_act,
            'reject_split_error_vs_0.02': r_act - 0.02,
            'water_closure_fraction': water_closure,
            'inlet1_static_pressure_pa': p_in1,
            'inlet2_static_pressure_pa': p_in2,
            'inlet_mean_static_pressure_pa': p_in,
            'underflow_static_pressure_pa': p_u,
            'overflow_static_pressure_pa': p_o,
            'dp_inlet_to_underflow_pa': dp_u,
            'dp_inlet_to_overflow_pa': dp_o,
            'pdr': pdr_val,
            'hydraulic_loss_w': loss_w,
            'hydraulic_loss_j_per_treated_m3': loss_j_m3,
            'device_hydraulic_loss_w': device_loss_w,
            'device_hydraulic_loss_j_per_treated_m3': device_loss_j_m3,
            'internal_device_overflow_reverse_flow_m3_s': device_ov_rev,
            'overflow_signed_flow_m3_s': ov_net,
            'overflow_reverse_flow_m3_s': ov_rev,
            'overflow_reverse_to_net_ratio': ov_rev_ratio,
            'underflow_signed_flow_m3_s': un_net,
            'underflow_reverse_flow_m3_s': un_rev,
            'underflow_reverse_to_net_ratio': un_rev_ratio,
        },
        'oil_kpis': {
            'e_oil_point': None,
            'e_reduced_point': None,
            'cout_point_mg_l': None,
            'cout_lower_mg_l': None,
            'cout_upper_mg_l': None,
            'passes_research_30_mg_l': None,
            'passes_stretch_20_mg_l': None,
            'grade_curve': None,
        },
        'fitness': {
            'qualified_multiobjective_vector': None,
            'qualified_capacity_gain': None,
            'note': 'Null because seed mesh checkMesh concavities (mesh_pilot_pass=false) and F1 steady initializer status prohibit campaign fitness admission.',
        },
    }


def evaluate_synthetic_algorithm_smoke_fixture(
    design_id: str,
    family: str,
    descriptors: dict[str, float],
    f0_passed: bool = True,
    mesh_checks_passed: bool = False,
    baseline_reference: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Explicitly synthetic, uncalibrated test function solely for unit-testing NSGA-II / MAP-Elites algorithm mechanics.

    NEVER used as baseline or candidate physical evidence. All compliance and qualified fields are strictly None.
    """
    swirl = float(descriptors.get('swirl_flux_index', 1.0))
    pitch_sin = float(descriptors.get('axial_pitch_sin', 0.0))
    shear_mit = float(descriptors.get('shear_mitigation_factor', 1.0))
    cone_acc = float(descriptors.get('cone_accel_persistence', 1.0))
    core_shield = float(descriptors.get('core_shield_ratio', 0.0))
    rifling = float(descriptors.get('wall_rifling_stabilization', 0.0))
    res_factor = float(descriptors.get('hydraulic_resistance_factor', 1.0))

    r_nom = 0.02
    cin_nom = 200.0
    weights = lognormal_bin_weights(STANDARD_BINS_UM, 15.0, 1.6)
    eps_in = 320.0 * (shear_mit ** 1.25)
    t_mat, breakup_diag = build_breakup_transfer_matrix(STANDARD_BINS_UM, eps_in, 0.0085)

    eff_weights = [0.0] * len(STANDARD_BINS_UM)
    for i, wi in enumerate(weights):
        for j in range(len(STANDARD_BINS_UM)):
            eff_weights[j] += wi * t_mat[i][j]

    d50_cut = 16.5 / ((swirl ** 1.15) * (cone_acc ** 0.55) * (1.0 + 0.08 * pitch_sin + 0.07 * core_shield + 0.05 * rifling))
    cohorts = []
    for k, d_um in enumerate(STANDARD_BINS_UM):
        e_red_k = 1.0 - math.exp(-0.693147 * ((d_um / d50_cut) ** 1.82))
        e_k = r_nom + (1.0 - r_nom) * e_red_k
        cohorts.append({
            'diameter_um': d_um,
            'inlet_mass_weight': weights[k],
            'post_breakup_mass_weight': eff_weights[k],
            'synthetic_capture_point': e_k,
            'synthetic_reduced_efficiency': reduced_efficiency(e_k, r_nom),
        })

    e_synth = sum(eff_weights[k] * cohorts[k]['synthetic_capture_point'] for k in range(len(STANDARD_BINS_UM)))
    cu_synth = concentration_mg_l(cin_nom, e_synth, r_nom)
    dp_synth_pa = 30854.67 * res_factor
    q_synth_m3_h = 3.92 / math.sqrt(res_factor)

    violation = 0.0 if f0_passed else 1.0
    objectives = [
        -q_synth_m3_h,
        cu_synth,
        dp_synth_pa,
    ]
    score = (85.0 - cu_synth) + 10.0 * (q_synth_m3_h - 3.92) - 500.0 * violation

    return {
        'schema_version': 2,
        'design_id': design_id,
        'family': family,
        'synthetic': True,
        'is_synthetic_smoke_fixture': True,
        'qualification_tier': 'synthetic_algorithm_smoke_test_not_physical_or_cfd_evidence',
        'qualified_multiobjective_vector': None,
        'qualified_capacity_gain': None,
        'passes_research_30_mg_l': None,
        'passes_stretch_20_mg_l': None,
        'kpis': {
            'synthetic': True,
            'status': 'synthetic_uncalibrated_test_function_for_search_unit_testing_only',
            'qualified_multiobjective_vector': None,
            'qualified_capacity_gain': None,
            'passes_research_30_mg_l': None,
            'passes_stretch_20_mg_l': None,
            'synthetic_cout_mg_l': cu_synth,
            'synthetic_e_oil': e_synth,
            'synthetic_e_reduced': reduced_efficiency(e_synth, r_nom),
            'synthetic_dp_pa': dp_synth_pa,
            'synthetic_flow_index_m3_h': q_synth_m3_h,
            'd_hinze_inlet_um': breakup_diag['d_hinze_um'],
        },
        'fitness': {
            'synthetic': True,
            'status': 'synthetic_algorithm_smoke_test_only',
            'qualified_multiobjective_vector': None,
            'qualified_capacity_gain': None,
            'f0_candidate_geometry_passed': f0_passed,
            'mesh_checks_passed': mesh_checks_passed,
            'constraint_violation': violation,
            'pareto_objectives_minimize': {
                'neg_synthetic_flow_index': objectives[0],
                'synthetic_cout_mg_l': objectives[1],
                'synthetic_dp_pa': objectives[2],
            },
            'synthetic_smoke_score': score,
        },
        'descriptors': descriptors,
        'grade_curve': cohorts,
        'breakup_diagnostics': breakup_diag,
    }


def constrained_dominates(cand_a: dict[str, Any], cand_b: dict[str, Any]) -> bool:
    """Deb's constrained Pareto dominance (SCIENCE.md §3)."""
    fa = cand_a['fitness']
    fb = cand_b['fitness']
    va = float(fa['constraint_violation'])
    vb = float(fb['constraint_violation'])
    if va == 0.0 and vb > 0.0:
        return True
    if va > 0.0 and vb == 0.0:
        return False
    if va > 0.0 and vb > 0.0:
        return va < vb - 1e-12
    oa = list(fa['pareto_objectives_minimize'].values())
    ob = list(fb['pareto_objectives_minimize'].values())
    no_worse = all(a <= b + 1e-12 for a, b in zip(oa, ob))
    strictly_better = any(a < b - 1e-12 for a, b in zip(oa, ob))
    return no_worse and strictly_better


if __name__ == '__main__':
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        '--integrals',
        type=Path,
        default=ROOT / 'runs/ct35-sst-extended-000/hydraulic-integrals-t600.json',
        help='Path to executed OpenFOAM boundary mechanical flux JSON',
    )
    ap.add_argument('--time-step', type=int, default=600, help='Required explicit numeric saved time')
    ap.add_argument('--out', type=Path, default=None, help='Immutable output path (opened with x mode)')
    args = ap.parse_args()
    rep = evaluate_measured_cfd_kpis_and_fitness(args.integrals, expected_time_step=args.time_step)
    all_points = []
    for p_rel, t_exp in [
        ('runs/ct35-sst-diagnostic-000/hydraulic-integrals-t300.json', 300),
        ('runs/ct35-sst-split-001/hydraulic-integrals-t300.json', 300),
        ('runs/ct35-sst-extended-000/hydraulic-integrals-t600.json', 600),
    ]:
        p_full = ROOT / p_rel
        if p_full.exists():
            all_points.append(evaluate_measured_cfd_kpis_and_fitness(p_full, expected_time_step=t_exp))
    rep['executed_baseline_operating_points'] = all_points
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        with open(args.out, 'x', encoding='utf-8') as f:
            f.write(json.dumps(rep, indent=2) + '\n')
    print(json.dumps(rep, indent=2))

