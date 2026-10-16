import json
import math
import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from analysis.evidence import manifest, verify, research_fitness, save_new
from analysis.inventory import bounds, concentration_bounds
from analysis.units import pressure_scale


class EvidenceTests(unittest.TestCase):
    def _disposition_module(self):
        import importlib.util
        path = Path(__file__).resolve().parents[1]/'crates/joil-evidence/receipts/ev12_disposition_audit.py'
        spec = importlib.util.spec_from_file_location('ev12_disposition', path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def _disposition_fixture(self, root, module):
        import hashlib
        from analysis.case_identity import finalize_case_identity
        inputs, runs, diagnostic = root/'inputs', root/'runs', root/'g1-diagnostic'
        complete = self._synthetic_wave4_case(inputs, module.SUBJECT_CASE)
        incomplete = self._synthetic_wave4_case(inputs, 'wave4-b3-s20261103-g0-coarse')
        shutil.rmtree(incomplete/'constant/polyMesh')
        active = runs/complete.name
        shutil.copytree(complete, active)
        finalize_case_identity(active)
        quarantine = runs/'unfinalized-pruned-wave4'/incomplete.name
        shutil.copytree(incomplete, quarantine)
        shutil.copytree(complete, diagnostic)
        (diagnostic/'constant/polyMesh/boundary').write_text('wall\n{\n nFaces 10;\n}\n')
        (diagnostic/'water-1-mpirun.log').write_bytes(b'')
        (diagnostic/'water-1-mpirun.log.receipt.json').write_text(json.dumps({
            'exit_code': 1, 'log_sha256': hashlib.sha256(b'').hexdigest(), 'log_bytes': 0}))
        (diagnostic/'water-1-mpirun.orig.log').write_text('FOAM FATAL ERROR\nUnknown patch name: inlet1\nMPI_ABORT\n')
        (diagnostic/'mesh-2-checkMesh.log').write_text('synthetic diagnostic checkMesh log')
        finalize_case_identity(diagnostic)
        (diagnostic/'constant/polyMesh/cellProc').write_text('synthetic changed decomposition')
        return inputs, runs, diagnostic, active, quarantine

    def test_ev12_disposition_accounts_complete_pruned_and_failed_cases_without_promotion(self):
        from analysis.case_identity import _inventory, audit_wave4_manifests
        module = self._disposition_module()
        private = Path(__file__).resolve().parents[1]/'.private'
        private.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=private) as directory:
            inputs, runs, diagnostic, active, quarantine = self._disposition_fixture(Path(directory), module)
            expected = {active.name, quarantine.name}
            before = _inventory(Path(directory), ['.'])
            report = module.audit(runs, inputs, diagnostic, expected)
            self.assertTrue(report['passed'], report)
            self.assertEqual(report['case_count'], 3)
            self.assertEqual(report['counts'], {'SCOPED_IDENTITY_VERIFIED': 1,
                                              'EXCLUDED_MISSING_EXECUTED_MESH': 1,
                                              'EXCLUDED_FAILED_G1_DIAGNOSTIC': 1})
            rows = {r['case']: r for r in report['cases']}
            self.assertIsNone(rows[quarantine.name]['mesh_hash'])
            self.assertIsNone(rows[quarantine.name]['case_hash'])
            self.assertEqual(rows[module.G1_CASE]['identity_sidecar_state'], 'STALE')
            self.assertFalse(rows[module.G1_CASE]['valid_finalized_identity_for_retained_state'])
            self.assertIsNone(rows[module.G1_CASE]['inlet_patches']['inlet1']['n_faces'])
            self.assertFalse(report['all_cases_have_verified_identity'])
            self.assertEqual(report['claim_eligible_cases'], 0)
            self.assertFalse(audit_wave4_manifests(runs/'unfinalized-pruned-wave4')['passed'])
            self.assertEqual(_inventory(Path(directory), ['.']), before, 'audit must be read-only')

    def test_ev12_disposition_rejects_inventory_loss_drift_and_broken_active_identity(self):
        from analysis.case_identity import finalize_case_identity
        module = self._disposition_module()
        private = Path(__file__).resolve().parents[1]/'.private'
        private.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=private) as directory:
            inputs, runs, diagnostic, active, quarantine = self._disposition_fixture(Path(directory), module)
            expected = {active.name, quarantine.name}
            self.assertFalse(module.audit(runs, inputs, diagnostic, expected | {'wave4-missing'})['passed'])
            self.assertFalse(module.audit(runs, inputs, diagnostic, set())['passed'])
            for case in [active, quarantine]:
                path = case/'0/U'
                original = path.read_bytes()
                path.write_bytes(original+b' changed')
                report = module.audit(runs, inputs, diagnostic, expected)
                self.assertFalse(report['passed'])
                self.assertIn(case.name, report['unresolved_cases'])
                path.write_bytes(original)
            extra = self._synthetic_wave4_case(runs, 'wave4-extra-unregistered')
            finalize_case_identity(extra)
            report = module.audit(runs, inputs, diagnostic, expected)
            self.assertFalse(report['passed'])
            self.assertIn(extra.name, report['unresolved_cases'])

    def test_ev12_g1_disposition_requires_bound_fatal_evidence_and_missing_inlet(self):
        module = self._disposition_module()
        private = Path(__file__).resolve().parents[1]/'.private'
        private.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=private) as directory:
            inputs, runs, diagnostic, active, quarantine = self._disposition_fixture(Path(directory), module)
            expected = {active.name, quarantine.name}
            for rel, replacement in [
                ('water-1-mpirun.log.receipt.json', b'{"exit_code":0}'),
                ('water-1-mpirun.orig.log', b'End\n'),
                ('water-1-mpirun.log', b'changed wrapper log'),
                ('constant/polyMesh/boundary', b'inlet1\n{\n nFaces 5;\n}\n'),
                ('design-program.json', b'{"forged":true}'),
            ]:
                path = diagnostic/rel
                original = path.read_bytes()
                path.write_bytes(replacement)
                with self.subTest(missing_or_forged=rel):
                    report = module.audit(runs, inputs, diagnostic, expected)
                    self.assertFalse(report['passed'])
                    self.assertIn(module.G1_CASE, report['unresolved_cases'])
                path.write_bytes(original)

    def test_ev12_real_disposition_inventory_when_explicitly_supplied(self):
        names = ['JOIL_WAVE4_RUNS_ROOT', 'JOIL_WAVE4_INPUTS_ROOT', 'JOIL_WAVE4_DIAGNOSTIC_ROOT']
        if not all(os.environ.get(name) for name in names):
            self.skipTest('set all three Wave4 roots for read-only real disposition audit')
        module = self._disposition_module()
        report = module.audit(*(os.environ[name] for name in names))
        self.assertTrue(report['passed'], (report['errors'], report['unresolved_cases']))
        self.assertEqual(report['case_count'], 12)
        self.assertEqual(report['counts'], {'SCOPED_IDENTITY_VERIFIED': 3,
                                          'EXCLUDED_MISSING_EXECUTED_MESH': 8,
                                          'EXCLUDED_FAILED_G1_DIAGNOSTIC': 1})

    def test_ev12_every_run_audit_includes_missing_bad_and_quarantined_cases(self):
        import importlib.util
        from analysis.case_identity import finalize_case_identity
        root = Path(__file__).resolve().parents[1]
        path = root/'crates/joil-evidence/receipts/ev12_every_run_audit.py'
        spec = importlib.util.spec_from_file_location('ev12_every_run', path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        (root/'.private').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root/'.private') as directory:
            runs = Path(directory)
            self.assertFalse(module.audit(runs)['all_listed_passed'])
            good = self._synthetic_wave4_case(runs, 'wave4-good-g0-coarse')
            bad = self._synthetic_wave4_case(runs, 'wave4-bad-g1-medium')
            preparing = self._synthetic_wave4_case(runs, 'wave4-preparing-g2-fine')
            quarantined = self._synthetic_wave4_case(runs/'unfinalized-pruned-wave4', 'wave4-old-g0-coarse')
            for case in [good, bad, quarantined]:
                finalize_case_identity(case)
                shutil.copyfile(case/'case-identity.json', case/'case.json')
            damaged = json.loads((bad/'case.json').read_text())
            damaged['mesh_hash'] = 'f'*64
            (bad/'case.json').write_text(json.dumps(damaged))
            (preparing/'constant/polyMesh/points').unlink()
            (quarantined/'constant/polyMesh/points').unlink()
            report = module.audit(runs)
            self.assertEqual(report['case_count'], 4)
            rows = {r['case']: r for r in report['cases']}
            self.assertEqual(rows[good.name]['status'], 'PASS')
            self.assertEqual(rows[bad.name]['status'], 'FAIL')
            self.assertEqual(rows[bad.name]['manifest_files']['case-identity.json']['status'], 'PASS')
            self.assertEqual(rows[preparing.name]['status'], 'FAIL')
            self.assertEqual(rows[quarantined.name]['scope'], 'quarantined')
            self.assertEqual(rows[quarantined.name]['status'], 'FAIL')
            (good/'case.json').unlink()
            self.assertEqual(module.audit_case('active', good, 'case.json')['status'], 'FAIL')
            self.assertEqual(module.audit_case('active', good, 'case-identity.json')['status'], 'PASS')

    def test_wave4_raw_independent_recompute_contract(self):
        import importlib.util
        path = Path(__file__).resolve().parents[1]/'crates/joil-evidence/receipts/wave4_raw_proof_tests.py'
        spec = importlib.util.spec_from_file_location('ev_raw_proof_tests', path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        suite = unittest.defaultTestLoader.loadTestsFromModule(module)
        result = unittest.TestResult()
        suite.run(result)
        self.assertEqual(result.testsRun, 10)
        self.assertTrue(result.wasSuccessful(), result.errors+result.failures)

    def _synthetic_wave4_case(self, runs, name):
        import hashlib
        from analysis.evidence import evidence_reference

        case = runs/name
        program = {'synthetic': True, 'z': 2, 'a': 0}
        genome_hash = hashlib.sha256(json.dumps(program, separators=(',', ':')).encode()).hexdigest()
        ir_program_hash = hashlib.sha256(json.dumps(program, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        files = {
            'design-program.json': json.dumps(program).encode(),
            'design-ir.json': json.dumps({'program_sha256': ir_program_hash,
                                          'ir_sha256': 'b'*64}).encode(),
            'constant/triSurface/ct35.stl': b'solid synthetic\nendsolid\n',
            '0/U': b'synthetic initial velocity',
            '0/p': b'synthetic initial pressure',
            'system/controlDict': b'synthetic solver config',
            'system/fvSchemes': b'synthetic schemes',
            'system/fvSolution': b'synthetic solution',
        }
        for mesh_file in ['points', 'faces', 'owner', 'neighbour', 'boundary']:
            files['constant/polyMesh/'+mesh_file] = ('synthetic '+mesh_file).encode()
        for name, content in files.items():
            path = case/name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        surface = 'constant/triSurface/ct35.stl'
        bundle = {key: evidence_reference(path, 'synthetic-v1', files[path])
                  for key, path in [('genome', 'design-program.json'),
                                    ('ir', 'design-ir.json'), ('surface_mesh', surface)]}
        for key in ['compiler', 'geometry_kernel', 'compiler_config']:
            bundle[key] = evidence_reference(key, 'synthetic-v1', b'synthetic fixture')
        (case/'input-manifest.json').write_text(json.dumps({
            'design_sha256': genome_hash, 'design_ir_sha256': 'b'*64,
            'surface_stl_relpath': surface,
            'surface_stl_sha256': hashlib.sha256(files[surface]).hexdigest(),
            'subject_bundle': bundle,
        }))
        return case

    def test_wave4_case_identity_binds_actual_mesh_and_solver_inputs(self):
        from analysis.case_identity import finalize_case_identity, verify_case_identity
        root = Path(__file__).resolve().parents[1]
        (root/'.private').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root/'.private') as directory:
            case = self._synthetic_wave4_case(Path(directory), 'wave4-synthetic-g0-coarse')
            identity = finalize_case_identity(case)
            self.assertNotEqual(identity['canonical_genome_hash'], identity['canonical_ir_source_genome_hash'])
            self.assertEqual(verify_case_identity(case)['state'], 'exact_artifact_bytes_verified')
            for key in ['genome_hash', 'ir_hash', 'geometry_hash', 'mesh_hash', 'case_hash']:
                self.assertRegex(identity[key], r'^[0-9a-f]{64}$')
            self.assertEqual(finalize_case_identity(case), identity)
            for rel in ['constant/polyMesh/points', 'system/fvSchemes', '0/U']:
                path = case/rel
                original = path.read_bytes()
                path.write_bytes(original+b' changed')
                with self.subTest(changed=rel):
                    with self.assertRaises(ValueError): verify_case_identity(case)
                    with self.assertRaises(ValueError): finalize_case_identity(case)
                path.write_bytes(original)
            (case/'constant/polyMesh/points').unlink()
            with self.assertRaises(ValueError): verify_case_identity(case)

    def test_wave4_manifest_audit_reads_every_case_and_rejects_missing_stage(self):
        from analysis.case_identity import audit_wave4_manifests, finalize_case_identity
        root = Path(__file__).resolve().parents[1]
        (root/'.private').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root/'.private') as directory:
            runs = Path(directory)
            self.assertFalse(audit_wave4_manifests(runs)['passed'])
            cases = [self._synthetic_wave4_case(runs, f'wave4-synthetic-g{g}-test') for g in range(3)]
            for case in cases: finalize_case_identity(case)
            audit = audit_wave4_manifests(runs)
            self.assertTrue(audit['passed'], audit['errors'])
            self.assertEqual(audit['manifest_count'], 3)
            (cases[-1]/'case-identity.json').unlink()
            audit = audit_wave4_manifests(runs)
            self.assertFalse(audit['passed'])
            self.assertEqual(len(audit['errors']), 1)
            self.assertEqual(audit['errors'][0]['case'], cases[-1].name)

    def test_wave4_case_identity_rejects_cross_case_and_path_substitution(self):
        from analysis.case_identity import finalize_case_identity, verify_case_identity
        root = Path(__file__).resolve().parents[1]
        (root/'.private').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root/'.private') as directory:
            runs = Path(directory)
            case = self._synthetic_wave4_case(runs, 'wave4-synthetic-g1-medium')
            finalize_case_identity(case)
            (case/'design-program.json').write_bytes(b'changed source')
            with self.assertRaises(ValueError): verify_case_identity(case)
            prepared = json.loads((case/'input-manifest.json').read_text())
            prepared['surface_stl_relpath'] = '../outside.stl'
            (case/'input-manifest.json').write_text(json.dumps(prepared))
            with self.assertRaises(ValueError): finalize_case_identity(case)

    def test_every_local_wave4_run_manifest_has_complete_exact_identity(self):
        from analysis.case_identity import audit_wave4_manifests
        runs = Path(os.environ.get('JOIL_WAVE4_RUNS_ROOT', Path(__file__).resolve().parents[1]/'runs'))
        if not any(p.is_dir() for p in runs.glob('wave4-*')):
            self.skipTest('no local Wave-4 run artifacts; actual acceptance requires nonempty audit')
        report = audit_wave4_manifests(runs)
        self.assertTrue(report['passed'], report['errors'])
        self.assertGreater(report['manifest_count'], 0)

    def test_wave4_audit_continues_after_malformed_manifest_and_checks_all_cases(self):
        from analysis.case_identity import audit_wave4_manifests, finalize_case_identity
        root = Path(__file__).resolve().parents[1]
        (root/'.private').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root/'.private') as directory:
            runs = Path(directory)
            for name in ['wave4-a-g0-test', 'wave4-z-rsm-continuation']:
                finalize_case_identity(self._synthetic_wave4_case(runs, name))
            bad = self._synthetic_wave4_case(runs, 'wave4-m-g1-test')
            original = json.loads((bad/'input-manifest.json').read_text())
            malformed = [None, [], {'subject_bundle': None}]
            for key, value in [('surface_stl_relpath', None), ('surface_stl_relpath', []),
                               ('subject_bundle', []), ('subject_bundle', {'genome': None})]:
                malformed.append({**original, key: value})
            for key in ['genome', 'ir', 'surface_mesh', 'compiler', 'geometry_kernel', 'compiler_config']:
                for value in [None, {'id': '', 'revision': 'v1', 'sha256': 'a'*64},
                              {'id': key, 'revision': None, 'sha256': 'a'*64},
                              {'id': key, 'revision': 'v1', 'sha256': 'not a hash'}]:
                    malformed.append({**original, 'subject_bundle': {**original['subject_bundle'], key: value}})
            # A valid sidecar forces the audit to inspect the malformed prepared manifest.
            finalize_case_identity(bad)
            for value in malformed:
                with self.subTest(prepared=value):
                    (bad/'input-manifest.json').write_text(json.dumps(value))
                    report = audit_wave4_manifests(runs)
                    self.assertFalse(report['passed'])
                    self.assertEqual(report['manifest_count'], 3)
                    self.assertEqual([r['case'] for r in report['verified']],
                                     ['wave4-a-g0-test', 'wave4-z-rsm-continuation'])
                    self.assertEqual([r['case'] for r in report['errors']], [bad.name])
            (bad/'input-manifest.json').write_text(json.dumps(original))
            for value in ['null', '[]', '{broken json']:
                with self.subTest(sidecar=value):
                    (bad/'case-identity.json').write_text(value)
                    report = audit_wave4_manifests(runs)
                    self.assertFalse(report['passed'])
                    self.assertEqual(len(report['verified']), 2)
                    self.assertEqual([r['case'] for r in report['errors']], [bad.name])

    def test_wave4_identity_rejects_symlinks_and_pruned_mesh_even_with_sidecar(self):
        from analysis.case_identity import audit_wave4_manifests, finalize_case_identity, verify_case_identity
        root = Path(__file__).resolve().parents[1]
        (root/'.private').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root/'.private') as directory:
            runs = Path(directory)
            case = self._synthetic_wave4_case(runs, 'wave4-synthetic-g2-fine')
            finalize_case_identity(case)
            for rel in ['case-identity.json', 'constant/polyMesh/points', 'system/fvSchemes']:
                path = case/rel
                original = path.read_bytes()
                outside = runs/'outside-artifact'
                outside.write_bytes(original)
                path.unlink()
                path.symlink_to(outside)
                with self.subTest(symlink=rel):
                    with self.assertRaises(ValueError): verify_case_identity(case)
                    with self.assertRaises(ValueError): finalize_case_identity(case)
                path.unlink()
                path.write_bytes(original)
            shutil.rmtree(case/'constant/polyMesh')
            with self.assertRaisesRegex(ValueError, 'missing artifact directory'):
                finalize_case_identity(case)
            report = audit_wave4_manifests(runs)
            self.assertFalse(report['passed'])
            self.assertEqual(len(report['verified']), 0)
            self.assertEqual([r['case'] for r in report['errors']], [case.name])

    def test_wave4_input_capture_replays_retained_runs_and_rejects_pruned_cases(self):
        """Opt-in real-input regression: source runs remain read-only, including sidecars."""
        from analysis.case_identity import audit_wave4_manifests, build_case_identity, finalize_case_identity
        source = os.environ.get('JOIL_WAVE4_INPUTS_ROOT')
        if source is None:
            self.skipTest('set JOIL_WAVE4_INPUTS_ROOT for retained/pruned real-input regression')
        runs = Path(source)
        cases = sorted(p for p in runs.glob('wave4-*') if p.is_dir())
        required = {'wave4-ct35-control-g1-medium', 'wave4-b3-s20261105-g0-coarse',
                    'wave4-b3-s20261108-g0-coarse'}
        self.assertTrue(required <= {p.name for p in cases}, 'required retained regression cases missing')
        root = Path(__file__).resolve().parents[1]
        (root/'.private').mkdir(exist_ok=True)
        captured, missing_mesh = [], []
        with tempfile.TemporaryDirectory(dir=root/'.private') as directory:
            snapshots = Path(directory)
            for case in cases:
                with self.subTest(case=case.name):
                    if not (case/'constant/polyMesh').is_dir():
                        with self.assertRaisesRegex(ValueError, 'missing artifact directory: constant/polyMesh'):
                            build_case_identity(case)
                        missing_mesh.append(case.name)
                        continue
                    identity = build_case_identity(case)
                    snapshot = snapshots/case.name
                    files = {'input-manifest.json', *identity['source_artifacts'],
                             *identity['mesh_files'], *identity['solver_input_files']}
                    for rel in sorted(files):
                        target = snapshot/rel
                        target.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copyfile(case/rel, target)
                    # Finalization writes only the private copy, never the original run.
                    self.assertEqual(finalize_case_identity(snapshot), identity)
                    captured.append(case.name)
            self.assertTrue(required <= set(captured))
            self.assertTrue(missing_mesh, 'no missing-mesh negative cases were exercised')
            self.assertEqual(len(captured)+len(missing_mesh), len(cases))
            report = audit_wave4_manifests(snapshots)
            self.assertTrue(report['passed'], report['errors'])
            self.assertEqual(report['manifest_count'], len(captured))

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



