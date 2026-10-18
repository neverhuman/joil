#!/usr/bin/env python3
"""Generate software-progress metadata from canonical records; no scientific observations."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools import v5_plan


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False, allow_nan=False) + "\n").encode()


def view(source):
    raw = encoded(source)
    identity = hashlib.sha256(raw).hexdigest()
    tasks = [row for packet in source['progress'].values() for row in packet['tasks'].values()]
    counts = {state: sum(row['status'] == state for row in tasks) for state in v5_plan.STATES}
    status = source['scientific_status']
    return {
        'schema_version': 1, 'revision': identity,
        'generated_at': 'Wall-clock time not recorded; revision identifies exact source bytes',
        'current': True,
        'source_snapshot': {'sha256': identity, 'source': './v5/source-' + identity + '.json'},
        'campaign': {
            'tasks_total': len(tasks), 'tasks_completed': counts['done'],
            'attempts': None, 'spend': None, 'reservations': None, 'checkpoint': None,
            'control_arms': [], 'admitted_capabilities': [],
            'blockers': status.get('campaign_blockers', []) + [
                str(counts['blocked']) + ' software tasks blocked; inspect the source snapshot for exact reasons',
                'V5-00-03 remains AUDIT-HELD on D-07, D-18 and D-21',
                'TR-16 / EV-09 / EV-12 remain deferred'],
            'software_status': ', '.join(str(counts[s]) + ' ' + s for s in v5_plan.STATES),
            'scientific_release': 'Scientific phase: ' + status['phase'] + '; campaign admission: ' + status['campaign_admission'],
        },
        # No physical value, champion, geometry or attempted campaign is inferred from task counts.
        'subjects': [], 'values': [], 'archive': [], 'historical_corrections': [],
    }, raw


def generated(root=ROOT):
    v5_plan.load(root)
    progress = {p.stem: v5_plan.read_json(p) for p in sorted((root / 'docs/v5-plan/progress').glob('*.json'))}
    source = {'schema_version': 1, 'kind': 'canonical_public_software_progress_not_physical_evidence',
              'task_registry_sha256': hashlib.sha256((root / 'docs/v5-plan/TASKS.json').read_bytes()).hexdigest(),
              'progress': progress, 'scientific_status': v5_plan.read_json(root / 'docs/STATUS.json')}
    manifest, raw = view(source)
    return {Path('apps/explorer/public/evidence-manifest.json'): encoded(manifest),
            Path('apps/explorer/public/v5/source-' + manifest['revision'] + '.json'): raw}


def apply(files, root=ROOT, *, check=False):
    if check:
        for name, data in files.items():
            path = root / name
            if path.is_symlink() or not path.is_file() or path.read_bytes() != data:
                raise ValueError('stale explorer manifest/source: ' + str(name))
        return
    for name, data in files.items():
        path = root / name
        if path.is_symlink():
            raise ValueError('refusing explorer output symlink')
        if name.parent.name == 'v5' and path.exists() and path.read_bytes() != data:
            raise ValueError('immutable source snapshot collision')
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=('build', 'check'))
    args = parser.parse_args()
    try:
        files = generated()
        apply(files, check=args.command == 'check')
        print('PASS: canonical software-progress explorer ' + args.command + '; physical observations absent')
        return 0
    except (ValueError, OSError, KeyError) as error:
        print('FAIL: ' + str(error), file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
