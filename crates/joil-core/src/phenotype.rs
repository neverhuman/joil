//! 3D Hydrocyclone Phenotype Compiler (`DesignProgram -> Phenotype3D + SolidMesh`).
//!
//! Computes exact 3D axial/azimuthal surface radii, internal ramp/manifold/skimmer volumes,
//! section Jacobians, wall/vane/bore clearances, and triangulates watertight outward-oriented
//! 3D surface meshes (`.stl` / `.obj`) in `< 5 ms` per candidate.

use crate::geometry::{baseline, Dimensions};
use crate::program::{DesignProgram, MechanismDescriptors};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub type P3 = [f64; 3];

/// Watertight 3D triangle mesh with topological edge census and signed volume verification.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SolidMesh {
    pub vertices: Vec<P3>,
    pub triangles: Vec<[usize; 3]>,
}

/// Topological edge census `(total_half_edges, boundary_open_edges, non_manifold_edges)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeCensus {
    pub directed_half_edges: usize,
    pub open_boundary_edges: usize,
    pub non_manifold_edges: usize,
}

impl SolidMesh {
    /// Compute signed volume via 3D divergence theorem (`sum(v0 . (v1 x v2)) / 6`).
    /// Positive for outward-pointing counter-clockwise triangles.
    pub fn signed_volume(&self) -> f64 {
        let mut vol6 = 0.0;
        for &[i0, i1, i2] in &self.triangles {
            let a = self.vertices[i0];
            let b = self.vertices[i1];
            let c = self.vertices[i2];
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            vol6 += a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2];
        }
        vol6 / 6.0
    }

    /// Verify watertightness: every undirected edge is shared by exactly 2 oppositely directed half-edges.
    pub fn edge_census(&self) -> EdgeCensus {
        let mut undirected: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        let mut directed: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        let mut total = 0usize;
        for &[a, b, c] in &self.triangles {
            for (u, v) in [(a, b), (b, c), (c, a)] {
                total += 1;
                *directed.entry((u, v)).or_insert(0) += 1;
                let key = if u < v { (u, v) } else { (v, u) };
                *undirected.entry(key).or_insert(0) += 1;
            }
        }
        let mut open = 0usize;
        let mut non_manifold = 0usize;
        for (&(u, v), &cnt) in &undirected {
            if cnt == 1 {
                open += 1;
            } else if cnt != 2 {
                non_manifold += 1;
            } else {
                let fwd = *directed.get(&(u, v)).unwrap_or(&0);
                let rev = *directed.get(&(v, u)).unwrap_or(&0);
                if fwd != 1 || rev != 1 {
                    non_manifold += 1;
                }
            }
        }
        EdgeCensus {
            directed_half_edges: total,
            open_boundary_edges: open,
            non_manifold_edges: non_manifold,
        }
    }

    /// Write ASCII STL file (in declared units, e.g. mm or m).
    pub fn write_stl(&self, path: &Path, solid_name: &str, scale: f64) -> Result<(), String> {
        let file = File::create(path).map_err(|e| e.to_string())?;
        let mut w = BufWriter::new(file);
        writeln!(w, "solid {solid_name}").map_err(|e| e.to_string())?;
        for &[i0, i1, i2] in &self.triangles {
            let a = [
                self.vertices[i0][0] * scale,
                self.vertices[i0][1] * scale,
                self.vertices[i0][2] * scale,
            ];
            let b = [
                self.vertices[i1][0] * scale,
                self.vertices[i1][1] * scale,
                self.vertices[i1][2] * scale,
            ];
            let c = [
                self.vertices[i2][0] * scale,
                self.vertices[i2][1] * scale,
                self.vertices[i2][2] * scale,
            ];
            let ux = b[0] - a[0];
            let uy = b[1] - a[1];
            let uz = b[2] - a[2];
            let vx = c[0] - a[0];
            let vy = c[1] - a[1];
            let vz = c[2] - a[2];
            let nx = uy * vz - uz * vy;
            let ny = uz * vx - ux * vz;
            let nz = ux * vy - uy * vx;
            let norm = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-18);
            writeln!(
                w,
                "  facet normal {:.8e} {:.8e} {:.8e}",
                nx / norm,
                ny / norm,
                nz / norm
            )
            .map_err(|e| e.to_string())?;
            writeln!(w, "    outer loop").map_err(|e| e.to_string())?;
            writeln!(w, "      vertex {:.8e} {:.8e} {:.8e}", a[0], a[1], a[2])
                .map_err(|e| e.to_string())?;
            writeln!(w, "      vertex {:.8e} {:.8e} {:.8e}", b[0], b[1], b[2])
                .map_err(|e| e.to_string())?;
            writeln!(w, "      vertex {:.8e} {:.8e} {:.8e}", c[0], c[1], c[2])
                .map_err(|e| e.to_string())?;
            writeln!(w, "    endloop").map_err(|e| e.to_string())?;
            writeln!(w, "  endfacet").map_err(|e| e.to_string())?;
        }
        writeln!(w, "endsolid {solid_name}").map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Write Wavefront OBJ file for 3D inspection and Blender import.
    pub fn write_obj(&self, path: &Path, scale: f64) -> Result<(), String> {
        let file = File::create(path).map_err(|e| e.to_string())?;
        let mut w = BufWriter::new(file);
        writeln!(w, "# Joil 3D Hydrocyclone Mesh").map_err(|e| e.to_string())?;
        for v in &self.vertices {
            writeln!(
                w,
                "v {:.8} {:.8} {:.8}",
                v[0] * scale,
                v[1] * scale,
                v[2] * scale
            )
            .map_err(|e| e.to_string())?;
        }
        for &[i0, i1, i2] in &self.triangles {
            writeln!(w, "f {} {} {}", i0 + 1, i1 + 1, i2 + 1).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

fn default_mechanism_descriptors() -> MechanismDescriptors {
    MechanismDescriptors {
        swirl_flux_index: 1.0,
        axial_pitch_sin: 0.0,
        azimuthal_symmetry_order: 2.0,
        shear_mitigation_factor: 1.0,
        cone_accel_persistence: 1.0,
        core_shield_ratio: 0.0,
        wall_rifling_stabilization: 0.0,
        hydraulic_resistance_factor: 1.0,
    }
}

/// Complete 3D geometric phenotype and F0 audit metrics for a `DesignProgram`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Phenotype3D {
    pub design_sha256: String,
    #[serde(default)]
    pub design_ir_sha256: String,
    /// Legacy in-memory field retained for internal callers; omitted from serialized subject (`IR-06`).
    #[serde(skip_serializing, default)]
    pub topology_class: String,
    pub zero_mutation_exact_baseline: bool,
    pub fluid_volume_mm3: f64,
    pub head_solid_volume_mm3: f64,
    pub head_mass_ti64_g: f64,
    pub minimum_wall_clearance_mm: f64,
    pub minimum_passage_diameter_mm: f64,
    pub min_section_jacobian: f64,
    pub frozen_collar_and_tail_diff_mm3: f64,
    pub terminal_inlet_area_each_mm2: f64,
    pub terminal_overflow_diameter_mm: f64,
    pub terminal_underflow_z_mm: f64,
    /// Legacy proxy from `VoxelSdfAuditReport::trapped_cavities_total`; not a measured cavity count.
    pub trapped_cavities: usize,
    /// Trapped powder is not observed by the current observer (`V5-01-02`).
    #[serde(default = "joil_ir::GeometryCountObservation::legacy_unrecorded")]
    pub trapped_powder_cavity_observation: joil_ir::GeometryCountObservation,
    /// Legacy heuristic descriptors; omitted from serialized subject (`IR-07`).
    #[serde(skip_serializing, default = "default_mechanism_descriptors")]
    pub descriptors: MechanismDescriptors,
    pub mesh_census: EdgeCensus,
    #[serde(default)]
    pub derived_taxon: Option<joil_ir::PhenotypeTaxon>,
    #[serde(default)]
    pub observed_geometry: Option<joil_ir::GeometryObservedDescriptors>,
    #[serde(default)]
    pub split_metrics: Option<joil_ir::SplitGeometryMetrics>,
    #[serde(default)]
    pub enactment_receipt: Option<joil_ir::FeatureEnactmentReceipt>,
}

impl Phenotype3D {
    /// Evaluate local inner fluid wall radius `R(theta, z)` in mm along the full liner `z in [0, 1639.247]`.
    pub fn inner_radius_mm(prog: &DesignProgram, d: &Dimensions, theta: f64, z_mm: f64) -> f64 {
        let r1 = d.d1_mm / 2.0;
        let r2 = d.d2_mm / 2.0;
        let r3 = d.d3_mm / 2.0;
        let l1 = d.chamber_length_mm;
        let zc = d.z_cone_end_mm;
        let zt = d.z_taper_end_mm;

        let mut r_wall = if z_mm <= l1 {
            if (20.0..=65.0).contains(&z_mm) && !prog.chamber_modes.is_empty() {
                let s = (z_mm - 20.0) / 45.0;
                let env = (PI * s).sin().powi(2);
                let delta: f64 = prog
                    .chamber_modes
                    .iter()
                    .map(|m| {
                        let phase = (m.azimuthal_order as f64) * theta + m.twist_rad * s;
                        m.cos_mm * phase.cos() + m.sin_mm * phase.sin()
                    })
                    .sum();
                r1 + env * delta
            } else {
                r1
            }
        } else if z_mm <= zc {
            let r_lin = r1 + (r2 - r1) * ((z_mm - l1) / d.cone_length_mm);
            if (72.0..=166.0).contains(&z_mm) {
                let s = (z_mm - 72.0) / (166.0 - 72.0);
                let env = (PI * s).sin().powi(2);
                let delta = env
                    * (prog.cone_profile.mode1_mm * (PI * s).sin()
                        + prog.cone_profile.mode2_mm * (2.0 * PI * s).sin()
                        + prog.cone_profile.mode3_mm * (3.0 * PI * s).sin());
                r_lin + delta
            } else {
                r_lin
            }
        } else if z_mm <= zt {
            r2 + (r3 - r2) * ((z_mm - zc) / d.taper_length_mm)
        } else {
            r3
        };

        // Wave-2 Layer B2: Internal helical guide-ribbon cascade on z in [12, 12 + axial_span] mm
        if !prog.ribbon_cascade.is_inactive() {
            let zs = 12.0;
            let ze = (12.0 + prog.ribbon_cascade.axial_span_mm).min(66.0);
            if (zs..=ze).contains(&z_mm) && ze > zs + 1.0 {
                let s = (z_mm - zs) / (ze - zs);
                let env = (PI * s).sin().powi(2);
                let m = prog.ribbon_cascade.vane_count as f64;
                let phase = m * theta + prog.ribbon_cascade.helical_twist_rad * s;
                let vane_profile = phase.cos().max(0.0).powi(4);
                r_wall -= env * (0.35 * prog.ribbon_cascade.radial_chord_mm) * vane_profile;
            }
        }

        // Wave-2 Layer C2: Peripheral TPMS coalescer/rectifier shell modulation
        if !prog.tpms_insert.is_inactive() {
            let zs = prog.tpms_insert.axial_start_mm;
            let ze = prog.tpms_insert.axial_end_mm;
            if (zs..=ze).contains(&z_mm) && ze > zs + 1.0 {
                let s = (z_mm - zs) / (ze - zs);
                let env = (PI * s).sin().powi(2);
                let kz = 2.0 * PI * (z_mm - zs) / prog.tpms_insert.axial_wavelength_mm;
                let kt = 4.0 * theta;
                let tpms_wave = match prog.tpms_insert.basis {
                    crate::program::TpmsBasis::None => 0.0,
                    crate::program::TpmsBasis::Gyroid
                    | crate::program::TpmsBasis::CylindricalGyroid => {
                        0.55 * (kt.sin() * kz.cos() + (kt + kz).sin() * 0.5)
                    }
                    crate::program::TpmsBasis::SchwarzP => 0.50 * (kt.cos() + kz.cos()) * 0.5,
                    crate::program::TpmsBasis::Diamond => 0.50 * (kt.sin() * kz.sin()),
                };
                r_wall += env * (0.45 * prog.tpms_insert.sheet_thickness_mm) * tpms_wave;
            }
        }

        r_wall
    }

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

/// Canonical OpenFOAM CFD boundary patch identifier (`CAD-06`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CfdPatch {
    Wall,
    Inlet1,
    Inlet2,
    Overflow,
    Underflow,
}

impl CfdPatch {
    pub fn as_str(&self) -> &'static str {
        match self {
            CfdPatch::Wall => "wall",
            CfdPatch::Inlet1 => "inlet1",
            CfdPatch::Inlet2 => "inlet2",
            CfdPatch::Overflow => "overflow",
            CfdPatch::Underflow => "underflow",
        }
    }

    pub fn openfoam_type(&self) -> &'static str {
        match self {
            CfdPatch::Wall => "wall",
            _ => "patch",
        }
    }

    pub fn all() -> [CfdPatch; 5] {
        [
            CfdPatch::Wall,
            CfdPatch::Inlet1,
            CfdPatch::Inlet2,
            CfdPatch::Overflow,
            CfdPatch::Underflow,
        ]
    }
}

/// Geometric and orientation audit for a single named CFD boundary patch (`CAD-06`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchMetrics {
    pub patch_name: String,
    pub openfoam_type: String,
    pub triangle_count: usize,
    pub area_m2: f64,
    pub expected_area_m2: Option<f64>,
    pub relative_area_error: Option<f64>,
    pub centroid_m: [f64; 3],
    pub mean_normal: [f64; 3],
    pub expected_normal: Option<[f64; 3]>,
    pub min_normal_alignment: Option<f64>,
    pub plane_axis: Option<usize>,
    pub plane_position_m: Option<f64>,
}

/// Full audit summary for a multi-patch SI CFD surface (`CAD-04` & `CAD-06`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiPatchAuditReport {
    pub design_sha256: String,
    #[serde(default)]
    pub design_ir_sha256: String,
    /// Legacy in-memory field retained for internal callers; omitted from serialized subject (`IR-06`).
    #[serde(skip_serializing, default)]
    pub topology_class: String,
    pub units: String,
    pub outlet_extension_m: f64,
    pub total_vertices: usize,
    pub total_triangles: usize,
    pub signed_volume_m3: f64,
    pub census: EdgeCensus,
    pub watertight_and_manifold: bool,
    pub all_terminal_patches_valid: bool,
    pub patches: BTreeMap<String, PatchMetrics>,
    #[serde(default)]
    pub derived_taxon: Option<joil_ir::PhenotypeTaxon>,
    #[serde(default)]
    pub enactment_receipt: Option<joil_ir::FeatureEnactmentReceipt>,
}

/// Multi-patch watertight 3D fluid domain surface in SI metres (`m`) ready for OpenFOAM `snappyHexMesh`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiPatchSurface {
    pub vertices_m: Vec<P3>,
    pub triangles: Vec<[usize; 3]>,
    pub triangle_patches: Vec<CfdPatch>,
    #[serde(default)]
    pub triangle_origin_ids: Vec<u16>,
    #[serde(default)]
    pub origin_table: Vec<joil_ir::SemanticOriginTrace>,
    #[serde(default)]
    pub internal_feature_triangles: Vec<([P3; 3], u16)>,
    pub audit: MultiPatchAuditReport,
}

impl MultiPatchSurface {
    /// Compile any `DesignProgram` into a conformal, watertight 5-patch SI surface (`wall`, `inlet1`, `inlet2`, `overflow`, `underflow`)
    /// directly from the canonical `joil-ir::IrCompiledMesh` (`IR-R1`).
    pub fn from_program(prog: &DesignProgram, outlet_extension_m: f64) -> Result<Self, String> {
        if !outlet_extension_m.is_finite() || !(0.0..=0.5).contains(&outlet_extension_m) {
            return Err("outlet_extension_m must be finite in [0.0, 0.5] m".into());
        }
        let p = prog.normalize()?;
        let g = baseline();
        let d = g.derive()?;
        let par = &g.parameters;

        let r_plenum_m = par.plenum_radius_mm * 1e-3;
        let r3_m = (d.d3_mm / 2.0) * 1e-3;
        let z_orifice_m =
            -(2.0 * (d.overflow_diameter_mm / 2.0) * par.orifice_length_over_do) * 1e-3;
        let z_overflow_m = z_orifice_m - par.plenum_length_mm * 1e-3;
        let z_underflow_m = d.z_underflow_mm * 1e-3 + outlet_extension_m;
        let x_inlet_m = par.inlet_length_cfd_mm * 1e-3;

        // Compile DesignIrV1 and build the canonical watertight 2-manifold IrCompiledMesh (`IR-02`, `IR-04`, `IR-R1`)
        let ir = crate::geometry::compile_design_ir(&p).map_err(|e| e.to_string())?;
        let ir_mesh = joil_ir::IrCompiledMesh::compile_with_extension(
            &ir,
            joil_ir::BackendId::RustMultiPatchCfdSurface,
            outlet_extension_m,
        )
        .map_err(|e| e.to_string())?;
        let enactment = ir_mesh
            .verify_feature_enactment(&ir)
            .map_err(|e| e.to_string())?;
        let f0_report = joil_ir::VoxelSdfObserver::observe(&ir, &ir_mesh);

        let oid_to_patch: BTreeMap<u16, CfdPatch> = ir_mesh
            .origin_table
            .iter()
            .map(|o| {
                let patch = match o.cfd_patch.as_str() {
                    "inlet1" => CfdPatch::Inlet1,
                    "inlet2" => CfdPatch::Inlet2,
                    "overflow" => CfdPatch::Overflow,
                    "underflow" => CfdPatch::Underflow,
                    _ => CfdPatch::Wall,
                };
                (o.origin_id, patch)
            })
            .collect();

        let vertices_m = ir_mesh.vertices_m;
        let triangles = ir_mesh.triangles;
        let triangle_patches: Vec<CfdPatch> = ir_mesh
            .triangle_origin_ids
            .iter()
            .map(|oid| oid_to_patch.get(oid).copied().unwrap_or(CfdPatch::Wall))
            .collect();

        // Topological watertightness & volume check on the compiled IR mesh
        let combined = SolidMesh {
            vertices: vertices_m.clone(),
            triangles: triangles.clone(),
        };
        let census = combined.edge_census();
        let signed_volume_m3 = combined.signed_volume();
        let watertight_and_manifold = census.open_boundary_edges == 0
            && census.non_manifold_edges == 0
            && signed_volume_m3 > 0.0;

        // Compute per-patch metrics and terminal patch verification (CAD-06)
        let expected_inlet_area_m2 = d.inlet_area_each_mm2 * 1e-6;
        let expected_overflow_area_m2 = PI * r_plenum_m * r_plenum_m;
        let expected_underflow_area_m2 = PI * r3_m * r3_m;

        let mut patches_map = BTreeMap::new();
        let mut all_terminal_patches_valid = true;

        for patch in CfdPatch::all() {
            let mut tri_cnt = 0usize;
            let mut area_sum = 0.0;
            let mut cx = 0.0;
            let mut cy = 0.0;
            let mut cz = 0.0;
            let mut nx_sum = 0.0;
            let mut ny_sum = 0.0;
            let mut nz_sum = 0.0;
            let (exp_area, exp_norm, plane_axis, plane_pos) = match patch {
                CfdPatch::Wall => (None, None, None, None),
                CfdPatch::Inlet1 => (
                    Some(expected_inlet_area_m2),
                    Some([-1.0, 0.0, 0.0]),
                    Some(0usize),
                    Some(-x_inlet_m),
                ),
                CfdPatch::Inlet2 => (
                    Some(expected_inlet_area_m2),
                    Some([1.0, 0.0, 0.0]),
                    Some(0usize),
                    Some(x_inlet_m),
                ),
                CfdPatch::Overflow => (
                    Some(expected_overflow_area_m2),
                    Some([0.0, 0.0, -1.0]),
                    Some(2usize),
                    Some(z_overflow_m),
                ),
                CfdPatch::Underflow => (
                    Some(expected_underflow_area_m2),
                    Some([0.0, 0.0, 1.0]),
                    Some(2usize),
                    Some(z_underflow_m),
                ),
            };
            let mut min_align = 1.0_f64;

            for (idx, &[i0, i1, i2]) in triangles.iter().enumerate() {
                if triangle_patches[idx] != patch {
                    continue;
                }
                tri_cnt += 1;
                let a = vertices_m[i0];
                let b = vertices_m[i1];
                let c = vertices_m[i2];
                let ux = b[0] - a[0];
                let uy = b[1] - a[1];
                let uz = b[2] - a[2];
                let vx = c[0] - a[0];
                let vy = c[1] - a[1];
                let vz = c[2] - a[2];
                let crx = uy * vz - uz * vy;
                let cry = uz * vx - ux * vz;
                let crz = ux * vy - uy * vx;
                let dbl_area = (crx * crx + cry * cry + crz * crz).sqrt();
                let area = 0.5 * dbl_area;
                let unit_n = if dbl_area > 1e-24 {
                    [crx / dbl_area, cry / dbl_area, crz / dbl_area]
                } else {
                    [0.0, 0.0, 0.0]
                };
                area_sum += area;
                cx += area * (a[0] + b[0] + c[0]) / 3.0;
                cy += area * (a[1] + b[1] + c[1]) / 3.0;
                cz += area * (a[2] + b[2] + c[2]) / 3.0;
                nx_sum += area * unit_n[0];
                ny_sum += area * unit_n[1];
                nz_sum += area * unit_n[2];
                if let Some(en) = exp_norm {
                    let dot = unit_n[0] * en[0] + unit_n[1] * en[1] + unit_n[2] * en[2];
                    min_align = min_align.min(dot);
                }
            }

            let centroid_m = if area_sum > 1e-24 {
                [cx / area_sum, cy / area_sum, cz / area_sum]
            } else {
                [0.0, 0.0, 0.0]
            };
            let n_mag = (nx_sum * nx_sum + ny_sum * ny_sum + nz_sum * nz_sum)
                .sqrt()
                .max(1e-24);
            let mean_normal = [nx_sum / n_mag, ny_sum / n_mag, nz_sum / n_mag];
            let rel_err = exp_area.map(|ea| (area_sum / ea) - 1.0);

            if let (Some(re), Some(_)) = (rel_err, exp_norm) {
                if tri_cnt == 0 || re.abs() > 0.002 || min_align < 0.999 {
                    all_terminal_patches_valid = false;
                }
            }

            patches_map.insert(
                patch.as_str().to_string(),
                PatchMetrics {
                    patch_name: patch.as_str().to_string(),
                    openfoam_type: patch.openfoam_type().to_string(),
                    triangle_count: tri_cnt,
                    area_m2: area_sum,
                    expected_area_m2: exp_area,
                    relative_area_error: rel_err,
                    centroid_m,
                    mean_normal,
                    expected_normal: exp_norm,
                    min_normal_alignment: exp_norm.map(|_| min_align),
                    plane_axis,
                    plane_position_m: plane_pos,
                },
            );
        }

        let audit = MultiPatchAuditReport {
            design_sha256: p.canonical_sha256()?,
            design_ir_sha256: ir.ir_sha256.clone(),
            topology_class: p.topology_class,
            units: "m".to_string(),
            outlet_extension_m,
            total_vertices: vertices_m.len(),
            total_triangles: triangles.len(),
            signed_volume_m3,
            census,
            watertight_and_manifold,
            all_terminal_patches_valid,
            patches: patches_map,
            derived_taxon: Some(f0_report.derived_taxon),
            enactment_receipt: Some(enactment),
        };

        Ok(Self {
            vertices_m,
            triangles,
            triangle_patches,
            triangle_origin_ids: ir_mesh.triangle_origin_ids,
            origin_table: ir_mesh.origin_table,
            internal_feature_triangles: ir_mesh.internal_feature_triangles,
            audit,
        })
    }

    /// Write multi-region ASCII STL (`solid wall ... endsolid wall`, `solid inlet1 ...`, etc.)
    /// directly compatible with OpenFOAM `snappyHexMesh` multi-region feature snapping.
    pub fn write_openfoam_ascii_stl(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = File::create(path).map_err(|e| e.to_string())?;
        let mut w = BufWriter::new(file);

        let write_facet = |w: &mut BufWriter<File>, a: P3, b: P3, c: P3| -> Result<(), String> {
            let ux = b[0] - a[0];
            let uy = b[1] - a[1];
            let uz = b[2] - a[2];
            let vx = c[0] - a[0];
            let vy = c[1] - a[1];
            let vz = c[2] - a[2];
            let nx = uy * vz - uz * vy;
            let ny = uz * vx - ux * vz;
            let nz = ux * vy - uy * vx;
            let norm = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-24);
            writeln!(
                w,
                " facet normal {:.8e} {:.8e} {:.8e}",
                nx / norm,
                ny / norm,
                nz / norm
            )
            .map_err(|e| e.to_string())?;
            writeln!(w, "  outer loop").map_err(|e| e.to_string())?;
            for v in [a, b, c] {
                writeln!(w, "   vertex {:.8e} {:.8e} {:.8e}", v[0], v[1], v[2])
                    .map_err(|e| e.to_string())?;
            }
            writeln!(w, "  endloop\n endfacet").map_err(|e| e.to_string())?;
            Ok(())
        };

        for patch in CfdPatch::all() {
            let name = patch.as_str();
            writeln!(w, "solid {name}").map_err(|e| e.to_string())?;
            for (idx, &[i0, i1, i2]) in self.triangles.iter().enumerate() {
                if self.triangle_patches[idx] != patch {
                    continue;
                }
                write_facet(
                    &mut w,
                    self.vertices_m[i0],
                    self.vertices_m[i1],
                    self.vertices_m[i2],
                )?;
            }
            if patch == CfdPatch::Wall {
                for &(pts, _) in &self.internal_feature_triangles {
                    write_facet(&mut w, pts[0], pts[1], pts[2])?;
                }
            }
            writeln!(w, "endsolid {name}").map_err(|e| e.to_string())?;
        }
        w.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}
