#!/usr/bin/env python3
"""Build/audit exact CT-35 research CAD. No CFD-ready boundary mesh is claimed."""
import argparse
import hashlib
import json
import math
import re
from pathlib import Path
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
from geometry import load


def build(p, d):
    import cadquery as cq
    def revolved(profile):
        return cq.Workplane('XZ').polyline(profile).close().revolve(360,(0,0,0),(0,1,0))
    def ducts(length, pad=0):
        first = (cq.Workplane('XY').box(length,d['w']+2*pad,d['h']+2*pad,centered=False)
                 .translate((-length,d['r1']-d['w']-pad,p['inlet_z_offset_mm']-pad)))
        return first, first.rotate((0,0,0),(0,0,1),180)
    def radius(z):
        return d['r2']+(d['r3']-d['r2'])*(z-d['zc'])/d['l2']
    fluid = revolved([(0,0),(d['r1'],0),(d['r1'],d['l1']),(d['r2'],d['zc']),
                      (d['r3'],d['zt']),(d['r3'],d['ze']),(0,d['ze'])])
    for duct in ducts(p['inlet_length_cfd_mm']):
        fluid = fluid.union(duct,tol=1e-4)
    bore_length = 2*d['ro']*p['orifice_length_over_do']
    bore = cq.Workplane('XY').workplane(offset=-bore_length-0.01).circle(d['ro']).extrude(bore_length+0.02)
    plenum = (cq.Workplane('XY').workplane(offset=-bore_length-p['plenum_length_mm'])
              .circle(p['plenum_radius_mm']).extrude(p['plenum_length_mm']))
    fluid = fluid.union(bore).union(plenum)

    t=p['wall_thickness_mm']; a=d['a']; b=math.atan((d['r2']-d['r3'])/d['l2'])
    zjoin=d['l1']+t*(1/math.cos(a)-1)/math.tan(a)
    zsecond=(d['r1']-d['r2']+d['l1']*math.tan(a)-d['zc']*math.tan(b)
             +t/math.cos(a)-t/math.cos(b))/(math.tan(a)-math.tan(b))
    outer = revolved([(0,-t),(d['r1']+t,-t),(d['r1']+t,zjoin),
                      (d['r2']-(zsecond-d['zc'])*math.tan(b)+t/math.cos(b),zsecond),
                      (radius(d['zh'])+t/math.cos(b),d['zh']),(0,d['zh'])])
    for boss in ducts(p['inlet_length_part_mm'],pad=t):
        outer=outer.union(boss,tol=1e-4)
    inner=revolved([(0,0),(d['r1'],0),(d['r1'],d['l1']),(d['r2'],d['zc']),
                    (radius(d['zh']+5),d['zh']+5),(0,d['zh']+5)])
    head=outer.cut(inner)
    for duct in ducts(p['inlet_length_part_mm']+10):
        head=head.cut(duct,tol=1e-4)
    hole=cq.Workplane('XY').workplane(offset=-t-1).circle(d['ro']).extrude(t+2)
    return fluid,head.cut(hole)


def main():
    import cadquery as cq
    import trimesh
    start=time.perf_counter()
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--genome',type=Path)
    ap.add_argument('--out',type=Path,required=True)
    args=ap.parse_args();args.out.mkdir(parents=True,exist_ok=True)
    if any(args.out.iterdir()):
        raise ValueError('output directory must be empty; preserve prior run and use a fresh output path')
    genome,d=load(args.genome);p=genome['parameters']
    # This builder qualifies only this public baseline. Freeform CAD is P02 work.
    baseline,_=load()
    if genome!=baseline:
        raise ValueError('only frozen baseline CAD is qualified by this bootstrap')
    fluid,head=build(p,d)
    report={'status':'cad_audited_not_cfd_or_manufacturing_qualified','cadquery_version':cq.__version__,
            'builder_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            'geometry_derivation_sha256':hashlib.sha256((Path(__file__).with_name('geometry.py')).read_bytes()).hexdigest(),
            'python_version':sys.version,
            'genome_file_sha256':hashlib.sha256((args.genome or Path(__file__).resolve().parents[2]/'baselines/ct35/genome.json').read_bytes()).hexdigest(),
            'dimensions_mm':d,'tessellation_chord_mm':0.01,'tessellation_angular_rad':0.1,
            'hardware_bore_length_mm':p['wall_thickness_mm'],
            'cfd_bore_length_mm':p['orifice_length_over_do']*2*d['ro'],
            'limitations':['CFD extensions are not hardware','head B-rep corners differ from SDF offset preview',
                           'labelled conformal CFD surface and minimum-wall/print/fit gates pending',
                           'no pressure rating, CFD or physical validation'],
            'shapes':{},'artifacts':{}}
    checks=[]
    for name,work in [('fluid',fluid),('head',head)]:
        shape=work.val();step=args.out/f'{name}_mm.step';stl=args.out/f'{name}_preview_mm.stl'
        cq.exporters.export(work,str(step));cq.exporters.export(work,str(stl),tolerance=0.01,angularTolerance=0.1)
        step_text=re.sub(r'\s+','',step.read_text())
        step_declares_mm='SI_UNIT(.MILLI.,.METRE.)' in step_text
        reimport=cq.importers.importStep(str(step)).val()
        mesh=trimesh.load_mesh(stl,process=True)
        reldiff=abs(reimport.Volume()/shape.Volume()-1)
        checks += [shape.isValid(),len(shape.Solids())==1,reimport.isValid(),len(reimport.Solids())==1,
                   reldiff<1e-8,mesh.is_watertight,mesh.is_winding_consistent,mesh.volume>0,step_declares_mm]
        record={'valid':shape.isValid(),'solids':len(shape.Solids()),'volume_mm3':shape.Volume(),
                'reimport_relative_volume_error':reldiff,'preview_watertight':mesh.is_watertight,
                'preview_winding_consistent':mesh.is_winding_consistent,'preview_volume_mm3':mesh.volume,
                'preview_triangles':len(mesh.faces),'bounds_mm':mesh.bounds.tolist(),
                'step_declared_length_unit':'millimetre' if step_declares_mm else 'UNEXPECTED'}
        report['shapes'][name]=record
        scene=mesh.copy();scene.apply_scale(0.001)
        scene.visual.face_colors=[45,185,195,255] if name=='fluid' else [155,168,177,255]
        scene.export(args.out/f'{name}_m.glb')
        # STEP stays canonical mm. A scaled shape written by the default STEP writer
        # would still declare mm, so numerical coordinate scaling is not SI metadata conversion.
        metric_stl=args.out/f'{name}_preview_m.stl'
        cq.exporters.export(shape.scale(0.001),str(metric_stl),tolerance=0.00001,angularTolerance=0.1)
        metric_mesh=trimesh.load_mesh(metric_stl,process=True)
        import numpy as np
        scale_ok=bool(np.allclose(metric_mesh.bounds,mesh.bounds*0.001,rtol=2e-6,atol=1e-9))
        record['metre_surface_bounds_verified']=scale_ok
        record['metre_surface_units']='m; STL is unitless, this manifest supplies units'
        checks += [scale_ok,metric_mesh.is_watertight,metric_mesh.is_winding_consistent,metric_mesh.volume>0]
    vf=report['shapes']['fluid']['volume_mm3'];va=d['analytic_fluid_volume_mm3']
    report['analytic_fluid_relative_error']=vf/va-1
    checks.append(abs(vf/va-1)<1e-6)
    report['head_mass_g_nominal_density']={'Ti-6Al-4V':report['shapes']['head']['volume_mm3']*0.00443,
                                          'CP-Ti-Grade-2':report['shapes']['head']['volume_mm3']*0.00451}
    patches={'status':'terminal_face_definitions_only_not_labelled_surface','units':'mm',
             'inlet1':{'plane_x':-p['inlet_length_cfd_mm'],'normal':[-1,0,0],'area_mm2':d['area']},
             'inlet2':{'plane_x':p['inlet_length_cfd_mm'],'normal':[1,0,0],'area_mm2':d['area']},
             'overflow':{'plane_z':-2*d['ro']*p['orifice_length_over_do']-p['plenum_length_mm'],'normal':[0,0,-1],
                         'area_mm2':math.pi*p['plenum_radius_mm']**2},
             'underflow':{'plane_z':d['ze'],'normal':[0,0,1],'area_mm2':math.pi*d['r3']**2}}
    # Measure each terminal from the B-rep independently of its declared area.
    faces=fluid.val().Faces()
    for name,contract in list(patches.items()):
        if not isinstance(contract,dict):continue
        axis='x' if 'plane_x' in contract else 'z';plane=contract.get('plane_x',contract.get('plane_z'))
        selected=[f for f in faces if f.geomType()=='PLANE' and abs(getattr(f.Center(),axis)-plane)<1e-7
                  and abs(getattr(f.normalAt(),axis))>0.999999]
        area=sum(f.Area() for f in selected)
        contract['measured_area_mm2']=area;contract['matched_faces']=len(selected)
        checks.append(len(selected)==1 and abs(area/contract['area_mm2']-1)<1e-8)
        if len(selected)==1:
            normal=selected[0].normalAt().toTuple()
            contract['measured_normal']=normal
            checks.append(sum(a*b for a,b in zip(normal,contract['normal']))>0.999999)
    (args.out/'patch_contract.json').write_text(json.dumps(patches,indent=2)+'\n')
    for f in sorted(args.out.iterdir()):
        if f.name!='cad_report.json' and f.is_file():
            report['artifacts'][f.name]={'bytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()}
    report['elapsed_seconds']=time.perf_counter()-start
    report['bootstrap_checks_pass']=all(checks)
    (args.out/'cad_report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))
    return 0 if all(checks) else 1


if __name__=='__main__':
    raise SystemExit(main())
