use crate::*;
use std::collections::BTreeMap;

impl IrCompiledMesh {
    /// Evaluate local inner fluid wall radius `R(theta, z_m)` in SI metres from `DesignIrV1`.
    pub fn inner_radius_m(ir: &DesignIrV1, theta: f64, z_m: f64) -> f64 {
        let r1_m = 0.035_f64;
        let r2_m = 0.0175_f64;
        let r3_m = 0.00875_f64;
        let l1_m = 0.070_f64;
        let lc_m = ((0.070_f64 - 0.035_f64) / 2.0) / 10.0_f64.to_radians().tan();
        let l2_m = 0.770_f64;
        let zc_m = l1_m + lc_m;
        let zt_m = zc_m + l2_m;

        let mut r_wall = if z_m <= l1_m {
            let mut delta = 0.0;
            if (0.020..=0.065).contains(&z_m) {
                let s = (z_m - 0.020) / 0.045;
                let env = (PI * s).sin().powi(2);
                for node in &ir.nodes {
                    if let IrNode::ChamberHarmonicMode {
                        azimuthal_order,
                        cos_m,
                        sin_m,
                        twist_rad,
                        ..
                    } = node
                    {
                        let phase = (*azimuthal_order as f64) * theta + twist_rad * s;
                        delta += env * (cos_m * phase.cos() + sin_m * phase.sin());
                    }
                }
            }
            r1_m + delta
        } else if z_m <= zc_m {
            let r_lin = r1_m + (r2_m - r1_m) * ((z_m - l1_m) / lc_m);
            let mut delta = 0.0;
            if (0.072..=0.166).contains(&z_m) {
                let s = (z_m - 0.072) / (0.166 - 0.072);
                let env = (PI * s).sin().powi(2);
                for node in &ir.nodes {
                    if let IrNode::ConeSplineProfile {
                        mode1_m,
                        mode2_m,
                        mode3_m,
                        ..
                    } = node
                    {
                        delta += env
                            * (mode1_m * (PI * s).sin()
                                + mode2_m * (2.0 * PI * s).sin()
                                + mode3_m * (3.0 * PI * s).sin());
                    }
                }
            }
            r_lin + delta
        } else if z_m <= zt_m {
            r2_m + (r3_m - r2_m) * ((z_m - zc_m) / l2_m)
        } else {
            r3_m
        };

        // HelicalRibbonCascade modulation
        for node in &ir.nodes {
            if let IrNode::HelicalRibbonCascade {
                vane_count,
                axial_start_m,
                axial_end_m,
                helical_twist_rad,
                radial_chord_m,
                ..
            } = node
            {
                if (*axial_start_m..=*axial_end_m).contains(&z_m)
                    && *axial_end_m > *axial_start_m + 1e-3
                {
                    let s = (z_m - axial_start_m) / (axial_end_m - axial_start_m);
                    let env = (PI * s).sin().powi(2);
                    let m = *vane_count as f64;
                    let phase = m * theta + helical_twist_rad * s;
                    let vane_profile = phase.cos().max(0.0).powi(4);
                    r_wall -= env * (0.35 * radial_chord_m) * vane_profile;
                }
            }
        }

        // TpmsPeripheralShell modulation
        for node in &ir.nodes {
            if let IrNode::TpmsPeripheralShell {
                basis,
                axial_start_m,
                axial_end_m,
                axial_wavelength_m,
                sheet_thickness_m,
                ..
            } = node
            {
                if (*axial_start_m..=*axial_end_m).contains(&z_m)
                    && *axial_end_m > *axial_start_m + 1e-3
                {
                    let s = (z_m - axial_start_m) / (axial_end_m - axial_start_m);
                    let env = (PI * s).sin().powi(2);
                    let kz = 2.0 * PI * (z_m - axial_start_m) / axial_wavelength_m;
                    let kt = 4.0 * theta;
                    let wave = match basis {
                        TpmsBasisIr::None | TpmsBasisIr::UnsupportedCustom => 0.0,
                        TpmsBasisIr::Gyroid | TpmsBasisIr::CylindricalGyroid => {
                            0.55 * (kt.sin() * kz.cos() + (kt + kz).sin() * 0.5)
                        }
                        TpmsBasisIr::SchwarzP => 0.50 * (kt.cos() + kz.cos()) * 0.5,
                        TpmsBasisIr::Diamond => 0.50 * (kt.sin() * kz.sin()),
                    };
                    r_wall += env * (0.45 * sheet_thickness_m) * wave;
                }
            }
        }

        r_wall
    }

    /// Compile a `DesignIrV1` into a 3D `IrCompiledMesh` for the given `BackendId` with zero outlet extension.
    pub fn compile(ir: &DesignIrV1, backend: BackendId) -> Result<Self, IrError> {
        Self::compile_with_extension(ir, backend, 0.0)
    }

    /// Compile a `DesignIrV1` into a watertight 2-manifold 3D `IrCompiledMesh` for the given `BackendId`
    /// and `outlet_extension_m` (`IR-04`, `IR-11`, `IR-R1`), enacting multi-branch inlet sub-channels,
    /// entry ramp contraction/drop, and folded vortex-finder skimmer sleeve directly in `triangles`.
    pub fn compile_with_extension(
        ir: &DesignIrV1,
        backend: BackendId,
        outlet_extension_m: f64,
    ) -> Result<Self, IrError> {
        ir.verify_reference_envelope()?;
        ir.check_backend_support(backend)?;
        if !outlet_extension_m.is_finite() || !(0.0..=0.5).contains(&outlet_extension_m) {
            return Err(IrError::InvalidParameter {
                gene_path: "outlet_extension_m".into(),
                reason: "outlet_extension_m must be finite in [0.0, 0.5] m".into(),
            });
        }

        let origin_lookup: BTreeMap<String, u16> = ir
            .semantic_origins
            .iter()
            .map(|o| (o.semantic_id.clone(), o.origin_id))
            .collect();
        let find_origin = |prefix: &str, fallback: &str| -> u16 {
            for (k, &v) in &origin_lookup {
                if k == prefix || k.starts_with(prefix) {
                    return v;
                }
            }
            *origin_lookup.get(fallback).unwrap_or(&0)
        };

        let n_theta = 48usize;
        let dtheta = 2.0 * PI / (n_theta as f64);
        let area_corr = (dtheta / dtheta.sin()).sqrt();

        let r_plenum_m = 0.006_f64;
        let r_plenum_poly_m = r_plenum_m * area_corr;
        let ro_m = 0.0007_f64;
        let r3_m = 0.00875_f64;
        let r3_poly_m = r3_m * area_corr;
        let z_orifice_m = -0.0028_f64;
        let z_overflow_m = -0.0228_f64;
        let lc_m = ((0.070_f64 - 0.035_f64) / 2.0) / 10.0_f64.to_radians().tan();
        let z_head_end_m = 0.070_f64 + lc_m + 0.5 * 0.035_f64;
        let z_collar_start_m = 0.070_f64 + lc_m - 0.5 * 0.035_f64;
        let z_taper_end_m = 0.070_f64 + lc_m + 0.770_f64;
        let z_underflow_m = z_taper_end_m + 0.700_f64 + outlet_extension_m;
        let inlet_area_each_m2 = 0.0625_f64 * PI * 0.070_f64 * 0.070_f64 / 4.0 / 2.0;
        let w_inlet_m = (inlet_area_each_m2 / 2.0_f64).sqrt();
        let h_inlet_m = 2.0_f64 * w_inlet_m;
        let x_inlet_m = 0.060_f64;
        let r1_m = 0.035_f64;

        let enact_internal = backend != BackendId::LegacyUnenactedCfdSurface
            && backend != BackendId::RustFastRadialSurface;

        // Inspect active InletBranchManifold, InletRamp, and VortexFinderSkimmer nodes
        let mut branches_per_port = 1u32;
        let mut vane_thickness_m = 0.0012_f64;
        let mut vane_z_fraction = 0.5_f64;
        let mut ramp_drop_m = 0.0_f64;
        let mut ramp_slot_factor = 1.0_f64;
        let mut ramp_is_active = false;
        let mut skimmer_spec: Option<(f64, f64, f64, u32)> = None;

        for node in &ir.nodes {
            match node {
                IrNode::InletBranchManifold {
                    branches_per_port: b,
                    vane_thickness_m: vt,
                    vane_z_fraction: vzf,
                    ..
                } if enact_internal => {
                    branches_per_port = *b;
                    vane_thickness_m = *vt;
                    vane_z_fraction = *vzf;
                }
                IrNode::InletRamp {
                    axial_drop_m,
                    radial_slot_factor,
                    ..
                } if enact_internal => {
                    ramp_drop_m = *axial_drop_m;
                    ramp_slot_factor = *radial_slot_factor;
                    ramp_is_active = true;
                }
                IrNode::VortexFinderSkimmer {
                    length_m,
                    outer_radius_m,
                    tip_radius_m,
                    capture_flutes,
                    ..
                } if enact_internal && *length_m > 1e-6 => {
                    skimmer_spec =
                        Some((*length_m, *outer_radius_m, *tip_radius_m, *capture_flutes));
                }
                _ => {}
            }
        }

        // Build vertical z_levels across [0, h_inlet_m]:
        // - If branches_per_port == 1: 4 uniform bands (5 levels).
        // - If branches_per_port == B >= 2: 2*B - 1 bands (even bands = B fluid sub-channels, odd bands = B-1 solid splitter vanes).
        let z_levels: Vec<f64> = if branches_per_port <= 1 {
            (0..=4).map(|iz| h_inlet_m * (iz as f64) / 4.0).collect()
        } else {
            let b_cnt = branches_per_port as usize;
            let n_vanes = b_cnt - 1;
            let mut zv = Vec::with_capacity(2 * b_cnt);
            zv.push(0.0);
            if n_vanes == 1 {
                let z_c = (vane_z_fraction * h_inlet_m).clamp(0.20 * h_inlet_m, 0.80 * h_inlet_m);
                let z_lo = (z_c - 0.5 * vane_thickness_m).max(0.001);
                let z_hi = (z_c + 0.5 * vane_thickness_m).min(h_inlet_m - 0.001);
                zv.push(z_lo);
                zv.push(z_hi);
                zv.push(h_inlet_m);
            } else {
                let sub_h =
                    ((h_inlet_m - (n_vanes as f64) * vane_thickness_m) / (b_cnt as f64)).max(0.001);
                let mut cur_z = 0.0;
                for b_idx in 0..b_cnt {
                    cur_z += sub_h;
                    if b_idx + 1 < b_cnt {
                        zv.push(cur_z);
                        cur_z += vane_thickness_m;
                        zv.push(cur_z);
                    } else {
                        zv.push(h_inlet_m);
                    }
                }
            }
            zv
        };
        let n_inlet_z = z_levels.len() - 1;

        let mut rings: Vec<(Vec<P3>, u16)> = Vec::new();
        let th_off = 0.0031_f64;
        let make_circle_ring = |r_m: f64, z_m: f64| -> Vec<P3> {
            (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    [r_m * th.cos(), r_m * th.sin(), z_m]
                })
                .collect()
        };

        let orig_plenum = find_origin("wall.plenum", "wall.plenum");
        let orig_orifice = find_origin("wall.orifice", "wall.orifice");
        let orig_chamber = find_origin("wall.chamber", "wall.chamber");
        let orig_cone = find_origin("wall.cone", "wall.cone");
        let orig_taper = find_origin("wall.collar_and_taper", "wall.collar_and_taper");
        let orig_tail = find_origin("wall.tail", "wall.tail");
        let orig_mode = find_origin("feature.chamber_mode", "wall.chamber");
        let orig_rib = find_origin("feature.ribbon_cascade", "wall.chamber");
        let orig_tpms = find_origin("feature.tpms_insert", "wall.chamber");
        let orig_cprof = find_origin("feature.cone_profile", "wall.cone");
        let orig_skim = find_origin("feature.skimmer", "wall.orifice");

        let n_plenum_z = 4usize;
        for iz in 0..=n_plenum_z {
            let f = (iz as f64) / (n_plenum_z as f64);
            let z_m = z_overflow_m + (z_orifice_m - z_overflow_m) * f;
            rings.push((make_circle_ring(r_plenum_poly_m, z_m), orig_plenum));
        }
        rings.push((
            make_circle_ring(0.5 * (r_plenum_poly_m + ro_m), z_orifice_m),
            orig_orifice,
        ));
        rings.push((make_circle_ring(ro_m, z_orifice_m), orig_orifice));
        rings.push((make_circle_ring(ro_m, 0.5 * z_orifice_m), orig_orifice));
        rings.push((make_circle_ring(ro_m, 0.0), orig_orifice));

        // If VortexFinderSkimmer is active, fold the axisymmetric meridian at z = 0:
        // (ro_m, 0.0) -> (ro_m, L_skim) -> (r_tip + flute, L_skim) -> (r_outer + flute, 0.0)
        // before continuing to the outer chamber roof ring (R_chamber, 0.0).
        if let Some((skim_len_m, skim_outer_r_m, skim_tip_r_m, skim_flutes)) = skimmer_spec {
            rings.push((make_circle_ring(ro_m, skim_len_m), orig_skim));
            let tip_ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let fl = if skim_flutes > 0 {
                        0.00018 * ((skim_flutes as f64) * th).cos()
                    } else {
                        0.0
                    };
                    let r_m = (skim_tip_r_m + fl).max(ro_m + 0.0004);
                    [r_m * th.cos(), r_m * th.sin(), skim_len_m]
                })
                .collect();
            rings.push((tip_ring, orig_skim));
            let base_ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let fl = if skim_flutes > 0 {
                        0.00018 * ((skim_flutes as f64) * th).cos()
                    } else {
                        0.0
                    };
                    let r_m = (skim_outer_r_m + fl).max(ro_m + 0.0006);
                    [r_m * th.cos(), r_m * th.sin(), 0.0]
                })
                .collect();
            rings.push((base_ring, orig_skim));
        }

        let k_chamber_0 = rings.len();
        for &z_m in &z_levels {
            let ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let r_m = Self::inner_radius_m(ir, th, z_m) * area_corr;
                    [r_m * th.cos(), r_m * th.sin(), z_m]
                })
                .collect();
            rings.push((ring, orig_chamber));
        }

        let n_head_rem = 36usize;
        for iz in 1..=n_head_rem {
            let f = (iz as f64) / (n_head_rem as f64);
            let z_m = h_inlet_m + (z_head_end_m - h_inlet_m) * f;
            let ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let r_m = Self::inner_radius_m(ir, th, z_m) * area_corr;
                    [r_m * th.cos(), r_m * th.sin(), z_m]
                })
                .collect();
            let ring_orig = if z_m <= 0.070 {
                if ir
                    .gene_mapping
                    .get("tpms_insert")
                    .map(|m| m.is_active())
                    .unwrap_or(false)
                    && (0.024..=0.044).contains(&z_m)
                {
                    orig_tpms
                } else if ir
                    .gene_mapping
                    .get("ribbon_cascade")
                    .map(|m| m.is_active())
                    .unwrap_or(false)
                    && (0.012..=0.024).contains(&z_m)
                {
                    orig_rib
                } else if ir
                    .gene_mapping
                    .get("chamber_modes")
                    .map(|m| m.is_active())
                    .unwrap_or(false)
                {
                    orig_mode
                } else {
                    orig_chamber
                }
            } else if z_m < z_collar_start_m {
                if ir
                    .gene_mapping
                    .get("cone_profile")
                    .map(|m| m.is_active())
                    .unwrap_or(false)
                {
                    orig_cprof
                } else {
                    orig_cone
                }
            } else {
                orig_taper
            };
            rings.push((ring, ring_orig));
        }

        let n_taper_rem = 12usize;
        for iz in 1..=n_taper_rem {
            let f = (iz as f64) / (n_taper_rem as f64);
            let z_m = z_head_end_m + (z_taper_end_m - z_head_end_m) * f;
            let ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let r_m = Self::inner_radius_m(ir, th, z_m) * area_corr;
                    [r_m * th.cos(), r_m * th.sin(), z_m]
                })
                .collect();
            rings.push((ring, orig_taper));
        }
        let n_tail = 8usize;
        for iz in 1..n_tail {
            let f = (iz as f64) / (n_tail as f64);
            let z_m = z_taper_end_m + (z_underflow_m - z_taper_end_m) * f;
            rings.push((make_circle_ring(r3_poly_m, z_m), orig_tail));
        }
        rings.push((make_circle_ring(r3_poly_m, z_underflow_m), orig_tail));

        let n_rings = rings.len();
        let mut vertices_m: Vec<P3> = Vec::with_capacity(2 + n_rings * n_theta + 256);
        vertices_m.push([0.00011, 0.00013, z_overflow_m]);
        vertices_m.push([0.00011, 0.00013, z_underflow_m]);

        let ring_base = 2usize;
        for (ring, _) in &rings {
            vertices_m.extend_from_slice(ring);
        }

        let mut triangles: Vec<[usize; 3]> = Vec::new();
        let mut triangle_origin_ids: Vec<u16> = Vec::new();

        let orig_term_ov = find_origin("terminal.overflow", "terminal.overflow");
        let orig_term_un = find_origin("terminal.underflow", "terminal.underflow");
        let orig_term_in1 = find_origin("terminal.inlet1", "terminal.inlet1");
        let orig_term_in2 = find_origin("terminal.inlet2", "terminal.inlet2");
        let orig_duct1 = find_origin("wall.inlet1_duct", "wall.inlet1_duct");
        let orig_duct2 = find_origin("wall.inlet2_duct", "wall.inlet2_duct");
        let orig_ramp = find_origin("feature.inlet_ramp", "wall.inlet1_duct");

        // 1. Overflow cap
        for it in 0..n_theta {
            let next = (it + 1) % n_theta;
            triangles.push([0, ring_base + next, ring_base + it]);
            triangle_origin_ids.push(orig_term_ov);
        }

        let n_port_t = 6usize;
        let it_inlet1_start = n_theta / 4;
        let it_inlet2_start = (3 * n_theta) / 4;

        // 2. Side wall quads
        for ir_idx in 0..(n_rings - 1) {
            let r0 = ring_base + ir_idx * n_theta;
            let r1 = ring_base + (ir_idx + 1) * n_theta;
            let in_inlet_z = ir_idx >= k_chamber_0 && ir_idx < k_chamber_0 + n_inlet_z;
            let band_orig = rings[ir_idx + 1].1;
            for it in 0..n_theta {
                let in_inlet1 =
                    in_inlet_z && it >= it_inlet1_start && it < it_inlet1_start + n_port_t;
                let in_inlet2 =
                    in_inlet_z && it >= it_inlet2_start && it < it_inlet2_start + n_port_t;
                if in_inlet1 || in_inlet2 {
                    continue;
                }
                let next = (it + 1) % n_theta;
                triangles.push([r0 + it, r0 + next, r1 + next]);
                triangle_origin_ids.push(band_orig);
                triangles.push([r0 + it, r1 + next, r1 + it]);
                triangle_origin_ids.push(band_orig);
            }
        }

        // Helper to connect a 4-walled rectangular duct segment between cross-section grids `idx_a` and `idx_b`
        // spanning `iz in iz_lo..=iz_hi`, `jt in 0..=n_port_t`.
        let connect_tube = |tris: &mut Vec<[usize; 3]>,
                            oids: &mut Vec<u16>,
                            idx_a: &dyn Fn(usize, usize) -> usize,
                            idx_b: &dyn Fn(usize, usize) -> usize,
                            iz_lo: usize,
                            iz_hi: usize,
                            bot_orig: u16,
                            top_orig: u16,
                            side_orig: u16| {
            for jt in 0..n_port_t {
                let (w0, w1, p0, p1) = (
                    idx_a(iz_lo, jt),
                    idx_a(iz_lo, jt + 1),
                    idx_b(iz_lo, jt),
                    idx_b(iz_lo, jt + 1),
                );
                tris.push([w0, w1, p1]);
                oids.push(bot_orig);
                tris.push([w0, p1, p0]);
                oids.push(bot_orig);
            }
            for iz in iz_lo..iz_hi {
                let (w0, w1, p0, p1) = (
                    idx_a(iz, n_port_t),
                    idx_a(iz + 1, n_port_t),
                    idx_b(iz, n_port_t),
                    idx_b(iz + 1, n_port_t),
                );
                tris.push([w0, w1, p1]);
                oids.push(side_orig);
                tris.push([w0, p1, p0]);
                oids.push(side_orig);
            }
            for jt in 0..n_port_t {
                let (w0, w1, p0, p1) = (
                    idx_a(iz_hi, jt + 1),
                    idx_a(iz_hi, jt),
                    idx_b(iz_hi, jt + 1),
                    idx_b(iz_hi, jt),
                );
                tris.push([w0, w1, p1]);
                oids.push(top_orig);
                tris.push([w0, p1, p0]);
                oids.push(top_orig);
            }
            for iz in iz_lo..iz_hi {
                let (w0, w1, p0, p1) = (
                    idx_a(iz + 1, 0),
                    idx_a(iz, 0),
                    idx_b(iz + 1, 0),
                    idx_b(iz, 0),
                );
                tris.push([w0, w1, p1]);
                oids.push(side_orig);
                tris.push([w0, p1, p0]);
                oids.push(side_orig);
            }
        };

        // 3. Extrude Inlet 1 and Inlet 2 (`IR-R1`):
        // - If unsplit and no active ramp: direct single-segment extrusion `wall_idx -> port_idx` (preserves exact baseline mesh).
        // - If `branches_per_port >= 2` or `ramp_is_active`: 3-segment watertight 2-manifold extrusion
        //   `wall_idx -> inner_idx (|x|=0.024 m) -> outer_idx (|x|=0.056 m) -> port_idx (|x|=0.060 m)`,
        //   where Segment 2 (`|x| in [0.024, 0.056] m`) extrudes `B` disjoint closed rectangular sub-channels
        //   and caps each solid splitter vane at `inner_idx` and `outer_idx`.
        for (port_number, term_orig, duct_orig, it_start, x_plane, y_from_it) in [
            (
                1u32,
                orig_term_in1,
                orig_duct1,
                it_inlet1_start,
                -x_inlet_m,
                true,
            ),
            (
                2u32,
                orig_term_in2,
                orig_duct2,
                it_inlet2_start,
                x_inlet_m,
                false,
            ),
        ] {
            let wall_idx = |iz: usize, jt: usize| -> usize {
                ring_base + (k_chamber_0 + iz) * n_theta + ((it_start + jt) % n_theta)
            };
            let strip_orig = if ramp_is_active { orig_ramp } else { duct_orig };

            if branches_per_port <= 1 && !ramp_is_active {
                let port_v_base = vertices_m.len();
                for &z_m in &z_levels {
                    for jt in 0..=n_port_t {
                        let ft = (jt as f64) / (n_port_t as f64);
                        let y_m = if y_from_it {
                            r1_m - w_inlet_m * ft
                        } else {
                            -r1_m + w_inlet_m * ft
                        };
                        vertices_m.push([x_plane, y_m, z_m]);
                    }
                }
                let port_idx =
                    |iz: usize, jt: usize| -> usize { port_v_base + iz * (n_port_t + 1) + jt };

                connect_tube(
                    &mut triangles,
                    &mut triangle_origin_ids,
                    &wall_idx,
                    &port_idx,
                    0,
                    n_inlet_z,
                    strip_orig,
                    strip_orig,
                    strip_orig,
                );
                for iz in 0..n_inlet_z {
                    for jt in 0..n_port_t {
                        let a = port_idx(iz, jt);
                        let b = port_idx(iz, jt + 1);
                        let c = port_idx(iz + 1, jt + 1);
                        let d_v = port_idx(iz + 1, jt);
                        triangles.push([a, b, c]);
                        triangle_origin_ids.push(term_orig);
                        triangles.push([a, c, d_v]);
                        triangle_origin_ids.push(term_orig);
                    }
                }
            } else {
                let sign_x = x_plane.signum();
                let x_inner = sign_x * 0.024_f64;
                let x_outer = sign_x * 0.056_f64;
                let w_inner_m = w_inlet_m * ramp_slot_factor;
                let dz_inner_m = -0.35 * ramp_drop_m;

                let inner_v_base = vertices_m.len();
                for &z_m in &z_levels {
                    for jt in 0..=n_port_t {
                        let ft = (jt as f64) / (n_port_t as f64);
                        let y_m = if y_from_it {
                            r1_m - w_inner_m * ft
                        } else {
                            -r1_m + w_inner_m * ft
                        };
                        vertices_m.push([x_inner, y_m, z_m + dz_inner_m]);
                    }
                }
                let outer_v_base = vertices_m.len();
                for &z_m in &z_levels {
                    for jt in 0..=n_port_t {
                        let ft = (jt as f64) / (n_port_t as f64);
                        let y_m = if y_from_it {
                            r1_m - w_inlet_m * ft
                        } else {
                            -r1_m + w_inlet_m * ft
                        };
                        vertices_m.push([x_outer, y_m, z_m]);
                    }
                }
                let port_v_base = vertices_m.len();
                for &z_m in &z_levels {
                    for jt in 0..=n_port_t {
                        let ft = (jt as f64) / (n_port_t as f64);
                        let y_m = if y_from_it {
                            r1_m - w_inlet_m * ft
                        } else {
                            -r1_m + w_inlet_m * ft
                        };
                        vertices_m.push([x_plane, y_m, z_m]);
                    }
                }

                let inner_idx =
                    |iz: usize, jt: usize| -> usize { inner_v_base + iz * (n_port_t + 1) + jt };
                let outer_idx =
                    |iz: usize, jt: usize| -> usize { outer_v_base + iz * (n_port_t + 1) + jt };
                let port_idx =
                    |iz: usize, jt: usize| -> usize { port_v_base + iz * (n_port_t + 1) + jt };

                // Segment 1: chamber wall window -> inner_idx (|x| = 0.024 m)
                connect_tube(
                    &mut triangles,
                    &mut triangle_origin_ids,
                    &wall_idx,
                    &inner_idx,
                    0,
                    n_inlet_z,
                    strip_orig,
                    strip_orig,
                    strip_orig,
                );

                // Segment 2: inner_idx (|x| = 0.024 m) -> outer_idx (|x| = 0.056 m)
                if branches_per_port <= 1 {
                    connect_tube(
                        &mut triangles,
                        &mut triangle_origin_ids,
                        &inner_idx,
                        &outer_idx,
                        0,
                        n_inlet_z,
                        strip_orig,
                        strip_orig,
                        strip_orig,
                    );
                } else {
                    let b_cnt = branches_per_port as usize;
                    for b_idx in 0..b_cnt {
                        let iz_even = 2 * b_idx;
                        let bot_orig = if b_idx == 0 {
                            strip_orig
                        } else {
                            find_origin(
                                &format!("feature.inlet_split.p{port_number}.v{}", b_idx - 1),
                                "wall.inlet1_duct",
                            )
                        };
                        let top_orig = if b_idx + 1 == b_cnt {
                            strip_orig
                        } else {
                            find_origin(
                                &format!("feature.inlet_split.p{port_number}.v{b_idx}"),
                                "wall.inlet1_duct",
                            )
                        };
                        connect_tube(
                            &mut triangles,
                            &mut triangle_origin_ids,
                            &inner_idx,
                            &outer_idx,
                            iz_even,
                            iz_even + 1,
                            bot_orig,
                            top_orig,
                            duct_orig,
                        );
                    }
                    // Cap each odd band (solid splitter vane) at inner_idx (trailing edge) and outer_idx (leading edge)
                    for v_idx in 0..(b_cnt - 1) {
                        let iz_odd = 2 * v_idx + 1;
                        let vane_orig = find_origin(
                            &format!("feature.inlet_split.p{port_number}.v{v_idx}"),
                            "wall.inlet1_duct",
                        );
                        for jt in 0..n_port_t {
                            let a = inner_idx(iz_odd, jt);
                            let b = inner_idx(iz_odd, jt + 1);
                            let c = inner_idx(iz_odd + 1, jt + 1);
                            let d_v = inner_idx(iz_odd + 1, jt);
                            triangles.push([a, b, c]);
                            triangle_origin_ids.push(vane_orig);
                            triangles.push([a, c, d_v]);
                            triangle_origin_ids.push(vane_orig);

                            let ao = outer_idx(iz_odd, jt);
                            let bo = outer_idx(iz_odd, jt + 1);
                            let co = outer_idx(iz_odd + 1, jt + 1);
                            let do_v = outer_idx(iz_odd + 1, jt);
                            triangles.push([ao, co, bo]);
                            triangle_origin_ids.push(vane_orig);
                            triangles.push([ao, do_v, co]);
                            triangle_origin_ids.push(vane_orig);
                        }
                    }
                }

                // Segment 3: outer_idx (|x| = 0.056 m) -> port_idx (|x| = 0.060 m)
                connect_tube(
                    &mut triangles,
                    &mut triangle_origin_ids,
                    &outer_idx,
                    &port_idx,
                    0,
                    n_inlet_z,
                    duct_orig,
                    duct_orig,
                    duct_orig,
                );

                // Terminal port patch at |x| = 0.060 m
                for iz in 0..n_inlet_z {
                    for jt in 0..n_port_t {
                        let a = port_idx(iz, jt);
                        let b = port_idx(iz, jt + 1);
                        let c = port_idx(iz + 1, jt + 1);
                        let d_v = port_idx(iz + 1, jt);
                        triangles.push([a, b, c]);
                        triangle_origin_ids.push(term_orig);
                        triangles.push([a, c, d_v]);
                        triangle_origin_ids.push(term_orig);
                    }
                }
            }
        }

        // 4. Underflow cap
        let last_ring = ring_base + (n_rings - 1) * n_theta;
        for it in 0..n_theta {
            let next = (it + 1) % n_theta;
            triangles.push([1, last_ring + it, last_ring + next]);
            triangle_origin_ids.push(orig_term_un);
        }

        let mesh = Self {
            ir_sha256: ir.ir_sha256.clone(),
            backend,
            units: "m".into(),
            vertices_m,
            triangles,
            triangle_origin_ids,
            origin_table: ir.semantic_origins.clone(),
            internal_feature_triangles: Vec::new(),
        };
        mesh.verify_all_boundary_faces_have_origin_trace()?;
        Ok(mesh)
    }
}
