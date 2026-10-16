"""Exact-byte identities, immutable manifests and conservative research fitness."""
import hashlib
import json
import math
import struct
import re
import unicodedata
from enum import Enum
from pathlib import Path


class EvidenceTier(str, Enum):
    SOFTWARE_CONTRACT = 'software_contract'
    GEOMETRY = 'geometry'
    ANALYTIC_SCREEN = 'analytic_screen'
    COUPLED_SCREEN = 'coupled_screen'
    NUMERICALLY_VERIFIED = 'numerically_verified'
    MODEL_CHALLENGE = 'model_challenge'
    PHYSICAL_VALIDATION = 'physical_validation'


def evidence_reference(identity, revision, data):
    if not identity or not revision or not isinstance(data, bytes):
        raise ValueError('exact bytes, identity and revision required')
    return {'id': identity, 'revision': revision,
            'sha256': hashlib.sha256(data).hexdigest()}


def strict_json(data):
    """Same finite-number, duplicate/NFC-key and payload limits as the Rust wire."""
    if isinstance(data, str):
        data = data.encode('utf-8')
    if len(data) > 1_048_576:
        raise ValueError('payload too large')

    def pairs(items):
        result, normalized = {}, set()
        for key, value in items:
            norm = unicodedata.normalize('NFC', key)
            if norm in normalized:
                raise ValueError('duplicate_or_normalization_collision')
            normalized.add(norm)
            result[key] = value
        return result

    def floating(token):
        value = float(token)
        mantissa = token.lower().split('e')[0]
        if not math.isfinite(value) or (value == 0 and any(c in '123456789' for c in mantissa)):
            raise ValueError('nonfinite or underflow number')
        return value

    def integer(token):
        if token == '-0':
            return -0.0
        value = int(token)
        if not -(2**63) <= value <= 2**64-1:
            raise ValueError('integer out of range')
        return value

    def constant(_):
        raise ValueError('nonfinite number')

    value = json.loads(data, object_pairs_hook=pairs, parse_float=floating,
                       parse_int=integer, parse_constant=constant)
    nodes = 0

    def bounded(item, depth=0):
        nonlocal nodes
        nodes += 1
        if nodes > 100_000 or depth > 64:
            raise ValueError('payload bounds')
        if isinstance(item, str):
            item.encode('utf-8')
        if isinstance(item, dict):
            for key in item:
                key.encode('utf-8')
        children = item.values() if isinstance(item, dict) else item if isinstance(item, list) else []
        for child in children:
            bounded(child, depth+1)
    bounded(value)
    return value


def canonical_wire(value):
    """Jarambit-derived exact numeric wire shared with joil-evidence identity.rs."""
    def encode(item):
        if item is None:
            return b'n'
        if isinstance(item, bool):
            return b't' if item else b'f'
        if isinstance(item, str):
            raw = item.encode('utf-8')
            return f's{len(raw)}:'.encode()+raw
        if isinstance(item, int):
            if not -(2**63) <= item <= 2**64-1:
                raise ValueError('integer out of range')
            return f'{"u" if item >= 0 else "i"}{item};'.encode()
        if isinstance(item, float):
            if not math.isfinite(item):
                raise ValueError('nonfinite number')
            return b'd'+struct.pack('>d', item).hex().encode()+b';'
        if isinstance(item, list):
            return f'a{len(item)}:'.encode()+b''.join(encode(x) for x in item)
        if isinstance(item, dict) and all(isinstance(k, str) for k in item):
            keys = [unicodedata.normalize('NFC', k) for k in item]
            if len(set(keys)) != len(keys):
                raise ValueError('normalization_collision')
            return f'o{len(item)}:'.encode()+b''.join(encode(k)+encode(item[k]) for k in sorted(item))
        raise ValueError('unsupported canonical value')
    return encode(value)


def scientific_hash(value):
    return hashlib.sha256(canonical_wire(value)).hexdigest()


def _schema_validate(value, spec, schema):
    """Validate the frozen schema's deliberately small, explicit JSON Schema subset."""
    if '$ref' in spec:
        return _schema_validate(value, schema['$defs'][spec['$ref'].removeprefix('#/$defs/')], schema)
    if 'oneOf' in spec:
        matches = 0
        for candidate in spec['oneOf']:
            try:
                _schema_validate(value, candidate, schema)
                matches += 1
            except ValueError:
                pass
        if matches != 1:
            raise ValueError('wire does not match exactly one schema variant')
        return
    kind = spec.get('type')
    valid = {'object': isinstance(value, dict), 'array': isinstance(value, list),
             'string': isinstance(value, str), 'null': value is None,
             'number': isinstance(value, (int, float)) and not isinstance(value, bool)}
    if kind and not valid[kind]:
        raise ValueError(f'expected {kind}')
    if ('const' in spec and value != spec['const']) or ('enum' in spec and value not in spec['enum']):
        raise ValueError('unknown enum or constant')
    if kind == 'object':
        properties = spec['properties']
        if not set(spec['required']) <= value.keys() or (spec.get('additionalProperties') is False and value.keys()-properties.keys()):
            raise ValueError('missing or unknown field')
        for key in value:
            _schema_validate(value[key], properties[key], schema)
    elif kind == 'array':
        if len(value) < spec.get('minItems', 0):
            raise ValueError('empty required inventory')
        for item in value:
            _schema_validate(item, spec['items'], schema)
    elif kind == 'string':
        if len(value) < spec.get('minLength', 0) or ('pattern' in spec and re.fullmatch(spec['pattern'], value) is None):
            raise ValueError('invalid string')
    elif kind == 'number':
        if not math.isfinite(value) or ('minimum' in spec and value < spec['minimum']):
            raise ValueError('invalid finite scalar')
        if isinstance(value, int) and int(float(value)) != value:
            raise ValueError('inexact integer scalar')


def ingest_observation(data):
    """Strict Rust-compatible wire and semantics; no missing state yields a scalar."""
    value = strict_json(data)
    schema_path = Path(__file__).resolve().parents[1]/'crates/joil-evidence/schemas/observation-v1.schema.json'
    schema = json.loads(schema_path.read_text())
    _schema_validate(value, schema, schema)

    def identity_string(text):
        if not text.strip() or len(text.encode()) > 512 or any(unicodedata.category(c) == 'Cc' for c in text):
            raise ValueError('invalid identity')

    if value['status'] != 'measured':
        identity_string(value['reason'])
        for missing in value.get('missing', []):
            identity_string(missing)
        for reference in value.get('diagnostics', []):
            identity_string(reference['id'])
            if not reference['revision'].strip():
                raise ValueError('invalid revision')
        return value
    m = value['measurement']
    for field in ['quantity', 'frame', 'region']:
        identity_string(m[field])
    refs = []

    def content_refs(item):
        if isinstance(item, dict):
            if set(item) == {'id', 'revision', 'sha256'}:
                refs.append(item)
            else:
                for child in item.values():
                    content_refs(child)
        elif isinstance(item, list):
            for child in item:
                content_refs(child)
    content_refs(m)
    seen, normalized = {}, {}
    for reference in refs:
        identity_string(reference['id'])
        if not reference['revision'].strip():
            raise ValueError('invalid revision')
    # Rust checks conflicts inside ScientificIdentity, not across separate evidence roles.
    refs = []
    content_refs(m['identity'])
    for reference in refs:
        key = reference['id']
        norm = unicodedata.normalize('NFC', key)
        if norm in normalized and normalized[norm] != key:
            raise ValueError('normalization_collision')
        if key in seen and seen[key] != reference:
            raise ValueError('conflicting content identity')
        normalized[norm], seen[key] = key, reference
    requirements = [unicodedata.normalize('NFC', r['id']) for r in m['identity']['requirements']]
    if type(m['identity']['schema']) is not int:
        raise ValueError('integer schema version required')
    if len(set(requirements)) != len(requirements):
        raise ValueError('duplicate requirement')
    subject = m['identity']['subject']
    if subject['kind'] == 'genome':
        if any(not math.isfinite(struct.unpack('>d', bytes.fromhex(bits))[0]) for bits in subject['bits']):
            raise ValueError('nonfinite genome')
    if subject['kind'] == 'specimen':
        identity_string(subject['id'])
    material = m['identity']['material']
    if material['status'] == 'unknown':
        if not material['reason'].strip():
            raise ValueError('unknown material needs reason')
    state = m['state']
    tier = EvidenceTier(m['tier'])
    if state['execution'] != 'succeeded' or state['numerical'] == 'invalid' or state['validation'] == 'invalid':
        raise ValueError('invalid measured state')
    for u in m['uncertainty']:
        if u['unit'] != m['unit']:
            raise ValueError('uncertainty unit mismatch')
        for field in ['component', 'method', 'dependence', 'coverage']:
            identity_string(u[field])
    if m['origin'] == 'synthetic' and tier != EvidenceTier.SOFTWARE_CONTRACT:
        raise ValueError('synthetic cannot be physical evidence')
    fidelity = {'geometry': 'F0', 'analytic_screen': 'F1', 'coupled_screen': 'F1',
                'numerically_verified': 'F2', 'model_challenge': 'F3', 'physical_validation': 'F4'}
    if tier != EvidenceTier.SOFTWARE_CONTRACT and m['fidelity'] != fidelity[tier.value]:
        raise ValueError('tier fidelity mismatch')
    if list(EvidenceTier).index(tier) >= list(EvidenceTier).index(EvidenceTier.NUMERICALLY_VERIFIED) and state['numerical'] != 'verified':
        raise ValueError('numerical verification missing')
    if state['validation'] == 'validated' or tier == EvidenceTier.PHYSICAL_VALIDATION:
        if (m['origin'] != 'measurement' or m['fidelity'] != 'F4' or tier != EvidenceTier.PHYSICAL_VALIDATION
                or state['validation'] != 'validated' or subject['kind'] != 'specimen' or material['status'] != 'identified'):
            raise ValueError('physical validation requires identified specimen measurement')
    return value


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def manifest(root, names):
    root = Path(root).resolve()
    result = {}
    for name in sorted(set(names)):
        p = (root/name).resolve()
        if not p.is_relative_to(root) or not p.is_file():
            raise ValueError('manifest path must be a regular file inside its root')
        result[name] = {'bytes': p.stat().st_size, 'sha256': sha256(p)}
    if not result:
        raise ValueError('empty evidence manifest')
    return result


def verify(root, files):
    if manifest(root, files) != files:
        raise ValueError('evidence changed after receipt')


def save_new(path, value):
    with Path(path).open('x') as f:
        json.dump(value, f, indent=2, allow_nan=False)
        f.write('\n')


def research_fitness(*, cout_bounds, flow_m3_h, reject, hydraulic_loss_j_m3,
                     quality_limit_mg_l=30., eligibility_reasons):
    """Always reports eligibility separately; unknown oil never becomes a score.

    A provisional fixed-duty screen minimizes [adverse Cout, hydraulic loss].
    This is not the qualified common-capacity fitness in SCIENCE.md, and cannot
    be used for campaign selection when any qualification reason remains.
    """
    xs = [flow_m3_h, reject, quality_limit_mg_l]
    if any(not math.isfinite(x) for x in xs) or flow_m3_h <= 0 or not 0 <= reject < 1:
        raise ValueError('invalid finite positive duty or split')
    if quality_limit_mg_l < 0:
        raise ValueError('negative quality limit')
    if hydraulic_loss_j_m3 is not None and (not math.isfinite(hydraulic_loss_j_m3) or hydraulic_loss_j_m3 < 0):
        raise ValueError('invalid hydraulic loss')
    if cout_bounds is not None:
        if len(cout_bounds)!=2 or any(not math.isfinite(x) or x<0 for x in cout_bounds) or cout_bounds[0]>cout_bounds[1]:
            raise ValueError('invalid concentration interval')
    quality = 'unknown' if cout_bounds is None else (
        'pass_at_sampled_point_only' if cout_bounds[1] <= quality_limit_mg_l else
        'fail_at_sampled_point' if cout_bounds[0] > quality_limit_mg_l else 'unresolved')
    reasons = sorted(set(eligibility_reasons))
    # This function cannot promote a result: the qualified evidence importer is separate.
    return {'policy': 'fixed-duty-exploratory-screen-v1',
            'minimize_screen_vector': None if cout_bounds is None or hydraulic_loss_j_m3 is None
                else [cout_bounds[1], hydraulic_loss_j_m3],
            'screen_units': ['mg/L dispersed oil', 'J/m3 treated water'],
            'sampled_quality': quality, 'treated_flow_m3_h': flow_m3_h*(1-reject),
            'qualified_multiobjective_vector': None,
            'qualified_capacity_gain': None,
            'campaign_admission': 'not_implemented_fail_closed',
            'remaining_qualification_reasons': reasons}
