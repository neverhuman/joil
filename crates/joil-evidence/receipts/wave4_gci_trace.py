"""Read-only, per-numeric-leaf audit of Wave-4 GCI inputs and derived arithmetic.

Run from the repo root. Raw runs are never written; only --out is written.
The JSONL output distinguishes source copies, recomputation and unsupported values.
"""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import re
import sys


def leaves(value, path=''):
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        yield path, value
    elif isinstance(value, dict):
        for key, child in value.items():
            yield from leaves(child, path + '/' + key.replace('~', '~0').replace('/', '~1'))
    elif isinstance(value, list):
        for i, child in enumerate(value):
            yield from leaves(child, path + '/' + str(i))


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def observed_order(f, h):
    """Independent bisection of the unequal-refinement power-law equation."""
    a, b = math.log(h[0]/h[1]), math.log(h[1]/h[2])
    target = math.log(abs((f[1]-f[0])/(f[2]-f[1])))
    def residual(p):
        return p*b + math.log(math.expm1(p*a)) - math.log(math.expm1(p*b)) - target
    lo, hi = 1e-8, 64.0
    if residual(lo) >= 0:
        # Published divergent-state value is an initial log-ratio estimate,
        # not a converged positive power-law fit.
        return target / math.log(0.5*(h[0]/h[1] + h[1]/h[2]))
    while residual(hi) < 0:
        hi *= 2
        if hi > 2048:
            raise ValueError('observed-order root not bracketed')
    for _ in range(100):
        mid = (lo+hi)/2
        if residual(mid) > 0:
            hi = mid
        else:
            lo = mid
    return (lo+hi)/2


def audit(repo, runs, output):
    report_path = repo/'docs/results/wave4-gci-v1.json'
    data = read(report_path)
    proof_path = repo/'docs/results/wave4-evolution-proof-v1.json'
    proof = read(proof_path)
    assert sha(proof_path) == data['source_wave4_proof_sha256']
    targets, rows, errors, sources = dict(leaves(data)), {}, [], {}

    def source(path):
        path = Path(path)
        label = path.relative_to(repo).as_posix() if path.is_relative_to(repo) else 'INT-runs/' + path.relative_to(runs).as_posix()
        sources[label] = sha(path)
        return label

    def add(path, value, origin, method='recomputed'):
        actual = targets[path]
        match = value is not None and math.isfinite(actual) and math.isfinite(value) and math.isclose(actual, value, rel_tol=1e-10, abs_tol=1e-12)
        rows[path] = dict(pointer=path, reported=actual, recomputed=value,
                          source=origin, method=method, match=match)

    def copy_tree(path, value, origin, method='source-bound copy'):
        for suffix, number in leaves(value):
            add(path+suffix, number, origin+suffix, method)

    add('/schema_version', 2, 'declared report schema v2', 'schema constant')
    ranked = sorted(enumerate(proof['arm_outcomes']['B3_Wave3_Exotic3D_QD_CEI']), key=lambda p:p[1]['final_best_kpi_score'], reverse=True)
    subjects = [('/same_pipeline_ct35_control', data['same_pipeline_ct35_control'])] + [
        (f'/wave4_b3_top_winners/{i}', s) for i,s in enumerate(data['wave4_b3_top_winners'])]
    derived_grids = {}
    for index, (prefix, subject) in enumerate(subjects):
        ref = runs/(subject['prefix']+'-g2-fine')
        if not (ref/'input-manifest.json').is_file():
            ref = runs/(subject['prefix']+'-g0-coarse')
        manifest_path = ref/'input-manifest.json'
        manifest = read(manifest_path)
        manifest_source = source(manifest_path)
        for field in ['derived_taxon', 'enactment_receipt', 'subject_bundle']:
            copy_tree(prefix+'/'+field, manifest[field], manifest_source+':/'+field)
        for field in ['design_sha256', 'design_ir_sha256', 'surface_stl_sha256']:
            if subject[field] != manifest[field]:
                errors.append(prefix+'/'+field+' differs from prepared source')
        stl = ref/manifest['surface_stl_relpath']
        if sha(stl) != manifest['surface_stl_sha256']:
            errors.append(prefix+' STL digest mismatch')
        source(stl)
        sys.path.insert(0, str(repo))
        from tools.f0_enactment import audit_stl_inlet_enactment
        replay = audit_stl_inlet_enactment(stl, read(ref/'design-program.json'))
        copy_tree(prefix+'/independent_f0_enactment_audit', replay,
                  source(stl)+':observer replay', 'same observer replay, not independent algorithm validation')
        if index == 0:
            candidate = proof['baseline_b0_ct35']
            metrics = dict(kpi_score=proof['baseline_b0_ct35_kpi_score'],
                           cout_sample_screen_upper_mg_l=candidate['mc_robustness']['cout_sample_screen_upper_mg_l'],
                           dp_u_pa=candidate['nominal_metrics']['dp_u_pa'])
            add(prefix+'/rank_in_arm', 0, 'frozen baseline reference', 'reference metadata')
            origin = source(proof_path)+':/baseline_b0_ct35'
            origins = dict(kpi_score=source(proof_path)+':/baseline_b0_ct35_kpi_score',
                           cout_sample_screen_upper_mg_l=origin+'/mc_robustness/cout_sample_screen_upper_mg_l',
                           dp_u_pa=origin+'/nominal_metrics/dp_u_pa')
        else:
            proof_index, outcome = ranked[index-1]
            candidate = outcome['best_candidate']
            metrics = dict(kpi_score=outcome['final_best_kpi_score'],
                           cout_sample_screen_upper_mg_l=candidate['mc_robustness']['cout_sample_screen_upper_mg_l'],
                           dp_u_pa=candidate['nominal_metrics']['dp_u_pa'])
            add(prefix+'/seed', outcome['seed'], source(proof_path)+f':/arm_outcomes/B3_Wave3_Exotic3D_QD_CEI/{proof_index}/seed')
            add(prefix+'/rank_in_arm', index, 'descending final_best_kpi_score rank in Wave4 proof')
            origin = source(proof_path)+f':/arm_outcomes/B3_Wave3_Exotic3D_QD_CEI/{proof_index}'
            origins = dict(kpi_score=origin+'/final_best_kpi_score',
                           cout_sample_screen_upper_mg_l=origin+'/best_candidate/mc_robustness/cout_sample_screen_upper_mg_l',
                           dp_u_pa=origin+'/best_candidate/nominal_metrics/dp_u_pa')
        for field, value in metrics.items():
            add(prefix+'/f1_sandbox_screen_metrics/'+field, value, origins[field], 'source-bound screen outputs')
        grids = []
        for j, grid in enumerate(subject['grids']):
            case = runs/Path(grid['case_dir']).name
            hyd_path, mesh_path = case/'hydraulic-integrals.json', case/'mesh-2-checkMesh.log'
            hyd, text = read(hyd_path), mesh_path.read_text()
            hyd_source, mesh_source = source(hyd_path), source(mesh_path)
            if sha(hyd_path) != grid['hydraulic_integrals_sha256'] or sha(mesh_path) != grid['checkmesh_log_sha256']:
                errors.append(prefix+f'/grids/{j} raw-source hash mismatch')
            def number(pattern, group=1):
                match = re.search(pattern, text)
                if match is None:
                    raise ValueError('checkMesh field absent: '+pattern)
                return float(match[group].rstrip('.'))
            cm = dict(cells=number(r'cells:\s+(\d+)'),
                      total_volume_m3=number(r'Total volume\s*=\s*([0-9.eE+-]+)'),
                      max_skewness=number(r'Max skewness\s*=\s*([0-9.eE+-]+)'),
                      max_non_orthogonality_deg=number(r'Mesh non-orthogonality Max:\s*([0-9.eE+-]+)\s*average:\s*([0-9.eE+-]+)'),
                      avg_non_orthogonality_deg=number(r'Mesh non-orthogonality Max:\s*([0-9.eE+-]+)\s*average:\s*([0-9.eE+-]+)',2))
            values = dict(cm)
            values['effective_cell_size_mm'] = 1000*(cm['total_volume_m3']/cm['cells'])**(1/3)
            for field in ['feed_m3_h', 'treated_m3_h', 'reject_fraction', 'water_closure_fraction', 'hydraulic_loss_w', 'hydraulic_loss_j_per_treated_m3']:
                values[field] = hyd[field]
            patches = hyd['patches']
            inlet = sum(patches[k]['area_average_static_pressure_pa'] for k in ['inlet1','inlet2'])/2
            values['dp_u_pa'] = inlet-patches['underflow']['area_average_static_pressure_pa']
            values['dp_o_pa'] = inlet-patches['overflow']['area_average_static_pressure_pa']
            values['pdr'] = values['dp_o_pa']/values['dp_u_pa']
            block_path = case/'system/blockMeshDict'
            block = re.search(r'hex\s*\([^)]*\)\s*\((\d+)\s+(\d+)\s+(\d+)\)', block_path.read_text())
            if block is None:
                raise ValueError('background grid dimensions missing')
            values['background_hex_grid'] = [int(v) for v in block.groups()]
            for field,value in values.items():
                origin = (source(block_path) if field=='background_hex_grid' else
                          mesh_source if field in cm or field=='effective_cell_size_mm' else hyd_source)
                copy_tree(prefix+f'/grids/{j}/'+field, value, origin+':'+field,
                          'raw log/integral parse or independent arithmetic')
            grids.append(values)
        derived_grids[prefix] = grids
        if not grids:
            continue
        for metric in ['dp_u_pa','hydraulic_loss_j_per_treated_m3','reject_fraction']:
            f = [g[metric] for g in grids]
            h = [g['effective_cell_size_mm'] for g in grids]
            r10,r21 = h[0]/h[1],h[1]/h[2]
            e10,e21 = f[1]-f[0],f[2]-f[1]
            floor = 64*sys.float_info.epsilon*max(abs(v) for v in f)
            p = None
            if abs(e10)>floor and abs(e21)>floor and e10*e21>0:
                p = observed_order(f,h)
            vals = dict(f_g0_coarse=f[0],f_g1_medium=f[1],f_g2_fine=f[2],
                        r21_linear=r21,r10_linear=r10,e21_fine_minus_medium=e21,
                        e10_medium_minus_coarse=e10,
                        relative_step_21_pct=100*abs(e21)/max(abs(f[2]),1e-15),
                        relative_step_10_pct=100*abs(e10)/max(abs(f[1]),1e-15),
                        convergence_ratio_R=e21/e10 if abs(e10)>floor else 0,
                        formal_upwind_order_p=1.0,
                        gci21_conservative_fs300_p1_pct=300*abs(e21)/max(abs(f[2]),1e-15)/(r21-1),
                        three_grid_extremum_span=max(f)-min(f),
                        three_grid_half_span_pct_of_f2=50*(max(f)-min(f))/max(abs(f[2]),1e-15))
            if p is not None:
                vals['observed_order_p'] = p
                if p>0:
                    vals['unverified_3grid_roache_fs125_diagnostic_pct'] = 125*abs(e21)/max(abs(f[2]),1e-15)/math.expm1(p*math.log(r21))
            gp = prefix+'/roache_gci_v2_analysis/'+metric
            copy_tree(gp, vals, 'docs/results/wave4-gci-v1.json:'+prefix+'/grids/* (raw-source trace above)',
                      'independent GCI arithmetic; positive order from bisection')
            recorded = subject['roache_gci_v2_analysis'][metric]
            if recorded['asymptotic_range_certified'] or recorded['gci21_standard_fs125_pct'] is not None:
                errors.append(gp+' incorrectly certifies three-grid standard GCI')

    control = data['same_pipeline_ct35_control']
    ctrl = derived_grids['/same_pipeline_ct35_control'][-1]
    hist_path = repo/'docs/results/exotic-vs-baseline-cfd-benchmark-v1.json'
    hist = read(hist_path)['baseline_ct35_openfoam14_reference']
    hist_dp, hist_el = hist['static_tap_drop_pa'], hist['device_hydraulic_loss_j_m3']
    hp = source(hist_path)+':/baseline_ct35_openfoam14_reference'
    cp = '/same_pipeline_ct35_control/g09_pipeline_confounding_audit'
    cg = derived_grids['/same_pipeline_ct35_control']
    span = max(g['dp_u_pa'] for g in cg)-min(g['dp_u_pa'] for g in cg)
    vals = dict(same_pipeline_unextended_ct35_g0_dp_u_pa=cg[0]['dp_u_pa'],same_pipeline_unextended_ct35_g1_dp_u_pa=cg[1]['dp_u_pa'],same_pipeline_unextended_ct35_g2_dp_u_pa=cg[2]['dp_u_pa'],same_pipeline_unextended_ct35_3grid_span_pa=span,historical_extended_ct35_reference_dp_u_pa=hist_dp,apparent_pipeline_offset_pa=ctrl['dp_u_pa']-hist_dp,apparent_pipeline_offset_pct=100*(ctrl['dp_u_pa']-hist_dp)/hist_dp)
    copy_tree(cp,vals,hp+' + recomputed control grids','independent comparator arithmetic')
    for field in ['archived_wave1_2port_wall_modulated_g2_dp_u_pa','true_delta_archived_wave1_vs_same_pipeline_ct35_pa','true_delta_archived_wave1_vs_same_pipeline_ct35_pct']:
        add(cp+'/'+field,None,'unreferenced hardcoded archived value in producer','unsupported source')
    for i,subject in enumerate(data['wave4_b3_top_winners']):
        prefix = f'/wave4_b3_top_winners/{i}'
        grids = derived_grids[prefix]
        if not grids:
            continue
        final = grids[-1]
        for comparator,ref_dp,ref_el in [('vs_same_pipeline_ct35_control_g2',ctrl['dp_u_pa'],ctrl['hydraulic_loss_j_per_treated_m3']),('vs_historical_extended_ct35_reference',hist_dp,hist_el)]:
            pre = prefix+'/comparative_margins/'+comparator
            same = comparator.startswith('vs_same')
            vals = {('ct35_control_g2_dp_u_pa' if same else 'historical_ct35_dp_u_pa'):ref_dp,('ct35_control_g2_eloss_j_m3' if same else 'historical_ct35_eloss_j_m3'):ref_el}
            for metric,label,ref_val,unit in [('dp_u_pa','dp_u',ref_dp,'pa'),('hydraulic_loss_j_per_treated_m3','eloss',ref_el,'j_m3')]:
                span = max(g[metric] for g in grids)-min(g[metric] for g in grids)
                vals['fine_grid_'+label+'_saving_'+unit] = ref_val-final[metric]
                vals['fine_grid_'+label+'_saving_pct'] = 100*(final[metric]-ref_val)/ref_val
                vals['saving_to_3grid_span_ratio_'+label] = (ref_val-final[metric])/max(span,1e-9)
                if same:
                    vals['three_grid_'+label+'_span_'+unit] = span
            copy_tree(pre,vals,hp+' + recomputed control/winner raw grids','independent comparator arithmetic')
    for path in targets.keys()-rows.keys():
        add(path,None,'no source/formula assigned','untraced numeric leaf')
    output.parent.mkdir(parents=True,exist_ok=True)
    with output.open('w') as stream:
        for path in sorted(rows):
            stream.write(json.dumps(rows[path],separators=(',',':'))+'\n')
    counts = Counter(row['method'] for row in rows.values())
    return dict(input_sha256=sha(report_path),numeric_leaves=len(targets),traced_leaves=len(rows),
                matched=sum(r['match'] for r in rows.values()),method_counts=dict(counts),
                mismatches=[r for r in rows.values() if not r['match']],metadata_errors=errors,
                source_artifacts_sha256=sources,trace_sha256=sha(output))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runs-root',type=Path,required=True)
    parser.add_argument('--out',type=Path,default=Path('.private/wave4-gci-number-trace.jsonl'))
    args = parser.parse_args()
    print(json.dumps(audit(Path.cwd(),args.runs_root,args.out),indent=2))
