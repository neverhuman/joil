"""Versioned smooth non-axisymmetric chamber programme with frozen CT-35 interfaces."""
import hashlib
import json
import math
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'cad/ct35'))
from geometry import load
from build import build


def validate(program):
    if set(program)!={'schema_version','family','baseline_file_sha256','modes'}:
        raise ValueError('unknown or missing design-program keys')
    if program['schema_version']!=1 or program['family']!='ct35-chamber-wall-field-v1':
        raise ValueError('unknown programme schema or family')
    expected=hashlib.sha256((ROOT/'baselines/ct35/genome.json').read_bytes()).hexdigest()
    if program['baseline_file_sha256']!=expected:raise ValueError('baseline/interface identity mismatch')
    if not isinstance(program['modes'],list) or len(program['modes'])>8:
        raise ValueError('initial compiler budget: at most eight smooth wall modes')
    amplitude=0.
    for mode in program['modes']:
        if set(mode)!={'azimuthal_order','cos_mm','sin_mm','twist_rad'}:raise ValueError('unknown wall-mode key')
        if type(mode['azimuthal_order']) is not int or not 2<=mode['azimuthal_order']<=8:
            raise ValueError('unresolved or unsupported azimuthal mode')
        if any(type(mode[k]) not in [int,float] or not math.isfinite(mode[k]) for k in ['cos_mm','sin_mm','twist_rad']):
            raise ValueError('nonfinite wall coefficient')
        if abs(mode['twist_rad'])>2*math.pi:raise ValueError('initial axial resolution budget exceeded')
        amplitude+=math.hypot(mode['cos_mm'],mode['sin_mm'])
    if amplitude>1.8+1e-12:raise ValueError('wall-clearance proposal budget exceeded')


def compile_program(program):
    import cadquery as cq
    validate(program)
    genome,d=load();p=genome['parameters']
    baseline_fluid,baseline_head=build(p,d)
    fluid,head=baseline_fluid.val(),baseline_head.val()
    active=[m for m in program['modes'] if m['cos_mm']!=0 or m['sin_mm']!=0]
    if not active:return fluid,head,{'zero_mutation_exact_baseline_branch':True,'minimum_wall_clearance_mm':3.}
    z0,z1=20.,65.
    wires=[]
    for j in range(13):
        s=j/12;z=z0+(z1-z0)*s
        if j in [0,12]:
            wire=cq.Wire.makeCircle(d['r1'],cq.Vector(0,0,z),cq.Vector(0,0,1))
        else:
            pts=[]
            for i in range(128):
                theta=2*math.pi*i/128
                delta=sum(m['cos_mm']*math.cos(m['azimuthal_order']*theta+m['twist_rad']*s)
                          +m['sin_mm']*math.sin(m['azimuthal_order']*theta+m['twist_rad']*s) for m in active)
                r=d['r1']+math.sin(math.pi*s)**2*delta
                pts.append(cq.Vector(r*math.cos(theta),r*math.sin(theta),z))
            wire=cq.Wire.assembleEdges([cq.Edge.makeSpline(pts,periodic=True)])
        wires.append(wire)
    chamber=cq.Solid.makeLoft(wires,ruled=False)
    old=cq.Solid.makeCylinder(d['r1']+.1,z1-z0,cq.Vector(0,0,z0))
    fluid=fluid.cut(old).fuse(chamber).clean()
    refill=cq.Solid.makeCylinder(d['r1'],z1-z0,cq.Vector(0,0,z0))
    head=head.fuse(refill).cut(fluid).clean()
    outer=cq.Solid.makeCylinder(d['r1']+p['wall_thickness_mm'],z1-z0,cq.Vector(0,0,z0))
    side=next(f for f in outer.Faces() if f.geomType()=='CYLINDER')
    clearance=chamber.distance(side)
    frozen_tail=cq.Solid.makeBox(200,200,2000,cq.Vector(-100,-100,d['zh']-17.5))
    a=fluid.intersect(frozen_tail);b=baseline_fluid.val().intersect(frozen_tail)
    difference=a.cut(b).Volume()+b.cut(a).Volume()
    report={'zero_mutation_exact_baseline_branch':False,'minimum_wall_clearance_mm':clearance,
            'frozen_collar_and_tail_symmetric_difference_mm3':difference,
            'active_modes':len(active),'axial_band_mm':[z0,z1],
            'loft_sections':13,'periodic_spline_samples_per_section':128,
            'compiler_scope':'smooth chamber field only; inlet graph and guide-vane compilers remain pending'}
    if clearance<1. or difference>1e-5:raise ValueError('minimum wall or fixed-tail contract violated')
    return fluid,head,report
