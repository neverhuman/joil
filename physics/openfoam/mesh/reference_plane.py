#!/usr/bin/env python3
"""Recover exact original outlet faces after extrusion, without a planar approximation."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(Path(__file__).resolve().parent))
from audit import contents


def recover(case, source):
    import numpy as np
    case=Path(case).resolve();source=Path(source).resolve()
    if not case.is_relative_to(ROOT/'runs') or not source.is_relative_to(ROOT/'runs'):
        raise ValueError('project runs only')
    extension=json.loads((case/'domain-extension.json').read_text())
    if extension['source_case']!=source.name:raise ValueError('extension/source mismatch')
    def mesh(folder):
        s=contents(folder/'points');points=np.fromstring(s[s.index('(')+1:s.rindex(')')].replace('(',' ').replace(')',' '),sep=' ').reshape(-1,3)
        s=contents(folder/'faces');faces=[np.fromstring(x,sep=' ',dtype=np.int64) for x in re.findall(r'\d+\s*\(([^()]*)\)',s)]
        return points,faces
    src=source/'constant/polyMesh';dst=case/'constant/polyMesh'
    sp,sf=mesh(src);dp,df=mesh(dst)
    body=re.search(r'\boverflow\s*\{([^{}]*)\}',contents(src/'boundary'))[1]
    start=int(re.search(r'startFace\s+(\d+)',body)[1]);count=int(re.search(r'nFaces\s+(\d+)',body)[1])
    def key(points,face):return tuple(sorted(tuple(v) for v in points[face]))
    wanted={key(sp,f) for f in sf[start:start+count]}
    ids=[i for i,f in enumerate(df) if key(dp,f) in wanted]
    neighbours=contents(dst/'neighbour');n_internal=int(neighbours[:neighbours.index('(')].strip())
    if len(wanted)!=count or len(ids)!=count or any(i>=n_internal for i in ids):
        raise ValueError('original outlet faces not recovered uniquely as internal faces')
    out=dst/'sets/deviceOverflow';out.parent.mkdir(exist_ok=True)
    with out.open('x') as f:
        f.write('FoamFile {format ascii; class faceSet; object deviceOverflow;}\n'+str(count)+'\n(\n'+'\n'.join(map(str,ids))+'\n)\n')
    receipt={'kind':'exact_original_outlet_internal_face_identity','faces':count,
        'source_mesh_face_sha256':extension['source_mesh_sha256']['faces'],
        'new_mesh_face_sha256':hashlib.sha256((dst/'faces').read_bytes()).hexdigest(),
        'face_set_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),
        'generator_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'coordinate_matching':'exact original vertex coordinates and polygon connectivity, order-independent',
        'outward_direction':[0,0,-1]}
    with (case/'device-plane.json').open('x') as f:json.dump(receipt,f,indent=2);f.write('\n')
    print(json.dumps(receipt,indent=2))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--case',type=Path,required=True)
    p.add_argument('--source',type=Path,required=True);a=p.parse_args();recover(a.case,a.source)
