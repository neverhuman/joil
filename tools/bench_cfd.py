#!/usr/bin/env python3
"""Execute bounded mesh or carrier stages; preserve every log and return code."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'physics/openfoam/environment'))
from runtime import run


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('stage',choices=['mesh','tet','water','reconstruct','centres','build-metrics','metrics'])
    ap.add_argument('--case',type=Path,required=True)
    ap.add_argument('--metrics-bin',type=Path)
    ap.add_argument('--time',type=float,help='explicit saved pseudo-time for reconstruction/extraction')
    a=ap.parse_args();case=a.case.resolve()
    stages={'mesh':[['blockMesh'],['snappyHexMesh','-overwrite'],['checkMesh','-allTopology','-allGeometry']],
            'tet':[['gmshToFoam','ct35.msh']],
            'water':[['decomposePar','-force'],['mpirun','--bind-to','none','-np','4','foamRun','-parallel']],
            'reconstruct':[['reconstructPar','-latestTime']],
            'centres':[['foamPostProcess','-func','writeCellCentres','-latestTime']],
            'build-metrics':[['bash','-c','cp -r /work/physics/openfoam/carrier/hydraulicMetrics build && export FOAM_USER_APPBIN=/case/bin && mkdir -p "$FOAM_USER_APPBIN" && cd build && wmake']],
            'metrics':[]}
    if a.stage=='metrics':
        if a.metrics_bin is None:raise ValueError('--metrics-bin must identify the compiled audited utility')
        binary=a.metrics_bin.resolve()
        if not binary.is_file() or not binary.is_relative_to(ROOT/'runs'):
            raise ValueError('metrics binary must be a project run artifact')
        stages['metrics']=[['/work/'+str(binary.relative_to(ROOT)),'-latestTime']]
        if (case/'hydraulic-integrals.json').exists():raise ValueError('preserve existing native integral output before another extraction')
        suffix=f'-t{a.time:g}' if a.time is not None else ''
        metadata=case/f'metrics-binary{suffix}.json'
        if metadata.exists():raise ValueError('metrics extraction already recorded for this saved time')
        metadata.write_text(json.dumps({'path':str(binary.relative_to(ROOT)),
            'sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2)+'\n')
    for i,argv in enumerate(stages[a.stage]):
        if a.time is not None and a.stage in ['reconstruct','centres','metrics']:
            argv=argv[:-1]+['-time',f'{a.time:g}']
        if a.stage=='metrics' and (case/'device-plane.json').exists():
            plane=json.loads((case/'device-plane.json').read_text())
            if hashlib.sha256((case/'constant/polyMesh/sets/deviceOverflow').read_bytes()).hexdigest()!=plane['face_set_sha256']:
                raise ValueError('original device measurement plane changed')
            argv=argv+['-devicePlane']
        label=a.stage+(f'-t{a.time:g}' if a.time is not None else '')
        p=case/f'{label}-{i}-{argv[0].split("/")[-1]}.log'
        if p.exists():raise ValueError('stage log exists; use a new named attempt/case')
        run(case,argv,p,timeout=3600)
    if a.stage=='metrics' and a.time is not None:
        target=case/f'hydraulic-integrals-t{a.time:g}.json'
        if target.exists():raise ValueError('integral receipt already exists')
        native=case/'hydraulic-integrals.json'
        value=json.loads(native.read_text())
        if value['time']!=a.time:raise ValueError('wrong saved time extracted')
        native.rename(target)
    if a.stage=='tet':
        p=case/'constant/polyMesh/boundary';before=p.read_bytes()
        import re
        text=before.decode()
        text,n=re.subn(r'(\bwall\s*\{\s*type\s+)patch(\s*;)',r'\1wall\2',text)
        if n!=1:raise ValueError('expected exactly one generic wall group from gmshToFoam')
        p.write_text(text)
        (case/'boundary-type-receipt.json').write_text(json.dumps({'operation':'set wall physical group to OpenFOAM wall patch type',
            'before_sha256':hashlib.sha256(before).hexdigest(),'after_sha256':hashlib.sha256(p.read_bytes()).hexdigest()},indent=2)+'\n')
        run(case,['checkMesh','-allTopology','-allGeometry'],case/'mesh-2-checkMesh.log',timeout=300)


if __name__=='__main__':main()
