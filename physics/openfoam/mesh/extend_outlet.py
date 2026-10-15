#!/usr/bin/env python3
"""Append a declared numerical outlet pipe; never mutate the baseline genotype."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sys

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'physics/openfoam/environment'))
sys.path.insert(0,str(Path(__file__).resolve().parent))
from runtime import run
from audit import contents


def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def extend(case, source, length=.12, layers=60):
    import numpy as np
    import re
    case=Path(case).resolve();source=Path(source).resolve()
    if not case.is_relative_to(ROOT/'runs') or not source.is_relative_to(ROOT/'runs'):
        raise ValueError('project runs only')
    if case.exists():raise ValueError('fresh case required')
    if not 0<length<=.3 or not 2<=layers<=300:raise ValueError('extension outside pilot budget')
    if (source/'domain-extension.json').exists():raise ValueError('extend the original domain only')
    source_audit=json.loads((source/'mesh-audit.json').read_text())
    area=source_audit['patches']['overflow']['area_m2']
    case.mkdir(parents=True);(case/'system').mkdir()
    shutil.copytree(source/'constant/polyMesh',case/'constant/polyMesh')
    shutil.copy2(source/'system/controlDict',case/'system/controlDict')
    # Remove fields/functions from the source control dictionary: this is meshing only.
    (case/'system/controlDict').write_text('FoamFile {format ascii; class dictionary; object controlDict;}\napplication extrudeMesh; startFrom startTime; startTime 0; stopAt endTime; endTime 1; deltaT 1; writeControl timeStep; writeInterval 1;\n')
    dictionary=f'''FoamFile {{format ascii; class dictionary; object extrudeMeshDict;}}
constructFrom mesh;
sourceCase ".";
sourcePatches (overflow);
flipNormals false;
zoneNames (numericalOverflowExtension);
extrudeModel
{{
    type linearDirection;
    nLayers {layers};
    expansionRatio 1;
    direction (0 0 -1);
    thickness {length:.15g};
}}
mergeFaces false;
mergeTol 0;
'''
    (case/'system/extrudeMeshDict').write_text(dictionary)
    inputs={n:sha(source/'constant/polyMesh'/n) for n in ['points','faces','owner','neighbour','boundary']}
    manifest=json.loads((source/'mesh-input.json').read_text())
    original_volume=manifest['analytic_volume_m3']
    # The added pipe uses the existing faceted outlet section, not a new circle.
    manifest['analytic_volume_m3']=original_volume+area*length
    if 'plane_position_m' in manifest['patches']['overflow']:
        manifest['patches']['overflow']['plane_position_m']-=length
    manifest['numerical_domain_extension']={'patch':'overflow','length_m':length,'layers':layers,
        'original_analytic_volume_m3':original_volume,'extruded_polygon_area_m2':area,
        'baseline_geometry_changed':False,'original_terminal_becomes_internal_reference_plane':True}
    (case/'mesh-input.json').write_text(json.dumps(manifest,indent=2)+'\n')
    run(case,['extrudeMesh'],case/'extension-0-extrudeMesh.log',timeout=300)
    # Proof independent of the extrusion utility: all original points remain, unchanged.
    def points(path):
        s=contents(path);return np.fromstring(s[s.index('(')+1:s.rindex(')')].replace('(',' ').replace(')',' '),sep=' ').reshape(-1,3)
    before=points(source/'constant/polyMesh/points');after=points(case/'constant/polyMesh/points')
    lookup={tuple(p) for p in after}
    if any(tuple(p) not in lookup for p in before):raise ValueError('extrusion changed an original mesh point')
    receipt={'kind':'explicit_numerical_outlet_extension','source_case':source.name,
        'source_mesh_sha256':inputs,'source_mesh_audit_sha256':sha(source/'mesh-audit.json'),
        'length_m':length,'layers':layers,'original_points_preserved':True,
        'baseline_geometry_changed':False,'source_audit_pass':source_audit['pilot_mesh_pass'],
        'new_mesh_sha256':{n:sha(case/'constant/polyMesh'/n) for n in inputs},
        'generator_sha256':sha(__file__),'dictionary_sha256':sha(case/'system/extrudeMeshDict'),
        'device_and_full_domain_energy_must_be_distinguished':True,
        'campaign_admission':False}
    (case/'domain-extension.json').write_text(json.dumps(receipt,indent=2)+'\n')
    run(case,['checkMesh','-allTopology','-allGeometry'],case/'mesh-2-checkMesh.log',timeout=300)


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--case',type=Path,required=True)
    ap.add_argument('--source',type=Path,required=True);ap.add_argument('--length',type=float,default=.12)
    ap.add_argument('--layers',type=int,default=60);a=ap.parse_args();extend(a.case,a.source,a.length,a.layers)
