#!/usr/bin/env python3
"""Executed red/positive controls for the 16 source-only V5-00 baseline findings (grant C006).

Every control runs the frozen e1e9203 component, never the current checkout bytes: a known-valid
case where an independently stated assertion holds (positive) and the counterexample on which the
same frozen component fails it (red). Static rows check frozen repository contracts and say so.
Blocked rows refuse to produce evidence and name the missing contract, input or harness.

  v5_baseline_source_controls.py matrix
  v5_baseline_source_controls.py extract DEST
  v5_baseline_source_controls.py build-frozen-rust FROZEN_ROOT   (needs CARGO_TARGET_DIR)
  v5_baseline_source_controls.py run ROW {positive,red} --scratch DIR [--frozen-rust DIR]

`run` owns a fresh or empty scratch directory and refuses anything else, so it can never reuse a
probe or overwrite earlier evidence. It exits 0 when the assertion holds, 1 with
`C006-RED <row>: <code> ...` when it fails, 2 with `C006-SETUP` on any scratch, inventory,
extraction, binding, compile or import error (never evidence) and 3 with `C006-BLOCKED` for
blocked rows. Synthetic fixtures are labelled synthetic; nothing here is a physical result, a
repair or a V5-00 acceptance.
"""
import argparse
import hashlib
import importlib.util
import io
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import sysconfig
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
FROZEN = 'e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0'
DEFECTS = ROOT / 'docs/v5-plan/execution/DEFECTS.json'
PROFILE = 'c006frozen'


class Red(Exception):
    """The independent assertion failed on the frozen component."""


class Setup(Exception):
    """Binding, extraction, compilation or import failure; never evidence."""


ROWS = {
    'D-05': {'kind': 'runtime', 'rust': True,
             'assertion': 'An equal-IR mesh with k sealed closed shells beyond the open fluid shell reports trapped_powder_cavities >= k; with k = 0 nothing is trapped and terminals stay reachable.',
             'positive': 'Frozen compiled baseline mesh (open connected cavity, k = 0).',
             'red': 'Same IR and mesh plus one SYNTHETIC sealed tetrahedral shell inside the wall band (k = 1).'},
    'D-07': {'kind': 'blocked',
             'blocker': 'Needs one accepted solid artifact with independently integrated volume (V5-13 input); frozen Phenotype3D::from_program has no seam accepting an arbitrary solid, so neither the positive nor the red can run on the frozen component.'},
    'D-10': {'kind': 'static',
             'assertion': 'STATIC: quantities declared by both frozen requirement sources admit a common value.',
             'positive': 'Quality and stretch limits (v1 thresholds vs RS-v1 quality).',
             'red': 'PDR: v1 sandbox_pdr_min vs RS-v1 limits.pdr_draft_range.',
             'limits': 'Original positive (explicit synthetic sandbox contract validates software without physical admission) is the current decision test complete_source_bound_synthetic_profile_is_semantically_valid_without_physical_admission, pinned in the receipt at the tested commit.'},
    'D-14': {'kind': 'runtime',
             'assertion': 'Each jet\'s seed share of a cell group equals that group\'s inward-flux share (U . n_in per equal SYNTHETIC cell area) within one 1/16 stratum.',
             'positive': 'Original positive: frozen verify_passive_tracer_split_oracle preserves inlet concentration; plus SYNTHETIC jets with all velocities parallel to the declared inlet normal (|U| equals inward flux).',
             'red': 'SYNTHETIC jets, equal speed, one azimuth half tangent to the inlet face (zero inward flux).'},
    'D-16': {'kind': 'runtime', 'rust': True,
             'assertion': 'Two wall authorities evaluating the same accepted program agree on the chamber wall radius at common sample points within 1e-9 mm.',
             'positive': 'Original positive: the exact inactive baseline of each implementation (Rust DesignProgram::baseline_ct35, Python default_exotic_program) agrees; plus frozen Rust Phenotype3D::inner_radius_mm vs Python exotic_graph.chamber_radius_mm on a recorded Wave-2 elite whose TPMS band lies inside [16, 66] mm.',
             'red': 'Same pair on a recorded Wave-2 elite whose TPMS band ends above 66 mm (valid for both normalizers).',
             'limits': 'cad/program/wall_field.compile_program (third authority) needs cadquery, absent here; accepted boundary artifacts are absent, so the full mismatch envelope stays unmeasured.'},
    'D-17': {'kind': 'runtime', 'rust': True,
             'assertion': 'The GPU audit boundary launches no child process without an explicit lease admission.',
             'positive': 'Frozen attach_cuda_audit_with_vram with the CUDA binary absent (the frozen disabled path).',
             'red': 'Same call with a SYNTHETIC recording stand-in executable and the frozen 85.5% VRAM request; no lease object exists in the API.'},
    'D-18': {'kind': 'blocked',
             'blocker': 'The frozen island loop has no injection seam for a recording fake evaluator; host idleness and speedup are unmeasured performance hypotheses, and running the loop would be a search campaign outside this lease.'},
    'D-21': {'kind': 'blocked',
             'blocker': 'Runtime evidence needs a browser/DOM render of frozen apps/explorer/src/main.tsx (not provisioned here). The frozen scene has no subject/time/field-bound trajectory manifest, so no positive exists on the frozen component; a text match would only be static source search.'},
    'D-22': {'kind': 'runtime',
             'assertion': 'Every plotted series is bound to evidence in the input record: a series follows changes in its evidence, and a series with no evidence is not plotted.',
             'positive': 'Frozen structural plate, actual Wave-2 record vs the same record with one family\'s TPMS amplitude raised by a SYNTHETIC 1 mm: only that family\'s Panel C bar changes.',
             'red': 'Frozen structural plate with every modal key removed from the record: Panel B still plots a modal-frequency series.',
             'limits': 'Original positive (a verified modal table bound to subject, load/material and units) has no frozen or current component; the positive is a known-valid case on the same frozen plate.'},
    'D-24': {'kind': 'runtime', 'rust': True,
             'assertion': 'An output command writes a fresh private destination, and refuses an existing destination while preserving its bytes.',
             'positive': 'Frozen joil-cli verify-rotation --out <fresh private path>.',
             'red': 'Frozen joil-cli verify-rotation --out <existing SYNTHETIC private sentinel>.'},
    'D-25': {'kind': 'static',
             'assertion': 'STATIC POLICY: the recorded v4 review default keeps LLMs out of judging and admits no agent/LLM proposal channel into the evolutionary search loop (current user authorization, as recorded in the V5-00 inventory).',
             'positive': 'Judge clause of the frozen GENOME_V4.md "LLM proposals" default.',
             'red': 'Proposal clause of the same frozen default.',
             'limits': 'Original runtime fixtures need an operator admission registry, absent at the frozen commit; this row is static policy only.'},
    'D-27': {'kind': 'runtime',
             'assertion': 'The publication gate admits reviewed public plan files and refuses a candidate artifact that carries no owner-admitted disclosure class.',
             'positive': 'Frozen check_public.violations on the frozen docs/v5-plan/INPUTS_AND_RISKS.md bytes.',
             'red': 'Frozen check_public.violations on a SYNTHETIC private candidate geometry export placed on a public path.'},
    'D-28': {'kind': 'blocked',
             'blocker': 'No ownership/acknowledgement checker exists at the frozen commit or now. Frozen tools/v5_plan.update is status bookkeeping, and recording an owner name is not a permission grant (integrator review 2026-10-18T02:29:42Z), so treating it as one would invent an ack-admission contract. The documented protocol (COORDINATION.md: a notice is a proposal until acknowledged) is already resolved for workers; its original positive needs the current private acknowledgement records, which are not public test inputs.'},
    'D-29': {'kind': 'blocked',
             'blocker': 'The negative fixture needs an architecture traversal checker, absent at the frozen commit; module size alone is not a measured defect, so there is no frozen component to run.'},
    'D-30': {'kind': 'runtime', 'rust': True,
             'assertion': 'Rust and Python GCI/KPI copies agree with independent equations on shared fixtures, including the same explicit state for an unrepresentable result.',
             'positive': 'Analytic monotone f = 1 + 0.5 h^2 (h = 4, 2, 1; p = 2), a sign-changing triple, and the passive-tracer oracle Cu = Ci when E = r.',
             'red': 'SYNTHETIC overflow fixture cin = 1e308 mg/L, capture 0, split 0.5 (Cu not finite).'},
    'D-31': {'kind': 'runtime',
             'assertion': 'A displayed numeric claim is accepted only while a live source with the same subject and unit supports it; an unsupported or revoked claim is invalidated.',
             'positive': 'Frozen claims-audit registry and scanner: a SYNTHETIC claim with its live same-subject kPa source is accepted, and an unbound numeral is refused.',
             'red': 'Same scanner after the claim\'s source is revoked, leaving only a SYNTHETIC different-subject wall thickness in mm with the same numeral.',
             'limits': 'Original positive (a live exact-query dependency chain reconstructible offline) has no frozen component; the positive is a known-valid case on the same frozen scanner.'},
}

PYTHON_SOURCES = {
    'D-10': ['requirements/joil-hydrocyclone-v1.toml', 'requirements/RS-joil-research-v1.json'],
    'D-14': ['physics/openfoam/oil/oil_grade.py', 'physics/openfoam/environment/runtime.py', 'analysis/kpi_fitness.py'],
    'D-16': ['cad/program/exotic_graph.py', 'cad/ct35', 'baselines/ct35/genome.json',
             'docs/results/wave2-multiseed-ablation-and-holdout-v1.json'],
    'D-22': ['tools/generate_wave2_figures_and_animations.py', 'docs/results/wave2-multiseed-ablation-and-holdout-v1.json'],
    'D-25': ['docs/design/GENOME_V4.md'],
    'D-27': ['tools/check_public.py', 'docs/v5-plan/INPUTS_AND_RISKS.md'],
    'D-30': ['tools/generate_gci_rsm_report_and_figures.py', 'analysis/kpi_fitness.py'],
    'D-31': ['tools/claims_audit.py'],
}
RUST_ARTIFACTS = {'D-05': ['libjoil_core', 'libjoil_ir'], 'D-16': ['libjoil_core', 'libjoil_ir'], 'D-17': ['libjoil_core', 'libjoil_ir'],
                  'D-24': ['joil-cli'], 'D-30': ['libjoil_core', 'libjoil_ir']}
ARTIFACT_FILES = {'libjoil_core': 'libjoil_core.rlib', 'libjoil_ir': 'libjoil_ir.rlib', 'joil-cli': 'joil-cli'}
BUILD_ARGV = ['cargo', 'build', '--locked', '--offline', '--manifest-path', '<frozen-e1-extract>/Cargo.toml',
              '--config', f'profile.{PROFILE}.inherits="dev"', '--profile', PROFILE, '-p', 'joil-core', '-p', 'joil-ir', '-p', 'joil-cli']
COMPILED_PROBES = {}  # probe path -> sha256, only for probes compiled by this invocation
PROBE_PROVENANCE = {}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def git(*args, binary=False):
    proc = subprocess.run(['git', '-C', str(ROOT), *args], capture_output=True)
    if proc.returncode != 0:
        raise Setup(f'git {args[0]} failed with exit {proc.returncode} (frozen {FROZEN[:7]} needs full history): '
                    f'{proc.stderr.decode(errors="replace").strip()[-500:]}')
    return proc.stdout if binary else proc.stdout.decode()


def load_inventory(defects=DEFECTS):
    try:
        inventory = json.loads(Path(defects).read_text())
    except (OSError, ValueError) as err:
        raise Setup(f'defects inventory unreadable: {type(err).__name__}: {err}') from err
    rows = inventory.get('defects') if isinstance(inventory, dict) else None
    if not isinstance(rows, list) or not all(isinstance(r, dict) and isinstance(r.get('id'), str) for r in rows):
        raise Setup('malformed defects inventory: expected a "defects" list of rows with string ids')
    return rows


def recorded(row, defects=DEFECTS):
    match = [r for r in load_inventory(defects) if r['id'] == row]
    if len(match) != 1:
        raise Setup(f'{row} appears {len(match)} times in the defects inventory')
    state, sources = match[0].get('evidence_state'), match[0].get('sources')
    if not isinstance(state, str) or state.startswith('runtime_reproduced'):
        raise Setup(f'{row} is not a source-only row')
    if not isinstance(sources, list) or not sources or any(
            not isinstance(s, dict) or not isinstance(s.get('path'), str) or not isinstance(s.get('symbol'), str)
            or not re.fullmatch(r'[0-9a-f]{64}', str(s.get('source_sha256', ''))) for s in sources):
        raise Setup(f'malformed recorded sources for {row}')
    return sources


def prepare_scratch(scratch):
    """A run owns a fresh or empty directory, so it can never reuse a probe or overwrite evidence."""
    scratch = Path(scratch)
    if scratch.is_symlink() or (scratch.exists() and not scratch.is_dir()):
        raise Setup(f'scratch is not a directory: {scratch}')
    if scratch.exists() and any(scratch.iterdir()):
        held = sorted(p.name for p in scratch.iterdir())
        raise Setup(f'scratch is not empty; refusing to reuse a probe or overwrite evidence: {scratch} holds {held[:5]}')
    scratch.mkdir(parents=True, exist_ok=True)
    return scratch.resolve()


def extract(paths, dest):
    data = git('archive', '--format=tar', FROZEN, '--', *paths, binary=True)
    with tarfile.open(fileobj=io.BytesIO(data)) as tar:
        tar.extractall(dest, filter='data')


def frozen_blobs(paths):
    data = git('archive', '--format=tar', FROZEN, '--', *paths, binary=True)
    with tarfile.open(fileobj=io.BytesIO(data)) as tar:
        return {m.name: tar.extractfile(m).read() for m in tar.getmembers() if m.isfile()}


def bind_python(row, dest, defects):
    out = []
    for source in recorded(row, defects):
        path = dest / source['path']
        if path.is_file():
            actual = sha(path.read_bytes())
            if actual != source['source_sha256']:
                raise Setup(f'binding mismatch {source["path"]}: {actual} != {source["source_sha256"]}')
            out.append({'path': source['path'], 'symbol': source['symbol'], 'sha256': actual, 'recorded': True})
    for rel in PYTHON_SOURCES.get(row, []):
        path = dest / rel
        if path.is_file() and not any(b['path'] == rel for b in out):
            out.append({'path': rel, 'symbol': None, 'sha256': sha(path.read_bytes()), 'recorded': False})
    return out


def bind_rust(row, rust_dir, defects):
    """Require every workspace source in each artifact's dep-info to equal the frozen blob."""
    rust_dir = Path(rust_dir)
    deps, frozen_root = [], None
    for artifact in RUST_ARTIFACTS[row]:
        info = rust_dir / f'{artifact}.d'
        if not info.is_file():
            raise Setup(f'missing frozen dep-info {info}')
        deps += [os.path.normpath(d) for d in info.read_text().split('\n', 1)[0].split(': ', 1)[1].split()]
    for dep in deps:
        if dep.endswith('/Cargo.toml') or '/crates/' not in dep:
            continue
        frozen_root = dep[:dep.index('/crates/') + 1]
        break
    if frozen_root is None:
        raise Setup('cannot locate frozen root from dep-info')
    rels = sorted({d[len(frozen_root):] for d in deps if d.startswith(frozen_root)})
    blobs = frozen_blobs(rels)
    for rel in rels:
        if rel not in blobs or (Path(frozen_root) / rel).read_bytes() != blobs[rel]:
            raise Setup(f'frozen Rust source drift: {rel}')
    out = []
    for source in recorded(row, defects):
        if source['path'] in rels:
            if sha(blobs[source['path']]) != source['source_sha256']:
                raise Setup(f'binding mismatch {source["path"]}')
            out.append({'path': source['path'], 'symbol': source['symbol'], 'sha256': source['source_sha256'], 'recorded': True})
    if not out:
        raise Setup(f'{row}: no recorded source among frozen artifact inputs')
    artifacts = {}
    for artifact in RUST_ARTIFACTS[row]:
        path = rust_dir / ARTIFACT_FILES[artifact]
        if not path.is_file():
            raise Setup(f'missing frozen artifact {path}')
        artifacts[ARTIFACT_FILES[artifact]] = sha(path.read_bytes())
    return out, {'frozen_root': frozen_root, 'dep_info_sources_checked': len(rels), 'artifact_sha256': artifacts,
                 'profile': PROFILE, 'build_argv': BUILD_ARGV, 'toolchain': toolchain()}


def rustc_path():
    return shutil.which('rustc') or str(Path.home() / '.cargo/bin/rustc')


def toolchain():
    proc = subprocess.run([rustc_path(), '-vV'], capture_output=True, text=True)
    if proc.returncode != 0:
        raise Setup(f'rustc -vV failed: {proc.stderr.strip()[-300:]}')
    return dict(line.split(': ', 1) for line in proc.stdout.splitlines()[1:] if ': ' in line)


def load_module(name, path, dest):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    root = ROOT.resolve()
    frozen = Path(dest).resolve()
    # CI intentionally installs interpreter dependencies in a checkout-local
    # virtualenv. They are interpreter inputs, not current repository modules.
    # Resolve both sides: a dependency-path symlink into source must still fail.
    libraries = {
        Path(value).resolve()
        for key in ('purelib', 'platlib', 'stdlib', 'platstdlib')
        if (value := sysconfig.get_path(key))
        and not root.is_relative_to(Path(value).resolve())
    }
    for mod in list(sys.modules.values()):
        f = getattr(mod, '__file__', None)
        if f:
            origin = Path(f).resolve()
            if (origin.is_relative_to(root) and not origin.is_relative_to(frozen)
                    and origin != Path(__file__).resolve()
                    and not any(origin.is_relative_to(library) for library in libraries)):
                raise Setup(f'current-checkout module leaked into frozen run: {f}')
    return module


RUST_PROBE = r'''
// C006 probe of frozen e1e9203 components. SYNTHETIC inputs only; no GPU, CFD or search.
use joil_core::{evolve, geometry, metrics, program, structure};
use joil_ir::{BackendId, IrCompiledMesh, VoxelSdfObserver};

fn num(x: Option<f64>) -> String { x.map(|v| format!("{v:?}")).unwrap_or_else(|| "null".into()) }

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a[1].as_str() {
        "d05" => {
            let p = program::DesignProgram::baseline_ct35();
            let ir = geometry::compile_design_ir(&p).unwrap();
            let mut mesh = IrCompiledMesh::compile(&ir, BackendId::RustMultiPatchCfdSurface).unwrap();
            let r_wall = IrCompiledMesh::inner_radius_m(&ir, 0.0, 0.040);
            let (r0, s) = (r_wall + 0.0006, 0.0003);
            let shells = if a[2] == "sealed" { 1 } else { 0 };
            for _ in 0..shells {
                let b = mesh.vertices_m.len();
                mesh.vertices_m.extend([[r0, 0.0, 0.040], [r0 + s, 0.0, 0.040], [r0, s, 0.040], [r0, 0.0, 0.040 + s]]);
                mesh.triangles.extend([[b, b + 2, b + 1], [b, b + 1, b + 3], [b + 1, b + 2, b + 3], [b + 2, b, b + 3]]);
                let o = mesh.triangle_origin_ids[0];
                mesh.triangle_origin_ids.extend([o; 4]);
            }
            let r = VoxelSdfObserver::observe(&ir, &mesh);
            println!("{{\"sealed_shells_added\":{shells},\"inner_wall_radius_m\":{r_wall:?},\"shell_radial_span_m\":[{r0:?},{:?}],\"connected_fluid_components\":{},\"trapped_fluid_cavities\":{},\"trapped_powder_cavities\":{},\"trapped_cavities_total\":{},\"all_terminals_reachable_from_chamber\":{}}}",
                r0 + s, r.connected_fluid_components, r.trapped_fluid_cavities, r.trapped_powder_cavities, r.trapped_cavities_total, r.all_terminals_reachable_from_chamber);
        }
        "d17" => {
            let cfg = evolve::EvolutionConfig::default();
            let (mut ind, _) = evolve::evaluate_individual("c006-synthetic-baseline".into(), program::DesignProgram::baseline_ct35(), 0, 0, &cfg).unwrap();
            evolve::attach_cuda_audit_with_vram(&mut ind, std::path::Path::new(&a[2]), Some(85.5));
            println!("{{\"cuda_bin_path_configured_by_default\":{},\"cuda_sdf_audit_attached\":{}}}", cfg.cuda_bin_path.is_some(), ind.cuda_sdf_audit.is_some());
        }
        "d16" => {
            let spec: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&a[2]).unwrap()).unwrap();
            let p = if spec["baseline"].as_bool() == Some(true) {
                program::DesignProgram::baseline_ct35()
            } else {
                serde_json::from_value::<program::DesignProgram>(spec["program"].clone()).unwrap()
            };
            let p = p.canonicalize().unwrap();
            let d = geometry::baseline().derive().unwrap();
            let radii: Vec<String> = spec["points"].as_array().unwrap().iter().map(|pt| {
                let (theta, z) = (pt[0].as_f64().unwrap(), pt[1].as_f64().unwrap());
                format!("{:?}", joil_core::phenotype::Phenotype3D::inner_radius_mm(&p, &d, theta, z))
            }).collect();
            println!("{{\"chamber_length_mm\":{:?},\"r1_mm\":{:?},\"radii_mm\":[{}]}}", d.chamber_length_mm, d.d1_mm / 2.0, radii.join(","));
        }
        "gci" => {
            let v: Vec<f64> = a[2..5].iter().map(|x| x.parse().unwrap()).collect();
            match structure::three_grid_order([v[0], v[1], v[2]], a[5].parse().unwrap()) {
                Ok(r) => println!("{{\"state\":\"{:?}\",\"order\":{}}}", r.state, num(r.observed_order)),
                Err(e) => println!("{{\"error\":{e:?}}}"),
            }
        }
        "conc" => {
            let v: Vec<f64> = a[2..5].iter().map(|x| x.parse().unwrap()).collect();
            match metrics::concentration_mg_l(v[0], v[1], v[2]) {
                Ok(c) => println!("{{\"value\":{c:?}}}"),
                Err(e) => println!("{{\"error\":{e:?}}}"),
            }
        }
        other => panic!("unknown probe {other}"),
    }
}
'''


def rust_probe(rust_dir, scratch):
    """Compile the probe once per invocation; never run a probe this invocation did not compile."""
    exe = Path(scratch) / 'c006-probe'
    if exe in COMPILED_PROBES:
        if not exe.is_file() or sha(exe.read_bytes()) != COMPILED_PROBES[exe]:
            raise Setup(f'probe changed after compilation; refusing to execute {exe}')
        return exe
    src = Path(scratch) / 'c006-probe.rs'
    if exe.exists() or exe.is_symlink() or src.exists():
        raise Setup(f'pre-existing probe refused and never executed: {exe}')
    src.write_text(RUST_PROBE)
    d = Path(rust_dir)
    serde_json = sorted((d / 'deps').glob('libserde_json-*.rlib'))
    if len(serde_json) != 1:
        raise Setup(f'expected one frozen serde_json rlib, found {len(serde_json)}')
    argv = [rustc_path(), '--edition=2021', str(src), '--extern', f'joil_core={d}/libjoil_core.rlib',
            '--extern', f'joil_ir={d}/libjoil_ir.rlib', '--extern', f'serde_json={serde_json[0]}',
            '-L', f'dependency={d}/deps', '-o', str(exe)]
    proc = subprocess.run(argv, capture_output=True, text=True, env={**os.environ, 'TMPDIR': str(scratch)})
    if proc.returncode != 0:
        raise Setup(f'probe compilation failed (not evidence): {proc.stderr[-2000:]}')
    COMPILED_PROBES[exe] = sha(exe.read_bytes())
    PROBE_PROVENANCE.update({
        'sha256': COMPILED_PROBES[exe], 'source_sha256': sha(RUST_PROBE.encode()),
        'serde_json_rlib': {'name': serde_json[0].name, 'sha256': sha(serde_json[0].read_bytes())},
        'compile_argv': ['rustc', '--edition=2021', 'c006-probe.rs', '--extern', 'joil_core=<profile>/libjoil_core.rlib',
                         '--extern', 'joil_ir=<profile>/libjoil_ir.rlib', '--extern', f'serde_json=<profile>/deps/{serde_json[0].name}',
                         '-L', 'dependency=<profile>/deps', '-o', 'c006-probe']})
    return exe


def probe(rust_dir, scratch, *args):
    proc = subprocess.run([str(rust_probe(rust_dir, scratch)), *map(str, args)], capture_output=True, text=True, timeout=300)
    if proc.returncode != 0:
        raise Setup(f'probe {args[0]} crashed (not evidence): {proc.stderr[-2000:]}')
    return json.loads(proc.stdout.strip().splitlines()[-1])


def control_d05(mode, ctx):
    obs = probe(ctx['rust'], ctx['scratch'], 'd05', 'sealed' if mode == 'red' else 'open')
    k = obs['sealed_shells_added']
    if k == 0 and (obs['trapped_cavities_total'] != 0 or not obs['all_terminals_reachable_from_chamber']):
        raise Red(f'open_baseline_not_accessible {json.dumps(obs)}')
    if obs['trapped_powder_cavities'] < k:
        raise Red(f'sealed_pocket_not_reported_as_powder trapped_powder={obs["trapped_powder_cavities"]} '
                  f'trapped_fluid={obs["trapped_fluid_cavities"]} sealed_shells={k}')
    return obs


def control_d10(mode, ctx):
    v1 = tomllib.loads((ctx['dest'] / 'requirements/joil-hydrocyclone-v1.toml').read_text())
    rs = json.loads((ctx['dest'] / 'requirements/RS-joil-research-v1.json').read_text())
    if mode == 'positive':
        pairs = {'quality_limit_mg_l': (v1['thresholds']['quality_limit_mg_l'], rs['quality']['research_limit_mg_l']),
                 'stretch_limit_mg_l': (v1['thresholds']['stretch_limit_mg_l'], rs['quality']['stretch_limit_mg_l'])}
        obs = {'static': True, 'pairs': pairs}
        for name, (a, b) in pairs.items():
            if a != b:
                raise Red(f'requirement_sources_disagree {name} v1={a} rs={b}')
        return obs
    lo = v1['thresholds']['sandbox_pdr_min']
    rlo, rhi = rs['limits']['pdr_draft_range']
    obs = {'static': True, 'v1_sandbox_pdr_min': lo, 'rs_pdr_draft_range': [rlo, rhi]}
    if max(lo, rlo) > rhi:
        raise Red(f'requirement_sources_disagree pdr v1_sandbox_pdr_min={lo} outside rs_pdr_draft_range=[{rlo}, {rhi}]')
    return obs


def control_d14(mode, ctx):
    import numpy as np
    oil = load_module('c006_oil_grade', ctx['dest'] / 'physics/openfoam/oil/oil_grade.py', ctx['dest'])
    n_in, tangent = np.array([1.0, 0.0, 0.0]), np.array([0.0, 0.0, 1.0])  # SYNTHETIC declared inlet normal
    centres, velocity, group_b = [], [], []
    for sign in (1.0, -1.0):
        theta = sign * np.linspace(0.4, 2.7, 32)
        upper = theta > np.median(theta)  # group B: the larger-azimuth half of each jet
        for t, b in zip(theta, upper):
            centres.append([0.030 * math.cos(t), 0.030 * math.sin(t), 0.006])
            group_b.append(bool(b))
            if mode == 'positive':
                velocity.append((6.0 if b else 3.0) * n_in)
            else:
                velocity.append(3.0 * (tangent if b else n_in))
    centres, velocity, group_b = np.array(centres), np.array(velocity), np.array(group_b)
    flux = np.clip(velocity @ n_in, 0.0, None)  # equal SYNTHETIC cell areas
    picked = oil.select_flux_weighted_inlet_seeds(centres, velocity)['all_indices']
    obs = {'synthetic': True, 'seeds': len(picked), 'jets': []}
    if mode == 'positive':
        obs['passive_tracer_oracle'] = oil.verify_passive_tracer_split_oracle()
        if not obs['passive_tracer_oracle']['passed']:
            raise Red(f'passive_tracer_oracle_failed {json.dumps(obs["passive_tracer_oracle"])}')
    for j in range(2):
        jet = slice(32 * j, 32 * (j + 1))
        seeds = [i for i in picked[16 * j:16 * (j + 1)]]
        share = sum(group_b[i] for i in seeds) / 16.0
        flux_share = float(flux[jet][group_b[jet]].sum() / flux[jet].sum())
        obs['jets'].append({'seed_share_group_b': share, 'inward_flux_share_group_b': flux_share})
        if abs(share - flux_share) > 1.0 / 16.0 + 1e-12:
            raise Red(f'seed_share_ignores_inward_flux jet={j} seed_share={share:.4f} inward_flux_share={flux_share:.4f}')
    return obs


def control_d16(mode, ctx):
    graph = load_module('c006_exotic_graph', ctx['dest'] / 'cad/program/exotic_graph.py', ctx['dest'])
    w2 = json.loads((ctx['dest'] / 'docs/results/wave2-multiseed-ablation-and-holdout-v1.json').read_text())

    def programs(x, path):  # every recorded design program, in document order
        if isinstance(x, dict):
            if {'schema_version', 'family', 'tpms_insert'} <= set(x):
                yield path, x
            for k, v in x.items():
                yield from programs(v, f'{path}/{k}')
        elif isinstance(x, list):
            for i, v in enumerate(x):
                yield from programs(v, f'{path}/{i}')
    in_band = lambda prog: 16.0 <= prog['tpms_insert']['axial_start_mm'] and prog['tpms_insert']['axial_end_mm'] <= 66.0
    where, program = next((p, prog) for p, prog in programs(w2, '') if prog['tpms_insert'].get('basis', 'None') != 'None'
                          and in_band(prog) == (mode == 'positive'))
    _, d = sys.modules['geometry'].load()
    points = [[theta, z] for theta in (0.0, 0.7, 1.9, 3.1, 4.4, 5.6) for z in (12.0, 18.0, 24.0, 30.0, 36.0, 42.0, 48.0, 54.0, 60.0)]

    def compare(name, rust_spec, python_program):
        spec = ctx['scratch'] / f'd16-{name}.json'
        spec.write_text(json.dumps({**rust_spec, 'points': points}))
        rust = probe(ctx['rust'], ctx['scratch'], 'd16', spec)
        if max(z for _, z in points) > rust['chamber_length_mm'] or abs(rust['r1_mm'] - d['r1']) > 1e-12:
            raise Setup('sample grid leaves the chamber or the two authorities use different r1')
        p_norm = graph.normalize_program(python_program)
        python = [graph.chamber_radius_mm(p_norm, d['r1'], theta, z) for theta, z in points]
        diffs = [abs(a - b) for a, b in zip(python, rust['radii_mm'])]
        return diffs, max(range(len(diffs)), key=diffs.__getitem__)

    obs = {}
    if mode == 'positive':
        diffs, worst = compare('baseline', {'baseline': True}, graph.default_exotic_program(2))
        obs['inactive_baseline_max_abs_diff_mm'] = diffs[worst]
        if diffs[worst] > 1e-9:
            raise Red(f'wall_authorities_disagree inactive_baseline max_abs_diff_mm={diffs[worst]:.6g} theta_z={points[worst]}')
    diffs, worst = compare('recorded', {'program': program}, program)
    band = [program['tpms_insert']['axial_start_mm'], program['tpms_insert']['axial_end_mm']]
    obs.update({'recorded_program': where, 'tpms_basis': program['tpms_insert']['basis'], 'tpms_band_mm': band,
                'points': len(points), 'max_abs_diff_mm': diffs[worst], 'worst_point_theta_z': points[worst]})
    if diffs[worst] > 1e-9:
        raise Red(f'wall_authorities_disagree recorded_program={where} tpms_band_mm={band} max_abs_diff_mm={diffs[worst]:.6g} '
                  f'theta_z={points[worst]}')
    return obs


def control_d17(mode, ctx):
    record = ctx['scratch'] / 'gpu-boundary-record.txt'
    exe = ctx['scratch'] / ('fake_joil_cuda_eval' if mode == 'red' else 'absent_joil_cuda_eval')
    if mode == 'red':
        exe.write_text('#!/bin/sh\n# SYNTHETIC recording stand-in for runs/bin/joil_cuda_eval; never touches a GPU.\n'
                       f'printf "%s\\n" "$@" > "{record}"\nexit 3\n')
        exe.chmod(0o755)
    obs = probe(ctx['rust'], ctx['scratch'], 'd17', exe)
    obs['child_launched'] = record.exists()
    if record.exists():
        argv = record.read_text().splitlines()
        vram = argv[argv.index('--vram-target-pct') + 1] if '--vram-target-pct' in argv else None
        obs.update({'child_argv_flags': [x for x in argv if x.startswith('--')], 'vram_target_pct': vram})
        raise Red(f'gpu_child_launched_without_lease flags={obs["child_argv_flags"]} vram_target_pct={vram}')
    return obs


def control_d22(mode, ctx):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.figure
    from matplotlib.patches import Rectangle
    gen = load_module('c006_wave2_figures', ctx['dest'] / 'tools/generate_wave2_figures_and_animations.py', ctx['dest'])
    gen.FIG_DIR = ctx['scratch'] / 'figures'
    gen.FIG_DIR.mkdir(exist_ok=True)
    captured = []
    original = matplotlib.figure.Figure.savefig

    def tap(fig, *args, **kwargs):  # observation only; the frozen figure is still saved unchanged
        captured.append({ax.get_title()[:2]: {'lines': [(ln.get_label(), [float(y) for y in ln.get_ydata()]) for ln in ax.get_lines()],
                                              'bars': [float(p.get_height()) for p in ax.patches if isinstance(p, Rectangle)]}
                         for ax in fig.axes})
        return original(fig, *args, **kwargs)
    matplotlib.figure.Figure.savefig = tap
    w2 = json.loads(gen.WAVE2_JSON.read_text())
    fams = sorted(w2['promoted_wave2_elites_by_family'])
    if mode == 'positive':
        gen.generate_structural_qualification_plate(json.loads(json.dumps(w2)))
        changed = json.loads(json.dumps(w2))
        tpms = changed['promoted_wave2_elites_by_family'][fams[0]]['program'].setdefault('tpms_insert', {})
        tpms['amplitude_mm'] = float(tpms.get('amplitude_mm', 0.0)) + 1.0  # SYNTHETIC perturbation
        gen.generate_structural_qualification_plate(changed)
        before, after = captured[0]['C.']['bars'], captured[1]['C.']['bars']
        diff = [i for i, (x, y) in enumerate(zip(before, after)) if x != y]
        obs = {'families': len(fams), 'panel_c_bars_changed': diff, 'changed_family_index': 0}
        if len(before) != len(fams) or diff != [0]:
            raise Red(f'series_not_bound_to_evidence panel=C changed={diff}')
        return obs

    removed = []

    def strip(x):
        if isinstance(x, dict):
            removed.extend(k for k in x if 'modal' in k.lower())
            return {k: strip(v) for k, v in x.items() if 'modal' not in k.lower()}
        return [strip(v) for v in x] if isinstance(x, list) else x

    def keys(x):
        if isinstance(x, dict):
            return [k for k in x] + [k2 for v in x.values() for k2 in keys(v)]
        return [k for v in x for k in keys(v)] if isinstance(x, list) else []
    stripped = strip(w2)
    if not removed or any('modal' in k.lower() for k in keys(stripped)):
        raise Setup('modal evidence removal did not apply')
    gen.generate_structural_qualification_plate(stripped)
    modal = [(label, ys) for label, ys in captured[0]['B.']['lines'] if 'mode' in label.lower()]
    obs = {'modal_keys_removed': len(removed), 'modal_key_names': sorted(set(removed)),
           'panel_b_series': [label for label, _ in captured[0]['B.']['lines']]}
    if modal:
        label, ys = modal[0]
        raise Red(f'modal_series_without_evidence label={label!r} min_hz={min(ys)} max_hz={max(ys)} points={len(ys)}')
    return obs


def control_d24(mode, ctx):
    exe = Path(ctx['rust']) / 'joil-cli'
    target = ctx['scratch'] / ('existing-sentinel.json' if mode == 'red' else 'fresh-output.json')
    before = None
    if mode == 'red':
        target.write_bytes(b'SYNTHETIC historical sentinel; must survive\n')
        before = sha(target.read_bytes())
    proc = subprocess.run([str(exe), 'verify-rotation', '--out', str(target)], capture_output=True, timeout=300)
    obs = {'binary_sha256': sha(exe.read_bytes()), 'exit_code': proc.returncode, 'exists': target.exists()}
    if before is None:
        if proc.returncode != 0 or not target.exists() or 'oracle' not in json.loads(target.read_text()):
            raise Red(f'fresh_destination_not_written exit={proc.returncode}')
        return obs
    after = sha(target.read_bytes())
    obs.update({'sentinel_sha256_before': before, 'sentinel_sha256_after': after})
    if proc.returncode == 0 or after != before:
        raise Red(f'existing_destination_overwritten exit={proc.returncode} sha_before={before} sha_after={after}')
    return obs


def control_d25(mode, ctx):
    text = (ctx['dest'] / 'docs/design/GENOME_V4.md').read_text()
    row = next(line for line in text.splitlines() if line.startswith('| LLM proposals |'))
    default = [c.strip() for c in row.strip('|').split('|')][2]
    obs = {'static': True, 'recorded_default': default}
    clauses = [c.strip() for c in default.split(';')]
    if mode == 'positive':
        if 'judge is never an LLM' not in clauses:
            raise Red(f'policy_admits_llm_judge default={default!r}')
        return obs
    if any(c.startswith('allowed') for c in clauses):
        raise Red(f'policy_admits_agent_proposals_in_search_loop default={default!r}')
    return obs


def control_d27(mode, ctx):
    gate = load_module('c006_check_public', ctx['dest'] / 'tools/check_public.py', ctx['dest'])
    root_names = git('ls-tree', '--name-only', FROZEN).split()
    obs = {'frozen_root_license_files': [n for n in root_names if n.upper().startswith(('LICENSE', 'COPYING'))],
           'private_path_control': gate.violations('runs/c006/synthetic.json', b'{}')}
    if mode == 'positive':
        name = 'docs/v5-plan/INPUTS_AND_RISKS.md'
        obs['violations'] = gate.violations(name, (ctx['dest'] / name).read_bytes())
        if obs['violations']:
            raise Red(f'reviewed_public_file_refused {obs["violations"]}')
        return obs
    name = 'docs/results/synthetic-candidate-geometry.glb'
    obs['violations'] = gate.violations(name, b'SYNTHETIC private candidate geometry export; owner disclosure class: none\n')
    if not obs['violations']:
        raise Red(f'candidate_without_disclosure_class_admitted path={name} violations=[]')
    return obs


def control_d30(mode, ctx):
    gci = load_module('c006_gci', ctx['dest'] / 'tools/generate_gci_rsm_report_and_figures.py', ctx['dest'])
    kpi = load_module('c006_kpi', ctx['dest'] / 'analysis/kpi_fitness.py', ctx['dest'])
    rust = lambda *a: probe(ctx['rust'], ctx['scratch'], *a)
    if mode == 'positive':
        f = [1.0 + 0.5 * h * h for h in (4.0, 2.0, 1.0)]  # independent oracle: p = 2, Monotone
        osc = [1.0, 1.2, 1.1]  # independent oracle: differences change sign
        py = lambda v: {k: gci.compute_roache_3grid_gci_v2(*v, 4.0, 2.0, 1.0)[k] for k in ('gci_validity_state', 'observed_order_p')}
        obs = {'monotone': {'rust': rust('gci', *f, 2.0), 'python': py(f)},
               'oscillatory': {'rust': rust('gci', *osc, 2.0), 'python': py(osc)},
               'tracer': {'rust': rust('conc', 200.0, 0.02, 0.02), 'python': kpi.concentration_mg_l(200.0, 0.02, 0.02)}}
        m, o, t = obs['monotone'], obs['oscillatory'], obs['tracer']
        p_py = m['python']['observed_order_p']
        if not (m['rust']['state'] == m['python']['gci_validity_state'] == 'Monotone' and abs(m['rust']['order'] - 2.0) < 1e-9
                and p_py is not None and abs(p_py - 2.0) < 1e-9):
            raise Red(f'monotone_oracle_disagrees {json.dumps(m)}')
        if not o['rust']['state'] == o['python']['gci_validity_state'] == 'Oscillatory':
            raise Red(f'oscillatory_oracle_disagrees {json.dumps(o)}')
        if not (t['rust'].get('value') == 200.0 and t['python'] == 200.0):
            raise Red(f'tracer_oracle_disagrees {json.dumps(t)}')
        return obs
    cin, capture, split = 1e308, 0.0, 0.5  # SYNTHETIC: Cu = 2e308 is not representable
    r = rust('conc', cin, capture, split)
    try:
        p = kpi.concentration_mg_l(cin, capture, split)
        p_state = {'value': p if math.isfinite(p) else repr(p)}
    except ValueError as err:
        p_state = {'error': str(err)}
    obs = {'synthetic': True, 'rust': r, 'python': p_state}
    if ('error' in r) != ('error' in p_state) or ('value' in p_state and not isinstance(p_state['value'], float)):
        raise Red(f'explicit_state_disagrees rust={json.dumps(r)} python={json.dumps(p_state)}')
    return obs


def control_d31(mode, ctx):
    audit = load_module('c006_claims_audit', ctx['dest'] / 'tools/claims_audit.py', ctx['dest'])
    live = {'synthetic-receipt-subject-a.json': {'subject': 'synthetic-subject-A', 'quantity': 'dp_u', 'unit': 'kPa', 'value': 12.3456}}
    other = {'synthetic-receipt-subject-b.json': {'subject': 'synthetic-subject-B', 'quantity': 'wall_thickness', 'unit': 'mm', 'value': 12.3456}}
    claim = {'SYNTHETIC.md': 'SYNTHETIC subject A underflow pressure drop is 12.3456 kPa.'}
    if mode == 'positive':
        bound, _ = audit.scan_markdown_numeric_tokens(claim, audit.build_allowed_numeric_registry(live))
        unbound, _ = audit.scan_markdown_numeric_tokens({'SYNTHETIC.md': 'SYNTHETIC subject A underflow pressure drop is 13.2468 kPa.'},
                                                        audit.build_allowed_numeric_registry(live))
        obs = {'synthetic': True, 'bound_claim_errors': bound, 'unbound_claim_errors': len(unbound)}
        if bound or not unbound:
            raise Red(f'claim_binding_not_enforced bound_errors={len(bound)} unbound_errors={len(unbound)}')
        return obs
    errors, scanned = audit.scan_markdown_numeric_tokens(claim, audit.build_allowed_numeric_registry(other))
    obs = {'synthetic': True, 'scanned_numerals': scanned, 'errors': errors, 'remaining_source': other}
    if not errors:
        raise Red('revoked_claim_still_accepted numeral=12.3456 remaining_match=synthetic-subject-B wall_thickness mm')
    return obs


CONTROLS = {'D-05': control_d05, 'D-10': control_d10, 'D-14': control_d14, 'D-16': control_d16, 'D-17': control_d17, 'D-22': control_d22,
            'D-24': control_d24, 'D-25': control_d25, 'D-27': control_d27, 'D-30': control_d30,
            'D-31': control_d31}


def run(row, mode, scratch, rust_dir, defects):
    spec = ROWS[row]
    if spec['kind'] == 'blocked':
        print(f'C006-BLOCKED {row}: {spec["blocker"]}', file=sys.stderr)
        return 3
    result = {'row': row, 'mode': mode, 'kind': spec['kind'], 'frozen_commit': FROZEN,
              'assertion': spec['assertion'], 'case': spec[mode]}
    try:
        scratch = prepare_scratch(scratch)
        dest = scratch / 'frozen'
        recorded(row, defects)
        bindings = []
        if row in PYTHON_SOURCES:
            extract(PYTHON_SOURCES[row], dest)
            bindings += bind_python(row, dest, defects)
        if spec.get('rust'):
            if not rust_dir:
                raise Setup(f'{row} needs --frozen-rust (target/{PROFILE} built from the frozen tree)')
            rust_bindings, meta = bind_rust(row, rust_dir, defects)
            bindings += rust_bindings
            result['frozen_rust'] = meta
        if not any(b['recorded'] for b in bindings):
            raise Setup(f'{row}: no recorded frozen source bound')
        result['bindings'] = bindings
        ctx = {'dest': dest, 'scratch': scratch, 'rust': rust_dir}
        try:
            result['observation'] = CONTROLS[row](mode, ctx)
            result['assertion_holds'] = True
        except Red as red:
            result['assertion_holds'] = False
            result['red_reason'] = str(red)
        except Setup:
            raise
        except Exception as err:  # a crash is never a reproduced defect
            raise Setup(f'control crashed (not evidence): {type(err).__name__}: {err}') from err
        if PROBE_PROVENANCE:
            result['frozen_rust']['probe'] = dict(PROBE_PROVENANCE)
    except Setup as err:
        print(f'C006-SETUP {row} {mode}: {err}', file=sys.stderr)
        return 2
    except Exception as err:  # extraction, filesystem or input failure outside a control
        print(f'C006-SETUP {row} {mode}: {type(err).__name__}: {err} (not evidence)', file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True))
    if not result['assertion_holds']:
        print(f'C006-RED {row}: {result["red_reason"]}', file=sys.stderr)
        return 1
    return 0


def build_frozen_rust(frozen_root):
    if not os.environ.get('CARGO_TARGET_DIR'):
        raise SystemExit('set CARGO_TARGET_DIR to the existing worker target; the frozen build uses its own profile there')
    argv = ['cargo', 'build', '--locked', '--offline', '--manifest-path', str(Path(frozen_root) / 'Cargo.toml'),
            '--config', f'profile.{PROFILE}.inherits="dev"', '--profile', PROFILE, '-p', 'joil-core', '-p', 'joil-ir', '-p', 'joil-cli']
    return subprocess.run(argv).returncode


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('matrix')
    ex = sub.add_parser('extract')
    ex.add_argument('dest')
    br = sub.add_parser('build-frozen-rust')
    br.add_argument('frozen_root')
    rp = sub.add_parser('run')
    rp.add_argument('row', choices=sorted(ROWS))
    rp.add_argument('mode', choices=('positive', 'red'))
    rp.add_argument('--scratch', required=True)
    rp.add_argument('--frozen-rust')
    rp.add_argument('--defects', default=str(DEFECTS))
    args = parser.parse_args()
    if args.command == 'run':
        return run(args.row, args.mode, args.scratch, args.frozen_rust, args.defects)
    try:
        if args.command == 'matrix':
            inventory = {r['id']: r for r in load_inventory()}
            rows = {row: {**spec, 'primary_task': inventory[row]['primary_task'],
                          'evidence_state': inventory[row]['evidence_state']} for row, spec in ROWS.items()}
            print(json.dumps({'frozen_commit': FROZEN, 'rows': rows}, indent=2, sort_keys=True))
            return 0
        if args.command == 'extract':
            if Path(args.dest).exists():
                raise Setup(f'refusing to extract into existing {args.dest}')
            extract(['.'], Path(args.dest))
            return 0
        return build_frozen_rust(args.frozen_root)
    except (Setup, KeyError) as err:
        print(f'C006-SETUP {args.command}: {type(err).__name__}: {err}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
