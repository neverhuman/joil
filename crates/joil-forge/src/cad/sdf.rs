//! Signed-distance primitives for the cartridge fluid domain (millimetres).
//!
//! Unions of exact primitive distances are exact outside the fluid, which is where the
//! printed shell (`0 < d ≤ wall`) is taken. Carving a channel through a plenum subtracts
//! the channel dilated by the wall, so the two stay separated by solid material.

use serde::{Deserialize, Serialize};

pub type V3 = [f64; 3];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Shape {
    /// Solid of revolution about a vertical axis at `(x, y)`; `profile` = `[z, r]`, z increasing.
    Revolve {
        x: f64,
        y: f64,
        profile: Vec<[f64; 2]>,
    },
    /// Vertical capped cylinder.
    Slab {
        x: f64,
        y: f64,
        z0: f64,
        z1: f64,
        r: f64,
    },
    Capsule {
        a: V3,
        b: V3,
        r: f64,
    },
    /// Oriented box: centre, orthonormal axes, half extents.
    OBox {
        c: V3,
        ax: [V3; 3],
        h: V3,
    },
    Sphere {
        c: V3,
        r: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Chamber,
    Jet,
    Orifice,
    Exit,
    InletPlenum,
    OverflowPlenum,
    UnderflowPlenum,
    Bed,
    Node,
    Channel,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Feature {
    pub shape: Shape,
    pub role: Role,
    /// Owning unit, or `None` for channels.
    pub unit: Option<usize>,
    /// Owning channel for channel capsules.
    pub channel: Option<usize>,
    /// Features subtracted (dilated by the wall) from this one.
    pub carve: Vec<usize>,
    /// Optional lattice inside a bed: (cell size, level) of a gyroid network.
    pub lattice: Option<(f64, f64)>,
    pub lo: V3,
    pub hi: V3,
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn len(a: V3) -> f64 {
    dot(a, a).sqrt()
}

fn seg2(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 <= 0.0 {
        0.0
    } else {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
    };
    (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
}

impl Shape {
    pub fn distance(&self, p: V3) -> f64 {
        match self {
            Shape::Revolve { x, y, profile } => {
                let rho = (p[0] - x).hypot(p[1] - y);
                let z = p[2];
                let q = [rho, z];
                let (z0, z1) = (profile[0][0], profile[profile.len() - 1][0]);
                let mut d = seg2(q, [0.0, z0], [profile[0][1], z0]);
                d = d.min(seg2(q, [0.0, z1], [profile[profile.len() - 1][1], z1]));
                let mut inside = false;
                for w in profile.windows(2) {
                    d = d.min(seg2(q, [w[0][1], w[0][0]], [w[1][1], w[1][0]]));
                    if z >= w[0][0] && z <= w[1][0] && w[1][0] > w[0][0] {
                        let f = (z - w[0][0]) / (w[1][0] - w[0][0]);
                        inside = rho < w[0][1] + f * (w[1][1] - w[0][1]);
                    }
                }
                if inside && z > z0 && z < z1 {
                    -d
                } else {
                    d
                }
            }
            Shape::Slab { x, y, z0, z1, r } => {
                let rho = (p[0] - x).hypot(p[1] - y);
                let hz = 0.5 * (z1 - z0);
                let dz = (p[2] - 0.5 * (z0 + z1)).abs() - hz;
                let dr = rho - r;
                let outside = dr.max(0.0).hypot(dz.max(0.0));
                outside + dr.max(dz).min(0.0)
            }
            Shape::Capsule { a, b, r } => {
                let pa = sub(p, *a);
                let ba = sub(*b, *a);
                let l2 = dot(ba, ba);
                let t = if l2 <= 0.0 {
                    0.0
                } else {
                    (dot(pa, ba) / l2).clamp(0.0, 1.0)
                };
                len([pa[0] - t * ba[0], pa[1] - t * ba[1], pa[2] - t * ba[2]]) - r
            }
            Shape::OBox { c, ax, h } => {
                let d = sub(p, *c);
                let q = [
                    dot(d, ax[0]).abs() - h[0],
                    dot(d, ax[1]).abs() - h[1],
                    dot(d, ax[2]).abs() - h[2],
                ];
                len([q[0].max(0.0), q[1].max(0.0), q[2].max(0.0)])
                    + q[0].max(q[1]).max(q[2]).min(0.0)
            }
            Shape::Sphere { c, r } => len(sub(p, *c)) - r,
        }
    }

    /// Axis-aligned bounds.
    pub fn bounds(&self) -> (V3, V3) {
        match self {
            Shape::Revolve { x, y, profile } => {
                let r = profile.iter().fold(0.0_f64, |m, q| m.max(q[1]));
                (
                    [x - r, y - r, profile[0][0]],
                    [x + r, y + r, profile[profile.len() - 1][0]],
                )
            }
            Shape::Slab { x, y, z0, z1, r } => ([x - r, y - r, *z0], [x + r, y + r, *z1]),
            Shape::Capsule { a, b, r } => (
                [a[0].min(b[0]) - r, a[1].min(b[1]) - r, a[2].min(b[2]) - r],
                [a[0].max(b[0]) + r, a[1].max(b[1]) + r, a[2].max(b[2]) + r],
            ),
            Shape::OBox { c, ax, h } => {
                let mut e = [0.0; 3];
                for (k, ek) in e.iter_mut().enumerate() {
                    *ek = (0..3).map(|i| (ax[i][k] * h[i]).abs()).sum();
                }
                (
                    [c[0] - e[0], c[1] - e[1], c[2] - e[2]],
                    [c[0] + e[0], c[1] + e[1], c[2] + e[2]],
                )
            }
            Shape::Sphere { c, r } => (
                [c[0] - r, c[1] - r, c[2] - r],
                [c[0] + r, c[1] + r, c[2] + r],
            ),
        }
    }
}

impl Feature {
    pub fn new(shape: Shape, role: Role, unit: Option<usize>) -> Self {
        let (lo, hi) = shape.bounds();
        Self {
            shape,
            role,
            unit,
            channel: None,
            carve: Vec::new(),
            lattice: None,
            lo,
            hi,
        }
    }
}

/// Gyroid network value scaled to an approximate distance (mm).
pub fn gyroid(p: V3, cell: f64) -> f64 {
    let k = 2.0 * std::f64::consts::PI / cell;
    let (x, y, z) = (k * p[0], k * p[1], k * p[2]);
    (x.sin() * y.cos() + y.sin() * z.cos() + z.sin() * x.cos()) / (1.5 * k)
}

/// Distance of one feature including carving and lattice.
pub fn feature_distance(fs: &[Feature], i: usize, p: V3, wall: f64) -> f64 {
    let f = &fs[i];
    let mut d = f.shape.distance(p);
    if let Some((cell, level)) = f.lattice {
        // Fluid where the gyroid value is below the level (struts are solid).
        d = d.max(gyroid(p, cell) - level);
    }
    for &c in &f.carve {
        d = d.max(wall - fs[c].shape.distance(p));
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_signs_and_values() {
        let s = Shape::Slab {
            x: 0.0,
            y: 0.0,
            z0: 0.0,
            z1: 10.0,
            r: 5.0,
        };
        assert!((s.distance([0.0, 0.0, 5.0]) + 5.0).abs() < 1e-12);
        assert!((s.distance([8.0, 0.0, 5.0]) - 3.0).abs() < 1e-12);
        let r = Shape::Revolve {
            x: 0.0,
            y: 0.0,
            profile: vec![[0.0, 10.0], [20.0, 10.0], [40.0, 5.0]],
        };
        assert!(r.distance([0.0, 0.0, 10.0]) < 0.0);
        assert!((r.distance([12.0, 0.0, 10.0]) - 2.0).abs() < 1e-12);
        assert!((r.distance([0.0, 0.0, -3.0]) - 3.0).abs() < 1e-12);
        let c = Shape::Capsule {
            a: [0.0; 3],
            b: [10.0, 0.0, 0.0],
            r: 1.0,
        };
        assert!((c.distance([5.0, 3.0, 0.0]) - 2.0).abs() < 1e-12);
        let b = Shape::OBox {
            c: [0.0; 3],
            ax: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            h: [1.0, 2.0, 3.0],
        };
        assert!((b.distance([3.0, 0.0, 0.0]) - 2.0).abs() < 1e-12);
    }
}
