#!/usr/bin/env python3
"""Native Foundation 14 liquid-density droplet terminal-rise/response oracle."""
import argparse
import json
import math
from pathlib import Path
import re
import sys

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'physics/openfoam/mesh'))
sys.path.insert(0,str(ROOT/'physics/openfoam/environment'))
from prepare import write
from runtime import run


def one_vector(path):
    s=Path(path).read_text()
    m=re.search(r'internalField\s+uniform\s*\(([^)]+)\)',s)
    if not m:
        m=re.search(r'internalField\s+nonuniform\s+List<vector>\s+1\s*\(\s*\(([^)]+)\)',s)
    if not m:raise ValueError('expected exactly one particle vector')
    return [float(x) for x in m[1].split()]


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--dt',type=float,default=1e-7)
    a=ap.parse_args()
    case=ROOT/f'runs/verification-drop-rise-{a.dt:g}'
    case.mkdir(parents=True,exist_ok=False)
    write(case,'system/blockMeshDict','''convertToMeters 1;
vertices ((-.02 -.02 -.02)(.02 -.02 -.02)(.02 .02 -.02)(-.02 .02 -.02)
          (-.02 -.02 .02)(.02 -.02 .02)(.02 .02 .02)(-.02 .02 .02));
blocks (hex (0 1 2 3 4 5 6 7) (2 2 2) simpleGrading (1 1 1)); edges ();
boundary (walls {type patch; faces ((0 3 2 1)(4 5 6 7)(0 1 5 4)(1 2 6 5)(2 3 7 6)(3 0 4 7));}); mergePatchPairs ();''')
    write(case,'constant/physicalProperties',f'viscosityModel constant; nu {.001/998:.16g};')
    write(case,'constant/momentumTransport','simulationType laminar;')
    write(case,'constant/g','dimensions [0 1 -2 0 0 0 0]; value (0 0 9.81);','uniformDimensionedVectorField')
    write(case,'0/U','dimensions [0 1 -1 0 0 0 0]; internalField uniform (0 0 0); boundaryField {walls {type fixedValue; value uniform (0 0 0);}}','volVectorField')
    write(case,'0/p','dimensions [0 2 -2 0 0 0 0]; internalField uniform 0; boundaryField {walls {type fixedValue; value uniform 0;}}','volScalarField')
    write(case,'system/controlDict',f'''application foamRun; solver functions; subSolver incompressibleFluid;
startFrom startTime; startTime 0; stopAt endTime; endTime .0002; deltaT {a.dt};
writeControl timeStep; writeInterval {round(1e-5/a.dt)}; writeFormat ascii; writePrecision 15;
timePrecision 12; runTimeModifiable false;
functions {{cloud {{type kinematicParticleCloud; libs ("libLagrangianCloud.so"); executeControl timeStep; writeControl writeTime;}}
position {{type cloudPosition; libs ("libLagrangianCloudFunctionObjects.so"); cloud cloud;}}}}''')
    write(case,'system/fvSchemes','''ddtSchemes {default Euler;} gradSchemes {default Gauss linear;}
divSchemes {default none; div(phi,U) Gauss linear;}
laplacianSchemes {default Gauss linear corrected;} interpolationSchemes {default linear;} snGradSchemes {default corrected;}''')
    write(case,'system/fvSolution','solvers {} PIMPLE {nCorrectors 1; pRefCell 0; pRefValue 0;}')
    write(case,'constant/Lagrangian/cloud/physicalProperties',f'rhoByRhoc {850/998:.16g};')
    write(case,'constant/Lagrangian/cloud/LagrangianModels','''injection {type manualInjection; positions ((.001 .001 .001)); time 0;}
drag {type SchillerNaumannDrag;}
pressure {type pressureGradientForce;}
addedMass {type constantCoefficientVirtualMass; Cvm .5;}
buoyancy {type gravity;}''')
    for name, value, dims, cls in [('U','(0 0 0)','0 1 -1','LagrangianVectorField'),
                                  ('d','1e-5','0 1 0','LagrangianScalarField')]:
        write(case,f'0/Lagrangian/cloud/{name}',f'''dimensions [{dims} 0 0 0 0]; internalField uniform {value};
boundaryField {{".*" {{type {'escapeVelocity' if name=='U' else 'calculated'};}}}}
sources {{injection {{type uniformFixedValue; uniformValue {value};}}}}''',cls)
    write(case,'system/Lagrangian/cloud/LagrangianSchemes','''tracking linear;
ddtSchemes {default Euler;} SpSchemes {default none; Sp(D,U) implicit;}
averagingSchemes {} interpolationSchemes {default cell; Uc cellPoint;} accumulationSchemes {}''')
    write(case,'system/Lagrangian/cloud/LagrangianSolution','maxTimeStepFraction .3; maxCellLengthScaleFraction .3; nCorrectors 1;')
    commands=[]
    for argv in [['blockMesh'],['foamRun']]:
        commands.append(run(case,argv,case/(argv[0]+'.log'),timeout=300))
    times=sorted((float(p.name),p) for p in case.iterdir() if p.is_dir() and re.fullmatch(r'[0-9.e+-]+',p.name) and float(p.name)>0)
    terminal=(850-998)*9.81*1e-10/(18*.001)
    tau=(850+.5*998)*1e-10/(18*.001)
    rows=[]
    for t,p in times:
        actual=one_vector(p/'Lagrangian/cloud/U')[2]
        exact=terminal*(1-math.exp(-t/tau))
        rows.append({'time_s':t,'actual_m_s':actual,'stokes_response_m_s':exact,'absolute_error_over_terminal':abs(actual-exact)/abs(terminal)})
    final=abs(rows[-1]['actual_m_s']/terminal-1)
    response=max(x['absolute_error_over_terminal'] for x in rows)
    passed=final<.001 and response<.01
    result={'kind':'native_liquid_density_force_verification','passed':passed,
            'models':['SchillerNaumannDrag','pressureGradientForce','constantCoefficientVirtualMass(Cvm=.5)','gravity'],
            'oil_density_kg_m3':850,'water_density_kg_m3':998,'oil_diameter_m':1e-5,
            'interface_assumption':'immobile spherical interface; no clean-drop correction',
            'oracle_terminal_m_s':terminal,'oracle_response_time_s':tau,
            'terminal_relative_error':final,'max_response_error_over_terminal':response,
            'dt_s':a.dt,'samples':rows,'commands':commands,
            'scope':'quiescent liquid, buoyancy and added-mass relaxation; pressure-gradient rotation and dispersion still unqualified'}
    (case/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ['samples','commands']},indent=2))
    if not passed:raise SystemExit(1)


if __name__=='__main__':main()
