#!/usr/bin/env python3
"""Publish small measured hydraulic observations, with qualification and fitness explicit."""
import argparse
from datetime import datetime, timezone
import json
import math
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from analysis.evidence import sha256, save_new, research_fitness


def reconcile(observed, integrals):
    if observed['pseudo_iteration']!=integrals['time']:
        raise ValueError('observation/integral saved times differ')
    for name,q in observed['port_flow_m3_s'].items():
        native=integrals['patches'][name]['signed_flow_m3_s']
        if not math.isfinite(native) or abs(q-native)>max(1e-12,abs(q)*1e-8):
            raise ValueError('native energy and pressure snapshot use different fluxes')
    energy=integrals['hydraulic_loss_j_per_treated_m3']
    if not math.isfinite(energy) or energy<0:raise ValueError('nonphysical mechanical loss')
    if integrals['rho_kg_m3']!=998 or integrals['gravity_m_s2']!=[0,0,0]:
        raise ValueError('unsupported energy density/gravity contract')
    return energy


def point(case, snapshot_dir):
    case=Path(case).resolve();snap=case/snapshot_dir/'snapshot.json'
    if not case.is_relative_to(ROOT/'runs'):raise ValueError('project runs only')
    s=json.loads(snap.read_text());obs=s['observations'];time=obs['pseudo_iteration']
    req=ROOT/'requirements/RS-joil-research-v1.json'
    if s['evidence']['requirements_file_sha256']!=sha256(req):raise ValueError('requirements changed')
    if s['evidence']['input_manifest_sha256']!=sha256(case/'input-manifest.json'):raise ValueError('input manifest changed')
    for name,digest in json.loads((case/'input-manifest.json').read_text()).items():
        path=(case/name).resolve()
        if not path.is_relative_to(case) or sha256(path)!=digest:raise ValueError('changed/escaped run input')
    for name,item in s['evidence']['captured_streams'].items():
        if sha256(case/snapshot_dir/'source-data'/(name+'.dat'))!=item['sha256']:
            raise ValueError('raw snapshot observation changed')
    native_path=case/f'hydraulic-integrals-t{time:g}.json'
    native=json.loads(native_path.read_text());energy=reconcile(obs,native)
    metrics=json.loads((case/f'metrics-binary-t{time:g}.json').read_text())
    if sha256(ROOT/metrics['path'])!=metrics['sha256']:raise ValueError('native metrics binary changed')
    receipts={}
    for path in [case/'water-1-mpirun.log.receipt.json',case/f'metrics-t{time:g}-0-joilHydraulicMetrics.log.receipt.json']:
        receipt=json.loads(path.read_text())
        log=path.with_name(path.name.removesuffix('.receipt.json'))
        if receipt['exit_code'] or sha256(log)!=receipt['log_sha256']:
            raise ValueError('failed or changed execution receipt')
        receipts[str(path.relative_to(ROOT))]={'sha256':sha256(path),'wall_seconds':receipt['wall_seconds'],
            'image_id':receipt['image_id'],'exit_code':receipt['exit_code']}
    reasons=list(s['fitness']['remaining_qualification_reasons'])
    reverse={n:native['patches'][n]['reverse_flow_m3_s'] for n in ['overflow','underflow']}
    if any(v>1e-11 for v in reverse.values()):reasons.append('external_outlet_backflow_requires_resolution')
    limits=json.loads(req.read_text())['limits']['pdr_draft_range']
    if obs['pdr'] is None or not limits[0]<=obs['pdr']<=limits[1]:reasons.append('outside_draft_pdr_range')
    fitness=research_fitness(cout_bounds=None,flow_m3_h=obs['feed_m3_h'],reject=obs['reject_fraction'],
        hydraulic_loss_j_m3=energy,eligibility_reasons=reasons)
    return {'run_id':case.name,'kind':'measured_unqualified_F1_initializer',
        'model':json.loads((case/'carrier-input.json').read_text())['model'],
        'numerical_outlet_extension_m':json.loads((case/'domain-extension.json').read_text())['length_m'] if (case/'domain-extension.json').exists() else 0,
        'observations':obs,'full_domain_hydraulic_loss_w':native['hydraulic_loss_w'],
        'full_domain_hydraulic_loss_j_per_treated_m3':energy,'external_outlet_reverse_flow_m3_s':reverse,
        'original_device_overflow':native.get('original_device_overflow'),
        'oil_capture':None,'cout_mg_l':None,'fitness':fitness,
        'evidence':{'snapshot':{'path':str(snap.relative_to(ROOT)),'sha256':sha256(snap)},
            'native_integrals':{'path':str(native_path.relative_to(ROOT)),'sha256':sha256(native_path)},
            'native_metrics_binary_sha256':metrics['sha256'],'execution':receipts}}


def build(specs,out):
    points=[point(case,snapshot) for case,snapshot in specs]
    result={'schema_version':1,'kind':'executed_baseline_hydraulic_diagnostics',
        'created_utc':datetime.now(timezone.utc).isoformat(),
        'baseline_genome_sha256':sha256(ROOT/'baselines/ct35/genome.json'),
        'requirements_sha256':sha256(ROOT/'requirements/RS-joil-research-v1.json'),
        'extractor_sha256':sha256(__file__),'points':points,
        'qualified_baseline':False,'physical_validation':False,'qualified_capacity_gain':None,
        'limitations':['Steady pseudo-iterations are not physical sampling times or confidence intervals.',
            'SST is an initializer; transient Reynolds-stress, grid/time/model and oil qualification remain open.',
            'The seed mesh fails the all-geometry concave-cell check and has no wall-layer qualification.',
            'Outlet extension changes the numerical domain, not the fixed device genotype.',
            'Static pressure taps and mechanical energy flux are distinct; pump/system energy is unknown.',
            'Oil-removal, compliance, structural suitability and superiority are unknown.']}
    save_new(out,result)
    print(json.dumps({'path':str(out),'points':len(points),'qualified_baseline':False},indent=2))


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--point',nargs=2,action='append',metavar=('CASE','SNAPSHOT_DIR'),required=True)
    ap.add_argument('--out',type=Path,required=True);a=ap.parse_args();build(a.point,a.out)
