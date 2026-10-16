#!/usr/bin/env python3
"""Run foundation checks and archive exact logs/receipts in ignored private storage."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import uuid

ROOT=Path(__file__).resolve().parents[1]
SUITES=('fmt','clippy','rust-tests','baseline','python-tests','public-check','claims-audit','ui')


def check_steps(root, *, suite=None, skip_ui=False, cad=False, runid='local'):
    """Select independent CI suites; preserve the complete local check by default."""
    if suite not in (None, *SUITES):
        raise ValueError(f'unknown check suite: {suite}')
    if suite and (skip_ui or cad):
        raise ValueError('--suite cannot be combined with --skip-ui or --cad')
    steps=[('fmt',['cargo','fmt','--all','--','--check'],root),
           ('clippy',['cargo','clippy','--workspace','--locked','--all-targets','--','-D','warnings'],root),
           ('rust-tests',['cargo','test','--workspace','--locked'],root),
           ('baseline',['cargo','run','--locked','-p','joil-cli','--','baseline'],root),
           ('python-tests',['python3','-m','unittest','discover','-s','tests','-v'],root),
           ('public-check',['python3','tools/check_public.py'],root),
           ('claims-audit',['python3','tools/claims_audit.py'],root)]
    if not skip_ui:
        steps += [('npm-ci',['npm','ci','--no-audit','--no-fund'],root/'apps/explorer'),
                  ('ui-build',['npm','run','build'],root/'apps/explorer')]
    if cad:
        steps += [('cad',[str(root/'.venv/bin/python'),'cad/ct35/build.py','--out',str(root/'artifacts'/runid/'ct35')],root)]
    if suite:
        names={'npm-ci','ui-build'} if suite=='ui' else {suite}
        steps=[step for step in steps if step[0] in names]
    return steps


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--cad',action='store_true',help='also run baseline CAD with project .venv')
    ap.add_argument('--skip-ui',action='store_true',help='record UI as unexecuted')
    ap.add_argument('--suite',choices=SUITES,help='run one independent CI suite (ui includes npm ci)')
    args=ap.parse_args()
    if args.suite and (args.skip_ui or args.cad):
        ap.error('--suite cannot be combined with --skip-ui or --cad')
    runid=datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]
    out=ROOT/'.private/checks'/runid;out.mkdir(parents=True)
    (ROOT/'.private').chmod(0o700)
    env=os.environ.copy()
    env['PATH']=os.pathsep.join([str(ROOT/'.private/node-runtime/bin'),str(ROOT/'.private/node-runtime/node_modules/.bin'),str(Path.home()/'joil/.private/node-runtime/bin'),str(Path.home()/'.cargo/bin'),env['PATH']])
    env.update(CARGO_BUILD_JOBS='4',OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',RAYON_NUM_THREADS='4')
    steps=check_steps(ROOT,suite=args.suite,skip_ui=args.skip_ui,cad=args.cad,runid=runid)
    receipt={'schema_version':1,'run_id':runid,'kind':'foundation_checks_not_physical_validation',
             'steps':[],'requested_suite':args.suite or 'all',
             'ui_executed':any(name=='ui-build' for name,_,_ in steps),
             'cad_executed':args.cad,'cfd_executed':False}
    revision=subprocess.run(['git','rev-parse','HEAD'],cwd=ROOT,capture_output=True,text=True)
    receipt['input_head']=revision.stdout.strip() if revision.returncode==0 else None
    names=subprocess.check_output(['git','ls-files','-z','--cached','--others','--exclude-standard'],cwd=ROOT).split(b'\0')
    receipt['source_files_sha256']={}
    for raw in names:
        if not raw:continue
        name=raw.decode();p=ROOT/name
        if p.is_file() and not name.startswith('docs/receipts/'):
            receipt['source_files_sha256'][name]=hashlib.sha256(p.read_bytes()).hexdigest()
    receipt['environment']={k:env[k] for k in ['CARGO_BUILD_JOBS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','RAYON_NUM_THREADS']}
    for name,cmd,cwd in steps:
        start=time.perf_counter()
        try:
            result=subprocess.run(cmd,cwd=cwd,env=env,capture_output=True,timeout=600)
            data=result.stdout+result.stderr;code=result.returncode
        except (OSError,subprocess.TimeoutExpired) as e:
            data=str(e).encode();code=124
        (out/f'{name}.log').write_bytes(data)
        receipt['steps'].append({'name':name,'command':cmd,'cwd':str(cwd.relative_to(ROOT)) or '.',
            'exit_code':code,'wall_seconds':time.perf_counter()-start,'log_bytes':len(data),
            'log_sha256':hashlib.sha256(data).hexdigest()})
        print(f'{name}: exit {code}, {receipt["steps"][-1]["wall_seconds"]:.3f}s',flush=True)
        if code:
            print(data.decode(errors='replace')[-5000:]);break
    receipt['all_requested_pass']=len(receipt['steps'])==len(steps) and all(x['exit_code']==0 for x in receipt['steps'])
    (out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print('receipt:',out/'receipt.json',flush=True)
    return 0 if receipt['all_requested_pass'] else 1


if __name__=='__main__':raise SystemExit(main())
