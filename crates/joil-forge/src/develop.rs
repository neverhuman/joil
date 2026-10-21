//! Development: genome → hardware plant (for the judges) + cartridge layout (for CAD and
//! geometry gates) + measured behaviour descriptors (for the diversity archive).

use crate::genome::{Body, Genome, Target};
use crate::layout::{
    depths, inlet_channel, pack_discs, ranks, translate, unit_shape, Envelope, Hardware, Placed,
};
use crate::routing::{runs_per_gap, Channel, End, Gap, Request, Router};
use joil_forge_judges::coalescer::CoalescerSpec;
use joil_forge_judges::cyclone::CycloneSpec;
use joil_forge_judges::plant::{Dest, Link, Plant, UnitNode, UnitSpec};
use joil_forge_judges::reference::PRINTED_ROUGHNESS_M;
use serde::{Deserialize, Serialize};

/// Titanium alloy density used for printed-mass estimates (Ti-6Al-4V, kg/m³).
pub const TI_DENSITY: f64 = 4430.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ports {
    pub feed: [f64; 3],
    pub reject: [f64; 3],
    pub product: [f64; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub envelope: Envelope,
    pub units: Vec<Placed>,
    pub channels: Vec<Channel>,
    pub gaps: Vec<Gap>,
    pub bays: Vec<[f64; 2]>,
    pub length_mm: f64,
    pub ports: Ports,
}

/// Descriptors measured from the developed hardware (not from gene labels).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Behavior {
    pub cyclone_cells: u32,
    pub series_depth: usize,
    pub recycles: usize,
    pub coalescer: bool,
    pub splitters: usize,
    pub min_cyclone_d_mm: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub fluid_volume_mm3: f64,
    pub wetted_area_mm2: f64,
    pub shell_mass_kg: f64,
    pub channel_length_mm: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Phenotype {
    pub plant: Plant,
    pub layout: Layout,
    pub violations: Vec<String>,
    pub behavior: Behavior,
    pub stats: Stats,
}

fn end_of(t: Target, g: &Genome) -> End {
    match t {
        Target::Unit(id) => End::Unit(g.index_of(id).expect("normalised reference")),
        Target::Product => End::Product,
        Target::Reject => End::Reject,
    }
}

pub fn develop(genome: &Genome, env: &Envelope) -> Result<Phenotype, String> {
    let g = genome.clone().normalized();
    if g.units.is_empty() {
        return Err("genome has no reachable units".into());
    }
    let t = env.wall_mm;
    let depth = ranks(&g);
    let mut levels: Vec<usize> = depth.clone();
    levels.sort_unstable();
    levels.dedup();
    let bay_of: Vec<usize> = depth
        .iter()
        .map(|d| levels.binary_search(d).expect("depth listed"))
        .collect();
    let nbays = levels.len();
    let d_in = inlet_channel(&g);
    let mut units: Vec<Placed> = (0..g.units.len())
        .map(|i| unit_shape(&g, i, d_in[i], t))
        .collect();
    let mut violations = Vec::new();
    for b in 0..nbays {
        let members: Vec<usize> = (0..units.len()).filter(|i| bay_of[*i] == b).collect();
        let radii: Vec<f64> = members.iter().map(|i| units[*i].footprint).collect();
        let centers = pack_discs(&radii, env.radius_mm).unwrap_or_else(|| {
            violations.push(format!(
                "bay {b}: units do not fit inside the Ø{:.0} mm envelope",
                2.0 * env.radius_mm
            ));
            let mut x = -env.radius_mm;
            radii
                .iter()
                .map(|r| {
                    x += r;
                    let c = [x, 0.0];
                    x += r;
                    c
                })
                .collect()
        });
        for (m, c) in members.iter().zip(centers) {
            units[*m].bay = b;
            translate(&mut units[*m], c, 0.0);
        }
    }
    // Channel requests (gap indices only, before z is known).
    let mut gap_pairs = Vec::new();
    let feed_end = end_of(g.feed.to, &g);
    let gap_end = |e: End| match e {
        End::Unit(v) => bay_of[v],
        End::Product => nbays,
        End::Reject => 0,
    };
    gap_pairs.push((0usize, gap_end(feed_end)));
    for (i, u) in g.units.iter().enumerate() {
        for (k, r) in u.routes.iter().enumerate() {
            let start = if matches!(u.body, Body::Cyclone(_)) && k == 0 {
                bay_of[i]
            } else {
                bay_of[i] + 1
            };
            gap_pairs.push((start, gap_end(end_of(r.to, &g))));
        }
    }
    let runs = runs_per_gap(&gap_pairs, nbays + 1);
    let dmax = g
        .units
        .iter()
        .flat_map(|u| u.routes.iter().map(|r| r.channel_mm))
        .fold(g.feed.channel_mm, f64::max);
    let pitch = dmax + 2.0 * t;
    let mut gaps = Vec::with_capacity(nbays + 1);
    let mut bays = Vec::with_capacity(nbays);
    let mut z = 0.0;
    for (b, n) in runs.iter().enumerate() {
        let h = (*n as f64 + 1.0) * pitch;
        gaps.push(Gap {
            z0: z,
            z1: z + h,
            levels: *n,
        });
        z += h;
        if b < nbays {
            let mut height: f64 = 0.0;
            for u in units.iter_mut().filter(|u| u.bay == b) {
                translate(u, [0.0, 0.0], z);
                height = height.max(u.z_bottom - z);
            }
            bays.push([z, z + height]);
            z += height;
        }
    }
    let length_mm = z;
    if length_mm > env.length_mm {
        violations.push(format!(
            "cartridge length {length_mm:.0} mm exceeds {:.0} mm",
            env.length_mm
        ));
    }
    let reject_d = g
        .units
        .iter()
        .flat_map(|u| u.routes.iter())
        .filter(|r| r.to == Target::Reject)
        .fold(3.0_f64, |m, r| m.max(r.channel_mm));
    let ports = Ports {
        feed: [
            -(env.radius_mm - t - 0.5 * g.feed.channel_mm - 1.0),
            0.0,
            0.0,
        ],
        reject: [env.radius_mm - t - 0.5 * reject_d - 1.0, 0.0, 0.0],
        product: [0.0, 0.0, length_mm],
    };
    let end_point = |e: End| -> ([f64; 3], f64, [f64; 2]) {
        match e {
            End::Unit(v) => (units[v].inlet, units[v].inlet_ring, units[v].center),
            End::Product => (ports.product, 0.0, [0.0, 0.0]),
            End::Reject => (ports.reject, 0.0, [0.0, 0.0]),
        }
    };
    let mut reqs = Vec::new();
    let (e, ring, ctr) = end_point(feed_end);
    reqs.push(Request {
        from: None,
        to: feed_end,
        d: g.feed.channel_mm,
        start: ports.feed,
        start_gap: 0,
        end: e,
        end_gap: gap_pairs[0].1,
        end_ring: ring,
        end_center: ctr,
    });
    let mut gi = 1;
    for (i, u) in g.units.iter().enumerate() {
        for (k, r) in u.routes.iter().enumerate() {
            let to = end_of(r.to, &g);
            let (e, ring, ctr) = end_point(to);
            reqs.push(Request {
                from: Some((i, k)),
                to,
                d: r.channel_mm,
                start: units[i].outlets[k],
                start_gap: gap_pairs[gi].0,
                end: e,
                end_gap: gap_pairs[gi].1,
                end_ring: ring,
                end_center: ctr,
            });
            gi += 1;
        }
    }
    let router = Router {
        units: &units,
        gaps: &gaps,
        radius: env.radius_mm,
        wall: t,
    };
    let (channels, route_violations) = router.route(&mut reqs);
    violations.extend(route_violations);
    let plant = build_plant(&g, &channels);
    let behavior = measure(&g, &units, nbays);
    let stats = estimate(&g, &units, &channels, t);
    Ok(Phenotype {
        plant,
        layout: Layout {
            envelope: *env,
            units,
            channels,
            gaps,
            bays,
            length_mm,
            ports,
        },
        violations,
        behavior,
        stats,
    })
}

fn build_plant(g: &Genome, channels: &[Channel]) -> Plant {
    let link_of = |from: Option<(usize, usize)>, to: Target, pumped: bool, mm: f64| -> Link {
        let length = channels
            .iter()
            .find(|c| c.from == from)
            .map(|c| c.length)
            .unwrap_or(50.0);
        Link {
            to: match to {
                Target::Unit(id) => Dest::Unit(g.index_of(id).expect("normalised")),
                Target::Product => Dest::Product,
                Target::Reject => Dest::Reject,
            },
            pumped,
            channel_d_m: mm * 1e-3,
            length_m: length * 1e-3,
        }
    };
    let units = g
        .units
        .iter()
        .enumerate()
        .map(|(i, u)| UnitNode {
            spec: match &u.body {
                Body::Cyclone(c) => {
                    let d = c.diameter_mm * 1e-3;
                    let r0 = 0.5 * d;
                    let mut st = vec![[0.0, r0], [c.chamber_len_ratio * d, r0]];
                    for s in &c.segments {
                        let z = st.last().unwrap()[0] + s.len_ratio * d;
                        st.push([z, s.end_ratio * r0]);
                    }
                    UnitSpec::Cyclone(CycloneSpec {
                        cells: c.cells,
                        stations: st,
                        inlet_jets: c.jets,
                        inlet_area_m2: c.inlet_area_ratio * std::f64::consts::PI * d * d / 4.0,
                        inlet_aspect: c.inlet_aspect,
                        involute: c.involute,
                        overflow_d_m: c.orifice_ratio * d,
                        split: c.split,
                        roughness_m: PRINTED_ROUGHNESS_M,
                    })
                }
                Body::Coalescer(c) => UnitSpec::Coalescer(CoalescerSpec {
                    cells: c.cells,
                    diameter_m: c.diameter_mm * 1e-3,
                    length_m: c.length_mm * 1e-3,
                    porosity: c.porosity,
                    strut_m: c.strut_mm * 1e-3,
                }),
                Body::Splitter { fraction } => UnitSpec::Splitter {
                    fraction: *fraction,
                },
            },
            outlets: u
                .routes
                .iter()
                .enumerate()
                .map(|(k, r)| link_of(Some((i, k)), r.to, r.pumped, r.channel_mm))
                .collect(),
        })
        .collect();
    Plant {
        units,
        feed: link_of(None, g.feed.to, false, g.feed.channel_mm),
    }
}

fn measure(g: &Genome, units: &[Placed], nbays: usize) -> Behavior {
    let depth = depths(g);
    let mut recycles = 0;
    for (i, u) in g.units.iter().enumerate() {
        for r in &u.routes {
            if let Target::Unit(id) = r.to {
                let j = g.index_of(id).expect("normalised");
                if depth[j] <= depth[i] {
                    recycles += 1;
                }
            }
        }
    }
    let _ = units;
    Behavior {
        cyclone_cells: g
            .units
            .iter()
            .filter(|u| matches!(u.body, Body::Cyclone(_)))
            .map(|u| u.body.cells())
            .sum(),
        series_depth: nbays,
        recycles,
        coalescer: g.units.iter().any(|u| matches!(u.body, Body::Coalescer(_))),
        splitters: g
            .units
            .iter()
            .filter(|u| matches!(u.body, Body::Splitter { .. }))
            .count(),
        min_cyclone_d_mm: g
            .units
            .iter()
            .filter_map(|u| {
                if let Body::Cyclone(c) = &u.body {
                    Some(c.diameter_mm)
                } else {
                    None
                }
            })
            .fold(f64::INFINITY, f64::min)
            .min(1.0e3),
    }
}

fn slab_area(r: f64, h: f64) -> f64 {
    2.0 * std::f64::consts::PI * r * r + 2.0 * std::f64::consts::PI * r * h
}

fn estimate(g: &Genome, units: &[Placed], channels: &[Channel], t: f64) -> Stats {
    let pi = std::f64::consts::PI;
    let (mut vol, mut area) = (0.0, 0.0);
    for (u, p) in g.units.iter().zip(units) {
        let n = u.body.cells() as f64;
        match &p.hw {
            Hardware::Cyclone {
                profile,
                inlet_plenum,
                overflow_plenum,
                underflow_plenum,
                ..
            } => {
                for w in profile.windows(2) {
                    let (h, a, b) = (w[1][0] - w[0][0], w[0][1], w[1][1]);
                    vol += n * pi * h * (a * a + a * b + b * b) / 3.0;
                    area += n * pi * (a + b) * (h * h + (a - b).powi(2)).sqrt();
                }
                for s in [inlet_plenum, overflow_plenum, underflow_plenum] {
                    vol += pi * s.r * s.r * (s.z1 - s.z0);
                    area += slab_area(s.r, s.z1 - s.z0);
                }
            }
            Hardware::Coalescer {
                radius,
                length,
                inlet_plenum,
                outlet_plenum,
                ..
            } => {
                vol += n * pi * radius * radius * length;
                area += n * 2.0 * pi * radius * length;
                for s in [inlet_plenum, outlet_plenum] {
                    vol += pi * s.r * s.r * (s.z1 - s.z0);
                    area += slab_area(s.r, s.z1 - s.z0);
                }
            }
            Hardware::Splitter { node_r } => {
                vol += 4.0 / 3.0 * pi * node_r.powi(3);
                area += 4.0 * pi * node_r * node_r;
            }
        }
    }
    let mut length = 0.0;
    for c in channels {
        vol += pi * 0.25 * c.d * c.d * c.length;
        area += pi * c.d * c.length;
        length += c.length;
    }
    Stats {
        fluid_volume_mm3: vol,
        wetted_area_mm2: area,
        shell_mass_kg: area * t * 1e-9 * TI_DENSITY,
        channel_length_mm: length,
    }
}
