#!/usr/bin/env python3
"""Check the exact local CPU image and archive native capability/version receipts."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[1]
ENV=ROOT/'physics/openfoam/environment'
sys.path.insert(0,str(ENV))
from runtime import run


def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def doctor(case):
    lock=json.loads((ENV/'lock.json').read_text())
    for name,key in [('Dockerfile','dockerfile_sha256'),('openfoam.asc','repository_key_sha256')]:
        if sha(ENV/name)!=lock[key]:raise ValueError('environment recipe/key changed from tested lock')
    info=json.loads(subprocess.check_output(['docker','image','inspect',lock['image_id']],text=True))[0]
    if info['Id']!=lock['image_id'] or info['Architecture']!='amd64' or info['Os']!='linux':
        raise ValueError('wrong native image identity/platform')
    case=Path(case).resolve()
    if not case.is_relative_to(ROOT/'runs') or case.exists():raise ValueError('fresh project run required')
    case.mkdir(parents=True)
    commands=[]
    for name,argv in [('solver',['foamRun','-help']),('compiler',['g++','--version']),
                      ('mpi',['mpirun','--version']),('packages',['dpkg-query','-W','-f=${Package}\t${Version}\n'])]:
        commands.append(run(case,argv,case/f'{name}.log',timeout=60))
    solver=(case/'solver.log').read_text()
    if lock['build'] not in solver:raise ValueError('native OpenFOAM build differs from lock')
    versions={n:(case/f'{n}.log').read_text().splitlines()[0] for n in ['compiler','mpi']}
    packages=(case/'packages.log').read_text()
    found=re.search(r'^openfoam14\t([^\n]+)$',packages,re.M)
    if not found or found[1]!=lock['package_version']:raise ValueError('installed solver package mismatch')
    result={'kind':'executed_CPU_environment_capability_not_physics_validation','passed':True,
        'image_id':lock['image_id'],'build':lock['build'],'platform':'linux/amd64','precision':'double',
        'versions':versions,'package_manifest_sha256':sha(case/'packages.log'),
        'lock_sha256':sha(ENV/'lock.json'),'doctor_source_sha256':sha(__file__),'commands':commands,
        'qualified_for':['native binary startup and version identity only'],
        'not_qualified':['turbulent carrier','oil transport','MPI scaling/restart','GPU','hydrocyclone performance'],
        'rebuild_limitation':'transitive apt dependency snapshot not pinned; revalidate a new build identity'}
    with (case/'capability.json').open('x') as f:json.dump(result,f,indent=2);f.write('\n')
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--case',type=Path,required=True)
    a=ap.parse_args();doctor(a.case)
