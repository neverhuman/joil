"""Foundation 14 carrier dictionaries. Research initializer, not accepted F2."""
import argparse
import math
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'physics/openfoam/mesh'))
sys.path.insert(0, str(ROOT / 'cad/ct35'))
from prepare import write
from geometry import load


def configure(case, flow=4., reject_pressure=-200., steps=500, model='SSG', transient=False):
    case = Path(case)
    if not case.resolve().is_relative_to(ROOT/'runs') or (case/'input-manifest.json').exists():
        raise ValueError('configure only a fresh project run; preserve previously bound inputs')
    if transient:
        raise NotImplementedError('physical-time transient profile requires explicit end-time/dt contract; this driver is F1 only')
    if not math.isfinite(flow) or flow<=0 or not math.isfinite(reject_pressure) or type(steps) is not int or steps<1:
        raise ValueError('finite positive flow, pressure and iteration budget required')
    if model not in ['SSG','LRR','kOmegaSST']:raise ValueError('unsupported model identity')
    _, d = load()
    qi = flow / 3600
    velocity = qi / (2*d['area']*1e-6)
    k = 1.5*(.05*velocity)**2
    dh = 2*d['w']*d['h']/(d['w']+d['h'])*.001
    epsilon = .09**.75 * k**1.5 / (.07*dh)
    write(case, 'constant/physicalProperties', f'viscosityModel constant; nu {0.001/998:.15g};')
    write(case, 'constant/momentumTransport', f'simulationType RAS; RAS {{model {model}; turbulence on; printCoeffs on;}}')
    write(case, '0/U', f'''dimensions [0 1 -1 0 0 0 0]; internalField uniform (0 0 0);
boundaryField {{
 inlet1 {{type flowRateInletVelocity; volumetricFlowRate constant {qi/2:.15g}; value uniform ({velocity} 0 0);}}
 inlet2 {{type flowRateInletVelocity; volumetricFlowRate constant {qi/2:.15g}; value uniform ({-velocity} 0 0);}}
 "(overflow|underflow)" {{type pressureInletOutletVelocity; value uniform (0 0 0);}}
 wall {{type noSlip;}}
}}''', 'volVectorField')
    write(case, '0/p', f'''dimensions [0 2 -2 0 0 0 0]; internalField uniform 0;
boundaryField {{inlet1 {{type zeroGradient;}} inlet2 {{type zeroGradient;}}
 overflow {{type fixedValue; value uniform {reject_pressure/998:.15g};}}
 underflow {{type fixedValue; value uniform 0;}} wall {{type zeroGradient;}}}}''', 'volScalarField')
    for name, value, dimensions in [('k',k,'0 2 -2'), ('epsilon',epsilon,'0 2 -3'), ('omega',epsilon/(.09*k),'0 0 -1')]:
        walltype = {'epsilon':'epsilonWallFunction','omega':'omegaWallFunction','k':'kqRWallFunction'}[name]
        write(case, '0/'+name, f'''dimensions [{dimensions} 0 0 0 0]; internalField uniform {value};
boundaryField {{"inlet[12]" {{type fixedValue; value uniform {value};}}
 "(overflow|underflow)" {{type inletOutlet; inletValue uniform {value}; value uniform {value};}}
 wall {{type {walltype}; value uniform {value};}}}}''', 'volScalarField')
    write(case, '0/nut', '''dimensions [0 2 -1 0 0 0 0]; internalField uniform 0;
boundaryField {"(inlet[12]|overflow|underflow)" {type calculated; value uniform 0;}
 wall {type nutkWallFunction; value uniform 0;}}''', 'volScalarField')
    r = f'({2*k/3} 0 0 {2*k/3} 0 {2*k/3})'
    write(case, '0/R', f'''dimensions [0 2 -2 0 0 0 0]; internalField uniform {r};
boundaryField {{"inlet[12]" {{type fixedValue; value uniform {r};}}
 "(overflow|underflow)" {{type inletOutlet; inletValue uniform {r}; value uniform {r};}}
 wall {{type kqRWallFunction; value uniform {r};}}}}''', 'volSymmTensorField')
    functions = []
    for name in ['inlet1','inlet2','overflow','underflow']:
        for suffix, operation, field in [('flow','sum','phi'),('pressure','areaAverage','p')]:
            functions.append(f'''{name}_{suffix} {{type surfaceFieldValue; libs ("libfieldFunctionObjects.so");
writeControl timeStep; writeInterval 1; log false; patch {name};
operation {operation}; fields ({field}); writeFields false;}}''')
    fields='p U R epsilon' if model in ['SSG','LRR'] else 'p U k omega'
    functions.append(f'''residuals {{type residuals; libs ("libutilityFunctionObjects.so");
writeControl timeStep; writeInterval 1; fields ({fields});}}''')
    write(case, 'system/controlDict', f'''application foamRun; solver incompressibleFluid;
startFrom latestTime; startTime 0; stopAt endTime; endTime {steps}; deltaT {0.00001 if transient else 1};
writeControl timeStep; writeInterval {max(1,steps//2)}; purgeWrite 2;
writeFormat ascii; writePrecision 12; writeCompression off; timePrecision 10;
runTimeModifiable false;
functions {{ {' '.join(functions)} }}''')
    write(case, 'system/fvSchemes', f'''ddtSchemes {{default {'Euler' if transient else 'steadyState'};}}
gradSchemes {{default cellLimited Gauss linear 1;}}
divSchemes {{default none;
 div(phi,U) bounded Gauss upwind;
 div(phi,R) bounded Gauss upwind;
 div(phi,k) bounded Gauss upwind;
 div(phi,epsilon) bounded Gauss upwind;
 div(phi,omega) bounded Gauss upwind;
 div((nuEff*dev2(T(grad(U))))) Gauss linear;
 div(R) Gauss linear;
}}
laplacianSchemes {{default Gauss linear limited .5;}}
interpolationSchemes {{default linear;}} snGradSchemes {{default limited .5;}}
wallDist {{method meshWave;}}''')
    write(case, 'system/fvSolution', '''solvers {
 p {solver GAMG; tolerance 1e-8; relTol .05; smoother GaussSeidel;}
 pFinal {$p; relTol 0;}
 pcorr {solver GAMG; tolerance 1e-8; relTol 0; smoother GaussSeidel;}
 "(U|R|k|epsilon|omega)" {solver smoothSolver; smoother symGaussSeidel; tolerance 1e-8; relTol .05;}
 "(U|R|k|epsilon|omega)Final" {solver smoothSolver; smoother symGaussSeidel; tolerance 1e-8; relTol 0;}
}
SIMPLE {nNonOrthogonalCorrectors 1; consistent yes;}
PIMPLE {nOuterCorrectors 1; nCorrectors 1; nNonOrthogonalCorrectors 1; consistent yes;}
relaxationFactors {fields {p .2;} equations {U .4; R .1; k .3; epsilon .3; omega .3;}}
''')
    write(case, 'system/decomposeParDict', 'numberOfSubdomains 4; method scotch;')
    (case/'carrier-input.json').write_text(json.dumps({'schema_version':1,'kind':'F1_steady_initializer_only',
        'model':model,'feed_m3_h':flow,'reject_pressure_pa':reject_pressure,'water_outlet_pressure_pa':0.,
        'rho_kg_m3':998.,'mu_pa_s':.001,'gravity_m_s2':[0,0,0],'iterations':steps,
        'time_is_pseudo_iteration':True,'inlet_turbulent_intensity':.05,'inlet_length_scale_m':.07*dh,
        'genome_file_sha256':hashlib.sha256((ROOT/'baselines/ct35/genome.json').read_bytes()).hexdigest(),
        'configuration_source_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},indent=2)+'\n')


if __name__ == '__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--case', type=Path, required=True)
    ap.add_argument('--flow', type=float, default=4)
    ap.add_argument('--reject-pressure-pa', type=float, default=-200)
    ap.add_argument('--steps', type=int, default=500)
    ap.add_argument('--model', choices=['SSG','LRR','kOmegaSST'], default='SSG')
    a=ap.parse_args()
    configure(a.case, a.flow, a.reject_pressure_pa, a.steps, a.model)
