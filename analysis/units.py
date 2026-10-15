"""Dimension-checked OpenFOAM pressure conversion; reject ambiguous p_rgh."""
import math
import re


def pressure_scale(field_text, density):
    if not math.isfinite(density) or density<=0:
        raise ValueError('positive density required')
    m=re.search(r'dimensions\s*\[([^]]+)\]\s*;',field_text)
    if not m: raise ValueError('explicit pressure dimensions required')
    try: dims=tuple(float(x) for x in m[1].split())
    except ValueError as e: raise ValueError('numeric SI dimensions required') from e
    if dims==(0,2,-2,0,0,0,0): return density
    if dims==(1,-1,-2,0,0,0,0): return 1.
    raise ValueError('unrecognized pressure dimensions')
