#!/usr/bin/env python3
"""Synthetic, bounded probes of baseline Python functions and frozen dimensions."""
import copy
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT/'tools'))


def load(name, relative):
    spec = importlib.util.spec_from_file_location(name, ROOT/relative)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


gate = load('v5_baseline_gate', 'tools/run_elite_3grid_gci_and_rsm.py')
prereg = json.loads((ROOT/'experiments/wave2-preregistration-v1.json').read_text())
dossier = {
    'subject_candidate_id': 'synthetic-not-a-physical-result',
    'grids': [dict(grid_id=f'G{i}', cells=100*(i+1), effective_cell_size_mm=3-i,
                   water_closure_fraction=0.0001, reject_fraction=0.02) for i in range(3)],
    'roache_gci_analysis': {'signal_to_uncertainty_margin': {
        'saving_to_3grid_span_ratio': 4.0, 'saving_to_3grid_eloss_span_ratio': 4.0}},
    'rsm_turbulence_closure_crosscheck': [dict(turbulence_model=m, water_closure_fraction=0.0001,
        reject_fraction=0.02, reynolds_stress_tensor_diagnostics={'positive_definite_normal_stress_fraction':1.0},
        delta_dp_u_vs_ct35_baseline_pct=-30.0, dp_u_pa=100.0, hydraulic_loss_j_per_treated_m3=110.0)
        for m in ('LRR','SSG')],
}
assert gate.evaluate_elite_gci_rsm_gate(dossier, prereg)['gate_passed']
strict = copy.deepcopy(prereg)
strict['mandatory_elite_cfd_gates']['min_signal_to_3grid_span_ratio'] = 100.0
assert gate.evaluate_elite_gci_rsm_gate(dossier, strict)['gate_passed']
strict['mandatory_elite_gci_and_rsm_gate'] = {'min_signal_to_3grid_span_ratio':100.0}
assert not gate.evaluate_elite_gci_rsm_gate(dossier, strict)['gate_passed']
validity_only = copy.deepcopy(dossier)
for item in validity_only['rsm_turbulence_closure_crosscheck']:
    item['delta_dp_u_vs_ct35_baseline_pct'] = 0.0
assert not gate.evaluate_elite_gci_rsm_gate(validity_only, prereg)['gate_passed']
print('D-03 canonical stricter SNR ignored; legacy-key stricter SNR rejects; zero improvement rejects despite identical verification data')

enact = load('v5_baseline_enact', 'tools/f0_enactment.py')
p = enact.exotic_graph.default_exotic_program()
p['inlet_ramp']['axial_drop_mm'] = 4.0
r = enact.build_enactment_receipt(p)
assert r['active_mechanisms'][0]['measured_geometric_delta_mm'] == 4.0
p['inlet_ramp']['axial_drop_mm'] = 6.0
r2 = enact.build_enactment_receipt(p)
assert r2['active_mechanisms'][0]['measured_geometric_delta_mm'] == 6.0
try:
    enact.build_enactment_receipt(p, dropped_features=('inlet_ramp',))
except enact.EnactmentVerificationError:
    pass
else:
    raise AssertionError('declared dropped feature should reject')
print('D-06 program-only receipt copies 4mm/6mm authored ramp values; declared dropped-feature control rejects; no mesh is measured')

audit = load('v5_baseline_claims', 'tools/claims_audit.py')
allowed = set()
audit.collect_json_numbers({'unrelated_pressure':100.0}, allowed)
errors, count = audit.scan_markdown_numeric_tokens({'synthetic':'Oil mass is 100.10 grams.'}, allowed)
assert count == 1 and not errors
errors2, _ = audit.scan_markdown_numeric_tokens({'synthetic':'Oil mass is 101.0 grams.'}, allowed)
assert errors2
assert 100000.0 in allowed and 100.0/1024 in allowed
print('D-23 unrelated 100.10 mass claim matches 100 pressure with fuzzy tolerance; 101.0 rejects; dimensionless conversion pool confirmed')

# Independent inverse CDF for df=4: F(t)=1/2+3u/4-u^3/4, u=t/sqrt(t^2+4).
lo, hi = 0.0, 10.0
for _ in range(100):
    t = (lo+hi)/2
    u = t/math.sqrt(t*t+4)
    if 0.5+0.75*u-0.25*u**3 < 0.975:
        lo = t
    else:
        hi = t
assert abs((lo+hi)/2 - 2.7764451051977987) < 1e-12
print(f'D-12 independent analytic t4 CDF inversion gives {(lo+hi)/2:.15f}')

genome = json.loads((ROOT/'baselines/ct35/genome.json').read_text())['parameters']
expected = {'d3_m':genome['d2_mm']*genome['d3_over_d2']/1000,
            'l2_m':genome['d2_mm']*genome['l2_over_d2']/1000,
            'l3_m':genome['d2_mm']*genome['d3_over_d2']*genome['l3_over_d3']/1000,
            'plenum_radius_m':genome['plenum_radius_mm']/1000,
            'inlet_h_over_w':genome['inlet_h_over_w']}
import re
source = (ROOT/'crates/joil-ir/src/lib.rs').read_text()
found = {name:float(re.search(r'let '+name+r' = ([0-9.]+)_f64;',source)[1]) for name in expected}
assert all(found[n] != expected[n] for n in expected)
print('D-15 frozen dimension mismatches: '+json.dumps({'IR':found,'genome_SI':expected},sort_keys=True))

history = json.loads((ROOT/'docs/receipts/v5-00-history-20261017.json').read_text())
assert all(not r['physical_pass'] for r in history['deferrals'])
print('Deferral records retain physical_pass=false. Production case-chain and sandbox-admission controls run separately in joil-evidence tests.')
print('PASS: synthetic Python defect probes and independent mathematical controls; no scientific qualification')
