"""Exact-byte identities, immutable manifests and conservative research fitness."""
import hashlib
import json
import math
import struct
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
