"""Oil cohort accounting with unresolved mass assigned adversarially."""
import math


def bounds(injected, reject, treated, remaining, wall, *, max_closure=.01):
    xs=[injected,reject,treated,remaining,wall,max_closure]
    if any(not math.isfinite(x) or x<0 for x in xs) or injected<=0 or max_closure>1:
        raise ValueError('finite nonnegative masses and positive injected mass required')
    closure=abs(injected-reject-treated-remaining-wall)/injected
    if closure>max_closure or reject>injected:
        raise ValueError('oil inventory does not close')
    low=max(0.,reject/injected-closure)
    high=min(1.,(reject+remaining+wall)/injected+closure)
    return {'capture_bounds':[low,high], 'closure_fraction':closure,
            'unresolved_fraction':(remaining+wall)/injected}


def concentration_bounds(cin_mg_l, capture_bounds, split):
    if not math.isfinite(cin_mg_l) or cin_mg_l<0 or not math.isfinite(split) or not 0<=split<1:
        raise ValueError('invalid concentration or split')
    lo,hi=capture_bounds
    if not all(math.isfinite(x) for x in [lo,hi]) or not 0<=lo<=hi<=1:
        raise ValueError('invalid capture bounds')
    return [cin_mg_l*(1-hi)/(1-split),cin_mg_l*(1-lo)/(1-split)]
