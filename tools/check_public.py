#!/usr/bin/env python3
"""Check Git-selected/public candidates; optional private deny rules never enter Git."""
from pathlib import Path
import re
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[1]
FORBIDDEN_PARTS={'.private','.venv','runs','artifacts','node_modules','target','.env'}
SECRET_PATTERNS=[re.compile(rb'-----BEGIN (?:OPENSSH|RSA|EC|DSA) PRIVATE KEY-----'),
                 re.compile(rb'gh[pousr]_[A-Za-z0-9]{30,}'),re.compile(rb'github_pat_[A-Za-z0-9_]{30,}')]


def violations(name,data,deny=()):
    errors=[];p=Path(name)
    if any(part in FORBIDDEN_PARTS for part in p.parts) or p.name.startswith('.env') and p.name!='.env.example':
        errors.append('private/generated path')
    if p.suffix.lower() in {'.zip','.stl','.step','.sif','.blend','.pem','.key'}:
        errors.append('raw/generated or credential file')
    if len(data)>2_000_000:errors.append('large file needs explicit publication review')
    if any(pattern.search(data) for pattern in SECRET_PATTERNS):errors.append('credential pattern')
    if any(term.lower() in data.lower() for term in deny if term):errors.append('private publication deny rule')
    return errors


def main():
    names=subprocess.check_output(['git','ls-files','-z','--cached','--others','--exclude-standard'],cwd=ROOT).split(b'\0')
    rules=ROOT/'.private/publication-deny.txt'
    if (ROOT/'.private/source').exists() and not rules.exists():
        print('FAIL: private source archive exists without publication deny rules');return 1
    deny=[x.strip().encode() for x in rules.read_text().splitlines() if x.strip()] if rules.exists() else []
    errors=[];count=0
    for raw in names:
        if not raw:continue
        name=raw.decode();p=ROOT/name
        if not p.is_file():continue
        count+=1
        for reason in violations(name,p.read_bytes(),deny):errors.append(f'{name}: {reason}')
    if errors:print('\n'.join(errors));return 1
    print(f'PASS: {count} public candidate files checked; private deny rules applied={bool(deny)}')
    return 0


if __name__=='__main__':raise SystemExit(main())
