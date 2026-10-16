"""Independent executable probes of the IR EV12 writer; all mutations are synthetic.

Build joil-core in this lane and compile ev12_writer_probe.rs against its rlib.
The probe accepts build/finalize/verify/archive/prune/prepare plus explicit paths.
Actual retained runs are read only. The fixture mesh is synthetic, not a CFD run.
Exit 1 means concrete writer findings, rather than a failed review execution.
"""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))
from analysis.case_identity import build_case_identity, finalize_case_identity


def review(probe, runs_root, reviewed_ir_commit=None):
    probe = Path(probe).resolve()
    checks, commands, findings = [], [], []

    def call(action, case, archive=None, cwd=ROOT):
        argv = [str(probe), action, str(case)]
        if archive is not None:
            argv.append(str(archive))
        p = subprocess.run(argv, cwd=cwd, capture_output=True, text=True)
        commands.append(dict(argv=argv, cwd=str(cwd), exit_code=p.returncode,
                             stdout=p.stdout, stderr=p.stderr))
        return p.returncode, json.loads(p.stdout)

    def check(name, passed, **details):
        checks.append(dict(check=name, passed=passed, **details))

    (ROOT/'.private').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=ROOT/'.private', prefix='ev12-review-') as directory:
        work = Path(directory)
        case = work/'synthetic-case'
        code, result = call('prepare', case)
        if code:
            raise ValueError(result)
        poly = case/'constant/polyMesh'
        poly.mkdir()
        for name in ['points', 'faces', 'owner', 'neighbour', 'boundary']:
            (poly/name).write_text('synthetic review '+name+'\n')
        # Public finalizer and verifier: same canonical record as the EV reader.
        code, result = call('finalize', case)
        check('valid_absolute_finalize_and_python_parity', code == 0 and result.get('value') == build_case_identity(case))
        check('missing_archive_refuses_prune', call('prune', case, work/'absent')[0] != 0 and poly.is_dir())
        archive = work/'archive'
        code, archive_result = call('archive', case, archive)
        check('absolute_archive_verifies_copy', code == 0)
        # Tampering must never allow pruning; restore exact bytes for later probes.
        points = archive/'constant/polyMesh/points'
        original = points.read_bytes()
        points.write_bytes(original+b' tampered')
        check('tampered_archive_refuses_prune', call('prune', case, archive)[0] != 0 and poly.is_dir())
        points.write_bytes(original)
        code, pruned = call('prune', case, archive)
        check('verified_disjoint_archive_allows_synthetic_prune', code == 0 and not poly.exists())
        a, b = archive_result['value'], pruned['value']
        consistent = (a['file_count'], a['total_bytes']) == (b['file_count'], b['total_bytes'])
        check('prune_receipt_matches_exact_archive_inventory', consistent,
              archive_file_count=a['file_count'], prune_file_count=b['file_count'],
              archive_total_bytes=a['total_bytes'], prune_total_bytes=b['total_bytes'])
        if not consistent:
            findings.append(dict(id='EV12-WR-01', priority='P2', path='crates/joil-core/src/cfd_case.rs',
                                 function='prune_polymesh_after_verified_archive',
                                 issue='Prune receipt double-counts shared STL inventory entries and excludes sidecar bytes; disagrees with exact archive receipt.',
                                 fix='Use the same deduplicated indexed-file inventory, including input manifest and sidecar, as archive_and_verify_wave4_case_inputs.'))

        # Distinct paths with identical retained bytes expose a false-success path.
        relative = archive.relative_to(ROOT)
        foreign_cwd = work/'foreign'
        foreign_case = foreign_cwd/relative
        foreign_case.parent.mkdir(parents=True)
        shutil.copytree(archive, foreign_case)
        shutil.copyfile(case/'ev12-capture-workflow.sh', foreign_case/'ev12-capture-workflow.sh')
        (archive/'case-identity.json').unlink()
        (foreign_case/'case-identity.json').unlink()
        code, relative_result = call('finalize', relative, cwd=foreign_cwd)
        good = code == 0 and (foreign_case/'case-identity.json').is_file() and not (archive/'case-identity.json').exists()
        check('relative_case_finalize_targets_callers_case', good, api_exit_code=code,
              callers_case_sidecar=(foreign_case/'case-identity.json').exists(),
              repository_relative_other_case_sidecar=(archive/'case-identity.json').exists())
        if not good:
            findings.append(dict(id='EV12-WR-02', priority='P1', path='crates/joil-core/src/cfd_case.rs',
                                 function='run_ev_python_case_identity',
                                 issue='Changing Python cwd to repo_root while passing an unresolved relative case path can finalize another same-bytes case and return success while the requested source has no sidecar.',
                                 fix='Resolve and validate the requested case to an absolute path before changing subprocess cwd; add a two-copy, foreign-cwd regression.'))

        # Builder canonical check must not silently disappear on typed decode failure.
        spec = importlib.util.spec_from_file_location('ev_synthetic_fixture', ROOT/'tests/test_evidence.py')
        fixtures = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(fixtures)
        malformed = fixtures.EvidenceTests()._synthetic_wave4_case(work, 'wave4-synthetic-bad-canonical')
        manifest_path = malformed/'input-manifest.json'
        manifest = json.loads(manifest_path.read_text())
        manifest['design_sha256'] = 'f'*64
        manifest_path.write_text(json.dumps(manifest))
        code, built = call('build', malformed)
        try:
            build_case_identity(malformed)
            python_rejected = False
        except ValueError:
            python_rejected = True
        check('rust_builder_rejects_forged_canonical_genome', code != 0,
              rust_exit_code=code, python_rejected=python_rejected)
        check('public_finalizer_rejects_forged_canonical_genome', call('finalize', malformed)[0] != 0)
        if code == 0:
            findings.append(dict(id='EV12-WR-03', priority='P1', path='crates/joil-core/src/cfd_case.rs',
                                 function='build_wave4_case_identity',
                                 issue='if-let-Ok DesignProgram parsing skips canonical_genome verification for a non-deserializable program; public Rust builder accepts forged canonical hash rejected by EV Python.',
                                 fix='Fail closed on unsupported DesignProgram decoding, or perform a complete order-preserving canonical check; never skip validation.'))

        # The emitted default $(pwd) workflow should run from its own case directory.
        env = os.environ.copy()
        env.pop('PYTHONPATH', None)
        p = subprocess.run(['bash', 'ev12-capture-workflow.sh'], cwd=foreign_case,
                           env=env, capture_output=True, text=True)
        commands.append(dict(argv=['bash', 'ev12-capture-workflow.sh'], cwd=str(foreign_case),
                             exit_code=p.returncode, stdout=p.stdout, stderr=p.stderr))
        check('emitted_workflow_imports_api_from_case_cwd', p.returncode == 0, stderr=p.stderr)
        if p.returncode:
            findings.append(dict(id='EV12-WR-04', priority='P2', path='crates/joil-core/src/cfd_case.rs',
                                 function='build_openfoam_candidate_case emitted ev12-capture-workflow.sh',
                                 issue='Default CASE_DIR=$(pwd) invocation inside case fails to import analysis.case_identity without externally supplied PYTHONPATH.',
                                 fix='Provide a reliable module/repository location or explicit repo argument; test emitted script from case cwd in a clean Python environment.'))

        # Before-solver capture should retain the mesh needed by the solver.
        env['PYTHONPATH'] = str(ROOT)
        shell_archive = work/'shell-archive'
        p = subprocess.run(['bash', str(foreign_case/'ev12-capture-workflow.sh'), str(foreign_case), str(shell_archive)],
                           cwd=ROOT, env=env, capture_output=True, text=True)
        commands.append(dict(argv=['bash', 'ev12-capture-workflow.sh', str(foreign_case), str(shell_archive)],
                             cwd=str(ROOT), exit_code=p.returncode, stdout=p.stdout, stderr=p.stderr))
        kept = p.returncode == 0 and (foreign_case/'constant/polyMesh').is_dir()
        check('pre_solver_capture_does_not_implicitly_prune_mesh', kept, exit_code=p.returncode,
              mesh_retained=(foreign_case/'constant/polyMesh').is_dir())
        if p.returncode and 'IndentationError' in p.stderr:
            findings.append(dict(id='EV12-WR-05', priority='P1', path='crates/joil-core/src/cfd_case.rs',
                                 function='emitted ev12-capture-workflow.sh archive Python block',
                                 issue='Rust escaped-newline string strips Python block indentation; archive invocation fails with IndentationError before copying anything.',
                                 fix='Use a raw string/preserve indentation and execute the generated script in tests. Separate before-solver capture from the current unconditional rm -rf cleanup.'))
        elif p.returncode == 0 and not kept:
            findings.append(dict(id='EV12-WR-05', priority='P1', path='crates/joil-core/src/cfd_case.rs',
                                 function='emitted ev12-capture-workflow.sh',
                                 issue='Supplying archive_dir to the documented before-solver workflow unconditionally deletes constant/polyMesh before solver execution.',
                                 fix='Separate capture from cleanup; require an explicit post-solver prune command/flag.'))

    # Absolute verify calls on actual runs never invoke prepare/finalize/archive/prune.
    actual = []
    for case in sorted(Path(runs_root).glob('wave4-*')):
        if not case.is_dir() or not (case/'case-identity.json').is_file():
            continue
        code, out = call('verify', case)
        actual.append(dict(case=case.name, passed=code == 0, result=out))
    return dict(kind='independent_writer_review_synthetic_mutations_real_runs_readonly',
                reviewed_ir_commit=reviewed_ir_commit,
                passed=not findings and all(x['passed'] for x in checks+actual),
                checks=checks, findings=findings, actual_retained_cases=actual,
                probe_sha256=hashlib.sha256(probe.read_bytes()).hexdigest(), commands=commands)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--probe', type=Path, default=ROOT/'target/ev12-writer-probe')
    parser.add_argument('--runs-root', type=Path, required=True)
    parser.add_argument('--reviewed-ir-commit', help='Commit of the source compiled into --probe; caller must verify binding')
    args = parser.parse_args()
    report = review(args.probe, args.runs_root, args.reviewed_ir_commit)
    print(json.dumps(report, indent=2))
    raise SystemExit(not report['passed'])
