#!/usr/bin/env python3
"""Independent Linux control: architecture/interchange tests with rtk absent.
Run at the repository root; this creates no working copy or installation.
"""
import os,subprocess,shutil,json
from pathlib import Path
sysroot=Path(subprocess.check_output([os.environ.get("RUSTC", "rustc"),"--print","sysroot"],text=True).strip())
env=os.environ.copy();env["PATH"]=str(sysroot/"bin")+":/usr/bin:/bin"
assert shutil.which("rtk",path=env["PATH"]) is None,"control PATH unexpectedly has rtk"
print(json.dumps({"negative_control":"rtk is absent from child PATH","old_wrapper_would_not_start":True}),flush=True)
commands=[[str(sysroot/"bin/cargo"),"test","--locked","--offline","-p","crucible-architecture","--test","blind_boundary"],[str(sysroot/"bin/cargo"),"test","--locked","--offline","-p","crucible-contract","--test","architecture","--test","interchange"]]
for command in commands:
 result=subprocess.run(command,env=env)
 print(json.dumps({"argv":command,"exit_code":result.returncode}),flush=True)
 if result.returncode:raise SystemExit(result.returncode)
