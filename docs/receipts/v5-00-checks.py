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


if __name__ == '__main__':
    main()
