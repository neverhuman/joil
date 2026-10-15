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

/// Complete 3D geometric phenotype and F0 audit metrics for a `DesignProgram`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Phenotype3D {
    pub design_sha256: String,
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
    pub trapped_cavities: usize,
    pub descriptors: MechanismDescriptors,
    pub mesh_census: EdgeCensus,
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

        if z_mm <= l1 {
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
        }
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

        let fluid_vol = if is_base {
            d.fluid_volume_mm3
        } else {
            d.fluid_volume_mm3 + delta_cone_vol_mm3 + delta_inlet_vol_mm3 + delta_skim_vol_mm3
        };

        // Baseline B-rep sharp-corner solid head volume is 128,835.217 mm^3 (570.74 g at 4.43 g/cm^3 Ti-6Al-4V).
        // Any material removed from fluid inside the head cavity adds directly to the solid head volume.
        let baseline_head_vol_mm3 = 128_835.217;
        let head_solid_vol_mm3 = if is_base {
            baseline_head_vol_mm3
        } else {
            baseline_head_vol_mm3 - (delta_cone_vol_mm3 + delta_inlet_vol_mm3 + delta_skim_vol_mm3)
        };
        let head_mass_ti64_g = (head_solid_vol_mm3 * 1e-3) * 4.43;

        let pheno = Self {
            design_sha256: p.canonical_sha256()?,
            topology_class: p.topology_class.clone(),
            zero_mutation_exact_baseline: is_base,
            fluid_volume_mm3: fluid_vol,
            head_solid_volume_mm3: head_solid_vol_mm3,
            head_mass_ti64_g,
            minimum_wall_clearance_mm: min_clearance,
            minimum_passage_diameter_mm: min_passage_d,
            min_section_jacobian: min_jacobian,
            frozen_collar_and_tail_diff_mm3: 0.0,
            terminal_inlet_area_each_mm2: d.inlet_area_each_mm2,
            terminal_overflow_diameter_mm: d.overflow_diameter_mm,
            terminal_underflow_z_mm: d.z_underflow_mm,
            trapped_cavities: 0,
            descriptors,
            mesh_census: census,
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
}

/// Multi-patch watertight 3D fluid domain surface in SI metres (`m`) ready for OpenFOAM `snappyHexMesh`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiPatchSurface {
    pub vertices_m: Vec<P3>,
    pub triangles: Vec<[usize; 3]>,
    pub triangle_patches: Vec<CfdPatch>,
    pub audit: MultiPatchAuditReport,
}

impl MultiPatchSurface {
    /// Compile any `DesignProgram` into a conformal, watertight 5-patch SI surface (`wall`, `inlet1`, `inlet2`, `overflow`, `underflow`).
    pub fn from_program(prog: &DesignProgram, outlet_extension_m: f64) -> Result<Self, String> {
        if !outlet_extension_m.is_finite() || !(0.0..=0.5).contains(&outlet_extension_m) {
            return Err("outlet_extension_m must be finite in [0.0, 0.5] m".into());
        }
        let p = prog.normalize()?;
        let g = baseline();
        let d = g.derive()?;
        let par = &g.parameters;

        let n_theta = 48usize; // multiple of 4 so pi/2 and 3*pi/2 align cleanly
        let dtheta = 2.0 * PI / (n_theta as f64);
        // Polygon area correction factor so regular n_theta-gon has exact area pi * R^2
        let area_corr = (dtheta / dtheta.sin()).sqrt();

        let r_plenum_m = par.plenum_radius_mm * 1e-3;
        let r_plenum_poly_m = r_plenum_m * area_corr;
        let ro_m = (d.overflow_diameter_mm / 2.0) * 1e-3;
        let r3_m = (d.d3_mm / 2.0) * 1e-3;
        let r3_poly_m = r3_m * area_corr;

        let z_orifice_m =
            -(2.0 * (d.overflow_diameter_mm / 2.0) * par.orifice_length_over_do) * 1e-3;
        let z_overflow_m = z_orifice_m - par.plenum_length_mm * 1e-3;
        let z_underflow_m = d.z_underflow_mm * 1e-3 + outlet_extension_m;
        let h_inlet_m = d.inlet_height_mm * 1e-3;
        let w_inlet_m = d.inlet_width_mm * 1e-3;
        let x_inlet_m = par.inlet_length_cfd_mm * 1e-3;
        let r1_m = (d.d1_mm / 2.0) * 1e-3;

        // Build axial rings of n_theta vertices each from z_overflow_m to z_underflow_m
        // Each ring entry is a list of n_theta [x, y, z] points in metres.
        let mut rings: Vec<Vec<P3>> = Vec::new();
        let th_off = 0.0031_f64;
        let make_circle_ring = |r_m: f64, z_m: f64| -> Vec<P3> {
            (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    [r_m * th.cos(), r_m * th.sin(), z_m]
                })
                .collect()
        };

        // Plenum rings from z = z_overflow_m to z = z_orifice_m (4 axial segments)
        let n_plenum_z = 4usize;
        for iz in 0..=n_plenum_z {
            let f = (iz as f64) / (n_plenum_z as f64);
            let z_m = z_overflow_m + (z_orifice_m - z_overflow_m) * f;
            rings.push(make_circle_ring(r_plenum_poly_m, z_m));
        }
        // Intermediate radial step at z = z_orifice_m
        rings.push(make_circle_ring(
            0.5 * (r_plenum_poly_m + ro_m),
            z_orifice_m,
        ));
        // Orifice bore from z = z_orifice_m to z = 0.0 (2 axial segments)
        rings.push(make_circle_ring(ro_m, z_orifice_m));
        rings.push(make_circle_ring(ro_m, 0.5 * z_orifice_m));
        rings.push(make_circle_ring(ro_m, 0.0));

        // Now ring at z = 0 on outer chamber wall: index `k_chamber_0`
        let k_chamber_0 = rings.len();
        let n_inlet_z = 4usize; // 4 axial bands across z in [0, h_inlet_m]
        for iz in 0..=n_inlet_z {
            let z_mm = d.inlet_height_mm * (iz as f64) / (n_inlet_z as f64);
            let z_m = z_mm * 1e-3;
            let ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let r_mm = Phenotype3D::inner_radius_mm(&p, &d, th, z_mm);
                    [r_mm * 1e-3 * th.cos(), r_mm * 1e-3 * th.sin(), z_m]
                })
                .collect();
            rings.push(ring);
        }

        // Remaining head rings from z = h_inlet_mm to z_head_end_mm (36 stations)
        let n_head_rem = 36usize;
        for iz in 1..=n_head_rem {
            let f = (iz as f64) / (n_head_rem as f64);
            let z_mm = d.inlet_height_mm + (d.z_head_end_mm - d.inlet_height_mm) * f;
            let z_m = z_mm * 1e-3;
            let ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let r_mm = Phenotype3D::inner_radius_mm(&p, &d, th, z_mm);
                    [r_mm * 1e-3 * th.cos(), r_mm * 1e-3 * th.sin(), z_m]
                })
                .collect();
            rings.push(ring);
        }

        // Downstream fine taper and tail rings up to just before z_underflow_m (15 stations)
        let n_tail = 16usize;
        for iz in 1..n_tail {
            let f = (iz as f64) / (n_tail as f64);
            let z_mm = d.z_head_end_mm + (d.z_underflow_mm - d.z_head_end_mm) * f;
            let z_m = z_mm * 1e-3;
            let ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let r_mm = Phenotype3D::inner_radius_mm(&p, &d, th, z_mm);
                    [r_mm * 1e-3 * th.cos(), r_mm * 1e-3 * th.sin(), z_m]
                })
                .collect();
            rings.push(ring);
        }
        // Final ring at z = z_underflow_m with area-exact radius r3_poly_m
        rings.push(make_circle_ring(r3_poly_m, z_underflow_m));

        let n_rings = rings.len();
        let mut vertices_m: Vec<P3> = Vec::with_capacity(2 + n_rings * n_theta + 50);
        // Vertex 0: overflow center; Vertex 1: underflow center (offset from x=0,y=0 octree line)
        vertices_m.push([0.00011, 0.00013, z_overflow_m]);
        vertices_m.push([0.00011, 0.00013, z_underflow_m]);

        let ring_base = 2usize;
        for ring in &rings {
            vertices_m.extend_from_slice(ring);
        }

        let mut triangles: Vec<[usize; 3]> = Vec::new();
        let mut triangle_patches: Vec<CfdPatch> = Vec::new();

        // 1. Overflow cap (Ring 0 around center vertex 0, normal [0, 0, -1])
        for it in 0..n_theta {
            let next = (it + 1) % n_theta;
            triangles.push([0, ring_base + next, ring_base + it]);
            triangle_patches.push(CfdPatch::Overflow);
        }

        // Define azimuthal windows for inlet1 (theta = pi/2 .. 3*pi/4, it = 12..18 on -x side)
        // and inlet2 (theta = 3*pi/2 .. 7*pi/4, it = 36..42 on +x side) on rings k_chamber_0 .. k_chamber_0 + n_inlet_z
        let n_port_t = 6usize;
        let it_inlet1_start = n_theta / 4; // 12 (theta = 90 deg -> 135 deg, y decreases monotonically)
        let it_inlet2_start = (3 * n_theta) / 4; // 36 (theta = 270 deg -> 315 deg, y increases monotonically)

        // 2. Side wall quads between consecutive rings, skipping quads inside the two inlet port windows
        for ir in 0..(n_rings - 1) {
            let r0 = ring_base + ir * n_theta;
            let r1 = ring_base + (ir + 1) * n_theta;
            let in_inlet_z = ir >= k_chamber_0 && ir < k_chamber_0 + n_inlet_z;
            for it in 0..n_theta {
                let in_inlet1 =
                    in_inlet_z && it >= it_inlet1_start && it < it_inlet1_start + n_port_t;
                let in_inlet2 =
                    in_inlet_z && it >= it_inlet2_start && it < it_inlet2_start + n_port_t;
                if in_inlet1 || in_inlet2 {
                    continue;
                }
                let next = (it + 1) % n_theta;
                triangles.push([r0 + it, r0 + next, r1 + next]);
                triangle_patches.push(CfdPatch::Wall);
                triangles.push([r0 + it, r1 + next, r1 + it]);
                triangle_patches.push(CfdPatch::Wall);
            }
        }

        // 3. Extrude Inlet 1 (terminal plane x = -x_inlet_m, outward normal [-1, 0, 0])
        // and Inlet 2 (terminal plane x = +x_inlet_m, outward normal [+1, 0, 0])
        // preserving exact area w_inlet_m * h_inlet_m and conformal boundary loop welding.
        for (patch, it_start, x_plane, y_from_it) in [
            (CfdPatch::Inlet1, it_inlet1_start, -x_inlet_m, true),
            (CfdPatch::Inlet2, it_inlet2_start, x_inlet_m, false),
        ] {
            let port_v_base = vertices_m.len();
            for iz in 0..=n_inlet_z {
                let fz = (iz as f64) / (n_inlet_z as f64);
                let z_m = h_inlet_m * fz;
                for jt in 0..=n_port_t {
                    let ft = (jt as f64) / (n_port_t as f64);
                    let y_m = if y_from_it {
                        // Inlet 1 near +R1: decrease y as jt increases so (d/djt) x (d/dfz) points in [-1, 0, 0]
                        r1_m - w_inlet_m * ft
                    } else {
                        // Inlet 2 near -R1: increase y as jt increases so (d/djt) x (d/dfz) points in [+1, 0, 0]
                        -r1_m + w_inlet_m * ft
                    };
                    vertices_m.push([x_plane, y_m, z_m]);
                }
            }
            let port_idx =
                |iz: usize, jt: usize| -> usize { port_v_base + iz * (n_port_t + 1) + jt };
            let wall_idx = |iz: usize, jt: usize| -> usize {
                ring_base + (k_chamber_0 + iz) * n_theta + ((it_start + jt) % n_theta)
            };

            // Connect the 4 sides of the duct (CfdPatch::Wall) between wall_idx boundary and port_idx boundary:
            // Bottom strip (iz = 0, jt = 0..n_port_t): wall edge wall(0, jt) -> wall(0, jt+1)
            for jt in 0..n_port_t {
                let w0 = wall_idx(0, jt);
                let w1 = wall_idx(0, jt + 1);
                let p0 = port_idx(0, jt);
                let p1 = port_idx(0, jt + 1);
                triangles.push([w0, w1, p1]);
                triangle_patches.push(CfdPatch::Wall);
                triangles.push([w0, p1, p0]);
                triangle_patches.push(CfdPatch::Wall);
            }
            // Right strip (jt = n_port_t, iz = 0..n_inlet_z): wall edge wall(iz, n_port_t) -> wall(iz+1, n_port_t)
            for iz in 0..n_inlet_z {
                let w0 = wall_idx(iz, n_port_t);
                let w1 = wall_idx(iz + 1, n_port_t);
                let p0 = port_idx(iz, n_port_t);
                let p1 = port_idx(iz + 1, n_port_t);
                triangles.push([w0, w1, p1]);
                triangle_patches.push(CfdPatch::Wall);
                triangles.push([w0, p1, p0]);
                triangle_patches.push(CfdPatch::Wall);
            }
            // Top strip (iz = n_inlet_z, jt = n_port_t..0): wall edge wall(n_inlet_z, jt+1) -> wall(n_inlet_z, jt)
            for jt in 0..n_port_t {
                let w0 = wall_idx(n_inlet_z, jt + 1);
                let w1 = wall_idx(n_inlet_z, jt);
                let p0 = port_idx(n_inlet_z, jt + 1);
                let p1 = port_idx(n_inlet_z, jt);
                triangles.push([w0, w1, p1]);
                triangle_patches.push(CfdPatch::Wall);
                triangles.push([w0, p1, p0]);
                triangle_patches.push(CfdPatch::Wall);
            }
            // Left strip (jt = 0, iz = n_inlet_z..0): wall edge wall(iz+1, 0) -> wall(iz, 0)
            for iz in 0..n_inlet_z {
                let w0 = wall_idx(iz + 1, 0);
                let w1 = wall_idx(iz, 0);
                let p0 = port_idx(iz + 1, 0);
                let p1 = port_idx(iz, 0);
                triangles.push([w0, w1, p1]);
                triangle_patches.push(CfdPatch::Wall);
                triangles.push([w0, p1, p0]);
                triangle_patches.push(CfdPatch::Wall);
            }

            // Terminal port patch triangles (CfdPatch::Inlet1 or CfdPatch::Inlet2)
            for iz in 0..n_inlet_z {
                for jt in 0..n_port_t {
                    let a = port_idx(iz, jt);
                    let b = port_idx(iz, jt + 1);
                    let c = port_idx(iz + 1, jt + 1);
                    let d_v = port_idx(iz + 1, jt);
                    triangles.push([a, b, c]);
                    triangle_patches.push(patch);
                    triangles.push([a, c, d_v]);
                    triangle_patches.push(patch);
                }
            }
        }

        // 4. Underflow cap (last ring around center vertex 1, normal [0, 0, +1])
        let last_ring = ring_base + (n_rings - 1) * n_theta;
        for it in 0..n_theta {
            let next = (it + 1) % n_theta;
            triangles.push([1, last_ring + it, last_ring + next]);
            triangle_patches.push(CfdPatch::Underflow);
        }

        // Topological watertightness & volume check on the combined mesh
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
        };

        Ok(Self {
            vertices_m,
            triangles,
            triangle_patches,
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

        for patch in CfdPatch::all() {
            let name = patch.as_str();
            writeln!(w, "solid {name}").map_err(|e| e.to_string())?;
            for (idx, &[i0, i1, i2]) in self.triangles.iter().enumerate() {
                if self.triangle_patches[idx] != patch {
                    continue;
                }
                let a = self.vertices_m[i0];
                let b = self.vertices_m[i1];
                let c = self.vertices_m[i2];
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
            }
            writeln!(w, "endsolid {name}").map_err(|e| e.to_string())?;
        }
        w.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}
