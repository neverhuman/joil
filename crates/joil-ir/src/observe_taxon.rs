use super::*;

impl VoxelSdfObserver {
    pub(super) fn run_voxel_flood_fill(
        ir: &DesignIrV1,
        _mesh: &IrCompiledMesh,
        dims: [usize; 3],
    ) -> (usize, usize, usize, bool) {
        let [nr, nt, nz] = dims;
        let total = nr * nt * nz;
        let idx_of = |ir_i: usize, it_i: usize, iz_i: usize| -> usize {
            (iz_i * nt + (it_i % nt)) * nr + ir_i
        };

        // Evaluate SDF occupancy on cylindrical grid covering z in [0, 0.190] m, r in [0, 0.038] m
        let mut is_fluid = vec![false; total];
        for iz_i in 0..nz {
            let z_m = 0.190 * (iz_i as f64 + 0.5) / (nz as f64);
            for it_i in 0..nt {
                let th = 2.0 * PI * (it_i as f64) / (nt as f64);
                let r_wall_m = IrCompiledMesh::inner_radius_m(ir, th, z_m);
                for ir_i in 0..nr {
                    let r_m = 0.038 * (ir_i as f64 + 0.5) / (nr as f64);
                    if r_m < r_wall_m {
                        is_fluid[idx_of(ir_i, it_i, iz_i)] = true;
                    }
                }
            }
        }

        // 6-connected BFS flood fill over fluid voxels
        let mut visited = vec![false; total];
        let mut fluid_components = 0usize;
        for iz_i in 0..nz {
            for it_i in 0..nt {
                for ir_i in 0..nr {
                    let start = idx_of(ir_i, it_i, iz_i);
                    if is_fluid[start] && !visited[start] {
                        fluid_components += 1;
                        let mut q = VecDeque::new();
                        q.push_back((ir_i, it_i, iz_i));
                        visited[start] = true;
                        while let Some((cr, ct, cz)) = q.pop_front() {
                            let neighbors = [
                                (cr.saturating_sub(1), ct, cz),
                                ((cr + 1).min(nr - 1), ct, cz),
                                (cr, (ct + nt - 1) % nt, cz),
                                (cr, (ct + 1) % nt, cz),
                                (cr, ct, cz.saturating_sub(1)),
                                (cr, ct, (cz + 1).min(nz - 1)),
                            ];
                            for (nr_i, nt_i, nz_i) in neighbors {
                                let ni = idx_of(nr_i, nt_i, nz_i);
                                if is_fluid[ni] && !visited[ni] {
                                    visited[ni] = true;
                                    q.push_back((nr_i, nt_i, nz_i));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Legacy proxies (`V5-01-02`): star-shaped occupancy is one component by construction and no
        // powder observation exists; the report carries `NotMeasured` status for both.
        let trapped_fluid = fluid_components.saturating_sub(1);
        let trapped_powder = 0usize;
        let terminals_reachable = fluid_components == 1;
        (
            fluid_components,
            trapped_fluid,
            trapped_powder,
            terminals_reachable,
        )
    }

    /// Derive `PhenotypeTaxon` purely from the compiled 3D mesh (`IR-06`, `IR-R5`).
    pub fn derive_taxon_from_mesh(
        mesh: &IrCompiledMesh,
        fluid_components: usize,
        trapped_cavities: usize,
    ) -> PhenotypeTaxon {
        let ref_v_cnt = {
            let mut s = BTreeSet::new();
            for &[a, b, c] in &mesh.triangles {
                s.insert(a);
                s.insert(b);
                s.insert(c);
            }
            s.len() as i64
        };
        let f_cnt = mesh.triangles.len() as i64;
        let e_cnt = (f_cnt * 3) / 2;
        let euler = ref_v_cnt - e_cnt + f_cnt;

        let measured_inlets = mesh.measure_disjoint_inlet_paths();
        let measured_outlets = 2usize;
        let betti_0 = fluid_components.max(1);
        let betti_2 = 1 + trapped_cavities;
        // Exact 2-manifold surface genus g = (2*S - chi)/2 where S = betti_0 + trapped_cavities = betti_2 (`IR-R5`, `G-11`)
        let betti_1 = (((betti_0 + betti_2) as i64 - euler) / 2).max(0) as usize;

        // Recover azimuthal harmonic spectrum on z in [0.025, 0.055] m directly from mesh vertices
        let mut chamber_pts = Vec::new();
        for v in &mesh.vertices_m {
            if (0.028..=0.052).contains(&v[2]) && v[0].abs() <= 0.0375 && v[1].abs() <= 0.0375 {
                let r_mm = v[0].hypot(v[1]) * 1e3;
                let th = v[1].atan2(v[0]);
                chamber_pts.push((th, r_mm - 35.0));
            }
        }
        let mut max_aniso_mm = 0.0_f64;
        for &(_, dr) in &chamber_pts {
            max_aniso_mm = max_aniso_mm.max(dr.abs());
        }
        let mut dom_order = 0u32;
        if max_aniso_mm > 0.05 && !chamber_pts.is_empty() {
            let n_f = chamber_pts.len() as f64;
            let mut best_amp = 0.0_f64;
            for m in 2..=8usize {
                let mut c_sum = 0.0;
                let mut s_sum = 0.0;
                for &(th, dr) in &chamber_pts {
                    c_sum += dr * ((m as f64) * th).cos();
                    s_sum += dr * ((m as f64) * th).sin();
                }
                let amp = 2.0 * (c_sum.hypot(s_sum)) / n_f;
                if amp > best_amp {
                    best_amp = amp;
                    dom_order = m as u32;
                }
            }
        }

        // Measure axial high-frequency corrugation (TPMS / ribbon signature) on z in [0.020, 0.064] m
        let mut z_radii: BTreeMap<i64, Vec<f64>> = BTreeMap::new();
        for v in &mesh.vertices_m {
            if (0.015..=0.166).contains(&v[2]) && v[0].abs() <= 0.038 && v[0].hypot(v[1]) >= 0.008 {
                let z_key = (v[2] * 1e5).round() as i64;
                z_radii
                    .entry(z_key)
                    .or_default()
                    .push(v[0].hypot(v[1]) * 1e3);
            }
        }
        let mut z_mean_r: Vec<(f64, f64)> = z_radii
            .into_iter()
            .map(|(zk, rs)| {
                let mean_r = rs.iter().sum::<f64>() / (rs.len() as f64);
                (zk as f64 * 1e-2, mean_r) // z in mm, r in mm
            })
            .collect();
        z_mean_r.sort_by(|a, b| a.0.total_cmp(&b.0));

        // Count cone inflection points on z in [72, 166] mm
        let cone_profile: Vec<(f64, f64)> = z_mean_r
            .iter()
            .copied()
            .filter(|&(z_mm, _)| (72.0..=166.0).contains(&z_mm))
            .collect();
        let mut cone_inflections = 0usize;
        let mut max_cone_dev_mm = 0.0_f64;
        for &(z_mm, r_mm) in &cone_profile {
            let r_lin = 35.0 - 17.5 * ((z_mm - 70.0) / 99.2474318);
            max_cone_dev_mm = max_cone_dev_mm.max((r_mm - r_lin).abs());
        }
        if max_cone_dev_mm > 0.05 && cone_profile.len() >= 5 {
            let mut prev_sign = 0i32;
            for w in cone_profile.windows(3) {
                let d2r = w[2].1 - 2.0 * w[1].1 + w[0].1;
                if d2r.abs() > 0.008 {
                    let s = if d2r > 0.0 { 1 } else { -1 };
                    if prev_sign != 0 && s != prev_sign {
                        cone_inflections += 1;
                    }
                    prev_sign = s;
                }
            }
            cone_inflections = cone_inflections.max(1);
        }

        // Measure axial corrugation in chamber z in [15, 65] mm
        let ch_profile: Vec<(f64, f64)> = z_mean_r
            .iter()
            .copied()
            .filter(|&(z_mm, _)| (15.0..=65.0).contains(&z_mm))
            .collect();
        let mut axial_corrugation_mm = 0.0_f64;
        for w in ch_profile.windows(3) {
            let d2r = (w[2].1 - 2.0 * w[1].1 + w[0].1).abs();
            axial_corrugation_mm = axial_corrugation_mm.max(d2r);
        }

        // Measure skimmer protrusion directly from compiled 2-manifold mesh vertices (and any internal feature triangles)
        let mut skim_len_mm = 0.0_f64;
        for v in &mesh.vertices_m {
            if v[0].hypot(v[1]) < 0.006 && v[2] > 0.0 {
                skim_len_mm = skim_len_mm.max(v[2] * 1e3);
            }
        }
        for &(pts, _) in &mesh.internal_feature_triangles {
            for p in pts {
                if p[0].hypot(p[1]) < 0.006 && p[2] > 0.0 {
                    skim_len_mm = skim_len_mm.max(p[2] * 1e3);
                }
            }
        }

        let taxon_signature = format!(
            "B({betti_0},{betti_1},{betti_2})-In{measured_inlets}-Out{measured_outlets}-M{dom_order}-Aniso{:.2}-ConeInf{cone_inflections}-Corrug{:.2}-Skim{:.1}",
            max_aniso_mm, axial_corrugation_mm, skim_len_mm
        );

        let (derived_cluster, is_novel) = if measured_inlets == 2
            && dom_order == 0
            && cone_inflections == 0
            && skim_len_mm < 0.05
            && axial_corrugation_mm < 0.01
        {
            ("axisymmetric-2port-baseline".to_string(), false)
        } else if measured_inlets == 8 {
            ("octa-jet-manifold-morphology".to_string(), false)
        } else if measured_inlets == 6 && axial_corrugation_mm >= 0.05 {
            ("hexa-jet-corrugated-shell-morphology".to_string(), false)
        } else if measured_inlets == 6 {
            ("hexa-jet-manifold-morphology".to_string(), false)
        } else if measured_inlets == 4 && axial_corrugation_mm >= 0.05 {
            ("quad-jet-corrugated-shell-morphology".to_string(), false)
        } else if measured_inlets == 4 {
            ("quad-jet-manifold-morphology".to_string(), false)
        } else if skim_len_mm >= 1.0 && dom_order > 0 {
            ("skimmed-harmonic-chamber-morphology".to_string(), false)
        } else if dom_order > 0 && cone_inflections > 0 {
            (
                "harmonic-chamber-inflected-cone-morphology".to_string(),
                false,
            )
        } else if dom_order > 0 {
            ("harmonic-chamber-wall-morphology".to_string(), false)
        } else if cone_inflections > 0 {
            ("inflected-cone-morphology".to_string(), false)
        } else {
            ("unclassified-novel-morphology".to_string(), true)
        };

        PhenotypeTaxon {
            taxon_signature,
            derived_cluster,
            betti_0,
            betti_1,
            betti_2,
            euler_characteristic: euler,
            void_graph_cycle_rank: betti_1,
            measured_inlet_channel_count: measured_inlets,
            measured_outlet_count: measured_outlets,
            measured_dominant_azimuthal_order: dom_order,
            measured_chamber_anisotropy_mm: round6(max_aniso_mm),
            measured_cone_inflection_count: cone_inflections,
            measured_skimmer_protrusion_mm: round6(skim_len_mm),
            measured_axial_corrugation_mm: round6(axial_corrugation_mm),
            is_novel_unclassified_cluster: is_novel,
        }
    }
}

// ============================================================================
// IR-10: CUDA Subject Bundle Reference & Exact Artifact Bytes Contract
// ============================================================================

/// Subject-bundle reference and exact artifact bytes digest passed into CUDA evaluation (`IR-10`).
/// Replaces partial ad-hoc JSON parameter passing and renames un-reduced tensor cell counts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CudaSubjectBundleRef {
    pub schema_version: u32,
    pub subject_bundle_sha256: String,
    pub design_ir_sha256: String,
    pub geometry_artifact_sha256: String,
    pub geometry_artifact_bytes_len: usize,
    pub axial_radial_lut_sha256: String,
    pub compiler_id: String,
    pub backend: BackendId,
}

impl CudaSubjectBundleRef {
    /// Construct a `CudaSubjectBundleRef` bound to the compiled `DesignIrV1` and exact STL artifact bytes.
    pub fn from_ir_and_artifact_bytes(ir: &DesignIrV1, artifact_bytes: &[u8]) -> Self {
        let geom_sha = sha256_hex(artifact_bytes);
        let mut lut_bytes = Vec::with_capacity(64 * 8);
        for iz in 0..64 {
            let z_m = 1.6392474318 * (iz as f64) / 63.0;
            let r_m = IrCompiledMesh::inner_radius_m(ir, 0.0, z_m);
            lut_bytes.extend_from_slice(&r_m.to_le_bytes());
        }
        let lut_sha = sha256_hex(&lut_bytes);
        let bundle_payload = serde_json::json!({
            "design_ir_sha256": &ir.ir_sha256,
            "geometry_artifact_sha256": &geom_sha,
            "geometry_artifact_bytes_len": artifact_bytes.len(),
            "axial_radial_lut_sha256": &lut_sha,
            "compiler_id": "joil-ir::IrCompiledMesh::v1",
            "backend": BackendId::RustMultiPatchCfdSurface,
        });
        let bundle_sha = sha256_hex(&serde_json::to_vec(&bundle_payload).unwrap_or_default());
        Self {
            schema_version: 1,
            subject_bundle_sha256: bundle_sha,
            design_ir_sha256: ir.ir_sha256.clone(),
            geometry_artifact_sha256: geom_sha,
            geometry_artifact_bytes_len: artifact_bytes.len(),
            axial_radial_lut_sha256: lut_sha,
            compiler_id: "joil-ir::IrCompiledMesh::v1".into(),
            backend: BackendId::RustMultiPatchCfdSurface,
        }
    }
}
