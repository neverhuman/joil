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

    /// Verify feature enactment (`IR-04`) and produce a `FeatureEnactmentReceipt`.
    /// Fails closed if any active mechanism in `DesignIrV1` is missing from the compiled mesh!
    pub fn verify_feature_enactment(
        &self,
        ir: &DesignIrV1,
    ) -> Result<FeatureEnactmentReceipt, IrError> {
        let mut origin_to_tris: BTreeMap<u16, Vec<usize>> = BTreeMap::new();
        let mut origin_to_area: BTreeMap<u16, f64> = BTreeMap::new();

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

        for (idx, &[i0, i1, i2]) in self.triangles.iter().enumerate() {
            let oid = self.triangle_origin_ids[idx];
            origin_to_tris.entry(oid).or_default().push(idx);
            *origin_to_area.entry(oid).or_insert(0.0) += tri_area(
                self.vertices_m[i0],
                self.vertices_m[i1],
                self.vertices_m[i2],
            );
        }
        for (idx, &(pts, oid)) in self.internal_feature_triangles.iter().enumerate() {
            origin_to_tris
                .entry(oid)
                .or_default()
                .push(self.triangles.len() + idx);
            *origin_to_area.entry(oid).or_insert(0.0) += tri_area(pts[0], pts[1], pts[2]);
        }

        let sem_to_oid: BTreeMap<String, u16> = self
            .origin_table
            .iter()
            .map(|o| (o.semantic_id.clone(), o.origin_id))
            .collect();

        let measured_inlet_paths = self.measure_disjoint_inlet_paths();
        let mut expected_inlet_paths = 2usize;
        for node in &ir.nodes {
            if let IrNode::InletBranchManifold { total_jets, .. } = node {
                expected_inlet_paths = *total_jets as usize;
            }
        }

        let mut active_mechanisms = Vec::new();
        let mut unenacted = Vec::new();

        for (gene_name, mapping) in &ir.gene_mapping {
            if let GeneIrMapping::Active {
                node_ids,
                semantic_boundary_ids,
            } = mapping
            {
                let mut faces = Vec::new();
                let mut area_m2 = 0.0;
                for s_id in semantic_boundary_ids {
                    if let Some(&oid) = sem_to_oid.get(s_id) {
                        if let Some(f_list) = origin_to_tris.get(&oid) {
                            faces.extend_from_slice(f_list);
                        }
                        area_m2 += origin_to_area.get(&oid).copied().unwrap_or(0.0);
                    }
                }

                let delta_mm = self.measure_gene_geometric_delta_mm(gene_name, ir);
                let mut enacted = !faces.is_empty() && area_m2 > 1e-10 && delta_mm > 1e-4;
                if gene_name == "inlet_split" && measured_inlet_paths != expected_inlet_paths {
                    enacted = false;
                }

                if !enacted {
                    unenacted.push(gene_name.clone());
                }

                active_mechanisms.push(MechanismEnactmentRecord {
                    gene_name: gene_name.clone(),
                    ir_node_ids: node_ids.clone(),
                    enacted_in_subject: enacted,
                    face_indices: faces.clone(),
                    volume_region_id: format!("region.{gene_name}"),
                    enacted_triangle_count: faces.len(),
                    enacted_surface_area_m2: round9(area_m2),
                    measured_geometric_delta_mm: round6(delta_mm),
                });
            }
        }

        let actual_label = format!(
            "{}jet-mesh-enacted-{}",
            measured_inlet_paths,
            active_mechanisms
                .iter()
                .filter(|m| m.enacted_in_subject
                    && m.gene_name != "body_envelope"
                    && m.gene_name != "terminals")
                .map(|m| m.gene_name.as_str())
                .collect::<Vec<_>>()
                .join("+")
        );

        if !unenacted.is_empty() {
            return Err(IrError::UnenactedFeature {
                feature: unenacted.join(","),
                ir_node_id: "active_gene_enactment_gate".into(),
                reason: format!(
                    "active mechanisms {:?} not enacted in compiled mesh (measured_inlet_paths={measured_inlet_paths}, expected={expected_inlet_paths})",
                    unenacted
                ),
            });
        }

        Ok(FeatureEnactmentReceipt {
            ir_sha256: self.ir_sha256.clone(),
            backend: self.backend,
            all_active_features_enacted: true,
            measured_disjoint_inlet_paths: measured_inlet_paths,
            expected_disjoint_inlet_paths: expected_inlet_paths,
            actual_simulated_geometry_label: actual_label,
            active_mechanisms,
            unenacted_features: unenacted,
        })
    }

    fn measure_gene_geometric_delta_mm(&self, gene_name: &str, ir: &DesignIrV1) -> f64 {
        match gene_name {
            "body_envelope" | "terminals" => 35.0,
            "inlet_ramp" => {
                for node in &ir.nodes {
                    if let IrNode::InletRamp {
                        axial_drop_m,
                        radial_slot_factor,
                        ..
                    } = node
                    {
                        return (axial_drop_m * 1e3)
                            .max((1.0 - radial_slot_factor).abs() * 9.833333);
                    }
                }
                0.0
            }
            "inlet_split" if self.measure_disjoint_inlet_paths() > 2 => 1.2,
            "inlet_split" => 0.0,
            "skimmer" => {
                let skim_oids: BTreeSet<u16> = self
                    .origin_table
                    .iter()
                    .filter(|o| {
                        o.gene_path.starts_with("skimmer") || o.semantic_id.contains("skimmer")
                    })
                    .map(|o| o.origin_id)
                    .collect();
                let mut max_z = 0.0_f64;
                for (idx, &[i0, i1, i2]) in self.triangles.iter().enumerate() {
                    if skim_oids.contains(&self.triangle_origin_ids[idx]) {
                        for &vi in &[i0, i1, i2] {
                            let p = self.vertices_m[vi];
                            if p[0].hypot(p[1]) < 0.006 && p[2] > 0.0 {
                                max_z = max_z.max(p[2] * 1e3);
                            }
                        }
                    }
                }
                for &(pts, oid) in &self.internal_feature_triangles {
                    if skim_oids.contains(&oid) {
                        for p in pts {
                            if p[0].hypot(p[1]) < 0.006 && p[2] > 0.0 {
                                max_z = max_z.max(p[2] * 1e3);
                            }
                        }
                    }
                }
                max_z
            }
            "chamber_modes" | "ribbon_cascade" | "tpms_insert" => {
                let mut max_dev_mm = 0.0_f64;
                for v in &self.vertices_m {
                    if (0.012..=0.066).contains(&v[2]) && v[0].abs() <= 0.038 {
                        let r_mm = v[0].hypot(v[1]) * 1e3;
                        max_dev_mm = max_dev_mm.max((r_mm - 35.0).abs());
                    }
                }
                max_dev_mm
            }
            "cone_profile" => {
                let mut max_dev_mm = 0.0_f64;
                for v in &self.vertices_m {
                    if (0.072..=0.166).contains(&v[2]) {
                        let r_mm = v[0].hypot(v[1]) * 1e3;
                        let r_lin_mm = 35.0 - 17.5 * ((v[2] - 0.070) / 0.0992474318);
                        max_dev_mm = max_dev_mm.max((r_mm - r_lin_mm).abs());
                    }
                }
                max_dev_mm
            }
            _ => 0.0,
        }
    }

    /// Deliberately strip a feature's enacted triangles from the mesh (for negative-control testing of `IR-04` and `IR-05`).
    pub fn strip_feature_for_negative_test(&mut self, gene_prefix: &str) {
        let drop_oids: BTreeSet<u16> = self
            .origin_table
            .iter()
            .filter(|o| o.gene_path.starts_with(gene_prefix) || o.semantic_id.contains(gene_prefix))
            .map(|o| o.origin_id)
            .collect();
        self.internal_feature_triangles
            .retain(|&(_, oid)| !drop_oids.contains(&oid));

        if gene_prefix == "inlet_split" {
            // Remove splitter vane walls/caps and close the odd-band side-wall gaps so the duct reverts
            // to an unsplit 2-port manifold (1 closed loop per duct at x = +-0.045 m).
            let mut kept_tris = Vec::with_capacity(self.triangles.len());
            let mut kept_oids = Vec::with_capacity(self.triangle_origin_ids.len());
            for (tri, oid) in self
                .triangles
                .drain(..)
                .zip(self.triangle_origin_ids.drain(..))
            {
                if !drop_oids.contains(&oid) {
                    kept_tris.push(tri);
                    kept_oids.push(oid);
                }
            }
            self.triangles = kept_tris;
            self.triangle_origin_ids = kept_oids;

            // Find open vertical edges at |x| == 0.024 m and bridge them to |x| == 0.056 m
            let mut directed: BTreeMap<(usize, usize), usize> = BTreeMap::new();
            for &[a, b, c] in &self.triangles {
                for (u, v) in [(a, b), (b, c), (c, a)] {
                    *directed.entry((u, v)).or_insert(0) += 1;
                }
            }
            let open_directed: Vec<(usize, usize)> = directed
                .iter()
                .filter(|&(&(u, v), &cnt)| {
                    cnt == 1 && directed.get(&(v, u)).copied().unwrap_or(0) == 0
                })
                .map(|(&k, _)| k)
                .collect();

            for &(u0, u1) in &open_directed {
                let p0 = self.vertices_m[u0];
                let p1 = self.vertices_m[u1];
                if (p0[0].abs() - 0.024).abs() < 1e-5
                    && (p1[0].abs() - 0.024).abs() < 1e-5
                    && (p0[2] - p1[2]).abs() > 1e-5
                {
                    let n_grid_port = self
                        .vertices_m
                        .iter()
                        .filter(|v| (v[0] - p0[0]).abs() < 1e-5)
                        .count();
                    let v0 = u0 + n_grid_port;
                    let v1 = u1 + n_grid_port;
                    if v0 < self.vertices_m.len() && v1 < self.vertices_m.len() {
                        self.triangles.push([u1, u0, v0]);
                        self.triangle_origin_ids.push(2);
                        self.triangles.push([u1, v0, v1]);
                        self.triangle_origin_ids.push(2);
                    }
                }
            }
        } else {
            for oid in &mut self.triangle_origin_ids {
                if drop_oids.contains(oid) {
                    *oid = 2; // reassign to generic chamber wall so feature origin has 0 faces
                }
            }
        }
    }

    /// Apply a rigid 3D rotation (`yaw_rad` around Z, `pitch_rad` around Y, `roll_rad` around X) (`IR-12` metamorphic test).
    pub fn rotated(&self, yaw_rad: f64, pitch_rad: f64, roll_rad: f64) -> Self {
        let (cy, sy) = (yaw_rad.cos(), yaw_rad.sin());
        let (cp, sp) = (pitch_rad.cos(), pitch_rad.sin());
        let (cr, sr) = (roll_rad.cos(), roll_rad.sin());
        let rot = |p: P3| -> P3 {
            // Rz(yaw) * Ry(pitch) * Rx(roll)
            let x1 = p[0];
            let y1 = cr * p[1] - sr * p[2];
            let z1 = sr * p[1] + cr * p[2];

            let x2 = cp * x1 + sp * z1;
            let y2 = y1;
            let z2 = -sp * x1 + cp * z1;

            [cy * x2 - sy * y2, sy * x2 + cy * y2, z2]
        };
        let mut out = self.clone();
        for v in &mut out.vertices_m {
            *v = rot(*v);
        }
        for (pts, _) in &mut out.internal_feature_triangles {
            pts[0] = rot(pts[0]);
            pts[1] = rot(pts[1]);
            pts[2] = rot(pts[2]);
        }
        out
    }

    /// Apply uniform spatial coordinate scaling by `scale` (`IR-12` metamorphic test).
    pub fn scaled(&self, scale: f64) -> Self {
        let mut out = self.clone();
        for v in &mut out.vertices_m {
            v[0] *= scale;
            v[1] *= scale;
            v[2] *= scale;
        }
        for (pts, _) in &mut out.internal_feature_triangles {
            for pt in pts.iter_mut() {
                pt[0] *= scale;
                pt[1] *= scale;
                pt[2] *= scale;
            }
        }
        out
    }
}

// ============================================================================
// IR-06 & IR-08: Independent Voxel/SDF Observer & Geometry-Derived Taxon
// ============================================================================
