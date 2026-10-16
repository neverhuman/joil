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


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--cad',action='store_true',help='also run baseline CAD with project .venv')
    ap.add_argument('--skip-ui',action='store_true',help='record UI as unexecuted')
    args=ap.parse_args()
    runid=datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]
    out=ROOT/'.private/checks'/runid;out.mkdir(parents=True)
    (ROOT/'.private').chmod(0o700)
    env=os.environ.copy()
    env['PATH']=os.pathsep.join([str(ROOT/'.private/node-runtime/bin'),str(ROOT/'.private/node-runtime/node_modules/.bin'),str(Path.home()/'joil/.private/node-runtime/bin'),str(Path.home()/'.cargo/bin'),env['PATH']])
    env.update(CARGO_BUILD_JOBS='4',OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',RAYON_NUM_THREADS='4')
    steps=[('fmt',['cargo','fmt','--all','--','--check'],ROOT),
           ('clippy',['cargo','clippy','--workspace','--locked','--all-targets','--','-D','warnings'],ROOT),
           ('rust-tests',['cargo','test','--workspace','--locked'],ROOT),
           ('baseline',['cargo','run','--locked','-p','joil-cli','--','baseline'],ROOT),
           ('python-tests',['python3','-m','unittest','discover','-s','tests','-v'],ROOT),
           ('public-check',['python3','tools/check_public.py'],ROOT),
           ('claims-audit',['python3','tools/claims_audit.py'],ROOT)]
    if not args.skip_ui:
        steps += [('npm-ci',['npm','ci','--no-audit','--no-fund'],ROOT/'apps/explorer'),
                  ('ui-build',['npm','run','build'],ROOT/'apps/explorer')]
    if args.cad:
        steps += [('cad',[str(ROOT/'.venv/bin/python'),'cad/ct35/build.py','--out',str(ROOT/'artifacts'/runid/'ct35')],ROOT)]
    receipt={'schema_version':1,'run_id':runid,'kind':'foundation_checks_not_physical_validation',
             'steps':[],'ui_executed':not args.skip_ui,'cad_executed':args.cad,'cfd_executed':False}
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
