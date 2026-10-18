#!/usr/bin/env python3
"""Record a bounded fresh-clone witness for the shipped synthetic fixture."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checkout', type=Path, required=True)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--expected-commit', required=True)
    args = parser.parse_args()
    checkout = args.checkout.resolve(strict=True)
    evidence = args.evidence.resolve()
    target = Path(os.environ['CARGO_TARGET_DIR']).resolve(strict=True)
    if '.private' not in evidence.parts or '.private' not in target.parts:
        raise ValueError('evidence and existing per-worker target must be private')
    evidence.mkdir(mode=0o700, parents=True, exist_ok=False)
    commands = []

    def call(argv, expected=0, timeout=240):
        start = time.monotonic()
        result = subprocess.run(argv, cwd=checkout, capture_output=True, timeout=timeout)
        number = len(commands)
        stdout, stderr = evidence / f'{number:02d}.stdout', evidence / f'{number:02d}.stderr'
        stdout.write_bytes(result.stdout)
        stderr.write_bytes(result.stderr)
        commands.append(dict(argv=argv, exit_code=result.returncode,
                             expected_exit_code=expected, wall_seconds=time.monotonic() - start,
                             stdout_sha256=sha(result.stdout), stderr_sha256=sha(result.stderr)))
        (evidence / 'commands.json').write_text(json.dumps(commands, indent=2) + '\n')
        if result.returncode != expected:
            raise ValueError(f'command {number} exit {result.returncode}: '
                             f'{result.stderr.decode(errors="replace")[-3000:]}')
        return result.stdout

    def git(*tail):
        return call(['git', *tail]).decode().strip()

    commit = git('rev-parse', 'HEAD')
    if commit != args.expected_commit or git('status', '--porcelain', '--untracked-files=all'):
        raise ValueError('fresh clone must start at the exact clean accepted source')
    common = Path(git('rev-parse', '--git-common-dir'))
    if not common.is_absolute():
        common = (checkout / common).resolve()
    if (common / 'objects/info/alternates').exists():
        raise ValueError('fresh clone must have its own objects, no alternates')
    origin = git('remote', 'get-url', 'origin')
    if origin not in ('https://github.com/neverhuman/joil.git', 'git@github.com:neverhuman/joil.git'):
        raise ValueError('Joil origin required')
    cores = set()
    for cpu in os.sched_getaffinity(0):
        top = Path(f'/sys/devices/system/cpu/cpu{cpu}/topology')
        cores.add(((top / 'physical_package_id').read_text().strip(),
                   (top / 'core_id').read_text().strip()))
    capability = {'physical_cores_available': len(cores), 'workers_requested': 2,
                  'workers_exercised': 2 if len(cores) >= 2 else 0,
                  'source_commit': commit, 'no_gpu': True}
    (evidence / 'worker-capability.json').write_text(json.dumps(capability, indent=2) + '\n')
    if len(cores) < 2:
        raise ValueError('shipped two-worker fixture requires two leased physical cores; no substitution')
    initial = {'commit': commit, 'tree': git('rev-parse', 'HEAD^{tree}'),
               'common_dir': str(common), 'origin': origin, 'no_object_alternates': True,
               'initial_status_clean': True, 'target_reused': str(target),
               'campaign_sha256': sha((checkout / 'campaigns/fixture-v5.toml').read_bytes())}
    (evidence / 'source.json').write_text(json.dumps(initial, indent=2) + '\n')
    cargo = ['cargo', 'run', '--locked', '-p', 'crucible-cli', '--']
    campaign = 'campaigns/fixture-v5.toml'

    def cli(*tail, expected=0):
        raw = call(cargo + list(tail), expected)
        return json.loads(raw) if expected == 0 else raw

    # This first run is the packet command exactly, with the shipped file unchanged.
    whole = cli('run', campaign)
    partial = cli('run', campaign, '--until', '1')
    resumed = cli('run', campaign, '--resume', partial['root'])
    verified = cli('checkpoint', 'verify', partial['root'])
    status = cli('status', partial['root'])
    assert whole['epochs'] == resumed['epochs'] == 2
    assert partial['epochs'] == 1
    assert whole['accounting']['charges'] == resumed['accounting']['charges'] == 4
    assert partial['accounting']['charges'] == 2
    assert not whole['accounting']['exceeded'] and not resumed['accounting']['exceeded']
    assert whole['logical_digest'] == resumed['logical_digest'] == verified['logical_digest']
    assert verified['offline'] is True and verified['physical_eligibility'] == 'Unsupported'
    bad = evidence / 'invalid-physical.toml'
    bad.write_text((checkout / campaign).read_text().replace(
        'synthetic_integer_fixture', 'physical_solver'))
    refused_root = evidence / 'must-not-exist'
    cli('run', str(bad), '--root', str(refused_root), expected=64)
    assert not refused_root.exists()
    assert b'bounded_software_campaign_admission' in (evidence / f'{len(commands)-1:02d}.stderr').read_bytes()
    final_status = git('status', '--porcelain', '--untracked-files=all')
    assert not final_status
    report = {'source': initial, 'worker_capability': capability, 'whole': whole,
              'partial': partial, 'resumed': resumed, 'verified': verified, 'status': status,
              'invalid_physical_input_refused_before_root_creation': True,
              'final_checkout_clean': True, 'physical_qualification': False,
              'limitations': ['Synthetic integer fixture only; existing toolchain/cache reused',
                              'No dependency reinstall or physical solver/GPU execution',
                              'Native attempts use declared runtime accounting scope']}
    (evidence / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    inventory = [{'path': str(p.relative_to(evidence)), 'sha256': sha(p.read_bytes()),
                  'bytes': p.stat().st_size} for p in sorted(evidence.rglob('*')) if p.is_file()]
    (evidence / 'inventory.json').write_text(json.dumps(inventory, indent=2) + '\n')
    print(json.dumps({'source_commit': commit, 'commands': len(commands),
                      'charges_uninterrupted': 4, 'charges_partial': 2, 'charges_resumed': 4,
                      'logical_replay_equal': True, 'offline_verified': True,
                      'invalid_input_refused': True, 'physical_qualification': False,
                      'inventory_sha256': sha((evidence / 'inventory.json').read_bytes())}))


if __name__ == '__main__':
    main()
