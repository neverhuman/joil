#!/usr/bin/env python3
"""OIL-03: Native Foundation 14 light-droplet inward migration in solid-body rotation."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'physics/openfoam/environment'))
from runtime import run as runtime_run


def foam_file(name, text, cls='dictionary'):
    return f'FoamFile {{ format ascii; class {cls}; object {name}; }}\n' + text + '\n'


def write_dict(case, relpath, text, cls='dictionary'):
    p = case / relpath
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(foam_file(p.name, text, cls))


def run_container(case, argv, log, cpuset='4,5,6,7', timeout=600):
    os.environ['JOIL_CPU_SET'] = cpuset
    return runtime_run(case, argv, log, timeout=timeout)


def parse_vectors(path):
    s = Path(path).read_text()
    m = re.search(r'internalField\s+nonuniform\s+List<vector>\s+(\d+)\s*\((.*?)\)\s*;', s, re.S)
    if m:
        count = int(m.group(1))
        tuples = re.findall(r'\(\s*([-+0-9.eE]+)\s+([-+0-9.eE]+)\s+([-+0-9.eE]+)\s*\)', m.group(2))
        if len(tuples) != count:
            raise ValueError(f'vector count mismatch in {path}')
        return [[float(a), float(b), float(c)] for a, b, c in tuples]
    m_uni = re.search(r'internalField\s+uniform\s*\(\s*([-+0-9.eE]+)\s+([-+0-9.eE]+)\s+([-+0-9.eE]+)\s*\)', s)
    if m_uni:
        return [[float(m_uni.group(1)), float(m_uni.group(2)), float(m_uni.group(3))]]
    raise ValueError(f'could not parse vector field {path}')


def parse_positions(path):
    s = Path(path).read_text()
    # OpenFOAM 14 cloudPosition writes (x y z) or barycentric coordinates; extract explicit Cartesian positions if present
    tuples = re.findall(r'\(\s*([-+0-9.eE]+)\s+([-+0-9.eE]+)\s+([-+0-9.eE]+)\s*\)', s)
    return [[float(a), float(b), float(c)] for a, b, c in tuples]


def main():
    import numpy as np
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--omega', type=float, default=200.0, help='Angular speed rad/s')
    ap.add_argument('--dt', type=float, default=2e-6, help='Timestep s')
    ap.add_argument('--end-time', type=float, default=0.002, help='Total integration time s')
    ap.add_argument('--cpuset', type=str, default='4,5,6,7')
    ap.add_argument('--recompute-only', type=Path, default=None, help='Re-evaluate an existing executed case directory')
    a = ap.parse_args()

    rho_w = 998.0
    rho_o = 850.0
    mu_w = 0.001
    nu_w = mu_w / rho_w
    omega = a.omega
    r0 = 0.01
    d1, d2 = 20e-6, 40e-6
    u0_y = omega * r0

    if a.recompute_only is not None:
        case = a.recompute_only.resolve()
        commands = []
        for log_name in ['blockMesh.log', 'writeCellCentres.log', 'foamRun.log']:
            rec = case / f'{log_name}.receipt.json'
            if rec.exists():
                commands.append(json.loads(rec.read_text()))
    else:
        case = ROOT / f'runs/gemini-verification-rotation-w{int(a.omega)}-dt{a.dt:g}'
        if case.exists():
            case = ROOT / f'runs/gemini-verification-rotation-w{int(a.omega)}-dt{a.dt:g}-{uuid.uuid4().hex[:6]}'
        case.mkdir(parents=True, exist_ok=False)

        nx, ny, nz = 24, 24, 2
        write_dict(case, 'system/blockMeshDict', f'''convertToMeters 1;
vertices (
    (-0.02 -0.02 -0.005) (0.02 -0.02 -0.005) (0.02 0.02 -0.005) (-0.02 0.02 -0.005)
    (-0.02 -0.02  0.005) (0.02 -0.02  0.005) (0.02 0.02  0.005) (-0.02 0.02  0.005)
);
blocks (hex (0 1 2 3 4 5 6 7) ({nx} {ny} {nz}) simpleGrading (1 1 1));
edges ();
boundary (
    walls {{ type patch; faces ((0 3 2 1)(4 5 6 7)(0 1 5 4)(1 2 6 5)(2 3 7 6)(3 0 4 7)); }}
);
mergePatchPairs ();''')

        write_dict(case, 'constant/physicalProperties', f'viscosityModel constant; nu {nu_w:.16g};')
        write_dict(case, 'constant/momentumTransport', 'simulationType laminar;')
        write_dict(case, 'constant/g', 'dimensions [0 1 -2 0 0 0 0]; value (0 0 0);', 'uniformDimensionedVectorField')

        write_dict(case, '0/U', 'dimensions [0 1 -1 0 0 0 0]; internalField uniform (0 0 0); boundaryField {walls {type calculated; value uniform (0 0 0);}}', 'volVectorField')
        write_dict(case, '0/p', 'dimensions [0 2 -2 0 0 0 0]; internalField uniform 0; boundaryField {walls {type calculated; value uniform 0;}}', 'volScalarField')
        write_dict(case, 'system/controlDict', f'''application foamRun; solver functions; subSolver incompressibleFluid;
startFrom startTime; startTime 0; stopAt endTime; endTime {a.end_time:g}; deltaT {a.dt:g};
writeControl timeStep; writeInterval {max(1, round(a.end_time / a.dt / 10))}; writeFormat ascii; writePrecision 15;
timePrecision 12; runTimeModifiable false;''')
        write_dict(case, 'system/fvSchemes', '''ddtSchemes {default Euler;} gradSchemes {default Gauss linear;}
divSchemes {default none; div(phi,U) Gauss linear;}
laplacianSchemes {default Gauss linear corrected;} interpolationSchemes {default linear;} snGradSchemes {default corrected;}''')
        write_dict(case, 'system/fvSolution', 'solvers {} PIMPLE {nCorrectors 0; momentumPredictor no; pRefCell 0; pRefValue 0;}')

        commands = []
        commands.append(run_container(case, ['blockMesh'], case / 'blockMesh.log', cpuset=a.cpuset))
        commands.append(run_container(case, ['foamPostProcess', '-func', 'writeCellCentres', '-time', '0'], case / 'writeCellCentres.log', cpuset=a.cpuset))

        centres = np.array(parse_vectors(case / '0/C'))
        n_cells = len(centres)
        u_cells = np.column_stack([-omega * centres[:, 1], omega * centres[:, 0], np.zeros(n_cells)])
        p_cells = 0.5 * (omega ** 2) * (centres[:, 0] ** 2 + centres[:, 1] ** 2)

        u_list_str = '\n'.join(f'({u[0]:.15g} {u[1]:.15g} {u[2]:.15g})' for u in u_cells)
        p_list_str = '\n'.join(f'{p:.15g}' for p in p_cells)

        write_dict(case, '0/U', f'''dimensions [0 1 -1 0 0 0 0];
internalField nonuniform List<vector> {n_cells} (
{u_list_str}
);
boundaryField {{ walls {{ type zeroGradient; }} }}''', 'volVectorField')

        write_dict(case, '0/p', f'''dimensions [0 2 -2 0 0 0 0];
internalField nonuniform List<scalar> {n_cells} (
{p_list_str}
);
boundaryField {{ walls {{ type zeroGradient; }} }}''', 'volScalarField')

        for cname, diam in [('cloud20', d1), ('cloud40', d2)]:
            write_dict(case, f'constant/Lagrangian/{cname}/physicalProperties', f'rhoByRhoc {rho_o / rho_w:.16g};')
            write_dict(case, f'constant/Lagrangian/{cname}/LagrangianModels', '''injection {
    type manualInjection;
    positions ((0.01 0 0.0001));
    time 0;
}
drag { type SchillerNaumannDrag; }
pressure { type pressureGradientForce; }
addedMass { type constantCoefficientVirtualMass; Cvm 0.5; }''')
            write_dict(case, f'0/Lagrangian/{cname}/U', f'''dimensions [0 1 -1 0 0 0 0];
internalField uniform (0 {u0_y:.15g} 0);
boundaryField {{ ".*" {{ type escapeVelocity; }} }}
sources {{ injection {{ type uniformFixedValue; uniformValue (0 {u0_y:.15g} 0); }} }}''', 'LagrangianVectorField')
            write_dict(case, f'0/Lagrangian/{cname}/d', f'''dimensions [0 1 0 0 0 0 0];
internalField uniform {diam:.15g};
boundaryField {{ ".*" {{ type calculated; }} }}
sources {{ injection {{ type uniformFixedValue; uniformValue {diam:.15g}; }} }}''', 'LagrangianScalarField')
            write_dict(case, f'system/Lagrangian/{cname}/LagrangianSchemes', '''tracking linear;
ddtSchemes {default Euler;} SpSchemes {default none; Sp(D,U) implicit;}
averagingSchemes {} interpolationSchemes {default cellPoint; Uc cellPoint; grad(p) cellPoint;} accumulationSchemes {}''')
            write_dict(case, f'system/Lagrangian/{cname}/LagrangianSolution', 'maxTimeStepFraction 0.2; maxCellLengthScaleFraction 0.2; nCorrectors 1;')

        write_dict(case, 'system/controlDict', f'''application foamRun; solver functions; subSolver incompressibleFluid;
startFrom startTime; startTime 0; stopAt endTime; endTime {a.end_time:g}; deltaT {a.dt:g};
writeControl timeStep; writeInterval {max(1, round(a.end_time / a.dt / 5))}; writeFormat ascii; writePrecision 15;
timePrecision 12; runTimeModifiable false;
functions {{
    cloud20 {{ type kinematicParticleCloud; libs ("libLagrangianCloud.so"); cloud cloud20; executeControl timeStep; writeControl writeTime; }}
    pos20 {{ type cloudPosition; libs ("libLagrangianCloudFunctionObjects.so"); cloud cloud20; }}
    cloud40 {{ type kinematicParticleCloud; libs ("libLagrangianCloud.so"); cloud cloud40; executeControl timeStep; writeControl writeTime; }}
    pos40 {{ type cloudPosition; libs ("libLagrangianCloudFunctionObjects.so"); cloud cloud40; }}
}}''')

        commands.append(run_container(case, ['foamRun'], case / 'foamRun.log', cpuset=a.cpuset))

    times = sorted(
        (float(p.name), p)
        for p in case.iterdir()
        if p.is_dir() and re.fullmatch(r'[0-9.eE+-]+', p.name) and float(p.name) > 0
    )
    if not times:
        raise RuntimeError('no output time directories written')

    t_final, p_final = times[-1]
    pos20 = parse_vectors(p_final / 'Lagrangian/cloud20/position')[0]
    pos40 = parse_vectors(p_final / 'Lagrangian/cloud40/position')[0]
    u20 = parse_vectors(p_final / 'Lagrangian/cloud20/U')[0]
    u40 = parse_vectors(p_final / 'Lagrangian/cloud40/U')[0]

    r20 = math.hypot(pos20[0], pos20[1])
    r40 = math.hypot(pos40[0], pos40[1])
    ur20 = (pos20[0] * u20[0] + pos20[1] * u20[1]) / r20
    ur40 = (pos40[0] * u40[0] + pos40[1] * u40[1]) / r40

    def integrate_coupled_ode(diam: float) -> tuple[float, float]:
        """Integrate exact coupled radial-tangential Lagrangian ODE (Schiller-Naumann + pressure gradient + virtual mass)."""
        r = r0
        ur = 0.0
        ut = omega * r0
        c_vm = 0.5
        m_eff = rho_o + c_vm * rho_w
        n_sub = max(1, round(t_final / (a.dt * 0.1)))
        h = t_final / n_sub
        for _ in range(n_sub):
            uf_r = 0.0
            uf_t = omega * r
            u_rel = math.hypot(uf_r - ur, uf_t - ut)
            re_p = rho_w * u_rel * diam / mu_w
            f_sn = 1.0 + 0.15 * (re_p ** 0.687) if re_p > 0.0 else 1.0
            drag_coeff = (18.0 * mu_w / (diam ** 2)) * f_sn
            # Radial equation of motion in inertial frame written in polar coordinates:
            # m_eff * du_r/dt = m_p * (ut^2 / r) + c_vm * rho_w * (ut^2 / r - omega^2 * r) - rho_w * omega^2 * r - drag_coeff * ur
            f_rad = rho_o * (ut ** 2 / r) + c_vm * rho_w * (ut ** 2 / r - (omega ** 2) * r) - rho_w * (omega ** 2) * r
            f_tan = -rho_o * (ur * ut / r) - c_vm * rho_w * (ur * ut / r) + drag_coeff * uf_t
            ur = (ur + (h / m_eff) * f_rad) / (1.0 + (h / m_eff) * drag_coeff)
            ut = (ut + (h / m_eff) * f_tan) / (1.0 + (h / m_eff) * drag_coeff)
            r += h * ur
        return r, ur

    r20_ode, ur20_ode = integrate_coupled_ode(d1)
    r40_ode, ur40_ode = integrate_coupled_ode(d2)
    ur20_stokes = -(rho_w - rho_o) * (omega ** 2) * r20 * (d1 ** 2) / (18.0 * mu_w)
    ur40_stokes = -(rho_w - rho_o) * (omega ** 2) * r40 * (d2 ** 2) / (18.0 * mu_w)

    inward_migration_ok = (r20 < r0) and (r40 < r20 < r0) and (ur20 < 0.0) and (ur40 < 0.0)
    d2_ratio_actual = (ur40 / r40) / (ur20 / r20)
    d2_ratio_ode = (ur40_ode / r40_ode) / (ur20_ode / r20_ode)
    rel_err_20_ode = abs(ur20 - ur20_ode) / abs(ur20_ode)
    rel_err_40_ode = abs(ur40 - ur40_ode) / abs(ur40_ode)
    rel_err_20_stokes = abs(ur20 - ur20_stokes) / abs(ur20_stokes)
    rel_err_40_stokes = abs(ur40 - ur40_stokes) / abs(ur40_stokes)
    passed = inward_migration_ok and (rel_err_20_ode < 0.02) and (rel_err_40_ode < 0.02) and (abs(d2_ratio_actual / d2_ratio_ode - 1.0) < 0.02)

    result = {
        'id': 'OIL-03',
        'kind': 'native_rotating_vortex_inward_migration_verification',
        'passed': passed,
        'omega_rad_s': omega,
        'r0_m': r0,
        't_final_s': t_final,
        'd20_um': {
            'r_final_m': r20,
            'r_ode_oracle_m': r20_ode,
            'delta_r_m': r20 - r0,
            'ur_actual_m_s': ur20,
            'ur_coupled_ode_oracle_m_s': ur20_ode,
            'ur_stokes_limit_m_s': ur20_stokes,
            'relative_error_vs_ode': rel_err_20_ode,
            'relative_error_vs_stokes': rel_err_20_stokes,
        },
        'd40_um': {
            'r_final_m': r40,
            'r_ode_oracle_m': r40_ode,
            'delta_r_m': r40 - r0,
            'ur_actual_m_s': ur40,
            'ur_coupled_ode_oracle_m_s': ur40_ode,
            'ur_stokes_limit_m_s': ur40_stokes,
            'relative_error_vs_ode': rel_err_40_ode,
            'relative_error_vs_stokes': rel_err_40_stokes,
        },
        'd2_drift_rate_ratio_actual': d2_ratio_actual,
        'd2_drift_rate_ratio_ode_oracle': d2_ratio_ode,
        'd2_drift_rate_ratio_stokes': 4.0,
        'inward_migration_verified': inward_migration_ok,
        'commands': commands,
        'scope': 'solid-body rotating vortex inward migration verified against coupled Schiller-Naumann + pressure-gradient + virtual-mass ODE (<3% tolerance) without centrifugal double-counting',
    }
    (case / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k: v for k, v in result.items() if k != 'commands'}, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
