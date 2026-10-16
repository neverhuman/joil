"""Capture legacy Plate 10 grids/profiles from retained raw OpenFOAM fields.

No pixels are read. Raw cases are read-only. Existing extraction formulas are
preserved, including the approximate coordinate method and its zero-owner-cell
defect, which are explicitly disclosed. This is provenance for a historical
diagnostic, not corrected spatial sampling or scientific/physical admission.
"""
import argparse
import hashlib
import importlib
import json
from pathlib import Path
import re
import sys
import tempfile

import numpy as np

ROOT = Path(__file__).resolve().parents[3]
SUBJECTS = {
    'wave1': ('rust-cfd-elite-gen024-isl2-ind014', '150', 'docs/results/elite-3grid-gci-and-rsm-verification-v2.json', '/grids/1/cells'),
    'wave2_tpms': ('wave2-tpms-gyroid-g1-medium', '150', 'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json', '/wave2_tpms_gyroid_elite_5run_dossier/grids/1/cells'),
    'baseline': ('ct35-sst-extended-000', '600', 'docs/results/elite-3grid-gci-and-rsm-verification-v2.json', '/baseline_ct35_comparator/cells'),
}
MESH_REQUIRED = ['points', 'faces', 'owner', 'neighbour', 'boundary']


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''): h.update(block)
    return h.hexdigest()


def finite_json(value):
    if isinstance(value, np.ndarray): return finite_json(value.tolist())
    if isinstance(value, (np.integer, np.floating)): return finite_json(value.item())
    if isinstance(value, dict): return {k: finite_json(v) for k,v in value.items()}
    if isinstance(value, (list,tuple)): return [finite_json(v) for v in value]
    if isinstance(value,float):
        if np.isnan(value): return None
        if not np.isfinite(value): raise ValueError('infinite derived value')
    return value


def modules():
    sys.path.insert(0, str(ROOT/'tools'))
    return (importlib.import_module('generate_openfoam14_flowfield_plate'),
            importlib.import_module('generate_wave2_vs_wave1_cfd_plate'))


def inventory(case, time):
    mesh=case/'constant/polyMesh'
    required=[case/time/name for name in ['U','p','k']]+[mesh/name for name in MESH_REQUIRED]
    required += [case/'cell-centers-summary.npz',case/'constant/physicalProperties']
    missing=[str(p) for p in required if not p.is_file()]
    if missing: raise FileNotFoundError('missing required raw inputs: '+', '.join(missing))
    paths=sorted(set(required+[p for p in mesh.rglob('*') if p.is_file()]))
    out={}
    for p in paths:
        if p.is_symlink() or not p.resolve().is_relative_to(case.resolve()):
            raise ValueError('raw input symlink/escape: '+str(p))
        out[p.relative_to(case).as_posix()]={'sha256':sha(p),'bytes':p.stat().st_size}
    return out


def raw_field(path, components, dimensions, count, parser):
    text=path.read_text()
    dim=re.search(r'dimensions\s*\[([^]]+)\]',text)
    header=re.search(r'internalField\s+nonuniform\s+List<(vector|scalar)>\s+(\d+)\s*\(',text)
    if not dim or list(map(int,dim.group(1).split()))!=dimensions:
        raise ValueError('unexpected field dimensions: '+str(path))
    if not header or header.group(1)!=('vector' if components==3 else 'scalar') or int(header.group(2))!=count:
        raise ValueError('field declaration/count mismatch: '+str(path))
    values=parser(path)
    shape=(count,3) if components==3 else (count,)
    if values.shape!=shape or not np.all(np.isfinite(values)):
        raise ValueError('field parsed count/nonfinite mismatch: '+str(path))
    values.setflags(write=False)
    return values


def coordinate_check(case, fields):
    """Recompute the exact legacy approximation; never write into the case."""
    pm=case/'constant/polyMesh'
    points=fields.parse_foam_points(pm/'points')
    fv=fields.parse_foam_faces_first_vertex(pm/'faces')
    nc,owners=fields.parse_foam_owner(pm/'owner')
    if (len(fv)!=len(owners) or not len(owners) or nc<=0 or
        np.any(owners<0) or np.any(owners>=nc) or np.any(fv<0) or np.any(fv>=len(points)) or
        not np.all(np.isfinite(points))): raise ValueError('malformed legacy mesh addressing')
    sums=np.zeros((nc,3));counts=np.zeros(nc)
    np.add.at(sums,owners,points[fv]);np.add.at(counts,owners,1.)
    calculated=sums/np.maximum(counts[:,None],1.)
    with np.load(case/'cell-centers-summary.npz',allow_pickle=False) as cache:
        centers=cache['centers']
    if centers.shape!=(nc,3) or not np.array_equal(centers,calculated):
        raise ValueError('retained coordinate cache does not match current mesh legacy reconstruction')
    return centers,{'method':'arithmetic mean of the FIRST vertex of each face OWNED by the cell; excludes neighbour contributions; NOT finite-volume centroids',
                    'mesh_cache_reconstruction_exact':True,'cells_without_owned_face_samples':int(np.sum(counts==0)),
                    'zero_owner_cell_coordinate':[0.,0.,0.],
                    'claim_state':'legacy_coordinate_defect_disclosed_not_corrected_spatial_sampling'}


def capture_case(case, time, fields, plate, meridional):
    before=inventory(case,time)
    centers,quality=coordinate_check(case,fields);n=len(centers)
    arrays={str(case/time/name):raw_field(case/time/name,3 if name=='U' else 1,
               [0,1,-1,0,0,0,0] if name=='U' else [0,2,-2,0,0,0,0],n,
               fields.parse_internal_vectors if name=='U' else fields.parse_internal_scalars) for name in ['U','p','k']}
    # Replace only in-memory callbacks: source functions cannot create caches or
    # silently truncate incompatible inputs. Original source files stay intact.
    originals=[]
    for module in [fields,plate]:
        for name in ['compute_cell_centers','parse_internal_vectors','parse_internal_scalars']:
            originals.append((module,name,getattr(module,name)))
            setattr(module,name,(lambda p:centers) if name=='compute_cell_centers' else (lambda p:arrays[str(p)]))
    try:
        profiles=fields.extract_case_profiles(case,time)
        grid=plate.extract_meridional_rz_grid(case,time) if meridional else None
    finally:
        for module,name,original in reversed(originals):setattr(module,name,original)
    if grid is not None:
        x,y,z=centers.T;r=np.hypot(x,y)*1000;z=z*1000
        mask=(z>=-20)&(z<=220)&(r<=35)
        iz=np.clip(np.digitize(z[mask],grid['z_edges_mm'])-1,0,47)
        ir=np.clip(np.digitize(r[mask],grid['r_edges_mm'])-1,0,27)
        index=ir*48+iz
        counts=np.bincount(index,minlength=28*48).reshape(28,48)
        sums=np.bincount(index,weights=arrays[str(case/time/'k')][mask],minlength=28*48).reshape(28,48)
        kmean=np.full((28,48),np.nan);np.divide(sums,counts,out=kmean,where=counts>=2)
        grid['bin_cell_count']=counts;grid['k_mean_m2_s2_grid']=kmean
        grid['valid_bin_mask']=(counts>=2)
    after=inventory(case,time)
    if after!=before:raise ValueError('raw inputs changed during extraction: '+str(case))
    mesh={p:d['sha256'] for p,d in before.items() if p.startswith('constant/polyMesh/')}
    return finite_json({'time':time,'cell_count':n,'raw_input_inventory':before,
          'mesh_inventory_sha256':hashlib.sha256(json.dumps(mesh,sort_keys=True,separators=(',',':')).encode()).hexdigest(),
          'mesh_inventory_digest_encoding':'SHA256 UTF8 sorted compact JSON of relative mesh filename -> raw byte SHA256; not a complete ScientificIdentity',
          'coordinate_review':quality,'field_review':{name:{'count':n,'all_finite':True,'min':float(v.min()),'max':float(v.max())} for name,v in [(name,arrays[str(case/time/name)]) for name in ['U','p','k']]},
          'meridional':grid,'profiles':profiles,'raw_inputs_unchanged_during_capture':True})


def capture(runs_root):
    fields,plate=modules();cases={}
    rho=json.loads((ROOT/'requirements/RS-joil-research-v1.json').read_text())['reference']['water_density_kg_m3']
    if rho!=998.0:raise ValueError('legacy extraction density differs from committed requirement')
    for key,(name,time,dossier,pointer) in SUBJECTS.items():
        dossier_count=json.loads((ROOT/dossier).read_text())
        for part in pointer.lstrip('/').split('/'):
            dossier_count=dossier_count[int(part)] if isinstance(dossier_count,list) else dossier_count[part]
        row=capture_case(runs_root/name,time,fields,plate,key!='baseline')
        row['raw_case_relpath']='runs/'+name;row['archived_dossier_cell_count']=dossier_count
        row['archived_dossier_cell_count_source']=dossier+'#'+pointer
        row['mesh_count_matches_archived_dossier']=row['cell_count']==dossier_count
        cases[key]=row
        print(key,'cells',row['cell_count'],'cache/mesh exact; zero-owner cells',row['coordinate_review']['cells_without_owned_face_samples'],flush=True)
    scripts=['tools/generate_openfoam14_flowfield_plate.py','tools/generate_wave2_vs_wave1_cfd_plate.py']
    return {'schema_version':1,'kind':'source_bound_legacy_Plate10_plot_arrays_not_physical_validation',
            'cases':cases,'source_scripts':{p:sha(ROOT/p) for p in scripts},
            'units':{'coordinates':'mm','raw_U':'m/s (vector components)','raw_p':'m2/s2 (kinematic gauge pressure)',
                     'raw_k':'m2/s2','u_theta_p90_grid':'m/s','u_z_mean_grid':'m/s','p_kpa_mean_grid':'kPa (static gauge)',
                     'k_mean_m2_s2_grid':'m2/s2','profile_r_mm':'mm','profile_u_theta_p75':'m/s (actual P90)',
                     'profile_u_z_mean':'m/s','profile_p_kpa_mean':'kPa (static gauge)',
                     'profile_accel_g_p75':'multiples of9.80665m/s2 (actual P90)','axial_tke_mean_m2_s2':'m2/s2'},
            'source_result_files':{p:sha(ROOT/p) for p in ['docs/results/elite-3grid-gci-and-rsm-verification-v2.json','docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json']},
            'postprocess_parameters':{'pressure_density_kg_m3':998.0,'density_source':'requirements/RS-joil-research-v1.json#/reference/water_density_kg_m3',
              'density_requirement_sha256':sha(ROOT/'requirements/RS-joil-research-v1.json'),
              'pressure_conversion':'raw kinematic p [m2/s2] *998 /1000 -> static gauge pressure [kPa]; not absolute pressure',
              'meridional':{'z_mm':[-20.,220.],'r_mm':[0.,35.],'nz':48,'nr':28,'minimum_cells_per_bin':2,'u_theta_quantile':0.90,'wall_radius_quantile':0.99,'p_u_z_k_aggregation':'unweighted arithmetic mean'},
              'radial_chamber':{'z_mm':[2.,55.],'r_edges_mm':[1.,34.5],'bins':16,'minimum_cells_per_bin':5,'u_theta_quantile':0.90,'acceleration_quantile':0.90},
              'radial_cone':{'z_mm':[55.,115.],'r_edges_mm':[1.,28.],'bins':14,'minimum_cells_per_bin':5,'u_theta_quantile':0.90},
              'axial':{'z_mm':[-20.,220.],'bins':24,'minimum_cells_per_bin':10,'u_theta_quantile':0.95,'k_aggregation':'unweighted mean'},
              'missing_bin_encoding':'JSON null = NaN/insufficient samples. Restore np.nan for plotting; never replace with zero.',
              'legacy_key_corrections':{'swirl_chamber_z20_90mm':'actual z2..55mm','taper_cone_z150_350mm':'actual z55..115mm','u_theta_p75':'actual P90','accel_g_p75':'actual P90','axial_profile/u_theta_p90_m_s':'actual P95'}},
            'limitations':['Exact legacy plotting arrays, not corrected cell-centroid sampling. Zero-owner cells retain artificial [0,0,0] coordinates under the historical algorithm; see per-case counts.',
              'Baseline retained mesh/field count313260 differs from archived dossier334768. No same-archived-mesh identity is certified.',
              'Raw U/p/k and mesh bytes are fingerprinted; selected grids/profiles are public, full raw fields stay private. This is post-hoc retained-artifact traceability, not recovery of missing historical scientific identity.',
              'All arrays are derived from raw solver files and retained mesh/cache; no pixel data is read. No oil, FEA, independent-label or physical validation follows.']}


def self_test(test_root):
    fields,_=modules();test_root.mkdir(parents=True,exist_ok=True);checks={}
    with tempfile.TemporaryDirectory(dir=test_root,prefix='synthetic-fields-') as folder:
        p=Path(folder)/'p'
        p.write_text('dimensions [0 2 -2 0 0 0 0]; internalField nonuniform List<scalar> 2 (1 3);')
        checks['strict_positive_scalars']=np.array_equal(raw_field(p,1,[0,2,-2,0,0,0,0],2,fields.parse_internal_scalars),[1.,3.])
        def rejects(name,fn):
            try:fn()
            except (OSError,ValueError,KeyError):checks[name]=True
            else:checks[name]=False
        rejects('wrong_dimensions_reject',lambda:raw_field(p,1,[0,1,-1,0,0,0,0],2,fields.parse_internal_scalars))
        rejects('wrong_declared_count_reject',lambda:raw_field(p,1,[0,2,-2,0,0,0,0],3,fields.parse_internal_scalars))
        p.write_text('dimensions [0 2 -2 0 0 0 0]; internalField nonuniform List<scalar> 2 (1);')
        rejects('truncated_values_reject',lambda:raw_field(p,1,[0,2,-2,0,0,0,0],2,fields.parse_internal_scalars))
        p.write_text('dimensions [0 2 -2 0 0 0 0]; internalField nonuniform List<scalar> 2 (1 nan);')
        rejects('nonfinite_values_reject',lambda:raw_field(p,1,[0,2,-2,0,0,0,0],2,fields.parse_internal_scalars))
        rejects('missing_raw_inputs_reject',lambda:inventory(Path(folder),'150'))
        checks['missing_bins_preserved_as_null']=finite_json([np.nan,1.])==[None,1.]
        rejects('infinite_derived_value_reject',lambda:finite_json([np.inf]))
        case=Path(folder)/'synthetic-case';pm=case/'constant/polyMesh';pm.mkdir(parents=True)
        (pm/'points').write_text('\n3\n(\n(0 0 0)\n(2 0 0)\n(0 2 0)\n)\n//\n')
        (pm/'faces').write_text('\n3\n(\n3(0 1 2)\n3(1 2 0)\n3(2 0 1)\n)\n//\n')
        (pm/'owner').write_text('// nCells: 2\n3\n(\n0\n0\n0\n)\n//\n')
        cache=case/'cell-centers-summary.npz'
        np.savez_compressed(cache,centers=np.array([[2/3,2/3,0.],[0.,0.,0.]]))
        coords,quality=coordinate_check(case,fields)
        checks['mesh_cache_positive_with_explicit_zero_owner_defect']=bool(
            quality['cells_without_owned_face_samples']==1 and np.array_equal(coords[1],[0.,0.,0.]))
        np.savez_compressed(cache,centers=np.zeros((2,3)))
        rejects('corrupt_coordinate_cache_reject',lambda:coordinate_check(case,fields))
    assert all(checks.values()),checks
    return checks


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--runs-root',type=Path,required=True)
    ap.add_argument('--output',type=Path,required=True)
    ap.add_argument('--test-root',type=Path,default=ROOT/'.private/ev13-f04-tests')
    args=ap.parse_args();result=capture(args.runs_root.resolve())
    result['extractor_sha256']=sha(__file__)
    result['checks']=self_test(args.test_root)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print('output',args.output,'sha256',sha(args.output), 'bytes',args.output.stat().st_size)
