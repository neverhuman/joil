#!/usr/bin/env python3
"""Build/check the planning views; progress is never scientific qualification."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
PLAN = Path('docs/v5-plan')
STATES = ('todo', 'in_progress', 'blocked', 'review', 'done')
SHA = re.compile(r'^[0-9a-f]{64}$')


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'duplicate JSON key: {key}')
        result[key] = value
    return result


def read_json(path):
    return json.loads(path.read_text(), object_pairs_hook=unique_object,
                      parse_constant=lambda v: (_ for _ in ()).throw(ValueError(f'nonfinite: {v}')))


def digest(data):
    return hashlib.sha256(data).hexdigest()


def inside(root, name):
    if not isinstance(name, str) or Path(name).is_absolute():
        raise ValueError(f'expected relative path: {name}')
    path = (root / name).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError(f'path escapes root: {name}')
    return path


def validate_receipt(root, ref, task_id):
    if set(ref) != {'path', 'sha256'} or not SHA.fullmatch(ref['sha256']):
        raise ValueError(f'{task_id}: invalid receipt reference')
    path = inside(root, ref['path'])
    if not path.is_file() or digest(path.read_bytes()) != ref['sha256']:
        raise ValueError(f'{task_id}: missing or stale receipt {path}')
    receipt = read_json(path)
    fields = ('task_ids', 'tested_commit', 'changed_paths', 'commands', 'positive_checks',
              'negative_checks', 'artifacts', 'resource_allocation', 'limitations', 'next_dependency')
    if any(key not in receipt for key in fields) or task_id not in receipt['task_ids']:
        raise ValueError(f'{task_id}: incomplete receipt')
    if not re.fullmatch(r'[0-9a-f]{40}', receipt['tested_commit']):
        raise ValueError(f'{task_id}: invalid tested commit')
    if not receipt['changed_paths'] or not receipt['commands'] or not receipt['positive_checks'] or not receipt['negative_checks']:
        raise ValueError(f'{task_id}: receipt lacks checks or paths')
    for name in receipt['changed_paths']:
        inside(root, name)
    for command in receipt['commands']:
        if not isinstance(command.get('argv'), list) or not command['argv'] or command.get('exit_code') != 0:
            raise ValueError(f'{task_id}: incomplete or failed command in completion receipt')
        if not SHA.fullmatch(command.get('log_sha256', '')):
            raise ValueError(f'{task_id}: missing log hash')
    if not receipt['artifacts'] or any(not SHA.fullmatch(a.get('sha256', '')) for a in receipt['artifacts']):
        raise ValueError(f'{task_id}: missing artifact hashes')


def load(root=ROOT):
    data = read_json(root / PLAN / 'TASKS.json')
    if data.get('schema_version') != 1 or len(data.get('packets', [])) != 32:
        raise ValueError('expected v1 registry with 32 packets')
    packets = data['packets']
    by_id = {p['id']: p for p in packets}
    if len(by_id) != len(packets):
        raise ValueError('duplicate packet ID')
    all_tasks = []
    progress = {}
    for p in packets:
        if not re.fullmatch(r'V5-\d{2}', p['id']) or not p['write_scope'] or not p['read_first']:
            raise ValueError(f'incomplete packet: {p["id"]}')
        for key in ('owner_role', 'edge_cases', 'validation_commands', 'legacy_packets'):
            if not p.get(key):
                raise ValueError(f'{p["id"]}: missing {key}')
        if len(set(p['dependencies'])) != len(p['dependencies']):
            raise ValueError(f'{p["id"]}: duplicate dependency')
        if any(d not in by_id or d == p['id'] for d in p['dependencies']):
            raise ValueError(f'{p["id"]}: invalid dependency')
        for path in p['read_first'] + p['write_scope']:
            inside(root, path)
        if any(s.startswith(('.private/', 'runs/', 'artifacts/')) for s in p['write_scope']):
            raise ValueError(f'{p["id"]}: public write scope includes private output')
        state = read_json(root / PLAN / 'progress' / f'{p["id"]}.json')
        if set(state) != {'schema_version', 'packet', 'tasks'} or state['schema_version'] != 1 or state['packet'] != p['id']:
            raise ValueError(f'{p["id"]}: invalid progress schema')
        ids = [t['id'] for t in p['tasks']]
        if set(ids) != set(state['tasks']):
            raise ValueError(f'{p["id"]}: progress task mismatch')
        for task in p['tasks']:
            if not re.fullmatch(p['id'] + r'-\d{2}', task['id']) or any(not task.get(k) for k in ('title', 'instructions', 'acceptance', 'negative_control')):
                raise ValueError('incomplete task')
            s = state['tasks'][task['id']]
            if set(s) != {'status', 'owner', 'note', 'evidence'} or s['status'] not in STATES:
                raise ValueError(f'{task["id"]}: invalid state')
            if s['owner'] is not None and not isinstance(s['owner'], str):
                raise ValueError('owner must be text or null')
            if not isinstance(s['note'], str) or not isinstance(s['evidence'], list):
                raise ValueError('invalid note/evidence')
            if s['status'] in ('in_progress', 'review', 'done') and not s['owner']:
                raise ValueError(f'{task["id"]}: active/completed task needs an owner')
            if s['status'] == 'blocked' and not s['note'].strip():
                raise ValueError(f'{task["id"]}: blocker needs a reason')
            if s['status'] == 'done' and not s['evidence']:
                raise ValueError(f'{task["id"]}: done needs a completion receipt')
            for ref in s['evidence']:
                validate_receipt(root, ref, task['id'])
            all_tasks.append(task['id'])
        progress[p['id']] = state
    if len(set(all_tasks)) != len(all_tasks) or len(all_tasks) != 160:
        raise ValueError('expected 160 unique tasks')
    visiting, visited = set(), set()

    def walk(pid):
        if pid in visiting:
            raise ValueError(f'dependency cycle: {pid}')
        if pid in visited:
            return
        visiting.add(pid)
        for dep in by_id[pid]['dependencies']:
            walk(dep)
        visiting.remove(pid)
        visited.add(pid)
    for pid in by_id:
        walk(pid)
    for p in packets:
        prior_done = True
        deps_done = all(all(v['status'] == 'done' for v in progress[d]['tasks'].values()) for d in p['dependencies'])
        for task in p['tasks']:
            status = progress[p['id']]['tasks'][task['id']]['status']
            if status in ('in_progress', 'review', 'done') and not (prior_done and deps_done):
                raise ValueError(f'{task["id"]}: dependencies or earlier ordered task incomplete')
            prior_done = prior_done and status == 'done'
    manifest = read_json(root / PLAN / 'SOURCE_MANIFEST.json')
    if manifest['source_count'] != 10 or len(manifest['sources']) != 10:
        raise ValueError('expected ten source files')
    if any(not SHA.fullmatch(s['sha256']) for s in manifest['sources']):
        raise ValueError('invalid source digest')
    catalogue = read_json(root / 'docs/TEST_CATALOG.json')['families']
    mapping = read_json(root / PLAN / 'CATALOGUE_MAP.json')['families']
    if len(mapping) != len(catalogue) or {f['id'] for f in mapping} != {f['id'] for f in catalogue}:
        raise ValueError('catalogue coverage mismatch')
    for f in mapping:
        if not f['recipe'] or f['packet'] not in by_id:
            raise ValueError('invalid catalogue mapping')
    defects = read_json(root / PLAN / 'DEFECT_MAP.json')['defects']
    if {d['id'] for d in defects} != {f'D-{i:02d}' for i in range(1, 32)}:
        raise ValueError('defect coverage mismatch')
    if any(d['primary_task'] not in all_tasks for d in defects):
        raise ValueError('unknown defect task')
    enriched = copy.deepcopy(data)
    defect_ids = {}
    for item in defects:
        defect_ids.setdefault(item['primary_task'], []).append(item['id'])
    for p in enriched['packets']:
        for t in p['tasks']:
            t.update(progress[p['id']]['tasks'][t['id']])
            t['defect_ids'] = defect_ids.get(t['id'], [])
    enriched['snapshot_sha256'] = digest(json.dumps(enriched, sort_keys=True, ensure_ascii=False).encode())
    return enriched


def generated(root=ROOT):
    data = load(root)
    paths = {}
    done_count = sum(t['status'] == 'done' for p in data['packets'] for t in p['tasks'])
    lines = ['# Joil v5 implementation ToDos', '',
             'Generated from `TASKS.json` and packet-owned `progress/*.json`. Run `rtk proxy python3 tools/v5_plan.py build` after a progress commit.', '',
             f'**Planning is prepared; engineering implementation progress is {done_count}/160. Scientific qualification remains governed by `docs/STATUS.json`.**', '',
             '| Packet | Milestone | Owner role | Complete | Dependencies |', '| --- | --- | --- | ---: | --- |']
    for p in data['packets']:
        complete = sum(t['status'] == 'done' for t in p['tasks'])
        lines.append(f'| [{p["id"]} — {p["title"]}](todos/{p["id"]}.md) | {p["milestone"]} | {p["owner_role"]} | {complete}/{len(p["tasks"])} | {", ".join(p["dependencies"]) or "none"} |')
        body = [f'# {p["id"]} — {p["title"]}', '', f'Milestone: {p["milestone"]}. Parent packets: {", ".join(p["legacy_packets"])}. Owner role: {p["owner_role"]}.', '',
                f'Dependencies: {", ".join(p["dependencies"]) or "none"}. Completion means accepted packet evidence; scientific admission is separate.', '',
                '## Objective and boundary', '', p['title'] + '. Execute the five ordered tasks below. Read other packets as dependencies; write only the listed scope. Root manifests, schemas, requirements and generated clients require the integrator.', '',
                '## Ordered read-first paths', '']
        body += [f'{i}. `{name}`' for i, name in enumerate(p['read_first'], 1)]
        body += ['', 'Read the global ordered reads in `../ENGINEERING_WORKPLAN.md` first. New paths are planned contracts: wait for their dependency commit before implementation.', '', '## Exclusive write scope', '']
        body += [f'- `{name}`' for name in p['write_scope']]
        body += ['', 'Forbidden: another project, unowned shared files, frozen raw inputs/receipts and private artifacts in Git. Generated views are rebuilt by the integrator. Concurrent writes to parent/child scopes require an explicit path split.', '', '## Ordered tasks', '']
        for t in p['tasks']:
            body += [f'### {"[x]" if t["status"] == "done" else "[ ]"} {t["id"]} — {t["title"]}', '',
                     f'Status: **{t["status"]}**. Assigned owner: {t["owner"] or "unassigned"}.', '', t['instructions'], '',
                     f'Acceptance: {t["acceptance"]}', '', f'Negative control: {t["negative_control"]}', '',
                     f'Progress note: {t["note"] or "None; work has not started."}', '']
            if t['evidence']:
                body += [f'Evidence: `{r["path"]}` SHA-256 `{r["sha256"]}`.' for r in t['evidence']] + ['']
        body += ['## Hard parts and edge cases', ''] + [f'- {v.strip()}.' for v in p['edge_cases']]
        body += ['', '## Validation commands and expected artifacts', '',
                 'Commands below are future packet acceptance commands. The target crate, test or CLI subcommand must be implemented and registered before use. Run from `/home/ubuntu/joil` in the assigned branch/checkout, with a current explicit resource lease for native/large tests.', '', '```sh']
        body += p['validation_commands'] + ['```', '',
                 'Expected artifacts: tested source commit; source-bound independent positive and negative fixture results; exact command arrays/exit codes; raw-log and result SHA-256; packet completion receipt; explicit scientific eligibility and remaining limits. Raw outputs stay private. A command failure blocks this packet; missing physical inputs block their dependent claims, not unrelated software work.', '',
                 '## Stop and handoff', '',
                 'Stop on ownership conflict, incompatible identity/units, missing mandatory evidence, failed numerical/model budget, stale lease or resource pressure. Preserve diagnostics; do not relax a threshold or replace a failed subject. Append material progress to root COORD.md and this packet’s progress record.', '',
                 f'Update `../progress/{p["id"]}.json` through `tools/v5_plan.py update`. A completed task requires a hashed receipt with tested commit, changed paths, exact commands, log/artifact digests, positive/negative checks, resources, limitations and next dependency. `done` is task completion and never scientific PASS. Integrator regenerates ToDos/dashboard and accepts the packet before downstream work.', '']
        paths[PLAN / 'todos' / f'{p["id"]}.md'] = '\n'.join(body)
    lines += ['', '## Progress commands', '', '```sh',
              'rtk proxy python3 tools/v5_plan.py update V5-00-01 --status in_progress --owner NAME --note "Baseline review started"',
              'rtk proxy python3 tools/v5_plan.py update V5-00-01 --status done --owner NAME --receipt docs/v5-plan/receipts/UNIQUE.json',
              'rtk proxy python3 tools/v5_plan.py build', 'rtk proxy python3 tools/v5_plan.py check', '```', '',
              'Only use `done` after the receipt is real and reviewed. Browser draft marks and notes stay local and never update this progress source.', '']
    paths[PLAN / 'TODO.md'] = '\n'.join(lines)
    template = (root / 'tools/v5_dashboard.html').read_text()
    encoded = json.dumps(data, ensure_ascii=False, separators=(',', ':')).replace('<', '\\u003c').replace('>', '\\u003e').replace('&', '\\u0026')
    if template.count('__PLAN_DATA__') != 1:
        raise ValueError('dashboard template needs exactly one data marker')
    paths[PLAN / 'dashboard.html'] = template.replace('__PLAN_DATA__', encoded)
    return data, paths


def build(root=ROOT, check=False):
    data, files = generated(root)
    stale = []
    for name, content in files.items():
        path = root / name
        if check:
            if not path.is_file() or path.read_text() != content:
                stale.append(str(name))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
    if stale:
        raise ValueError('stale generated views: ' + ', '.join(stale))
    return data


def update(root, task_id, status, owner, note, receipt):
    data = load(root)
    packet = next((p for p in data['packets'] if any(t['id'] == task_id for t in p['tasks'])), None)
    if packet is None:
        raise ValueError(f'unknown task: {task_id}')
    path = root / PLAN / 'progress' / f'{packet["id"]}.json'
    old = path.read_bytes()
    state = read_json(path)
    row = state['tasks'][task_id]
    row['status'] = status
    if owner is not None:
        row['owner'] = owner
    if note is not None:
        row['note'] = note
    if receipt:
        refpath = inside(root, receipt)
        ref = {'path': str(refpath.relative_to(root.resolve())), 'sha256': digest(refpath.read_bytes())}
        validate_receipt(root, ref, task_id)
        row['evidence'] = [ref]
    path.write_text(json.dumps(state, indent=2, ensure_ascii=False) + '\n')
    try:
        load(root)
    except Exception:
        path.write_bytes(old)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('build')
    sub.add_parser('check')
    ap = sub.add_parser('update')
    ap.add_argument('task')
    ap.add_argument('--status', required=True, choices=STATES)
    ap.add_argument('--owner')
    ap.add_argument('--note')
    ap.add_argument('--receipt')
    args = parser.parse_args()
    try:
        if args.command == 'update':
            update(ROOT, args.task, args.status, args.owner, args.note, args.receipt)
            print('Progress source updated; integrator must run build and review the generated diff.')
            return 0
        data = build(check=args.command == 'check')
        done = sum(t['status'] == 'done' for p in data['packets'] for t in p['tasks'])
        print(f'PASS: 32 packets, 160 tasks, {done} completed; 10 sources, 31 defects, catalogue coverage; views {args.command} OK')
        return 0
    except (ValueError, OSError, KeyError, TypeError) as exc:
        print(f'FAIL: {exc}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
