//! Fluid-domain features of a developed cartridge.

use super::sdf::{Feature, Role, Shape, V3};
use crate::develop::Phenotype;
use crate::layout::Hardware;
use crate::routing::End;

/// Extension of port channels beyond the end faces, so the planar cut opens them.
pub const PORT_STUB_MM: f64 = 3.0;

pub struct Model {
    pub features: Vec<Feature>,
    pub wall: f64,
    pub length: f64,
    pub radius: f64,
    /// Whether coalescer lattices are represented (needs voxel ≤ strut / 2.5).
    pub lattice: bool,
}

fn overlaps(a: &Feature, b: &Feature, m: f64) -> bool {
    (0..3).all(|k| a.lo[k] - m <= b.hi[k] && b.lo[k] - m <= a.hi[k])
}

pub fn build(ph: &Phenotype, voxel: f64) -> Model {
    let lay = &ph.layout;
    let t = lay.envelope.wall_mm;
    let mut fs: Vec<Feature> = Vec::new();
    // (unit, slab role) → feature index, for attachment and carving.
    let mut slabs: Vec<(usize, Role, usize)> = Vec::new();
    let mut lattice = false;
    for u in &lay.units {
        let i = u.index;
        match &u.hw {
            Hardware::Cyclone {
                profile,
                jets,
                jet_w,
                jet_h,
                involute,
                orifice_d,
                inlet_plenum,
                overflow_plenum,
                underflow_plenum,
                ..
            } => {
                let r0 = profile[0][1];
                let r_exit = profile[profile.len() - 1][1];
                let exit = u.z_roof + profile[profile.len() - 1][0];
                let mut chambers = Vec::new();
                for (ci, q) in u.cells.iter().enumerate() {
                    let prof = profile.iter().map(|p| [u.z_roof + p[0], p[1]]).collect();
                    chambers.push(fs.len());
                    fs.push(Feature::new(
                        Shape::Revolve {
                            x: q[0],
                            y: q[1],
                            profile: prof,
                        },
                        Role::Chamber,
                        Some(i),
                    ));
                    fs.push(Feature::new(
                        Shape::Slab {
                            x: q[0],
                            y: q[1],
                            z0: exit - 0.5,
                            z1: exit + t + 0.5,
                            r: r_exit,
                        },
                        Role::Exit,
                        Some(i),
                    ));
                    fs.push(Feature::new(
                        Shape::Capsule {
                            a: [q[0], q[1], u.z_roof + 0.5],
                            b: [q[0], q[1], u.z_roof - t - 0.5],
                            r: 0.5 * orifice_d,
                        },
                        Role::Orifice,
                        Some(i),
                    ));
                    let r_inj = r0 - 0.5 * jet_w;
                    let reach = (r0 + t + 1.0 + 0.5 * jet_w).powi(2) - r_inj * r_inj;
                    let ell = reach.max(1.0).sqrt();
                    let phase = 0.37 * ci as f64;
                    for k in 0..*jets {
                        let phi = phase + 2.0 * std::f64::consts::PI * k as f64 / *jets as f64;
                        let (c, s) = (phi.cos(), phi.sin());
                        let tang: V3 = [-s, c, 0.0];
                        let rad: V3 = [c, s, 0.0];
                        // Involute entries enter at a shallower angle (smoother wrap-in).
                        let tilt = if *involute { 0.25 } else { 0.0 };
                        let ax0: V3 = {
                            let v = [tang[0] - tilt * rad[0], tang[1] - tilt * rad[1], 0.0];
                            let n = (v[0] * v[0] + v[1] * v[1]).sqrt();
                            [v[0] / n, v[1] / n, 0.0]
                        };
                        let ax1: V3 = [-ax0[1], ax0[0], 0.0];
                        let p = [
                            q[0] + r_inj * c,
                            q[1] + r_inj * s,
                            u.z_roof + 0.5 + 0.5 * jet_h,
                        ];
                        let cen = [
                            p[0] - ax0[0] * (0.5 * ell - 1.0),
                            p[1] - ax0[1] * (0.5 * ell - 1.0),
                            p[2],
                        ];
                        fs.push(Feature::new(
                            Shape::OBox {
                                c: cen,
                                ax: [ax0, ax1, [0.0, 0.0, 1.0]],
                                h: [0.5 * ell, 0.5 * jet_w, 0.5 * jet_h],
                            },
                            Role::Jet,
                            Some(i),
                        ));
                    }
                }
                for (slab, role) in [
                    (inlet_plenum, Role::InletPlenum),
                    (overflow_plenum, Role::OverflowPlenum),
                    (underflow_plenum, Role::UnderflowPlenum),
                ] {
                    let mut f = Feature::new(
                        Shape::Slab {
                            x: slab.x,
                            y: slab.y,
                            z0: slab.z0,
                            z1: slab.z1,
                            r: slab.r,
                        },
                        role,
                        Some(i),
                    );
                    if role == Role::InletPlenum {
                        f.carve = chambers.clone();
                    }
                    slabs.push((i, role, fs.len()));
                    fs.push(f);
                }
            }
            Hardware::Coalescer {
                radius,
                length,
                porosity,
                strut,
                inlet_plenum,
                outlet_plenum,
            } => {
                let with_lattice = voxel <= strut / 2.5;
                lattice |= with_lattice;
                for q in &u.cells {
                    let mut f = Feature::new(
                        Shape::Slab {
                            x: q[0],
                            y: q[1],
                            z0: u.z_roof - 0.5,
                            z1: u.z_roof + length + 0.5,
                            r: *radius,
                        },
                        Role::Bed,
                        Some(i),
                    );
                    if with_lattice {
                        // Network gyroid g ∈ [-1.5, 1.5]: solid {g > c} has volume fraction
                        // ≈ 0.5 - c/3, so c = 3 (porosity - 0.5); level is c in distance units.
                        let cell = 3.0 * strut;
                        let level = 2.0 * (porosity - 0.5) * cell / (2.0 * std::f64::consts::PI);
                        f.lattice = Some((cell, level));
                    }
                    fs.push(f);
                }
                for (slab, role) in [
                    (inlet_plenum, Role::InletPlenum),
                    (outlet_plenum, Role::UnderflowPlenum),
                ] {
                    slabs.push((i, role, fs.len()));
                    fs.push(Feature::new(
                        Shape::Slab {
                            x: slab.x,
                            y: slab.y,
                            z0: slab.z0,
                            z1: slab.z1,
                            r: slab.r,
                        },
                        role,
                        Some(i),
                    ));
                }
            }
            Hardware::Splitter { node_r } => {
                fs.push(Feature::new(
                    Shape::Sphere {
                        c: [u.center[0], u.center[1], u.z_roof],
                        r: *node_r,
                    },
                    Role::Node,
                    Some(i),
                ));
            }
        }
    }
    // Channels, with port stubs extended through the end faces.
    let mut channel_feats: Vec<(usize, usize)> = Vec::new();
    for (ci, ch) in lay.channels.iter().enumerate() {
        let mut pts = ch.pts.clone();
        let n = pts.len();
        if ch.from.is_none() && n > 0 {
            pts[0][2] -= PORT_STUB_MM;
        }
        if matches!(ch.to, End::Reject) && n > 0 {
            pts[n - 1][2] -= PORT_STUB_MM;
        }
        if matches!(ch.to, End::Product) && n > 0 {
            pts[n - 1][2] += PORT_STUB_MM;
        }
        for w in pts.windows(2) {
            let mut f = Feature::new(
                Shape::Capsule {
                    a: w[0],
                    b: w[1],
                    r: 0.5 * ch.d,
                },
                Role::Channel,
                None,
            );
            f.channel = Some(ci);
            channel_feats.push((ci, fs.len()));
            fs.push(f);
        }
    }
    // Carve channels through plenums they do not attach to.
    let attached = |ci: usize, unit: usize, role: Role| -> bool {
        let ch = &lay.channels[ci];
        let from = ch.from.is_some_and(|(u, k)| {
            u == unit
                && match role {
                    Role::OverflowPlenum => {
                        k == 0 && matches!(lay.units[u].hw, Hardware::Cyclone { .. })
                    }
                    Role::UnderflowPlenum => {
                        !matches!(lay.units[u].hw, Hardware::Cyclone { .. }) || k == 1
                    }
                    _ => false,
                }
        });
        let to = ch.to == End::Unit(unit) && role == Role::InletPlenum;
        from || to
    };
    for &(unit, role, fi) in &slabs {
        for &(ci, cf) in &channel_feats {
            if !attached(ci, unit, role) && overlaps(&fs[fi], &fs[cf], t) {
                fs[fi].carve.push(cf);
            }
        }
    }
    Model {
        features: fs,
        wall: t,
        length: lay.length_mm,
        radius: lay.envelope.radius_mm,
        lattice,
    }
}
