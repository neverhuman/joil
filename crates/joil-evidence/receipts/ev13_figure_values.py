"""Independent receipt projections for the EV13 captured plotting calls.

No generator extraction helper is used here. Each comparison records explicit
RFC6901 source pointers, the transformation, expected values and captured values.
Capture line numbers refer to the immutable source inventory in the receipt.
Missing/extra calls and numeric differences fail closed; raw-field arrays with
no public JSON source remain explicit UNTRACED rows, never numeric PASS.
"""
import argparse
import copy
import hashlib
import json
import math
from pathlib import Path

import numpy as np

B3 = 'B3_Wave3_Exotic3D_QD_CEI'
B1 = 'B1_Random_Search'
A1 = 'A1_No_Mutation_Crossover_Only'
A2 = 'A2_No_Selection_Random_Walk'
B2 = 'B2_Conventional_Axisym_NSGA2'
W4 = 'docs/results/wave4-evolution-proof-v1.json'
W3 = 'docs/results/wave3-evolution-proof-v1.json'
W2 = 'docs/results/wave2-multiseed-ablation-and-holdout-v1.json'
G1 = 'docs/results/elite-3grid-gci-and-rsm-verification-v2.json'
G2 = 'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json'
G4 = 'docs/results/wave4-gci-v1.json'
FIELD_ARRAYS = 'crates/joil-evidence/receipts/Plate10-field-arrays.json'
TP = 'wave2_tpms_gyroid_elite_5run_dossier'
OC = 'wave2_octa_branch_8jet_elite_5run_dossier'


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def same(actual, expected):
    if isinstance(expected, list):
        return (isinstance(actual, list) and len(actual) == len(expected)
                and all(same(a, b) for a, b in zip(actual, expected)))
    if isinstance(expected, (int, float)):
        return (isinstance(actual, (int, float)) and math.isfinite(actual)
                and math.isclose(actual, expected, rel_tol=1e-12, abs_tol=1e-12))
    return actual == expected


def dispatch_line(call, line_map):
    """An explicitly reviewed new->original line map preserves old audit pins."""
    return line_map.get(call['source'], {}).get(str(call['line']), call['line'])


class Audit:
    def __init__(self, root, capture, line_map=None):
        self.root, self.capture = Path(root), copy.deepcopy(capture)
        for figure in self.capture['figures'].values():
            for call in figure['calls']:
                call['source_line_at_capture'] = call['line']
                call['line'] = dispatch_line(call, line_map or {})
        self.rows, self.covered = [], set()
        self.cache = {}

    def get(self, file, pointer=''):
        if file not in self.cache:
            self.cache[file] = json.loads((self.root/file).read_text())
        v = self.cache[file]
        for key in pointer.lstrip('/').split('/') if pointer else []:
            key = key.replace('~1', '/').replace('~0', '~')
            v = v[int(key)] if isinstance(v, list) else v[key]
        return v

    def refs(self, file, pointers):
        return [file+'#'+p for p in pointers]

    def add(self, figure, label, actual, expected, refs, transform='identity'):
        self.rows.append(dict(figure=figure, series=label, sources=refs,
                              transform=transform, expected=expected,
                              captured=actual, status='PASS' if same(actual, expected) else 'FAIL'))

    def calls(self, fig, line):
        return [(i, c) for i, c in enumerate(self.capture['figures'][fig]['calls']) if c['line'] == line]

    def check(self, fig, idx, part, expected, refs, transform='identity'):
        c = self.capture['figures'][fig]['calls'][idx]
        actual = c['kwargs'][part] if isinstance(part, str) else c['args'][part]
        self.covered.add((fig, idx))
        self.add(fig, f"{c['method']}@{c['source']}:{c['source_line_at_capture']} call{idx} {part}", actual, expected, refs, transform)

    def vector(self, fig, idx, part, file, pointers, factor=1):
        expected = [self.get(file, p)*factor for p in pointers]
        self.check(fig, idx, part, expected, self.refs(file, pointers), f'values * {factor}')

    def text(self, fig, axis, expected, refs, context=None):
        f = self.capture['figures'][fig]
        texts = f['figure_text'] if axis is None else sum(
            [f['axes'][axis][k] for k in ['text', 'legend', 'xlabels', 'ylabels']], []) + [
                f['axes'][axis]['title'], f['axes'][axis]['xlabel'], f['axes'][axis]['ylabel']]
        if context is not None:
            texts=[line for t in texts for line in t.splitlines() if context in line]
        self.add(fig, f'text axis{axis}: {context or ""} {expected}', any(expected in t for t in texts), True, refs, 'formatted text occurrence in specified axis/comparison')

    def proof(self, wave):
        file = W4 if wave == 'wave4' else W3
        d = self.get(file)
        best_c = 'best_cout_sample_screen_upper_mg_l' if wave == 'wave4' else 'best_cout_p95_mg_l'
        med_c = best_c.replace('best_', 'median_')
        base_c = 'baseline_b0_ct35_cout_sample_screen_upper_mg_l' if wave == 'wave4' else 'baseline_b0_ct35_cout_p95_mg_l'
        hero = wave+'-readme-hero-kpi-and-elite.png'
        plate = wave+'-evolution-ablation-and-lineage-proof.png'
        gptr = [f'/generation_band_curves/{B3}/{j}/generation' for j in range(len(d['generation_band_curves'][B3]))]
        for fig in [hero, plate]:
            for i, c in enumerate(self.capture['figures'][fig]['calls']):
                line, key = c['line'], c['bindings'].get('key')
                if fig == hero and line in [146, 179]:
                    seed = c['bindings']['seed']
                    oi = next(j for j, o in enumerate(d['arm_outcomes'][B3]) if o['seed'] == seed)
                    metric = 'best_kpi_score' if line == 146 else best_c
                    ps = [f'/arm_outcomes/{B3}/{oi}/generation_trace/{j}/{metric}' for j in range(len(gptr))]
                    self.vector(fig, i, 0, file, gptr)
                    self.vector(fig, i, 1, file, ps)
                elif (fig == hero and line in [153,154,187,188]) or (fig == plate and line in [599,600]):
                    metric = 'best_kpi_score' if line in [153,154] else best_c if line in [187,188] else 'shannon_diversity_bits'
                    self.vector(fig, i, 0, file, gptr)
                    stats = ['ci95_low','ci95_high'] if c['method']=='fill_between' else ['mean']
                    for part, stat in enumerate(stats, 1):
                        ps = [f'/generation_band_curves/{key}/{j}/{metric}/{stat}' for j in range(len(gptr))]
                        self.vector(fig, i, part, file, ps)
                    if stats == ['mean']:
                        self.text(fig, c['axis'], f"{self.get(file, ps[-1]):.2f}", self.refs(file, [ps[-1]]))
                elif fig == hero and line in [160,161,162,192,193]:
                    key = {160:B3,161:B1,162:A2,192:B3,193:B1}[line]
                    metric = 'median_kpi_score' if line < 190 else med_c
                    self.vector(fig,i,0,file,gptr)
                    ps=[f'/generation_band_curves/{key}/{j}/{metric}/mean' for j in range(len(gptr))]
                    self.vector(fig,i,1,file,ps)
                    self.text(fig,c['axis'],f'{self.get(file,ps[-1]):.2f}',self.refs(file,[ps[-1]]))
                elif fig == hero and line in [141,175] or fig == plate and line in [457,641]:
                    p='/baseline_b0_ct35_kpi_score' if line in [141,457,641] else '/'+base_c
                    self.check(fig,i,1 if line==457 else 0,[self.get(file,p)] if line==457 else self.get(file,p),[file+'#'+p])
                elif fig == plate and line == 469:
                    ps=[f'/arm_outcomes/{key}/{j}/final_best_kpi_score' for j in range(len(d['arm_outcomes'][key]))]
                    self.vector(fig,i,1,file,ps)
                elif fig == plate and line == 481:
                    order=[B2,A2,A1,B1,B3]
                    ps=['/baseline_b0_ct35_kpi_score']+[f'/final_summary_statistics/{k}/best_kpi_score/mean' for k in order]
                    means=[self.get(file,p) for p in ps]
                    self.vector(fig,i,1,file,ps)
                    lo=[f'/final_summary_statistics/{k}/best_kpi_score/ci95_low' for k in order]
                    hi=[p.replace('ci95_low','ci95_high') for p in lo]
                    err=[[0.]+[max(0.,m-self.get(file,p)) for m,p in zip(means[1:],lo)],
                         [0.]+[max(0.,self.get(file,p)-m) for m,p in zip(means[1:],hi)]]
                    self.check(fig,i,'yerr',err,self.refs(file,ps+lo+hi),'asymmetric mean-CI errors; frozen reference has zero CI')
                    for p,m in zip(ps,means):self.text(fig,0,f'{m:.2f}',[file+'#'+p])
                elif fig == plate and line in [533,534,544,552]:
                    oi=c['bindings']['s_idx'] if line in [533,534] else max(range(len(d['arm_outcomes'][B3])),key=lambda j:d['arm_outcomes'][B3][j]['final_best_kpi_score'])
                    ms=d['arm_outcomes'][B3][oi]['lineage_milestones']
                    for part,metric in enumerate(['generation','kpi_score']):
                        self.vector(fig,i,part,file,[f'/arm_outcomes/{B3}/{oi}/lineage_milestones/{j}/{metric}' for j in range(len(ms))])
                elif fig == plate and line == 630:
                    indices=sorted(range(len(d['held_out_geometry_generalization'])),key=lambda j:d['held_out_geometry_generalization'][j]['held_out_best_kpi_score'])
                    ps=[f'/held_out_geometry_generalization/{j}/held_out_best_kpi_score' for j in indices]
                    self.vector(fig,i,1,file,ps)
                    if wave=='wave3':
                        p_err=[f'/held_out_geometry_generalization/{j}/lofo_surrogate_rmse_kpi_score' for j in indices]
                        self.vector(fig,i,'xerr',file,p_err)
                    else:
                        self.check(fig,i,'xerr',None,self.refs(file,[f'/held_out_geometry_generalization/{j}/sandbox_authored_family_rmse_kpi_score' for j in indices]),'all KPI RMSE null; no error bars')
        # Exact displayed tests; hardcoded narrative is evaluated separately as a finding.
        for ti,t in enumerate(d['pairwise_tests']):
            if t['comparator_a']!=B3 or (t['metric'],t['comparator_b']) not in [
                ('final_best_kpi_score',B1),('final_median_kpi_score',B1),('final_best_kpi_score',A1),('final_best_kpi_score',A2)]:continue
            for field,fmt,prefix in [('mean_diff','+.2f','Δ='),('mann_whitney_u','.0f','U='),('p_value_holm_bonferroni','.4g','p_Holm='),('cliffs_delta','+.2f','δ=')]:
                context=('B3 vs B1 (Pop-Median' if t['metric']=='final_median_kpi_score' else
                         'B3 vs '+{B1:'B1 (Best',A1:'A1 (No-Mut',A2:'A2 (No-Sel'}[t['comparator_b']])
                self.text(plate,0,prefix+format(t[field],fmt),[f'{file}#/pairwise_tests/{ti}/{field}'],context)
        topi=max(range(len(d['arm_outcomes'][B3])),key=lambda j:d['arm_outcomes'][B3][j]['final_best_kpi_score'])
        top=d['arm_outcomes'][B3][topi];ms=top['lineage_milestones'];prefix=f'/arm_outcomes/{B3}/{topi}'
        self.text(plate,1,f"Top Seed {top['seed']}",[file+'#'+prefix+'/seed'])
        self.text(plate,1,f'{len(ms)} Milestones',[file+'#'+prefix+'/lineage_milestones'])
        for j in sorted({0,len(ms)//3,2*len(ms)//3,len(ms)-1}):
            m=ms[j];p=prefix+f'/lineage_milestones/{j}'
            self.text(plate,1,f"g={m['generation']}: {m['child_topology_class']}\n({m['total_entry_jets']} jets, TPMS={m['tpms_basis']}, KPI={m['kpi_score']:.2f})",[file+'#'+p])
        for j,f in enumerate(d['held_out_geometry_generalization']):
            p=f'/held_out_geometry_generalization/{j}'
            self.text(plate,3,f"{f['held_out_best_kpi_score']:.2f} ({f['de_novo_rediscovery_seeds']}/10 seeds",[file+'#'+p+'/held_out_best_kpi_score',file+'#'+p+'/de_novo_rediscovery_seeds',file+'#/n_seeds'])
            if wave=='wave4':self.text(plate,3,f"Cout RMSE={f['sandbox_authored_family_rmse_cout_mg_l']:.2f} mg/L",[file+'#'+p+'/sandbox_authored_family_rmse_cout_mg_l'])
        self.hero_cfd(hero,wave)
        if wave=='wave4':self.hero_labels(hero)

    def hero_labels(self,fig):
        ps=['/n_seeds','/total_3d_evaluations_across_arms','/promoted_wave3_top_elite/mc_robustness/samples']
        for p,fmt in zip(ps,['d',',d','d']):self.text(fig,None,format(self.get(W4,p),fmt),[W4+'#'+p])
        for p,fmt in [('/baseline_b0_ct35_kpi_score','.2f'),('/baseline_b0_ct35_cout_sample_screen_upper_mg_l','.2f'),('/baseline_b0_ct35/nominal_metrics/hinze_dmax_um','.2f'),('/promoted_wave3_top_elite/nominal_metrics/hinze_dmax_um','.2f'),('/promoted_wave3_top_elite/program/tpms_insert/radial_inner_fraction','.4f'),('/promoted_wave3_top_elite_kpi_score','.2f')]:
            self.text(fig,2,format(self.get(W4,p),fmt),[W4+'#'+p])
        p='/promoted_wave3_top_elite/mc_robustness/cout_sample_screen_upper_mg_l'
        self.text(fig,2,f'{self.get(W4,p):.2f}',[W4+'#'+p])
        base=self.get(W4,'/baseline_b0_ct35_cout_sample_screen_upper_mg_l')
        self.text(fig,2,f'{(self.get(W4,p)-base)/base*100:.1f}%', [W4+'#'+p,W4+'#/baseline_b0_ct35_cout_sample_screen_upper_mg_l'])
        p='/promoted_wave3_top_elite/program/inlet_split/branches_per_port'
        self.text(fig,2,f'{2*self.get(W4,p)} jets',[W4+'#'+p,W4+'#/promoted_wave3_top_elite/program/inlet_split/enabled'])

    def hero_cfd(self,fig,wave):
        manifest=self.get('docs/figures/'+fig+'.manifest.json')
        file=next(s['path'] for s in manifest['source_receipts'] if Path(s['path']).name.startswith('wave2-tpms-elite-gci-rsm-and-f2-oil-'))
        data=self.get(file);rows=data['wave1_vs_wave2_surrogate_to_cfd_ranking_audit']
        order=['trifurcated-6jet-vortex-head','cylindrical-gyroid-coalescer','octa-branch-helical-manifold']
        ids=[next(j for j,r in enumerate(rows) if r['topology_class']==name) for name in order]
        base=f'/{TP}/baseline_ct35_comparator/dp_u_pa'
        for line,suffix in [(350,'g2'),(359,'lrr'),(360,'ssg')]:
            ps=[f'/wave1_vs_wave2_surrogate_to_cfd_ranking_audit/{j}/openfoam_{suffix}_dp_u_pa' for j in ids]
            if suffix=='g2':ps=[base]+ps
            i=self.calls(fig,line)[0][0];self.vector(fig,i,1,file,ps,.001)
            if line==350:
                if file.endswith('-v2.json'):
                    # Recompute the actual half range, rather than taking its summary label.
                    errors=[0.];refs=[]
                    for f,prefix in [(G1,''),(file,'/'+TP),(file,'/'+OC)]:
                        gps=[prefix+f'/grids/{g}/dp_u_pa' for g in range(3)]
                        vals=[self.get(f,p) for p in gps];errors.append((max(vals)-min(vals))/2000)
                        refs+=self.refs(f,gps)
                    self.check(fig,i,'yerr',errors,refs,'(max(grid dp)-min(grid dp))/2/1000; CT35 reference no CI')
                else:
                    ps_err=['/'+TP+'/roache_gci_analysis/dp_u_pa/gci21_standard_fs125_pct',
                            '/'+OC+'/roache_gci_analysis/dp_u_pa/gci21_standard_fs125_pct']
                    vals=[self.get(file,p)*.001 for p in ps]
                    errors=[0.,vals[1]*5.019376028759711/100]+[vals[j+2]*self.get(file,p)/100 for j,p in enumerate(ps_err)]
                    self.check(fig,i,'yerr',errors,self.refs(file,ps+ps_err)+['tools/generate_wave3_proof_figures.py#historical-wave3-main-w1_gci_pct=5.019376028759711'],
                               'historical clamped v1 GCI plus authored W1 constant; NOT admitted GCI (see findings)')
                    self.rows[-1]['numeric_match']=self.rows[-1]['status']=='PASS'
                    self.rows[-1]['status']='UNTRACED'
                    self.rows[-1]['reason']='W1 error-bar scalar is hardcoded; no committed result JSON pointer for the supplied constant; historical PNG also differs from current replay'
                for j,p in enumerate(ps):
                    val=self.get(file,p)/1000
                    self.text(fig,3,f'{val:.2f} kPa',[file+'#'+p])
                    if j:self.text(fig,3,f'{(self.get(file,p)/self.get(file,base)-1)*100:.1f}%',self.refs(file,[p,base]))

    def page1(self):
        fig='wave4-page1-why-it-matters.png';d=self.get(W4);n=len(d['generation_band_curves'][B3])
        gs=[f'/generation_band_curves/{B3}/{j}/generation' for j in range(n)]
        counters={B1:0,B3:0}
        for i,c in enumerate(self.capture['figures'][fig]['calls']):
            line=c['line'];k={'B1':B1,'B3':B3}.get(c['bindings'].get('name'))
            if line==91:self.check(fig,i,0,d['baseline_b0_ct35_kpi_score'],[W4+'#/baseline_b0_ct35_kpi_score']);continue
            self.vector(fig,i,0,W4,gs)
            if line==84:
                oi=counters[k];counters[k]+=1
                ps=[f'/arm_outcomes/{k}/{oi}/generation_trace/{j}/best_kpi_score' for j in range(n)]
                self.vector(fig,i,1,W4,ps)
            else:
                metric='median_kpi_score' if line==89 else 'best_kpi_score'
                for part,stat in enumerate(['ci95_low','ci95_high'] if line==85 else ['mean'],1):
                    self.vector(fig,i,part,W4,[f'/generation_band_curves/{k}/{j}/{metric}/{stat}' for j in range(n)])
        for k,short in [(B1,'B1'),(B3,'B3')]:
            for metric in ['best_kpi_score','median_kpi_score']:
                for st in ['mean','ci95_low','ci95_high'] if metric=='best_kpi_score' else ['mean']:
                    p=f'/generation_band_curves/{k}/{n-1}/{metric}/{st}'
                    self.text(fig,0,f'{self.get(W4,p):.2f}',[W4+'#'+p])
        for ti,t in enumerate(d['pairwise_tests']):
            if t['comparator_b']==B1 and t['metric'] in ['final_best_kpi_score','final_median_kpi_score']:
                for k,fmt,prefix in [('mann_whitney_u','g','U = '),('p_value_holm_bonferroni','g','p_Holm = '),('cliffs_delta','+.2f','δ = ')]:
                    self.text(fig,0,prefix+format(t[k],fmt),[f'{W4}#/pairwise_tests/{ti}/{k}'])
        for p in ['/n_seeds','/population_per_generation','/evaluations_per_seed_per_arm']:
            self.text(fig,0,str(self.get(W4,p)),[W4+'#'+p])

    def wave4_gci(self):
        fig='wave4-gci-same-pipeline-control.png'
        control='/same_pipeline_ct35_control'
        for i,c in enumerate(self.capture['figures'][fig]['calls']):
            line=c['line']
            if line==90:
                p=control+'/g09_pipeline_confounding_audit/historical_extended_ct35_reference_dp_u_pa'
                self.check(fig,i,0,self.get(G4,p)/1000,[G4+'#'+p],'Pa / 1000');continue
            if line==81:file,prefix=G4,control
            elif line==89:
                file,prefix={'Wave-1 shell (label 6-jet)':(G1,''),'Wave-2 shell (label gyroid)':(G2,'/'+TP),'Wave-2 shell (label 8-jet)':(G2,'/'+OC)}[c['bindings']['name']]
            else:
                winners=self.get(G4,'/wave4_b3_top_winners')
                j=next(j for j,w in enumerate(winners) if w['subject_id']==c['bindings']['name'])
                file,prefix=G4,f'/wave4_b3_top_winners/{j}'
                self.text(fig,1,f"{self.get(file,prefix+'/enactment_receipt/measured_disjoint_inlet_paths')} inlet paths",[file+'#'+prefix+'/enactment_receipt/measured_disjoint_inlet_paths'])
            for part,metric,factor in [(0,'effective_cell_size_mm',1),(1,'dp_u_pa',.001)]:
                self.vector(fig,i,part,file,[prefix+f'/grids/{g}/{metric}' for g in range(3)],factor)
        winners=self.get(G4,'/wave4_b3_top_winners')
        self.text(fig,1,f"{sum(w['solver_status']!='Solved3Grid' for w in winners)} other top designs not solved",[G4+'#/wave4_b3_top_winners'])

    def plate3(self):
        fig='elite-gci-and-rsm-verification.png'
        prefixes=[f'/grids/{j}' for j in range(3)]+[f'/rsm_turbulence_closure_crosscheck/{j}' for j in range(2)]+['/baseline_ct35_comparator']
        for i,c in enumerate(self.capture['figures'][fig]['calls']):
            line=c['line']
            if line in [533,534]:
                p='/baseline_ct35_comparator/'+('dp_u_pa' if line==533 else 'hydraulic_loss_j_per_treated_m3')
                self.check(fig,i,0,self.get(G1,p)/1000,[G1+'#'+p],'Pa or J/m3 divided by1000')
            elif line in [535,536]:
                hps=[f'/grids/{j}/effective_cell_size_mm' for j in [2,1,0]]
                self.check(fig,i,0,[self.get(G1,p)/self.get(G1,hps[0]) for p in hps],self.refs(G1,hps),'h / fine-grid h')
                metric='dp_u_pa' if line==535 else 'hydraulic_loss_j_per_treated_m3'
                self.vector(fig,i,1,G1,[f'/grids/{j}/{metric}' for j in [2,1,0]],.001)
            elif line==539:
                ps=[p+'/dp_u_pa' for p in prefixes[:3]];vals=[self.get(G1,p)/1000 for p in ps]
                self.check(fig,i,1,min(vals),self.refs(G1,ps),'min(dp)/1000')
                self.check(fig,i,2,max(vals),self.refs(G1,ps),'max(dp)/1000')
            elif line in [576,585,615,616]:
                metric,factor={576:('reject_fraction',100),585:('water_closure_fraction',1e6),615:('dp_u_pa',.001),616:('hydraulic_loss_j_per_treated_m3',.001)}[line]
                self.vector(fig,i,1,G1,[p+'/'+metric for p in prefixes],factor)
            elif line in [638,639,640,643]:
                p='/rsm_turbulence_closure_crosscheck/'+('1' if line==643 else '0')+'/reynolds_stress_tensor_diagnostics/'+{638:'quantiles_rxx',639:'quantiles_ryy',640:'quantiles_rzz',643:'quantiles_rzz'}[line]
                self.check(fig,i,1,self.get(G1,p),[G1+'#'+p])
        for axis,metric,fmt,factor in [(1,'reject_fraction','.3f',100),(3,'dp_u_pa','.2f',.001),(3,'hydraulic_loss_j_per_treated_m3','.2f',.001)]:
            for p in prefixes:self.text(fig,axis,format(self.get(G1,p+'/'+metric)*factor,fmt),[G1+'#'+p+'/'+metric])
        prefix='/rsm_turbulence_closure_crosscheck/0/reynolds_stress_tensor_diagnostics/'
        for k,fmt,factor in [('mean_lumley_anisotropy_norm','.3f',1),('p95_lumley_anisotropy_norm','.3f',1),('mean_axial_to_transverse_normal_stress_ratio','.3f',1),('positive_definite_normal_stress_fraction','.2f',100)]:
            self.text(fig,4,format(self.get(G1,prefix+k)*factor,fmt),[G1+'#'+prefix+k])
        for j in range(3):
            p=f'/grids/{j}/cells'
            self.text(fig,0,f'{round(self.get(G1,p)/1000)}k',[G1+'#'+p])
        for p,fmt in [('/roache_gci_analysis/dp_u_pa/convergence_ratio_R','.3f'),('/roache_gci_analysis/dp_u_pa/three_grid_half_span_pct_of_f2','.2f')]:
            self.text(fig,0,format(self.get(G1,p),fmt),[G1+'#'+p])
        self.text(fig,4,f"{self.get(G1,prefix+'cell_count'):,}",[G1+'#'+prefix+'cell_count'])
        label=self.capture['figures'][fig]['axes'][2]['ylabel']
        if 'Gate' in label:
            self.rows.append(dict(figure=fig,series='mass-closure numerical gate annotation',status='UNTRACED',sources=[],
                                  captured_label=label,code_reference='analysis/kpi_fitness.py:383',
                                  reason='1000 in units 1e-4 percent equals the implemented0.001 numerical tolerance, but no result/manifest JSON pointer binds that policy scalar; not a physical requirement'))
        else:self.add(fig,'unsupported mass-closure gate annotation removed','Gate' in label,False,[],'non-data annotation coverage')

    def wave2(self):
        fig='wave2-multiseed-ablation-and-ci.png';d=self.get(W2)
        armmap={'arm_b2_conventional_axisym_outcomes':'B2_Conventional_Axisym','arm_b1_random_lhs_outcomes':'B1_Random_LHS','arm_b3_exotic3d_qd_cei_outcomes':'B3_Exotic3D_QD_CEI'}
        order=list(armmap.values());meanps=['/baseline_b0_ct35/mc_robustness/cout_p95_mg_l']+[f'/summary_statistics/{k}/cout_p95_mg_l/mean' for k in order]
        means=[self.get(W2,p) for p in meanps]
        for i,c in enumerate(self.capture['figures'][fig]['calls']):
            line=c['line']
            if line in [296,297]:
                k=c['bindings']['key'];curves=np.array([o['hypervolume_by_generation'] for o in d[k]])
                ps=[f'/{k}/{j}/hypervolume_by_generation' for j in range(len(curves))]
                self.check(fig,i,0,list(range(curves.shape[1])),self.refs(W2,ps),'generation indices')
                mean=np.mean(curves,axis=0);err=1.96*np.std(curves,axis=0,ddof=1)/math.sqrt(len(curves))
                for part,v in enumerate([mean-err,mean+err] if line==297 else [mean],1):
                    self.check(fig,i,part,v.tolist(),self.refs(W2,ps),'seed arithmetic mean +/-1.96 sample SE (not bootstrap)')
            elif line in [341,343]:
                self.check(fig,i,1,means,self.refs(W2,meanps))
                if line==343:
                    low=[f'/summary_statistics/{k}/cout_p95_mg_l/ci95_low' for k in order];high=[p.replace('ci95_low','ci95_high') for p in low]
                    err=[[0]+[m-self.get(W2,p) for m,p in zip(means[1:],low)],[0]+[self.get(W2,p)-m for m,p in zip(means[1:],high)]]
                    self.check(fig,i,'yerr',err,self.refs(W2,meanps+low+high),'asymmetric CI around mean')
            elif line==348:
                k=list(armmap)[c['bindings']['idx']-1]
                self.vector(fig,i,1,W2,[f'/{k}/{j}/best_cout_p95_mg_l' for j in range(len(d[k]))])
            elif line in [394,395]:
                key='baseline_b0_held_out_duties' if line==394 else 'promoted_wave2_top_elite_held_out_duties'
                self.vector(fig,i,1,W2,[f'/{key}/{j}/cout_mg_l' for j in range(4)])
                for j in range(4):self.text(fig,2,f'{d[key][j]["cout_mg_l"]:.1f}',[f'{W2}#/{key}/{j}/cout_mg_l'])
            elif line==427:
                ps=[f'/summary_statistics/{k}/map_elites_cells/mean' for k in [order[0],order[2],order[1]]]
                self.vector(fig,i,1,W2,ps)
                for p in ps:self.text(fig,3,f'{self.get(W2,p):.1f} cells',[W2+'#'+p])
        for ti,t in enumerate(d['pairwise_tests'][:2]):
            for k,fmt in [('mann_whitney_u','.0f'),('p_value_holm_bonferroni','.5f'),('cliffs_delta','.2f')]:
                self.text(fig,1,format(t[k],fmt),[f'{W2}#/pairwise_tests/{ti}/{k}'],f'B3 vs B{ti+1}')
        for p,m in zip(meanps,means):self.text(fig,1,f'{m:.2f} mg/L',[W2+'#'+p])
        for j,k,fmt in [(0,'q_in_m3_h','.1f'),(1,'q_in_m3_h','.1f'),(2,'d50_feed_um','.1f'),(3,'cin_mg_l','.0f'),(3,'viscosity_factor','.2f')]:
            p=f'/promoted_wave2_top_elite_held_out_duties/{j}/{k}'
            self.text(fig,2,format(self.get(W2,p),fmt),[W2+'#'+p])
        for p,fmt,mult in [('/rejected_cohort_10pct_audit/total_rejected_or_low_rank_pool','d',1),('/rejected_cohort_10pct_audit/audited_sample_count','d',1),('/rejected_cohort_10pct_audit/false_omission_rank0_count','d',1),('/rejected_cohort_10pct_audit/false_omission_rate','.2f',100),('/rejected_cohort_10pct_audit/worst_rank_reversal_cout_mg_l','.2f',1)]:
            self.text(fig,3,format(self.get(W2,p)*mult,fmt),[W2+'#'+p])

    def plate10(self):
        fig='wave2-vs-wave1-openfoam14-cfd-comparison.png'
        manifest=self.get('docs/figures/'+fig+'.manifest.json')
        bound_fields=manifest['source_receipts'].get('field_array_bundle') == FIELD_ARRAYS
        # Explicit subject/observable bindings from the independently reviewed
        # legacy calls. The current generator's extraction helper is not used.
        field_calls={
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
        for i,c in enumerate(self.capture['figures'][fig]['calls']):
            line=c['line']
            if line==340:
                p='/baseline_ct35_comparator/dp_u_pa';self.check(fig,i,0,self.get(G1,p)/1000,[G1+'#'+p],'Pa /1000')
            elif line in [341,349,357]:
                file,prefix={341:(G1,''),349:(G2,'/'+TP),357:(G2,'/'+OC)}[line]
                ps=[prefix+f'/grids/{j}/dp_u_pa' for j in range(3)]+[prefix+f'/rsm_turbulence_closure_crosscheck/{j}/dp_u_pa' for j in range(2)]
                self.vector(fig,i,1,file,ps,.001)
                for metric,fmt in [('convergence_ratio_R','.3f' if line==341 else '.4f'),('three_grid_half_span_pct_of_f2','.2f')]:
                    p=prefix+'/roache_gci_analysis/dp_u_pa/'+metric
                    self.text(fig,8,format(self.get(file,p),fmt),[file+'#'+p])
            elif bound_fields and line in field_calls:
                case,section,keys=field_calls[line]
                for part,key in enumerate(keys):
                    pointer=f'/cases/{case}/{section}/{key}'
                    self.check(fig,i,part,self.get(FIELD_ARRAYS,pointer),[FIELD_ARRAYS+'#'+pointer],
                               'identity including JSON null mask; legacy coordinates, not corrected FV centroids')
            else:
                self.covered.add((fig,i))
                self.rows.append(dict(figure=fig,series=f'{c["method"]}@{c["source"]}:{c["source_line_at_capture"]}',sources=[],status='UNTRACED',reason='private U/p/k/mesh-derived field or profile array; no committed numeric JSON source',array_digest=hashlib.sha256(json.dumps(c['args'],sort_keys=True).encode()).hexdigest()))
        for axis,file,prefix in [(0,G1,''),(1,G2,'/'+TP)]:
            for key,fmt,factor in [('cells',',d',1),('dp_u_pa','.2f',.001)]:
                p=prefix+'/grids/1/'+key;self.text(fig,axis,format(self.get(file,p)*factor,fmt),[file+'#'+p])
        for axis,file,prefix in [(3,G1,''),(4,G2,'/'+TP)]:
            p=prefix+'/grids/1/hydraulic_loss_j_per_treated_m3'
            self.text(fig,axis,format(self.get(file,p)/1000,'.2f'),[file+'#'+p])

    def manifests(self):
        for fig in self.capture['figures']:
            path='docs/figures/'+fig+'.manifest.json'
            m=self.get(path)
            self.add(fig,'manifest figure digest',m['figure_sha256'],digest(self.root/'docs/figures'/fig),[path+'#/figure_sha256'],'SHA256 committed PNG')
            self.add(fig,'capture input PNG digest',self.capture['figures'][fig]['committed_sha256'],digest(self.root/'docs/figures'/fig),[path+'#/figure_sha256'],'captured PNG identity equals immutable input PNG')
            if 'figure_bytes' in m:self.add(fig,'manifest byte count',m['figure_bytes'],(self.root/'docs/figures'/fig).stat().st_size,[path+'#/figure_bytes'],'committed PNG bytes')
            if 'generator_sha256' in m:self.add(fig,'manifest generator digest',m['generator_sha256'],digest(self.root/m['generator_script']),[path+'#/generator_sha256'],'SHA256 pinned generator')
            for key in ['source_artifacts','source_receipts']:
                if isinstance(m.get(key),list):
                    for j,src in enumerate(m[key]):
                        self.add(fig,f'manifest {key} {j}',src['sha256'],digest(self.root/src['path']),[f'{path}#/{key}/{j}/sha256'],'SHA256 committed result file')
            if 'source_dossier' in m:self.add(fig,'manifest dossier digest',m['source_dossier_sha256'],digest(self.root/m['source_dossier']),[path+'#/source_dossier_sha256'],'SHA256 committed dossier')
            if isinstance(m.get('source_receipts'),dict):
                s=m['source_receipts']
                for key in ['wave1_gci_rsm_dossier','wave2_gci_rsm_f2_dossier']:
                    self.add(fig,'manifest '+key,s[key+'_sha256'],digest(self.root/s[key]),[path+'#/source_receipts/'+key+'_sha256'],'SHA256 committed result file')
                for k,f,p in [('wave1_g1_integrals_sha256',G1,'/grids/1/hydraulic_integrals_sha256'),('wave2_tpms_g1_integrals_sha256',G2,'/'+TP+'/grids/1/hydraulic_integrals_sha256'),('baseline_ct35_integrals_sha256','docs/results/exotic-vs-baseline-cfd-benchmark-v1.json','/baseline_ct35/hydraulic_integrals_sha256')]:
                    if k=='baseline_ct35_integrals_sha256':continue # no field arrays or source paths are bound by this hash
                    if k in s:self.add(fig,'manifest '+k,s[k],self.get(f,p),[path+'#/source_receipts/'+k,f+'#'+p],'same recorded integral digest; does not bind raw field arrays')
                if 'field_array_bundle' in s:
                    self.add(fig,'manifest field bundle path',s['field_array_bundle'],FIELD_ARRAYS,[path+'#/source_receipts/field_array_bundle'],'reviewed committed field-array subject')
                    self.add(fig,'manifest field bundle digest',s['field_array_bundle_sha256'],digest(self.root/s['field_array_bundle']),[path+'#/source_receipts/field_array_bundle_sha256'],'SHA256 entire committed bundle including null masks and raw/mesh inventory')
                    for key,case,section in [('wave1_grid','wave1','meridional'),('wave2_grid','wave2_tpms','meridional'),('wave1_profiles','wave1','profiles'),('wave2_profiles','wave2_tpms','profiles'),('baseline_profiles','baseline','profiles')]:
                        pointer=f'/cases/{case}/{section}'
                        self.add(fig,'manifest field-series pointer '+key,m['field_series_json_pointers'][key],pointer,[path+'#/field_series_json_pointers/'+key,FIELD_ARRAYS+'#'+pointer])
                    for case in ['wave1','wave2_tpms','baseline']:
                        pointer=f'/cases/{case}/coordinate_review'
                        self.add(fig,'manifest coordinate caveat '+case,m['coordinate_reviews'][case],self.get(FIELD_ARRAYS,pointer),[path+'#/coordinate_reviews/'+case,FIELD_ARRAYS+'#'+pointer])
                    pointer='/cases/baseline/mesh_count_matches_archived_dossier'
                    self.add(fig,'manifest archived baseline identity caveat',m['baseline_field_mesh_matches_archived_dossier'],self.get(FIELD_ARRAYS,pointer),[path+'#/baseline_field_mesh_matches_archived_dossier',FIELD_ARRAYS+'#'+pointer])

    def run(self):
        expected_counts={'wave4-readme-hero-kpi-and-elite.png':51,'wave4-evolution-ablation-and-lineage-proof.png':41,
                         'wave3-readme-hero-kpi-and-elite.png':51,'wave3-evolution-ablation-and-lineage-proof.png':41,
                         'wave4-page1-why-it-matters.png':27,'wave4-gci-same-pipeline-control.png':7,
                         'wave2-multiseed-ablation-and-ci.png':16,'elite-gci-and-rsm-verification.png':14,
                         'wave2-vs-wave1-openfoam14-cfd-comparison.png':18}
        self.add('ALL','expected figure set',sorted(self.capture['figures']),sorted(expected_counts),[],'pinned audit coverage contract')
        for fig,n in expected_counts.items():
            self.add(fig,'direct generator plot-call count',len(self.capture['figures'][fig]['calls']),n,[],'pinned audit coverage contract; missing/extra calls reject')
        self.proof('wave4');self.proof('wave3');self.page1();self.wave4_gci();self.plate3();self.wave2();self.plate10();self.manifests()
        # Plot-only constants are declared separately from measurements. The
        # sandbox oil ceiling and numerical policies have committed contracts.
        policy='requirements/RS-joil-research-v1.json'
        ceiling=self.get(policy,'/quality/research_limit_mg_l')
        policies={('wave4-readme-hero-kpi-and-elite.png',174):(policy+'#/quality/research_limit_mg_l',ceiling),
                  ('wave3-readme-hero-kpi-and-elite.png',174):(policy+'#/quality/research_limit_mg_l',ceiling),
                  ('wave2-multiseed-ablation-and-ci.png',350):(policy+'#/quality/research_limit_mg_l',ceiling),
                  ('wave2-multiseed-ablation-and-ci.png',396):(policy+'#/quality/research_limit_mg_l',ceiling),
                  ('elite-gci-and-rsm-verification.png',577):(policy+'#/reference/reject_fraction',self.get(policy,'/reference/reject_fraction')*100)}
        for fig,f in self.capture['figures'].items():
            for i,c in enumerate(f['calls']):
                if (fig,i) in self.covered:continue
                if (fig,c['line']) in policies:
                    ref,value=policies[(fig,c['line'])];self.check(fig,i,0,value,[ref],'sandbox authored policy; not admitted physical limit')
                else:self.add(fig,f'UNCOVERED call{i} line{c["line"]}',False,True,[],'must add explicit source binding')
        return self.rows


def audit(root, captured, line_map=None):
    worker=Audit(root,captured,line_map)
    rows=worker.run()
    return dict(schema_version=1,kind='independent_plot_call_to_committed_json_trace_not_physical_validation',
                numeric_tolerance=dict(relative=1e-12,absolute=1e-12),
                summary={status:sum(r['status']==status for r in rows) for status in ['PASS','FAIL','UNTRACED']},
                rows=rows)


def self_test(root, captured, line_map=None):
    """Meaningful negatives: wrong seed, wrong arm band, GCI units, dropped call."""
    results={}
    assert audit(root,captured,line_map)['summary']['FAIL']==0
    for name,fig,line,part in [
        ('wrong_seed_point','wave4-readme-hero-kpi-and-elite.png',146,1),
        ('wrong_arm_ci','wave4-page1-why-it-matters.png',85,1),
        ('wrong_gci_units','wave4-gci-same-pipeline-control.png',89,1)]:
        changed=copy.deepcopy(captured)
        call=next(c for c in changed['figures'][fig]['calls'] if dispatch_line(c,line_map or {})==line)
        call['args'][part][0]=call['args'][part][0]*1000 if name=='wrong_gci_units' else call['args'][part][0]+0.1
        results[name]=audit(root,changed,line_map)['summary']['FAIL']>0
    changed=copy.deepcopy(captured)
    changed['figures']['wave4-page1-why-it-matters.png']['calls'].pop(0)
    results['dropped_plot_call']=audit(root,changed,line_map)['summary']['FAIL']>0
    changed=copy.deepcopy(captured)
    changed['figures']['wave4-page1-why-it-matters.png']['committed_sha256']='0'*64
    results['wrong_capture_png_identity']=audit(root,changed,line_map)['summary']['FAIL']>0
    fig='wave2-vs-wave1-openfoam14-cfd-comparison.png'
    manifest=json.loads((Path(root)/('docs/figures/'+fig+'.manifest.json')).read_text())
    if manifest['source_receipts'].get('field_array_bundle') == FIELD_ARRAYS:
        for name in ['changed_field_value','swapped_field_subjects','field_null_to_zero']:
            changed=copy.deepcopy(captured)
            calls=changed['figures'][fig]['calls']
            call=next(c for c in calls if dispatch_line(c,line_map or {})==191)
            if name=='swapped_field_subjects':
                other=next(c for c in calls if dispatch_line(c,line_map or {})==211)
                call['args'],other['args']=other['args'],call['args']
            else:
                missing=name=='field_null_to_zero'
                for row in call['args'][2]:
                    index=next((j for j,v in enumerate(row) if (v is None)==missing),None)
                    if index is not None:
                        row[index]=0. if missing else row[index]+1.
                        break
                else:raise AssertionError('negative check fixture lacks required field bin')
            results[name]=audit(root,changed,line_map)['summary']['FAIL']>0
    assert all(results.values()),results
    return results


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--input-root',type=Path,required=True)
    ap.add_argument('--capture',type=Path,required=True)
    ap.add_argument('--output',type=Path,required=True)
    ap.add_argument('--self-test',action='store_true')
    ap.add_argument('--plot-line-map',type=Path,help='reviewed JSON map: source path -> new line -> original audit line')
    args=ap.parse_args()
    captured=json.loads(args.capture.read_text())
    line_map=json.loads(args.plot_line_map.read_text()) if args.plot_line_map else None
    result=audit(args.input_root,captured,line_map)
    if line_map is not None:result['reviewed_plot_line_map']=line_map
    if args.self_test:result['negative_checks']=self_test(args.input_root,captured,line_map)
    args.output.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print(json.dumps(result['summary']))
    for row in result['rows']:
        if row['status']=='FAIL':print(json.dumps(row))
    raise SystemExit(bool(result['summary']['FAIL']))
