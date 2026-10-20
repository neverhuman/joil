//! Native Rust OpenFOAM 14 Candidate Case Generator (`P03/P04/P05`).
//!
//! Compiles any 3D `DesignProgram` into a conformal 5-patch SI STL (`wall`, `inlet1`,
//! `inlet2`, `overflow`, `underflow`) and writes the complete OpenFOAM 14 meshing and
//! carrier dictionary tree (`blockMeshDict`, `snappyHexMeshDict`, `0/{U,p,k,epsilon,omega,nut,R}`,
//! `controlDict`, `fvSchemes`, `fvSolution`, `decomposeParDict`) along with immutable
//! SHA-256 provenance manifests (`mesh-input.json`, `carrier-input.json`, `input-manifest.json`).

use crate::geometry::baseline;
use crate::phenotype::{MultiPatchAuditReport, MultiPatchSurface, Phenotype3D};
use crate::program::DesignProgram;
use crate::redteam;
use crate::structure;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Configuration for generating an OpenFOAM 14 candidate case.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CfdCaseConfig {
    pub feed_m3_h: f64,
    pub reject_pressure_pa: f64,
    pub turbulence_model: String,
    pub steps: usize,
    pub refinement_level: usize,
    pub outlet_extension_m: f64,
    pub transient: bool,
    pub dt_s: f64,
    pub end_time_s: f64,
    pub n_subdomains: usize,
}

impl Default for CfdCaseConfig {
    fn default() -> Self {
        Self {
            feed_m3_h: 4.0,
            reject_pressure_pa: -200.0,
            turbulence_model: "SSG".to_string(),
            steps: 600,
            refinement_level: 0,
            outlet_extension_m: 0.0,
            transient: false,
            dt_s: 1e-5,
            end_time_s: 0.25,
            n_subdomains: 4,
        }
    }
}

/// Complete provenance manifest for a prepared OpenFOAM 14 candidate case.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CfdCaseManifest {
    pub schema_version: u32,
    pub kind: String,
    pub case_dir: String,
    pub design_sha256: String,
    #[serde(default)]
    pub design_ir_sha256: String,
    /// Legacy in-memory field retained for internal callers; omitted from serialized subject (`IR-06`).
    #[serde(skip_serializing, default)]
    pub topology_class: String,
    pub config: CfdCaseConfig,
    pub surface_stl_relpath: String,
    pub surface_stl_sha256: String,
    pub surface_stl_bytes: usize,
    pub multi_patch_audit: MultiPatchAuditReport,
    pub redteam_passed: bool,
    #[serde(
        rename = "analytic_structural_screen_passed",
        alias = "structural_qualified"
    )]
    pub structural_qualified: bool,
    pub dictionary_files_sha256: BTreeMap<String, String>,
    pub qualified_for_oil_grade: bool,
    pub limitations: Vec<String>,
    #[serde(default)]
    pub derived_taxon: Option<joil_ir::PhenotypeTaxon>,
    #[serde(default)]
    pub enactment_receipt: Option<joil_ir::FeatureEnactmentReceipt>,
    #[serde(default)]
    pub subject_bundle: Option<joil_evidence::subject::SubjectBundle>,
}

pub(crate) fn foam_file(object_name: &str, class_name: &str, body: &str) -> String {
    format!("FoamFile {{ format ascii; class {class_name}; object {object_name}; }}\n{body}\n")
}

pub(crate) fn write_dict(
    case_dir: &Path,
    relpath: &str,
    class_name: &str,
    body: &str,
    hashes: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let dest = case_dir.join(relpath);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let obj_name = dest
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("dictionary");
    let content = foam_file(obj_name, class_name, body);
    fs::write(&dest, content.as_bytes()).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hashes.insert(relpath.to_string(), format!("{:x}", hasher.finalize()));
    Ok(())
}

/// Build a fresh, self-contained OpenFOAM 14 candidate case directory from a 3D `DesignProgram`.
pub fn build_openfoam_candidate_case(
    prog: &DesignProgram,
    case_dir: &Path,
    config: &CfdCaseConfig,
) -> Result<CfdCaseManifest, String> {
    if case_dir.exists() || case_dir.join("input-manifest.json").exists() {
        return Err(format!(
            "refusing to overwrite existing case directory {}",
            case_dir.display()
        ));
    }
    if !config.feed_m3_h.is_finite()
        || config.feed_m3_h <= 0.0
        || !config.reject_pressure_pa.is_finite()
        || config.steps == 0
    {
        return Err("finite positive flow, reject pressure, and step count required".into());
    }
    if !["SSG", "LRR", "kOmegaSST"].contains(&config.turbulence_model.as_str()) {
        return Err(format!(
            "unsupported turbulence model '{}'",
            config.turbulence_model
        ));
    }
    if config.refinement_level > 1 {
        return Err("refinement_level must be 0 or 1".into());
    }

    let p = prog.normalize()?;
    let g = baseline();
    let d = g.derive()?;

    // Run F0 Red-Team + P08 Structural + Multi-Patch CFD Surface compilation
    let (pheno, mesh) = Phenotype3D::from_program(&p)?;
    redteam::check(&p, &pheno, &mesh)
        .map_err(|rej| format!("candidate failed F0 red-team gate: {rej:?}"))?;
    let structure =
        structure::analytic_structural_screen(&p, &pheno, 29468.22, config.feed_m3_h / 3600.0)?;
    if !structure.passed {
        return Err(format!(
            "candidate failed the P08 analytic structural/LPBF screen: {:?}",
            structure.failure_reasons
        ));
    }

    let surface = MultiPatchSurface::from_program(&p, config.outlet_extension_m)?;
    if !surface.audit.watertight_and_manifold || !surface.audit.all_terminal_patches_valid {
        return Err(
            "multi-patch surface failed watertightness or terminal patch area/normal gate".into(),
        );
    }

    fs::create_dir_all(case_dir).map_err(|e| e.to_string())?;
    let stl_rel = "constant/triSurface/ct35.stl";
    let stl_path = case_dir.join(stl_rel);
    surface.write_openfoam_ascii_stl(&stl_path)?;
    let stl_bytes = fs::read(&stl_path).map_err(|e| e.to_string())?;
    let stl_sha = {
        let mut h = Sha256::new();
        h.update(&stl_bytes);
        format!("{:x}", h.finalize())
    };

    let mut dict_hashes = BTreeMap::new();
    dict_hashes.insert(stl_rel.to_string(), stl_sha.clone());
    dictionaries::write_openfoam_dictionaries(case_dir, config, &d, &mut dict_hashes)?;

    let design_json = serde_json::to_string_pretty(&p).map_err(|e| e.to_string())?;
    fs::write(case_dir.join("design-program.json"), design_json.as_bytes())
        .map_err(|e| e.to_string())?;
    dict_hashes.insert(
        "design-program.json".to_string(),
        joil_ir::sha256_hex(design_json.as_bytes()),
    );

    let ir = crate::geometry::compile_design_ir(&p).map_err(|e| e.to_string())?;
    let ir_json = serde_json::to_string_pretty(&ir).map_err(|e| e.to_string())?;
    fs::write(case_dir.join("design-ir.json"), ir_json.as_bytes()).map_err(|e| e.to_string())?;
    dict_hashes.insert(
        "design-ir.json".to_string(),
        joil_ir::sha256_hex(ir_json.as_bytes()),
    );

    if let Some(ref receipt) = surface.audit.enactment_receipt {
        let receipt_json = serde_json::to_string_pretty(receipt).map_err(|e| e.to_string())?;
        fs::write(
            case_dir.join("feature-enactment-receipt.json"),
            receipt_json.as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        dict_hashes.insert(
            "feature-enactment-receipt.json".to_string(),
            joil_ir::sha256_hex(receipt_json.as_bytes()),
        );
    }

    let cfg_bytes = serde_json::to_vec(config).map_err(|e| e.to_string())?;
    let audit_bytes = serde_json::to_vec(&surface.audit).map_err(|e| e.to_string())?;
    let dict_hashes_bytes = serde_json::to_vec(&dict_hashes).map_err(|e| e.to_string())?;
    use joil_evidence::identity::ContentRef;
    let subject_bundle = joil_evidence::subject::SubjectBundle {
        genome: ContentRef::from_bytes("design-program.json", "v1", design_json.as_bytes()),
        ir: ContentRef::from_bytes("design-ir.json", "v1", ir_json.as_bytes()),
        compiler: ContentRef::from_bytes(
            "joil-core::cfd_case::build_openfoam_candidate_case",
            env!("CARGO_PKG_VERSION"),
            b"joil-core::cfd_case::v1",
        ),
        geometry_kernel: ContentRef::from_bytes(
            "joil-ir::IrCompiledMesh",
            env!("CARGO_PKG_VERSION"),
            b"joil-ir::IrCompiledMesh::v1",
        ),
        compiler_config: ContentRef::from_bytes("cfd_case_config", "v1", &cfg_bytes),
        geometry: ContentRef::from_bytes("multi_patch_audit", "v1", &audit_bytes),
        surface_mesh: ContentRef::from_bytes(stl_rel, "v1", &stl_bytes),
        volume_mesh: None,
        solver_case: Some(ContentRef::from_bytes(
            "dictionary_files_sha256",
            "v1",
            &dict_hashes_bytes,
        )),
    };

    let derived_taxon = surface.audit.derived_taxon.clone();
    let enactment_receipt = surface.audit.enactment_receipt.clone();

    let manifest = CfdCaseManifest {
        schema_version: 1,
        kind: "rust_native_openfoam14_candidate_case_input_not_executed_cfd".to_string(),
        case_dir: case_dir.display().to_string(),
        design_sha256: p.canonical_sha256()?,
        design_ir_sha256: ir.ir_sha256,
        topology_class: p.topology_class.clone(),
        config: config.clone(),
        surface_stl_relpath: stl_rel.to_string(),
        surface_stl_sha256: stl_sha,
        surface_stl_bytes: stl_bytes.len(),
        multi_patch_audit: surface.audit,
        redteam_passed: true,
        structural_qualified: structure.passed,
        dictionary_files_sha256: dict_hashes,
        qualified_for_oil_grade: false,
        limitations: vec![
            "Case inputs and 5-patch watertight STL are prepared and hash-bound; OpenFOAM meshing and solver execution must still run under resource lease.".to_string(),
            "Not a qualified F2 oil-grade result until mesh convergence, zero-backflow steady/transient carrier, and 10-bin inlet DSD transport pass.".to_string(),
        ],
        derived_taxon,
        enactment_receipt,
        subject_bundle: Some(subject_bundle),
    };

    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(
        case_dir.join("input-manifest.json"),
        format!("{manifest_json}\n"),
    )
    .map_err(|e| e.to_string())?;

    dictionaries::write_ev12_capture_script(case_dir)?;

    Ok(manifest)
}

#[path = "cfd_case/dictionaries.rs"]
mod dictionaries;
#[path = "cfd_case/identity.rs"]
mod identity;
#[cfg(test)]
#[path = "cfd_case/tests.rs"]
mod tests;

pub use self::identity::*;
