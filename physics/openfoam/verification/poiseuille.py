#!/usr/bin/env python3
"""Executed laminar plane-channel oracle, with independent analytical solution."""
import json
import argparse
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'physics/openfoam/mesh'))
sys.path.insert(0,str(ROOT/'physics/openfoam/environment'))
from prepare import write
from runtime import run


def vectors(path):
    import numpy as np
    s=Path(path).read_text()
    m=re.search(r'internalField\s+nonuniform\s+List<vector>\s+(\d+)\s*\((.*?)\)\s*;',s,re.S)
    if not m: raise ValueError(f'expected explicit vector field {path}')
    v=np.fromstring(m[2].replace('(',' ').replace(')',' '),sep=' ').reshape(-1,3)
    if len(v)!=int(m[1]): raise ValueError('field length mismatch')
    return v


def main():
    import numpy as np
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--cells',type=int,choices=[16,32,64],default=32)
    ap.add_argument('--iterations',type=int,default=2000)
    a=ap.parse_args()
    case=ROOT/f'runs/verification-poiseuille-{a.cells}-{a.iterations}'
    case.mkdir(parents=True,exist_ok=False)
    write(case,'system/blockMeshDict',('''convertToMeters 1;
vertices ((0 -.005 0)(.1 -.005 0)(.1 .005 0)(0 .005 0)
          (0 -.005 .001)(.1 -.005 .001)(.1 .005 .001)(0 .005 .001));
blocks (hex (0 1 2 3 4 5 6 7) (8 NCELLS 1) simpleGrading (1 1 1)); edges ();
boundary (
 inlet {type patch; faces ((0 4 7 3));}
 outlet {type patch; faces ((1 2 6 5));}
 wall {type wall; faces ((0 1 5 4)(3 7 6 2));}
 frontBack {type empty; faces ((0 3 2 1)(4 5 6 7));}); mergePatchPairs ();''').replace('NCELLS',str(a.cells)))
    write(case,'constant/physicalProperties','viscosityModel constant; nu 1e-6;')
    write(case,'constant/momentumTransport','simulationType laminar;')
    write(case,'0/U','''dimensions [0 1 -1 0 0 0 0]; internalField uniform (0 0 0);
boundaryField {"(inlet|outlet)" {type zeroGradient;} wall {type noSlip;} frontBack {type empty;}}''','volVectorField')
    write(case,'0/p','''dimensions [0 2 -2 0 0 0 0]; internalField uniform 0;
boundaryField {inlet {type fixedValue; value uniform .0001;} outlet {type fixedValue; value uniform 0;}
 wall {type zeroGradient;} frontBack {type empty;}}''','volScalarField')
    write(case,'system/controlDict',f'''application foamRun; solver incompressibleFluid;
startFrom startTime; startTime 0; stopAt endTime; endTime {a.iterations}; deltaT 1;
writeControl timeStep; writeInterval {a.iterations}; writeFormat ascii; writePrecision 14; runTimeModifiable false;''')
    write(case,'system/fvSchemes','''ddtSchemes {default steadyState;} gradSchemes {default Gauss linear;}
divSchemes {default none; div(phi,U) bounded Gauss upwind; div((nuEff*dev2(T(grad(U))))) Gauss linear;}
laplacianSchemes {default Gauss linear corrected;} interpolationSchemes {default linear;} snGradSchemes {default corrected;}''')
    write(case,'system/fvSolution','''solvers {p {solver GAMG; tolerance 1e-12; relTol 0; smoother GaussSeidel;}
U {solver smoothSolver; smoother symGaussSeidel; tolerance 1e-12; relTol 0;}}
SIMPLE {nNonOrthogonalCorrectors 0; consistent yes;}
relaxationFactors {fields {p .3;} equations {U .7;}}''')
    commands=[]
    for argv in [['blockMesh'], ['checkMesh'], ['foamRun'], ['foamPostProcess','-func','writeCellCentres','-latestTime']]:
        commands.append(run(case,argv,case/(argv[0]+'.log'),timeout=180))
    c=vectors(case/str(a.iterations)/'C');u=vectors(case/str(a.iterations)/'U')
    expected=.001*(.005**2-c[:,1]**2)/(2e-6)
    error=float(np.sqrt(np.mean((u[:,0]-expected)**2))/np.sqrt(np.mean(expected**2)))
    cross=float(np.max(np.linalg.norm(u[:,1:],axis=1)))
    # Second-order face placement gives O(h^2) error on this 32-cell grid.
    passed=error<.005 and cross<1e-8
    result={'kind':'elementary_numerical_verification','oracle':'u(y)=(-dp/dx)(h^2-y^2)/(2 nu)',
            'density_kg_m3':1000.,'kinematic_pressure_drop_m2_s2':.0001,'length_m':.1,
            'half_gap_m':.005,'nu_m2_s':1e-6,'cells_across_gap':a.cells,'relative_l2_velocity_error':error,
            'max_cross_velocity_m_s':cross,'passed':passed,'commands':commands,
            'scope':'laminar pressure-driven channel only; no turbulence or droplet qualification'}
    (case/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
    if not passed: raise SystemExit(1)


if __name__=='__main__':main()
