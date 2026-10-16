#!/usr/bin/env python3
"""Generate conservative public status from the byte-pinned evidence inventory.

This importer admits software check receipts only. Scientific observation admission
requires the trusted Rust claim resolver; legacy booleans are retained as history.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from analysis.evidence import scientific_hash, strict_json

REGISTRY = 'crates/joil-evidence/registry/status-registry.json'
UNIT_MAP = {'mg/L': 'mg_per_litre_dispersed_oil', 'J/m^3': 'joule_per_treated_cubic_metre',
            '-': 'dimensionless', 'um': 'micrometre', 'Pa': 'pascal'}


def pinned_json(root, reference):
    path = (root/reference['path']).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError('registry path escapes repository')
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != reference['sha256']:
        raise ValueError(f'stale or changed evidence: {reference["path"]}')
    return strict_json(raw)


def objective_contract(root):
    requirements = tomllib.loads((root/'requirements/joil-hydrocyclone-v1.toml').read_text())
    prereg_path = root/'experiments/wave2-preregistration-v1.json'
    prereg_bytes = prereg_path.read_bytes()
    if hashlib.sha256(prereg_bytes).hexdigest() != requirements['preregistration']['sha256']:
        raise ValueError('preregistration changed without requirement revision')
    baseline = root/'baselines/ct35/genome.json'
    if hashlib.sha256(baseline.read_bytes()).hexdigest() != requirements['baseline']['sha256']:
        raise ValueError('baseline changed without requirement revision')
    prereg = strict_json(prereg_bytes)
    expected = []
    for spec in prereg['objectives_minimize_5d']:
        grade = spec['id'] == 'one_minus_grade_20um_p05'
        expected.append({'id': spec['id'], 'source_id': 'grade_20um_p05' if grade else spec['id'],
                         'sense': spec['sense'], 'unit': UNIT_MAP[spec['units']],
                         'transform': 'one_minus' if grade else 'identity', 'reference_point': None})
    actual = [dict(spec, reference_point=spec.get('reference_point')) for spec in requirements['objectives']]
    if actual != expected:
        raise ValueError('preregistered/executed objective IDs/order/units/sense/transform/reference drift')
    scenarios = requirements['scenarios']
    if [s['id'] for s in scenarios] != [s['duty_id'] for s in prereg['held_out_operating_duties']]:
        raise ValueError('scenario inventory drift')
    if any(s['sha256'] != requirements['preregistration']['sha256'] for s in scenarios):
        raise ValueError('scenario distribution identity drift')
    return requirements, actual, scientific_hash(actual)


def verify_campaign_receipt(receipt, objectives, objective_hash):
    if receipt.get('schema') != 'joil.campaign.v1' or receipt.get('objective_sha256') != objective_hash:
        raise ValueError('campaign objective hash mismatch')
    if receipt.get('executed_objectives') != objectives:
        raise ValueError('campaign executable objective contract mismatch')


def generate(root=ROOT):
    root = Path(root).resolve()
    registry = strict_json((root/REGISTRY).read_bytes())
    if registry['schema'] != 1:
        raise ValueError('unsupported status registry')
    requirements, objectives, objective_hash = objective_contract(root)
    software = []
    for entry in registry['software']:
        receipt = pinned_json(root, entry['receipt'])
        checks = receipt.get('checks', [])
        software.append({'id': entry['id'], 'state': 'software_contract_verified' if checks
                         and all(check.get('exit_code') == 0 for check in checks) else 'incomplete',
                         'receipt': entry['receipt']})
    # No scalar, caller bool, or unreviewed observation may grant a scientific claim.
    if registry['observations']:
        raise ValueError('scientific observations require a trusted Rust resolver receipt; importer not admitted')
    claims = {entry['id']: {'state': 'incomplete', 'missing': entry['required_observations']}
              for entry in registry['scientific_claims']}
    historical = pinned_json(root, registry['historical_status'])
    for path in sorted((root/'runs').glob('*/campaign.json')):
        verify_campaign_receipt(strict_json(path.read_bytes()), objectives, objective_hash)
    return {'schema_version': 2, 'generated_by': 'tools/gen_status.py',
            'evidence_registry_sha256': hashlib.sha256((root/REGISTRY).read_bytes()).hexdigest(),
            'phase': 'software_truth_repair_scientific_claim_inventory_incomplete',
            'campaign_admission': requirements['admission'], 'campaign_blockers': requirements['blockers'],
            'requirements_sha256': hashlib.sha256((root/registry['requirements']).read_bytes()).hexdigest(),
            'objective_sha256': objective_hash, 'software': software, 'claims': claims,
            'qualified_baseline': False, 'structural_qualification': False, 'physical_validation': False,
            'oil_grade_evaluated': False, 'qualified_fitness': None, 'qualified_capacity_gain': None,
            'customer_or_site_data_public': False,
            'legacy_status_is_historical_not_authoritative': True, 'legacy_status': historical}


def check_generated(root=ROOT):
    root = Path(root)
    expected = json.dumps(generate(root), indent=2, allow_nan=False)+'\n'
    if (root/'docs/STATUS.json').read_text() != expected:
        raise ValueError('docs/STATUS.json is stale or hand-edited; run python3 tools/gen_status.py')
    return expected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    try:
        if args.check:
            check_generated()
        else:
            (ROOT/'docs/STATUS.json').write_text(json.dumps(generate(), indent=2, allow_nan=False)+'\n')
    except (ValueError, KeyError, OSError) as error:
        print(f'FAIL: {error}', file=sys.stderr)
        return 1
    print('PASS: generated status and preregistered objective identities agree')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
