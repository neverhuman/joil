//! CAD: fluid-domain and printed-part meshes of a developed cartridge.
//!
//! The fluid domain is the union of chambers, jets, plenums, beds and channels; the printed
//! part is the shell `0 < d ≤ wall` around it, cut open at the feed, reject and product
//! ports. Both are meshed by marching tetrahedra on the same signed-distance samples and
//! audited independently (closure, orientation, volume, components, genus).

pub mod audit;
pub mod export;
pub mod mesh;
pub mod model;
pub mod print;
pub mod sdf;

use crate::campaign::Campaign;
use crate::develop::{develop, Phenotype, TI_DENSITY};
use crate::genome::Genome;
use crate::layout::Hardware;
use audit::MeshAudit;
use export::{write_glb, write_stl, GlbPart};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CadReport {
    pub genome_sha256: String,
    pub voxel_mm: f64,
    pub wall_mm: f64,
    pub length_mm: f64,
    pub envelope_radius_mm: f64,
    pub fluid: MeshAudit,
    pub part: MeshAudit,
    pub fluid_volume_estimate_mm3: f64,
    pub fluid_volume_rel_diff: f64,
    pub printed_mass_kg: f64,
    pub lattice_represented: bool,
    /// Smallest designed passage (jet, orifice, channel; lattice strut when meshed), mm.
    pub min_feature_mm: f64,
    /// Under-sampling warnings: a wall or passage spanning fewer than two voxels can
    /// pinch shut or open spurious holes, which changes the mesh topology.
    pub resolution_warnings: Vec<String>,
    pub print: print::PrintAudit,
    pub geometry_violations: Vec<String>,
    pub note: String,
}

/// Smallest designed passage of a developed design (mm).
pub fn min_feature_mm(ph: &Phenotype) -> f64 {
    let mut m = f64::INFINITY;
    for u in &ph.layout.units {
        if let Hardware::Cyclone {
            jet_w,
            jet_h,
            orifice_d,
            ..
        } = &u.hw
        {
            m = m.min(*jet_w).min(*jet_h).min(*orifice_d);
        }
    }
    for ch in &ph.layout.channels {
        m = m.min(ch.d);
    }
    m
}

/// Voxel that puts at least two samples across the wall and the smallest passage,
/// bounded to [0.5, 1.0] mm to keep meshes tractable (warnings record any shortfall).
pub fn auto_voxel(g: &Genome, c: &Campaign) -> Result<f64, String> {
    let ph = develop(g, &c.envelope)?;
    Ok((0.5 * c.envelope.wall_mm)
        .min(0.5 * min_feature_mm(&ph))
        .clamp(0.5, 1.0))
}

pub struct Meshed {
    pub phenotype: Phenotype,
    pub fluid: mesh::Mesh,
    pub part: mesh::Mesh,
    pub report: CadReport,
}

pub fn build(g: &Genome, c: &Campaign, voxel: f64) -> Result<Meshed, String> {
    if !(0.2..=5.0).contains(&voxel) {
        return Err("voxel size must lie in [0.2, 5] mm".into());
    }
    let ph = develop(g, &c.envelope)?;
    let model = model::build(&ph, voxel);
    let (fluid, part) = mesh::mesh(&model, voxel);
    let fa = audit::audit(&fluid);
    let pa = audit::audit(&part);
    let est = ph.stats.fluid_volume_mm3;
    let min_feature = min_feature_mm(&ph);
    let mut resolution_warnings = Vec::new();
    for (what, size) in [("wall", model.wall), ("smallest passage", min_feature)] {
        if size < 2.0 * voxel {
            resolution_warnings.push(format!(
                "{what} {size:.2} mm spans {:.1} voxels (< 2): mesh topology may be wrong there",
                size / voxel
            ));
        }
    }
    let print = print::audit(&model, &fluid, c.envelope.length_mm, voxel);
    let report = CadReport {
        genome_sha256: g.canonical_sha256(),
        voxel_mm: voxel,
        wall_mm: model.wall,
        length_mm: model.length,
        envelope_radius_mm: model.radius,
        fluid_volume_estimate_mm3: est,
        fluid_volume_rel_diff: (fa.volume_mm3 - est) / est.max(1e-9),
        printed_mass_kg: pa.volume_mm3 * 1e-9 * TI_DENSITY,
        fluid: fa,
        part: pa,
        lattice_represented: model.lattice,
        min_feature_mm: min_feature,
        resolution_warnings,
        print,
        geometry_violations: ph.violations.clone(),
        note: "Geometry observations only; rendering and meshing are not validation. The \
               volume estimate is the layout's analytic sum, which double-counts overlaps."
            .into(),
    };
    Ok(Meshed {
        phenotype: ph,
        fluid,
        part,
        report,
    })
}

/// Write `fluid.stl`, `part.stl`, `cartridge.glb`, `cad-audit.json`, `genome.json`.
pub fn export(g: &Genome, c: &Campaign, out: &Path, voxel: f64) -> Result<Vec<PathBuf>, String> {
    let m = build(g, c, voxel)?;
    write_outputs(g, &m, out)
}

pub fn write_outputs(g: &Genome, m: &Meshed, out: &Path) -> Result<Vec<PathBuf>, String> {
    let files = vec![
        out.join("fluid.stl"),
        out.join("part.stl"),
        out.join("cartridge.glb"),
        out.join("cad-audit.json"),
        out.join("genome.json"),
    ];
    write_stl(&files[0], &m.fluid, "fluid domain")?;
    write_stl(&files[1], &m.part, "printed part")?;
    write_glb(
        &files[2],
        &[
            GlbPart {
                name: "printed part",
                mesh: &m.part,
                base: [0.78, 0.80, 0.84, 0.28],
                vertex_colors: false,
                metallic: 0.6,
            },
            GlbPart {
                name: "fluid domain",
                mesh: &m.fluid,
                base: [1.0, 1.0, 1.0, 1.0],
                vertex_colors: true,
                metallic: 0.0,
            },
        ],
        m.report.length_mm,
    )?;
    std::fs::write(
        &files[3],
        serde_json::to_vec_pretty(&m.report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        &files[4],
        serde_json::to_vec_pretty(g).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(files)
}
