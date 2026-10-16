#!/usr/bin/env python3
"""Frozen-carrier 10-bin monodisperse light-droplet grade efficiency & inventory tracker (OpenFOAM 14).

Implements P05 requirements and Codex COORD.md agreements:
1. Refuses diverged or unverified carrier fields (e.g. runs/ct35-hydraulic-diagnostic-001).
   Requires an explicit numeric saved time_step, verifies input-manifest.json and native
   joilHydraulicMetrics receipts, and rejects missing/non-finite/non-positive outlet signed flow
   or terminal reverse flow > 1% on either overflow or underflow.
2. Uses pinned Foundation 14 Lagrangian force conventions:
   - rhoByRhoc = 850 / 998 (light oil in water)
   - SchillerNaumannDrag
   - pressureGradientForce (drives inward radial migration in pressure vortex)
   - constantCoefficientVirtualMass (Cvm = 0.5)
   - gravity
   - Stationary frame (no double-counted centrifugal term).
3. Deterministic monodisperse size-class cohorts: [3, 5, 8, 10, 15, 20, 25, 40, 60, 100] um + passive tracer.
4. Tracks native 4-way parcel/mass inventory using Foundation 14 cloudNumberFlux (<cloud>:phiNumber)
   + surfaceFieldValue patch sums time-integrated with actual dt, plus native Lagrangian/<cloud>/face
   patch/internal classification:
   - rejected_exit_fraction (overflow terminal patch removal via cloudNumberFlux)
   - treated_exit_fraction (underflow terminal patch removal via cloudNumberFlux)
   - wall_fraction (wall stickVelocity hit on wall patch faces, never counted as captured)
   - remaining_in_domain_fraction (still in internal mesh at stop time)
   - discrepancy_fraction (injected - [rejected + treated + wall + remaining])
5. Never overwrites existing result.json files; writes new immutable revisioned files with 'x' mode.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import sys
import uuid
from typing import Any

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'physics/openfoam/environment'))
sys.path.insert(0, str(ROOT / 'analysis'))

from runtime import run as runtime_run
from kpi_fitness import (
    STANDARD_BINS_UM,
    close_cohort_inventory,
    concentration_mg_l,
    file_sha256,
    lognormal_bin_weights,
    reduced_efficiency,
    verify_carrier_case_provenance,
)

DEFAULT_LOGNORMAL_DSD = {'d50_um': 15.0, 'gsd': 1.6}
EXCLUDED_DIVERGED_CASES = {'ct35-hydraulic-diagnostic-001'}


def write_immutable_json(path: Path, payload: dict[str, Any]) -> Path:
    """Write JSON without ever overwriting an existing file (uses exclusive 'x' creation)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    target = path
    rev = 2
    while target.exists():
        target = path.with_name(f'{path.stem}-r{rev}{path.suffix}')
        rev += 1
    with open(target, 'x', encoding='utf-8') as f:
        f.write(json.dumps(payload, indent=2) + '\n')
    return target


def verify_passive_tracer_split_oracle(cin_mg_l: float = 200.0, r: float = 0.02) -> dict[str, Any]:
    cu_tr = concentration_mg_l(cin_mg_l, r, r)
    e_red = reduced_efficiency(r, r)
    err = max(abs(cu_tr - cin_mg_l), abs(e_red))
    return {'passed': err < 1e-12, 'cout_tracer_mg_l': cu_tr, 'e_reduced_tracer': e_red, 'max_abs_error': err}


def foam_file(name: str, text: str, cls: str = 'dictionary') -> str:
    return f'FoamFile {{ format ascii; class {cls}; object {name}; }}\n{text}\n'


def write_dict(case: Path, relpath: str, text: str, cls: str = 'dictionary') -> None:
    p = case / relpath
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(foam_file(p.name, text, cls))


def parse_poly_mesh_boundary(boundary_path: Path) -> dict[str, Any]:
    """Dynamically parse constant/polyMesh/boundary for exact patch startFace and nFaces."""
    if not boundary_path.exists():
        raise ValueError(f'Missing polyMesh/boundary file: {boundary_path}')
    s = re.sub(r'FoamFile\s*\{.*?\}\s*', '', boundary_path.read_text(), flags=re.S)
    patches: dict[str, dict[str, int]] = {}
    for m in re.finditer(r'(\w+)\s*\{\s*([^}]+)\}', s, re.S):
        pname = m.group(1)
        body = m.group(2)
        m_nf = re.search(r'nFaces\s+(\d+)\s*;', body)
        m_sf = re.search(r'startFace\s+(\d+)\s*;', body)
        if m_nf and m_sf:
            sf = int(m_sf.group(1))
            nf = int(m_nf.group(1))
            patches[pname] = {'startFace': sf, 'nFaces': nf, 'endFace': sf + nf}
    if not patches or 'wall' not in patches or 'overflow' not in patches or 'underflow' not in patches:
        raise ValueError(f'Failed to parse required patches from {boundary_path}: {list(patches.keys())}')
    n_internal = min(p['startFace'] for p in patches.values())
    return {'nInternalFaces': n_internal, 'patches': patches, 'boundary_sha256': file_sha256(boundary_path)}


def parse_vectors(path: Path) -> list[list[float]]:
    if not path.exists():
        raise ValueError(f'Missing required vector field file: {path}')
    s = path.read_text()
    m = re.search(r'internalField\s+nonuniform\s+List<vector>\s+(\d+)\s*\((.*?)\)\s*;', s, re.S)
    if m:
        count = int(m.group(1))
        body = m.group(2)
        if count == 0:
            if body.strip():
                raise ValueError(f'Extra entries in zero-length vector field {path}')
            return []
        tuple_pat = r'\(\s*([-+0-9.eE]+)\s+([-+0-9.eE]+)\s+([-+0-9.eE]+)\s*\)'
        tuples = re.findall(tuple_pat, body)
        remainder = re.sub(tuple_pat, '', body).strip()
        if remainder:
            raise ValueError(f'Malformed or trailing tokens in vector field {path}: {remainder!r}')
        if len(tuples) != count:
            raise ValueError(f'Vector field count mismatch in {path}: expected {count}, parsed {len(tuples)}')
        out: list[list[float]] = []
        for a, b, c in tuples:
            vec = [float(a), float(b), float(c)]
            if not all(math.isfinite(v) for v in vec):
                raise ValueError(f'Non-finite vector entry in {path}: {vec}')
            out.append(vec)
        return out
    if re.search(r'internalField\s+nonuniform\s+List<vector>\s+0\s*\(\s*\)', s):
        return []
    m_uni = re.search(r'internalField\s+uniform\s*\(\s*([-+0-9.eE]+)\s+([-+0-9.eE]+)\s+([-+0-9.eE]+)\s*\)\s*;', s)
    if m_uni:
        vec = [float(m_uni.group(1)), float(m_uni.group(2)), float(m_uni.group(3))]
        if not all(math.isfinite(v) for v in vec):
            raise ValueError(f'Non-finite uniform vector in {path}: {vec}')
        return [vec]
    raise ValueError(f'Unparsable vector field file: {path}')


def parse_labels(path: Path) -> list[int]:
    if not path.exists():
        raise ValueError(f'Missing required labelField file: {path}')
    s = path.read_text()
    # Strip FoamFile header block and C++ comments so numbers inside header do not match
    s_body = re.sub(r'FoamFile\s*\{.*?\}\s*', '', s, flags=re.S)
    s_body = re.sub(r'//.*', '', s_body)
    s_body = re.sub(r'/\*.*?\*/', '', s_body, flags=re.S).strip()
    m = re.match(r'^(?:dimensions\s*\[[^\]]*\]\s*;\s*)?(?:internalField\s+)?(?:nonuniform\s+List<label>\s+)?(\d+)\s*\((.*?)\)\s*;?\s*$', s_body, re.S)
    if m:
        count = int(m.group(1))
        tokens = m.group(2).split()
        if len(tokens) != count:
            raise ValueError(f'labelField count mismatch in {path}: expected {count}, parsed {len(tokens)}')
        return [int(x) for x in tokens]
    if re.match(r'^0\s*\(\s*\)\s*;?\s*$', s_body):
        return []
    m_uni = re.match(r'^(\d+)\s*\{\s*(-?\d+)\s*\}\s*;?\s*$', s_body)
    if m_uni:
        return [int(m_uni.group(2))] * int(m_uni.group(1))
    raise ValueError(f'Unparsable labelField file: {path}')


def checked_near_integer_flux(flux_integral: float, tol: float = 1e-6) -> tuple[int, float]:
    nearest = round(flux_integral)
    residual = abs(flux_integral - nearest)
    if not math.isfinite(flux_integral) or residual > tol:
        raise ValueError(f'cloudNumberFlux integral {flux_integral:.12g} exceeds integer tolerance {tol} (residual={residual:.6g})')
    return int(nearest), float(residual)


def parse_surface_field_value_sums(dat_path: Path, cloud_names: list[str], dt: float) -> dict[str, float]:
    """Integrate cloudNumberFlux (number/s) * dt over all time steps from surfaceFieldValue.dat."""
    if not dat_path.exists():
        raise ValueError(f'Missing required surfaceFieldValue output: {dat_path}')
    if not math.isfinite(dt) or dt <= 0.0:
        raise ValueError(f'Invalid dt={dt} for surfaceFieldValue integration')
    totals = {c: 0.0 for c in cloud_names}
    t_prev = 0.0
    n_rows = 0
    for raw_line in dat_path.read_text().splitlines():
        line = raw_line.strip()
        if not line or line.startswith('#'):
            continue
        parts = line.split()
        if len(parts) != 1 + len(cloud_names):
            raise ValueError(
                f'Malformed surfaceFieldValue row in {dat_path}: expected {1 + len(cloud_names)} columns, got {len(parts)}: {line}'
            )
        t_cur = float(parts[0])
        if not math.isfinite(t_cur) or (n_rows == 0 and t_cur < 0.0) or (n_rows > 0 and t_cur <= t_prev):
            raise ValueError(f'Non-monotonic or non-finite timestamp in {dat_path}: t_cur={t_cur}, t_prev={t_prev}')
        step_dt = t_cur - t_prev
        t_prev = t_cur
        n_rows += 1
        for idx, cname in enumerate(cloud_names):
            flux_rate = float(parts[1 + idx])
            if not math.isfinite(flux_rate):
                raise ValueError(f'Non-finite flux_rate for {cname} at t={t_cur} in {dat_path}')
            totals[cname] += flux_rate * step_dt
    if n_rows == 0:
        raise ValueError(f'Empty surfaceFieldValue output in {dat_path}')
    return totals


def validate_carrier_case_eligibility(
    carrier_case: Path,
    time_step: int,
    allow_diagnostic_on_failed_seed_mesh: bool = False,
    allow_unextended_overflow_backflow_diagnostic: bool = False,
) -> dict[str, Any]:
    """Verify carrier run has not diverged, satisfies water mass closure, and has no terminal backflow.

    Requires an explicit numeric time_step (no lexical glob fallback) and refuses missing/non-finite
    or negative outlet flows on both overflow and underflow.
    """
    if not isinstance(time_step, int) or time_step <= 0:
        raise ValueError(f'Explicit positive integer time_step is required; got {time_step!r}')
    name = carrier_case.name
    if name in EXCLUDED_DIVERGED_CASES:
        raise ValueError(
            f'Refusing carrier case {name}: marked as numerically diverged in COORD.md '
            '(flow closure > 1e19, pressure ~ 1e35 Pa).'
        )
    integrals_path = carrier_case / f'hydraulic-integrals-t{time_step}.json'
    if not integrals_path.exists():
        raise ValueError(f'Carrier case {name} is missing explicit {integrals_path.name}; cannot run oil tracking.')
    integrals = json.loads(integrals_path.read_text())
    if int(integrals.get('time', -1)) != time_step:
        raise ValueError(f'Integrals time {integrals.get("time")} does not match requested time_step {time_step}')
    integrals_sha256 = file_sha256(integrals_path)

    prov = verify_carrier_case_provenance(carrier_case, time_step)
    if prov['failed_marker_present']:
        raise ValueError(f'Refusing carrier case {name}: failed-run marker present.')
    if not prov['input_manifest_verified'] or prov['mismatched_inputs']:
        raise ValueError(f'Refusing carrier case {name}: input-manifest verification failed ({prov["mismatched_inputs"]}).')
    if not prov['metrics_receipt_verified']:
        raise ValueError(f'Refusing carrier case {name}: native joilHydraulicMetrics receipt unverified.')

    if 'water_closure_fraction' not in integrals:
        raise ValueError(f'Missing water_closure_fraction in {integrals_path}')
    closure = float(integrals['water_closure_fraction'])
    if not math.isfinite(closure) or closure < 0.0 or closure > 0.001:
        raise ValueError(f'Carrier case {name} failed water mass closure ({closure:.6g} > 0.001).')

    patches = integrals.get('patches', {})
    backflow_exceeded = False
    for outlet_name in ('overflow', 'underflow'):
        if outlet_name not in patches:
            raise ValueError(f'Missing outlet patch {outlet_name} in {integrals_path}')
        p_dict = patches[outlet_name]
        if 'signed_flow_m3_s' not in p_dict or 'reverse_flow_m3_s' not in p_dict:
            raise ValueError(f'Missing signed_flow_m3_s or reverse_flow_m3_s on {outlet_name} in {integrals_path}')
        net_q = float(p_dict['signed_flow_m3_s'])
        rev_q = float(p_dict['reverse_flow_m3_s'])
        if not (math.isfinite(net_q) and math.isfinite(rev_q)) or net_q <= 0.0 or rev_q < 0.0:
            raise ValueError(
                f'Invalid outlet flow on {outlet_name} in {name}: signed_flow={net_q}, reverse_flow={rev_q}'
            )
        if rev_q / net_q > 0.01:
            backflow_exceeded = True
            if not (allow_unextended_overflow_backflow_diagnostic and outlet_name == 'overflow'):
                raise ValueError(
                    f'Carrier case {name} ({integrals_path.name}, sha256={integrals_sha256[:12]}) rejected: '
                    f'{outlet_name} terminal reverse flow {rev_q:.6g} m3/s exceeds 1% of net outward flow {net_q:.6g} m3/s.'
                )

    status_path = carrier_case / 'research-status.json'
    status_doc = json.loads(status_path.read_text()) if status_path.exists() else {}
    mesh_pass = bool(status_doc.get('mesh_pilot_pass', False))
    campaign_admitted = bool(status_doc.get('campaign_admission', False))
    if not (mesh_pass and campaign_admitted) and not allow_diagnostic_on_failed_seed_mesh:
        raise ValueError(
            f'Carrier case {name} has mesh_pilot_pass={mesh_pass}, campaign_admission={campaign_admitted}; '
            'pass allow_diagnostic_on_failed_seed_mesh=True only for explicitly unqualified diagnostic pilots.'
        )

    return {
        'integrals': integrals,
        'integrals_file': str(integrals_path.relative_to(ROOT)),
        'integrals_sha256': integrals_sha256,
        'provenance': prov,
        'research_status': status_doc,
        'is_qualified_carrier': bool(mesh_pass and campaign_admitted and not backflow_exceeded),
    }


def evaluate_swirl_box_case(case: Path, dt: float, end_time: float, omega: float = 280.0, commands: list | None = None) -> dict[str, Any]:
    times = sorted(
        (float(p.name), p)
        for p in case.iterdir()
        if p.is_dir() and re.fullmatch(r'[0-9.eE+-]+', p.name) and float(p.name) > 0
    )
    if not times:
        raise RuntimeError(f'No Lagrangian time output produced in {case}')
    t_final, p_final = times[-1]

    n_rings = 16
    r_rings = [0.0060 + (0.0160 - 0.0060) * (k + 0.5) / n_rings for k in range(n_rings)]
    r_core_capture = 0.0092

    weights = lognormal_bin_weights(STANDARD_BINS_UM, DEFAULT_LOGNORMAL_DSD['d50_um'], DEFAULT_LOGNORMAL_DSD['gsd'])
    cohorts = []
    reject_split = 0.02
    for d_um, w in zip(STANDARD_BINS_UM, weights):
        cname = f'cloud{int(d_um)}um'
        pts = parse_vectors(p_final / f'Lagrangian/{cname}/position')
        radii = [math.hypot(pt[0], pt[1]) for pt in pts]
        mean_delta_r = sum(r - r0 for r, r0 in zip(radii, r_rings)) / len(r_rings) if len(radii) == len(r_rings) else 0.0

        n_inj = len(r_rings)
        n_in_domain = len(pts)
        n_core_zone = sum(1 for r in radii if r <= r_core_capture)
        n_wall_zone = sum(1 for r in radii if r >= 0.022)
        n_outer_zone = sum(1 for r in radii if r_core_capture < r < 0.022)

        f_rem = n_in_domain / n_inj
        f_rej_exit = 0.0
        f_tr_exit = 0.0
        f_wall_hit = n_wall_zone / n_inj
        disc = 1.0 - (f_rej_exit + f_tr_exit + f_wall_hit + f_rem)

        f_core_zone = n_core_zone / n_inj
        f_outer_zone = n_outer_zone / n_inj
        e_red_core_proxy = reduced_efficiency(f_core_zone, reject_split)

        cohorts.append({
            'diameter_um': d_um,
            'inlet_mass_weight': w,
            'injected_count': n_inj,
            'in_domain_count': n_in_domain,
            'mean_radial_displacement_m': mean_delta_r,
            'remaining_in_domain_fraction': f_rem,
            'rejected_exit_fraction': f_rej_exit,
            'treated_exit_fraction': f_tr_exit,
            'wall_fraction': f_wall_hit,
            'discrepancy_fraction': disc,
            'in_domain_core_zone_fraction': f_core_zone,
            'in_domain_outer_zone_fraction': f_outer_zone,
            'unclamped_core_zone_reduced_index': e_red_core_proxy,
        })

    dr_10 = cohorts[3]['mean_radial_displacement_m']
    dr_100 = cohorts[-1]['mean_radial_displacement_m']
    monotonic_inward = all(
        cohorts[i + 1]['mean_radial_displacement_m'] < cohorts[i]['mean_radial_displacement_m']
        for i in range(len(cohorts) - 1)
    ) and (dr_100 < dr_10 < 0.0)
    inventory_closed = all(abs(c['discrepancy_fraction']) <= 1e-9 and c['remaining_in_domain_fraction'] == 1.0 for c in cohorts)
    tracer_oracle = verify_passive_tracer_split_oracle(cin_mg_l=200.0, r=reject_split)

    result = {
        'schema_version': 2,
        'kind': 'synthetic_swirl_box_monodisperse_cohort_displacement_verification',
        'qualification_tier': 'elementary_swirl_box_verification_not_full_liner_separator',
        'qualified_multiobjective_vector': None,
        'qualified_capacity_gain': None,
        'passed': bool(monotonic_inward and inventory_closed and tracer_oracle['passed']),
        'dt_s': dt,
        'end_time_s': end_time,
        'omega_rad_s': omega,
        'monotonic_inward_displacement_with_size': monotonic_inward,
        'inventory_closure_passed': inventory_closed,
        'arithmetic_passive_tracer_split_oracle': tracer_oracle,
        'cohorts': cohorts,
        'commands': commands or [],
    }
    write_immutable_json(case / 'result.json', result)
    return result


def run_swirl_separator_cohort_verification(
    dt: float = 2e-6,
    end_time: float = 0.0005,
    cpuset: str = '4,5,6,7',
) -> dict[str, Any]:
    """Run 10 monodisperse light-droplet cohorts in a 3D swirling-vortex box in OpenFOAM 14."""
    import numpy as np

    os.environ['JOIL_CPU_SET'] = cpuset
    case = ROOT / f'runs/gemini-oil-cohort-verification-dt{dt:g}-{uuid.uuid4().hex[:6]}'
    case.mkdir(parents=True, exist_ok=False)

    nx, ny, nz = 24, 24, 4
    write_dict(case, 'system/blockMeshDict', f'''convertToMeters 1;
vertices (
    (-0.025 -0.025 0.0) (0.025 -0.025 0.0) (0.025 0.025 0.0) (-0.025 0.025 0.0)
    (-0.025 -0.025 0.04) (0.025 -0.025 0.04) (0.025 0.025 0.04) (-0.025 0.025 0.04)
);
blocks (hex (0 1 2 3 4 5 6 7) ({nx} {ny} {nz}) simpleGrading (1 1 1));
edges ();
boundary (
    walls {{ type patch; faces ((0 1 5 4)(1 2 6 5)(2 3 7 6)(3 0 4 7)); }}
    overflow {{ type patch; faces ((0 3 2 1)); }}
    underflow {{ type patch; faces ((4 5 6 7)); }}
);
mergePatchPairs ();''')

    rho_w = 998.0
    rho_o = 850.0
    mu_w = 0.001
    nu_w = mu_w / rho_w
    omega = 280.0
    uz_drift = 2.0

    write_dict(case, 'constant/physicalProperties', f'viscosityModel constant; nu {nu_w:.16g};')
    write_dict(case, 'constant/momentumTransport', 'simulationType laminar;')
    write_dict(case, 'constant/g', 'dimensions [0 1 -2 0 0 0 0]; value (0 0 9.81);', 'uniformDimensionedVectorField')
    write_dict(case, '0/U', 'dimensions [0 1 -1 0 0 0 0]; internalField uniform (0 0 0); boundaryField {walls {type zeroGradient;} overflow {type zeroGradient;} underflow {type zeroGradient;}}', 'volVectorField')
    write_dict(case, '0/p', 'dimensions [0 2 -2 0 0 0 0]; internalField uniform 0; boundaryField {walls {type zeroGradient;} overflow {type zeroGradient;} underflow {type zeroGradient;}}', 'volScalarField')
    write_dict(case, 'system/controlDict', f'''application foamRun; solver functions; subSolver incompressibleFluid;
startFrom startTime; startTime 0; stopAt endTime; endTime {end_time:g}; deltaT {dt:g};
writeControl timeStep; writeInterval {max(1, round(end_time / dt))}; writeFormat ascii; writePrecision 15;
timePrecision 12; runTimeModifiable false;''')
    write_dict(case, 'system/fvSchemes', '''ddtSchemes {default Euler;} gradSchemes {default Gauss linear;}
divSchemes {default none; div(phi,U) Gauss linear;}
laplacianSchemes {default Gauss linear corrected;} interpolationSchemes {default linear;} snGradSchemes {default corrected;}''')
    write_dict(case, 'system/fvSolution', 'solvers {} PIMPLE {nCorrectors 0; momentumPredictor no; pRefCell 0; pRefValue 0;}')

    commands = []
    commands.append(runtime_run(case, ['blockMesh'], case / 'blockMesh.log', timeout=300))
    commands.append(runtime_run(case, ['foamPostProcess', '-func', 'writeCellCentres', '-time', '0'], case / 'writeCellCentres.log', timeout=300))

    centres = np.array(parse_vectors(case / '0/C'))
    n_cells = len(centres)
    u_cells = np.column_stack([-omega * centres[:, 1], omega * centres[:, 0], np.full(n_cells, uz_drift)])
    p_cells = 0.5 * (omega ** 2) * (centres[:, 0] ** 2 + centres[:, 1] ** 2)

    u_str = '\n'.join(f'({u[0]:.15g} {u[1]:.15g} {u[2]:.15g})' for u in u_cells)
    p_str = '\n'.join(f'{p:.15g}' for p in p_cells)
    write_dict(case, '0/U', f'''dimensions [0 1 -1 0 0 0 0];
internalField nonuniform List<vector> {n_cells} (
{u_str}
);
boundaryField {{ walls {{ type zeroGradient; }} overflow {{ type zeroGradient; }} underflow {{ type zeroGradient; }} }}''', 'volVectorField')
    write_dict(case, '0/p', f'''dimensions [0 2 -2 0 0 0 0];
internalField nonuniform List<scalar> {n_cells} (
{p_str}
);
boundaryField {{ walls {{ type zeroGradient; }} overflow {{ type zeroGradient; }} underflow {{ type zeroGradient; }} }}''', 'volScalarField')

    n_rings = 16
    r_rings = [0.0060 + (0.0160 - 0.0060) * (k + 0.5) / n_rings for k in range(n_rings)]
    pos_str = ' '.join(f'({r:.6f} 0 0.005)' for r in r_rings)

    fn_entries = []
    for d_um in STANDARD_BINS_UM:
        cname = f'cloud{int(d_um)}um'
        d_m = d_um * 1e-6
        write_dict(case, f'constant/Lagrangian/{cname}/physicalProperties', f'rhoByRhoc {rho_o / rho_w:.16g};')
        write_dict(case, f'constant/Lagrangian/{cname}/LagrangianModels', f'''injection {{
    type manualInjection;
    positions ({pos_str});
    time 0;
}}
drag {{ type SchillerNaumannDrag; }}
pressure {{ type pressureGradientForce; }}
addedMass {{ type constantCoefficientVirtualMass; Cvm 0.5; }}
buoyancy {{ type gravity; }}''')
        write_dict(case, f'0/Lagrangian/{cname}/U', f'''dimensions [0 1 -1 0 0 0 0];
internalField uniform (0 {omega * 0.011:.8g} {uz_drift:.8g});
boundaryField {{ ".*" {{ type escapeVelocity; }} }}
sources {{ injection {{ type uniformFixedValue; uniformValue (0 {omega * 0.011:.8g} {uz_drift:.8g}); }} }}''', 'LagrangianVectorField')
        write_dict(case, f'0/Lagrangian/{cname}/d', f'''dimensions [0 1 0 0 0 0 0];
internalField uniform {d_m:.15g};
boundaryField {{ ".*" {{ type calculated; }} }}
sources {{ injection {{ type uniformFixedValue; uniformValue {d_m:.15g}; }} }}''', 'LagrangianScalarField')
        write_dict(case, f'system/Lagrangian/{cname}/LagrangianSchemes', '''tracking linear;
ddtSchemes {default Euler;} SpSchemes {default none; Sp(D,U) implicit;}
averagingSchemes {} interpolationSchemes {default cellPoint; Uc cellPoint; grad(p) cellPoint;} accumulationSchemes {}''')
        write_dict(case, f'system/Lagrangian/{cname}/LagrangianSolution', 'maxTimeStepFraction 0.2; maxCellLengthScaleFraction 0.2; nCorrectors 1;')
        fn_entries.append(
            f'    {cname} {{ type kinematicParticleCloud; libs ("libLagrangianCloud.so"); cloud {cname}; executeControl timeStep; writeControl writeTime; }}\n'
            f'    pos_{cname} {{ type cloudPosition; libs ("libLagrangianCloudFunctionObjects.so"); cloud {cname}; }}'
        )

    fns_block = '\n'.join(fn_entries)
    write_dict(case, 'system/controlDict', f'''application foamRun; solver functions; subSolver incompressibleFluid;
startFrom startTime; startTime 0; stopAt endTime; endTime {end_time:g}; deltaT {dt:g};
writeControl timeStep; writeInterval {max(1, round(end_time / dt))}; writeFormat ascii; writePrecision 15;
timePrecision 12; runTimeModifiable false;
functions {{
{fns_block}
}}''')

    commands.append(runtime_run(case, ['foamRun'], case / 'foamRun.log', timeout=600))
    return evaluate_swirl_box_case(case, dt=dt, end_time=end_time, omega=omega, commands=commands)


def select_deterministic_full_liner_seeds(centres, u_cells) -> dict[str, Any]:
    """Select 32 deterministic interior cell centres in ct35-sst-extended-000:
    - 16 in the swirl chamber (z in [0.008, 0.065] m, r in [0.008, 0.026] m) for radial migration dr(d),
    - 8 near the extended overflow exit pipe (z in [-0.141, -0.118] m, r < 0.0042 m, Uz < -0.5 m/s) for native overflow cloudNumberFlux,
    - 8 near the extended underflow exit pipe (z in [1.615, 1.637] m, r < 0.0065 m, Uz > 3.5 m/s) for native underflow cloudNumberFlux.
    """
    import numpy as np

    r_all = np.hypot(centres[:, 0], centres[:, 1])
    z_all = centres[:, 2]
    uz_all = u_cells[:, 2]

    swirl_indices = []
    for k in range(16):
        r_target = 0.0085 + (0.0255 - 0.0085) * (k + 0.5) / 16.0
        z_target = 0.012 + 0.003 * (k % 8)
        theta_target = 2.0 * math.pi * k / 16.0
        x_t = r_target * math.cos(theta_target)
        y_t = r_target * math.sin(theta_target)
        d2 = (centres[:, 0] - x_t) ** 2 + (centres[:, 1] - y_t) ** 2 + (z_all - z_target) ** 2
        idx = int(np.argmin(d2))
        swirl_indices.append(idx)

    ov_mask = np.where((z_all >= -0.1415) & (z_all <= -0.125) & (r_all <= 0.0040) & (uz_all < -0.8))[0]
    if len(ov_mask) < 8:
        ov_mask = np.where((z_all < -0.120) & (r_all <= 0.0048) & (uz_all < -0.2))[0]
    ov_indices = [int(ov_mask[int(i * len(ov_mask) / 8)]) for i in range(8)]

    un_mask = np.where((z_all >= 1.615) & (z_all <= 1.637) & (r_all <= 0.0065) & (uz_all > 3.0))[0]
    if len(un_mask) < 8:
        un_mask = np.where((z_all > 1.600) & (r_all <= 0.0075) & (uz_all > 1.0))[0]
    un_indices = [int(un_mask[int(i * len(un_mask) / 8)]) for i in range(8)]

    all_indices = swirl_indices + ov_indices + un_indices
    seed_pts = [centres[i].tolist() for i in all_indices]
    swirl_pts = [centres[i].tolist() for i in swirl_indices]
    return {
        'injection_mode': 'deterministic_32_probe_seeds',
        'all_indices': all_indices,
        'swirl_indices': swirl_indices,
        'overflow_sentinel_indices': ov_indices,
        'underflow_sentinel_indices': un_indices,
        'all_positions_m': seed_pts,
        'swirl_positions_m': swirl_pts,
        'swirl_initial_radii_m': [float(math.hypot(p[0], p[1])) for p in swirl_pts],
    }


def select_flux_weighted_inlet_seeds(centres, u_cells) -> dict[str, Any]:
    """Select 32 deterministic interior cells in the two tangential inlet jets (16 in +y jet, 16 in -y jet)
    stratified by cumulative inward velocity flux |U_i| / sum(|U_j|) so each parcel represents an equal 1/32
    slice of the incoming tangential inlet flow (OIL-08 stratified flux weighting).
    """
    import numpy as np

    r_all = np.hypot(centres[:, 0], centres[:, 1])
    z_all = centres[:, 2]
    u_mag = np.linalg.norm(u_cells, axis=1)

    inlet_indices: list[int] = []
    for sign_y in (1.0, -1.0):
        mask = np.where(
            (z_all >= 0.0015)
            & (z_all <= 0.0115)
            & (r_all >= 0.0245)
            & (r_all <= 0.0342)
            & (centres[:, 1] * sign_y > 0.004)
            & (u_mag > 2.5)
        )[0]
        if len(mask) < 16:
            mask = np.where(
                (z_all >= 0.001) & (z_all <= 0.015) & (r_all >= 0.020) & (centres[:, 1] * sign_y > 0.0) & (u_mag > 1.0)
            )[0]
        # Sort by azimuthal angle and stratify by cumulative velocity magnitude (flux proxy)
        order = np.argsort(np.arctan2(centres[mask, 1], centres[mask, 0]))
        sorted_cells = mask[order]
        weights = u_mag[sorted_cells]
        cdf = np.cumsum(weights) / np.sum(weights)
        for k in range(16):
            q_target = (k + 0.5) / 16.0
            pos = int(np.searchsorted(cdf, q_target, side='left'))
            pos = max(0, min(len(sorted_cells) - 1, pos))
            inlet_indices.append(int(sorted_cells[pos]))

    seed_pts = [centres[i].tolist() for i in inlet_indices]
    swirl_indices = inlet_indices[:16]
    swirl_pts = [centres[i].tolist() for i in swirl_indices]
    return {
        'injection_mode': 'flux_weighted_inlet',
        'all_indices': inlet_indices,
        'swirl_indices': swirl_indices,
        'overflow_sentinel_indices': [],
        'underflow_sentinel_indices': [],
        'all_positions_m': seed_pts,
        'swirl_positions_m': swirl_pts,
        'swirl_initial_radii_m': [float(math.hypot(p[0], p[1])) for p in swirl_pts],
    }


def run_full_liner_frozen_carrier_pilot(
    carrier_case: Path,
    time_step: int = 600,
    dt: float = 5e-5,
    end_time: float = 0.01,
    cpuset: str = '4,5,6,7',
    seed_cache: dict[str, Any] | None = None,
    injection_mode: str = 'deterministic_32_probe_seeds',
    allow_unextended_overflow_backflow_diagnostic: bool = False,
) -> tuple[dict[str, Any], dict[str, Any]]:
    """Run a bounded diagnostic full-liner frozen-carrier 10-bin + passive-tracer Lagrangian pilot on ct35-sst-extended-000.

    Uses Foundation 14 native cloudNumberFlux (<cloud>:phiNumber) + surfaceFieldValue patch sums
    time-integrated with actual dt for terminal overflow/underflow counts, plus stickVelocity on wall
    and native Lagrangian/<cloud>/face inspection for wall vs remaining_in_domain accounting.
    """
    import numpy as np

    elig = validate_carrier_case_eligibility(
        carrier_case,
        time_step=time_step,
        allow_diagnostic_on_failed_seed_mesh=True,
        allow_unextended_overflow_backflow_diagnostic=allow_unextended_overflow_backflow_diagnostic,
    )
    os.environ['JOIL_CPU_SET'] = cpuset
    mode_tag = 'inlet' if injection_mode == 'flux_weighted_inlet' else 'pilot'
    case = ROOT / f'runs/gemini-ct35-full-liner-{mode_tag}-t{time_step}-dt{dt:g}-{uuid.uuid4().hex[:6]}'
    case.mkdir(parents=True, exist_ok=False)

    # Copy polyMesh and frozen carrier fields (U, p, phi) from time_step to 0/ and bind launch-time manifest
    shutil.copytree(carrier_case / 'constant/polyMesh', case / 'constant/polyMesh')
    (case / '0').mkdir(parents=True, exist_ok=True)
    for fname in ('U', 'p', 'phi'):
        shutil.copy2(carrier_case / str(time_step) / fname, case / '0' / fname)
    shutil.copy2(carrier_case / 'constant/physicalProperties', case / 'constant/physicalProperties')
    launch_parent_hashes = {
        'constant/polyMesh/boundary': file_sha256(case / 'constant/polyMesh/boundary'),
        'constant/polyMesh/points': file_sha256(case / 'constant/polyMesh/points'),
        'constant/polyMesh/faces': file_sha256(case / 'constant/polyMesh/faces'),
        'constant/polyMesh/owner': file_sha256(case / 'constant/polyMesh/owner'),
        'constant/polyMesh/neighbour': file_sha256(case / 'constant/polyMesh/neighbour'),
        'constant/physicalProperties': file_sha256(case / 'constant/physicalProperties'),
        f'{time_step}/U': file_sha256(case / '0/U'),
        f'{time_step}/p': file_sha256(case / '0/p'),
        f'{time_step}/phi': file_sha256(case / '0/phi'),
    }
    write_immutable_json(case / 'parent-carrier-manifest.json', launch_parent_hashes)
    write_dict(case, 'constant/momentumTransport', 'simulationType laminar;')
    write_dict(case, 'constant/g', 'dimensions [0 1 -2 0 0 0 0]; value (0 0 0);', 'uniformDimensionedVectorField')

    write_dict(case, 'system/fvSchemes', '''ddtSchemes {default Euler;}
gradSchemes {default cellLimited Gauss linear 1;}
divSchemes {default none; div(phi,U) bounded Gauss upwind; div((nuEff*dev2(T(grad(U))))) Gauss linear;}
laplacianSchemes {default Gauss linear limited .5;}
interpolationSchemes {default linear;} snGradSchemes {default limited .5;}''')
    write_dict(case, 'system/fvSolution', '''solvers {}
PIMPLE {nOuterCorrectors 0; nCorrectors 0; momentumPredictor no; pRefCell 0; pRefValue 0;}''')

    commands = []
    if seed_cache is None:
        write_dict(case, 'system/controlDict', f'''application foamRun; solver functions; subSolver incompressibleFluid;
startFrom startTime; startTime 0; stopAt endTime; endTime {dt:g}; deltaT {dt:g};
writeControl timeStep; writeInterval 1; writeFormat ascii; writePrecision 12; timePrecision 12; runTimeModifiable false;''')
        commands.append(runtime_run(case, ['foamPostProcess', '-func', 'writeCellCentres', '-time', '0'], case / 'writeCellCentres.log', timeout=300))
        centres = np.array(parse_vectors(case / '0/C'))
        u_cells = np.array(parse_vectors(case / '0/U'))
        if injection_mode == 'flux_weighted_inlet':
            seed_cache = select_flux_weighted_inlet_seeds(centres, u_cells)
        else:
            seed_cache = select_deterministic_full_liner_seeds(centres, u_cells)
        # Remove 0/C after extracting seed coordinates to save disk space
        (case / '0/C').unlink(missing_ok=True)


    pos_all_str = ' '.join(f'({p[0]:.8f} {p[1]:.8f} {p[2]:.8f})' for p in seed_cache['all_positions_m'])
    pos_swirl_str = ' '.join(f'({p[0]:.8f} {p[1]:.8f} {p[2]:.8f})' for p in seed_cache['swirl_positions_m'])

    rho_w = 998.0
    rho_o = 850.0
    n_steps = max(1, round(end_time / dt))

    cloud_specs = [
        (f'cloud{int(d_um)}um', d_um, rho_o / rho_w, pos_all_str, len(seed_cache['all_positions_m']))
        for d_um in STANDARD_BINS_UM
    ] + [
        ('cloud_tracer', 1.0, 1.0, pos_all_str, len(seed_cache['all_positions_m'])),
        ('swirl10um', 10.0, rho_o / rho_w, pos_swirl_str, len(seed_cache['swirl_positions_m'])),
        ('swirl40um', 40.0, rho_o / rho_w, pos_swirl_str, len(seed_cache['swirl_positions_m'])),
        ('swirl100um', 100.0, rho_o / rho_w, pos_swirl_str, len(seed_cache['swirl_positions_m'])),
    ]

    fn_entries = []
    phi_field_names = []
    for cname, d_um, rho_ratio, pos_str, _ in cloud_specs:
        d_m = d_um * 1e-6
        phi_field_names.append(f'{cname}:phiNumber')
        write_dict(case, f'constant/Lagrangian/{cname}/physicalProperties', f'rhoByRhoc {rho_ratio:.16g};')
        write_dict(case, f'constant/Lagrangian/{cname}/LagrangianModels', f'''injection {{
    type manualInjection;
    positions ({pos_str});
    time 0;
}}
drag {{ type SchillerNaumannDrag; }}
pressure {{ type pressureGradientForce; }}
addedMass {{ type constantCoefficientVirtualMass; Cvm 0.5; }}
buoyancy {{ type gravity; }}''')
        # overflow and underflow are terminal removal patches (escapeVelocity -> cloudNumberFlux records +1/dt);
        # wall uses stickVelocity so wall hits stick on wall faces [908240, 970301) and are never counted as captured;
        # inlet1/inlet2 use reboundVelocity.
        write_dict(case, f'0/Lagrangian/{cname}/U', '''dimensions [0 1 -1 0 0 0 0];
internalField uniform (0 0 0);
boundaryField {
    overflow { type escapeVelocity; }
    underflow { type escapeVelocity; }
    wall { type stickVelocity; }
    inlet1 { type reboundVelocity; e 1.0; mu 0.0; }
    inlet2 { type reboundVelocity; e 1.0; mu 0.0; }
}
sources { injection { type carrier; } }''', 'LagrangianVectorField')
        write_dict(case, f'0/Lagrangian/{cname}/d', f'''dimensions [0 1 0 0 0 0 0];
internalField uniform {d_m:.15g};
boundaryField {{ ".*" {{ type calculated; }} }}
sources {{ injection {{ type uniformFixedValue; uniformValue {d_m:.15g}; }} }}''', 'LagrangianScalarField')
        write_dict(case, f'system/Lagrangian/{cname}/LagrangianSchemes', '''tracking linear;
ddtSchemes {default Euler;} SpSchemes {default none; Sp(D,U) implicit;}
averagingSchemes {} interpolationSchemes {default cellPoint; Uc cellPoint; grad(p) cellPoint;} accumulationSchemes {}''')
        write_dict(case, f'system/Lagrangian/{cname}/LagrangianSolution', 'maxTimeStepFraction 0.25; maxCellLengthScaleFraction 0.25; nCorrectors 1;')
        fn_entries.append(
            f'    {cname} {{ type kinematicParticleCloud; libs ("libLagrangianCloud.so"); cloud {cname}; executeControl timeStep; writeControl writeTime; }}\n'
            f'    pos_{cname} {{ type cloudPosition; libs ("libLagrangianCloudFunctionObjects.so"); cloud {cname}; }}\n'
            f'    flux_{cname} {{ type cloudNumberFlux; libs ("libLagrangianCloudFunctionObjects.so"); cloud {cname}; executeControl timeStep; writeControl writeTime; }}'
        )

    fields_str = ' '.join(phi_field_names)
    fn_entries.append(f'''    overflowNumberFluxSum {{
        type surfaceFieldValue;
        libs ("libfieldFunctionObjects.so");
        log false;
        executeControl timeStep;
        writeControl timeStep;
        writeFields false;
        patch overflow;
        operation sum;
        fields ({fields_str});
    }}
    underflowNumberFluxSum {{
        type surfaceFieldValue;
        libs ("libfieldFunctionObjects.so");
        log false;
        executeControl timeStep;
        writeControl timeStep;
        writeFields false;
        patch underflow;
        operation sum;
        fields ({fields_str});
    }}''')

    fns_block = '\n'.join(fn_entries)
    write_dict(case, 'system/controlDict', f'''application foamRun; solver functions; subSolver incompressibleFluid;
startFrom startTime; startTime 0; stopAt endTime; endTime {end_time:g}; deltaT {dt:g};
writeControl timeStep; writeInterval {n_steps}; writeFormat ascii; writePrecision 12;
timePrecision 12; runTimeModifiable false;
functions {{
{fns_block}
}}''')

    run_rec = runtime_run(case, ['foamRun'], case / 'foamRun.log', timeout=900)
    commands.append(run_rec)

    times = sorted(
        (float(p.name), p)
        for p in case.iterdir()
        if p.is_dir() and re.fullmatch(r'[0-9.eE+-]+', p.name) and float(p.name) > 0
    )
    if not times:
        raise RuntimeError(f'No Lagrangian time output produced in full-liner pilot {case}')
    t_final, p_final = times[-1]

    # Remove duplicated 0/{U,p,phi} and final-time Eulerian fields after run to keep disk usage tiny,
    # retaining all Lagrangian outputs, logs, receipts, and postProcessing surfaceFieldValue files.
    for euler_f in ('U', 'p', 'phi'):
        (case / '0' / euler_f).unlink(missing_ok=True)
        (p_final / euler_f).unlink(missing_ok=True)
    shutil.rmtree(case / 'constant/polyMesh', ignore_errors=True)

    result = evaluate_full_liner_pilot_case(
        case=case,
        carrier_case=carrier_case,
        time_step=time_step,
        dt=dt,
        end_time=end_time,
        commands=commands,
        injection_mode=injection_mode,
        allow_unextended_overflow_backflow_diagnostic=allow_unextended_overflow_backflow_diagnostic,
    )
    return result, seed_cache


def evaluate_full_liner_pilot_case(
    case: Path,
    carrier_case: Path,
    time_step: int,
    dt: float,
    end_time: float,
    commands: list[dict[str, Any]] | None = None,
    injection_mode: str = 'deterministic_32_probe_seeds',
    allow_unextended_overflow_backflow_diagnostic: bool = False,
) -> dict[str, Any]:
    elig = validate_carrier_case_eligibility(
        carrier_case,
        time_step=time_step,
        allow_diagnostic_on_failed_seed_mesh=True,
        allow_unextended_overflow_backflow_diagnostic=allow_unextended_overflow_backflow_diagnostic,
    )
    boundary_meta = parse_poly_mesh_boundary(carrier_case / 'constant/polyMesh/boundary')
    wall_start = boundary_meta['patches']['wall']['startFace']
    wall_end = boundary_meta['patches']['wall']['endFace']
    n_internal_faces = boundary_meta['nInternalFaces']

    parent_field_hashes = {
        'constant/polyMesh/boundary': boundary_meta['boundary_sha256'],
        'constant/polyMesh/points': file_sha256(carrier_case / 'constant/polyMesh/points'),
        'constant/polyMesh/faces': file_sha256(carrier_case / 'constant/polyMesh/faces'),
        'constant/polyMesh/owner': file_sha256(carrier_case / 'constant/polyMesh/owner'),
        'constant/polyMesh/neighbour': file_sha256(carrier_case / 'constant/polyMesh/neighbour'),
        'constant/physicalProperties': file_sha256(carrier_case / 'constant/physicalProperties'),
        f'{time_step}/U': file_sha256(carrier_case / str(time_step) / 'U'),
        f'{time_step}/p': file_sha256(carrier_case / str(time_step) / 'p'),
        f'{time_step}/phi': file_sha256(carrier_case / str(time_step) / 'phi'),
    }
    pilot_manifest_path = case / 'parent-carrier-manifest.json'
    if not pilot_manifest_path.exists():
        write_immutable_json(pilot_manifest_path, parent_field_hashes)

    times = sorted(
        (float(p.name), p)
        for p in case.iterdir()
        if p.is_dir() and re.fullmatch(r'[0-9.eE+-]+', p.name) and float(p.name) > 0
    )
    if not times:
        raise RuntimeError(f'No Lagrangian time output produced in full-liner pilot {case}')
    t_final, p_final = times[-1]
    n_steps = max(1, round(end_time / dt))

    if commands is None:
        commands = []
        for rec_file in sorted(case.glob('*.receipt.json')):
            commands.append(json.loads(rec_file.read_text()))
    foam_rec_path = case / 'foamRun.log.receipt.json'
    wall_sec = (
        json.loads(foam_rec_path.read_text())['wall_seconds']
        if foam_rec_path.exists()
        else (commands[-1]['wall_seconds'] if commands else 0.0)
    )

    all_mod_txt = (case / 'constant/Lagrangian/cloud3um/LagrangianModels').read_text()
    all_pos_tuples = re.findall(r'\(\s*([-+0-9.eE]+)\s+([-+0-9.eE]+)\s+([-+0-9.eE]+)\s*\)', all_mod_txt)
    all_pos_xyz = [[float(x), float(y), float(z)] for x, y, z in all_pos_tuples]
    n_inj_all = len(all_pos_xyz)
    n_inj_swirl_group = sum(1 for _, _, z in all_pos_xyz if 0.0 < z < 0.5)
    n_inj_overflow_group = sum(1 for _, _, z in all_pos_xyz if z < 0.0)
    n_inj_underflow_group = sum(1 for _, _, z in all_pos_xyz if z >= 0.5)

    swirl_mod_txt = (case / 'constant/Lagrangian/swirl10um/LagrangianModels').read_text()
    swirl_pos_tuples = re.findall(r'\(\s*([-+0-9.eE]+)\s+([-+0-9.eE]+)\s+([-+0-9.eE]+)\s*\)', swirl_mod_txt)
    r0_swirl = [math.hypot(float(x), float(y)) for x, y, _ in swirl_pos_tuples]

    cloud_names_all = [f'cloud{int(d_um)}um' for d_um in STANDARD_BINS_UM] + [
        'cloud_tracer',
        'swirl10um',
        'swirl40um',
        'swirl100um',
    ]
    ov_sums = parse_surface_field_value_sums(
        case / 'postProcessing/overflowNumberFluxSum/0/surfaceFieldValue.dat',
        cloud_names_all,
        dt,
    )
    un_sums = parse_surface_field_value_sums(
        case / 'postProcessing/underflowNumberFluxSum/0/surfaceFieldValue.dat',
        cloud_names_all,
        dt,
    )

    tr_pts = parse_vectors(p_final / 'Lagrangian/cloud_tracer/position')
    tr_faces = parse_labels(p_final / 'Lagrangian/cloud_tracer/face')
    if len(tr_pts) != len(tr_faces):
        raise ValueError(f'Count mismatch between position ({len(tr_pts)}) and face ({len(tr_faces)}) in cloud_tracer')
    tr_swirl_radii = [math.hypot(p[0], p[1]) for p in tr_pts if 0.0 < p[2] < 0.5]

    weights = lognormal_bin_weights(STANDARD_BINS_UM, DEFAULT_LOGNORMAL_DSD['d50_um'], DEFAULT_LOGNORMAL_DSD['gsd'])
    cohorts = []
    for d_um, w in zip(STANDARD_BINS_UM, weights):
        cname = f'cloud{int(d_um)}um'
        n_inj = n_inj_all
        faces = parse_labels(p_final / f'Lagrangian/{cname}/face')
        pts = parse_vectors(p_final / f'Lagrangian/{cname}/position')
        if len(faces) != len(pts):
            raise ValueError(f'Count mismatch between position ({len(pts)}) and face ({len(faces)}) in {cname}')
        swirl_radii = [math.hypot(p[0], p[1]) for p in pts if 0.0 < p[2] < 0.5]
        mean_dr_vs_tracer = (
            sum(r - rt for r, rt in zip(swirl_radii, tr_swirl_radii)) / len(tr_swirl_radii)
            if len(swirl_radii) == len(tr_swirl_radii) and tr_swirl_radii
            else None
        )
        n_active = len(faces)
        n_wall_stuck = sum(1 for f in faces if wall_start <= f < wall_end)
        n_rem_internal = sum(1 for f in faces if f < n_internal_faces)
        n_other_patch_active = n_active - n_wall_stuck - n_rem_internal
        n_ov_escaped, ov_res = checked_near_integer_flux(ov_sums[cname])
        n_un_escaped, un_res = checked_near_integer_flux(un_sums[cname])
        n_accounted = n_ov_escaped + n_un_escaped + n_wall_stuck + n_rem_internal + n_other_patch_active
        n_disc = n_inj - n_accounted

        cohorts.append({
            'diameter_um': d_um,
            'inlet_mass_weight_reference_only': w,
            'injected_count': n_inj,
            'probe_subgroups_injected': {
                'swirl_chamber_internal_probe_count': n_inj_swirl_group,
                'overflow_pipe_near_terminal_probe_count': n_inj_overflow_group,
                'underflow_pipe_near_terminal_probe_count': n_inj_underflow_group,
            },
            'overflow_terminal_escaped_count': n_ov_escaped,
            'overflow_cloud_number_flux_integral': ov_sums[cname],
            'overflow_flux_integer_residual': ov_res,
            'underflow_terminal_escaped_count': n_un_escaped,
            'underflow_cloud_number_flux_integral': un_sums[cname],
            'underflow_flux_integer_residual': un_res,
            'wall_stuck_count': n_wall_stuck,
            'remaining_in_domain_count': n_rem_internal,
            'other_patch_active_count': n_other_patch_active,
            'discrepancy_count': n_disc,
            'rejected_exit_fraction': n_ov_escaped / n_inj,
            'treated_exit_fraction': n_un_escaped / n_inj,
            'wall_fraction': n_wall_stuck / n_inj,
            'remaining_in_domain_fraction': n_rem_internal / n_inj,
            'discrepancy_fraction': n_disc / n_inj,
            'mean_swirl_radial_displacement_vs_reference_1um_m': mean_dr_vs_tracer,
        })

    swirl_drift = {}
    for cname, d_um in [('swirl10um', 10.0), ('swirl40um', 40.0), ('swirl100um', 100.0)]:
        pts = parse_vectors(p_final / f'Lagrangian/{cname}/position')
        faces = parse_labels(p_final / f'Lagrangian/{cname}/face')
        if len(pts) != len(faces):
            raise ValueError(f'Count mismatch between position and face in {cname}')
        radii = [math.hypot(p[0], p[1]) for p in pts]
        mean_dr = sum(r - r0 for r, r0 in zip(radii, r0_swirl)) / len(r0_swirl) if len(radii) == len(r0_swirl) else None
        mean_dr_vs_tr = (
            sum(r - rt for r, rt in zip(radii, tr_swirl_radii)) / len(tr_swirl_radii)
            if len(radii) == len(tr_swirl_radii) and tr_swirl_radii
            else None
        )
        swirl_drift[cname] = {
            'diameter_um': d_um,
            'count': len(pts),
            'wall_stuck_count': sum(1 for f in faces if wall_start <= f < wall_end),
            'mean_radial_displacement_m': mean_dr,
            'mean_radial_displacement_vs_reference_1um_m': mean_dr_vs_tr,
        }

    tr_wall = sum(1 for f in tr_faces if wall_start <= f < wall_end)
    tr_rem = sum(1 for f in tr_faces if f < n_internal_faces)
    tr_ov, tr_ov_res = checked_near_integer_flux(ov_sums['cloud_tracer'])
    tr_un, tr_un_res = checked_near_integer_flux(un_sums['cloud_tracer'])
    tr_inj = n_inj_all

    inventory_closed = all(c['discrepancy_count'] == 0 for c in cohorts) and (tr_inj == tr_ov + tr_un + tr_wall + tr_rem)
    monotonic_10bin_inward_vs_ref = all(
        cohorts[i]['mean_swirl_radial_displacement_vs_reference_1um_m'] is not None
        and cohorts[i + 1]['mean_swirl_radial_displacement_vs_reference_1um_m'] is not None
        and cohorts[i + 1]['mean_swirl_radial_displacement_vs_reference_1um_m']
        < cohorts[i]['mean_swirl_radial_displacement_vs_reference_1um_m']
        < 0.0
        for i in range(len(cohorts) - 1)
    )

    result = {
        'schema_version': 3,
        'kind': (
            'diagnostic_flux_weighted_inlet_cohort_probe_not_full_residence_grade_curve'
            if injection_mode == 'flux_weighted_inlet'
            else 'diagnostic_full_liner_capability_probe_not_inlet_dsd_grade_curve'
        ),
        'injection_mode': injection_mode,
        'qualification_tier': 'single_case_capability_probe_unqualified_on_failed_seed_mesh_and_f1_sst',
        'is_qualified_grade_curve': False,
        'qualified_multiobjective_vector': None,
        'qualified_capacity_gain': None,
        'cout_mg_l': None,
        'capture_efficiency': None,
        'carrier_case': carrier_case.name,
        'carrier_time_step': time_step,
        'carrier_eligibility': {
            'integrals_file': elig['integrals_file'],
            'integrals_sha256': elig['integrals_sha256'],
            'provenance': elig['provenance'],
            'is_qualified_carrier': elig['is_qualified_carrier'],
        },
        'parent_carrier_manifest_file': str(pilot_manifest_path.relative_to(ROOT)),
        'parent_carrier_manifest_sha256': file_sha256(pilot_manifest_path),
        'parent_carrier_field_hashes': parent_field_hashes,
        'boundary_face_metadata': boundary_meta,
        'dt_s': dt,
        'end_time_s': t_final,
        'n_steps': n_steps,
        'foamRun_wall_seconds': wall_sec,
        'estimated_wall_seconds_per_0.1s_physical': wall_sec * (0.1 / end_time),
        'native_event_accounting_method': (
            'Foundation 14 cloudNumberFlux (<cloud>:phiNumber) + surfaceFieldValue patch sum '
            'time-integrated over actual dt with explicit near-integer tolerance check (<= 1e-6) '
            'on overflow/underflow (escapeVelocity), plus dynamic polyMesh/boundary face-range '
            'classification for stickVelocity wall hits and internal faces.'
        ),
        'inventory_closure_passed': inventory_closed,
        'relative_oil_vs_reference_1um_inward_slip_observed': monotonic_10bin_inward_vs_ref,
        'passed': bool(inventory_closed),
        'swirl_chamber_radial_drift_cohorts': swirl_drift,
        'neutral_density_1um_reference_cloud': {
            'kind': 'neutral_density_1um_lagrangian_reference_not_zero_inertia_scalar_oracle',
            'injected_count': tr_inj,
            'overflow_terminal_escaped_count': tr_ov,
            'overflow_flux_integer_residual': tr_ov_res,
            'underflow_terminal_escaped_count': tr_un,
            'underflow_flux_integer_residual': tr_un_res,
            'wall_stuck_count': tr_wall,
            'remaining_in_domain_count': tr_rem,
            'discrepancy_count': tr_inj - (tr_ov + tr_un + tr_wall + tr_rem),
        },
        'cohorts': cohorts,
        'commands': commands,
    }
    rev_v2_path = case / 'result-review-v2.json'
    res_path = write_immutable_json(rev_v2_path, result)
    result['result_file'] = str(res_path.relative_to(ROOT))
    result['result_sha256'] = file_sha256(res_path)
    return result


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--carrier-case', type=Path, default=None, help='Path to converged OpenFOAM carrier run')
    ap.add_argument('--time-step', type=int, default=600, help='Required explicit numeric saved time for carrier-case')
    ap.add_argument('--run-full-liner-pilot', action='store_true', help='Run bounded 0.01s full-liner frozen-carrier pilot with dt sensitivity')
    ap.add_argument('--run-inlet-cohort-pilot', action='store_true', help='Run launch-time parent-bound flux-weighted inlet 10-bin cohort pilot')
    ap.add_argument('--recompute-full-liner-pilot', action='store_true', help='Re-evaluate existing full-liner pilot directories into new immutable revision')
    ap.add_argument('--verify-swirl-separator', action='store_true', help='Run self-contained 10-bin swirling box verification in OpenFOAM 14')
    ap.add_argument('--recompute-existing', action='store_true', help='Re-evaluate existing gemini-oil-cohort-verification-* directories into new immutable revision')
    ap.add_argument('--dt', type=float, default=2e-6)
    ap.add_argument('--pilot-dt', type=float, default=5e-5)
    ap.add_argument('--pilot-end-time', type=float, default=0.01)
    ap.add_argument('--cpuset', type=str, default='4,5,6,7')
    args = ap.parse_args()

    if args.run_inlet_cohort_pilot:
        c_case = (args.carrier_case or (ROOT / 'runs/ct35-sst-extended-000')).resolve()
        existing_inlet_dirs = sorted(
            d for d in (ROOT / 'runs').glob(f'gemini-ct35-full-liner-inlet-t{args.time_step}-dt{args.pilot_dt:g}-*')
            if (d / f'{args.pilot_end_time:g}' / 'Lagrangian').is_dir()
        )
        if existing_inlet_dirs:
            inlet_res = evaluate_full_liner_pilot_case(
                existing_inlet_dirs[-1],
                c_case,
                args.time_step,
                args.pilot_dt,
                args.pilot_end_time,
                injection_mode='flux_weighted_inlet',
            )
        else:
            inlet_res, _ = run_full_liner_frozen_carrier_pilot(
                c_case,
                time_step=args.time_step,
                dt=args.pilot_dt,
                end_time=args.pilot_end_time,
                cpuset=args.cpuset,
                seed_cache=None,
                injection_mode='flux_weighted_inlet',
            )
        out_inlet = write_immutable_json(ROOT / 'runs/gemini-ct35-flux-weighted-inlet-pilot.json', inlet_res)
        print(f'Wrote immutable flux-weighted inlet pilot summary to {out_inlet}')
        print(json.dumps(inlet_res, indent=2))
        if not inlet_res['passed']:
            raise SystemExit(1)
        return


    if args.run_full_liner_pilot or args.recompute_full_liner_pilot:
        c_case = (args.carrier_case or (ROOT / 'runs/ct35-sst-extended-000')).resolve()
        if args.recompute_full_liner_pilot:
            coarse_dirs = sorted((ROOT / 'runs').glob(f'gemini-ct35-full-liner-pilot-t{args.time_step}-dt{args.pilot_dt:g}-*'))
            fine_dirs = sorted((ROOT / 'runs').glob(f'gemini-ct35-full-liner-pilot-t{args.time_step}-dt{args.pilot_dt * 0.5:g}-*'))
            if not coarse_dirs or not fine_dirs:
                raise RuntimeError('No existing full-liner pilot directories found')
            pilot_coarse = evaluate_full_liner_pilot_case(
                coarse_dirs[-1], c_case, args.time_step, args.pilot_dt, args.pilot_end_time
            )
            pilot_fine = evaluate_full_liner_pilot_case(
                fine_dirs[-1], c_case, args.time_step, args.pilot_dt * 0.5, args.pilot_end_time
            )
        else:
            pilot_coarse, seed_cache = run_full_liner_frozen_carrier_pilot(
                c_case,
                time_step=args.time_step,
                dt=args.pilot_dt,
                end_time=args.pilot_end_time,
                cpuset=args.cpuset,
                seed_cache=None,
            )
            pilot_fine, _ = run_full_liner_frozen_carrier_pilot(
                c_case,
                time_step=args.time_step,
                dt=args.pilot_dt * 0.5,
                end_time=args.pilot_end_time,
                cpuset=args.cpuset,
                seed_cache=seed_cache,
            )
        dr_diffs = {
            k: abs(
                pilot_coarse['swirl_chamber_radial_drift_cohorts'][k]['mean_radial_displacement_m']
                - pilot_fine['swirl_chamber_radial_drift_cohorts'][k]['mean_radial_displacement_m']
            )
            for k in ('swirl10um', 'swirl40um', 'swirl100um')
        }
        dr_vs_tr_diffs = {
            k: abs(
                pilot_coarse['swirl_chamber_radial_drift_cohorts'][k]['mean_radial_displacement_vs_reference_1um_m']
                - pilot_fine['swirl_chamber_radial_drift_cohorts'][k]['mean_radial_displacement_vs_reference_1um_m']
            )
            for k in ('swirl10um', 'swirl40um', 'swirl100um')
        }
        pilot_summary = {
            'schema_version': 2,
            'kind': 'diagnostic_full_liner_frozen_carrier_pilot_and_dt_sensitivity',
            'qualification_tier': 'diagnostic_unqualified_on_failed_seed_mesh_and_f1_sst',
            'is_qualified_grade_curve': False,
            'qualified_multiobjective_vector': None,
            'qualified_capacity_gain': None,
            'carrier_case': c_case.name,
            'carrier_time_step': args.time_step,
            'passed': bool(pilot_coarse['passed'] and pilot_fine['passed']),
            'dt_coarse_s': args.pilot_dt,
            'dt_fine_s': args.pilot_dt * 0.5,
            'end_time_s': args.pilot_end_time,
            'coarse_foamRun_wall_seconds': pilot_coarse['foamRun_wall_seconds'],
            'fine_foamRun_wall_seconds': pilot_fine['foamRun_wall_seconds'],
            'swirl_radial_displacement_dt_abs_diff_m': dr_diffs,
            'swirl_radial_displacement_vs_tracer_dt_abs_diff_m': dr_vs_tr_diffs,
            'max_swirl_radial_displacement_dt_abs_diff_m': max(dr_diffs.values()),
            'max_swirl_radial_displacement_vs_tracer_dt_abs_diff_m': max(dr_vs_tr_diffs.values()),
            'pilot_coarse': pilot_coarse,
            'pilot_fine': pilot_fine,
        }
        out_pilot = write_immutable_json(ROOT / 'runs/gemini-ct35-full-liner-oil-pilot-summary.json', pilot_summary)
        print(f'Wrote immutable full-liner pilot summary to {out_pilot}')
        print(json.dumps(pilot_summary, indent=2))
        if not pilot_summary['passed']:
            raise SystemExit(1)
        return

    if args.carrier_case is not None and not args.verify_swirl_separator and not args.recompute_existing:
        elig = validate_carrier_case_eligibility(args.carrier_case.resolve(), time_step=args.time_step)
        print(json.dumps(elig, indent=2))
        return

    if args.recompute_existing:
        coarse_dirs = sorted((ROOT / 'runs').glob(f'gemini-oil-cohort-verification-dt{args.dt:g}-*'))
        fine_dirs = sorted((ROOT / 'runs').glob(f'gemini-oil-cohort-verification-dt{args.dt * 0.5:g}-*'))
        if not coarse_dirs or not fine_dirs:
            raise RuntimeError('No existing cohort verification directories found')
        res_coarse = evaluate_swirl_box_case(coarse_dirs[-1], dt=args.dt, end_time=0.0005)
        res_fine = evaluate_swirl_box_case(fine_dirs[-1], dt=args.dt * 0.5, end_time=0.0005)
    else:
        res_coarse = run_swirl_separator_cohort_verification(dt=args.dt, cpuset=args.cpuset)
        res_fine = run_swirl_separator_cohort_verification(dt=args.dt * 0.5, cpuset=args.cpuset)

    max_dt_diff = max(
        abs(c1['mean_radial_displacement_m'] - c2['mean_radial_displacement_m'])
        for c1, c2 in zip(res_coarse['cohorts'], res_fine['cohorts'])
    )
    summary = {
        'schema_version': 2,
        'kind': 'synthetic_swirl_box_cohort_and_dt_sensitivity_verification',
        'qualification_tier': 'elementary_swirl_box_verification_not_full_liner_separator',
        'qualified_multiobjective_vector': None,
        'qualified_capacity_gain': None,
        'passed': bool(res_coarse['passed'] and res_fine['passed'] and max_dt_diff < 5e-5),
        'dt_coarse_s': args.dt,
        'dt_fine_s': args.dt * 0.5,
        'max_mean_radial_displacement_dt_diff_m': max_dt_diff,
        'arithmetic_passive_tracer_oracle_passed': res_coarse['arithmetic_passive_tracer_split_oracle']['passed'],
        'inventory_closure_passed': res_coarse['inventory_closure_passed'],
        'monotonic_inward_migration_passed': res_coarse['monotonic_inward_displacement_with_size'],
        'cohorts_dt_fine': res_fine['cohorts'],
    }
    out_path = write_immutable_json(ROOT / 'runs/gemini-oil-grade-verification.json', summary)
    print(f'Wrote immutable verification summary to {out_path}')
    print(json.dumps(summary, indent=2))
    if not summary['passed']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
