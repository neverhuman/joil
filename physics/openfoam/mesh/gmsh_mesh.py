#!/usr/bin/env python3
"""Alternative exact-STEP tetrahedral mesh, independently checked in OpenFOAM."""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import time

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'cad/ct35'))
from geometry import load


def main():
    import gmsh
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--case',type=Path,required=True)
    ap.add_argument('--step',type=Path,required=True)
    ap.add_argument('--hmax',type=float,default=.003)
    ap.add_argument('--bore-size',type=float,default=.00015)
    ap.add_argument('--algorithm',type=int,choices=[1,4,10],default=10)
    ap.add_argument('--inlet-tangent-size',type=float,help='explicit independent mesh-family challenge near inlet tangencies')
    a=ap.parse_args();case=a.case.resolve()
    if not case.is_relative_to(ROOT/'runs') or (case/'ct35.msh').exists():
        raise ValueError('fresh mesh in project run required')
    _,d=load();start=time.perf_counter()
    gmsh.initialize()
    gmsh.logger.start()
    gmsh.option.setNumber('General.NumThreads',4)
    gmsh.option.setNumber('Mesh.MaxNumThreads1D',1)
    gmsh.option.setNumber('Mesh.MaxNumThreads2D',1)
    gmsh.option.setNumber('Mesh.MaxNumThreads3D',4)
    # Ask the STEP reader to convert declared units. Post-import OCC dilation
    # changed GProp volume by ~0.9% on this compound even though bounds scaled.
    gmsh.option.setString('Geometry.OCCTargetUnit','M')
    entities=gmsh.model.occ.importShapes(str(a.step))
    volumes=[e for e in entities if e[0]==3]
    if len(volumes)!=1:raise ValueError('single exact fluid solid required')
    gmsh.model.occ.synchronize()
    volume=gmsh.model.occ.getMass(*volumes[0])
    if abs(volume/(d['analytic_fluid_volume_mm3']*1e-9)-1)>1e-7:
        raise ValueError(f'STEP import units or volume differ from frozen baseline: {volume} vs {d["analytic_fluid_volume_mm3"]*1e-9}')
    patches={n:[] for n in ['wall','inlet1','inlet2','overflow','underflow']}
    for _,tag in gmsh.model.getBoundary(volumes,oriented=False):
        x,y,z=gmsh.model.occ.getCenterOfMass(2,tag)
        b=gmsh.model.getBoundingBox(2,tag)
        name='wall'
        if abs(x+.06)<1e-8 and b[3]-b[0]<1e-6:name='inlet1'
        elif abs(x-.06)<1e-8 and b[3]-b[0]<1e-6:name='inlet2'
        elif abs(z+.0228)<1e-8 and b[5]-b[2]<1e-6:name='overflow'
        elif abs(z-d['ze']*.001)<1e-8 and b[5]-b[2]<1e-6:name='underflow'
        patches[name].append(tag)
    for n,tags in patches.items():
        if not tags or (n!='wall' and len(tags)!=1):raise ValueError(f'wrong STEP terminal {n}')
        group=gmsh.model.addPhysicalGroup(2,tags);gmsh.model.setPhysicalName(2,group,n)
    group=gmsh.model.addPhysicalGroup(3,[v[1] for v in volumes]);gmsh.model.setPhysicalName(3,group,'fluid')
    gmsh.option.setNumber('Mesh.MeshSizeMin',a.bore_size)
    gmsh.option.setNumber('Mesh.MeshSizeMax',a.hmax)
    gmsh.option.setNumber('Mesh.MeshSizeFromCurvature',32)
    gmsh.option.setNumber('Mesh.MeshSizeExtendFromBoundary',1)
    gmsh.model.mesh.setSize(gmsh.model.getEntities(0),a.hmax)
    box=gmsh.model.mesh.field.add('Box')
    for key,value in {'VIn':a.bore_size,'VOut':a.hmax,'XMin':-.001,'XMax':.001,
                      'YMin':-.001,'YMax':.001,'ZMin':-.0035,'ZMax':.001,
                      'Thickness':.002}.items():gmsh.model.mesh.field.setNumber(box,key,value)
    fields=[box]
    if a.inlet_tangent_size is not None:
        if not .0001<=a.inlet_tangent_size<=.0015:raise ValueError('invalid tangency refinement')
        for sign in [-1,1]:
            inlet=gmsh.model.mesh.field.add('Box')
            for key,value in {'VIn':a.inlet_tangent_size,'VOut':a.hmax,'XMin':-.004,'XMax':.004,
                'YMin':sign*.035-.003,'YMax':sign*.035+.003,'ZMin':-.001,'ZMax':d['h']*.001+.001,
                'Thickness':.002}.items():gmsh.model.mesh.field.setNumber(inlet,key,value)
            fields.append(inlet)
        minimum=gmsh.model.mesh.field.add('Min')
        gmsh.model.mesh.field.setNumbers(minimum,'FieldsList',fields)
        gmsh.model.mesh.field.setAsBackgroundMesh(minimum)
    else:gmsh.model.mesh.field.setAsBackgroundMesh(box)
    gmsh.option.setNumber('Mesh.Algorithm',6)
    gmsh.option.setNumber('Mesh.Algorithm3D',a.algorithm)
    gmsh.option.setNumber('Mesh.Optimize',1)
    gmsh.option.setNumber('Mesh.OptimizeThreshold',.6)
    gmsh.option.setNumber('Mesh.Smoothing',20)
    gmsh.model.mesh.generate(3)
    gmsh.model.mesh.optimize('Netgen',niter=10)
    gmsh.model.mesh.optimize('Relocate3D',niter=5)
    _,tags,_=gmsh.model.mesh.getElements(3)
    count=sum(len(t) for t in tags)
    if count>1200000:raise ValueError(f'pilot cell cap exceeded: {count}')
    gmsh.option.setNumber('Mesh.MshFileVersion',2.2)
    gmsh.option.setNumber('Mesh.Binary',0)
    gmsh.write(str(case/'ct35.msh'))
    messages='\n'.join(gmsh.logger.get())+'\n'
    (case/'gmsh.log').write_text(messages)
    manifest={'kind':'exact_step_tetrahedral_mesh','gmsh_version':gmsh.__version__,
              'step_file_sha256':hashlib.sha256(a.step.read_bytes()).hexdigest(),
              'brep_import_volume_m3':volume,'tetrahedra':count,'hmax_m':a.hmax,
              'bore_size_m':a.bore_size,'threads':4,'elapsed_seconds':time.perf_counter()-start,
              'algorithm_3d':a.algorithm,'inlet_tangent_size_m':a.inlet_tangent_size,
              'generator_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'gmsh_log_sha256':hashlib.sha256(messages.encode()).hexdigest(),
              'physical_groups':patches,'mesh_sha256':hashlib.sha256((case/'ct35.msh').read_bytes()).hexdigest()}
    (case/'gmsh-input.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps(manifest,indent=2));gmsh.finalize()


if __name__=='__main__':main()
