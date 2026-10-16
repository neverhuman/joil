"""Read every Wave-4 case, including missing manifests and quarantined cases.

The primary filename defaults to the workplan's case.json. A case.json, when
present, must carry the exact scoped joil.wave4-case-identity.v1 record; the
separate case-identity.json status is always shown. The filename can be changed
only under an explicitly recorded integration contract decision. This audit
does not create, finalize, repair, prune or run any case. Exit 1 means FAIL rows.
"""

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))
from analysis.case_identity import MANIFEST, _digest, _json_object, _safe, build_case_identity


def discover(runs_root):
    root = Path(runs_root)
    rows = [('active', p) for p in root.glob('wave4-*') if p.is_dir()]
    rows += [('quarantined', p) for p in (root/'unfinalized-pruned-wave4').glob('wave4-*') if p.is_dir()]
    return sorted(rows, key=lambda x: (x[0], x[1].name))


def audit_case(scope, case, primary):
    result = dict(case=case.name, scope=scope, status='FAIL',
                  primary_manifest=primary, manifest_files={}, actual_chain=None,
                  retained_artifacts='FAIL', stable_snapshot=False, reasons=[])
    records = {}
    for name in sorted({'case.json', MANIFEST, primary}):
        try:
            path = _safe(case, name)
            records[name] = _json_object(path)
            result['manifest_files'][name] = dict(status='READ', present=True, **_digest(path))
        except (ValueError, KeyError, OSError) as exc:
            result['manifest_files'][name] = dict(status='FAIL',
                present=(case/name).exists() or (case/name).is_symlink(),
                reason=str(exc).replace(str(case), '$CASE'))
    try:
        first = build_case_identity(case)
        second = build_case_identity(case)
        result['stable_snapshot'] = first == second
        if not result['stable_snapshot']:
            raise ValueError('case input bytes changed during audit; retry after owner DONE')
        result['retained_artifacts'] = 'PASS'
        result['actual_chain'] = {key: second[key] for key in
                                  ['genome_hash', 'ir_hash', 'geometry_hash', 'mesh_hash', 'case_hash']}
        for name, recorded in records.items():
            match = recorded == second
            # Detect a receipt replaced during the potentially long artifact read.
            match = match and _digest(_safe(case, name)) == {
                k: result['manifest_files'][name][k] for k in ['sha256', 'bytes']}
            result['manifest_files'][name]['status'] = 'PASS' if match else 'FAIL'
            if not match:
                result['manifest_files'][name]['reason'] = 'manifest differs from exact retained bytes or changed during audit'
    except (ValueError, KeyError, OSError) as exc:
        result['reasons'].append(str(exc).replace(str(case), '$CASE'))
        for name in records:
            result['manifest_files'][name]['status'] = 'FAIL'
            result['manifest_files'][name]['reason'] = 'retained artifact chain could not be verified'
    primary_ok = result['manifest_files'][primary]['status'] == 'PASS'
    conflicting = any(info['present'] and info['status'] != 'PASS'
                      for name, info in result['manifest_files'].items())
    if primary_ok and not conflicting and result['stable_snapshot']:
        result['status'] = 'PASS'
    else:
        result['reasons'].append('required '+primary+' has not verified' if not primary_ok
                                 else 'conflicting supplied manifest')
    return result


def audit(runs_root, primary='case.json'):
    started = datetime.now(timezone.utc).isoformat()
    cases = discover(runs_root)
    results = [audit_case(scope, case, primary) for scope, case in cases]
    errors = []
    if not cases:
        errors.append('no Wave-4 cases; empty audit is not acceptance')
    if cases != discover(runs_root):
        errors.append('case directory set changed during audit; retry after owner DONE')
    counts = {scope: dict(Counter(r['status'] for r in results if r['scope'] == scope))
              for scope in ['active', 'quarantined']}
    return dict(schema='joil.ev12-every-run-audit.v1',
                kind='exact_scoped_artifact_identity_snapshot_not_qualification',
                started_utc=started, finished_utc=datetime.now(timezone.utc).isoformat(),
                primary_manifest=primary,
                contract='joil.wave4-case-identity.v1 exact record; broader ScientificIdentity remains separate',
                audit_source_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                case_count=len(results), counts=counts, cases=results, errors=errors,
                active_passed=bool(counts['active']) and counts['active'].get('FAIL', 0) == 0 and not errors,
                all_listed_passed=bool(results) and all(r['status'] == 'PASS' for r in results) and not errors,
                limitation='Quarantined cases are listed explicitly; moving them is not identity completion. Re-run after INT DONE for new grids; a live snapshot cannot certify future inputs.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runs-root', type=Path, required=True)
    parser.add_argument('--manifest-name', choices=['case.json', MANIFEST], default='case.json')
    args = parser.parse_args()
    report = audit(args.runs_root, args.manifest_name)
    print(json.dumps(report, indent=2))
    raise SystemExit(not report['all_listed_passed'])
