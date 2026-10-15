"""Exact-byte identities, immutable manifests and conservative research fitness."""
import hashlib
import json
import math
from pathlib import Path


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
