#!/usr/bin/env python3
"""Render actual baseline B-rep geometry, with no synthetic physics fields."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'cad/ct35'))
from geometry import load
from build import build


def main():
    import cadquery as cq
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    from mpl_toolkits.mplot3d.art3d import Poly3DCollection
    import numpy as np
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--out',type=Path,required=True)
    args=ap.parse_args();args.out.parent.mkdir(parents=True,exist_ok=True)
    obj,d=load();fluid,head=build(obj['parameters'],d)
    cutter=cq.Workplane('XY').box(200,100,250,centered=False).translate((-100,0,-30))
    clipped_head=head.cut(cutter)
    region=cq.Workplane('XY').box(200,200,d['zh']+30,centered=False).translate((-100,-100,-30))
    head_fluid=fluid.intersect(region)
    fig=plt.figure(figsize=(14,8),dpi=160,facecolor='#101b24')
    colors=['#aeb9c1','#65aab8']
    for idx,(shape,title,subtitle) in enumerate([
        (clipped_head,'Printed head / exact cutaway','3 mm nominal wall · 1.4 mm oil bore · sharp-corner B-rep'),
        (head_fluid,'Head fluid domain / geometry only','Twin rectangular feeds · full simulated liner must extend downstream')]):
        ax=fig.add_subplot(1,2,idx+1,projection='3d',facecolor='#101b24')
        vertices,triangles=shape.val().tessellate(0.05,0.15)
        points=np.array([v.toTuple() for v in vertices]);tris=points[np.array(triangles)]
        col=np.tile(matplotlib.colors.to_rgba(colors[idx]),(len(tris),1))
        if idx==0:
            cut=np.max(np.abs(tris[:,:,1]),axis=1)<1e-5;col[cut]=matplotlib.colors.to_rgba('#d8b573')
        normals=np.cross(tris[:,1]-tris[:,0],tris[:,2]-tris[:,0])
        normals/=np.maximum(np.linalg.norm(normals,axis=1,keepdims=True),1e-30)
        light=np.array([0.3,0.5,-0.8]);light/=np.linalg.norm(light)
        col[:,:3]*=(0.6+0.4*np.abs(normals@light))[:,None]
        collection=Poly3DCollection(tris,facecolors=col,edgecolors='none',rasterized=True)
        ax.add_collection3d(collection)
        ax.set_xlim(-65,65);ax.set_ylim(-45,45);ax.set_zlim(d['zh']+5,-28)
        ax.set_box_aspect((130,90,d['zh']+33));ax.view_init(elev=18,azim=55)
        ax.set_proj_type('ortho');ax.set_axis_off()
        ax.set_title(title,color='#dbe5eb',fontsize=14,pad=15,loc='left')
        ax.text2D(0,0.94,subtitle,transform=ax.transAxes,color='#8ea6b6',fontsize=8)
    fig.suptitle('joil  /  CT-35 research baseline',x=.05,y=.98,ha='left',color='#dfc28c',fontsize=19)
    fig.text(.05,.07,'D1 70 mm   /   D2 35 mm   /   head end z = 186.747 mm   /   full liner z = 1639.247 mm',color='#c4d0d8',fontsize=10)
    fig.text(.05,.035,'Nominal CAD only. No separation result, pressure rating, fit approval or manufacturing qualification.',color='#8ea6b6',fontsize=9)
    fig.subplots_adjust(left=.04,right=.96,bottom=.1,top=.88,wspace=.12)
    fig.savefig(args.out,facecolor=fig.get_facecolor());plt.close(fig)
    manifest={'kind':'cosmetic_geometry_view_not_simulation','units':'mm','source':'baselines/ct35/genome.json',
              'source_sha256':hashlib.sha256((ROOT/'baselines/ct35/genome.json').read_bytes()).hexdigest(),
              'builder_sha256':hashlib.sha256((ROOT/'cad/ct35/build.py').read_bytes()).hexdigest(),
              'image_sha256':hashlib.sha256(args.out.read_bytes()).hexdigest(),
              'cutaway':'render derivative only: remove y>=0 from exact head; accepted solid unchanged',
              'physics_fields':[],'cadquery_version':cq.__version__}
    args.out.with_suffix('.manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps(manifest,indent=2))


if __name__=='__main__':main()
