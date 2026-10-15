#!/usr/bin/env python3
"""Compositional Exotic Design Program (ct35-exotic-head-v2) & B-Rep Compiler.

Supports:
1. Exact zero-mutation CT-35 baseline identity roundtrip
2. Full compatibility with Codex's ct35-chamber-wall-field-v1 programmes
3. Four composable exotic head mechanisms (all preserving frozen ports x=+-60 mm,
   overflow z=-22.8 mm, underflow z=1639.25 mm, and frozen collar/tail z >= 169.25 mm):
   - Layer A (inlet_ramp): Downward helical/pitched tangential entry ramp + radial slot narrowing
   - Layer B (inlet_split): Streamlined internal flow-divider vane splitting 2 ports -> 4 staggered entries
   - Layer C (chamber_modes + cone_profile): Azimuthal/spiral chamber rifling + C2-clamped S-curve reducing cone
   - Layer D (skimmer): Axial vortex-finder skimmer sleeve around the 1.4 mm overflow bore
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import sys
import time
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'cad/ct35'))
from geometry import load
from build import build as build_baseline


def default_exotic_program() -> dict[str, Any]:
    """Return the zero-perturbation exotic program (exact CT-35 baseline identity)."""
    baseline_sha = hashlib.sha256((ROOT / 'baselines/ct35/genome.json').read_bytes()).hexdigest()
    return {
        'schema_version': 2,
        'family': 'ct35-exotic-head-v2',
        'topology_class': 'baseline-ct35',
        'baseline_file_sha256': baseline_sha,
        'inlet_ramp': {
            'axial_drop_mm': 0.0,
            'radial_slot_factor': 1.0,
        },
        'inlet_split': {
            'enabled': False,
            'vane_thickness_mm': 1.2,
            'vane_z_fraction': 0.5,
        },
        'chamber_modes': [],
        'cone_profile': {
            'mode1_mm': 0.0,
            'mode2_mm': 0.0,
        },
        'skimmer': {
            'length_mm': 0.0,
            'outer_radius_mm': 2.4,
        },
    }


def normalize_program(program: dict[str, Any]) -> dict[str, Any]:
    """Accept either ct35-chamber-wall-field-v1 or ct35-exotic-head-v2, reject unknown keys, and canonicalize inactive genes."""
    expected_sha = hashlib.sha256((ROOT / 'baselines/ct35/genome.json').read_bytes()).hexdigest()
    if program.get('baseline_file_sha256') != expected_sha:
        raise ValueError('baseline_file_sha256 mismatch with frozen CT-35 genome')

    if program.get('family') == 'ct35-chamber-wall-field-v1':
        if set(program) != {'schema_version', 'family', 'baseline_file_sha256', 'modes'}:
            raise ValueError('unknown keys in ct35-chamber-wall-field-v1 program')
        out = default_exotic_program()
        out['topology_class'] = 'chamber-wall-field' if program.get('modes') else 'baseline-ct35'
        out['chamber_modes'] = list(program.get('modes', []))
        program = out

    allowed_top = {
        'schema_version',
        'family',
        'topology_class',
        'baseline_file_sha256',
        'inlet_ramp',
        'inlet_split',
        'chamber_modes',
        'cone_profile',
        'skimmer',
    }
    if not set(program).issubset(allowed_top):
        raise ValueError(f'unknown top-level keys in design program: {sorted(set(program) - allowed_top)}')
    if program.get('schema_version') != 2 or program.get('family') != 'ct35-exotic-head-v2':
        raise ValueError('unsupported design program schema or family')

    ramp = program.get('inlet_ramp', {})
    if not isinstance(ramp, dict) or not set(ramp).issubset({'axial_drop_mm', 'radial_slot_factor', 'superellipse_exponent'}):
        raise ValueError('unknown inlet_ramp keys')
    drop_mm = float(ramp.get('axial_drop_mm', 0.0))
    slot_factor = float(ramp.get('radial_slot_factor', 1.0))
    se_exp = float(ramp.get('superellipse_exponent', 2.0))
    if not (math.isfinite(drop_mm) and 0.0 <= drop_mm <= 8.0):
        raise ValueError(f'inlet_ramp.axial_drop_mm {drop_mm} out of bounds [0, 8.0]')
    if not (math.isfinite(slot_factor) and 0.72 <= slot_factor <= 1.05):
        raise ValueError(f'inlet_ramp.radial_slot_factor {slot_factor} out of bounds [0.72, 1.05]')
    if not (math.isfinite(se_exp) and 2.0 <= se_exp <= 4.0):
        raise ValueError(f'inlet_ramp.superellipse_exponent {se_exp} out of bounds [2.0, 4.0]')
    if abs(drop_mm) < 1e-12 and abs(slot_factor - 1.0) < 1e-12:
        se_exp = 2.0

    split = program.get('inlet_split', {})
    if not isinstance(split, dict) or not set(split).issubset(
        {'enabled', 'vane_thickness_mm', 'vane_z_fraction', 'branches_per_port', 'azimuthal_phase_deg'}
    ):
        raise ValueError('unknown inlet_split keys')
    split_enabled = bool(split.get('enabled', False))
    vane_t = float(split.get('vane_thickness_mm', 1.2))
    vane_z = float(split.get('vane_z_fraction', 0.5))
    branches = int(split.get('branches_per_port', 2 if split_enabled else 1))
    phase_deg = float(split.get('azimuthal_phase_deg', 0.0))
    if split_enabled:
        if not (
            math.isfinite(vane_t)
            and math.isfinite(vane_z)
            and math.isfinite(phase_deg)
            and 1.0 <= vane_t <= 1.8
            and 0.35 <= vane_z <= 0.65
            and 0.0 <= phase_deg <= 45.0
        ):
            raise ValueError('invalid inlet_split vane parameters')
        if branches not in (2, 3):
            branches = 2
    else:
        # Canonicalize inactive vane parameters so inactive genes never create duplicate genotypes
        vane_t = 1.2
        vane_z = 0.5
        branches = 1
        phase_deg = 0.0

    modes = program.get('chamber_modes', [])
    if not isinstance(modes, list) or len(modes) > 8:
        raise ValueError('at most 8 chamber_modes allowed')
    amp_sum = 0.0
    canon_modes: list[dict[str, Any]] = []
    for m in modes:
        if not isinstance(m, dict) or set(m) != {'azimuthal_order', 'cos_mm', 'sin_mm', 'twist_rad'}:
            raise ValueError('invalid or unknown chamber_mode keys')
        order = m.get('azimuthal_order')
        if type(order) is not int or not (2 <= order <= 8):
            raise ValueError('azimuthal_order must be int in [2, 8]')
        for k in ['cos_mm', 'sin_mm', 'twist_rad']:
            if not isinstance(m.get(k), (int, float)) or not math.isfinite(float(m[k])):
                raise ValueError(f'nonfinite mode parameter {k}')
        if abs(float(m['twist_rad'])) > 2.0 * math.pi:
            raise ValueError('twist_rad exceeds 2*pi')
        amp = math.hypot(float(m['cos_mm']), float(m['sin_mm']))
        amp_sum += amp
        if amp > 1e-12:
            canon_modes.append({
                'azimuthal_order': order,
                'cos_mm': float(m['cos_mm']),
                'sin_mm': float(m['sin_mm']),
                'twist_rad': float(m['twist_rad']),
            })
    if amp_sum > 1.8 + 1e-12:
        raise ValueError('chamber_modes total amplitude exceeds 1.8 mm wall clearance budget')
    canon_modes.sort(key=lambda item: (item['azimuthal_order'], item['cos_mm'], item['sin_mm']))

    cone = program.get('cone_profile', {})
    if not isinstance(cone, dict) or not set(cone).issubset({'mode1_mm', 'mode2_mm', 'mode3_mm'}):
        raise ValueError('unknown cone_profile keys')
    m1 = float(cone.get('mode1_mm', 0.0))
    m2 = float(cone.get('mode2_mm', 0.0))
    m3 = float(cone.get('mode3_mm', 0.0))
    if not (math.isfinite(m1) and math.isfinite(m2) and math.isfinite(m3) and abs(m1) + abs(m2) + abs(m3) <= 1.6 + 1e-12):
        raise ValueError('cone_profile amplitude exceeds 1.6 mm budget')

    skim = program.get('skimmer', {})
    if not isinstance(skim, dict) or not set(skim).issubset({'length_mm', 'outer_radius_mm', 'capture_flutes'}):
        raise ValueError('unknown skimmer keys')
    skim_len = float(skim.get('length_mm', 0.0))
    skim_r = float(skim.get('outer_radius_mm', 2.4))
    flutes = int(skim.get('capture_flutes', 0))
    if not (math.isfinite(skim_len) and 0.0 <= skim_len <= 10.0):
        raise ValueError('skimmer.length_mm out of bounds [0, 10.0]')
    if skim_len <= 1e-12:
        skim_len = 0.0
        skim_r = 2.4
        flutes = 0
    elif not (math.isfinite(skim_r) and 1.8 <= skim_r <= 4.0 and flutes in (0, 4, 6)):
        raise ValueError('skimmer.outer_radius_mm or capture_flutes out of bounds')

    return {
        'schema_version': 2,
        'family': 'ct35-exotic-head-v2',
        'topology_class': str(program.get('topology_class', 'baseline-ct35')),
        'baseline_file_sha256': expected_sha,
        'inlet_ramp': {
            'axial_drop_mm': drop_mm,
            'radial_slot_factor': slot_factor,
            'superellipse_exponent': se_exp,
        },
        'inlet_split': {
            'enabled': split_enabled,
            'vane_thickness_mm': vane_t,
            'vane_z_fraction': vane_z,
            'branches_per_port': branches,
            'azimuthal_phase_deg': phase_deg,
        },
        'chamber_modes': canon_modes,
        'cone_profile': {
            'mode1_mm': m1,
            'mode2_mm': m2,
            'mode3_mm': m3,
        },
        'skimmer': {
            'length_mm': skim_len,
            'outer_radius_mm': skim_r,
            'capture_flutes': flutes,
        },
    }


def is_exact_baseline(program: dict[str, Any]) -> bool:
    p = normalize_program(program)
    ramp = p['inlet_ramp']
    split = p['inlet_split']
    cone = p['cone_profile']
    skim = p['skimmer']
    active_modes = [m for m in p['chamber_modes'] if m['cos_mm'] != 0 or m['sin_mm'] != 0]
    return (
        abs(float(ramp['axial_drop_mm'])) < 1e-12
        and abs(float(ramp['radial_slot_factor']) - 1.0) < 1e-12
        and not bool(split['enabled'])
        and not active_modes
        and abs(float(cone['mode1_mm'])) < 1e-12
        and abs(float(cone['mode2_mm'])) < 1e-12
        and abs(float(skim['length_mm'])) < 1e-12
    )


def compute_descriptors(program: dict[str, Any]) -> dict[str, float]:
    """Compute dimensionless physical mechanism descriptors for quality-diversity and physics evaluation."""
    p = normalize_program(program)
    _, d = load()
    ramp = p['inlet_ramp']
    split = p['inlet_split']
    cone = p['cone_profile']
    skim = p['skimmer']
    modes = [m for m in p['chamber_modes'] if m['cos_mm'] != 0 or m['sin_mm'] != 0]

    drop_mm = float(ramp['axial_drop_mm'])
    slot_factor = float(ramp['radial_slot_factor'])
    split_enabled = bool(split['enabled'])
    vane_t = float(split['vane_thickness_mm']) if split_enabled else 0.0
    m1 = float(cone['mode1_mm'])
    m2 = float(cone['mode2_mm'])
    skim_len = float(skim['length_mm'])

    # 1. Swirl flux index: narrowing radial entry slot shifts centroid outward from r1 - w/2 toward r1 - slot_factor*w/2
    # and mildly accelerates tangential entry jet, while split vane slightly increases wetted perimeter.
    r_centroid_base = d['r1'] - 0.5 * d['w']
    r_centroid_new = d['r1'] - 0.5 * d['w'] * slot_factor
    jet_accel = 1.0 / (0.35 + 0.65 * slot_factor)
    swirl_flux_idx = (r_centroid_new / r_centroid_base) * (1.0 + 0.38 * (jet_accel - 1.0))

    # 2. Downward axial entry pitch sin(alpha_z) over the 32 mm internal ramp length
    ramp_len_mm = 32.0
    axial_pitch_sin = drop_mm / math.hypot(ramp_len_mm, drop_mm)

    # 3. Azimuthal/entry symmetry order (2.0 for dual ports, 4.0 when bifurcated manifold is active,
    # plus partial harmonic contribution from helical chamber modes)
    mode_amp = sum(math.hypot(float(m['cos_mm']), float(m['sin_mm'])) for m in modes)
    symmetry_order = (3.65 if split_enabled else 2.0) + min(0.35, 0.2 * mode_amp)

    # 4. Inlet shear mitigation factor (< 1 means gentler impingement & lower peak turbulent dissipation eps)
    # Downward pitch + slight slot shaping + S-cone upper expansion reduce sudden chamber entry shock
    shear_mitigation = (
        1.0
        - 0.22 * (drop_mm / 8.0)
        - 0.14 * max(0.0, m1 / 1.2)
        + 0.35 * max(0.0, (0.85 - slot_factor) / 0.15) ** 2
    )
    if split_enabled:
        # Bifurcating into 2 balanced sub-streams reduces large-eddy entry mixing scale l_t ~ 0.07*D_h
        shear_mitigation *= 0.88

    # 5. Cone angular acceleration persistence (S-curve cone with m1 > 0, m2 < 0 sustains swirl into fine taper)
    cone_accel_persistence = 1.0 + 0.11 * (m1 / 1.0) - 0.07 * (m2 / 1.0)

    # 6. Core shield ratio from axial vortex-finder skimmer sleeve (normalized by inlet height h=15.51 mm)
    core_shield_ratio = skim_len / d['h']

    # 7. Wall rifling vortex stabilization index from twisted azimuthal chamber modes
    helical_alignment = 0.0
    for m in modes:
        amp = math.hypot(float(m['cos_mm']), float(m['sin_mm']))
        # Negative swirl around +z aligns constructively with negative twist_rad
        twist = float(m['twist_rad'])
        sign_bonus = 1.0 if twist < 0 else 0.4
        helical_alignment += (amp / 1.5) * sign_bonus * (1.0 if m['azimuthal_order'] in (2, 3, 4) else 0.6)
    wall_rifling_stab = min(1.2, helical_alignment)

    # 8. Hydraulic resistance factor (ratio of Δp_u to CT-35 baseline at equal feed Q)
    # Gentle S-cone expansion (m1 > 0) and downward helical entry reduce entry/contraction head loss!
    hydraulic_resistance_factor = (
        1.0
        - 0.065 * (drop_mm / 8.0)
        - 0.055 * (m1 / 1.2)
        + 0.025 * (mode_amp / 1.5)
        + (0.018 if split_enabled else 0.0)
        + 0.12 * max(0.0, 1.0 - slot_factor)
    )

    return {
        'swirl_flux_index': round(swirl_flux_idx, 6),
        'axial_pitch_sin': round(axial_pitch_sin, 6),
        'azimuthal_symmetry_order': round(symmetry_order, 6),
        'shear_mitigation_factor': round(shear_mitigation, 6),
        'cone_accel_persistence': round(cone_accel_persistence, 6),
        'core_shield_ratio': round(core_shield_ratio, 6),
        'wall_rifling_stabilization': round(wall_rifling_stab, 6),
        'hydraulic_resistance_factor': round(hydraulic_resistance_factor, 6),
    }


def compile_exotic_program(program: dict[str, Any]):
    """Compile DesignProgram v2 to exact CadQuery fluid and solid head B-reps with F0 verification."""
    import cadquery as cq

    p_norm = normalize_program(program)
    genome, d = load()
    p = genome['parameters']
    baseline_fluid_wq, baseline_head_wq = build_baseline(p, d)
    fluid = baseline_fluid_wq.val()
    head = baseline_head_wq.val()

    descriptors = compute_descriptors(p_norm)
    if is_exact_baseline(p_norm):
        return fluid, head, {
            'zero_mutation_exact_baseline_branch': True,
            'f0_qualification_status': 'candidate_geometry_checks_only_pending_integrator_f0',
            'topology_class': p_norm.get('topology_class', 'baseline-ct35'),
            'minimum_wall_clearance_mm': 3.0,
            'exact_global_brep_minimum_clearance_mm': 3.0,
            'clearance_method': 'exact_baseline_uniform_wall_thickness',
            'frozen_collar_and_tail_symmetric_difference_mm3': 0.0,
            'descriptors': descriptors,
        }

    # 1. Chamber Wall Field (z in [20, 65] mm)
    active_modes = [m for m in p_norm['chamber_modes'] if m['cos_mm'] != 0 or m['sin_mm'] != 0]
    min_clearance = 3.0
    if active_modes:
        z0, z1 = 20.0, 65.0
        wires = []
        for j in range(13):
            s = j / 12.0
            z = z0 + (z1 - z0) * s
            if j in (0, 12):
                wire = cq.Wire.makeCircle(d['r1'], cq.Vector(0, 0, z), cq.Vector(0, 0, 1))
            else:
                pts = []
                for i in range(128):
                    theta = 2.0 * math.pi * i / 128.0
                    delta = sum(
                        float(m['cos_mm']) * math.cos(int(m['azimuthal_order']) * theta + float(m['twist_rad']) * s)
                        + float(m['sin_mm']) * math.sin(int(m['azimuthal_order']) * theta + float(m['twist_rad']) * s)
                        for m in active_modes
                    )
                    r = d['r1'] + (math.sin(math.pi * s) ** 2) * delta
                    pts.append(cq.Vector(r * math.cos(theta), r * math.sin(theta), z))
                wire = cq.Wire.assembleEdges([cq.Edge.makeSpline(pts, periodic=True)])
            wires.append(wire)
        chamber = cq.Solid.makeLoft(wires, ruled=False)
        old = cq.Solid.makeCylinder(d['r1'] + 0.1, z1 - z0, cq.Vector(0, 0, z0))
        fluid = fluid.cut(old).fuse(chamber).clean()
        refill = cq.Solid.makeCylinder(d['r1'], z1 - z0, cq.Vector(0, 0, z0))
        head = head.fuse(refill).cut(fluid).clean()
        outer = cq.Solid.makeCylinder(d['r1'] + p['wall_thickness_mm'], z1 - z0, cq.Vector(0, 0, z0))
        side = next(f for f in outer.Faces() if f.geomType() == 'CYLINDER')
        min_clearance = min(min_clearance, float(chamber.distance(side)))

    # 2. C2-Clamped S-Curve Reducing Cone (z in [72, 166] mm, strictly inside [l1=70, zc=169.247])
    m1 = float(p_norm['cone_profile']['mode1_mm'])
    m2 = float(p_norm['cone_profile']['mode2_mm'])
    m3 = float(p_norm['cone_profile'].get('mode3_mm', 0.0))
    if abs(m1) > 1e-9 or abs(m2) > 1e-9 or abs(m3) > 1e-9:
        zc0, zc1 = 72.0, 166.0
        cone_wires = []
        for j in range(13):
            s = j / 12.0
            z = zc0 + (zc1 - zc0) * s
            r_lin = d['r1'] + (d['r2'] - d['r1']) * ((z - d['l1']) / d['lc'])
            delta_r = (math.sin(math.pi * s) ** 2) * (
                m1 * math.sin(math.pi * s) + m2 * math.sin(2.0 * math.pi * s) + m3 * math.sin(3.0 * math.pi * s)
            )
            r_z = r_lin + delta_r
            cone_wires.append(cq.Wire.makeCircle(r_z, cq.Vector(0, 0, z), cq.Vector(0, 0, 1)))
        # Dense 200-station analytical lower bound on radial wall thickness along [zc0, zc1]
        for k_s in range(201):
            s_dense = k_s / 200.0
            dr_dense = (math.sin(math.pi * s_dense) ** 2) * (
                m1 * math.sin(math.pi * s_dense)
                + m2 * math.sin(2.0 * math.pi * s_dense)
                + m3 * math.sin(3.0 * math.pi * s_dense)
            )
            min_clearance = min(min_clearance, float(p['wall_thickness_mm'] - max(0.0, dr_dense)) * math.cos(d['a']))
        cone_loft = cq.Solid.makeLoft(cone_wires, ruled=False)
        old_cone = cq.Solid.makeCylinder(d['r1'] + 3.0, zc1 - zc0, cq.Vector(0, 0, zc0))
        fluid = fluid.cut(old_cone).fuse(cone_loft).clean()
        # Update solid head inner cavity to match
        refill_cone = cq.Solid.makeCone(
            d['r1'] - (zc0 - d['l1']) * math.tan(d['a']),
            d['r1'] - (zc1 - d['l1']) * math.tan(d['a']),
            zc1 - zc0,
            cq.Vector(0, 0, zc0),
        )
        head = head.fuse(refill_cone).cut(fluid).clean()

    # 3. Downward-Pitched & Radial-Slot-Contracted Tangential Entry Ramp (inboard of x = +-42 mm)
    drop_mm = float(p_norm['inlet_ramp']['axial_drop_mm'])
    slot_factor = float(p_norm['inlet_ramp']['radial_slot_factor'])
    if drop_mm > 1e-6:
        wedge_h = min(drop_mm * 0.45, 3.2)
        if wedge_h > 0.15:
            pts_xz = [(-38.0, d['h']), (-14.0, d['h']), (-14.0, d['h'] - wedge_h)]
            guide1 = (
                cq.Workplane('XZ')
                .workplane(offset=d['r1'] - d['w'])
                .polyline(pts_xz)
                .close()
                .extrude(d['w'])
                .val()
            )
            guide2 = guide1.mirror('YZ').mirror('XZ')
            for g in (guide1, guide2):
                fluid = fluid.cut(g).clean()
                head = head.fuse(g).clean()
    if slot_factor < 1.0 - 1e-6:
        # Sculpt an internal radial-wall convergence ramp on the inner duct wall (y = r1 - w)
        # from |x| = 38 mm to |x| = 14 mm, physically narrowing the entry throat width to slot_factor * w
        delta_w = min(d['w'] * (1.0 - slot_factor), d['w'] - 1.5)
        if delta_w > 0.05:
            pts_xy = [(-38.0, d['r1'] - d['w']), (-14.0, d['r1'] - d['w']), (-14.0, d['r1'] - d['w'] + delta_w)]
            slot_wedge1 = (
                cq.Workplane('XY')
                .workplane(offset=0.0)
                .polyline(pts_xy)
                .close()
                .extrude(d['h'])
                .val()
            )
            slot_wedge2 = slot_wedge1.mirror('YZ').mirror('XZ')
            for sw in (slot_wedge1, slot_wedge2):
                fluid = fluid.cut(sw).clean()
                head = head.fuse(sw).clean()
            min_clearance = min(min_clearance, d['w'] - delta_w)

    # 4. Multi-Branch Tangential Manifold (2 -> 4 or 2 -> 6 staggered entry streams via internal flow-divider vanes)
    if bool(p_norm['inlet_split']['enabled']):
        vane_t = float(p_norm['inlet_split']['vane_thickness_mm'])
        branches = int(p_norm['inlet_split'].get('branches_per_port', 2))
        z_fracs = [float(p_norm['inlet_split']['vane_z_fraction'])] if branches == 2 else [0.34, 0.66]
        for zf in z_fracs:
            vane_z = zf * d['h']
            vane1 = cq.Solid.makeBox(
                18.0,
                d['w'],
                vane_t,
                cq.Vector(-40.0, d['r1'] - d['w'], vane_z - 0.5 * vane_t),
            )
            vane2 = cq.Solid.makeBox(
                18.0,
                d['w'],
                vane_t,
                cq.Vector(22.0, -d['r1'], vane_z - 0.5 * vane_t),
            )
            for v in (vane1, vane2):
                fluid = fluid.cut(v).clean()
                head = head.fuse(v).clean()
        min_clearance = min(min_clearance, vane_t)

    # 5. Axial Vortex-Finder Skimmer Sleeve around the 1.4 mm overflow bore (z in [0, skim_len])
    skim_len = float(p_norm['skimmer']['length_mm'])
    skim_r = float(p_norm['skimmer']['outer_radius_mm'])
    if skim_len > 0.1:
        tip_r = max(d['ro'] + 1.0, skim_r - 0.4)
        outer_cyl = cq.Solid.makeCone(skim_r, tip_r, skim_len, cq.Vector(0, 0, 0))
        inner_bore = cq.Solid.makeCylinder(d['ro'], skim_len + 0.2, cq.Vector(0, 0, -0.1))
        sleeve = outer_cyl.cut(inner_bore).clean()
        fluid = fluid.cut(sleeve).clean()
        head = head.fuse(sleeve).clean()
        min_clearance = min(min_clearance, tip_r - d['ro'])

    # Verify frozen collar and full downstream liner invariance (z >= zh - 17.5 = 169.24743 mm)
    frozen_tail = cq.Solid.makeBox(200, 200, 2000, cq.Vector(-100, -100, d['zh'] - 17.5))
    a_tail = fluid.intersect(frozen_tail)
    b_tail = baseline_fluid_wq.val().intersect(frozen_tail)
    diff_mm3 = float(a_tail.cut(b_tail).Volume() + b_tail.cut(a_tail).Volume())

    if min_clearance < 1.0 or diff_mm3 > 1e-5:
        raise ValueError(f'Candidate geometry gate failed: min_clearance={min_clearance:.4f} mm (< 1.0 mm), tail_diff={diff_mm3:.6g} mm3')

    report = {
        'zero_mutation_exact_baseline_branch': False,
        'f0_qualification_status': 'candidate_geometry_checks_only_pending_integrator_f0',
        'topology_class': p_norm.get('topology_class', 'exotic-hybrid'),
        'minimum_wall_clearance_mm': float(min_clearance),
        'exact_global_brep_minimum_clearance_mm': None,
        'clearance_method': 'brep_chamber_distance_plus_200pt_normal_cone_and_vane_thickness_lower_bound',
        'frozen_collar_and_tail_symmetric_difference_mm3': diff_mm3,
        'active_chamber_modes': len(active_modes),
        'inlet_ramp_drop_mm': drop_mm,
        'inlet_split_enabled': bool(p_norm['inlet_split']['enabled']),
        'cone_modes_mm': [m1, m2],
        'skimmer_length_mm': skim_len,
        'descriptors': descriptors,
    }
    return fluid, head, report


if __name__ == '__main__':
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--program', type=Path)
    ap.add_argument('--out', type=Path, required=True)
    args = ap.parse_args()
    prog = json.loads(args.program.read_text()) if args.program else default_exotic_program()
    t0 = time.perf_counter()
    fluid, head, rep = compile_exotic_program(prog)
    rep['fluid_volume_mm3'] = float(fluid.Volume())
    rep['head_volume_mm3'] = float(head.Volume())
    rep['elapsed_seconds'] = time.perf_counter() - t0
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(rep, indent=2) + '\n')
    print(json.dumps(rep, indent=2))
