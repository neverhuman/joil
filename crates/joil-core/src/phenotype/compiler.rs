//! 3D surface mesh compilation and `DesignProgram -> Phenotype3D` compiler.

use super::*;

impl Phenotype3D {
    /// Build a watertight outward-oriented 3D surface mesh of the full fluid domain (or removable head).
    pub fn build_fluid_mesh(
        prog: &DesignProgram,
        n_theta: usize,
        n_z: usize,
    ) -> Result<SolidMesh, String> {
        let p = prog.normalize()?;
        let d = baseline().derive()?;
        let n_theta = n_theta.max(24);
        let n_z = n_z.max(32);

        let mut vertices = Vec::with_capacity((n_z + 1) * n_theta + 2);
        let mut triangles = Vec::with_capacity(n_z * n_theta * 2 + n_theta * 2);

        // Bottom center at z = 0 (index 0) and top center at z = z_underflow (index 1)
        vertices.push([0.0, 0.0, 0.0]);
        vertices.push([0.0, 0.0, d.z_underflow_mm]);

        // Ring vertices along z in [0, z_underflow_mm]
        for iz in 0..=n_z {
            let s_z = iz as f64 / n_z as f64;
            // Cluster axial stations in the head (z in [0, 186.75] mm) for high 3D feature fidelity
            let z_mm = if iz <= (n_z * 3) / 4 {
                let f = iz as f64 / ((n_z * 3) / 4) as f64;
                d.z_head_end_mm * f
            } else {
                let f = (iz - (n_z * 3) / 4) as f64 / (n_z - (n_z * 3) / 4) as f64;
                d.z_head_end_mm + (d.z_underflow_mm - d.z_head_end_mm) * f
            };
            let _ = s_z;
            for it in 0..n_theta {
                let theta = 2.0 * PI * (it as f64) / (n_theta as f64);
                let mut r = Self::inner_radius_mm(&p, &d, theta, z_mm);
                // Include smooth tangential inlet boss blend on z in [0, h] near theta = pi/2 and 3*pi/2
                if z_mm <= d.inlet_height_mm {
                    let boss_env = (PI * (z_mm / d.inlet_height_mm)).sin();
                    let ang1 = (theta - 0.5 * PI).cos().max(0.0).powi(8);
                    let ang2 = (theta - 1.5 * PI).cos().max(0.0).powi(8);
                    let ramp_drop_factor = 1.0 - 0.04 * (p.inlet_ramp.axial_drop_mm / 8.0);
                    r += 6.5
                        * boss_env
                        * (ang1 + ang2)
                        * ramp_drop_factor
                        * p.inlet_ramp.radial_slot_factor;
                }
                vertices.push([r * theta.cos(), r * theta.sin(), z_mm]);
            }
        }

        let ring_start = 2usize;
        // Cap at z = 0 (outward normal -z => clockwise when viewed from +z: 0, next, curr)
        for it in 0..n_theta {
            let next = (it + 1) % n_theta;
            triangles.push([0, ring_start + next, ring_start + it]);
        }

        // Side wall quads split into 2 outward triangles each
        for iz in 0..n_z {
            let r0 = ring_start + iz * n_theta;
            let r1 = ring_start + (iz + 1) * n_theta;
            for it in 0..n_theta {
                let next = (it + 1) % n_theta;
                triangles.push([r0 + it, r0 + next, r1 + next]);
                triangles.push([r0 + it, r1 + next, r1 + it]);
            }
        }

        // Cap at z = z_underflow (outward normal +z => counter-clockwise: 1, curr, next)
        let last_ring = ring_start + n_z * n_theta;
        for it in 0..n_theta {
            let next = (it + 1) % n_theta;
            triangles.push([1, last_ring + it, last_ring + next]);
        }

        Ok(SolidMesh {
            vertices,
            triangles,
        })
    }

    /// Compile `DesignProgram -> (Phenotype3D, SolidMesh)` with exact clearances and volumes.
    pub fn from_program(prog: &DesignProgram) -> Result<(Self, SolidMesh), String> {
        let p = prog.normalize()?;
        let g = baseline();
        let d = g.derive()?;
        let wall_t = g.parameters.wall_thickness_mm; // 3.0 mm nominal outer wall thickness
        let cone_angle_rad = g.parameters.cone_half_angle_deg.to_radians();

        let is_base = p.is_exact_baseline()?;
        let descriptors = p.descriptors()?;
        let mesh = Self::build_fluid_mesh(&p, 48, 64)?;
        let census = mesh.edge_census();

        let mut min_clearance = wall_t;
        let mut min_passage_d = d.overflow_diameter_mm; // 1.4 mm overflow bore
        let mut min_jacobian = 1.0_f64;

        // 1. Chamber wall modes clearance on z in [20, 65] mm
        if !p.chamber_modes.is_empty() {
            for iz in 1..24 {
                let s = iz as f64 / 24.0;
                let env = (PI * s).sin().powi(2);
                for it in 0..96 {
                    let theta = 2.0 * PI * (it as f64) / 96.0;
                    let mut dr = 0.0;
                    let mut ddr_dtheta = 0.0;
                    for m in &p.chamber_modes {
                        let ord = m.azimuthal_order as f64;
                        let ph = ord * theta + m.twist_rad * s;
                        dr += env * (m.cos_mm * ph.cos() + m.sin_mm * ph.sin());
                        ddr_dtheta += env * ord * (-m.cos_mm * ph.sin() + m.sin_mm * ph.cos());
                    }
                    let local_clear = wall_t - dr.max(0.0);
                    min_clearance = min_clearance.min(local_clear);
                    let r_loc = d.d1_mm / 2.0 + dr;
                    let jac =
                        (r_loc / (d.d1_mm / 2.0)) - (ddr_dtheta.abs() / (d.d1_mm / 2.0)) * 0.15;
                    min_jacobian = min_jacobian.min(jac);
                }
            }
        }

        // 2. C2 S-curve reducing cone clearance on z in [72, 166] mm
        let mut delta_cone_vol_mm3 = 0.0;
        let dz_cone = (166.0 - 72.0) / 200.0;
        for k in 0..=200 {
            let s = k as f64 / 200.0;
            let z = 72.0 + (166.0 - 72.0) * s;
            let r_lin = (d.d1_mm / 2.0)
                + ((d.d2_mm - d.d1_mm) / 2.0) * ((z - d.chamber_length_mm) / d.cone_length_mm);
            let env = (PI * s).sin().powi(2);
            let dr = env
                * (p.cone_profile.mode1_mm * (PI * s).sin()
                    + p.cone_profile.mode2_mm * (2.0 * PI * s).sin()
                    + p.cone_profile.mode3_mm * (3.0 * PI * s).sin());
            let normal_clear = (wall_t - dr.max(0.0)) * cone_angle_rad.cos();
            min_clearance = min_clearance.min(normal_clear);
            if k < 200 {
                delta_cone_vol_mm3 += PI * ((r_lin + dr).powi(2) - r_lin.powi(2)) * dz_cone;
            }
        }

        // 3. Inlet ramp & radial slot factor geometry change and passage check
        let mut delta_inlet_vol_mm3 = 0.0;
        if p.inlet_ramp.axial_drop_mm > 1e-6 {
            let wedge_h = (p.inlet_ramp.axial_drop_mm * 0.45).min(3.2);
            if wedge_h > 0.15 {
                // Two triangular ceiling guide ramps (each 24 mm long x w wide x wedge_h high)
                delta_inlet_vol_mm3 -= 2.0 * (0.5 * 24.0 * wedge_h * d.inlet_width_mm);
            }
        }
        if (p.inlet_ramp.radial_slot_factor - 1.0).abs() > 1e-6 {
            let throat_w = d.inlet_width_mm * p.inlet_ramp.radial_slot_factor;
            min_clearance = min_clearance.min(throat_w);
            let dh_throat = 2.0 * throat_w * d.inlet_height_mm / (throat_w + d.inlet_height_mm);
            min_passage_d = min_passage_d.min(dh_throat);
            if p.inlet_ramp.radial_slot_factor < 1.0 {
                let delta_w = d.inlet_width_mm * (1.0 - p.inlet_ramp.radial_slot_factor);
                delta_inlet_vol_mm3 -= 2.0 * (0.5 * 24.0 * delta_w * d.inlet_height_mm);
            }
        }

        // 4. Multi-branch tangential manifold splitter vane clearance & volume
        if p.inlet_split.enabled {
            let vt = p.inlet_split.vane_thickness_mm;
            let n_vanes = (p.inlet_split.branches_per_port.saturating_sub(1)) as f64;
            min_clearance = min_clearance.min(vt);
            let sub_h =
                (d.inlet_height_mm - n_vanes * vt) / (p.inlet_split.branches_per_port as f64);
            let dh_sub = 2.0 * d.inlet_width_mm * sub_h / (d.inlet_width_mm + sub_h);
            min_passage_d = min_passage_d.min(dh_sub);
            delta_inlet_vol_mm3 -= 2.0 * n_vanes * (18.0 * d.inlet_width_mm * vt);
        }

        // 5. Axial vortex-finder skimmer sleeve clearance & volume
        let mut delta_skim_vol_mm3 = 0.0;
        if p.skimmer.length_mm > 0.1 {
            let ro = d.overflow_diameter_mm / 2.0; // 0.7 mm
            let tip_r = (ro + 1.0).max(p.skimmer.outer_radius_mm - 0.4);
            min_clearance = min_clearance.min(tip_r - ro);
            let frustum_outer = PI
                * p.skimmer.length_mm
                * (p.skimmer.outer_radius_mm.powi(2)
                    + p.skimmer.outer_radius_mm * tip_r
                    + tip_r.powi(2))
                / 3.0;
            let cyl_inner = PI * ro * ro * p.skimmer.length_mm;
            delta_skim_vol_mm3 = -(frustum_outer - cyl_inner);
        }

        // 6. Wave-2 Helical guide-ribbon cascade & Peripheral TPMS coalescer shell volume/clearance
        let mut delta_wave2_vol_mm3 = 0.0;
        if !p.ribbon_cascade.is_inactive() {
            min_clearance = min_clearance.min(p.ribbon_cascade.thickness_mm);
            let vane_vol = (p.ribbon_cascade.vane_count as f64)
                * p.ribbon_cascade.axial_span_mm
                * p.ribbon_cascade.radial_chord_mm
                * p.ribbon_cascade.thickness_mm;
            delta_wave2_vol_mm3 -= vane_vol;
        }
        if !p.tpms_insert.is_inactive() {
            min_clearance = min_clearance.min(p.tpms_insert.sheet_thickness_mm);
            let r_outer = d.d1_mm / 2.0;
            let r_inner = r_outer * p.tpms_insert.radial_inner_fraction;
            let span_z = p.tpms_insert.axial_end_mm - p.tpms_insert.axial_start_mm;
            let annulus_vol = PI * (r_outer * r_outer - r_inner * r_inner) * span_z;
            let solid_frac = (p.tpms_insert.sheet_thickness_mm
                / p.tpms_insert.radial_wavelength_mm)
                .clamp(0.10, 0.28);
            delta_wave2_vol_mm3 -= annulus_vol * solid_frac;
            let pore_d =
                (p.tpms_insert.radial_wavelength_mm - p.tpms_insert.sheet_thickness_mm).max(3.5);
            min_passage_d = min_passage_d.min(pore_d);
        }

        let fluid_vol = if is_base {
            d.fluid_volume_mm3
        } else {
            d.fluid_volume_mm3
                + delta_cone_vol_mm3
                + delta_inlet_vol_mm3
                + delta_skim_vol_mm3
                + delta_wave2_vol_mm3
        };

        // Baseline B-rep sharp-corner solid head volume is 128,835.217 mm^3 (570.74 g at 4.43 g/cm^3 Ti-6Al-4V).
        // Any material removed from fluid inside the head cavity adds directly to the solid head volume.
        let baseline_head_vol_mm3 = 128_835.217;
        let head_solid_vol_mm3 = if is_base {
            baseline_head_vol_mm3
        } else {
            baseline_head_vol_mm3
                - (delta_cone_vol_mm3
                    + delta_inlet_vol_mm3
                    + delta_skim_vol_mm3
                    + delta_wave2_vol_mm3)
        };
        let head_mass_ti64_g = (head_solid_vol_mm3 * 1e-3) * 4.43;

        // Lower to canonical DesignIrV1, compile IrCompiledMesh, verify feature enactment (`IR-04`),
        // and run independent VoxelSdfObserver (`IR-06`, `IR-07`, `IR-08`, `IR-09`).
        let ir = crate::geometry::compile_design_ir(&p).map_err(|e| e.to_string())?;
        let ir_mesh =
            joil_ir::IrCompiledMesh::compile(&ir, joil_ir::BackendId::RustMultiPatchCfdSurface)
                .map_err(|e| e.to_string())?;
        let enactment = ir_mesh
            .verify_feature_enactment(&ir)
            .map_err(|e| e.to_string())?;
        let f0_report = joil_ir::VoxelSdfObserver::observe(&ir, &ir_mesh);

        let pheno = Self {
            design_sha256: p.canonical_sha256()?,
            design_ir_sha256: ir.ir_sha256.clone(),
            topology_class: p.topology_class.clone(),
            zero_mutation_exact_baseline: is_base,
            fluid_volume_mm3: fluid_vol,
            head_solid_volume_mm3: head_solid_vol_mm3,
            head_mass_ti64_g,
            head_mass_basis: HEAD_MASS_ESTIMATE_BASIS.to_string(),
            minimum_wall_clearance_mm: min_clearance,
            minimum_passage_diameter_mm: min_passage_d,
            min_section_jacobian: min_jacobian,
            frozen_collar_and_tail_diff_mm3: f0_report.frozen_collar_and_tail_diff_mm3,
            terminal_inlet_area_each_mm2: 0.5
                * (f0_report.terminal_inlet1_area_mm2 + f0_report.terminal_inlet2_area_mm2),
            terminal_overflow_diameter_mm: f0_report.terminal_overflow_diameter_mm,
            terminal_underflow_z_mm: f0_report.terminal_underflow_z_mm,
            trapped_cavities: f0_report.trapped_cavities_total,
            trapped_powder_cavity_observation: f0_report.trapped_powder_cavity_observation.clone(),
            descriptors,
            mesh_census: census,
            derived_taxon: Some(f0_report.derived_taxon),
            observed_geometry: Some(f0_report.observed_geometry),
            split_metrics: Some(f0_report.split_metrics),
            enactment_receipt: Some(enactment),
        };
        Ok((pheno, mesh))
    }
}
