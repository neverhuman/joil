//! Native Rust Binary glTF 2.0 (`.glb`) 3D Scene Exporter & Resource Lease Gate
//! (`P09`, `OPS_VIEW-01`..`OPS_VIEW-04`).
//!
//! Implements:
//! - `OPS_VIEW-01` (`verify_resource_lease`): Enforces physical CPU core vs. SMT thread mapping,
//!   RAM ceiling, and shared output scratch ceiling (`<= 4` physical cores, `<= 12 GiB` RAM,
//!   `<= 5 GiB` outputs unless explicitly superseded by a user-authorized allocation).
//! - `OPS_VIEW-02` & `OPS_VIEW-03` (`export_glb_scene_bundle`): Zero-dependency native Rust binary
//!   glTF 2.0 (`.glb`) exporter from any `MultiPatchSurface` / `SolidMesh` in true SI metre scale
//!   (`1.0`), preserving exact coordinate frames, camera presets, and SHA-256 bindings to the
//!   governing geometry/physics digests.

use crate::phenotype::MultiPatchSurface;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// `OPS_VIEW-01`: Physical CPU core, memory, and scratch disk lease audit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLeaseAudit {
    pub lease_id: String,
    pub requested_logical_cpus: Vec<usize>,
    pub physical_cores_used: usize,
    pub max_allowed_physical_cores: usize,
    pub memory_gib: usize,
    pub max_allowed_memory_gib: usize,
    pub scratch_bytes_used: u64,
    pub max_allowed_scratch_bytes: u64,
    pub smt_mistaken_for_physical_rejected: bool,
    pub admitted: bool,
}

/// Verify resource lease admission (`OPS_VIEW-01`).
/// On dual-thread SMT topologies where `(cpu_id, cpu_id + physical_total)` share a core,
/// rejects requests that mistake SMT sibling threads for independent physical cores or exceed limits.
pub fn verify_resource_lease(
    lease_id: &str,
    requested_logical_cpus: &[usize],
    claimed_physical_cores: usize,
    total_physical_cores_on_host: usize,
    memory_gib: usize,
    scratch_bytes_used: u64,
) -> Result<ResourceLeaseAudit, String> {
    if requested_logical_cpus.is_empty() || total_physical_cores_on_host == 0 {
        return Err("non-empty CPU list and positive host physical core count required".into());
    }
    let mut physical_ids: Vec<usize> = requested_logical_cpus
        .iter()
        .map(|&cid| cid % total_physical_cores_on_host)
        .collect();
    physical_ids.sort_unstable();
    physical_ids.dedup();
    let actual_physical = physical_ids.len();

    if claimed_physical_cores != actual_physical {
        return Err(format!(
            "OPS_VIEW-01 negative control triggered: SMT logical threads ({}) mistaken for {} physical cores (actual distinct physical cores = {})",
            requested_logical_cpus.len(),
            claimed_physical_cores,
            actual_physical
        ));
    }

    let max_cores = 4usize;
    let max_mem_gib = 12usize;
    let max_scratch: u64 = 5 * 1024 * 1024 * 1024;
    let admitted = actual_physical <= max_cores
        && memory_gib <= max_mem_gib
        && scratch_bytes_used <= max_scratch;

    if !admitted {
        return Err(format!(
            "resource lease exceeded: physical_cores={actual_physical}/{max_cores}, mem_gib={memory_gib}/{max_mem_gib}, scratch={scratch_bytes_used}/{max_scratch}"
        ));
    }

    Ok(ResourceLeaseAudit {
        lease_id: lease_id.to_string(),
        requested_logical_cpus: requested_logical_cpus.to_vec(),
        physical_cores_used: actual_physical,
        max_allowed_physical_cores: max_cores,
        memory_gib,
        max_allowed_memory_gib: max_mem_gib,
        scratch_bytes_used,
        max_allowed_scratch_bytes: max_scratch,
        smt_mistaken_for_physical_rejected: true,
        admitted,
    })
}

/// `OPS_VIEW-02` & `OPS_VIEW-03`: Scene bundle manifest binding `.glb` geometry to governing SHA-256 digests.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlbSceneManifest {
    pub schema_version: u32,
    pub kind: String,
    pub design_sha256: String,
    #[serde(default)]
    pub design_ir_sha256: String,
    /// Legacy in-memory field retained for internal callers; omitted from serialized subject (`IR-06`).
    #[serde(skip_serializing, default)]
    pub topology_class: String,
    pub glb_path: String,
    pub glb_sha256: String,
    pub glb_bytes: usize,
    pub units: String,
    pub true_deformation_scale: f64,
    pub coordinate_frame: String,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub bounds_min_m: [f32; 3],
    pub bounds_max_m: [f32; 3],
    pub camera_preset: CameraPreset,
    pub governing_surface_stl_sha256: String,
    #[serde(default)]
    pub derived_taxon: Option<joil_ir::PhenotypeTaxon>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraPreset {
    pub eye_m: [f64; 3],
    pub target_m: [f64; 3],
    pub up: [f64; 3],
    pub fov_deg: f64,
}

/// Export a `MultiPatchSurface` directly to a standards-compliant glTF 2.0 binary `.glb` file
/// and write its hash-bound `.scene-manifest.json` (`OPS_VIEW-02`, `OPS_VIEW-03`).
pub fn export_glb_scene_bundle(
    surface: &MultiPatchSurface,
    governing_stl_sha256: &str,
    deformation_scale: f64,
    glb_path: &Path,
) -> Result<GlbSceneManifest, String> {
    if (deformation_scale - 1.0).abs() > 1e-12 {
        return Err(
            "OPS_VIEW-03 negative control triggered: non-unity deformation/smoothing scale forbidden in scientific scene bundle"
                .into(),
        );
    }
    if governing_stl_sha256.len() != 64
        || !governing_stl_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("OPS_VIEW-02: governing_stl_sha256 must be a SHA-256 hex digest".into());
    }
    if surface.vertices_m.is_empty() || surface.triangles.is_empty() {
        return Err("cannot export empty surface to GLB".into());
    }
    // V5-01-05: the governing STL must be this surface's own serialization. A digest of another
    // subject (for example a historical run's STL) is not evidence of equivalence.
    let check_path = glb_path.with_extension(format!("subject-check-{}.stl", std::process::id()));
    if check_path.exists() {
        return Err(format!("refusing to replace {}", check_path.display()));
    }
    surface.write_openfoam_ascii_stl(&check_path)?;
    let own_sha = crate::openfoam::file_sha256(&check_path);
    let _ = fs::remove_file(&check_path);
    if own_sha? != governing_stl_sha256 {
        return Err(
            "OPS_VIEW-02: governing STL digest does not match the exported surface; subject equivalence unknown"
                .into(),
        );
    }

    let n_verts = surface.vertices_m.len();
    let n_tris = surface.triangles.len();
    let n_indices = n_tris * 3;

    let mut min_pt = [f32::INFINITY; 3];
    let mut max_pt = [f32::NEG_INFINITY; 3];
    let mut pos_bytes: Vec<u8> = Vec::with_capacity(n_verts * 12);
    for v in &surface.vertices_m {
        for k in 0..3 {
            let val = v[k] as f32;
            min_pt[k] = min_pt[k].min(val);
            max_pt[k] = max_pt[k].max(val);
            pos_bytes.extend_from_slice(&val.to_le_bytes());
        }
    }

    let mut idx_bytes: Vec<u8> = Vec::with_capacity(n_indices * 4);
    for &[i0, i1, i2] in &surface.triangles {
        for idx in [i0 as u32, i1 as u32, i2 as u32] {
            idx_bytes.extend_from_slice(&idx.to_le_bytes());
        }
    }

    let pos_len = pos_bytes.len();
    let idx_len = idx_bytes.len();
    let mut bin_chunk = Vec::with_capacity(pos_len + idx_len + 4);
    bin_chunk.extend_from_slice(&pos_bytes);
    bin_chunk.extend_from_slice(&idx_bytes);
    while bin_chunk.len() % 4 != 0 {
        bin_chunk.push(0u8);
    }

    let gltf_json = serde_json::json!({
        "asset": {
            "version": "2.0",
            "generator": "joil-core::scene::export_glb_scene_bundle"
        },
        "scene": 0,
        "scenes": [{ "nodes": [0], "name": surface.audit.topology_class }],
        "nodes": [{
            "mesh": 0,
            "name": surface.audit.topology_class,
            "extras": {
                "units": "m",
                "design_sha256": surface.audit.design_sha256,
                "governing_surface_stl_sha256": governing_stl_sha256,
                "true_deformation_scale": 1.0
            }
        }],
        "meshes": [{
            "name": surface.audit.topology_class,
            "primitives": [{
                "attributes": { "POSITION": 0 },
                "indices": 1,
                "mode": 4
            }]
        }],
        "accessors": [
            {
                "bufferView": 0,
                "byteOffset": 0,
                "componentType": 5126,
                "count": n_verts,
                "type": "VEC3",
                "min": [min_pt[0], min_pt[1], min_pt[2]],
                "max": [max_pt[0], max_pt[1], max_pt[2]]
            },
            {
                "bufferView": 1,
                "byteOffset": 0,
                "componentType": 5125,
                "count": n_indices,
                "type": "SCALAR"
            }
        ],
        "bufferViews": [
            {
                "buffer": 0,
                "byteOffset": 0,
                "byteLength": pos_len,
                "target": 34962
            },
            {
                "buffer": 0,
                "byteOffset": pos_len,
                "byteLength": idx_len,
                "target": 34963
            }
        ],
        "buffers": [{
            "byteLength": bin_chunk.len()
        }]
    });

    let mut json_bytes = serde_json::to_vec(&gltf_json).map_err(|e| e.to_string())?;
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }

    let total_len = 12 + 8 + json_bytes.len() + 8 + bin_chunk.len();
    let mut glb: Vec<u8> = Vec::with_capacity(total_len);
    // Header: magic "glTF" (0x46546C67), version 2, total length
    glb.extend_from_slice(&0x46546C67u32.to_le_bytes());
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&(total_len as u32).to_le_bytes());
    // JSON chunk header: chunkLength, chunkType "JSON" (0x4E4F534A)
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4E4F534Au32.to_le_bytes());
    glb.extend_from_slice(&json_bytes);
    // BIN chunk header: chunkLength, chunkType "BIN\0" (0x004E4942)
    glb.extend_from_slice(&(bin_chunk.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004E4942u32.to_le_bytes());
    glb.extend_from_slice(&bin_chunk);

    if let Some(parent) = glb_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(glb_path, &glb).map_err(|e| e.to_string())?;

    let glb_sha = {
        let mut h = Sha256::new();
        h.update(&glb);
        format!("{:x}", h.finalize())
    };

    let manifest = GlbSceneManifest {
        schema_version: 1,
        kind: "rust_native_gltf2_scientific_scene_bundle".to_string(),
        design_sha256: surface.audit.design_sha256.clone(),
        design_ir_sha256: surface.audit.design_ir_sha256.clone(),
        topology_class: surface.audit.topology_class.clone(),
        glb_path: glb_path.display().to_string(),
        glb_sha256: glb_sha,
        glb_bytes: glb.len(),
        units: "m".to_string(),
        true_deformation_scale: 1.0,
        coordinate_frame: "right_handed_SI_z_downstream_m".to_string(),
        vertex_count: n_verts,
        triangle_count: n_tris,
        bounds_min_m: min_pt,
        bounds_max_m: max_pt,
        camera_preset: CameraPreset {
            eye_m: [-0.22, -0.26, 0.08],
            target_m: [0.0, 0.0, 0.08],
            up: [0.0, 0.0, 1.0],
            fov_deg: 35.0,
        },
        governing_surface_stl_sha256: governing_stl_sha256.to_string(),
        derived_taxon: surface.audit.derived_taxon.clone(),
    };

    let manifest_path = glb_path.with_extension("scene-manifest.json");
    let manifest_str = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(manifest_path, format!("{manifest_str}\n")).map_err(|e| e.to_string())?;

    Ok(manifest)
}

/// Construct a `CudaSubjectBundleRef` bound to the compiled `DesignIrV1` and exact STL artifact bytes (`IR-10`).
pub fn build_cuda_subject_bundle_ref(
    prog: &crate::program::DesignProgram,
    surface_stl_bytes: &[u8],
) -> Result<joil_ir::CudaSubjectBundleRef, String> {
    let ir = crate::geometry::compile_design_ir(prog).map_err(|e| e.to_string())?;
    Ok(joil_ir::CudaSubjectBundleRef::from_ir_and_artifact_bytes(
        &ir,
        surface_stl_bytes,
    ))
}

/// Provenance of every trajectory this module writes (`V5-01-04`).
pub const SCHEMATIC_TRAJECTORY_PROVENANCE: &str =
    "SCHEMATIC_AUTHORED_CURVE_NOT_SIMULATED_OR_MEASURED";

fn schematic_trajectory_provenance() -> String {
    SCHEMATIC_TRAJECTORY_PROVENANCE.to_string()
}

/// Authored 3D path for visualization in `apps/explorer` (`P09`). The points come from a closed-form
/// curve with an authored inward-migration law, not from integrated particles or measurements;
/// `captured_to_overflow` is the authored rule `diameter >= screen d50` (`V5-01-04`). Records
/// without provenance are historical exports of the same authored curves and read as SCHEMATIC.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DropletTrajectory3D {
    pub label: String,
    pub diameter_um: f64,
    pub captured_to_overflow: bool,
    pub points_mm: Vec<[f64; 3]>,
    #[serde(default = "schematic_trajectory_provenance")]
    pub provenance: String,
}

/// Interactive 3D geometry, internal swirl hardware, droplet trajectories, and metrics for one candidate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interactive3DCandidate {
    pub id: String,
    pub topology_class: String,
    pub genome_sha256: String,
    pub generation_born: usize,
    pub island_origin: usize,
    pub branches_per_port: u32,
    pub axial_drop_mm: f64,
    pub radial_slot_factor: f64,
    pub skimmer_length_mm: f64,
    pub skimmer_flutes: u32,
    pub chamber_mode_count: usize,
    pub nominal_cout_mg_l: f64,
    pub cout_p05_mg_l: f64,
    pub cout_p50_mg_l: f64,
    pub cout_p95_mg_l: f64,
    pub dp_u_pa: f64,
    pub pdr: f64,
    pub d50_um: f64,
    pub grade_20um_p05: f64,
    pub max_entry_shear_s_inv: f64,
    pub short_circuit_fraction: f64,
    pub p08_yield_safety_factor: f64,
    pub p08_max_von_mises_mpa: f64,
    pub p08_vane_modal_freq_ratio: f64,
    pub p08_lpbf_overhang_deg: f64,
    pub min_wall_clearance_mm: f64,
    pub grade_curve_bins: Vec<(f64, f64)>,
    pub outer_wall_rings_mm: Vec<Vec<[f64; 3]>>,
    pub skimmer_rings_mm: Vec<Vec<[f64; 3]>>,
    pub inlet_jets_3d_mm: Vec<Vec<[f64; 3]>>,
    pub helical_ribbon_3d_mm: Vec<[f64; 3]>,
    pub droplet_trajectories_3d_mm: Vec<DropletTrajectory3D>,
}

/// Compact summary point for the interactive Pareto & MAP-Elites scatter/grid explorer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchiveExplorerPoint {
    pub id: String,
    pub topology_class: String,
    pub cell_key: String,
    pub cout_p95_mg_l: f64,
    pub dp_u_pa: f64,
    pub d50_um: f64,
    pub grade_20um_p05: f64,
    pub max_entry_shear_s_inv: f64,
    pub swirl_flux_index: f64,
    pub core_shield_ratio: f64,
    pub is_pareto_rank0: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interactive3DPreviewBundle {
    pub schema_version: u32,
    pub kind: String,
    pub units: String,
    pub true_scale: f64,
    pub campaign_summary_sha256: String,
    pub candidates_3d: Vec<Interactive3DCandidate>,
    pub archive_points: Vec<ArchiveExplorerPoint>,
}

#[path = "scene/preview.rs"]
mod preview;
pub use self::preview::*;
