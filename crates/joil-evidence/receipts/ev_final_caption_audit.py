"""Read-only audit of the lead/hero captions at an explicit committed revision.

Numeric observations use explicit JSON fields, with subject/statistic specified.
CI confidence is encoded in the ci95 field names, not borrowed from a different
quantile's confidence leaf. Labels, section numbers and hashes are classified
separately. Run the EV13 plot-call audit as well to verify every plotted series.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

W4='docs/results/wave4-evolution-proof-v1.json'
B3='B3_Wave3_Exotic3D_QD_CEI'
B1='B1_Random_Search'


def check(root,revision,overrides=None,run_negatives=True):
    cache={}
    def blob(path):
        if path not in cache:
            cache[path]=(overrides[path] if overrides and path in overrides else
                         subprocess.check_output(['git','show',revision+':'+path],cwd=root))
        return cache[path]
    def data(path):return json.loads(blob(path))
    def at(pointer):
        value=data(W4)
        for key in pointer.lstrip('/').split('/'):
            value=value[int(key)] if isinstance(value,list) else value[key]
        return value
    readme=blob('README.md').decode();paper=blob('docs/WHITEPAPER.md').decode()
    hero=readme[readme.index('*Figure 0 (Wave-4,'):readme.index('\n## Status')]
    lead=paper[paper.index('*What the figure shows,'):paper.index('\nSix external reviews')]
    rows=[]
    def add(caption,role,token,pointer,formatter):
        expected=format(at(pointer),formatter)
        rows.append(dict(caption=caption,role=role,printed=token,source=W4+'#'+pointer,
                         source_value=at(pointer),formatted_value=expected,
                         status='PASS' if expected==token and token in (hero if caption=='README hero' else lead) else 'FAIL'))
    for name in ['README hero','WHITEPAPER lead']:
        add(name,'seeds per arm','10','/n_seeds','d')
    add('README hero','Monte Carlo samples per candidate','72','/promoted_wave3_top_elite/mc_robustness/samples','d')
    add('README hero','CT35 screen Hinze diameter, micrometres','64.0','/baseline_b0_ct35/nominal_metrics/hinze_dmax_um','.1f')
    add('README hero','initial generation index','0',f'/generation_band_curves/{B3}/0/generation','d')
    add('WHITEPAPER lead','charged proposals per seed and arm','450','/evaluations_per_seed_per_arm','d')
    for role,token,pointer in [
        ('B1 final best KPI, seed mean','45.936820',f'/final_summary_statistics/{B1}/best_kpi_score/mean'),
        ('B3 final best KPI, seed mean','45.131872',f'/final_summary_statistics/{B3}/best_kpi_score/mean'),
        ('best KPI B3>B1, Holm p','1.0','/pairwise_tests/0/p_value_holm_bonferroni'),
        ('best KPI B3>B1, Cliff delta','-0.68','/pairwise_tests/0/cliffs_delta'),
        ('population median B3>B1, Holm p','0.0012568','/pairwise_tests/1/p_value_holm_bonferroni'),
        ('B3 final population median KPI, seed mean','43.628366',f'/final_summary_statistics/{B3}/median_kpi_score/mean'),
        ('B1 final population median KPI, seed mean','37.983620',f'/final_summary_statistics/{B1}/median_kpi_score/mean'),
        ('CT35 frozen reference KPI','23.931038','/baseline_b0_ct35_kpi_score')]:
        add('WHITEPAPER lead',role,token,pointer,f'.{len(token.partition(".")[2])}f')
    # Verify comparator identity, not just a coincident scalar in the dossier.
    for index,metric in [(0,'final_best_kpi_score'),(1,'final_median_kpi_score')]:
        assert at(f'/pairwise_tests/{index}/comparator_a')==B3
        assert at(f'/pairwise_tests/{index}/comparator_b')==B1
        assert at(f'/pairwise_tests/{index}/metric')==metric
    ci_refs=[W4+f'#/generation_band_curves/{arm}/24/best_kpi_score/{key}'
             for arm in [B3,B1] for key in ['ci95_low','ci95_high']]
    declarations=[
        dict(caption='WHITEPAPER lead',printed='95%',role='bootstrap interval confidence',sources=ci_refs,
             interpretation='ci95_low/ci95_high field-name semantics, not a separate scalar confidence leaf; intervals are for the seed mean',
             status='PASS_DECLARED_FIELD_SEMANTICS' if '95%' in lead else 'FAIL'),
        dict(caption='WHITEPAPER lead',printed='single frozen reference evaluation',role='reference evaluation count per seed',
             source=W4+f'#/arm_outcomes/{B3}/0/reference_evaluations',source_value=at(f'/arm_outcomes/{B3}/0/reference_evaluations'),
             status='PASS' if at(f'/arm_outcomes/{B3}/0/reference_evaluations')==1 and 'single frozen reference evaluation' in lead else 'FAIL')]
    figures=[]
    for document,text,path in [('README.md',readme,'docs/figures/wave4-readme-hero-kpi-and-elite.png'),
                               ('docs/WHITEPAPER.md',paper,'docs/figures/wave4-page1-why-it-matters.png')]:
        png=blob(path);manifest_path=path+'.manifest.json';m=data(manifest_path)
        actual=hashlib.sha256(png).hexdigest()
        sources=[dict(path=s['path'],sha256=s['sha256'],match=hashlib.sha256(blob(s['path'])).hexdigest()==s['sha256']) for s in m['source_receipts']]
        figures.append(dict(document=document,path=path,exists=bool(png),png_sha256=actual,
                            manifest_path=manifest_path,manifest_sha256=hashlib.sha256(blob(manifest_path)).hexdigest(),
                            png_matches_manifest=actual==m['figure_sha256'],digest_listed_in_document=actual in text,
                            source_receipts=sources))
    # Negative checks change the named source fields and ensure caption binding
    # fails. No source file is changed, and no result is taken from pixels.
    negatives={}
    if run_negatives:
        changed=data(W4)
        changed['final_summary_statistics'][B3]['best_kpi_score']['mean']=at(f'/final_summary_statistics/{B1}/best_kpi_score/mean')
        negatives['wrong_best_arm_reject']=not check(root,revision,{W4:json.dumps(changed).encode()},False)['PASS']
        changed=data(W4)
        changed['pairwise_tests'][0]['p_value_holm_bonferroni']=at('/pairwise_tests/1/p_value_holm_bonferroni')
        negatives['wrong_pvalue_metric_reject']=not check(root,revision,{W4:json.dumps(changed).encode()},False)['PASS']
        png_path=figures[0]['path']
        negatives['wrong_png_bytes_reject']=not check(root,revision,{png_path:blob(png_path)+b'changed'},False)['PASS']
    return dict(schema_version=1,input_head=subprocess.check_output(['git','rev-parse',revision],cwd=root,text=True).strip(),
                kind='independent_final_lead_and_hero_caption_trace_not_physical_validation',
                captions={'README hero':hero,'WHITEPAPER lead':lead},numeric_observations=rows,
                confidence_and_spelled_count=declarations,figures=figures,
                nonmeasurement_numerals=['Figure0=index','Wave1/2/3/4=campaign identifiers','F1=fidelity label',
                                        'CT35=baseline designation','A1/A2/B1/B2/B3=arm identifiers',
                                        'PanelA/B/C/D=panel indices','sections4.6/5.7=locations','correctionC4=review index',
                                        'SHA256=hash algorithm; full opaque digests independently verified'],
                negative_checks=negatives,all_claims_or_physical_PASS=False,
                PASS=(all(r['status'].startswith('PASS') for r in rows+declarations)
                      and all(f['exists'] and f['png_matches_manifest'] and f['digest_listed_in_document']
                              and all(s['match'] for s in f['source_receipts']) for f in figures)
                      and all(negatives.values())))


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--root',type=Path,default=Path('.'))
    ap.add_argument('--revision',required=True)
    ap.add_argument('--output',type=Path,required=True)
    args=ap.parse_args();result=check(args.root,args.revision)
    args.output.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print(json.dumps({'input_head':result['input_head'],'PASS':result['PASS'],'numeric_observations':len(result['numeric_observations']),'figures':len(result['figures'])}))
    raise SystemExit(not result['PASS'])
