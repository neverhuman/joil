#!/usr/bin/env python3
"""Render actual saved OpenFOAM p/U fields; no invented flow or averaging."""
import argparse
import hashlib
import json
from pathlib import Path


def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def render(case,time,out):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    import numpy as np
    import vtk
    from vtk.util.numpy_support import vtk_to_numpy, numpy_to_vtk
    case=Path(case).resolve();out=Path(out)
    if out.exists():raise ValueError('fresh output folder required')
    field=case/f'{time:g}'
    for name in ['U','p']:
        if not (field/name).exists():raise ValueError('explicit reconstructed saved p/U required')
    marker=case/'visualization.foam';marker.touch(exist_ok=True)
    reader=vtk.vtkOpenFOAMReader();reader.SetFileName(str(marker));reader.SetSkipZeroTime(True)
    reader.UpdateInformation();reader.DisableAllPatchArrays();reader.SetPatchArrayStatus('internalMesh',1)
    reader.EnableAllCellArrays();reader.SetCreateCellToPoint(True);reader.SetTimeValue(time);reader.Update()
    if reader.GetTimeValue()!=time:raise ValueError('reader selected wrong saved state')
    iterator=reader.GetOutput().NewIterator();iterator.InitTraversal();grids=[]
    while not iterator.IsDoneWithTraversal():
        obj=iterator.GetCurrentDataObject()
        if isinstance(obj,vtk.vtkUnstructuredGrid) and obj.GetNumberOfCells():grids.append(obj)
        iterator.GoToNextItem()
    if len(grids)!=1:raise ValueError(f'expected one full internal volume mesh, got {len(grids)}')
    grid=grids[0]
    if not grid.GetPointData().GetArray('U') or not grid.GetPointData().GetArray('p'):
        raise ValueError('missing actual field arrays')
    def probe(xyz):
        points=vtk.vtkPoints();points.SetData(numpy_to_vtk(xyz,deep=True))
        data=vtk.vtkPolyData();data.SetPoints(points)
        p=vtk.vtkProbeFilter();p.SetInputData(data);p.SetSourceData(grid);p.Update()
        result=p.GetOutput();mask=vtk_to_numpy(result.GetPointData().GetArray('vtkValidPointMask')).astype(bool)
        arrays={n:vtk_to_numpy(result.GetPointData().GetArray(n)).copy() for n in ['U','p']}
        for values in arrays.values():values[~mask]=np.nan
        return arrays
    z,x=np.meshgrid(np.linspace(-.023,.19,400),np.linspace(-.04,.04,200))
    head=probe(np.column_stack([x.ravel(),np.zeros(x.size),z.ravel()]))
    cx,cy=np.meshgrid(np.linspace(-.036,.036,181),np.linspace(-.036,.036,181))
    chamber=probe(np.column_stack([cx.ravel(),cy.ravel(),np.full(cx.size,.04)]))
    fig,axes=plt.subplots(1,2,figsize=(14,6),facecolor='#101b24',gridspec_kw={'width_ratios':[1.6,1]})
    for ax in axes:
        ax.set_facecolor('#101b24');ax.tick_params(colors='#aec2cd');ax.xaxis.label.set_color('#aec2cd');ax.yaxis.label.set_color('#aec2cd')
        for spine in ax.spines.values():spine.set_color('#49616e')
    image=axes[0].pcolormesh(z*1000,x*1000,head['p'].reshape(x.shape)*998/1000,cmap='coolwarm',shading='auto')
    axes[0].set_aspect('equal');axes[0].set(xlabel='z downstream / mm',ylabel='x / mm',title='Actual static gauge pressure / y = 0')
    cb=fig.colorbar(image,ax=axes[0],fraction=.035,pad=.025);cb.set_label('kPa (relative to treated outlet)',color='#aec2cd');cb.ax.tick_params(colors='#aec2cd')
    U=chamber['U'].reshape(*cx.shape,3)
    image=axes[1].pcolormesh(cx*1000,cy*1000,U[:,:,2],cmap='RdBu_r',shading='auto')
    step=10;axes[1].quiver(cx[::step,::step]*1000,cy[::step,::step]*1000,U[::step,::step,0],U[::step,::step,1],color='#17202b',scale=110,width=.003)
    axes[1].set_aspect('equal');axes[1].set(xlabel='x / mm',ylabel='y / mm',title='Actual axial velocity / z = 40 mm')
    cb=fig.colorbar(image,ax=axes[1],fraction=.04,pad=.025);cb.set_label('U_z / m s⁻¹',color='#aec2cd');cb.ax.tick_params(colors='#aec2cd')
    for ax in axes:ax.title.set_color('#e2e8eb');ax.title.set_fontsize(11)
    fig.suptitle('joil  /  CT-35 saved flow initializer',x=.055,ha='left',color='#dfc28c',fontsize=19)
    fig.text(.055,.89,f'{case.name} · pseudo-iteration {time:g} · SST · arrows: in-plane velocity',color='#a0b6c4',fontsize=10)
    fig.text(.055,.065,'Executed CFD fields. Seed mesh fails quality checks; no transient statistics, oil-grade, experimental or structural validation.',color='#c6ad7b',fontsize=9)
    fig.text(.055,.035,'Pressure uses ρ = 998 kg/m³ and g = 0. Absolute pressure unknown. White/empty pixels excluded using native mesh validity mask.',color='#8fa8b8',fontsize=9)
    fig.subplots_adjust(left=.055,right=.95,top=.8,bottom=.19,wspace=.3)
    out.mkdir(parents=True);plate=out/'head-fields.png';fig.savefig(plate,dpi=160,facecolor=fig.get_facecolor());plt.close(fig)
    manifest={'kind':'executed_unqualified_CFD_saved_field_view','run_id':case.name,'saved_pseudo_iteration':time,
        'physical_time_averaging':False,'units':'m','field_units':{'p':'kinematic SI; displayed kPa after rho998','U':'m/s'},
        'input_sha256':{f'{time:g}/{n}':sha(field/n) for n in ['p','U']},
        'mesh_sha256':{n:sha(case/'constant/polyMesh'/n) for n in ['points','faces','owner','neighbour','boundary']},
        'image_sha256':sha(plate),'extractor_sha256':sha(__file__),'vtk_version':vtk.vtkVersion.GetVTKVersion(),
        'interpolation':'VTK cell-to-point, then mesh-aware probe; masks outside volume; display only',
        'governing_KPI_extractor':'native conservative face integrals, not image samples',
        'qualification':'initializer diagnostic, no validated performance claim'}
    (out/'scene.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps(manifest,indent=2))


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--case',type=Path,required=True)
    ap.add_argument('--time',type=float,required=True);ap.add_argument('--out',type=Path,required=True)
    a=ap.parse_args();render(a.case,a.time,a.out)
