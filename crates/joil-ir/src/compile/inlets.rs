//! Watertight 2-manifold extrusion of Inlet 1 and Inlet 2 (`IR-R1`), including multi-branch
//! splitter vanes and entry ramp contraction/drop.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn extrude_inlet_ports<F>(
    vertices_m: &mut Vec<P3>,
    triangles: &mut Vec<[usize; 3]>,
    triangle_origin_ids: &mut Vec<u16>,
    z_levels: &[f64],
    ring_base: usize,
    k_chamber_0: usize,
    n_inlet_z: usize,
    n_theta: usize,
    n_port_t: usize,
    it_inlet1_start: usize,
    it_inlet2_start: usize,
    x_inlet_m: f64,
    r1_m: f64,
    w_inlet_m: f64,
    branches_per_port: u32,
    ramp_is_active: bool,
    ramp_slot_factor: f64,
    ramp_drop_m: f64,
    orig_term_in1: u16,
    orig_term_in2: u16,
    orig_duct1: u16,
    orig_duct2: u16,
    orig_ramp: u16,
    find_origin: &F,
) where
    F: Fn(&str, &str) -> u16,
{
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
            for &z_m in z_levels {
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
                triangles,
                triangle_origin_ids,
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
            for &z_m in z_levels {
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
            for &z_m in z_levels {
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
            for &z_m in z_levels {
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
                triangles,
                triangle_origin_ids,
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
                    triangles,
                    triangle_origin_ids,
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
                        triangles,
                        triangle_origin_ids,
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
                triangles,
                triangle_origin_ids,
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
}
