#!/usr/bin/env python3
"""Independently audit all supplied STL subjects and duplicate archives. Private outputs."""
import hashlib
import io
import json
from pathlib import Path
import time
import zipfile


def audit(data):
    import trimesh
    mesh=trimesh.load_mesh(io.BytesIO(data),file_type='stl',process=True)
    mesh.merge_vertices()
    return {'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),
            'triangles':len(mesh.faces),'vertices':len(mesh.vertices),
            'watertight':bool(mesh.is_watertight),'winding_consistent':bool(mesh.is_winding_consistent),
            'signed_volume_mm3':float(mesh.volume),'bounds_mm':mesh.bounds.tolist(),
            'euler_number':int(mesh.euler_number)}


def main():
    root=Path(__file__).resolve().parents[1];src=root/'.private/source';start=time.perf_counter()
    report={'status':'independent_supplied_geometry_audit_not_cfd_validation','meshes':{},'archives':{}}
    for name in ['ct35_fluid_domain.stl','ct35_head_cartridge.stl']:
        result=audit((src/name).read_bytes());report['meshes'][name]=result
        print(name,json.dumps(result),flush=True)
    zip_names=['ct35_baseline_rev1.zip','ct35_baseline_rev1 (1).zip']
    for name in zip_names:
        data=(src/name).read_bytes()
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            report['archives'][name]={'sha256':hashlib.sha256(data).hexdigest(),
                'members':[{'name':i.filename,'bytes':i.file_size,'sha256':hashlib.sha256(z.read(i)).hexdigest()} for i in z.infolist()]}
            if name==zip_names[0]:
                for info in z.infolist():
                    if info.filename.endswith('.stl'):
                        result=audit(z.read(info));report['meshes'][info.filename]=result
                        print(info.filename,json.dumps(result),flush=True)
    report['duplicate_archives_identical']=report['archives'][zip_names[0]]['sha256']==report['archives'][zip_names[1]]['sha256']
    reference=json.loads((src/'ct35_baseline_params.json').read_text())
    expected=reference['fluid_domain_checks']['analytic_volume_mm3']
    report['latest_fluid_relative_volume_error']=report['meshes']['ct35_fluid_domain.stl']['signed_volume_mm3']/expected-1
    report['all_surfaces_closed_and_oriented']=all(m['watertight'] and m['winding_consistent'] and m['signed_volume_mm3']>0 for m in report['meshes'].values())
    report['elapsed_seconds']=time.perf_counter()-start
    (root/'.private/supplied-mesh-audit.json').write_text(json.dumps(report,indent=2)+'\n')
    print('closed/oriented',report['all_surfaces_closed_and_oriented'],'elapsed',report['elapsed_seconds'])
    return 0 if report['all_surfaces_closed_and_oriented'] and abs(report['latest_fluid_relative_volume_error'])<0.001 else 1


if __name__=='__main__':raise SystemExit(main())
