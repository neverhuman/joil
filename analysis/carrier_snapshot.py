#!/usr/bin/env python3
"""Bind same-iteration observations from a running carrier to immutable raw bytes."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from analysis.units import pressure_scale
from analysis.evidence import research_fitness, save_new, sha256


def scalar_rows(data):
    rows={}
    for line in data.decode().splitlines(keepends=True):
        if not line.endswith('\n'):continue  # writer may be halfway through a row
        text=line.strip()
        if not text or text.startswith('#'):continue
        values=[float(x) for x in text.split()]
        if len(values)!=2 or any(not math.isfinite(x) for x in values):
            raise ValueError('expected finite scalar time/value rows')
        if values[0] in rows:raise ValueError('duplicate observation time')
        rows[values[0]]=values[1]
    if not rows:raise ValueError('empty scalar observation stream')
    return rows


def extract(observations, scale):
    common=set.intersection(*(set(v) for v in observations.values()))
    if not common:raise ValueError('no same-iteration observations across all ports')
    t=max(common)
    q={n:observations[n+'_flow'][t] for n in ['inlet1','inlet2','overflow','underflow']}
    p={n:observations[n+'_pressure'][t]*scale for n in ['inlet1','inlet2','overflow','underflow']}
    qi=-q['inlet1']-q['inlet2'];qu=q['underflow'];qo=q['overflow']
    if qi<=0:
        raise ValueError('no positive inlet total flow')
    pin=(-q['inlet1']*p['inlet1']-q['inlet2']*p['inlet2'])/qi
    dp=pin-p['underflow']
    return {'pseudo_iteration':t,'physical_elapsed_time_s':None,
            'feed_m3_h':qi*3600,'treated_m3_h':qu*3600,'reject_fraction':qo/qi,
            'flow_orientation_valid':qu>0 and 0<=qo<qi,
            'water_closure_fraction':abs(sum(q.values()))/qi,
            'static_tap_drop_pa':dp,'pdr':(pin-p['overflow'])/dp if dp>0 else None,
            'port_flow_m3_s':q,'port_area_average_static_pressure_pa':p}


def snapshot(case, out):
    case=Path(case).resolve();out=Path(out).resolve()
    if not case.is_relative_to(ROOT/'runs') or not out.is_relative_to(ROOT/'runs'):
        raise ValueError('case and raw snapshot must stay in project runs')
    out.mkdir(parents=True,exist_ok=False)
    raw=out/'source-data';raw.mkdir()
    bound_inputs=json.loads((case/'input-manifest.json').read_text())
    for name,expected in bound_inputs.items():
        p=(case/name).resolve()
        if not p.is_relative_to(case) or sha256(p)!=expected:
            raise ValueError(f'run input changed or escaped its case: {name}')
    mesh_input=json.loads((case/'mesh-input.json').read_text())
    if mesh_input['genome_file_sha256']!=sha256(ROOT/'baselines/ct35/genome.json'):
        raise ValueError('case is not bound to the frozen CT35 baseline')
    streams={};files={}
    for n in ['inlet1','inlet2','overflow','underflow']:
        for suffix in ['flow','pressure']:
            name=n+'_'+suffix
            paths=list((case/'postProcessing'/name).glob('0/*.dat'))
            if len(paths)!=1:raise ValueError(f'expected one uninterrupted observation stream for {name}')
            data=paths[0].read_bytes();(raw/(name+'.dat')).write_bytes(data)
            streams[name]=scalar_rows(data)
            files[name]={'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)}
    pressure=(case/'0/p').read_bytes();(raw/'pressure-field-input').write_bytes(pressure)
    req=json.loads((ROOT/'requirements/RS-joil-research-v1.json').read_text())
    density=req['reference']['water_density_kg_m3']
    values=extract(streams,pressure_scale(pressure.decode(),density))
    reasons=['steady_initializer_not_transient_statistics','spatial_and_temporal_error_unresolved',
             'oil_transport_not_included','pressure_energy_and_absolute_pressure_limits_unresolved',
             'no_independent_physical_validation']
    if (case/'mesh-audit.json').exists():
        mesh=json.loads((case/'mesh-audit.json').read_text())
        if not mesh['pilot_mesh_pass']:reasons.append('mesh_qualification_failed')
    else:reasons.append('mesh_qualification_missing')
    if values['water_closure_fraction']>.001:reasons.append('water_closure_exceeds_seed_0.1pct')
    if abs(values['reject_fraction']-.02)>.0002:reasons.append('target_split_not_met')
    if not values['flow_orientation_valid']:reasons.append('reverse_outlet_net_flow')
    fitness=research_fitness(cout_bounds=None,flow_m3_h=values['feed_m3_h'],
                 reject=values['reject_fraction'],hydraulic_loss_j_m3=None,eligibility_reasons=reasons) if values['flow_orientation_valid'] else {
        'policy':'fixed-duty-exploratory-screen-v1','minimize_screen_vector':None,
        'qualified_multiobjective_vector':None,'qualified_capacity_gain':None,
        'sampled_quality':'unknown','campaign_admission':'blocked_reverse_flow_and_unqualified_physics',
        'remaining_qualification_reasons':reasons}
    carrier=json.loads((case/'carrier-input.json').read_text()) if (case/'carrier-input.json').exists() else {'model':'SSG'}
    result={'schema_version':1,'kind':'executed_unqualified_carrier_snapshot',
            'run_id':case.name,'created_utc':datetime.now(timezone.utc).isoformat(),
            'fidelity':'F1 steady '+carrier['model']+' initializer diagnostic',
            'observations':values,'oil_capture':None,'cout_mg_l':None,
            'hydraulic_loss_j_per_treated_m3':None,
            'fitness':fitness,
            'evidence':{'captured_streams':files,'pressure_input_sha256':hashlib.sha256(pressure).hexdigest(),
                        'requirements_file_sha256':sha256(ROOT/'requirements/RS-joil-research-v1.json'),
                        'genome_file_sha256':sha256(ROOT/'baselines/ct35/genome.json'),
                        'extractor_sha256':sha256(__file__),
                        'input_manifest_sha256':sha256(case/'input-manifest.json')},
            'uncertainty':{'iteration_samples_are_physical_time_samples':False,
                           'sampling':None,'spatial':None,'model_form':None,
                           'confidence_interval':None},
            'limitations':['Incomplete running initializer; no accepted baseline or performance ranking.',
                           'Pressure values are static patch taps, not mechanical-energy loss.',
                           'Raw observation bytes archived with this immutable snapshot.']}
    save_new(out/'snapshot.json',result)
    print(json.dumps(result,indent=2))
    return result


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--case',type=Path,required=True)
    ap.add_argument('--out',type=Path,required=True);a=ap.parse_args();snapshot(a.case,a.out)
