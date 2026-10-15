#!/usr/bin/env python3
"""Prepare a content-bound operating point; failed mesh needs explicit diagnostic mode."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import sys

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(Path(__file__).resolve().parent))
from configure import configure


def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def prepare_point(case,mesh_case,*,model,flow,reject_pressure,iterations,
                  diagnostic=False,warm_case=None,warm_time=None):
    case=Path(case).resolve();mesh_case=Path(mesh_case).resolve()
    if not case.is_relative_to(ROOT/'runs') or not mesh_case.is_relative_to(ROOT/'runs'):
        raise ValueError('project runs only')
    if case.exists():raise ValueError('new case path required')
    audit=json.loads((mesh_case/'mesh-audit.json').read_text())
    if not audit['pilot_mesh_pass'] and not diagnostic:
        raise ValueError('mesh qualification failed; only explicit unqualified diagnostics allowed')
    case.mkdir(parents=True)
    shutil.copytree(mesh_case/'constant/polyMesh',case/'constant/polyMesh')
    for name in ['mesh-input.json','mesh-audit.json']:shutil.copy2(mesh_case/name,case/name)
    if (mesh_case/'domain-extension.json').exists():
        shutil.copy2(mesh_case/'domain-extension.json',case/'domain-extension.json')
    configure(case,flow=flow,reject_pressure=reject_pressure,steps=iterations,model=model)
    if warm_case is not None:
        warm_case=Path(warm_case).resolve()
        if not warm_case.is_relative_to(ROOT/'runs') or warm_time is None:
            raise ValueError('explicit project warm-start case and saved time required')
        if (warm_case/'numerical-failure.json').exists():raise ValueError('cannot seed from a failed numerical run')
        source=json.loads((warm_case/'carrier-input.json').read_text())
        if source['model']!=model:raise ValueError('cross-model warm start requires a separate field conversion contract')
        receipt=json.loads((warm_case/'water-1-mpirun.log.receipt.json').read_text())
        if receipt['exit_code']!=0:raise ValueError('source run did not complete successfully')
        for name in ['points','faces','owner','neighbour','boundary']:
            if sha(warm_case/'constant/polyMesh'/name)!=sha(case/'constant/polyMesh'/name):
                raise ValueError('cannot map fields across a changed mesh without qualified conservative mapping')
        src=warm_case/f'{warm_time:g}'
        transferred={}
        for name in ['U','p','phi','k','omega','epsilon','R','nut']:
            f=src/name
            if f.exists():
                transferred[name]=sha(f);shutil.copy2(f,case/'0'/name)
        if not {'U','p','phi','k','nut'}.issubset(transferred):raise ValueError('incomplete saved carrier fields')
        # Preserve internal pressure while changing only the reject boundary.
        p=case/'0/p';text=p.read_text()
        text,n=re.subn(r'(\boverflow\s*\{[^{}]*?\bvalue\s+uniform\s+)[-+\d.eE]+(?=\s*;)',
                       lambda m:m[1]+f'{reject_pressure/998:.15g}',text,flags=re.S)
        if n!=1:raise ValueError('reject pressure boundary not uniquely identified')
        p.write_text(text)
        (case/'warm-start.json').write_text(json.dumps({'source_case':warm_case.name,'source_saved_pseudo_time':warm_time,
             'source_field_sha256':transferred,'method':'identical mesh direct copy, reject boundary pressure revised',
             'physical_time_continuation':False},indent=2)+'\n')
    (case/'research-status.json').write_text(json.dumps({'purpose':'unqualified F1 operating-point diagnostic' if diagnostic else 'pilot carrier point',
        'fidelity':'F1 steady initializer','mesh_pilot_pass':audit['pilot_mesh_pass'],
        'physical_validation':False,'campaign_admission':False,'qualified_fitness':None},indent=2)+'\n')
    files=[p for p in case.rglob('*') if p.is_file()]
    (case/'input-manifest.json').write_text(json.dumps({str(p.relative_to(case)):sha(p) for p in files},indent=2)+'\n')
    print(case)


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--case',type=Path,required=True);ap.add_argument('--mesh-case',type=Path,required=True)
    ap.add_argument('--model',choices=['SSG','LRR','kOmegaSST'],default='kOmegaSST')
    ap.add_argument('--flow',type=float,default=4.);ap.add_argument('--reject-pressure-pa',type=float,required=True)
    ap.add_argument('--iterations',type=int,default=300);ap.add_argument('--diagnostic',action='store_true')
    ap.add_argument('--warm-case',type=Path);ap.add_argument('--warm-time',type=float)
    a=ap.parse_args();prepare_point(a.case,a.mesh_case,model=a.model,flow=a.flow,
        reject_pressure=a.reject_pressure_pa,iterations=a.iterations,diagnostic=a.diagnostic,
        warm_case=a.warm_case,warm_time=a.warm_time)
