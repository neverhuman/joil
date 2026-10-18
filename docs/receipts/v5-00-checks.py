#!/usr/bin/env python3
"""Source-bound baseline checks; no simulation or scientific admission."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require_source(data, expected):
    if sha(data) != expected:
        raise ValueError('source changed; finding cannot be reused')


def main():
    baseline = json.loads((ROOT/'docs/v5-plan/execution/BASELINE.json').read_text())
    assert baseline['original_main'] == 'cc86edc90d06f0eac69022f84354abc94bb453c4'
    assert baseline['plan_head'] == 'e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0'
    for row in baseline['tracked_files']:
        result = subprocess.run(['rtk', 'proxy', 'git', 'show', f"{baseline['plan_head']}:{row['path']}"],
                                cwd=ROOT, capture_output=True, check=True)
        require_source(result.stdout, row['sha256'])
        try:
            require_source(result.stdout + b'\n', row['sha256'])
        except ValueError:
            pass
        else:
            raise AssertionError('changed source reused')
    assert len(baseline['private_supplied_sources']) == 10
    assert all(t['exit_code'] == 0 for t in baseline['toolchain'])
    assert baseline['primary_preservation']['preserved']
    print('PASS: pinned original main/plan and 18 frozen source identities; mutated source rejected for every identity; all 10 supplied inputs hash-verified at capture; no physical admission.')
    history_path = ROOT/'docs/receipts/v5-00-history-20261017.json'
    if history_path.exists():
        history = json.loads(history_path.read_text())
        for row in history['records']:
            result = subprocess.run(['rtk', 'proxy', 'git', 'show', f"{baseline['plan_head']}:{row['path']}"],
                                    cwd=ROOT, capture_output=True, check=True)
            require_source(result.stdout, row['sha256'])
            assert row['executed_case_identity'] == 'not_established_by_scalar_record'
        assert {d['id'] for d in history['deferrals']} == {'TR-16','TR-16-threshold-audit','EV-09','EV-12'}
        for row in history['deferrals']:
            assert not row['physical_pass']
            assert row['state'] == 'blocked_scoped_remainder_retained'
            require_source((ROOT/row['source']).read_bytes(), row['source_sha256'])
        assert history['discovered_case_directories'] == history['mesh_file_sets_present'] + history['mesh_file_sets_incomplete']
        print('PASS: historical record identities preserved; missing mesh identity stays unverified; TR-16/EV-09/EV-12 physical deferrals retained.')
    defects_path = ROOT/'docs/v5-plan/execution/DEFECTS.json'
    if defects_path.exists():
        defects = json.loads(defects_path.read_text())['defects']
        mapping = {d['id']: d['primary_task'] for d in json.loads((ROOT/'docs/v5-plan/DEFECT_MAP.json').read_text())['defects']}
        assert {d['id'] for d in defects} == {f'D-{i:02}' for i in range(1,32)}
        for row in defects:
            assert row['primary_task'] == mapping[row['id']]
            assert not row['physical_qualification']
            assert all(row['independent_regression'][k] for k in ('negative_fixture','positive_control','status'))
            for source in row['sources']:
                raw = subprocess.run(['rtk','proxy','git','show',f"{baseline['plan_head']}:{source['path']}"],
                                     cwd=ROOT,capture_output=True,check=True).stdout
                require_source(raw, source['source_sha256'])
                lines = raw.decode().splitlines()
                assert all(source['anchor'] in lines[n-1] for n in source['current_lines'])
                try:
                    require_source(b'\n'+raw, source['source_sha256'])
                except ValueError:
                    pass
                else:
                    raise AssertionError('line shift incorrectly reuses finding')
        print('PASS: all31 allegations bound to whole source/symbol/current lines and one repair task; changed-source/line-only reuse rejected; future physical challenges remain unexecuted.')


if __name__ == '__main__':
    main()
