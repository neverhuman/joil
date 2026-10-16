"""Read-only proof-figure audit; renders only into an EV-private directory.

Use a pinned, selectively exported git snapshot as --input-root. Original source
PNGs, manifests and receipts stay unchanged. Capture the actual plotting calls,
compare receipt-bound data explicitly, and keep missing source coverage visible.
Raster equality binds the observed plot calls back to the committed PNG; it is
not scientific validation. Raw-field panels without committed arrays are gaps.
"""
import argparse
import ast
import hashlib
import importlib.util
import inspect
import json
from pathlib import Path
import sys

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.axes import Axes
from matplotlib.figure import Figure
import numpy as np
from PIL import Image


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def plain(value):
    if isinstance(value, np.ndarray): return plain(value.tolist())
    if isinstance(value, (list, tuple)): return [plain(x) for x in value]
    if isinstance(value, dict): return {str(k): plain(v) for k, v in value.items()}
    if isinstance(value, (np.integer, np.floating)): return plain(value.item())
    if isinstance(value, float) and not np.isfinite(value): return None
    if value is None or isinstance(value, (str, int, float, bool)): return value
    return repr(value)


def load_module(root, name):
    path = root/'tools'/name
    spec = importlib.util.spec_from_file_location('ev13_'+path.stem, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def capture(root, output):
    root, output = root.resolve(), output.resolve()
    if not output.is_relative_to(Path(__file__).resolve().parents[3]/'.private'):
        raise ValueError('render output must remain in EV-private storage')
    output.mkdir(parents=True, exist_ok=True)
    sys.path.insert(0, str(root))
    calls, figures, errors = {}, {}, []
    originals = {}
    methods = ['plot', 'step', 'scatter', 'bar', 'barh', 'fill_between', 'axhline',
               'axvline', 'errorbar', 'pcolormesh']
    for method in methods:
        original = getattr(Axes, method)
        originals[method] = original
        def wrapper(self, *args, _name=method, _original=original, **kwargs):
            frame = inspect.currentframe().f_back
            path = Path(frame.f_code.co_filename)
            if path.is_relative_to(root/'tools'):
                local = frame.f_locals
                bindings = {k: plain(local[k]) for k in ['key', 'idx', 'name', 's_idx', 'm_idx', 'j', 'i'] if k in local}
                if isinstance(local.get('o'), dict): bindings['seed'] = local['o'].get('seed')
                calls.setdefault(id(self.figure), []).append(dict(
                    method=_name, axis=self.figure.axes.index(self), source=path.relative_to(root).as_posix(),
                    line=frame.f_lineno, bindings=bindings, args=plain(args), kwargs=plain(kwargs)))
            return _original(self, *args, **kwargs)
        setattr(Axes, method, wrapper)
    save, write = Figure.savefig, Path.write_text
    def save_capture(fig, path, *args, **kwargs):
        filename = Path(path).name
        destination = output/filename
        save(fig, destination, *args, **kwargs)
        original = root/'docs/figures'/filename
        a, b = Image.open(original).convert('RGBA'), Image.open(destination).convert('RGBA')
        same_shape = a.size == b.size
        figures[filename] = dict(
            committed_sha256=sha(original), regenerated_sha256=sha(destination),
            raster_equal=same_shape and bool(np.array_equal(np.asarray(a), np.asarray(b))),
            committed_size=list(a.size), regenerated_size=list(b.size),
            figure_text=[t.get_text() for t in fig.texts],
            axes=[dict(title=ax.get_title(), xlabel=ax.get_xlabel(), ylabel=ax.get_ylabel(),
                       text=[t.get_text() for t in ax.texts],
                       legend=[t.get_text() for t in ax.get_legend().texts] if ax.get_legend() else [],
                       xlabels=[t.get_text() for t in ax.get_xticklabels()],
                       ylabels=[t.get_text() for t in ax.get_yticklabels()]) for ax in fig.axes],
            calls=calls.pop(id(fig), []))
    def private_write(path, text, *args, **kwargs):
        if not path.resolve().is_relative_to(root):
            raise ValueError('generator attempted a write outside its audit snapshot')
        return write(output/(path.name+'.generated'), text, *args, **kwargs)
    Figure.savefig, Path.write_text = save_capture, private_write
    try:
        proof = load_module(root, 'generate_wave3_proof_figures.py')
        w4 = json.loads((root/'docs/results/wave4-evolution-proof-v1.json').read_text())
        w3 = json.loads((root/'docs/results/wave3-evolution-proof-v1.json').read_text())
        v2 = json.loads((root/'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json').read_text())
        v1 = json.loads((root/'docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json').read_text())
        w1 = json.loads((root/'docs/results/elite-3grid-gci-and-rsm-verification-v2.json').read_text())
        hs = w1['roache_gci_analysis']['dp_u_pa']['three_grid_half_span_pct_of_f2']
        for wave, data in [('wave4', w4), ('wave3', w3)]:
            manifest = json.loads((root/f'docs/figures/{wave}-readme-hero-kpi-and-elite.png.manifest.json').read_text())
            cfd_name = Path(next(s['path'] for s in manifest['source_receipts']
                                 if Path(s['path']).name.startswith('wave2-tpms-elite-gci-rsm-and-f2-oil-'))).name
            cfd = v2 if cfd_name.endswith('-v2.json') else v1
            spread = hs if cfd_name.endswith('-v2.json') else 5.019376028759711
            plt.rcdefaults()
            path = root/f'docs/results/{wave}-evolution-proof-v1.json'
            proof.generate_readme_hero(data, cfd, sha(path), sha(root/'docs/results'/cfd_name),
                                      root/f'docs/figures/{wave}-readme-hero-kpi-and-elite.png',
                                      wave.replace('wave', 'Wave-'), spread, path.name, cfd_name)
            proof.generate_plate11_proof(data, sha(path), root/f'docs/figures/{wave}-evolution-ablation-and-lineage-proof.png',
                                         wave.replace('wave', 'Wave-'), path.relative_to(root).as_posix())
        for name in ['generate_page1_figure.py', 'generate_wave4_gci_figure.py']:
            plt.rcdefaults()
            module = load_module(root, name)
            if name == 'generate_page1_figure.py': series = module.extract_series(w4)
            else: series = module.extract_series(json.loads((root/'docs/results/wave4-gci-v1.json').read_text()), w1, v2)
            module.render(series, module.OUT)
        plt.rcdefaults()
        wave2 = load_module(root, 'generate_wave2_figures_and_animations.py')
        wave2.generate_ablation_plate(json.loads((root/'docs/results/wave2-multiseed-ablation-and-holdout-v1.json').read_text()))
        # Plate3's unchanged plotting block consumes the committed v2 dossier;
        # omit its preliminary private-raw re-analysis, which is not a JSON trace.
        plt.rcdefaults()
        gci = load_module(root, 'generate_gci_rsm_report_and_figures.py')
        tree = ast.parse((root/'tools/generate_gci_rsm_report_and_figures.py').read_text())
        main = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'main')
        begin = next(i for i, n in enumerate(main.body) if isinstance(n, ast.Assign)
                     and any(isinstance(t, ast.Name) and t.id == 'v2_json' for t in n.targets))
        main.body = main.body[begin:]
        exec(compile(ast.Module(body=[main], type_ignores=[]), str(root/'tools/generate_gci_rsm_report_and_figures.py'), 'exec'), gci.__dict__)
        gci.main()
        # Historical pins use raw fields. Current pins load FIELD_ARRAYS from
        # the snapshot root; include the committed bundle in the export. Never
        # substitute an older extractor callback for the current normal main.
        plt.rcdefaults()
        plate10 = load_module(root, 'generate_wave2_vs_wave1_cfd_plate.py')
        try: plate10.main()
        except (OSError, ValueError, KeyError) as exc:
            errors.append(dict(figure='wave2-vs-wave1-openfoam14-cfd-comparison.png', error=str(exc)))
    finally:
        Figure.savefig, Path.write_text = save, write
        for name, original in originals.items(): setattr(Axes, name, original)
        plt.close('all')
    return dict(figures=figures, capture_errors=errors)


if __name__ == '__main__':
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--input-root', type=Path, required=True)
    ap.add_argument('--output-root', type=Path, required=True)
    args = ap.parse_args()
    result = capture(args.input_root, args.output_root)
    (args.output_root/'captured.json').write_text(json.dumps(result, indent=2, allow_nan=False)+'\n')
    print(json.dumps({k: {name: row[name] for name in ['raster_equal', 'committed_size', 'regenerated_size']}
                      for k, row in result['figures'].items()}, indent=2))
    print(json.dumps(result['capture_errors']))
