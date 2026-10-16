"""Verify committed Plate 10 arrays against the prior direct plotting capture.

The derivation reads raw files via plate10_field_arrays.py. This separate check
compares captured numeric calls, never pixels as a data source. An optional
private replay restores JSON nulls to NaN and proves PNG reproduction from JSON.
"""
import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

import numpy as np

ROOT=Path(__file__).resolve().parents[3]
FIGURE='wave2-vs-wave1-openfoam14-cfd-comparison.png'
ARRAY_BINDINGS={
    191:('wave1','meridional',['z_edges_mm','r_edges_mm','u_theta_p90_grid']),
    199:('wave1','meridional',['z_centers_mm','r_wall_mm']),
    211:('wave2_tpms','meridional',['z_edges_mm','r_edges_mm','u_theta_p90_grid']),
    219:('wave2_tpms','meridional',['z_centers_mm','r_wall_mm']),
    242:('wave1','meridional',['z_edges_mm','r_edges_mm','p_kpa_mean_grid']),
    250:('wave1','meridional',['z_centers_mm','r_wall_mm']),
    261:('wave2_tpms','meridional',['z_edges_mm','r_edges_mm','p_kpa_mean_grid']),
    269:('wave2_tpms','meridional',['z_centers_mm','r_wall_mm']),
    291:('baseline','profiles/swirl_chamber_z20_90mm',['r_mm','u_theta_p75']),
    292:('wave1','profiles/swirl_chamber_z20_90mm',['r_mm','u_theta_p75']),
    293:('wave2_tpms','profiles/swirl_chamber_z20_90mm',['r_mm','u_theta_p75']),
    297:('baseline','profiles/swirl_chamber_z20_90mm',['r_mm','p_kpa_mean']),
    298:('wave1','profiles/swirl_chamber_z20_90mm',['r_mm','p_kpa_mean']),
    299:('wave2_tpms','profiles/swirl_chamber_z20_90mm',['r_mm','p_kpa_mean']),
}


def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def values(bundle,pointer):
    value=bundle
    for key in pointer.lstrip('/').split('/'):value=value[key]
    return value


def compare(bundle,captured):
    calls=captured['figures'][FIGURE]['calls'];rows=[]
    for line,(case,section,keys) in ARRAY_BINDINGS.items():
        matches=[c for c in calls if c['line']==line]
        if len(matches)!=1:raise ValueError(f'expected one pinned plot call at line{line}')
        for arg,key in enumerate(keys):
            pointer=f'/cases/{case}/{section}/{key}'
            expected=values(bundle,pointer);actual=matches[0]['args'][arg]
            rows.append({'source_ref':'crates/joil-evidence/receipts/Plate10-field-arrays.json#'+pointer,
                         'pinned_plot_call':f'tools/generate_wave2_vs_wave1_cfd_plate.py:{line}',
                         'positional_argument':arg,'match_exact_including_null_mask':actual==expected})
    return rows


def negative_tests(bundle,captured):
    results={}
    changed=copy.deepcopy(bundle)
    a=changed['cases']['wave1']['meridional']['u_theta_p90_grid']
    for row in a:
        finite=next((j for j,v in enumerate(row) if v is not None),None)
        if finite is not None:row[finite]+=1.;break
    results['changed_raw_derived_value_reject']=not all(r['match_exact_including_null_mask'] for r in compare(changed,captured))
    changed=copy.deepcopy(bundle)
    changed['cases']['wave1'],changed['cases']['wave2_tpms']=changed['cases']['wave2_tpms'],changed['cases']['wave1']
    results['swapped_subject_arrays_reject']=not all(r['match_exact_including_null_mask'] for r in compare(changed,captured))
    changed=copy.deepcopy(bundle)
    a=changed['cases']['wave1']['meridional']['u_theta_p90_grid']
    for row in a:
        missing=next((j for j,v in enumerate(row) if v is None),None)
        if missing is not None:row[missing]=0.;break
    results['null_to_zero_reject']=not all(r['match_exact_including_null_mask'] for r in compare(changed,captured))
    assert all(results.values()),results
    return results


def replay(bundle,output):
    from PIL import Image
    import matplotlib.pyplot as plt
    output=output.resolve()
    if not output.is_relative_to(ROOT/'.private'):raise ValueError('replay output must be EV-private')
    (output/'docs/figures').mkdir(parents=True,exist_ok=True)
    for name in ['elite-3grid-gci-and-rsm-verification-v2.json','wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json']:
        dest=output/'docs/results'/name;dest.parent.mkdir(parents=True,exist_ok=True)
        dest.write_bytes((ROOT/'docs/results'/name).read_bytes())
    spec=importlib.util.spec_from_file_location('ev_plate10_json_replay',ROOT/'tools/generate_wave2_vs_wave1_cfd_plate.py')
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    module.ROOT=output
    cases={Path(v['raw_case_relpath']).name:(v['time'],v) for v in bundle['cases'].values()}
    def selected(case,time):
        expected,data=cases[case.name]
        if time!=expected:raise ValueError('unexpected raw time in replay')
        return data
    module.extract_meridional_rz_grid=lambda case,time:{k:np.asarray(v,dtype=float) if isinstance(v,list) else v for k,v in selected(case,time)['meridional'].items()}
    module.extract_case_profiles=lambda case,time:selected(case,time)['profiles']
    def forbidden(*args):raise AssertionError('JSON replay tried to read raw fields/coordinates')
    module.compute_cell_centers=module.parse_internal_vectors=module.parse_internal_scalars=forbidden
    original_sha=module.sha256_file
    # Legacy main() fingerprints integral files solely when writing its unused
    # private manifest. Supply the already committed manifest values for those
    # bookkeeping calls, never for plotted data. No raw directory exists here.
    manifest=json.loads((ROOT/'docs/figures'/f'{FIGURE}.manifest.json').read_text())
    names={'rust-cfd-elite-gen024-isl2-ind014':'wave1_g1_integrals_sha256',
           'wave2-tpms-gyroid-g1-medium':'wave2_tpms_g1_integrals_sha256',
           'ct35-sst-extended-000':'baseline_ct35_integrals_sha256'}
    def replay_sha(path):
        if path.parent.name in names:return manifest['source_receipts'][names[path.parent.name]]
        return original_sha(path)
    module.sha256_file=replay_sha
    assert not (output/'runs').exists()
    plt.rcdefaults();module.main()
    actual=output/'docs/figures'/FIGURE;reference=ROOT/'docs/figures'/FIGURE
    a,b=np.asarray(Image.open(actual).convert('RGBA')),np.asarray(Image.open(reference).convert('RGBA'))
    equal=a.shape==b.shape and bool(np.array_equal(a,b))
    return {'data_inputs':'committed arrays JSON and two committed v2 dossiers; NO raw directories/fields/mesh or pixel-derived data',
            'raster_equal_to_committed_legacy_PNG':equal,'reference_png_sha256':sha(reference),
            'replayed_png_sha256':sha(actual),'private_legacy_manifest':'Not a new authoritative manifest; bookkeeping integral hashes copied from committed manifest only.'}


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--bundle',type=Path,required=True)
    ap.add_argument('--capture',type=Path,required=True)
    ap.add_argument('--output',type=Path,required=True)
    ap.add_argument('--replay-root',type=Path)
    args=ap.parse_args();bundle=json.loads(args.bundle.read_text());captured=json.loads(args.capture.read_text())
    rows=compare(bundle,captured)
    out={'schema_version':1,'kind':'EV13_F04_raw_array_source_and_legacy_replay_review_not_physical_validation',
         'array_file':str(args.bundle),'array_sha256':sha(args.bundle),'extractor_sha256':bundle['extractor_sha256'],
         'reviewer_sha256':sha(__file__),
         'pinned_direct_plot_capture_sha256':sha(args.capture),'plot_calls_traced':14,'array_bindings':len(rows),
         'all_bindings_match_exact':all(r['match_exact_including_null_mask'] for r in rows),
         'bindings':rows,'extractor_checks':bundle['checks'],'negative_checks':negative_tests(bundle,captured),
         'source_capture_status':'DONE_SCOPED; legacy numeric arrays source-bound, not corrected spatial data or physical claim evidence',
         'residuals':bundle['limitations']}
    if args.replay_root:out['JSON_only_replay']=replay(bundle,args.replay_root)
    args.output.write_text(json.dumps(out,indent=2,allow_nan=False)+'\n')
    print('bindings',len(rows),'match',out['all_bindings_match_exact'],'negative checks',out['negative_checks'])
    if args.replay_root:print(out['JSON_only_replay'])
    raise SystemExit(not out['all_bindings_match_exact'] or (args.replay_root is not None and not out['JSON_only_replay']['raster_equal_to_committed_legacy_PNG']))
