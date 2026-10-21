//! Footprint, vertical extent and plenum hardware of each unit type.

use crate::genome::{Body, Genome, Target};
use crate::layout::{hex_cluster, Hardware, Obstacle, Placed, Slab};

/// Largest channel diameter attached to each unit's inlet (from routes and the feed).
pub fn inlet_channel(g: &Genome) -> Vec<f64> {
    let mut d = vec![3.0_f64; g.units.len()];
    let mut note = |to: Target, mm: f64| {
        if let Target::Unit(id) = to {
            if let Some(i) = g.index_of(id) {
                d[i] = d[i].max(mm);
            }
        }
    };
    note(g.feed.to, g.feed.channel_mm);
    for u in &g.units {
        for r in &u.routes {
            note(r.to, r.channel_mm);
        }
    }
    d
}

/// Footprint, vertical extent and hardware of one unit with its top at `z = 0` and centre
/// at the origin. Placement later translates it.
pub fn unit_shape(g: &Genome, i: usize, d_in: f64, t: f64) -> Placed {
    let u = &g.units[i];
    let d_out: Vec<f64> = u.routes.iter().map(|r| r.channel_mm).collect();
    match &u.body {
        Body::Cyclone(c) => {
            let d = c.diameter_mm;
            let r0 = 0.5 * d;
            let mut profile = vec![[0.0, r0], [c.chamber_len_ratio * d, r0]];
            for s in &c.segments {
                let z = profile.last().unwrap()[0] + s.len_ratio * d;
                profile.push([z, s.end_ratio * r0]);
            }
            let r_max = profile.iter().fold(0.0_f64, |m, p| m.max(p[1]));
            let r_exit = profile.last().unwrap()[1];
            let len = profile.last().unwrap()[0];
            let area = c.inlet_area_ratio * std::f64::consts::PI * d * d / 4.0;
            let jet_w = (area / c.jets as f64 / c.inlet_aspect).sqrt();
            let jet_h = jet_w * c.inlet_aspect;
            let column_r = r_max + t;
            let cell_r = column_r + jet_w + 2.0;
            let cells = hex_cluster(c.cells as usize, 2.0 * cell_r);
            let reach = cells.iter().fold(0.0_f64, |m, p| m.max(p[0].hypot(p[1])));
            // Inlet chamber annulus around the cluster: wide enough for the jets and for
            // the arriving channel, which lands on its roof.
            let annulus = (jet_w + 2.0).max(d_in + 2.0);
            let inlet_r = reach + column_r + annulus;
            let h_op = d_out[0] + 2.0;
            let h_up = d_out[1] + 2.0;
            let roof = h_op + t;
            let exit = roof + len;
            let over_r = (reach + 0.5 * c.orifice_ratio * d + 1.5).max(0.5 * d_out[0] + 1.5);
            let under_r = (reach + r_exit + 1.0).max(0.5 * d_out[1] + 1.5);
            let z_bottom = exit + t + h_up;
            let over = Slab {
                x: 0.0,
                y: 0.0,
                z0: 0.0,
                z1: h_op,
                r: over_r,
            };
            let under = Slab {
                x: 0.0,
                y: 0.0,
                z0: exit + t,
                z1: z_bottom,
                r: under_r,
            };
            let inlet = Slab {
                x: 0.0,
                y: 0.0,
                z0: roof,
                z1: roof + jet_h + 1.0,
                r: inlet_r,
            };
            let mut obstacles = Vec::new();
            for q in &cells {
                obstacles.push(Obstacle {
                    x: q[0],
                    y: q[1],
                    z0: roof,
                    z1: exit + t,
                    r: r_max,
                });
                obstacles.push(Obstacle {
                    x: q[0],
                    y: q[1],
                    z0: roof,
                    z1: roof + jet_h + 1.0,
                    r: column_r + jet_w + 0.5,
                });
            }
            Placed {
                index: i,
                id: u.id,
                bay: 0,
                center: [0.0, 0.0],
                cells,
                footprint: inlet_r.max(under_r).max(over_r) + t,
                z_top: 0.0,
                z_bottom,
                z_roof: roof,
                hw: Hardware::Cyclone {
                    profile,
                    jets: c.jets,
                    jet_w,
                    jet_h,
                    involute: c.involute,
                    orifice_d: c.orifice_ratio * d,
                    column_r,
                    inlet_plenum: inlet,
                    overflow_plenum: over,
                    underflow_plenum: under,
                },
                inlet: [inlet_r - 0.5 * annulus, 0.0, roof],
                inlet_ring: inlet_r - 0.5 * annulus,
                outlets: vec![[0.0, 0.0, 0.0], [0.0, 0.0, z_bottom]],
                obstacles,
            }
        }
        Body::Coalescer(c) => {
            let r = 0.5 * c.diameter_mm;
            let cells = hex_cluster(c.cells as usize, 2.0 * (r + t));
            let reach = cells.iter().fold(0.0_f64, |m, p| m.max(p[0].hypot(p[1])));
            let cluster_r = (reach + r).max(0.5 * d_in + 1.0);
            let h_in = d_in + 2.0;
            let h_out = d_out[0] + 2.0;
            let bed_top = h_in;
            let bed_end = bed_top + c.length_mm;
            let z_bottom = bed_end + h_out;
            Placed {
                index: i,
                id: u.id,
                bay: 0,
                center: [0.0, 0.0],
                cells: cells.clone(),
                footprint: cluster_r + t,
                z_top: 0.0,
                z_bottom,
                z_roof: bed_top,
                hw: Hardware::Coalescer {
                    radius: r,
                    length: c.length_mm,
                    porosity: c.porosity,
                    strut: c.strut_mm,
                    inlet_plenum: Slab {
                        x: 0.0,
                        y: 0.0,
                        z0: 0.0,
                        z1: bed_top,
                        r: cluster_r,
                    },
                    outlet_plenum: Slab {
                        x: 0.0,
                        y: 0.0,
                        z0: bed_end,
                        z1: z_bottom,
                        r: cluster_r,
                    },
                },
                inlet: [0.0, 0.0, 0.0],
                inlet_ring: 0.0,
                outlets: vec![[0.0, 0.0, z_bottom]],
                obstacles: cells
                    .iter()
                    .map(|q| Obstacle {
                        x: q[0],
                        y: q[1],
                        z0: bed_top,
                        z1: bed_end,
                        r,
                    })
                    .collect(),
            }
        }
        Body::Splitter { .. } => {
            let (a, b) = (0.5 * d_out[0], 0.5 * d_out[1]);
            let node_r = (a + b + t + 1.0).max(0.5 * d_in + 1.0);
            let off = 0.5 * node_r;
            Placed {
                index: i,
                id: u.id,
                bay: 0,
                center: [0.0, 0.0],
                cells: vec![[0.0, 0.0]],
                footprint: node_r + t,
                z_top: 0.0,
                z_bottom: 2.0 * node_r,
                z_roof: node_r,
                hw: Hardware::Splitter { node_r },
                inlet: [0.0, 0.0, 0.0],
                inlet_ring: 0.0,
                outlets: vec![[-off, 0.0, 2.0 * node_r], [off, 0.0, 2.0 * node_r]],
                obstacles: vec![Obstacle {
                    x: 0.0,
                    y: 0.0,
                    z0: 0.0,
                    z1: 2.0 * node_r,
                    r: node_r,
                }],
            }
        }
    }
}
