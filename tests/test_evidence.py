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


if __name__=='__main__':unittest.main()


