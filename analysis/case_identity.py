"""Wave-4 exact artifact chain. Capture after meshing, before inputs are pruned.

This scoped OpenFOAM artifact receipt is not physical qualification or a substitute
for the full ScientificIdentity required by a claim-bearing campaign.
"""

import argparse
import hashlib
import json
from pathlib import Path
import re

from analysis.evidence import save_new, scientific_hash, strict_json

SCHEMA = 'joil.wave4-case-identity.v1'
MANIFEST = 'case-identity.json'


def _safe(case, relative):
    path = case / relative
    if Path(relative).is_absolute() or '..' in Path(relative).parts:
        raise ValueError('case artifact path escapes case')
    if not path.resolve().is_relative_to(case.resolve()):
        raise ValueError('case artifact path escapes case')
    if any(p.is_symlink() for p in [path, *path.parents] if p != case.parent):
        raise ValueError('symlink is not an exact case artifact')
    return path


def _digest(path):
    h = hashlib.sha256()
    size = 0
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            size += len(chunk)
            h.update(chunk)
    return {'sha256': h.hexdigest(), 'bytes': size}


def _inventory(case, roots, exclude=()):
    found = {}
    for root in roots:
        directory = _safe(case, root)
        if not directory.is_dir():
            raise ValueError(f'missing artifact directory: {root}')
        for path in sorted(directory.rglob('*')):
            rel = path.relative_to(case).as_posix()
            if any(rel == x or rel.startswith(x + '/') for x in exclude):
                continue
            _safe(case, rel)
            if path.is_file():
                found[rel] = _digest(path)
    if not found:
        raise ValueError('empty artifact inventory')
    return found


def _hash_id(value):
    if not isinstance(value, str) or re.fullmatch('[0-9a-f]{64}', value) is None:
        raise ValueError('missing or invalid canonical source hash')
    return value


def build_case_identity(case):
    """Read actual retained input bytes; missing stages cannot be finalized."""
    case = Path(case)
    prepared_path = _safe(case, 'input-manifest.json')
    prepared = strict_json(prepared_path.read_bytes())
    bundle = prepared.get('subject_bundle')
    if not isinstance(bundle, dict):
        raise ValueError('prepared compiler subject bundle missing')
    artifacts = {name: _digest(_safe(case, name))
                 for name in ['design-program.json', 'design-ir.json']}
    surface = prepared['surface_stl_relpath']
    artifacts[surface] = _digest(_safe(case, surface))
    for key, name in [('genome', 'design-program.json'), ('ir', 'design-ir.json'),
                      ('surface_mesh', surface)]:
        if bundle[key]['sha256'] != artifacts[name]['sha256']:
            raise ValueError(f'prepared {key} digest differs from actual artifact')
    if prepared['surface_stl_sha256'] != artifacts[surface]['sha256']:
        raise ValueError('prepared surface digest mismatch')
    canonical_genome = _hash_id(prepared['design_sha256'])
    canonical_ir = _hash_id(prepared['design_ir_sha256'])
    ir = strict_json(_safe(case, 'design-ir.json').read_bytes())
    if ir['ir_sha256'] != canonical_ir or ir['program_sha256'] != canonical_genome:
        raise ValueError('IR and prepared manifest disagree on canonical subjects')
    mesh = _inventory(case, ['constant/polyMesh'])
    for name in ['points', 'faces', 'owner', 'neighbour', 'boundary']:
        if not any('constant/polyMesh/' + name + suffix in mesh for suffix in ['', '.gz']):
            raise ValueError(f'missing executed volume mesh component: {name}')
    for name in ['0/U', '0/p', 'system/controlDict', 'system/fvSchemes', 'system/fvSolution']:
        if not _safe(case, name).is_file():
            raise ValueError(f'missing solver input: {name}')
    inputs = _inventory(case, ['0', 'constant', 'system'], exclude=['constant/polyMesh'])
    mesh_hash = scientific_hash({'contract': SCHEMA, 'stage': 'volume_mesh', 'files': mesh})
    chain = {
        'genome_hash': artifacts['design-program.json']['sha256'],
        'ir_hash': artifacts['design-ir.json']['sha256'],
        'geometry_hash': artifacts[surface]['sha256'],
        'mesh_hash': mesh_hash,
    }
    compiler = {key: bundle[key] for key in ['compiler', 'geometry_kernel', 'compiler_config']}
    chain['case_hash'] = scientific_hash({
        'contract': SCHEMA, 'stage': 'solver_case', 'upstream': chain,
        'compiler': compiler, 'files': inputs,
    })
    return {
        'schema': SCHEMA, 'kind': 'openfoam_artifact_identity_not_qualification',
        'hash_basis': 'genome/IR/geometry exact file bytes; mesh/case domain-separated inventories',
        'canonical_genome_hash': canonical_genome, 'canonical_ir_hash': canonical_ir,
        **chain, 'prepared_manifest': _digest(prepared_path),
        'compiler': compiler, 'source_artifacts': artifacts,
        'mesh_files': mesh, 'solver_input_files': inputs,
    }


def finalize_case_identity(case):
    """Immutable writer API for INT: call while mesh and solver inputs exist."""
    case = Path(case)
    record = build_case_identity(case)
    path = case / MANIFEST
    if path.exists():
        if strict_json(path.read_bytes()) != record:
            raise ValueError('existing case identity is stale; never overwrite it')
    else:
        save_new(path, record)
    return record


def verify_case_identity(case):
    case = Path(case)
    recorded = strict_json(_safe(case, MANIFEST).read_bytes())
    actual = build_case_identity(case)
    if recorded != actual:
        raise ValueError('case identity differs from retained artifacts')
    return {'case': case.name, 'case_hash': actual['case_hash'],
            'mesh_hash': actual['mesh_hash'], 'state': 'exact_artifact_bytes_verified'}


def audit_wave4_manifests(runs_root, require_cases=True):
    """Read every G0/G1/G2 Wave-4 case, including RSM cases named with g1."""
    cases = sorted(p for p in Path(runs_root).glob('wave4-*-g[012]-*') if p.is_dir())
    verified, errors = [], []
    for case in cases:
        try:
            verified.append(verify_case_identity(case))
        except (ValueError, KeyError, OSError) as exc:
            errors.append({'case': case.name, 'reason': str(exc)})
    if require_cases and not cases:
        errors.append({'case': None, 'reason': 'no Wave-4 cases available; acceptance not verified'})
    return {'passed': not errors, 'manifest_count': len(cases),
            'verified': verified, 'errors': errors}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runs-root', type=Path, default=Path('runs'))
    args = parser.parse_args()
    result = audit_wave4_manifests(args.runs_root)
    print(json.dumps(result, indent=2))
    raise SystemExit(not result['passed'])
