"""Restricted, project-scoped OpenFOAM invocation. Never uses a mutable image tag."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import uuid
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[3]
LOCK = Path(__file__).with_name('lock.json')


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def cpu_allocation(value=None):
    value = value or os.environ.get('JOIL_CPU_SET','0,1,2,3')
    cpus = [int(x.strip()) for x in value.split(',')]
    if len(cpus)!=4 or len(set(cpus))!=4:
        raise ValueError('pilot runner requires four distinct explicitly allocated CPUs')
    if cpus == [0,1,2,3]:
        return cpus, 'initial_project_allocation'
    lease_path=ROOT/'.private/coordination/resource-lease.json'
    lease=json.loads(lease_path.read_text())
    if datetime.fromisoformat(lease['expires_utc']) <= datetime.now(timezone.utc):
        raise ValueError('resource lease expired')
    if not any(x['cpus']==cpus for x in lease['leases']):
        raise ValueError('CPU set is not allocated in the coordination lease')
    return cpus, digest(lease_path)


def output_bytes():
    total=0
    for folder in ['runs','artifacts']:
        for directory, _, files in os.walk(ROOT/folder):
            for name in files:
                p=Path(directory)/name
                try:
                    if not p.is_symlink():total+=p.stat().st_size
                except FileNotFoundError:pass
    return total


def run(case, argv, log, timeout=3600):
    case = Path(case).resolve()
    if not case.is_relative_to(ROOT / 'runs'):
        raise ValueError('only project runs may be writable solver cases')
    lock = json.loads(LOCK.read_text())
    cpus, lease_id = cpu_allocation()
    source_sha256=digest(__file__)
    image = lock['image_id']
    if not image.startswith('sha256:') or len(image) != 71:
        raise ValueError('runtime must be bound to an immutable local image ID')
    container_name = 'joil-' + uuid.uuid4().hex[:16]
    cmd = ['docker', 'run', '--name', container_name, '--rm', '--init', '--network=none', '--read-only',
           '--cpus=4', '--cpuset-cpus='+','.join(map(str,cpus)), '--memory=12g', '--memory-swap=12g',
           '--pids-limit=256', '--cap-drop=ALL', '--security-opt=no-new-privileges',
           '--user', f'{os.getuid()}:{os.getgid()}',
           '--tmpfs', '/tmp:rw,nosuid,nodev,size=256m,mode=1777',
           '-e', 'HOME=/tmp', '-e', 'OMP_NUM_THREADS=1',
           '-e', 'OPENBLAS_NUM_THREADS=1',
           '-v', f'{ROOT}:/work:ro', '-v', f'{case}:/case:rw', '-w', '/case',
           image, *argv]
    start = time.perf_counter()
    peak_output=output_bytes()
    if peak_output>5*1024**3:raise ValueError('shared 5 GiB output allocation exhausted')
    log = Path(log)
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open('wb') as stream:
        process=subprocess.Popen(cmd, stdout=stream, stderr=subprocess.STDOUT)
        stop_reason=None
        next_output_check=start+10
        while process.poll() is None:
            now=time.perf_counter()
            if now>=next_output_check:
                peak_output=max(peak_output,output_bytes());next_output_check=now+10
                if peak_output>5*1024**3:stop_reason='shared_output_allocation_exhausted'
            if now-start>timeout:stop_reason='timeout'
            if stop_reason:
                # Killing the Docker client alone can leave its container running.
                subprocess.run(['docker','stop','-t','5',container_name], capture_output=True, timeout=20)
                try:process.wait(timeout=10)
                except subprocess.TimeoutExpired:process.kill();process.wait()
                break
            time.sleep(.25)
        code=124 if stop_reason else process.returncode
    receipt = {'command': argv, 'image_id': image, 'exit_code': code,
               'wall_seconds': time.perf_counter() - start, 'log_sha256': digest(log),
               'log_bytes': log.stat().st_size,'stop_reason':stop_reason,
               'driver_source_sha256':source_sha256,'resource_lease_id':lease_id,
               'max_observed_shared_output_bytes':peak_output,
               'allocation': {'logical_cpus': cpus, 'quota_cores': 4,
                              'memory_gib': 12, 'gpu': False}}
    log.with_suffix(log.suffix + '.receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(f'{log.name}: exit {code}, {receipt["wall_seconds"]:.2f}s', flush=True)
    if code:
        raise RuntimeError(log.read_text(errors='replace')[-5000:])
    return receipt
