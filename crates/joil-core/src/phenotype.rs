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

/// Mass/solid-volume estimate label (`V5-01-05`).
pub const HEAD_MASS_ESTIMATE_BASIS: &str = "analytic_estimate_v2: baseline B-rep head volume minus closed-form fluid deltas; TPMS credited by annulus solid fraction although the meshed wall carries only a corrugation; not integrated from the accepted solid";

fn legacy_head_mass_basis() -> String {
    "legacy_record_analytic_estimate_basis_unrecorded".to_string()
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
    /// Basis of the two estimates above (`V5-01-05`): not integrated from the accepted solid.
    #[serde(default = "legacy_head_mass_basis")]
    pub head_mass_basis: String,
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
}

#[path = "phenotype/compiler.rs"]
mod compiler;
#[path = "phenotype/surface.rs"]
mod surface;
pub use surface::*;
