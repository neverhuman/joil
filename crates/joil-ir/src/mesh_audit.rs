use crate::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

impl IrCompiledMesh {
    /// Verify that 100% of boundary triangles and internal feature triangles map to a valid `SemanticOriginTrace` (`IR-11`).
    pub fn verify_all_boundary_faces_have_origin_trace(&self) -> Result<(), IrError> {
        if self.triangles.len() != self.triangle_origin_ids.len() {
            return Err(IrError::MissingSemanticOriginTrace {
                triangle_index: self.triangle_origin_ids.len(),
                reason: "triangles.len() != triangle_origin_ids.len()".into(),
            });
        }
        let valid_ids: BTreeSet<u16> = self.origin_table.iter().map(|o| o.origin_id).collect();
        for (idx, &oid) in self.triangle_origin_ids.iter().enumerate() {
            if !valid_ids.contains(&oid) {
                return Err(IrError::MissingSemanticOriginTrace {
                    triangle_index: idx,
                    reason: format!("triangle {idx} references unknown origin_id {oid}"),
                });
            }
        }
        for (idx, &(_, oid)) in self.internal_feature_triangles.iter().enumerate() {
            if !valid_ids.contains(&oid) {
                return Err(IrError::MissingSemanticOriginTrace {
                    triangle_index: self.triangles.len() + idx,
                    reason: format!("internal triangle {idx} references unknown origin_id {oid}"),
                });
            }
        }
        Ok(())
    }

    /// Signed enclosed volume in `m^3` via 3D divergence theorem.
    pub fn signed_volume_m3(&self) -> f64 {
        let mut vol6 = 0.0;
        for &[i0, i1, i2] in &self.triangles {
            let a = self.vertices_m[i0];
            let b = self.vertices_m[i1];
            let c = self.vertices_m[i2];
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            vol6 += a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2];
        }
        vol6 / 6.0
    }

    /// Total wetted surface area in `m^2` (boundary wall + internal feature surfaces).
    pub fn wetted_area_m2(&self) -> f64 {
        let tri_area = |a: P3, b: P3, c: P3| -> f64 {
            let ux = b[0] - a[0];
            let uy = b[1] - a[1];
            let uz = b[2] - a[2];
            let vx = c[0] - a[0];
            let vy = c[1] - a[1];
            let vz = c[2] - a[2];
            let cx = uy * vz - uz * vy;
            let cy = uz * vx - ux * vz;
            let cz = ux * vy - uy * vx;
            0.5 * (cx * cx + cy * cy + cz * cz).sqrt()
        };
        let mut area = 0.0;
        for &[i0, i1, i2] in &self.triangles {
            area += tri_area(
                self.vertices_m[i0],
                self.vertices_m[i1],
                self.vertices_m[i2],
            );
        }
        for &(pts, _) in &self.internal_feature_triangles {
            area += tri_area(pts[0], pts[1], pts[2]);
        }
        area
    }

    /// Intersect the mesh with the plane `x = x_slice_m` and count the number of disjoint closed
    /// polygon loops in the `(y, z)` cross-section plane via segment-graph connectivity (`IR-R1`, `IR-R2`).
    pub fn count_cross_section_loops_at_x(&self, x_slice_m: f64) -> usize {
        Self::count_slice_loops_from_tri_iter(
            self.triangles
                .iter()
                .map(|&[a, b, c]| [self.vertices_m[a], self.vertices_m[b], self.vertices_m[c]]),
            x_slice_m,
        )
    }

    fn count_slice_loops_from_tri_iter<I>(tris: I, x_slice_m: f64) -> usize
    where
        I: IntoIterator<Item = [P3; 3]>,
    {
        let xp = x_slice_m + 1.37e-9;
        let quant =
            |y: f64, z: f64| -> (i64, i64) { ((y * 1e7).round() as i64, (z * 1e7).round() as i64) };
        let mut adj: BTreeMap<(i64, i64), BTreeSet<(i64, i64)>> = BTreeMap::new();
        for tri in tris {
            let z_mean = (tri[0][2] + tri[1][2] + tri[2][2]) / 3.0;
            if !(-0.010..=0.025).contains(&z_mean) {
                continue;
            }
            let mut pts = Vec::with_capacity(2);
            for (u, w) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                let du = u[0] - xp;
                let dw = w[0] - xp;
                if (du < 0.0 && dw > 0.0) || (du > 0.0 && dw < 0.0) {
                    let t = (xp - u[0]) / (w[0] - u[0]);
                    let y = u[1] + t * (w[1] - u[1]);
                    let z = u[2] + t * (w[2] - u[2]);
                    pts.push(quant(y, z));
                }
            }
            if pts.len() == 2 && pts[0] != pts[1] {
                adj.entry(pts[0]).or_default().insert(pts[1]);
                adj.entry(pts[1]).or_default().insert(pts[0]);
            }
        }

        let mut visited: BTreeSet<(i64, i64)> = BTreeSet::new();
        let mut closed_loops = 0usize;
        for &start in adj.keys() {
            if visited.contains(&start) {
                continue;
            }
            let mut q = VecDeque::new();
            let mut comp_nodes = 0usize;
            let mut all_deg_ge_2 = true;
            q.push_back(start);
            visited.insert(start);
            while let Some(u) = q.pop_front() {
                comp_nodes += 1;
                if let Some(nbrs) = adj.get(&u) {
                    if nbrs.len() < 2 {
                        all_deg_ge_2 = false;
                    }
                    for &v in nbrs {
                        if !visited.contains(&v) {
                            visited.insert(v);
                            q.push_back(v);
                        }
                    }
                } else {
                    all_deg_ge_2 = false;
                }
            }
            if comp_nodes >= 3 && all_deg_ge_2 {
                closed_loops += 1;
            }
        }
        closed_loops
    }

    /// Audit an ASCII STL string by slicing at `x = -0.045 m` and `x = +0.045 m` and counting
    /// closed cross-section loops via segment-graph connectivity (`IR-R2`).
    /// Returns `(total_disjoint_inlet_loops, loops_port1_neg_x, loops_port2_pos_x)`.
    pub fn audit_ascii_stl_inlet_enactment(stl_text: &str) -> (usize, usize, usize) {
        let mut tris: Vec<[P3; 3]> = Vec::new();
        let mut cur_verts: Vec<P3> = Vec::with_capacity(3);
        for line in stl_text.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("vertex ") {
                let mut parts = rest.split_whitespace();
                if let (Some(xs), Some(ys), Some(zs)) = (parts.next(), parts.next(), parts.next()) {
                    if let (Ok(x), Ok(y), Ok(z)) =
                        (xs.parse::<f64>(), ys.parse::<f64>(), zs.parse::<f64>())
                    {
                        cur_verts.push([x, y, z]);
                        if cur_verts.len() == 3 {
                            tris.push([cur_verts[0], cur_verts[1], cur_verts[2]]);
                            cur_verts.clear();
                        }
                    }
                }
            }
        }
        let loops_neg = Self::count_slice_loops_from_tri_iter(tris.iter().copied(), -0.045);
        let loops_pos = Self::count_slice_loops_from_tri_iter(tris.iter().copied(), 0.045);
        (loops_neg + loops_pos, loops_neg, loops_pos)
    }

    /// Measure the number of disjoint inlet channels at `x = +-0.045 m` directly from the compiled mesh
    /// via plane-slice closed-loop connectivity (`IR-04`, `IR-05`, `IR-R1`, `IR-R2`).
    pub fn measure_disjoint_inlet_paths(&self) -> usize {
        self.count_cross_section_loops_at_x(-0.045) + self.count_cross_section_loops_at_x(0.045)
    }
}

#[path = "mesh_audit/enactment.rs"]
mod enactment;
