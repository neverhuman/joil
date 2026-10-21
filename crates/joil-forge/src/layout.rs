//! Automatic cartridge layout: bays, cell clusters, plenum chambers and vertical stacking.
//!
//! Units are stacked in bays along the cartridge axis (`z` downward from the top face) by
//! their series depth from the feed. Inside a bay each unit occupies a disc; a multi-cell
//! unit is a hexagonal cluster of identical cells sharing three plenum chambers: an
//! overflow (reject) chamber above the roofs, an inlet chamber at jet height surrounding
//! the cell columns, and an underflow chamber below the exits. Channels between units are
//! routed by [`crate::routing`]. Every dimension is millimetres.

use crate::genome::{Body, Genome, Target};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    /// Printer build footprint radius (the part is printed standing on its head).
    pub radius_mm: f64,
    /// Printer build height: the longest part that prints in one piece.
    pub length_mm: f64,
    pub wall_mm: f64,
    /// Narrowest swirl passage allowed (powder removal after printing, sand in service).
    pub min_bore_mm: f64,
    /// Longest cyclone cell, in swirl-chamber diameters, the screen is trusted for.
    pub max_cell_aspect: f64,
    /// Widest ceiling the printer bridges without support. Wider horizontal channels get
    /// teardrop roofs and plenum ceilings get support posts at this pitch.
    pub bridge_mm: f64,
}

impl Default for Envelope {
    /// Ø100 mm admits the CT-35 head with its inlet manifold (the supplied CT-35 inlet
    /// faces sit at x = ±47 mm). 400 mm is a typical large powder-bed build height; set it
    /// to the machine. CT-35, the geometry behind the model's CFD anchor, is about 23
    /// swirl-chamber diameters long, so 30 allows some extrapolation and no more.
    fn default() -> Self {
        Self {
            radius_mm: 50.0,
            length_mm: 400.0,
            wall_mm: 1.5,
            min_bore_mm: 3.0,
            max_cell_aspect: 30.0,
            bridge_mm: 8.0,
        }
    }
}

impl Envelope {
    pub fn validate(&self) -> Result<(), String> {
        if ![
            self.radius_mm,
            self.length_mm,
            self.wall_mm,
            self.min_bore_mm,
            self.max_cell_aspect,
            self.bridge_mm,
        ]
        .iter()
        .all(|x| x.is_finite())
        {
            return Err("envelope dimensions and print rules must be finite".into());
        }
        if self.wall_mm < 1.0 {
            return Err("wall thickness below the 1.0 mm printable minimum".into());
        }
        let ok = self.radius_mm > 0.0
            && self.length_mm > 0.0
            && self.min_bore_mm > 0.0
            && self.max_cell_aspect > 1.0
            && self.bridge_mm > 2.0;
        if ok {
            Ok(())
        } else {
            Err("envelope dimensions and print rules must be positive".into())
        }
    }
}

/// Vertical cylinder a channel may not cross (vortex chambers, jets, beds, nodes).
/// Plenum slabs are not obstacles: a channel may pass through one inside its own wall.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Obstacle {
    pub x: f64,
    pub y: f64,
    pub z0: f64,
    pub z1: f64,
    pub r: f64,
}

/// Disc-shaped chamber.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slab {
    pub x: f64,
    pub y: f64,
    pub z0: f64,
    pub z1: f64,
    pub r: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Hardware {
    Cyclone {
        /// `[z relative to roof, wall radius]`.
        profile: Vec<[f64; 2]>,
        jets: u32,
        jet_w: f64,
        jet_h: f64,
        involute: bool,
        orifice_d: f64,
        column_r: f64,
        inlet_plenum: Slab,
        overflow_plenum: Slab,
        underflow_plenum: Slab,
    },
    Coalescer {
        radius: f64,
        length: f64,
        porosity: f64,
        strut: f64,
        inlet_plenum: Slab,
        outlet_plenum: Slab,
    },
    Splitter {
        node_r: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placed {
    pub index: usize,
    pub id: u32,
    pub bay: usize,
    pub center: [f64; 2],
    pub cells: Vec<[f64; 2]>,
    /// Radius of the unit's disc including its outer wall.
    pub footprint: f64,
    pub z_top: f64,
    pub z_bottom: f64,
    /// Cyclone roof, coalescer bed top or splitter node centre.
    pub z_roof: f64,
    pub hw: Hardware,
    /// Inlet attachment and the radius of the circle where an inlet may attach.
    pub inlet: [f64; 3],
    pub inlet_ring: f64,
    pub outlets: Vec<[f64; 3]>,
    pub obstacles: Vec<Obstacle>,
}

/// Forward edges of the network from a depth-first search from the feed; back edges
/// (recycles) are excluded. Each edge is `(from, outlet port, to)`.
pub fn forward_edges(g: &Genome) -> Vec<(usize, usize, usize)> {
    let n = g.units.len();
    let mut state = vec![0u8; n];
    let mut edges = Vec::new();
    fn dfs(u: usize, g: &Genome, state: &mut [u8], edges: &mut Vec<(usize, usize, usize)>) {
        state[u] = 1;
        for (k, r) in g.units[u].routes.iter().enumerate() {
            if let Target::Unit(id) = r.to {
                if let Some(v) = g.index_of(id) {
                    match state[v] {
                        0 => {
                            edges.push((u, k, v));
                            dfs(v, g, state, edges);
                        }
                        2 => edges.push((u, k, v)),
                        _ => {}
                    }
                }
            }
        }
        state[u] = 2;
    }
    if let Target::Unit(id) = g.feed.to {
        if let Some(s) = g.index_of(id) {
            dfs(s, g, &mut state, &mut edges);
        }
    }
    for u in 0..n {
        if state[u] == 0 {
            dfs(u, g, &mut state, &mut edges);
        }
    }
    edges
}

/// Series depth of each unit (longest forward path from the feed).
pub fn depths(g: &Genome) -> Vec<usize> {
    let n = g.units.len();
    let edges = forward_edges(g);
    let mut depth = vec![0usize; n];
    for _ in 0..=n {
        let mut changed = false;
        for &(u, _, v) in &edges {
            if depth[v] < depth[u] + 1 {
                depth[v] = depth[u] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    depth
}

/// Vertical rank of each unit (0 = top bay). Streams leave a cyclone's overflow upward and
/// every other outlet downward, so a unit fed by an overflow sits above its source and a
/// unit fed by an underflow sits below it. Inconsistent constraints fall back to depth.
pub fn ranks(g: &Genome) -> Vec<usize> {
    let n = g.units.len();
    let mut cons = Vec::new(); // x[b] >= x[a] + 1
    for (u, k, v) in forward_edges(g) {
        let upward = matches!(g.units[u].body, Body::Cyclone(_)) && k == 0;
        cons.push(if upward { (v, u) } else { (u, v) });
    }
    let mut x = vec![0i64; n];
    let mut consistent = false;
    for _ in 0..=n + 1 {
        let mut changed = false;
        for &(a, b) in &cons {
            if x[b] < x[a] + 1 {
                x[b] = x[a] + 1;
                changed = true;
            }
        }
        if !changed {
            consistent = true;
            break;
        }
    }
    if !consistent {
        return depths(g);
    }
    let lo = x.iter().copied().min().unwrap_or(0);
    x.iter().map(|v| (v - lo) as usize).collect()
}

/// First `n` points of a hexagonal lattice of pitch `p`, nearest the origin, re-centred.
pub fn hex_cluster(n: usize, p: f64) -> Vec<[f64; 2]> {
    let mut pts = Vec::new();
    let k = (n as f64).sqrt().ceil() as i64 + 2;
    for i in -k..=k {
        for j in -k..=k {
            let x = p * (i as f64 + 0.5 * j as f64);
            let y = p * (3.0_f64.sqrt() / 2.0) * j as f64;
            pts.push([x, y]);
        }
    }
    pts.sort_by(|a, b| {
        let (ra, rb) = (a[0].hypot(a[1]), b[0].hypot(b[1]));
        ra.partial_cmp(&rb)
            .unwrap()
            .then(a[1].atan2(a[0]).partial_cmp(&b[1].atan2(b[0])).unwrap())
    });
    pts.truncate(n);
    let (cx, cy) = pts.iter().fold((0.0, 0.0), |s, p| {
        (s.0 + p[0] / n as f64, s.1 + p[1] / n as f64)
    });
    pts.iter().map(|p| [p[0] - cx, p[1] - cy]).collect()
}

/// Greedy placement of discs (radii) inside a circle of radius `bound`.
/// Returns `None` if some disc cannot be placed.
pub fn pack_discs(radii: &[f64], bound: f64) -> Option<Vec<[f64; 2]>> {
    let mut order: Vec<usize> = (0..radii.len()).collect();
    order.sort_by(|a, b| radii[*b].partial_cmp(&radii[*a]).unwrap().then(a.cmp(b)));
    let mut placed: Vec<Option<[f64; 2]>> = vec![None; radii.len()];
    for &i in &order {
        let r = radii[i];
        let mut found = None;
        let mut ring = 0.0_f64;
        'search: while ring + r <= bound + 1e-9 {
            let steps = if ring == 0.0 {
                1
            } else {
                ((2.0 * std::f64::consts::PI * ring) / 1.0)
                    .ceil()
                    .clamp(8.0, 720.0) as usize
            };
            for s in 0..steps {
                let th = 2.0 * std::f64::consts::PI * s as f64 / steps as f64;
                let c = [ring * th.cos(), ring * th.sin()];
                let clear = placed.iter().enumerate().all(|(j, p)| match p {
                    Some(q) => (c[0] - q[0]).hypot(c[1] - q[1]) >= r + radii[j] - 1e-9,
                    None => true,
                });
                if clear {
                    found = Some(c);
                    break 'search;
                }
            }
            ring += 0.5;
        }
        placed[i] = Some(found?);
    }
    Some(placed.into_iter().map(|p| p.expect("placed")).collect())
}

pub use crate::shapes::{inlet_channel, unit_shape};

/// Translate a placed unit horizontally by `c` and vertically by `dz`.
pub fn translate(p: &mut Placed, c: [f64; 2], dz: f64) {
    let shift = |s: &mut Slab| {
        s.x += c[0];
        s.y += c[1];
        s.z0 += dz;
        s.z1 += dz;
    };
    p.center = [p.center[0] + c[0], p.center[1] + c[1]];
    for q in &mut p.cells {
        q[0] += c[0];
        q[1] += c[1];
    }
    p.z_top += dz;
    p.z_bottom += dz;
    p.z_roof += dz;
    p.inlet = [p.inlet[0] + c[0], p.inlet[1] + c[1], p.inlet[2] + dz];
    for o in &mut p.outlets {
        *o = [o[0] + c[0], o[1] + c[1], o[2] + dz];
    }
    for o in &mut p.obstacles {
        o.x += c[0];
        o.y += c[1];
        o.z0 += dz;
        o.z1 += dz;
    }
    match &mut p.hw {
        Hardware::Cyclone {
            inlet_plenum,
            overflow_plenum,
            underflow_plenum,
            ..
        } => {
            shift(inlet_plenum);
            shift(overflow_plenum);
            shift(underflow_plenum);
        }
        Hardware::Coalescer {
            inlet_plenum,
            outlet_plenum,
            ..
        } => {
            shift(inlet_plenum);
            shift(outlet_plenum);
        }
        Hardware::Splitter { .. } => {}
    }
}
