//! Streaming marching tetrahedra over a regular grid, for the fluid domain and the
//! printed shell at once. Vertices are welded by grid edge, so the output is closed and
//! consistently oriented wherever the field is.

use super::model::Model;
use super::sdf::{feature_distance, V3};
use rayon::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub v: Vec<[f32; 3]>,
    pub f: Vec<[u32; 3]>,
    /// Owning unit per vertex (`u16::MAX` = channel / none).
    pub owner: Vec<u16>,
}

/// Freudenthal decomposition of a cube (corners in binary xyz order) into 6 tetrahedra
/// sharing the 0–7 diagonal; consistent across neighbouring cubes.
const TETS: [[usize; 4]; 6] = [
    [0, 1, 3, 7],
    [0, 3, 2, 7],
    [0, 2, 6, 7],
    [0, 6, 4, 7],
    [0, 4, 5, 7],
    [0, 5, 1, 7],
];

/// A grid corner: (global id, position, field value, owning unit).
type Corner = (u64, V3, f32, u16);

struct Slice {
    fluid: Vec<f32>,
    solid: Vec<f32>,
    owner: Vec<u16>,
}

struct Builder {
    mesh: Mesh,
    edges: HashMap<(u64, u64), u32>,
}

impl Builder {
    fn new() -> Self {
        Self {
            mesh: Mesh::default(),
            edges: HashMap::new(),
        }
    }

    fn vertex(&mut self, a: Corner, b: Corner) -> u32 {
        let key = if a.0 < b.0 { (a.0, b.0) } else { (b.0, a.0) };
        if let Some(i) = self.edges.get(&key) {
            return *i;
        }
        let t = (a.2 / (a.2 - b.2)).clamp(0.0, 1.0) as f64;
        let p = [
            a.1[0] + t * (b.1[0] - a.1[0]),
            a.1[1] + t * (b.1[1] - a.1[1]),
            a.1[2] + t * (b.1[2] - a.1[2]),
        ];
        let owner = if a.2 < 0.0 { a.3 } else { b.3 };
        let i = self.mesh.v.len() as u32;
        self.mesh.v.push([p[0] as f32, p[1] as f32, p[2] as f32]);
        self.mesh.owner.push(owner);
        self.edges.insert(key, i);
        i
    }

    /// Emit one triangle on the edges `e`, keeping the given winding when `keep`.
    fn tri(&mut self, e: [(Corner, Corner); 3], keep: bool) {
        let ids = [
            self.vertex(e[0].0, e[0].1),
            self.vertex(e[1].0, e[1].1),
            self.vertex(e[2].0, e[2].1),
        ];
        if ids[0] == ids[1] || ids[1] == ids[2] || ids[0] == ids[2] {
            return;
        }
        self.mesh
            .f
            .push(if keep { ids } else { [ids[0], ids[2], ids[1]] });
    }

    /// Emit the surface of one tetrahedron; corners are (grid id, position, value, owner).
    /// Winding is decided combinatorially from the exact orientation of the tetrahedron
    /// (grid-aligned corners), so slivers can never flip.
    fn tet(&mut self, c: [Corner; 4]) {
        let inside: Vec<usize> = (0..4).filter(|i| c[*i].2 < 0.0).collect();
        let outside: Vec<usize> = (0..4).filter(|i| c[*i].2 >= 0.0).collect();
        if inside.is_empty() || outside.is_empty() {
            return;
        }
        let det = |a: usize, b: usize, x: usize, y: usize| {
            let o = c[a].1;
            let u = [c[b].1[0] - o[0], c[b].1[1] - o[1], c[b].1[2] - o[2]];
            let v = [c[x].1[0] - o[0], c[x].1[1] - o[1], c[x].1[2] - o[2]];
            let w = [c[y].1[0] - o[0], c[y].1[1] - o[1], c[y].1[2] - o[2]];
            u[0] * (v[1] * w[2] - v[2] * w[1]) - u[1] * (v[0] * w[2] - v[2] * w[0])
                + u[2] * (v[0] * w[1] - v[1] * w[0])
        };
        match inside.len() {
            1 | 3 => {
                let single = inside.len() == 1;
                let (lone, o) = if single {
                    (inside[0], outside)
                } else {
                    (outside[0], inside)
                };
                // For a positive tet (a,b,c,d), (ab, ac, ad) faces away from a.
                let away = det(lone, o[0], o[1], o[2]) > 0.0;
                let keep = if single { away } else { !away };
                self.tri(
                    [(c[lone], c[o[0]]), (c[lone], c[o[1]]), (c[lone], c[o[2]])],
                    keep,
                );
            }
            _ => {
                let (a, b) = (inside[0], inside[1]);
                let (x, y) = (outside[0], outside[1]);
                let keep = det(a, b, x, y) > 0.0;
                self.tri([(c[a], c[x]), (c[a], c[y]), (c[b], c[y])], keep);
                self.tri([(c[a], c[x]), (c[b], c[y]), (c[b], c[x])], keep);
            }
        }
    }
}

/// Grid spacing and extent of a model at voxel size `h` (mm).
pub struct Grid {
    pub h: f64,
    pub x0: f64,
    pub z0: f64,
    pub nx: usize,
    pub nz: usize,
}

pub fn grid(m: &Model, h: f64) -> Grid {
    let pad = m.wall + 2.0 * h;
    let x0 = -(m.radius + pad);
    let nx = ((2.0 * (m.radius + pad)) / h).ceil() as usize + 1;
    // Quarter-voxel offset keeps grid planes off the planar port cuts at z = 0 and z = L.
    let z0 = -pad - 0.25 * h;
    let nz = ((m.length + 2.0 * pad) / h).ceil() as usize + 1;
    Grid { h, x0, z0, nx, nz }
}

fn sample(m: &Model, g: &Grid, k: usize) -> Slice {
    let z = g.z0 + k as f64 * g.h;
    let margin = m.wall + 2.0 * g.h;
    let far = (margin + 1.0) as f32;
    let cand: Vec<usize> = (0..m.features.len())
        .filter(|i| {
            let f = &m.features[*i];
            f.lo[2] - margin <= z && f.hi[2] + margin >= z
        })
        .collect();
    let n = g.nx;
    let rows: Vec<(Vec<f32>, Vec<f32>, Vec<u16>)> = (0..n)
        .into_par_iter()
        .map(|j| {
            let y = g.x0 + j as f64 * g.h;
            let near: Vec<usize> = cand
                .iter()
                .copied()
                .filter(|i| {
                    let f = &m.features[*i];
                    f.lo[1] - margin <= y && f.hi[1] + margin >= y
                })
                .collect();
            let mut fl = vec![far; n];
            let mut so = vec![far; n];
            let mut ow = vec![u16::MAX; n];
            for i in 0..n {
                let x = g.x0 + i as f64 * g.h;
                let p = [x, y, z];
                let mut best = far as f64;
                let mut who = u16::MAX;
                for &fi in &near {
                    let f = &m.features[fi];
                    if f.lo[0] - margin > x || f.hi[0] + margin < x {
                        continue;
                    }
                    let d = feature_distance(&m.features, fi, p, m.wall);
                    if d < best {
                        best = d;
                        who = f.unit.map(|u| u as u16).unwrap_or(u16::MAX);
                    }
                }
                let cut = (-z).max(z - m.length);
                fl[i] = best.max(cut) as f32;
                so[i] = (best - m.wall).max(-best).max(cut) as f32;
                ow[i] = who;
            }
            (fl, so, ow)
        })
        .collect();
    let mut s = Slice {
        fluid: Vec::with_capacity(n * n),
        solid: Vec::with_capacity(n * n),
        owner: Vec::with_capacity(n * n),
    };
    for (fl, so, ow) in rows {
        s.fluid.extend(fl);
        s.solid.extend(so);
        s.owner.extend(ow);
    }
    s
}

/// Mesh the fluid domain and the printed shell of a model at voxel size `h`.
pub fn mesh(m: &Model, h: f64) -> (Mesh, Mesh) {
    let g = grid(m, h);
    let n = g.nx;
    let plane = (n * n) as u64;
    let (mut fluid, mut solid) = (Builder::new(), Builder::new());
    let mut prev = sample(m, &g, 0);
    for k in 0..g.nz - 1 {
        let next = sample(m, &g, k + 1);
        for (which, b) in [(0usize, &mut fluid), (1usize, &mut solid)] {
            let val = |s: &Slice, idx: usize| {
                if which == 0 {
                    s.fluid[idx]
                } else {
                    s.solid[idx]
                }
            };
            for j in 0..n - 1 {
                for i in 0..n - 1 {
                    let mut corners = [(0u64, [0.0f64; 3], 0.0f32, 0u16); 8];
                    let (mut any_in, mut any_out) = (false, false);
                    for (ci, corner) in corners.iter_mut().enumerate() {
                        let (dx, dy, dz) = (ci & 1, (ci >> 1) & 1, (ci >> 2) & 1);
                        let (ii, jj) = (i + dx, j + dy);
                        let s = if dz == 0 { &prev } else { &next };
                        let idx = jj * n + ii;
                        let v = val(s, idx);
                        any_in |= v < 0.0;
                        any_out |= v >= 0.0;
                        let gid = (k + dz) as u64 * plane + idx as u64;
                        let p = [
                            g.x0 + ii as f64 * g.h,
                            g.x0 + jj as f64 * g.h,
                            g.z0 + (k + dz) as f64 * g.h,
                        ];
                        *corner = (gid, p, v, s.owner[idx]);
                    }
                    if !(any_in && any_out) {
                        continue;
                    }
                    for t in TETS {
                        b.tet([corners[t[0]], corners[t[1]], corners[t[2]], corners[t[3]]]);
                    }
                }
            }
            let keep_from = (k + 1) as u64 * plane;
            b.edges.retain(|key, _| key.0 >= keep_from);
        }
        prev = next;
    }
    (fluid.mesh, solid.mesh)
}
