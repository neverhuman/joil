"""Account for every retained Wave-4 case without promoting incomplete identities.

Read-only audit of the owner-designated inputs, active/quarantined cases and G1
diagnostic. PASS means exact scoped identity OR an evidence-bound exclusion;
it never means every manifest verified, or physical/claim-bearing qualification.
No hashes are reconstructed for missing meshes. Exit 1 means unresolved coverage.
"""

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))
from analysis.case_identity import MANIFEST, _digest, _inventory, _json_object, build_case_identity
from analysis.evidence import scientific_hash

SUBJECT_CASE = 'wave4-b3-s20261108-g0-coarse'
G1_CASE = 'wave4-b3-s20261108-g1-medium'
EXPECTED_INPUTS = frozenset([
    'wave4-b3-s20261103-g0-coarse', 'wave4-b3-s20261105-g0-coarse',
    'wave4-b3-s20261106-g0-coarse', 'wave4-b3-s20261106-g1-medium',
    'wave4-b3-s20261106-g2-fine', 'wave4-b3-s20261107-g0-coarse',
    SUBJECT_CASE, 'wave4-b3-s20261110-g0-coarse',
    'wave4-ct35-control-g0-coarse', 'wave4-ct35-control-g1-medium',
    'wave4-ct35-control-g2-fine',
])
CHAIN_KEYS = ['genome_hash', 'ir_hash', 'geometry_hash', 'mesh_hash', 'case_hash']


def discover(runs):
    return sorted([(scope, p) for scope, root in [
        ('active', runs), ('quarantined', runs/'unfinalized-pruned-wave4')]
        for p in root.glob('wave4-*') if p.is_dir()], key=lambda x: (x[0], x[1].name))


def chain(record):
    return {k: record[k] for k in CHAIN_KEYS}


def diagnostic(case, subject):
    actual = build_case_identity(case)
    reference = build_case_identity(subject)
    if any(actual[k] != reference[k] for k in ['genome_hash', 'ir_hash']):
        raise ValueError('diagnostic does not bind the designated s20261108 subject')
    run_receipt = _json_object(case/'water-1-mpirun.log.receipt.json')
    code = run_receipt.get('exit_code')
    if isinstance(code, bool) or not isinstance(code, int) or code == 0:
        raise ValueError('diagnostic lacks a nonzero solver execution receipt')
    log = (case/'water-1-mpirun.orig.log').read_text()
    if not all(s in log for s in ['FOAM FATAL ERROR', 'Unknown patch name: inlet1', 'MPI_ABORT']):
        raise ValueError('diagnostic lacks the retained fatal missing-inlet/MPI log')
    wrapper_log = _digest(case/'water-1-mpirun.log')
    if wrapper_log != {k: run_receipt.get(v) for k, v in [('sha256', 'log_sha256'), ('bytes', 'log_bytes')]}:
        raise ValueError('diagnostic driver receipt does not bind its captured wrapper log')
    boundary = (case/'constant/polyMesh/boundary').read_text()
    patches = {m[0]: int(m[1]) for m in re.findall(
        r'(?m)^\s*(\w+)\s*\n\s*\{[^{}]*?nFaces\s+(\d+)\s*;', boundary)}
    if not patches or any(patches.get(name, 0) > 0 for name in ['inlet1', 'inlet2']):
        raise ValueError('diagnostic boundary does not corroborate missing/empty inlet patches')
    recorded = _json_object(case/MANIFEST) if (case/MANIFEST).exists() else None
    sidecar_state = 'ABSENT' if recorded is None else ('VERIFIED' if recorded == actual else 'STALE')
    changed = {section: [k for k in sorted(set(recorded.get(section, {})) | set(actual[section]))
                         if recorded.get(section, {}).get(k) != actual[section].get(k)]
               for section in ['source_artifacts', 'mesh_files', 'solver_input_files']} if recorded else {}
    return dict(
        disposition='EXCLUDED_FAILED_G1_DIAGNOSTIC', claim_eligible=False,
        reason='Missing inlet patch after meshing; solver fatally aborted. Retained as diagnostic only; no performance/identity qualification.',
        solver_exit_code=code, solver_time_steps_observed=len(re.findall(r'(?m)^Time\s*=', log)),
        solver_image_id=run_receipt.get('image_id'),
        log_binding='Driver receipt binds water-1-mpirun.log; fatal details remain in water-1-mpirun.orig.log because the shell aborted before copying them to wrapper stdout. Both files are separately fingerprinted.',
        boundary_patch_faces=patches,
        inlet_patches={name: dict(present=name in patches, n_faces=patches.get(name))
                       for name in ['inlet1', 'inlet2']},
        identity_sidecar_state=sidecar_state,
        recorded_chain=chain(recorded) if recorded else None,
        observed_retained_chain=chain(actual), changed_sidecar_inventory_paths=changed,
        valid_finalized_identity_for_retained_state=sidecar_state == 'VERIFIED',
        chronology=dict(owner_report='INT10:50:52 reports solver fatal before identity finalization',
                        independently_verified='Solver fatal and no valid final identity for the retained state when sidecar is stale/absent.',
                        caveat='A stale sidecar exists in this retained diagnostic; earlier board09:46 reports pre-solve finalization. No claim that no sidecar was ever written, and no inferred finalization timestamp.'),
        evidence={name: _digest(case/name) for name in [
            'water-1-mpirun.log.receipt.json', 'water-1-mpirun.log', 'water-1-mpirun.orig.log',
            'mesh-2-checkMesh.log', 'constant/polyMesh/boundary']},
        physical_infeasibility_inferred=False)


def audit(runs_root, inputs_root, diagnostic_root, expected_inputs=EXPECTED_INPUTS):
    runs, inputs, g1 = map(Path, [runs_root, inputs_root, diagnostic_root])
    started = datetime.now(timezone.utc).isoformat()
    expected = set(expected_inputs)
    inputs_before = sorted(p.name for p in inputs.glob('wave4-*') if p.is_dir())
    current = discover(runs)
    errors, rows, snapshots = [], [], []
    if not expected or set(inputs_before) != expected:
        errors.append('preserved input case set differs from pinned nonempty inventory')

    def snapshot(path):
        files = _inventory(path, ['.'])
        snapshots.append((path, files))
        return dict(file_count=len(files), total_bytes=sum(v['bytes'] for v in files.values()),
                    inventory_sha256=scientific_hash(files), files=files)

    names = expected | set(inputs_before) | {p.name for _, p in current} | {G1_CASE}
    for name in sorted(names):
        row = dict(case=name, accounted=False, claim_eligible=False, disposition='UNRESOLVED')
        rows.append(row)
        try:
            locations = [(scope, p) for scope, p in current if p.name == name]
            row['run_locations'] = [scope for scope, _ in locations]
            if name == G1_CASE:
                if locations:
                    raise ValueError('designated diagnostic also appears in runs; reconcile duplicate location')
                row['diagnostic_inventory'] = snapshot(g1)
                row.update(diagnostic(g1, inputs/SUBJECT_CASE))
                row['accounted'] = True
                continue
            if name not in expected or name not in inputs_before or len(locations) != 1:
                raise ValueError('case missing from inputs/current inventory, unexpected, or duplicated')
            source = inputs/name
            scope, case = locations[0]
            row['preserved_inventory'] = snapshot(source)
            row['current_inventory'] = snapshot(case)
            try:
                source_identity = build_case_identity(source)
            except ValueError as exc:
                if str(exc) != 'missing artifact directory: constant/polyMesh':
                    raise
                source_identity = None
            if source_identity is not None:
                if scope != 'active':
                    raise ValueError('complete retained input case unexpectedly quarantined')
                actual = build_case_identity(case)
                recorded = _json_object(case/MANIFEST)
                if actual != recorded or chain(actual) != chain(source_identity):
                    raise ValueError('active identity/retained input chain mismatch; cannot exclude a broken active case automatically')
                row.update(disposition='SCOPED_IDENTITY_VERIFIED', scientific_artifact_chain=chain(actual),
                           manifest=_digest(case/MANIFEST), accounted=True,
                           reason='Exact retained artifact chain verified; broader claim-bearing ScientificIdentity and physics admission remain separate.')
            else:
                if scope != 'quarantined' or row['preserved_inventory'] != row['current_inventory']:
                    raise ValueError('incomplete case not quarantined or differs from preserved input bytes')
                if (case/MANIFEST).exists():
                    raise ValueError('incomplete case has an unexplained supplied identity sidecar')
                receipt_path = case/'water-1-mpirun.log.receipt.json'
                code = _json_object(receipt_path).get('exit_code') if receipt_path.exists() else None
                prepared = _json_object(source/'input-manifest.json')
                partial = {key: _digest(source/rel)['sha256'] for key, rel in [
                    ('genome_hash', 'design-program.json'), ('ir_hash', 'design-ir.json'),
                    ('geometry_hash', prepared['surface_stl_relpath'])]}
                row.update(disposition='EXCLUDED_MISSING_EXECUTED_MESH', accounted=True,
                           retained_source_hashes=partial, mesh_hash=None, case_hash=None,
                           recorded_solver_exit_code=code,
                           execution_evidence='retained solver receipt' if code is not None else 'no retained solver receipt',
                           reason='Executed volume mesh and finalized identity unavailable; preserved bytes match quarantine. Excluded from claim-bearing use. Retained numbers, if any, are incomplete historical diagnostics; a rerun is a new run, not identity recovery.')
        except (ValueError, KeyError, TypeError, OSError) as exc:
            row['error'] = str(exc).replace(str(runs), '$RUNS').replace(str(inputs), '$INPUTS').replace(str(g1), '$G1')
    for path, files in snapshots:
        try:
            if _inventory(path, ['.']) != files:
                errors.append('retained artifact inventory changed during audit: '+path.name)
        except (ValueError, OSError) as exc:
            errors.append('retained artifact inventory disappeared/invalid during audit: '+path.name+': '+str(exc))
    if inputs_before != sorted(p.name for p in inputs.glob('wave4-*') if p.is_dir()) or current != discover(runs):
        errors.append('case directory set changed during audit')
    counts = dict(Counter(r['disposition'] for r in rows))
    passed = bool(rows) and all(r['accounted'] for r in rows) and not errors
    return dict(schema='joil.ev12-identity-or-disposition.v1',
                kind='scoped_case_accountability_not_physical_qualification',
                started_utc=started, finished_utc=datetime.now(timezone.utc).isoformat(),
                state='PASS_SCOPED_IDENTITY_OR_DISPOSITION' if passed else 'UNRESOLVED',
                passed=passed, expected_preserved_inputs=sorted(expected),
                case_count=len(rows), counts=counts, cases=rows, errors=errors,
                unresolved_cases=[r['case'] for r in rows if not r['accounted']],
                all_cases_have_verified_identity=counts.get('SCOPED_IDENTITY_VERIFIED', 0) == len(rows),
                claim_eligible_cases=0,
                audit_source_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                policy='OWNER11:07 permits full scoped Wave4 identity or explicit disposition. Missing/pruned meshes stay unknown; no finalized identity is synthesized. Historical/FEA remainder stays in section6.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runs-root', type=Path, required=True)
    parser.add_argument('--inputs-root', type=Path, required=True)
    parser.add_argument('--diagnostic-root', type=Path, required=True)
    args = parser.parse_args()
    report = audit(args.runs_root, args.inputs_root, args.diagnostic_root)
    print(json.dumps(report, indent=2, allow_nan=False))
    raise SystemExit(not report['passed'])
