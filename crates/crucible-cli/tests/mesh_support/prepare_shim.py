"""Test-only minimal array/STL oracle adapter for prepare.py without site packages.

Pure Python arithmetic and edge incidence; synthetic fixtures only, not CAD validation.
"""
import builtins as b
import math
import struct
import sys
import types

class Array(list):
    def __getitem__(self, key):
        if isinstance(key, tuple):
            first, *rest = key
            value = self[first]
            if not rest:
                return value
            if isinstance(first, slice):
                return array(row[tuple(rest)] if isinstance(row, Array) else row for row in value)
            return value[tuple(rest)]
        if isinstance(key, Array):
            return array(v for v, take in zip(self, key) if take)
        value = super().__getitem__(key)
        return array(value) if isinstance(value, list) else value
    def __setitem__(self, key, value):
        if isinstance(key, Array):
            for i, take in enumerate(key):
                if take: super().__setitem__(i, value)
        else: super().__setitem__(key, value)
    def __sub__(self, v): return array(x-v for x in self)
    def __abs__(self): return array(b.abs(x) for x in self)
    def __lt__(self, v): return array(x<v for x in self)
    def __eq__(self, v): return array(x==v for x in self)
    def any(self): return b.any(self)
    def sum(self): return b.sum(self)

def array(v): return Array(array(x) if isinstance(x, (list,tuple)) else x for x in v)
def f32(v): return struct.unpack('<f', struct.pack('<f',v))[0]
def spacing(v):
    bits=struct.unpack('<I',struct.pack('<f',v))[0]
    return struct.unpack('<f',struct.pack('<I',bits+1))[0]-v
np=types.ModuleType('numpy')
np.float32=f32; np.spacing=spacing; np.pi=math.pi; np.array=array
np.full=lambda n,v,dtype=None:array([v]*n)
np.abs=b.abs
np.all=lambda v,axis:array(b.all(row) for row in v)
np.allclose=lambda a,c,atol: b.abs(a-c)<=atol+1e-5*b.abs(c)
np.min=b.min
np.flatnonzero=lambda v:[i for i,t in enumerate(v) if t]
np.einsum=lambda spec, rows, normal:array(b.sum(x*y for x,y in zip(row,normal)) for row in rows)
def cross(a,c):return [a[1]*c[2]-a[2]*c[1],a[2]*c[0]-a[0]*c[2],a[0]*c[1]-a[1]*c[0]]
class Mesh:
    def __init__(self, triangles):
        self.triangles=array(triangles); self.area_faces=array([]); self.face_normals=array([])
        edges={}; self.volume=0.
        for t in triangles:
            for i in range(3):
                a=tuple(round(x,8) for x in t[i]);c=tuple(round(x,8) for x in t[(i+1)%3]);key=tuple(sorted([a,c]))
                count,w=edges.get(key,(0,0));edges[key]=(count+1,w+(1 if a<c else -1))
            normal=cross([c-a for a,c in zip(t[0],t[1])],[c-a for a,c in zip(t[0],t[2])]);norm=math.sqrt(sum(x*x for x in normal))
            self.area_faces.append(norm/2);self.face_normals.append(array([x/norm for x in normal] if norm else [0,0,0]))
            self.volume+=sum(x*y for x,y in zip(t[0],cross(t[1],t[2])))/6
        self.is_watertight=b.all(n==2 for n,w in edges.values())
        self.is_winding_consistent=b.all(w==0 for n,w in edges.values())
        self.bounds=array([[fn(p[i] for t in triangles for p in t) for i in range(3)] for fn in [min,max]])
def load_mesh(path,process=True):
    from pathlib import Path
    raw=Path(path).read_bytes()
    if len(raw)>=84 and len(raw)==84+50*struct.unpack('<I',raw[80:84])[0]:
        triangles=[[list(struct.unpack('<3f',raw[o+12+j*12:o+24+j*12])) for j in range(3)] for o in range(84,len(raw),50)]
    else:
        vertices=[[float(x) for x in line.split()[1:]] for line in raw.decode().splitlines() if line.strip().startswith('vertex ')]
        triangles=[vertices[i:i+3] for i in range(0,len(vertices),3)]
    return Mesh(triangles)
trimesh=types.ModuleType('trimesh');trimesh.load_mesh=load_mesh
sys.modules['numpy']=np;sys.modules['trimesh']=trimesh
