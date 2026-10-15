"""Independent ASCII polyMesh terminal audit, separate from CAD and snappy."""
import argparse
import json
from pathlib import Path
import re
import math


def planar_section(points, faces, plane_z):
    """Recover a closed wall/plane intersection; reject missing/branched loops."""
    import numpy as np
    edges={}
    coordinates={}
    for face in faces:
        poly=points[face]
        if poly[:,2].min()>plane_z or poly[:,2].max()<plane_z:continue
        hits={}
        for a,b in zip(poly,np.roll(poly,-1,axis=0)):
            if (a[2]-plane_z)*(b[2]-plane_z)>0 or abs(a[2]-b[2])<1e-15:continue
            fraction=(plane_z-a[2])/(b[2]-a[2])
            if not -1e-12<=fraction<=1+1e-12:continue
            xy=a[:2]+fraction*(b[:2]-a[:2])
            key=tuple(round(float(x),11) for x in xy);hits[key]=xy
        if not hits:continue
        # A triangle can touch the section at one vertex without contributing
        # a line segment. This is common when a surface ring lies on the plane.
        if len(hits)==1 and (poly[:,2].min()>=plane_z or poly[:,2].max()<=plane_z):continue
        if len(hits)!=2:raise ValueError('ambiguous or nonplanar wall-section face')
        a,b=hits
        edges.setdefault(a,set()).add(b);edges.setdefault(b,set()).add(a);coordinates.update(hits)
    if len(edges)<3 or any(len(v)!=2 for v in edges.values()):raise ValueError('bore section is not a closed unbranched loop')
    start=min(edges);loop=[start];previous=None;current=start
    while True:
        nxt=next(x for x in edges[current] if x!=previous)
        if nxt==start:break
        if nxt in loop:raise ValueError('section self-reconnects before closing')
        loop.append(nxt);previous,current=current,nxt
    if len(loop)!=len(edges):raise ValueError('multiple wall-section components')
    xy=np.array([coordinates[k] for k in loop]);following=np.roll(xy,-1,axis=0)
    area=abs(float(np.sum(xy[:,0]*following[:,1]-xy[:,1]*following[:,0])))*.5
    perimeter=float(np.linalg.norm(following-xy,axis=1).sum())
    return {'plane_z_m':plane_z,'vertices':len(xy),'area_m2':area,
            'perimeter_m':perimeter,'hydraulic_diameter_m':4*area/perimeter,
            'area_equivalent_diameter_m':2*math.sqrt(area/math.pi)}


def contents(path):
    s=Path(path).read_text()
    s=re.sub(r'/\*.*?\*/|//[^\n]*','',s,flags=re.S)
    return re.sub(r'FoamFile\s*\{[^}]*\}','',s,count=1).strip()


def audit(case, output_name='mesh-audit.json'):
    import numpy as np
    case=Path(case)
    root=case/'constant/polyMesh'
    s=contents(root/'points')
    n=int(s[:s.index('(')].strip())
    points=np.fromstring(s[s.index('(')+1:s.rindex(')')].replace('(',' ').replace(')',' '),sep=' ').reshape(-1,3)
    if len(points)!=n:raise ValueError('mesh point count mismatch')
    s=contents(root/'faces')
    faces=[np.fromstring(v,sep=' ',dtype=np.int64) for v in re.findall(r'\d+\s*\(([^()]*)\)',s)]
    expected=json.loads((case/'mesh-input.json').read_text())
    boundary=contents(root/'boundary')
    patches={}
    wall_faces=[]
    for name,body in re.findall(r'(\w+)\s*\{([^{}]*)\}',boundary):
        count=int(re.search(r'nFaces\s+(\d+)',body)[1]);start=int(re.search(r'startFace\s+(\d+)',body)[1])
        if count==0:continue
        if name=='wall':wall_faces=faces[start:start+count]
        vectors=[]
        allpts=[]
        for f in faces[start:start+count]:
            v=points[f];allpts.extend(f)
            vectors.append(np.cross(v,np.roll(v,-1,axis=0)).sum(axis=0)*.5)
        vectors=np.array(vectors);area=float(np.linalg.norm(vectors,axis=1).sum())
        rec={'faces':count,'area_m2':area,'net_area_vector_m2':vectors.sum(axis=0).tolist()}
        if name in expected['patches']:
            e=expected['patches'][name]
            rec['relative_surface_area_error']=area/e['area_m2']-1
            rec['normal_alignment']=float(np.dot(vectors.sum(axis=0),e['normal'])/area)
            if 'plane_axis' in e and 'plane_position_m' in e:
                rec['max_terminal_plane_error_m']=float(np.max(np.abs(points[allpts,e['plane_axis']]-e['plane_position_m'])))
                rec['terminal_position_pass']=rec['max_terminal_plane_error_m']<=max(1e-6,e['plane_tolerance_m'])
        patches[name]=rec
    log=(case/'mesh-2-checkMesh.log').read_text()
    cellcount=int(re.search(r'cells:\s+(\d+)',log)[1])
    volume=float(re.search(r'Total volume\s*=\s*([-+]?\d*\.?\d+(?:[eE][-+]?\d+)?)',log)[1])
    required=set(expected['patches'])
    bore=planar_section(points,wall_faces,-.0014)
    bore['relative_analytic_area_error']=bore['area_m2']/(math.pi*.0007**2)-1
    checks={'checkMesh_OK':'Mesh OK.' in log,
            'terminal_names_present':set(patches)==required|{'wall'},
            'no_background_boundary':'background' not in patches,
            'volume_relative_error_lt_2pct':abs(volume/expected['analytic_volume_m3']-1)<.02,
            'terminal_area_relative_error_lt_2pct':all(abs(patches[n].get('relative_surface_area_error',1))<.02 for n in required if n in patches),
            'terminal_normals':all(patches[n].get('normal_alignment',0)>.999 for n in required if n in patches),
            'terminal_positions':all(patches.get(n,{}).get('terminal_position_pass',False) for n in required),
            'bore_area_error_lt_2pct':abs(bore['relative_analytic_area_error'])<.02,
            'cell_cap':cellcount<=1200000}
    metrics={}
    for name,pattern,limit,minimum in [
        ('max_nonorthogonality_deg',r'Mesh non-orthogonality Max:\s*([-+\d.eE]+)',65,False),
        ('max_skewness',r'Max skewness\s*=\s*([-+\d.eE]+)',4,False),
        ('min_determinant',r'Cell determinant.*?minimum:\s*([-+\d.eE]+)',.001,True),
        ('min_face_weight',r'Face interpolation weight.*?minimum:\s*([-+\d.eE]+)',.05,True),
        ('min_volume_ratio',r'Face volume ratio.*?minimum:\s*([-+\d.eE]+)',.01,True)]:
        match=re.search(pattern,log)
        value=float(match[1]) if match else None;metrics[name]=value
        checks[name]=value is not None and (value>=limit if minimum else value<=limit)
    result={'kind':'exploratory_full_liner_mesh_audit','cells':cellcount,'volume_m3':volume,
            'relative_volume_error':volume/expected['analytic_volume_m3']-1,
            'patches':patches,'bore_section':bore,'quality_metrics':metrics,'checks':checks,'pilot_mesh_pass':all(checks.values()),
            'qualification':'geometric pilot only; no mesh convergence or wall-treatment qualification'}
    if Path(output_name).name!=output_name:raise ValueError('audit output must be a case-local filename')
    out=case/output_name
    with out.open('x') as f:json.dump(result,f,indent=2);f.write('\n')
    print(json.dumps(result,indent=2))
    if not result['pilot_mesh_pass']:raise SystemExit(1)
    return result


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--case',type=Path,required=True)
    ap.add_argument('--output-name',default='mesh-audit.json')
    a=ap.parse_args();audit(a.case,a.output_name)
