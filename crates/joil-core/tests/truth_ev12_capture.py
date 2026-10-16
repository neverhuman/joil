"""Retain finalized EV-12 case inputs before pruning; never run or delete a case.

Requires analysis.case_identity from EV12-WAVE4-HASH-CORRECTION (cfdbf7d).
The case owner must finalize the identity after all mesh/input edits. This helper
only reads that case and copies its indexed bytes into a new capture directory.
It excludes solver outputs and does not establish convergence or validation.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import sys

sys.path.append(str(Path(__file__).resolve().parents[3]))


def digest(path):
    checksum = hashlib.sha256()
    size = 0
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            checksum.update(chunk)
            size += len(chunk)
    return {'sha256': checksum.hexdigest(), 'bytes': size}


def capture_case(case, capture):
    from analysis.case_identity import MANIFEST, verify_case_identity
    from analysis.evidence import save_new, strict_json

    case, capture = Path(case).resolve(), Path(capture).resolve()
    if capture.is_relative_to(case):
        raise ValueError('capture must be outside the source case')
    if capture.exists():
        raise FileExistsError('capture directory already exists; never overwrite')
    identity_bytes = (case / MANIFEST).read_bytes()
    verify_case_identity(case)
    identity = strict_json(identity_bytes)
    files = {'input-manifest.json': identity['prepared_manifest'],
             MANIFEST: {'sha256': hashlib.sha256(identity_bytes).hexdigest(),
                        'bytes': len(identity_bytes)}}
    for inventory in ['source_artifacts', 'mesh_files', 'solver_input_files']:
        for name, expected in identity[inventory].items():
            if name in files and files[name] != expected:
                raise ValueError(f'conflicting inventory entry: {name}')
            files[name] = expected
    for name in files:
        if Path(name).is_absolute() or '..' in Path(name).parts:
            raise ValueError('capture path escapes case')

    # A failed copy keeps partial evidence but never emits a completion receipt.
    capture.mkdir(parents=True, exist_ok=False)
    for name, expected in sorted(files.items()):
        source, target = case / name, capture / name
        if source.is_symlink() or not source.resolve().is_relative_to(case):
            raise ValueError('capture source is not an exact case artifact')
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        if digest(target) != expected:
            raise ValueError(f'capture bytes differ from finalized identity: {name}')
    verify_case_identity(capture)
    verify_case_identity(case)
    if (case / MANIFEST).read_bytes() != identity_bytes:
        raise ValueError('source identity changed during capture')
    receipt = {
        'schema': 'joil.truth-ev12-capture.v1',
        'state': 'exact_indexed_inputs_captured_not_validation',
        'captured_utc': datetime.now(timezone.utc).isoformat(),
        'source_case_name': case.name,
        'identity_manifest': files[MANIFEST],
        'chain': {key: identity[key] for key in
                  ['genome_hash', 'ir_hash', 'geometry_hash', 'mesh_hash', 'case_hash']},
        'files': files,
        'file_count': len(files),
        'total_bytes': sum(entry['bytes'] for entry in files.values()),
        'limitations': 'Input retention only; solver outputs/logs and scientific acceptance need separate evidence.',
    }
    save_new(capture / 'tr-ev12-capture-receipt.json', receipt)
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--case', required=True, type=Path)
    parser.add_argument('--capture', required=True, type=Path)
    args = parser.parse_args()
    try:
        receipt = capture_case(args.case, args.capture)
    except (ImportError, OSError, ValueError, KeyError) as error:
        print(f'capture refused: {error}', file=sys.stderr)
        return 1
    print(json.dumps(receipt, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
