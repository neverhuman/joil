import json
import math
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from analysis.evidence import manifest, verify, research_fitness, save_new
from analysis.inventory import bounds, concentration_bounds
from analysis.units import pressure_scale


class EvidenceTests(unittest.TestCase):
    def test_changed_evidence_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'p';p.write_bytes(b'pressure')
            m=manifest(d,['p']);verify(d,m)
            p.write_bytes(b'Pressure')
            with self.assertRaises(ValueError):verify(d,m)
            with self.assertRaises(ValueError):manifest(d,['../outside'])

    def test_receipts_cannot_be_overwritten(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'receipt.json';save_new(p,{'a':1})
            with self.assertRaises(FileExistsError):save_new(p,{'a':2})

    def test_pressure_dimensions(self):
        self.assertEqual(pressure_scale('dimensions [0 2 -2 0 0 0 0];',998),998)
        self.assertEqual(pressure_scale('dimensions [1 -1 -2 0 0 0 0];',998),1)
        with self.assertRaises(ValueError):pressure_scale('dimensions [0 1 -1 0 0 0 0];',998)
        with self.assertRaises(ValueError):pressure_scale('p 100;',998)

    def test_tracer_is_not_oil_separation(self):
        self.assertEqual(concentration_bounds(500,[.02,.02],.02),[500,500])

    def test_inventory_wall_and_remaining_are_not_known_capture(self):
        b=bounds(1,.8,.1,.05,.05)
        self.assertAlmostEqual(b['capture_bounds'][0],.8)
        self.assertAlmostEqual(b['capture_bounds'][1],.9)
        with self.assertRaises(ValueError):bounds(1,.9,.2,0,0)

    def test_threshold_and_unknown_fitness(self):
        kwargs=dict(flow_m3_h=4,reject=.02,hydraulic_loss_j_m3=10000,
                    eligibility_reasons=['no validation'])
        unknown=research_fitness(cout_bounds=None,**kwargs)
        self.assertIsNone(unknown['minimize_screen_vector'])
        self.assertIsNone(unknown['qualified_multiobjective_vector'])
        self.assertEqual(unknown['sampled_quality'],'unknown')
        fail=research_fitness(cout_bounds=concentration_bounds(500,[.94,.94],.02),**kwargs)
        self.assertEqual(fail['sampled_quality'],'fail_at_sampled_point')
        self.assertAlmostEqual(fail['minimize_screen_vector'][0],30.6122448979592)
        self.assertIsNone(fail['qualified_capacity_gain'])
        with self.assertRaises(ValueError):research_fitness(cout_bounds=[math.nan,1],**kwargs)

    def test_wave2_preregistration_contract(self):
        import hashlib
        root=Path(__file__).resolve().parents[1]
        prereg=json.loads((root/'experiments/wave2-preregistration-v1.json').read_text())
        base_sha=hashlib.sha256((root/'baselines/ct35/genome.json').read_bytes()).hexdigest()
        self.assertEqual(prereg['baseline_file_sha256'],base_sha)
        self.assertEqual(len(prereg['topology_families_11']),11)
        self.assertEqual(len(prereg['equal_budget_comparators']),4)
        self.assertEqual(prereg['multiseed_statistical_protocol']['n_seeds'],10)
        self.assertEqual(len(prereg['held_out_operating_duties']),4)
        self.assertEqual(prereg['mandatory_elite_cfd_gates']['rsm_closures_required'],['LRR','SSG'])

    def test_mandatory_elite_gci_and_rsm_promotion_gate(self):
        import copy
        from tools.run_elite_3grid_gci_and_rsm import evaluate_elite_gci_rsm_gate
        root=Path(__file__).resolve().parents[1]
        dossier=json.loads((root/'docs/results/elite-3grid-gci-and-rsm-verification-v1.json').read_text())
        prereg=json.loads((root/'experiments/wave2-preregistration-v1.json').read_text())
        passed=evaluate_elite_gci_rsm_gate(dossier,prereg)
        self.assertTrue(passed['gate_passed'],passed['failures'])
        self.assertGreaterEqual(passed['checks']['saving_to_3grid_span_ratio_dp_u'],3.0)
        self.assertLessEqual(passed['checks']['inter_rsm_dp_u_delta_pct'],5.0)
        bad=copy.deepcopy(dossier)
        bad['rsm_turbulence_closure_crosscheck'][0]['reynolds_stress_tensor_diagnostics']['positive_definite_normal_stress_fraction']=0.98
        bad['roache_gci_analysis']['signal_to_uncertainty_margin']['saving_to_3grid_span_ratio']=2.1
        failed=evaluate_elite_gci_rsm_gate(bad,prereg)
        self.assertFalse(failed['gate_passed'])
        self.assertGreaterEqual(len(failed['failures']),2)

        w2_bundle=json.loads((root/'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json').read_text())
        self.assertIsNone(w2_bundle['qualified_multiobjective_vector'])
        for key in ('wave2_tpms_gyroid_elite_5run_dossier','wave2_octa_branch_8jet_elite_5run_dossier'):
            w2_res=evaluate_elite_gci_rsm_gate(w2_bundle[key],prereg)
            self.assertTrue(w2_res['gate_passed'],w2_res['failures'])
            self.assertGreaterEqual(w2_res['checks']['saving_to_3grid_span_ratio_dp_u'],3.0)
            self.assertLessEqual(w2_res['checks']['inter_rsm_dp_u_delta_pct'],5.0)
        f2_chk=w2_bundle['f2_oil_transient_droplet_closure_checkpoint_and_resumable_plan']
        self.assertFalse(f2_chk['qualified_for_f2_oil_grade'])
        self.assertTrue(f2_chk['strict_unextended_overflow_backflow_gate_rejected'])
        self.assertTrue(f2_chk['stage0_inventory_closure_passed'])
        self.assertEqual(f2_chk['stage0_total_discrepancy_count'],0)
        self.assertEqual(len(f2_chk['resumable_8stage_continuation_plan']['stages']),8)

    def test_wave2_receipt_and_whitepaper_hash_consistency(self):
        import hashlib
        root=Path(__file__).resolve().parents[1]
        receipt=json.loads((root/'docs/receipts/wave2-genome-and-10seed-ablation-20261015.json').read_text())
        whitepaper=(root/'docs/WHITEPAPER.md').read_text()
        for rel_path,expected_sha in receipt['artifacts_sha256'].items():
            full_path=root/rel_path
            self.assertTrue(full_path.is_file(),f'missing artifact {rel_path}')
            actual_sha=hashlib.sha256(full_path.read_bytes()).hexdigest()
            self.assertEqual(actual_sha,expected_sha,f'hash mismatch for {rel_path}')
            self.assertIn(expected_sha,whitepaper,f'{rel_path} sha256 missing from WHITEPAPER.md')
            manifest_path=Path(f'{full_path}.manifest.json')
            if manifest_path.is_file():
                m=json.loads(manifest_path.read_text())
                self.assertEqual(m['figure_sha256'],actual_sha)

    def test_claims_to_receipts_audit_and_negative_mutation_rejection(self):
        from tools.claims_audit import (
            build_allowed_numeric_registry,
            build_receipt_bundle,
            run_claims_audit,
            scan_markdown_numeric_tokens,
            verify_paths_and_sha256_digests,
        )
        root=Path(__file__).resolve().parents[1]
        report=run_claims_audit(root)
        self.assertTrue(report['passed'],report['errors'])
        self.assertEqual(report['error_count'],0)
        self.assertGreaterEqual(report['structured_claims_verified'],120)
        self.assertGreaterEqual(report['numeric_tokens_scanned'],800)

        bundle=build_receipt_bundle(root)
        allowed=build_allowed_numeric_registry(bundle)
        bad_num_errs,_=scan_markdown_numeric_tokens(
            {'docs/WHITEPAPER.md':'Synthetic untraceable claim: 987654.321 mg/L carry-under.'},
            allowed,
        )
        self.assertEqual(len(bad_num_errs),1)
        self.assertIn('987654.321',bad_num_errs[0])

        bad_path_errs=verify_paths_and_sha256_digests(
            {'docs/WHITEPAPER.md':'Reference `docs/results/fabricated-receipt.json` (`SHA-256: ' + ('f'*64) + '`).'},
            bundle,
            root,
        )
        self.assertGreaterEqual(len(bad_path_errs),2)

    def test_wave3_evolution_proof_kpi_and_ablation_audit(self):
        from analysis.kpi_fitness import verify_wave3_evolution_proof_kpis
        root=Path(__file__).resolve().parents[1]
        rep=verify_wave3_evolution_proof_kpis(root/'docs/results/wave3-evolution-proof-v1.json')
        self.assertTrue(rep['passed'])
        self.assertEqual(rep['n_seeds'],10)
        self.assertEqual(rep['verified_seed_runs'],50)
        self.assertAlmostEqual(rep['baseline_b0_ct35_kpi_score'],26.486473,places=5)
        self.assertGreater(rep['promoted_wave3_top_elite_kpi_score'],47.0)
        self.assertIsNone(rep['qualified_multiobjective_vector'])
        self.assertIsNone(rep['qualified_capacity_gain'])

    def test_tr07_int05_python_gci_order_and_wave2_v2_reports(self):
        from tools.generate_gci_rsm_report_and_figures import (
            compute_roache_3grid_gci_v2,
            compute_roache_4grid_gci_v2,
        )
        from tools.run_elite_3grid_gci_and_rsm import evaluate_elite_gci_rsm_gate

        for p_true in (1.5, 2.0, 3.2, 5.5):
            h = [4.0, 2.0, 1.0, 0.5]
            vals = [100.0 + 3.0 * (hi**p_true) for hi in h]
            g3 = compute_roache_3grid_gci_v2(vals[0], vals[1], vals[2], h[0], h[1], h[2])
            self.assertEqual(g3['gci_validity_state'], 'Monotone')
            self.assertFalse(g3['asymptotic_range_certified'])
            self.assertIsNone(g3['gci21_standard_fs125_pct'])
            self.assertAlmostEqual(g3['observed_order_p'], p_true, places=10)

            g4 = compute_roache_4grid_gci_v2(vals, h, relative_order_tolerance=0.05)
            self.assertEqual(g4['gci_validity_state'], 'Asymptotic')
            self.assertTrue(g4['asymptotic_range_certified'])
            self.assertAlmostEqual(g4['observed_order_p'], p_true, places=10)
            self.assertIsNotNone(g4['gci21_standard_fs125_pct'])

        osc = compute_roache_3grid_gci_v2(17419.12, 19576.17, 18723.94, 3.55, 2.74, 2.25)
        self.assertEqual(osc['gci_validity_state'], 'Oscillatory')
        self.assertFalse(osc['asymptotic_range_certified'])
        self.assertIsNone(osc['observed_order_p'])
        self.assertIsNone(osc['gci21_standard_fs125_pct'])

        div = compute_roache_3grid_gci_v2(10.0, 11.0, 13.5, 4.0, 2.0, 1.0)
        self.assertEqual(div['gci_validity_state'], 'Divergent')
        self.assertFalse(div['asymptotic_range_certified'])
        self.assertLess(div['observed_order_p'], 0.0)
        self.assertIsNone(div['gci21_standard_fs125_pct'])

        ind = compute_roache_3grid_gci_v2(10.0, 10.0, 10.0, 4.0, 2.0, 1.0)
        self.assertEqual(ind['gci_validity_state'], 'Indeterminate')
        self.assertIsNone(ind['observed_order_p'])
        self.assertIsNone(ind['gci21_standard_fs125_pct'])

        root = Path(__file__).resolve().parents[1]
        prereg = json.loads((root / 'experiments/wave2-preregistration-v1.json').read_text())
        v2_w1 = json.loads((root / 'docs/results/elite-3grid-gci-and-rsm-verification-v2.json').read_text())
        self.assertEqual(v2_w1['schema_version'], 2)
        self.assertEqual(v2_w1['roache_gci_analysis']['dp_u_pa']['gci_validity_state'], 'Oscillatory')
        self.assertIsNone(v2_w1['roache_gci_analysis']['dp_u_pa']['observed_order_p'])
        self.assertIsNone(v2_w1['roache_gci_analysis']['dp_u_pa']['gci21_standard_fs125_pct'])
        self.assertTrue(evaluate_elite_gci_rsm_gate(v2_w1, prereg)['gate_passed'])

        v2_w2 = json.loads((root / 'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json').read_text())
        self.assertEqual(v2_w2['schema_version'], 2)
        for key in ('wave2_tpms_gyroid_elite_5run_dossier', 'wave2_octa_branch_8jet_elite_5run_dossier'):
            sub = v2_w2[key]
            self.assertEqual(sub['roache_gci_analysis']['dp_u_pa']['gci_validity_state'], 'Oscillatory')
            self.assertIsNone(sub['roache_gci_analysis']['dp_u_pa']['observed_order_p'])
            self.assertIsNone(sub['roache_gci_analysis']['dp_u_pa']['gci21_standard_fs125_pct'])
            self.assertTrue(evaluate_elite_gci_rsm_gate(sub, prereg)['gate_passed'])


if __name__=='__main__':unittest.main()



