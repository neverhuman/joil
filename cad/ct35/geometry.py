"""Independent Python derivation of the public CT-35 reconstruction, mm."""
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def load(path=None):
    obj = json.loads(Path(path or ROOT / 'baselines/ct35/genome.json').read_text())
    if obj['schema_version'] != 1 or obj['family'] != 'ct35-parametric-v1':
        raise ValueError('unsupported genome')
    return obj, derive(obj['parameters'])


def derive(p):
    if any(not math.isfinite(v) for v in p.values()):
        raise ValueError('nonfinite parameter')
    if p['n_inlets'] != 2 or not 0 < p['cone_half_angle_deg'] < 89:
        raise ValueError('invalid inlet count or cone angle')
    if any(v <= 0 for k, v in p.items() if k != 'inlet_z_offset_mm') or p['inlet_z_offset_mm'] < 0:
        raise ValueError('invalid dimension')
    d2 = p['d2_mm']; d1 = d2 * p['d1_over_d2']; d3 = d2 * p['d3_over_d2']
    if not d1 > d2 > d3 > 0:
        raise ValueError('invalid diameters')
    r1, r2, r3 = d1 / 2, d2 / 2, d3 / 2
    a = math.radians(p['cone_half_angle_deg'])
    l1 = d1 * p['l1_over_d1']; lc = (r1-r2)/math.tan(a)
    l2 = d2 * p['l2_over_d2']; l3 = d3 * p['l3_over_d3']
    area = p['inlet_area_ratio'] * math.pi * r1**2 / 2
    w = math.sqrt(area / p['inlet_h_over_w']); h = w * p['inlet_h_over_w']
    ro = d2*p['do_over_d2']/2
    if (w >= r1 or p['inlet_z_offset_mm'] + h >= l1
            or min(p['inlet_length_cfd_mm'], p['inlet_length_part_mm']) <= r1
            or p['head_spigot_over_d2']*d2 >= l2 or p['plenum_radius_mm'] <= ro):
        raise ValueError('inlet, collar or plenum does not fit')
    def frustum(length, ra, rb):
        return math.pi * length * (ra**2 + ra*rb + rb**2) / 3
    def primitive(y):
        return (y*math.sqrt(max(r1*r1-y*y, 0)) + r1*r1*math.asin(y/r1))/2
    v = (math.pi*r1*r1*l1 + frustum(lc,r1,r2) + frustum(l2,r2,r3)
         + math.pi*r3*r3*l3 + 2*h*(p['inlet_length_cfd_mm']*w-primitive(r1)+primitive(r1-w))
         + math.pi*ro*ro*2*ro*p['orifice_length_over_do']
         + math.pi*p['plenum_radius_mm']**2*p['plenum_length_mm'])
    return dict(r1=r1,r2=r2,r3=r3,ro=ro,l1=l1,lc=lc,l2=l2,l3=l3,a=a,
                zc=l1+lc,zt=l1+lc+l2,ze=l1+lc+l2+l3,
                zh=l1+lc+p['head_spigot_over_d2']*d2,w=w,h=h,area=area,
                analytic_fluid_volume_mm3=v)
