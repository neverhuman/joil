//! Independent mesh observations: closure, orientation, volume, area, components and
//! Euler characteristic (hence genus). These are measured on the triangles, never taken
//! from the genome or the layout.

use super::mesh::Mesh;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeshAudit {
    pub vertices: usize,
    pub triangles: usize,
    pub closed: bool,
    pub consistently_oriented: bool,
    pub boundary_edges: usize,
    pub nonmanifold_edges: usize,
    pub misoriented_edges: usize,
    pub volume_mm3: f64,
    pub area_mm2: f64,
    pub components: usize,
    pub euler_characteristic: i64,
    /// Genus of each closed component (`(2 - χ_c) / 2`), largest component first.
    pub genus: Vec<i64>,
}

fn find(p: &mut [u32], mut x: u32) -> u32 {
    while p[x as usize] != x {
        p[x as usize] = p[p[x as usize] as usize];
        x = p[x as usize];
    }
    x
}

pub fn audit(m: &Mesh) -> MeshAudit {
    let mut edges: HashMap<(u32, u32), (u32, i32)> = HashMap::with_capacity(m.f.len() * 2);
    let (mut vol, mut area) = (0.0f64, 0.0f64);
    let mut parent: Vec<u32> = (0..m.v.len() as u32).collect();
    for t in &m.f {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            let key = if a < b { (a, b) } else { (b, a) };
            let e = edges.entry(key).or_insert((0, 0));
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            if ra != rb {
                parent[ra as usize] = rb;
            }
        }
        let p = |i: u32| {
            let v = m.v[i as usize];
            [v[0] as f64, v[1] as f64, v[2] as f64]
        };
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        vol += (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
            + a[2] * (b[0] * c[1] - b[1] * c[0]))
            / 6.0;
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * w[2] - u[2] * w[1],
            u[2] * w[0] - u[0] * w[2],
            u[0] * w[1] - u[1] * w[0],
        ];
        area += 0.5 * (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    }
    let boundary = edges.values().filter(|e| e.0 == 1).count();
    let nonmanifold = edges.values().filter(|e| e.0 > 2).count();
    let misoriented = edges.values().filter(|e| e.0 == 2 && e.1 != 0).count();
    let oriented = misoriented == 0;
    // Per-component V, E, F (only vertices used by faces count).
    let mut used = vec![false; m.v.len()];
    for t in &m.f {
        for &i in t {
            used[i as usize] = true;
        }
    }
    let mut comp: HashMap<u32, (i64, i64, i64)> = HashMap::new();
    for (i, u) in used.iter().enumerate() {
        if *u {
            let r = find(&mut parent, i as u32);
            comp.entry(r).or_default().0 += 1;
        }
    }
    for (a, _) in edges.keys() {
        let r = find(&mut parent, *a);
        comp.entry(r).or_default().1 += 1;
    }
    for t in &m.f {
        let r = find(&mut parent, t[0]);
        comp.entry(r).or_default().2 += 1;
    }
    let mut parts: Vec<(i64, i64)> = comp.values().map(|(v, e, f)| (*f, v - e + f)).collect();
    parts.sort_by_key(|p| std::cmp::Reverse(p.0));
    let chi: i64 = parts.iter().map(|p| p.1).sum();
    MeshAudit {
        vertices: used.iter().filter(|u| **u).count(),
        triangles: m.f.len(),
        closed: boundary == 0 && nonmanifold == 0,
        consistently_oriented: oriented,
        boundary_edges: boundary,
        nonmanifold_edges: nonmanifold,
        misoriented_edges: misoriented,
        volume_mm3: vol,
        area_mm2: area,
        components: parts.len(),
        euler_characteristic: chi,
        genus: parts.iter().map(|p| (2 - p.1) / 2).collect(),
    }
}
