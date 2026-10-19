#!/usr/bin/env python3
"""Compile/audit proposal CAD and export metre scenes. No flow or strength claims."""
import subprocess
import argparse
import hashlib
import json
from pathlib import Path
import sys
import time

sys.path.insert(0,str(Path(__file__).resolve().parent))
from wall_field import compile_program


def main():
    import cadquery as cq
    import trimesh
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--archive',type=Path,required=True);ap.add_argument('--out',type=Path,required=True)
    a=ap.parse_args();a.out.mkdir(parents=True,exist_ok=False)
    archive=json.loads(a.archive.read_text());reports=[]
    for index,program in enumerate(archive['programmes']):
        dest=a.out/f'proposal-{index:03d}';dest.mkdir();start=time.perf_counter()
        report={'index':index,'status':'kernel_not_run','programme':program,'fitness':None,'artifacts':{}}
        (dest/'programme.json').write_text(json.dumps(program,indent=2)+'\n')
        try:
            fluid,head,details=compile_program(program)
            report.update(details)
            for name,shape in [('fluid',fluid),('head',head)]:
                if not shape.isValid() or len(shape.Solids())!=1 or shape.Volume()<=0:
                    raise ValueError(f'{name} invalid or disconnected')
                cq.exporters.export(shape,str(dest/f'{name}_mm.step'))
                cq.exporters.export(shape.scale(.001),str(dest/f'{name}_preview_m.stl'),tolerance=.00002,angularTolerance=.1)
                mesh=trimesh.load_mesh(dest/f'{name}_preview_m.stl',process=True)
                if not mesh.is_watertight or not mesh.is_winding_consistent or mesh.volume<=0:
                    raise ValueError(f'{name} surface not outward and closed')
                mesh.visual.face_colors=[35,185,195,255] if name=='fluid' else [160,175,185,255]
                mesh.export(dest/f'{name}_m.glb')
                imported=cq.importers.importStep(str(dest/f'{name}_mm.step')).val()
                if not imported.isValid() or abs(imported.Volume()/shape.Volume()-1)>1e-7:
                    raise ValueError('STEP round-trip changed the subject')
                report[name]={'volume_mm3':shape.Volume(),'stl_volume_m3':float(mesh.volume),
                              'stl_relative_volume_error':float(mesh.volume/(shape.Volume()*1e-9)-1),
                              'triangles':len(mesh.faces),'valid_single_solid':True}
            report['status']='geometry_audited_proposal_not_performance_qualified'
        except (OSError, ValueError, KeyError, TypeError, RuntimeError, subprocess.SubprocessError) as e:
            report.update(status='geometry_or_kernel_failure',error=str(e))
        report['elapsed_seconds']=time.perf_counter()-start
        for f in dest.iterdir():
            if f.is_file():report['artifacts'][f.name]={'bytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()}
        (dest/'report.json').write_text(json.dumps(report,indent=2)+'\n')
        reports.append(report)
        print(index,report['status'],f'{report["elapsed_seconds"]:.1f}s',flush=True)
    (a.out/'archive-report.json').write_text(json.dumps({'reports':reports,'physics_executed':False,
        'performance_fitness':None,'manufacturing_and_fit':'unqualified'},indent=2)+'\n')
    if any(r['status']=='geometry_or_kernel_failure' for r in reports):raise SystemExit(1)


if __name__=='__main__':main()
