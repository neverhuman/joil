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

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn build_3d_candidate_preview(
    ind: &crate::evolve::Individual,
) -> Result<Interactive3DCandidate, String> {
    use crate::openfoam::STANDARD_BINS_UM;
    use crate::phenotype::Phenotype3D;
    use std::f64::consts::PI;

    let prog = &ind.program;
    let d = crate::geometry::baseline().derive()?;
    let (pheno, _) = Phenotype3D::from_program(prog)?;
    let sq = crate::structure::analytic_structural_screen(
        prog,
        &pheno,
        ind.nominal_metrics.dp_u_pa,
        0.001111111,
    )?;

    // 1. Outer wall 3D rings (22 axial stations in z in [0, 186.75] mm x 28 azimuthal points)
    let n_z = 22usize;
    let n_theta = 28usize;
    let mut outer_wall_rings_mm = Vec::with_capacity(n_z);
    for iz in 0..n_z {
        let z_mm = 186.747 * (iz as f64) / ((n_z - 1) as f64);
        let mut ring = Vec::with_capacity(n_theta);
        for it in 0..n_theta {
            let theta = 2.0 * PI * (it as f64) / (n_theta as f64);
            let r = Phenotype3D::inner_radius_mm(prog, &d, theta, z_mm);
            ring.push([
                round2(r * theta.cos()),
                round2(r * theta.sin()),
                round2(z_mm),
            ]);
        }
        outer_wall_rings_mm.push(ring);
    }

    // 2. Overflow vortex finder & contoured/fluted skimmer rings (z in [-22.8, skimmer.length_mm] mm)
    let skim_end = prog.skimmer.length_mm.max(0.5);
    let n_sz = 7usize;
    let n_st = 20usize;
    let mut skimmer_rings_mm = Vec::with_capacity(n_sz);
    for iz in 0..n_sz {
        let s = (iz as f64) / ((n_sz - 1) as f64);
        let z_mm = -22.8 + (22.8 + skim_end) * s;
        let base_r = if z_mm < 0.0 {
            1.4
        } else {
            prog.skimmer.outer_radius_mm - 0.35 * (z_mm / skim_end)
        };
        let mut ring = Vec::with_capacity(n_st);
        for it in 0..n_st {
            let theta = 2.0 * PI * (it as f64) / (n_st as f64);
            let flute_mod = if z_mm >= 0.0 && prog.skimmer.capture_flutes > 0 {
                0.18 * ((prog.skimmer.capture_flutes as f64) * theta).cos()
            } else {
                0.0
            };
            let r = base_r + flute_mod;
            ring.push([
                round2(r * theta.cos()),
                round2(r * theta.sin()),
                round2(z_mm),
            ]);
        }
        skimmer_rings_mm.push(ring);
    }

    // 3. Multi-branch 3D tangential entry jet centerlines
    let branches = if prog.inlet_split.enabled {
        prog.inlet_split.branches_per_port.max(2)
    } else {
        1
    };
    let mut inlet_jets_3d_mm = Vec::new();
    for &port_sign in &[-1.0_f64, 1.0_f64] {
        for b in 0..branches {
            let b_frac = if branches == 1 {
                0.0
            } else {
                ((b as f64) / ((branches - 1) as f64)) - 0.5
            };
            let mut jet = Vec::with_capacity(16);
            for step in 0..16 {
                let t = (step as f64) / 15.0;
                if t <= 0.45 {
                    let u = t / 0.45;
                    let x = port_sign * (60.0 - 52.0 * u);
                    let y = -port_sign
                        * (31.28 * prog.inlet_ramp.radial_slot_factor + 1.8 * b_frac * u);
                    let z = 6.45 + prog.inlet_ramp.axial_drop_mm * u + 3.2 * b_frac * u;
                    jet.push([round2(x), round2(y), round2(z)]);
                } else {
                    let u = (t - 0.45) / 0.55;
                    let base_theta = if port_sign < 0.0 { -0.5 * PI } else { 0.5 * PI }
                        + (prog.inlet_split.azimuthal_phase_deg.to_radians() * b_frac);
                    let theta = base_theta + 1.65 * PI * u;
                    let r = 31.0 * (1.0 - 0.12 * u);
                    let z = 6.45
                        + prog.inlet_ramp.axial_drop_mm
                        + (18.0 + 1.8 * prog.inlet_ramp.axial_drop_mm) * u
                        + 3.2 * b_frac;
                    jet.push([round2(r * theta.cos()), round2(r * theta.sin()), round2(z)]);
                }
            }
            inlet_jets_3d_mm.push(jet);
        }
    }

    // 4. Internal helical swirl ribbon / wall rifling guide curve
    let mut helical_ribbon_3d_mm = Vec::with_capacity(40);
    let pitch_turns =
        1.6 + 0.15 * prog.inlet_ramp.axial_drop_mm + 0.25 * (prog.chamber_modes.len() as f64);
    for i in 0..40 {
        let s = (i as f64) / 39.0;
        let z_mm = 6.0 + 96.0 * s;
        let theta = 2.0 * PI * pitch_turns * s;
        let r = (Phenotype3D::inner_radius_mm(prog, &d, theta, z_mm) - 1.6).max(12.0);
        helical_ribbon_3d_mm.push([
            round2(r * theta.cos()),
            round2(r * theta.sin()),
            round2(z_mm),
        ]);
    }

    // 5. SCHEMATIC authored droplet and carrier paths (10, 20, 40 um oil + carrier water); no particle
    // is integrated (`V5-01-04`).
    let swirl_boost = ind.descriptors.swirl_flux_index;
    let mut droplet_trajectories_3d_mm = Vec::new();
    for &(label, d_um, is_water) in &[
        ("SCHEMATIC 40 μm oil droplet path (authored)", 40.0, false),
        ("SCHEMATIC 20 μm oil droplet path (authored)", 20.0, false),
        ("SCHEMATIC 10 μm oil droplet path (authored)", 10.0, false),
        (
            "SCHEMATIC treated-water outer vortex path (authored)",
            0.0,
            true,
        ),
    ] {
        let mut pts = Vec::with_capacity(48);
        for k in 0..48 {
            let s = (k as f64) / 47.0;
            let turns = (3.8 + 0.9 * swirl_boost) * s;
            let theta = 2.0 * PI * turns + (d_um * 0.04);
            let (r_mm, z_mm) = if is_water {
                let z = 6.0 + 178.0 * s;
                let r = (Phenotype3D::inner_radius_mm(prog, &d, theta, z) * 0.82).max(9.5);
                (r, z)
            } else {
                // Radial inward migration speed scales with d^2 * swirl_boost^2
                let mig_rate = (d_um / 22.0).powi(2) * swirl_boost * 1.15;
                let r_start = 29.5;
                let r_core = 0.85;
                let r = r_core + (r_start - r_core) * (-mig_rate * s * 2.2).exp();
                // Once r approaches the reverse-flowing inner vortex core (r < 6.5 mm), axial velocity reverses toward z = -22.8 mm!
                let z = if r > 6.5 {
                    6.0 + 115.0 * s
                } else {
                    let core_frac = ((6.5 - r) / (6.5 - r_core)).clamp(0.0, 1.0);
                    (6.0 + 75.0 * (1.0 - core_frac)) * (1.0 - core_frac) - 22.0 * core_frac
                };
                (r, z)
            };
            pts.push([
                round2(r_mm * theta.cos()),
                round2(r_mm * theta.sin()),
                round2(z_mm),
            ]);
        }
        droplet_trajectories_3d_mm.push(DropletTrajectory3D {
            label: label.to_string(),
            diameter_um: d_um,
            captured_to_overflow: !is_water && d_um >= ind.nominal_metrics.d50_um,
            points_mm: pts,
            provenance: schematic_trajectory_provenance(),
        });
    }

    let grade_curve_bins: Vec<(f64, f64)> = STANDARD_BINS_UM
        .iter()
        .map(|&d_bin| {
            let key = format!("{}um", d_bin as u32);
            let g = *ind
                .nominal_metrics
                .grade_efficiency_bins
                .get(&key)
                .unwrap_or(&0.0);
            (d_bin, (g * 10000.0).round() / 10000.0)
        })
        .collect();

    Ok(Interactive3DCandidate {
        id: ind.id.clone(),
        topology_class: ind.topology_class.clone(),
        genome_sha256: ind.genome_sha256.clone(),
        generation_born: ind.generation_born,
        island_origin: ind.island_origin,
        branches_per_port: branches,
        axial_drop_mm: prog.inlet_ramp.axial_drop_mm,
        radial_slot_factor: prog.inlet_ramp.radial_slot_factor,
        skimmer_length_mm: prog.skimmer.length_mm,
        skimmer_flutes: prog.skimmer.capture_flutes,
        chamber_mode_count: prog.chamber_modes.len(),
        nominal_cout_mg_l: ind.nominal_metrics.cout_mg_l,
        cout_p05_mg_l: ind.mc_robustness.cout_p05_mg_l,
        cout_p50_mg_l: ind.mc_robustness.cout_p50_mg_l,
        cout_p95_mg_l: ind.mc_robustness.cout_p95_mg_l,
        dp_u_pa: ind.nominal_metrics.dp_u_pa,
        pdr: ind.nominal_metrics.pdr,
        d50_um: ind.nominal_metrics.d50_um,
        grade_20um_p05: ind.mc_robustness.grade_20um_p05,
        max_entry_shear_s_inv: ind.nominal_metrics.max_entry_shear_s_inv,
        short_circuit_fraction: ind.nominal_metrics.short_circuit_fraction,
        p08_yield_safety_factor: sq.min_yield_safety_factor,
        p08_max_von_mises_mpa: sq.max_wall_von_mises_mpa,
        p08_vane_modal_freq_ratio: sq.vane_modal_freq_ratio,
        p08_lpbf_overhang_deg: sq.max_lpbf_overhang_from_vertical_deg,
        min_wall_clearance_mm: pheno.minimum_wall_clearance_mm,
        grade_curve_bins,
        outer_wall_rings_mm,
        skimmer_rings_mm,
        inlet_jets_3d_mm,
        helical_ribbon_3d_mm,
        droplet_trajectories_3d_mm,
    })
}

/// Export an interactive 3D preview & MAP-Elites/Pareto archive bundle (`docs/results/interactive-3d-archetypes-v1.json`).
pub fn export_interactive_3d_preview_bundle(
    campaign_summary_path: &Path,
    archive_path: &Path,
    out_path: &Path,
) -> Result<Interactive3DPreviewBundle, String> {
    if !campaign_summary_path.is_file() && out_path.is_file() {
        let raw_existing = fs::read_to_string(out_path).map_err(|e| e.to_string())?;
        let existing: Interactive3DPreviewBundle =
            serde_json::from_str(&raw_existing).map_err(|e| e.to_string())?;
        if existing.candidates_3d.is_empty() {
            return Err("existing interactive 3D preview bundle has empty candidates_3d".into());
        }
        return Ok(existing);
    }
    let raw_sum = fs::read_to_string(campaign_summary_path).map_err(|e| e.to_string())?;
    let campaign: crate::evolve::CampaignSummary =
        serde_json::from_str(&raw_sum).map_err(|e| e.to_string())?;
    let sum_sha = crate::openfoam::file_sha256(campaign_summary_path)?;

    let mut candidates_3d = Vec::new();
    candidates_3d.push(build_3d_candidate_preview(&campaign.baseline_reference)?);
    for elite in &campaign.top_pareto_elites {
        if !candidates_3d
            .iter()
            .any(|c| c.genome_sha256 == elite.genome_sha256)
        {
            candidates_3d.push(build_3d_candidate_preview(elite)?);
        }
    }
    let mut wave2_rng = crate::program::Rng::new(20261101);
    let wave2_seeds = crate::program::DesignProgram::seed_wave2_archetypes(&mut wave2_rng);
    let mut seen_w2_fam = std::collections::BTreeSet::new();
    for (idx, w2_prog) in wave2_seeds.into_iter().skip(15).enumerate() {
        let fam = w2_prog.classify_topology();
        if !seen_w2_fam.insert(fam) {
            continue;
        }
        let (ind, _) = crate::evolve::evaluate_individual(
            format!("wave2-exotic-{idx:02}-{}", w2_prog.topology_class),
            w2_prog,
            0,
            0,
            &campaign.config,
        )?;
        if !candidates_3d
            .iter()
            .any(|c| c.genome_sha256 == ind.genome_sha256)
        {
            candidates_3d.push(build_3d_candidate_preview(&ind)?);
        }
    }

    let mut archive_points = Vec::new();
    if archive_path.is_file() {
        let raw_arch = fs::read_to_string(archive_path).map_err(|e| e.to_string())?;
        let map: std::collections::BTreeMap<String, crate::evolve::Individual> =
            serde_json::from_str(&raw_arch).map_err(|e| e.to_string())?;
        for (cell_key, ind) in map {
            let is_pareto = campaign
                .top_pareto_elites
                .iter()
                .any(|e| e.genome_sha256 == ind.genome_sha256);
            archive_points.push(ArchiveExplorerPoint {
                id: ind.id,
                topology_class: ind.topology_class,
                cell_key,
                cout_p95_mg_l: round2(ind.mc_robustness.cout_p95_mg_l),
                dp_u_pa: round2(ind.nominal_metrics.dp_u_pa),
                d50_um: round2(ind.nominal_metrics.d50_um),
                grade_20um_p05: (ind.mc_robustness.grade_20um_p05 * 10000.0).round() / 10000.0,
                max_entry_shear_s_inv: round2(ind.nominal_metrics.max_entry_shear_s_inv),
                swirl_flux_index: (ind.descriptors.swirl_flux_index * 1000.0).round() / 1000.0,
                core_shield_ratio: (ind.descriptors.core_shield_ratio * 1000.0).round() / 1000.0,
                is_pareto_rank0: is_pareto,
            });
        }
    }

    let bundle = Interactive3DPreviewBundle {
        schema_version: 1,
        kind: "interactive_3d_hydrocyclone_preview_and_archive_bundle".to_string(),
        units: "mm".to_string(),
        true_scale: 1.0,
        campaign_summary_sha256: sum_sha,
        candidates_3d,
        archive_points,
    };
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json_str = serde_json::to_string(&bundle).map_err(|e| e.to_string())?;
    fs::write(out_path, format!("{json_str}\n")).map_err(|e| e.to_string())?;
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v5_01_04_new_trajectories_are_schematic() {
        let cfg = crate::evolve::EvolutionConfig {
            mc_samples: 4,
            ..crate::evolve::EvolutionConfig::default()
        };
        let (ind, _) = crate::evolve::evaluate_individual(
            "base".into(),
            crate::program::DesignProgram::baseline_ct35(),
            0,
            0,
            &cfg,
        )
        .unwrap();
        let preview = build_3d_candidate_preview(&ind).unwrap();
        assert_eq!(preview.droplet_trajectories_3d_mm.len(), 4);
        for traj in &preview.droplet_trajectories_3d_mm {
            assert_eq!(traj.provenance, SCHEMATIC_TRAJECTORY_PROVENANCE);
            assert!(traj.label.starts_with("SCHEMATIC "), "{}", traj.label);
        }
    }

    #[test]
    fn v5_01_04_historical_bundle_reads_as_schematic_not_measured() {
        // Read-only path: a missing campaign summary returns the existing bundle without writing.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bundle_path = root.join("docs/results/interactive-3d-archetypes-v1.json");
        let before = fs::read(&bundle_path).unwrap();
        let bundle = export_interactive_3d_preview_bundle(
            &root.join("runs/v5-01-04-missing-campaign-summary.json"),
            &root.join("runs/v5-01-04-missing-archive.json"),
            &bundle_path,
        )
        .unwrap();
        assert!(!before.windows(10).any(|w| w == b"provenance"));
        let mut count = 0;
        for cand in &bundle.candidates_3d {
            for traj in &cand.droplet_trajectories_3d_mm {
                assert_eq!(traj.provenance, SCHEMATIC_TRAJECTORY_PROVENANCE);
                count += 1;
            }
        }
        assert!(count > 0);
        assert_eq!(fs::read(&bundle_path).unwrap(), before);
    }
}
