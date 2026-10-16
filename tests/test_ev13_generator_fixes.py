"""EV-13: receipt-dependent narratives and honest numerical-policy labels."""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools import generate_wave3_proof_figures as proof
from tools import generate_wave2_figures_and_animations as wave2
from tools import generate_gci_rsm_report_and_figures as gci
from tools import generate_wave2_vs_wave1_cfd_plate as plate10
import matplotlib.pyplot as plt
from matplotlib.figure import Figure


def load(name):
    return json.loads((ROOT / 'docs/results' / name).read_text())


def figure_text(fig):
    text = [t.get_text() for t in fig.texts]
    for ax in fig.axes:
        text.extend([ax.get_title(), ax.get_xlabel(), ax.get_ylabel()])
        text.extend(t.get_text() for t in ax.texts)
        legend = ax.get_legend()
        if legend:
            text.extend(t.get_text() for t in legend.texts)
    return '\n'.join(text)


class EV13GeneratorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.w3 = load('wave3-evolution-proof-v1.json')
        cls.w4 = load('wave4-evolution-proof-v1.json')
        cls.cfd = load('wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json')

    def tearDown(self):
        plt.close('all')
        plt.rcdefaults()

    def test_actual_opposite_orderings_and_tests(self):
        old, new = proof.comparison_labels(self.w3), proof.comparison_labels(self.w4)
        self.assertEqual((old['best_order'], old['best_p'], old['median_delta']),
                         ('B3 > B1', '0.01017', '+7.76'))
        self.assertEqual((new['best_order'], new['best_p'], new['median_delta']),
                         ('B1 > B3', '1', '+5.64'))

    def test_changed_source_reverses_narrative_and_missing_test_rejects(self):
        changed = deepcopy(self.w4)
        changed['final_summary_statistics']['B1_Random_Search']['best_kpi_score']['mean'] = 0
        self.assertEqual(proof.comparison_labels(changed)['best_order'], 'B3 > B1')
        changed['pairwise_tests'] = []
        with self.assertRaises(KeyError):
            proof.comparison_labels(changed)

    def test_rendered_labels_follow_each_wave_without_blanket_cells_or_permit(self):
        for wave, data, order, p, delta in [('Wave-3', self.w3, 'B3 > B1', '0.01017', '+7.76'),
                                            ('Wave-4', self.w4, 'B1 > B3', '1', '+5.64')]:
            with self.subTest(wave=wave), patch.object(Figure, 'savefig'), patch.object(plt, 'close'):
                proof.generate_readme_hero(data, self.cfd, 'source', 'cfd', wave_label=wave)
                text = figure_text(plt.gcf())
                self.assertIn(f'{order} on Best KPI (p_Holm={p})', text)
                if wave == 'Wave-3':
                    self.assertNotIn('Charged Proposals', text)
                self.assertIn('A1/A2/B3 shared Gen0; B2 conventionalized control; B1 random search', text)
                self.assertIn('Authored research-screen threshold (30.0 mg/L)', text)
                for wrong in ['Discharge Ceiling', '552k cells', '305k cells', 'Shared Unseeded Gen-0 Init']:
                    self.assertNotIn(wrong, text)
                proof.generate_plate11_proof(data, 'source', wave_label=wave)
                text = figure_text(plt.gcf())
                self.assertIn(f'{order} on Best KPI (p_Holm={p})', text)
                self.assertIn(f'({delta}, p_Holm=', text)
                if wave == 'Wave-3':
                    self.assertNotIn('Charged Proposals', text)
                self.assertIn('Authored', text)
                self.assertIn('Not Geometry Holdout' if wave == 'Wave-3' else 'Not Held-Out', text)
                self.assertNotIn('Held-Out 3D Geometry', text)

    def test_missing_requirement_cannot_silently_draw_a_limit(self):
        with patch.object(proof, 'REQUIREMENTS_JSON', ROOT / '.private/absent-ev13-requirement.json'):
            with self.assertRaises(FileNotFoundError):
                proof.generate_readme_hero(self.w4, self.cfd, 'source', 'cfd')

    def test_wave2_draws_executed_values_with_protocol_and_policy_qualifications(self):
        data = load('wave2-multiseed-ablation-and-holdout-v1.json')
        with patch.object(Figure, 'savefig'), patch.object(plt, 'close'), patch.object(wave2, 'write_manifest'):
            wave2.generate_ablation_plate(data)
            fig = plt.gcf()
            text = figure_text(fig)
            self.assertIn('Legacy Hypervolume Telemetry', text)
            self.assertIn('Executed Screen Duties (D1..D4; Protocol Deviation)', text)
            self.assertIn('Authored research-screen threshold (30.0 mg/L)', text)
            for wrong in ['2.0% Gate', 'Compliance Threshold', 'Pre-Registered Held-Out', 'Status: PASS']:
                self.assertNotIn(wrong, text)
            self.assertEqual([p.get_height() for p in fig.axes[2].patches],
                             [r['cout_mg_l'] for r in data['baseline_b0_held_out_duties']]
                             + [r['cout_mg_l'] for r in data['promoted_wave2_top_elite_held_out_duties']])

    def test_plate3_needs_no_raw_cases_and_omits_untraced_gate(self):
        read = Path.read_text

        def committed_only(path, *args, **kwargs):
            if 'runs' in path.parts:
                raise AssertionError('private raw case read during committed-source rendering')
            return read(path, *args, **kwargs)

        with patch.object(Path, 'read_text', committed_only), patch.object(Path, 'write_text'), \
                patch.object(Figure, 'savefig'), patch.object(plt, 'close'), \
                patch.object(gci, 'load_grid_case', side_effect=AssertionError('raw case access')):
            gci.main()
            self.assertNotIn('Gate', figure_text(plt.gcf()))
            self.assertNotIn('Realizability', figure_text(plt.gcf()))
            self.assertIn('Normal-Stress Positivity', figure_text(plt.gcf()))
            closure = load('elite-3grid-gci-and-rsm-verification-v2.json')
            expected = [r['water_closure_fraction'] * 1e6 for r in closure['grids']]
            expected += [r['water_closure_fraction'] * 1e6 for r in closure['rsm_turbulence_closure_crosscheck']]
            expected += [closure['baseline_ct35_comparator']['water_closure_fraction'] * 1e6]
            closure_axis = next(ax for ax in plt.gcf().axes if 'Mass Closure Residual' in ax.get_ylabel())
            self.assertEqual(list(closure_axis.lines[0].get_ydata()), expected)

    def test_plate10_preserves_nulls_and_rejects_zero_filling(self):
        bundle_path = ROOT / plate10.FIELD_ARRAYS
        bundle, grids = plate10.load_field_arrays(bundle_path)
        import numpy as np
        self.assertTrue(np.isnan(grids['wave1']['u_theta_p90_grid']).any())
        grid = bundle['cases']['wave1']['meridional']['u_theta_p90_grid']
        row = next(row for row in grid if None in row)
        row[row.index(None)] = 0.0
        with tempfile.TemporaryDirectory(dir=ROOT / '.private') as temp:
            path = Path(temp) / 'changed.json'
            path.write_text(json.dumps(bundle))
            with self.assertRaisesRegex(ValueError, 'missing-bin mask'):
                plate10.load_field_arrays(path)

    def test_plate10_renders_from_json_without_raw_cases_and_binds_caveats(self):
        files = [plate10.FIELD_ARRAYS, 'docs/results/elite-3grid-gci-and-rsm-verification-v2.json',
                 'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json']
        with tempfile.TemporaryDirectory(dir=ROOT / '.private') as temp:
            root = Path(temp)
            for rel in files:
                dest = root / rel
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_bytes((ROOT / rel).read_bytes())
            (root / 'docs/figures').mkdir()
            self.assertFalse((root / 'runs').exists())
            with patch.object(plate10, 'ROOT', root), \
                    patch.object(plate10, 'extract_meridional_rz_grid', side_effect=AssertionError('raw read')), \
                    patch.object(plate10, 'extract_case_profiles', side_effect=AssertionError('raw read')):
                plate10.main()
            path = root / 'docs/figures/wave2-vs-wave1-openfoam14-cfd-comparison.png'
            manifest = json.loads(Path(str(path) + '.manifest.json').read_text())
            self.assertEqual(manifest['source_receipts']['field_array_bundle_sha256'],
                             hashlib.sha256((root / plate10.FIELD_ARRAYS).read_bytes()).hexdigest())
            self.assertFalse(manifest['baseline_field_mesh_matches_archived_dossier'])
            self.assertIn('NOT finite-volume centroids', manifest['coordinate_reviews']['wave1']['method'])
            self.assertFalse((root / 'runs').exists())

    def test_live_manifests_bind_current_png_generator_and_all_sources(self):
        names = ['wave3-readme-hero-kpi-and-elite', 'wave3-evolution-ablation-and-lineage-proof',
                 'wave4-readme-hero-kpi-and-elite', 'wave4-evolution-ablation-and-lineage-proof',
                 'wave2-multiseed-ablation-and-ci', 'elite-gci-and-rsm-verification']
        for name in names:
            with self.subTest(figure=name):
                png = ROOT / f'docs/figures/{name}.png'
                manifest = json.loads(Path(str(png) + '.manifest.json').read_text())
                self.assertEqual(manifest['figure_sha256'], hashlib.sha256(png.read_bytes()).hexdigest())
                generator = ROOT / manifest['generator_script']
                self.assertEqual(manifest['generator_sha256'], hashlib.sha256(generator.read_bytes()).hexdigest())
                for source in manifest.get('source_receipts', manifest.get('source_artifacts', [])):
                    self.assertEqual(source['sha256'], hashlib.sha256((ROOT / source['path']).read_bytes()).hexdigest())
                if 'source_dossier' in manifest:
                    self.assertEqual(manifest['source_dossier_sha256'], hashlib.sha256((ROOT / manifest['source_dossier']).read_bytes()).hexdigest())
                self.assertNotIn('Phylogenetic', manifest.get('title', ''))
                self.assertNotIn('6-Arm', manifest.get('title', ''))


if __name__ == '__main__':
    unittest.main()
