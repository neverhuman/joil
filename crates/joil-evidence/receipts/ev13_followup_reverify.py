"""Reverify the EV13 follow-up from exported git snapshots and plot captures.

Run ev13_figure_trace.py first against the new snapshot, including FIELD_ARRAYS
at its committed relative path and no runs directory. This check derives its
own AST line map, checks every numeric call against the previous capture, and
compares the 32 field-array arguments exactly, including nulls. It reads neither
raw cases nor pixels as data. Raster replay is not scientific validation.
"""
import argparse
import ast
from collections import defaultdict
import copy
import hashlib
import json
from pathlib import Path

from ev13_figure_values import Audit, FIELD_ARRAYS, G1, W3, W4, audit, dispatch_line, self_test
from plate10_field_array_review import compare, negative_tests

METHODS = {'plot', 'step', 'scatter', 'bar', 'barh', 'fill_between',
           'axhline', 'axvline', 'errorbar', 'pcolormesh'}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def plot_nodes(path):
    code=path.read_text();rows=[]
    class Visitor(ast.NodeVisitor):
        def __init__(self):self.stack=[]
        def visit_FunctionDef(self,node):
            self.stack.append(node.name);self.generic_visit(node);self.stack.pop()
        def visit_Call(self,node):
            if isinstance(node.func,ast.Attribute) and node.func.attr in METHODS:
                signature={'function':self.stack.copy(),
                           'call':ast.dump(node.func,include_attributes=False),
                           'positional':[ast.dump(a,include_attributes=False) for a in node.args]}
                rows.append(dict(line=node.lineno,signature=signature,
                                 expression=ast.get_source_segment(code,node)))
            self.generic_visit(node)
    Visitor().visit(ast.parse(code))
    return rows


def line_map(original, current, sources):
    mapping={};manual=[];unused=[]
    for source in sorted(sources):
        by_signature=defaultdict(list)
        for row in plot_nodes(original/source):
            by_signature[json.dumps(row['signature'],sort_keys=True)].append(row)
        mapping[source]={}
        for row in plot_nodes(current/source):
            found=by_signature[json.dumps(row['signature'],sort_keys=True)]
            if len(found)==1:
                target=found[0]['line']
            else:
                node=ast.parse(row['expression']).body[0].value
                policy_calls={
                    ('tools/generate_wave3_proof_figures.py','ax_b.axhline'):174,
                    ('tools/generate_wave2_figures_and_animations.py','ax_ci.axhline'):350,
                    ('tools/generate_wave2_figures_and_animations.py','ax_duty.axhline'):396}
                target=policy_calls.get((source,ast.unparse(node.func)))
                if target is not None and ast.unparse(node.args[0])=='research_limit':
                    # Independently bind this changed positional expression to
                    # the old literal-30 call; the value audit checks its JSON.
                    old=next(r for r in plot_nodes(original/source) if r['line']==target)
                    old_node=ast.parse(old['expression']).body[0].value
                    assert ast.literal_eval(old_node.args[0])==30
                    assert ast.dump(old_node.func)==ast.dump(node.func)
                    assert len(node.args)==len(old_node.args)==1
                    manual.append(dict(source=source,actual_line=row['line'],original_line=target,
                                       original_expression=old['expression'],new_expression=row['expression'],
                                       change='literal30 -> committed research_limit; value checked independently'))
                else:
                    unused.append(dict(source=source,line=row['line'],candidate_count=len(found)))
                    continue
            mapping[source][str(row['line'])]=target
    return mapping,manual,unused


def normalized(capture,mapping):
    result=copy.deepcopy(capture)
    for figure in result['figures'].values():
        for call in figure['calls']:
            assert str(call['line']) in mapping[call['source']],call
            call['line']=dispatch_line(call,mapping)
    return result


def labels(capture,root):
    def read(path):return json.loads((root/path).read_text())
    d3,d4=read(W3),read(W4)
    stress_data=read(G1)['rsm_turbulence_closure_crosscheck'][0]['reynolds_stress_tensor_diagnostics']
    baseline=read(FIELD_ARRAYS)['cases']['baseline']
    figures=capture['figures']
    w3h=figures['wave3-readme-hero-kpi-and-elite.png']
    w3p=figures['wave3-evolution-ablation-and-lineage-proof.png']
    w4h=figures['wave4-readme-hero-kpi-and-elite.png']
    w4p=figures['wave4-evolution-ablation-and-lineage-proof.png']
    stress='\n'.join(figures['elite-gci-and-rsm-verification.png']['axes'][4]['text'])
    caveat='\n'.join(figures['wave2-vs-wave1-openfoam14-cfd-comparison.png']['figure_text'])
    w3all=json.dumps([w3h['figure_text'],w3h['axes'],w3p['figure_text'],w3p['axes']])
    return {
        'wave3_450_evaluations_per_seed':f"{d3['evaluations_per_seed_per_arm']} Total Evaluations/Seed" in w3h['axes'][0]['xlabel'],
        'wave3_22500_evaluations':f"{d3['total_3d_evaluations_across_arms']:,} Evaluations Across 5 Search Arms" in '\n'.join(w3p['figure_text']),
        'wave3_no_charged_claim':'charged' not in w3all.lower(),
        'wave4_450_charged_preserved':f"{d4['evaluations_per_seed_per_arm']} Total Charged Proposals/Seed" in w4h['axes'][0]['xlabel'],
        'wave4_22500_charged_preserved':f"{d4['total_3d_evaluations_across_arms']:,} Charged Proposals Across 5 Search Arms" in '\n'.join(w4p['figure_text']),
        'plate3_normal_stress_positivity':'Normal-Stress Positivity' in stress and f"{stress_data['positive_definite_normal_stress_fraction']*100:.2f}%" in stress,
        'plate3_no_tensor_realizability_claim':'realizability' not in stress.lower(),
        'plate10_legacy_spatial_caveat':all(s in caveat for s in ['owned-face first-vertex','not FV centroids','zero-owner cells placed at origin']),
        'plate10_archived_identity_caveat':all(s in caveat for s in [f"{baseline['cell_count']:,}",f"{baseline['archived_dossier_cell_count']:,}",'identity mismatch','Numerical diagnostic only'])}


def additional_negatives(root,capture,mapping):
    result={}
    for name in ['historical_charged_wording','full_realizability_wording','omitted_spatial_caveat']:
        changed=copy.deepcopy(capture)
        if name=='historical_charged_wording':
            changed['figures']['wave3-readme-hero-kpi-and-elite.png']['axes'][0]['xlabel']='450 Total Charged Proposals/Seed'
        elif name=='full_realizability_wording':
            texts=changed['figures']['elite-gci-and-rsm-verification.png']['axes'][4]['text']
            texts[:]=[s.replace('Normal-Stress Positivity','Full Tensor Realizability') for s in texts]
        else:changed['figures']['wave2-vs-wave1-openfoam14-cfd-comparison.png']['figure_text']=[]
        result[name+'_reject']=not all(labels(changed,root).values())
    worker=Audit(root,capture,mapping)
    manifest='docs/figures/wave2-vs-wave1-openfoam14-cfd-comparison.png.manifest.json'
    worker.get(manifest)['source_receipts']['field_array_bundle_sha256']='0'*64
    result['wrong_manifest_field_bundle_sha256_reject']=any(r['status']=='FAIL' for r in worker.run())
    assert all(result.values()),result
    return result


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--input-root',type=Path,required=True)
    ap.add_argument('--original-root',type=Path,required=True)
    ap.add_argument('--capture',type=Path,required=True)
    ap.add_argument('--previous-capture',type=Path,required=True)
    ap.add_argument('--previous-line-map',type=Path,required=True)
    ap.add_argument('--line-map',type=Path,required=True)
    ap.add_argument('--value-trace',type=Path,required=True)
    ap.add_argument('--output',type=Path,required=True)
    args=ap.parse_args()
    capture=json.loads(args.capture.read_text())
    assert not capture['capture_errors'],capture['capture_errors']
    assert not (args.input_root/'runs').exists(),'raw-free snapshot required'
    sources=set(json.loads(args.previous_line_map.read_text()))
    mapping,manual,unused=line_map(args.original_root,args.input_root,sources)
    args.line_map.write_text(json.dumps(mapping,indent=2,sort_keys=True)+'\n')
    current=normalized(capture,mapping)
    previous=normalized(json.loads(args.previous_capture.read_text()),json.loads(args.previous_line_map.read_text()))
    unchanged=[]
    for figure,f in current['figures'].items():
        old=previous['figures'][figure]['calls'];new=f['calls']
        # All positional arrays and numeric error bars; text/style kwargs are
        # covered separately and deliberately may change between these pins.
        def data(c):
            return {k:c[k] for k in ['method','source','line','axis','bindings','args']}|{
                'numeric_kwargs':{k:v for k,v in c['kwargs'].items() if k in ['xerr','yerr','bottom','height']}}
        unchanged.append(dict(figure=figure,previous_calls=len(old),current_calls=len(new),
                              all_data_calls_exact=[data(c) for c in old]==[data(c) for c in new]))
    bundle=json.loads((args.input_root/FIELD_ARRAYS).read_text())
    fields=compare(bundle,current)
    values=audit(args.input_root,capture,mapping)
    values['reviewed_plot_line_map']=mapping
    negatives=self_test(args.input_root,capture,mapping)
    negatives.update(negative_tests(bundle,current))
    negatives.update(additional_negatives(args.input_root,capture,mapping))
    values['negative_checks']=negatives
    args.value_trace.write_text(json.dumps(values,separators=(',',':'),allow_nan=False)+'\n')
    result=dict(schema_version=1,kind='independent_EV13_followup_numeric_raster_label_reverify_not_physical_validation',
                line_mapping=dict(algorithm='unique AST function/receiver/positional signatures; explicit reviewed policy changes',
                                  path=str(args.line_map),sha256=sha(args.line_map),
                                  all_observed_calls_mapped=True,observed_runtime_calls=sum(len(f['calls']) for f in capture['figures'].values()),
                                  manual_policy_changes=manual,unmapped_unused_nodes=unused),
                no_raw_runs_directory=True,capture_errors=capture['capture_errors'],
                figures={n:{k:f[k] for k in ['committed_sha256','regenerated_sha256','raster_equal','committed_size']}|
                           {'byte_equal':f['committed_sha256']==f['regenerated_sha256']} for n,f in capture['figures'].items()},
                field_array_sha256=sha(args.input_root/FIELD_ARRAYS),field_calls=14,field_array_bindings=fields,
                exact_field_arrays=sum(r['match_exact_including_null_mask'] for r in fields),
                unchanged_numeric_calls=unchanged,label_checks=labels(capture,args.input_root),
                numeric_comparison=values['summary'],numeric_trace_path=str(args.value_trace),numeric_trace_sha256=sha(args.value_trace),
                negative_checks=negatives)
    ok=(len(fields)==32 and all(r['match_exact_including_null_mask'] for r in fields)
        and len(capture['figures'])==9 and all(r['raster_equal'] and r['byte_equal'] for r in result['figures'].values())
        and all(r['all_data_calls_exact'] for r in unchanged) and all(result['label_checks'].values())
        and values['summary']['FAIL']==values['summary']['UNTRACED']==0)
    result['requested_followup_PASS']=ok
    result['all_claims_or_scientific_evidence_pass']=False
    args.output.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print(json.dumps({'requested_followup_PASS':ok,'numeric_comparison':values['summary'],
                      'exact_field_arrays':result['exact_field_arrays'],'negative_checks':negatives}))
    raise SystemExit(not ok)
