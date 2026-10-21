//! Printability audit of a meshed design, for a part printed standing on its head
//! (the feed/reject face on the build plate, build direction +z).
//!
//! A ceiling is a fluid-boundary face whose outward normal (into the metal) points up
//! within 40° of vertical: metal printed over powder there. It is supported when, in the
//! horizontal plane just below it, solid lies within half the bridge length in some
//! direction (a wall, a post or the far side of a narrow passage).

use super::mesh::Mesh;
use super::model::{fluid_distance, Model};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrintAudit {
    pub build_height_mm: f64,
    pub part_height_mm: f64,
    pub fits_build_height: bool,
    pub bridge_mm: f64,
    pub ceiling_area_mm2: f64,
    /// Ceiling area farther than half the bridge from any support (estimated from a
    /// deterministic sample of ceiling faces).
    pub unsupported_ceiling_area_mm2: f64,
    pub sampled_ceiling_faces: usize,
    /// Up to eight sampled unsupported points (mm, design frame), lowest z first.
    pub unsupported_examples_mm: Vec<[f64; 3]>,
    pub support_posts: usize,
    pub teardrop_runs: usize,
    pub note: String,
}

pub fn audit(m: &Model, fluid: &Mesh, build_height_mm: f64, voxel: f64) -> PrintAudit {
    let cos_ceiling = 40.0_f64.to_radians().cos();
    let mut ceil: Vec<([f64; 3], f64)> = Vec::new();
    for f in &fluid.f {
        let [a, b, c] = f.map(|i| {
            let v = fluid.v[i as usize];
            [v[0] as f64, v[1] as f64, v[2] as f64]
        });
        let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let twice = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if twice <= 0.0 {
            continue;
        }
        let ctr = [
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ];
        // The open product port at the far end face is not a ceiling.
        if n[2] / twice > cos_ceiling && ctr[2] < m.length - voxel {
            ceil.push((ctr, 0.5 * twice));
        }
    }
    let area: f64 = ceil.iter().map(|c| c.1).sum();
    let stride = (ceil.len() / 4000).max(1);
    let half = 0.5 * m.bridge;
    let sample: Vec<&([f64; 3], f64)> = ceil.iter().step_by(stride).collect();
    let unsupported: Vec<bool> = sample
        .par_iter()
        .map(|(c, _)| {
            let z = c[2] - 0.6 * voxel;
            let step = 0.25 * voxel.min(1.0);
            (0..16).all(|k| {
                let th = std::f64::consts::PI * k as f64 / 8.0;
                let (dx, dy) = (th.cos(), th.sin());
                let mut s = step;
                while s <= half {
                    if fluid_distance(m, [c[0] + s * dx, c[1] + s * dy, z]) > 0.0 {
                        return false;
                    }
                    s += step;
                }
                true
            })
        })
        .collect();
    let (mut a_s, mut a_u) = (0.0, 0.0);
    let mut examples: Vec<[f64; 3]> = Vec::new();
    for ((c, a), u) in sample.iter().zip(&unsupported) {
        a_s += a;
        if *u {
            a_u += a;
            examples.push(c.map(|x| (x * 10.0).round() / 10.0));
        }
    }
    examples.sort_by(|a, b| a[2].total_cmp(&b[2]).then(a[0].total_cmp(&b[0])));
    let k = examples.len().div_ceil(8).max(1);
    let examples: Vec<[f64; 3]> = examples.into_iter().step_by(k).take(8).collect();
    PrintAudit {
        build_height_mm,
        part_height_mm: m.length,
        fits_build_height: m.length <= build_height_mm + 1e-9,
        bridge_mm: m.bridge,
        ceiling_area_mm2: area,
        unsupported_ceiling_area_mm2: if a_s > 0.0 { area * a_u / a_s } else { 0.0 },
        sampled_ceiling_faces: sample.len(),
        unsupported_examples_mm: examples,
        support_posts: m.posts,
        teardrop_runs: m.teardrops,
        note: "Orientation: head on the plate, built along +z. A ceiling is a downward-facing \
               internal surface within 40° of horizontal; supported means solid within half \
               the bridge length in the plane just below. A screen, not a print simulation."
            .into(),
    }
}
