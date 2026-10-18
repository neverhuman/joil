#!/usr/bin/env python3
"""Audit frozen baseline inventory; this never qualifies a physical claim."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
FROZEN = 'e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0'
EXECUTION = Path('docs/v5-plan/execution')
REMAINDERS = frozenset(('TR-12', 'G-01', 'G-04', 'G-05', 'INT-06', 'F2-Oil',
                        'F4', 'TR-16', 'TR-16-threshold-audit', 'EV-09', 'EV-12',
                        'V4-follow-on-program'))


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def source_bound(raw, expected, source=None):
    require(sha(raw) == expected, 'source changed; inventory reuse refused')
    if source:
        lines = raw.decode().splitlines()
        require(bool(source['symbol']) and bool(source['anchor']), 'missing source anchor')
        require(bool(source['current_lines']) and all(
            isinstance(n, int) and 0 < n <= len(lines)
            and source['anchor'] in lines[n - 1] for n in source['current_lines']),
            'source anchor/line drift')


def validate(inventory_raw, mapping, gates, deferrals, packets, read_source=None):
    """Validate manifests; a reader additionally verifies exact frozen source bytes."""
    inventory = json.loads(inventory_raw)
    require(inventory['frozen_source'] == gates['frozen_source_commit'] ==
            deferrals['frozen_source_commit'] == FROZEN, 'frozen source drift')
    require(gates['inventory_path'] == str(EXECUTION / 'DEFECTS.json') and
            gates['inventory_sha256'] == sha(inventory_raw), 'inventory identity drift')
    expected = {f'D-{n:02d}' for n in range(1, 32)}
    original = {r['id']: r for r in inventory['defects']}
    repairs = {r['id']: r['primary_task'] for r in mapping['defects']}
    records = gates['records']
    require(len(records) == len(original) == len(repairs) == 31 and
            {r['id'] for r in records} == set(original) == set(repairs) == expected,
            'defect coverage/duplicates')
    behaviors = 0
    for row in records:
        old = original[row['id']]
        probe = old['minimal_reproducer']['runtime_probe']
        require(row['primary_task'] == old['primary_task'] == repairs[row['id']],
                'primary repair drift')
        require(row['baseline_disposition'] == old['evidence_state'], 'disposition drift')
        require(row['source_finding_sha256'] == [s['source_sha256'] for s in old['sources']],
                'source identity drift')
        require(row['runtime_probe'] == probe and
                row['behavior_probe_recorded_at_baseline'] is bool(probe) and
                row['inventory_evidence_kind'] == (
                    'behavior_probe_recorded' if probe else 'source_only_inventory'),
                'unexecuted regression promoted')
        require(row['positive_fixture'] == old['independent_regression']['positive_control']
                and row['negative_fixture'] == old['independent_regression']['negative_fixture']
                and bool(row['positive_fixture']) and bool(row['negative_fixture']),
                'independent challenge missing/drift')
        require('executed' in row['primary_repair_closure'] and
                'inventory alone cannot close' in row['primary_repair_closure'],
                'primary repair closure gate missing')
        require(row['physical_qualification'] is False and
                old['physical_qualification'] is False, 'physical promotion refused')
        behaviors += bool(probe)
        if read_source:
            for source in old['sources']:
                source_bound(read_source(source['path']), source['source_sha256'], source)
    require(behaviors == 15, 'baseline behavior count drift')
    rows = deferrals['deferrals']
    require(len(rows) == len(REMAINDERS) and {r['id'] for r in rows} == REMAINDERS,
            'v4 deferral coverage/duplicates')
    require(deferrals['source_path'] == 'docs/WORKPLAN-2026-10-15.md', 'v4 source drift')
    for row in rows:
        require(all(isinstance(row.get(k), str) and row[k].strip() for k in (
            'source_location', 'title', 'historical_scoped_disposition', 'owner',
            'next_dependency')), 'deferral owner/dependency/source missing')
        require(row['target_packets'] and set(row['target_packets']) <= set(packets),
                'deferral target missing')
        require(row['physical_pass'] is False and
                row['current_state'] == 'blocked_scoped_remainder_retained',
                'deferral promotion refused')
    if read_source:
        source_bound(read_source(deferrals['source_path']), deferrals['source_sha256'])
    return {'allegations': 31, 'recorded_behavior_probes': behaviors,
            'source_only_inventory': 31 - behaviors, 'retained_deferrals': len(rows),
            'frozen_source_bytes_checked': read_source is not None,
            'physical_qualification': False}


def load(root=ROOT, source_check=False):
    def read(name):
        return json.loads((root / name).read_text())

    def frozen(path):
        return subprocess.run(['git', 'show', f'{FROZEN}:{path}'], cwd=root,
                              capture_output=True, check=True).stdout

    return validate((root / EXECUTION / 'DEFECTS.json').read_bytes(),
                    read('docs/v5-plan/DEFECT_MAP.json'),
                    read(EXECUTION / 'DEFECT_ACCEPTANCE_GATES.json'),
                    read(EXECUTION / 'V4_DEFERRALS.json'),
                    [p['id'] for p in read('docs/v5-plan/TASKS.json')['packets']],
                    frozen if source_check else None)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-check', action='store_true',
                        help='also read exact historical Git objects (requires frozen commit)')
    print(json.dumps(load(source_check=parser.parse_args().source_check), sort_keys=True))
