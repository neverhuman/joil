#!/usr/bin/env python3
"""Deterministic mutation proposals. Geometry diversity only; no CFD performance selection."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import random

ROOT=Path(__file__).resolve().parents[2]


def proposals(seed=20261015,count=8):
    rng=random.Random(seed)
    baseline=hashlib.sha256((ROOT/'baselines/ct35/genome.json').read_bytes()).hexdigest()
    programmes=[{'schema_version':1,'family':'ct35-chamber-wall-field-v1',
                 'baseline_file_sha256':baseline,'modes':[]}]
    for i in range(count):
        modes=[]
        for j in range(1+(i%3)):
            phase=rng.uniform(-math.pi,math.pi)
            amplitude=rng.uniform(.9,1.7)/(1+(i%3))
            modes.append({'azimuthal_order':rng.randint(2,8),
                          'cos_mm':amplitude*math.cos(phase),'sin_mm':amplitude*math.sin(phase),
                          'twist_rad':rng.uniform(-2*math.pi,2*math.pi)})
        programmes.append({'schema_version':1,'family':'ct35-chamber-wall-field-v1',
                           'baseline_file_sha256':baseline,'modes':modes})
    return {'schema_version':1,'seed':seed,'operator':'smooth-wall-mode-insertion-v1',
            'status':'proposal_archive_not_performance_evolution',
            'fitness':None,'selection':'none; ordered deterministic proposals including zero-mutation baseline',
            'programmes':programmes}


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--out',type=Path,required=True)
    ap.add_argument('--count',type=int,default=8);ap.add_argument('--seed',type=int,default=20261015)
    a=ap.parse_args();a.out.parent.mkdir(parents=True,exist_ok=True)
    with a.out.open('x') as f:json.dump(proposals(a.seed,a.count),f,indent=2);f.write('\n')
