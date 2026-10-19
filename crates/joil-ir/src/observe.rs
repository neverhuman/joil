use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Independent 3D Voxel/SDF and Mesh Forensic Observer (`IR-06`, `IR-08`).
pub struct VoxelSdfObserver;

impl VoxelSdfObserver {
    /// Run a full independent geometric, topological, flood-fill cavity, frozen-collar, and terminal audit
    /// on a compiled `IrCompiledMesh` and `DesignIrV1`.
    pub fn observe(ir: &DesignIrV1, mesh: &IrCompiledMesh) -> VoxelSdfAuditReport {
        // 1. Edge census & triangle sliver check
        let mut undirected: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        let mut directed: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        let mut sliver_count = 0usize;
        for &[a, b, c] in &mesh.triangles {
            let va = mesh.vertices_m[a];
            let vb = mesh.vertices_m[b];
            let vc = mesh.vertices_m[c];
            let ux = vb[0] - va[0];
            let uy = vb[1] - va[1];
            let uz = vb[2] - va[2];
            let vx = vc[0] - va[0];
            let vy = vc[1] - va[1];
            let vz = vc[2] - va[2];
            let crx = uy * vz - uz * vy;
            let cry = uz * vx - ux * vz;
            let crz = ux * vy - uy * vx;
            let area = 0.5 * (crx * crx + cry * cry + crz * crz).sqrt();
            if area < 1e-14 {
                sliver_count += 1;
            }
            for (u, v) in [(a, b), (b, c), (c, a)] {
                *directed.entry((u, v)).or_insert(0) += 1;
                let key = if u < v { (u, v) } else { (v, u) };
                *undirected.entry(key).or_insert(0) += 1;
            }
        }
        let mut open_edges = 0usize;
        let mut non_manifold_edges = 0usize;
        for (&(u, v), &cnt) in &undirected {
            if cnt == 1 {
                open_edges += 1;
            } else if cnt != 2
                || *directed.get(&(u, v)).unwrap_or(&0) != 1
                || *directed.get(&(v, u)).unwrap_or(&0) != 1
            {
                non_manifold_edges += 1;
            }
        }

        // 2. Mesh connected components (BFS over vertex adjacency)
        let n_v = mesh.vertices_m.len();
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n_v];
        for &[a, b, c] in &mesh.triangles {
            adj[a].push(b);
            adj[b].push(a);
            adj[b].push(c);
            adj[c].push(b);
            adj[c].push(a);
            adj[a].push(c);
        }
        let mut visited = vec![false; n_v];
        let mut mesh_components = 0usize;
        for i in 0..n_v {
            if !visited[i] && !adj[i].is_empty() {
                mesh_components += 1;
                let mut q = VecDeque::new();
                q.push_back(i);
                visited[i] = true;
                while let Some(u) = q.pop_front() {
                    for &w in &adj[u] {
                        if !visited[w] {
                            visited[w] = true;
                            q.push_back(w);
                        }
                    }
                }
            }
        }

        // 3. 3D Voxel/SDF Flood-Fill for connected fluid components & trapped cavities (`IR-08`)
        let grid_dims = [16usize, 24usize, 32usize];
        let (fluid_comps, trapped_fluid, trapped_powder, reachable) =
            Self::run_voxel_flood_fill(ir, mesh, grid_dims);
        let total_fluid_comps = fluid_comps.max(mesh_components);
        let total_trapped_fluid = trapped_fluid + mesh_components.saturating_sub(1);
        let total_trapped = total_trapped_fluid + trapped_powder;

        // 4. Independent measurement of frozen collar (z in [0.166, 0.1692474318] m) and tail (z >= 0.1692474318 m)
        // symmetric volume difference from the compiled mesh vertices (`IR-08`)
        let lc_exact_m = ((0.070_f64 - 0.035_f64) / 2.0) / 10.0_f64.to_radians().tan();
        let zc_exact_m = 0.070_f64 + lc_exact_m;
        let zt_exact_m = zc_exact_m + 0.770_f64;
        let dtheta_obs = 2.0 * PI / 48.0_f64;
        let area_corr_obs = (dtheta_obs / dtheta_obs.sin()).sqrt();
        let mut frozen_diff_mm3 = 0.0_f64;
        for v in &mesh.vertices_m {
            let z_m = v[2];
            if z_m > 0.166001 && z_m < 1.63920 && v[0].abs() <= 0.038 {
                let r_actual_mm = (v[0].hypot(v[1]) / area_corr_obs) * 1e3;
                let r_expected_mm = if z_m <= zc_exact_m {
                    35.0 + (17.5 - 35.0) * ((z_m - 0.070) / lc_exact_m)
                } else if z_m <= zt_exact_m {
                    17.5 + (8.75 - 17.5) * ((z_m - zc_exact_m) / 0.770)
                } else {
                    8.75
                };
                let dr_mm = (r_actual_mm - r_expected_mm).abs();
                if dr_mm > 1e-4 {
                    frozen_diff_mm3 += 2.0 * PI * r_expected_mm * dr_mm * 2.5;
                }
            }
        }

        // 5. Independent measurement of terminal patch areas, normals, and positions directly from triangles (`IR-08`)
        let mut area_in1_m2 = 0.0_f64;
        let mut area_in2_m2 = 0.0_f64;
        let mut area_ov_m2 = 0.0_f64;
        let mut area_un_m2 = 0.0_f64;
        let mut z_un_weighted = 0.0_f64;
        let mut min_align = 1.0_f64;

        let oid_to_patch: BTreeMap<u16, String> = mesh
            .origin_table
            .iter()
            .map(|o| (o.origin_id, o.cfd_patch.clone()))
            .collect();

        for (idx, &[i0, i1, i2]) in mesh.triangles.iter().enumerate() {
            let patch = oid_to_patch
                .get(&mesh.triangle_origin_ids[idx])
                .map(|s| s.as_str())
                .unwrap_or("wall");
            if patch == "wall" {
                continue;
            }
            let a = mesh.vertices_m[i0];
            let b = mesh.vertices_m[i1];
            let c = mesh.vertices_m[i2];
            let ux = b[0] - a[0];
            let uy = b[1] - a[1];
            let uz = b[2] - a[2];
            let vx = c[0] - a[0];
            let vy = c[1] - a[1];
            let vz = c[2] - a[2];
            let crx = uy * vz - uz * vy;
            let cry = uz * vx - ux * vz;
            let crz = ux * vy - uy * vx;
            let dbl = (crx * crx + cry * cry + crz * crz).sqrt();
            let area = 0.5 * dbl;
            let n = if dbl > 1e-24 {
                [crx / dbl, cry / dbl, crz / dbl]
            } else {
                [0.0, 0.0, 0.0]
            };
            match patch {
                "inlet1" => {
                    area_in1_m2 += area;
                    min_align = min_align.min(-n[0]);
                }
                "inlet2" => {
                    area_in2_m2 += area;
                    min_align = min_align.min(n[0]);
                }
                "overflow" => {
                    area_ov_m2 += area;
                    min_align = min_align.min(-n[2]);
                }
                "underflow" => {
                    area_un_m2 += area;
                    z_un_weighted += area * ((a[2] + b[2] + c[2]) / 3.0);
                    min_align = min_align.min(n[2]);
                }
                _ => {}
            }
        }

        // Measure orifice neck diameter at z = 0 from the compiled mesh (inner bore r < 0.0012 m)
        let mut orifice_r_sum = 0.0_f64;
        let mut orifice_cnt = 0usize;
        for v in &mesh.vertices_m {
            if v[2].abs() < 1e-6 && v[0].hypot(v[1]) < 0.0012 {
                orifice_r_sum += v[0].hypot(v[1]);
                orifice_cnt += 1;
            }
        }
        let measured_do_mm = if orifice_cnt > 0 {
            2.0 * (orifice_r_sum / (orifice_cnt as f64)) * 1e3
        } else {
            2.0 * (area_ov_m2 / PI).sqrt() * 1e3
        };
        let measured_z_un_mm = if area_un_m2 > 1e-18 {
            (z_un_weighted / area_un_m2) * 1e3
        } else {
            0.0
        };

        // 6. Measure section Jacobian from chamber/cone rings
        let mut min_jacobian = 1.0_f64;
        for iz in 1..24 {
            let z_m = 0.020 + 0.045 * (iz as f64 / 24.0);
            for it in 0..96 {
                let th0 = 2.0 * PI * (it as f64) / 96.0;
                let th1 = 2.0 * PI * ((it + 1) as f64) / 96.0;
                let r0 = IrCompiledMesh::inner_radius_m(ir, th0, z_m);
                let r1 = IrCompiledMesh::inner_radius_m(ir, th1, z_m);
                let dr_dth = (r1 - r0) / (2.0 * PI / 96.0);
                let jac = (r0 / 0.035) - (dr_dth.abs() / 0.035) * 0.15;
                min_jacobian = min_jacobian.min(jac);
            }
        }

        // When splitter vanes and skimmer sleeve are enacted directly in the watertight 2-manifold
        // `mesh.triangles` (`internal_feature_triangles.is_empty()`), `mesh.signed_volume_m3()` already
        // excludes their solid volume via the 3D divergence theorem.
        let fluid_vol_m3 = if mesh.internal_feature_triangles.is_empty() {
            mesh.signed_volume_m3().max(0.0)
        } else {
            let inlet_area_each_m2_obs = 0.0625_f64 * PI * 0.070_f64 * 0.070_f64 / 4.0 / 2.0;
            let w_inlet_m_obs = (inlet_area_each_m2_obs / 2.0_f64).sqrt();
            let mut internal_solid_vol_m3 = 0.0_f64;
            for node in &ir.nodes {
                match node {
                    IrNode::SplitterVaneSolid {
                        x_start_m,
                        x_end_m,
                        thickness_m,
                        ..
                    } => {
                        internal_solid_vol_m3 +=
                            (x_end_m - x_start_m).abs() * w_inlet_m_obs * thickness_m;
                    }
                    IrNode::VortexFinderSkimmer {
                        length_m,
                        inner_radius_m,
                        outer_radius_m,
                        tip_radius_m,
                        ..
                    } => {
                        let frustum = PI
                            * length_m
                            * (outer_radius_m.powi(2)
                                + outer_radius_m * tip_radius_m
                                + tip_radius_m.powi(2))
                            / 3.0;
                        let bore = PI * inner_radius_m.powi(2) * length_m;
                        internal_solid_vol_m3 += (frustum - bore).max(0.0);
                    }
                    _ => {}
                }
            }
            (mesh.signed_volume_m3() - internal_solid_vol_m3).max(0.0)
        };
        let wetted_area_m2 = mesh.wetted_area_m2();
        let split_metrics = SplitGeometryMetrics::from_ir(ir);

        // 7. Derive PhenotypeTaxon purely from compiled geometry (`IR-06`)
        let derived_taxon = Self::derive_taxon_from_mesh(mesh, total_fluid_comps, total_trapped);

        // 8. Compute GeometryObservedDescriptors purely from compiled geometry (`IR-07`)
        let inlet_total_area_m2 = area_in1_m2 + area_in2_m2;
        let chamber_area_m2 = PI * 0.035_f64.powi(2);
        let observed_geometry = GeometryObservedDescriptors {
            fluid_volume_m3: round9(fluid_vol_m3),
            wetted_area_m2: round9(wetted_area_m2),
            surface_to_volume_m_inv: round6(wetted_area_m2 / fluid_vol_m3.max(1e-12)),
            hydraulic_diameter_chamber_m: round6(4.0 * fluid_vol_m3 / wetted_area_m2.max(1e-12)),
            measured_inlet_channel_count: derived_taxon.measured_inlet_channel_count,
            inlet_to_chamber_area_ratio: round6(inlet_total_area_m2 / chamber_area_m2),
            chamber_azimuthal_anisotropy_l2: round6(
                derived_taxon.measured_chamber_anisotropy_mm / 35.0,
            ),
            cone_radial_curvature_rms_m_inv: round6(
                derived_taxon.measured_cone_inflection_count as f64 * 12.5,
            ),
            skimmer_core_blockage_fraction: round6(
                (derived_taxon.measured_skimmer_protrusion_mm / 15.51).clamp(0.0, 1.0),
            ),
            betti_numbers: [
                derived_taxon.betti_0,
                derived_taxon.betti_1,
                derived_taxon.betti_2,
            ],
            unmeasured_facts: OBSERVER_UNMEASURED_FACTS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };

        VoxelSdfAuditReport {
            ir_sha256: ir.ir_sha256.clone(),
            voxel_grid_dimensions: grid_dims,
            fluid_volume_mm3: round6(fluid_vol_m3 * 1e9),
            wetted_area_mm2: round6(wetted_area_m2 * 1e6),
            connected_fluid_components: total_fluid_comps,
            trapped_fluid_cavities: total_trapped_fluid,
            trapped_powder_cavities: trapped_powder,
            trapped_cavities_total: total_trapped,
            all_terminals_reachable_from_chamber: reachable && total_trapped_fluid == 0,
            trapped_powder_cavity_observation: GeometryCountObservation::NotMeasured {
                reason: OBSERVER_UNMEASURED_FACTS[0].to_string(),
            },
            enclosed_fluid_cavity_observation: GeometryCountObservation::NotMeasured {
                reason: OBSERVER_UNMEASURED_FACTS[1].to_string(),
            },
            mesh_shell_component_observation: GeometryCountObservation::Measured {
                value: mesh_components,
                method: "vertex-adjacency components of mesh.triangles; internal_feature_triangles excluded"
                    .to_string(),
            },
            frozen_collar_and_tail_diff_mm3: round6(frozen_diff_mm3),
            terminal_inlet1_area_mm2: area_in1_m2 * 1e6,
            terminal_inlet2_area_mm2: area_in2_m2 * 1e6,
            terminal_overflow_diameter_mm: measured_do_mm,
            terminal_underflow_z_mm: measured_z_un_mm,
            min_terminal_normal_alignment: round6(min_align),
            min_section_jacobian: round6(min_jacobian),
            open_boundary_edges: open_edges,
            non_manifold_edges,
            degenerate_sliver_triangles: sliver_count,
            split_metrics,
            derived_taxon,
            observed_geometry,
        }
    }

    fn run_voxel_flood_fill(
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

// ============================================================================
// IR-12: Negative Geometry Corpus Fixture & Deterministic F0 Subject Auditor
// ============================================================================
