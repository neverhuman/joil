#!/usr/bin/env python3
"""Plot sections through the actual exported CAD surfaces, with no performance coloring."""
import argparse
import hashlib
import json
from pathlib import Path


def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def render(archive,out):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    import numpy as np
    import trimesh
    archive=Path(archive);out=Path(out)
    report=json.loads((archive/'archive-report.json').read_text())
    records=report['reports']
    if len(records)!=9 or any(r['status']!='geometry_audited_proposal_not_performance_qualified' for r in records):
        raise ValueError('this plate requires the complete audited nine-proposal archive')
    fig,axes=plt.subplots(3,3,figsize=(12,12),facecolor='#101b24')
    identity=[]
    for ax,rec in zip(axes.flat,records):
        directory=archive/f'proposal-{rec["index"]:03d}';surface=directory/'fluid_preview_m.stl'
        if sha(surface)!=rec['artifacts'][surface.name]['sha256']:raise ValueError('CAD surface changed')
        mesh=trimesh.load_mesh(surface,process=False)
        section=mesh.section(plane_origin=[0,0,.0425],plane_normal=[0,0,1])
        if section is None:raise ValueError('missing chamber section')
        for curve in section.discrete:ax.plot(curve[:,0]*1000,curve[:,1]*1000,color='#75cbd0',lw=2)
        theta=np.linspace(0,2*np.pi,300)
        ax.plot(35*np.cos(theta),35*np.sin(theta),'--',color='#d1b57d',lw=.8,alpha=.8)
        ax.plot(38*np.cos(theta),38*np.sin(theta),color='#78909f',lw=1)
        ax.set_aspect('equal');ax.set(xlim=(-42,42),ylim=(-42,42),xticks=[-35,0,35],yticks=[-35,0,35])
        ax.set_facecolor('#101b24');ax.tick_params(colors='#a2b5c2',labelsize=8)
        for spine in ax.spines.values():spine.set_color('#293b48')
        clearance=rec['minimum_wall_clearance_mm']
        ax.set_title(('000 / exact baseline' if rec['index']==0 else f'{rec["index"]:03d} / wall-field proposal')+f'\nminimum clearance {clearance:.3f} mm',color='#dde8ec',fontsize=10)
        identity.append({'index':rec['index'],'surface_sha256':sha(surface),'report_sha256':sha(directory/'report.json')})
    fig.suptitle('joil  /  flexible chamber proposals',color='#dfc28c',fontsize=20,x=.065,ha='left')
    fig.text(.065,.935,'Actual CAD surface sections at z = 42.5 mm · axes in mm · common scale',color='#a2b5c2',fontsize=11)
    fig.text(.065,.035,'Cyan: exported fluid surface   /   dashed gold: CT-35 baseline   /   gray: outer wall',color='#cbd7dd',fontsize=10)
    fig.text(.065,.015,'Geometry checks passed. Flow, separation, manufacturing, structural suitability and superiority remain unqualified.',color='#a2b5c2',fontsize=9)
    fig.subplots_adjust(top=.87,bottom=.085,left=.065,right=.965,hspace=.36,wspace=.2)
    out.parent.mkdir(parents=True,exist_ok=True)
    if out.exists():raise ValueError('new immutable output path required')
    fig.savefig(out,dpi=160,facecolor=fig.get_facecolor());plt.close(fig)
    manifest={'kind':'actual_cad_surface_section_not_simulation','plane_z_m':.0425,'axis_units':'mm',
        'sources':identity,'extractor_sha256':sha(__file__),'image_sha256':sha(out),'physics_fields':[]}
    out.with_suffix('.manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps(manifest,indent=2))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--archive',type=Path,required=True)
    p.add_argument('--out',type=Path,required=True);a=p.parse_args();render(a.archive,a.out)
