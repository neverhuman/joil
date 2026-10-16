//! Native Rust OpenFOAM 14 Candidate Case Generator (`P03/P04/P05`).
//!
//! Compiles any 3D `DesignProgram` into a conformal 5-patch SI STL (`wall`, `inlet1`,
//! `inlet2`, `overflow`, `underflow`) and writes the complete OpenFOAM 14 meshing and
//! carrier dictionary tree (`blockMeshDict`, `snappyHexMeshDict`, `0/{U,p,k,epsilon,omega,nut,R}`,
//! `controlDict`, `fvSchemes`, `fvSolution`, `decomposeParDict`) along with immutable
//! SHA-256 provenance manifests (`mesh-input.json`, `carrier-input.json`, `input-manifest.json`).

use crate::geometry::baseline;
use crate::phenotype::{CfdPatch, MultiPatchAuditReport, MultiPatchSurface, Phenotype3D};
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

fn foam_file(object_name: &str, class_name: &str, body: &str) -> String {
    format!("FoamFile {{ format ascii; class {class_name}; object {object_name}; }}\n{body}\n")
}

fn write_dict(
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

    let z_max_bg = 1.656 + config.outlet_extension_m;
    let nz_bg = ((z_max_bg + 0.036) / 0.006).round() as usize;
    let block_mesh = format!(
        "convertToMeters 1;\n\
vertices ((-.072 -.048 -.036) (.072 -.048 -.036) (.072 .048 -.036) (-.072 .048 -.036)\n\
          (-.072 -.048 {z_max_bg:.6}) (.072 -.048 {z_max_bg:.6}) (.072 .048 {z_max_bg:.6}) (-.072 .048 {z_max_bg:.6}));\n\
blocks (hex (0 1 2 3 4 5 6 7) (24 16 {nz_bg}) simpleGrading (1 1 1));\n\
edges (); boundary (background {{type patch; faces ((0 3 2 1)(4 5 6 7)(0 1 5 4)(1 2 6 5)(2 3 7 6)(3 0 4 7));}});\n\
mergePatchPairs ();"
    );
    write_dict(
        case_dir,
        "system/blockMeshDict",
        "dictionary",
        &block_mesh,
        &mut dict_hashes,
    )?;

    let lvl = config.refinement_level;
    let mut region_names = Vec::new();
    let mut face_levels = Vec::new();
    for patch in CfdPatch::all() {
        let name = patch.as_str();
        let ptype = patch.openfoam_type();
        let flvl = (if patch == CfdPatch::Wall { 2 } else { 3 }) + lvl;
        region_names.push(format!("{name} {{ name {name}; }}"));
        face_levels.push(format!(
            "{name} {{ level ({flvl} {flvl}); patchInfo {{ type {ptype}; }} }}"
        ));
    }
    let snappy_dict = format!(
        "castellatedMesh true; snap true; addLayers false;\n\
geometry {{\n\
 ct35 {{ type triSurfaceMesh; file \"ct35.stl\"; name ct35; regions {{ {} }} }}\n\
 head {{type searchableBox; min (-.064 -.039 -.026); max (.064 .039 .19);}}\n\
 bore {{type searchableBox; min (-.002 -.002 -.004); max (.002 .002 .003);}}\n\
}}\n\
castellatedMeshControls {{\n\
 maxLocalCells 800000; maxGlobalCells 1200000; minRefinementCells 0; nCellsBetweenLevels 3;\n\
 features ();\n\
 refinementSurfaces {{ct35 {{level ({} {}); regions {{ {} }} }} }}\n\
 resolveFeatureAngle 30;\n\
 refinementRegions {{head {{mode inside; level {};}} bore {{mode inside; level {};}}}}\n\
 insidePoint (0.00013 0.00017 .03513); allowFreeStandingZoneFaces true;\n\
}}\n\
snapControls {{nSmoothPatch 3; tolerance 1.5; nSolveIter 40; nRelaxIter 5;\n\
 nFeatureSnapIter 10; implicitFeatureSnap true; explicitFeatureSnap false; multiRegionFeatureSnap true;}}\n\
addLayersControls {{relativeSizes true; layers {{}} expansionRatio 1.2; finalLayerThickness .3;\n\
 minThickness .1; nGrow 0; featureAngle 60; nRelaxIter 5; nSmoothSurfaceNormals 1;\n\
 nSmoothNormals 3; nSmoothThickness 10; maxFaceThicknessRatio .5; maxThicknessToMedialRatio .3;\n\
 minMedianAxisAngle 90; nBufferCellsNoExtrude 0; nLayerIter 50;}}\n\
meshQualityControls {{maxNonOrtho 65; maxBoundarySkewness 20; maxInternalSkewness 4;\n\
 maxConcave 80; minFlatness .5; minVol 1e-18; minTetQuality 1e-15; minArea -1; minTwist .02;\n\
 minDeterminant .001; minFaceWeight .02; minVolRatio .01; minTriangleTwist -1;\n\
 nSmoothScale 4; errorReduction .75; relaxed {{maxNonOrtho 75;}}}}\n\
writeFlags (scalarLevels layerSets layerFields); mergeTolerance 1e-7;",
        region_names.join("\n"),
        1 + lvl,
        1 + lvl,
        face_levels.join("\n"),
        2 + lvl,
        5 + lvl
    );
    write_dict(
        case_dir,
        "system/snappyHexMeshDict",
        "dictionary",
        &snappy_dict,
        &mut dict_hashes,
    )?;

    // Carrier physical properties & boundary conditions
    let qi = config.feed_m3_h / 3600.0;
    let area_each_m2 = d.inlet_area_each_mm2 * 1e-6;
    let velocity = qi / (2.0 * area_each_m2);
    let k_turb = 1.5 * (0.05 * velocity).powi(2);
    let dh =
        2.0 * d.inlet_width_mm * d.inlet_height_mm / (d.inlet_width_mm + d.inlet_height_mm) * 1e-3;
    let epsilon = 0.09_f64.powf(0.75) * k_turb.powf(1.5) / (0.07 * dh);
    let omega = epsilon / (0.09 * k_turb);
    let nu = 0.001 / 998.0;

    write_dict(
        case_dir,
        "constant/physicalProperties",
        "dictionary",
        &format!("viscosityModel constant; nu {nu:.15e};"),
        &mut dict_hashes,
    )?;
    write_dict(
        case_dir,
        "constant/momentumTransport",
        "dictionary",
        &format!(
            "simulationType RAS; RAS {{model {}; turbulence on; printCoeffs on;}}",
            config.turbulence_model
        ),
        &mut dict_hashes,
    )?;

    let u_body = format!(
        "dimensions [0 1 -1 0 0 0 0]; internalField uniform (0 0 0);\n\
boundaryField {{\n\
 inlet1 {{type flowRateInletVelocity; volumetricFlowRate constant {:.15e}; value uniform ({velocity:.12e} 0 0);}}\n\
 inlet2 {{type flowRateInletVelocity; volumetricFlowRate constant {:.15e}; value uniform ({:.12e} 0 0);}}\n\
 \"(overflow|underflow)\" {{type pressureInletOutletVelocity; value uniform (0 0 0);}}\n\
 wall {{type noSlip;}}\n\
}}",
        qi / 2.0,
        qi / 2.0,
        -velocity
    );
    write_dict(case_dir, "0/U", "volVectorField", &u_body, &mut dict_hashes)?;

    let p_body = format!(
        "dimensions [0 2 -2 0 0 0 0]; internalField uniform 0;\n\
boundaryField {{inlet1 {{type zeroGradient;}} inlet2 {{type zeroGradient;}}\n\
 overflow {{type fixedValue; value uniform {:.15e};}}\n\
 underflow {{type fixedValue; value uniform 0;}} wall {{type zeroGradient;}}}}",
        config.reject_pressure_pa / 998.0
    );
    write_dict(case_dir, "0/p", "volScalarField", &p_body, &mut dict_hashes)?;

    for (name, val, dims, wall_fn) in [
        ("k", k_turb, "0 2 -2", "kqRWallFunction"),
        ("epsilon", epsilon, "0 2 -3", "epsilonWallFunction"),
        ("omega", omega, "0 0 -1", "omegaWallFunction"),
    ] {
        let body = format!(
            "dimensions [{dims} 0 0 0 0]; internalField uniform {val:.12e};\n\
boundaryField {{\"inlet[12]\" {{type fixedValue; value uniform {val:.12e};}}\n\
 \"(overflow|underflow)\" {{type inletOutlet; inletValue uniform {val:.12e}; value uniform {val:.12e};}}\n\
 wall {{type {wall_fn}; value uniform {val:.12e};}}}}"
        );
        write_dict(
            case_dir,
            &format!("0/{name}"),
            "volScalarField",
            &body,
            &mut dict_hashes,
        )?;
    }

    write_dict(
        case_dir,
        "0/nut",
        "volScalarField",
        "dimensions [0 2 -1 0 0 0 0]; internalField uniform 0;\n\
boundaryField {\"(inlet[12]|overflow|underflow)\" {type calculated; value uniform 0;}\n\
 wall {type nutkWallFunction; value uniform 0;}}",
        &mut dict_hashes,
    )?;

    let r_diag = 2.0 * k_turb / 3.0;
    let r_tensor = format!("({r_diag:.12e} 0 0 {r_diag:.12e} 0 {r_diag:.12e})");
    write_dict(
        case_dir,
        "0/R",
        "volSymmTensorField",
        &format!(
            "dimensions [0 2 -2 0 0 0 0]; internalField uniform {r_tensor};\n\
boundaryField {{\"inlet[12]\" {{type fixedValue; value uniform {r_tensor};}}\n\
 \"(overflow|underflow)\" {{type inletOutlet; inletValue uniform {r_tensor}; value uniform {r_tensor};}}\n\
 wall {{type kqRWallFunction; value uniform {r_tensor};}}}}"
        ),
        &mut dict_hashes,
    )?;

    let mut fn_objs = Vec::new();
    for patch_name in ["inlet1", "inlet2", "overflow", "underflow"] {
        for (suffix, op, field) in [("flow", "sum", "phi"), ("pressure", "areaAverage", "p")] {
            fn_objs.push(format!(
                "{patch_name}_{suffix} {{type surfaceFieldValue; libs (\"libfieldFunctionObjects.so\");\n\
writeControl timeStep; writeInterval 1; log false; patch {patch_name};\n\
operation {op}; fields ({field}); writeFields false;}}"
            ));
        }
    }
    let res_fields = if ["SSG", "LRR"].contains(&config.turbulence_model.as_str()) {
        "p U R epsilon"
    } else {
        "p U k omega"
    };
    fn_objs.push(format!(
        "residuals {{type residuals; libs (\"libutilityFunctionObjects.so\");\n\
writeControl timeStep; writeInterval 1; fields ({res_fields});}}"
    ));

    let end_time_val = if config.transient {
        config.end_time_s
    } else {
        config.steps as f64
    };
    let dt_val = if config.transient { config.dt_s } else { 1.0 };
    let write_int = (config.steps / 2).max(1);
    let control_dict = format!(
        "application foamRun; solver incompressibleFluid;\n\
startFrom latestTime; startTime 0; stopAt endTime; endTime {end_time_val}; deltaT {dt_val};\n\
writeControl timeStep; writeInterval {write_int}; purgeWrite 2;\n\
writeFormat ascii; writePrecision 12; writeCompression off; timePrecision 10;\n\
runTimeModifiable false;\n\
functions {{ {} }}",
        fn_objs.join(" ")
    );
    write_dict(
        case_dir,
        "system/controlDict",
        "dictionary",
        &control_dict,
        &mut dict_hashes,
    )?;

    let ddt_scheme = if config.transient {
        "Euler"
    } else {
        "steadyState"
    };
    let fv_schemes = format!(
        "ddtSchemes {{default {ddt_scheme};}}\n\
gradSchemes {{default cellLimited Gauss linear 1;}}\n\
divSchemes {{default none;\n\
 div(phi,U) bounded Gauss upwind;\n\
 div(phi,R) bounded Gauss upwind;\n\
 div(phi,k) bounded Gauss upwind;\n\
 div(phi,epsilon) bounded Gauss upwind;\n\
 div(phi,omega) bounded Gauss upwind;\n\
 div((nuEff*dev2(T(grad(U))))) Gauss linear;\n\
 div(R) Gauss linear;\n\
}}\n\
laplacianSchemes {{default Gauss linear limited .5;}}\n\
interpolationSchemes {{default linear;}} snGradSchemes {{default limited .5;}}\n\
wallDist {{method meshWave;}}"
    );
    write_dict(
        case_dir,
        "system/fvSchemes",
        "dictionary",
        &fv_schemes,
        &mut dict_hashes,
    )?;

    let fv_solution = "solvers {\n\
 p {solver GAMG; tolerance 1e-8; relTol .05; smoother GaussSeidel;}\n\
 pFinal {$p; relTol 0;}\n\
 pcorr {solver GAMG; tolerance 1e-8; relTol 0; smoother GaussSeidel;}\n\
 \"(U|R|k|epsilon|omega)\" {solver smoothSolver; smoother symGaussSeidel; tolerance 1e-8; relTol .05;}\n\
 \"(U|R|k|epsilon|omega)Final\" {solver smoothSolver; smoother symGaussSeidel; tolerance 1e-8; relTol 0;}\n\
}\n\
SIMPLE {nNonOrthogonalCorrectors 1; consistent yes;}\n\
PIMPLE {nOuterCorrectors 1; nCorrectors 1; nNonOrthogonalCorrectors 1; consistent yes;}\n\
relaxationFactors {fields {p .2;} equations {U .4; R .1; k .3; epsilon .3; omega .3;}}";
    write_dict(
        case_dir,
        "system/fvSolution",
        "dictionary",
        fv_solution,
        &mut dict_hashes,
    )?;

    write_dict(
        case_dir,
        "system/decomposeParDict",
        "dictionary",
        &format!(
            "numberOfSubdomains {}; method scotch;",
            config.n_subdomains.max(1)
        ),
        &mut dict_hashes,
    )?;

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

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let ev12_script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
# EV-12 (crates/joil-evidence/reports/EV12-capture-checklist.md):
# 1) Finalize case-identity.json (joil.wave4-case-identity.v1) AFTER the last mesh/0/dictionary/RSM edit
#    and BEFORE solver start.
# 2) Optionally copy all indexed inputs + sidecars to ARCHIVE_DIR and verify both copies (retaining polyMesh).
# 3) Pass --prune-polymesh as 3rd argument ONLY after solver completion to prune constant/polyMesh after
#    re-verifying the disjoint archive copy.
REPO_ROOT="${{JOIL_REPO_ROOT:-{repo_root}}}"
export PYTHONPATH="${{REPO_ROOT}}${{PYTHONPATH:+:$PYTHONPATH}}"
CASE_DIR="${{1:-$(pwd)}}"
ARCHIVE_DIR="${{2:-}}"
PRUNE_FLAG="${{3:-}}"
python3 - "$CASE_DIR" <<'PY'
import sys
from analysis.case_identity import finalize_case_identity, verify_case_identity
finalize_case_identity(sys.argv[1])
print(verify_case_identity(sys.argv[1]))
PY
if [ -n "$ARCHIVE_DIR" ] && [ ! -e "$ARCHIVE_DIR" ]; then
  python3 - "$CASE_DIR" "$ARCHIVE_DIR" <<'PY'
import pathlib, shutil, sys
from analysis.case_identity import MANIFEST, verify_case_identity
from analysis.evidence import strict_json
case = pathlib.Path(sys.argv[1]).resolve()
arch = pathlib.Path(sys.argv[2]).resolve()
if arch.is_relative_to(case) or case.is_relative_to(arch):
    raise SystemExit('archive_dir must be outside case_dir')
verify_case_identity(case)
ident = strict_json((case / MANIFEST).read_bytes())
rels = {{'input-manifest.json', MANIFEST, *ident['source_artifacts'], *ident['mesh_files'], *ident['solver_input_files']}}
arch.mkdir(parents=True, exist_ok=False)
for rel in sorted(rels):
    dst = arch / rel
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(case / rel, dst)
print(verify_case_identity(arch))
verify_case_identity(case)
PY
fi
if [ "$PRUNE_FLAG" = "--prune-polymesh" ]; then
  if [ -z "$ARCHIVE_DIR" ]; then
    echo "refusing to prune polyMesh without verified ARCHIVE_DIR" >&2
    exit 1
  fi
  python3 - "$CASE_DIR" "$ARCHIVE_DIR" <<'PY'
import pathlib, sys
from analysis.case_identity import verify_case_identity
case = pathlib.Path(sys.argv[1]).resolve()
arch = pathlib.Path(sys.argv[2]).resolve()
if arch.is_relative_to(case) or case.is_relative_to(arch):
    raise SystemExit('archive_dir must be outside case_dir')
vc = verify_case_identity(case)
va = verify_case_identity(arch)
if vc['case_hash'] != va['case_hash'] or vc['mesh_hash'] != va['mesh_hash']:
    raise SystemExit('archive verification hashes differ from source case')
PY
  rm -rf "$CASE_DIR/constant/polyMesh"
elif [ -n "$PRUNE_FLAG" ]; then
  echo "unknown option: $PRUNE_FLAG (expected --prune-polymesh)" >&2
  exit 1
fi
"#,
        repo_root = repo_root.display()
    );
    fs::write(
        case_dir.join("ev12-capture-workflow.sh"),
        ev12_script.as_bytes(),
    )
    .map_err(|e| e.to_string())?;

    Ok(manifest)
}

/// Frozen EV-12 Wave-4 OpenFOAM case identity sidecar schema (`crates/joil-evidence/reports/EV12-capture-checklist.md`).
pub const WAVE4_CASE_IDENTITY_SCHEMA: &str = "joil.wave4-case-identity.v1";

/// Sidecar filename written at the root of each finalized OpenFOAM case directory.
pub const WAVE4_CASE_IDENTITY_FILE: &str = "case-identity.json";

/// Exact file digest entry (`{bytes, sha256}`) used in `joil.wave4-case-identity.v1` inventories.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileDigestEntry {
    pub bytes: u64,
    pub sha256: String,
}

/// Complete `joil.wave4-case-identity.v1` sidecar record (`case-identity.json`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave4CaseIdentity {
    pub canonical_genome_hash: String,
    pub canonical_ir_hash: String,
    pub canonical_ir_source_genome_hash: String,
    pub case_hash: String,
    pub compiler: BTreeMap<String, joil_evidence::identity::ContentRef>,
    pub genome_hash: String,
    pub geometry_hash: String,
    pub hash_basis: String,
    pub ir_hash: String,
    pub kind: String,
    pub mesh_files: BTreeMap<String, FileDigestEntry>,
    pub mesh_hash: String,
    pub prepared_manifest: FileDigestEntry,
    pub schema: String,
    pub solver_input_files: BTreeMap<String, FileDigestEntry>,
    pub source_artifacts: BTreeMap<String, FileDigestEntry>,
}

/// Verification result returned by `verify_wave4_case_identity` (`analysis.case_identity.verify_case_identity`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave4CaseVerification {
    pub case: String,
    pub case_hash: String,
    pub mesh_hash: String,
    pub state: String,
}

/// Receipt proving that all indexed inputs + `case-identity.json` were archived to a separate
/// directory and verified via `verify_wave4_case_identity` prior to any `constant/polyMesh` pruning.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave4ArchiveVerificationReceipt {
    pub schema: String,
    pub source_case: String,
    pub archive_dir: String,
    pub case_hash: String,
    pub mesh_hash: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub restored_verification: Wave4CaseVerification,
}

fn check_no_symlink_case_root(case_dir: &Path) -> Result<std::path::PathBuf, String> {
    let meta = fs::symlink_metadata(case_dir).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err("symlink is not an exact case artifact".into());
    }
    if !meta.is_dir() {
        return Err(format!(
            "case directory does not exist: {}",
            case_dir.display()
        ));
    }
    case_dir.canonicalize().map_err(|e| e.to_string())
}

fn safe_case_relpath(case_dir: &Path, relative: &str) -> Result<std::path::PathBuf, String> {
    if relative.is_empty() {
        return Err("case artifact path must be a nonempty string".into());
    }
    let rel_path = Path::new(relative);
    if rel_path.is_absolute() {
        return Err("case artifact path escapes case".into());
    }
    for comp in rel_path.components() {
        match comp {
            std::path::Component::Normal(_) => {}
            _ => return Err("case artifact path escapes case".into()),
        }
    }
    let _canon_case = check_no_symlink_case_root(case_dir)?;
    let mut current = case_dir.to_path_buf();
    for comp in rel_path.components() {
        current.push(comp);
        if let Ok(meta) = fs::symlink_metadata(&current) {
            if meta.file_type().is_symlink() {
                return Err("symlink is not an exact case artifact".into());
            }
        }
    }
    Ok(current)
}

fn digest_file(path: &Path) -> Result<FileDigestEntry, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err("symlink is not an exact case artifact".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    Ok(FileDigestEntry {
        bytes: bytes.len() as u64,
        sha256: joil_ir::sha256_hex(&bytes),
    })
}

fn collect_inventory_recursive(
    case_dir: &Path,
    cur_dir: &Path,
    exclude: &[&str],
    out: &mut BTreeMap<String, FileDigestEntry>,
) -> Result<(), String> {
    let mut entries: Vec<_> = fs::read_dir(cur_dir)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let rel = path
            .strip_prefix(case_dir)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if exclude
            .iter()
            .any(|ex| rel == *ex || rel.starts_with(&format!("{ex}/")))
        {
            continue;
        }
        safe_case_relpath(case_dir, &rel)?;
        let ftype = fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type();
        if ftype.is_symlink() {
            return Err("symlink is not an exact case artifact".into());
        }
        if ftype.is_dir() {
            collect_inventory_recursive(case_dir, &path, exclude, out)?;
        } else if ftype.is_file() {
            out.insert(rel, digest_file(&path)?);
        }
    }
    Ok(())
}

fn build_inventory(
    case_dir: &Path,
    roots: &[&str],
    exclude: &[&str],
) -> Result<BTreeMap<String, FileDigestEntry>, String> {
    let mut found = BTreeMap::new();
    for root in roots {
        let dir = safe_case_relpath(case_dir, root)?;
        let Ok(meta) = fs::symlink_metadata(&dir) else {
            return Err(format!("missing artifact directory: {root}"));
        };
        if meta.file_type().is_symlink() {
            return Err("symlink is not an exact case artifact".into());
        }
        if !meta.is_dir() {
            return Err(format!("missing artifact directory: {root}"));
        }
        collect_inventory_recursive(case_dir, &dir, exclude, &mut found)?;
    }
    if found.is_empty() {
        return Err("empty artifact inventory".into());
    }
    Ok(found)
}

fn require_hash_id(val: Option<&serde_json::Value>) -> Result<String, String> {
    let s = val
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing or invalid canonical source hash".to_string())?;
    if !joil_evidence::identity::is_hash(s) {
        return Err("missing or invalid canonical source hash".into());
    }
    Ok(s.to_string())
}

fn require_content_ref(
    bundle: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<joil_evidence::identity::ContentRef, String> {
    let v = bundle
        .get(key)
        .ok_or_else(|| format!("missing or invalid prepared {key} reference"))?;
    let cref: joil_evidence::identity::ContentRef = serde_json::from_value(v.clone())
        .map_err(|_| format!("missing or invalid prepared {key} reference"))?;
    cref.check()
        .map_err(|_| format!("missing or invalid prepared {key} reference"))?;
    Ok(cref)
}

#[derive(Clone, Debug, PartialEq)]
enum OrderedJson {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<OrderedJson>),
    Object(Vec<(String, OrderedJson)>),
}

impl Serialize for OrderedJson {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Number(n) => n.serialize(serializer),
            Self::String(s) => serializer.serialize_str(s),
            Self::Array(arr) => serializer.collect_seq(arr),
            Self::Object(entries) => serializer.collect_map(entries.iter().map(|(k, v)| (k, v))),
        }
    }
}

impl<'de> Deserialize<'de> for OrderedJson {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct OrderedVisitor;
        impl<'de> serde::de::Visitor<'de> for OrderedVisitor {
            type Value = OrderedJson;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("any valid JSON value")
            }
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
                Ok(OrderedJson::Bool(v))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                Ok(OrderedJson::Number(v.into()))
            }
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(OrderedJson::Number(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(OrderedJson::Number)
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
                Ok(OrderedJson::String(v.to_owned()))
            }
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
                Ok(OrderedJson::String(v))
            }
            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(OrderedJson::Null)
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(OrderedJson::Null)
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut vec = Vec::new();
                while let Some(elem) = seq.next_element()? {
                    vec.push(elem);
                }
                Ok(OrderedJson::Array(vec))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some((k, v)) = map.next_entry()? {
                    entries.push((k, v));
                }
                Ok(OrderedJson::Object(entries))
            }
        }
        deserializer.deserialize_any(OrderedVisitor)
    }
}

/// Build the exact `joil.wave4-case-identity.v1` record from retained artifacts on disk.
///
/// Enforces every path, symlink, canonical-subject, volume-mesh, and solver-input invariant
/// defined in `analysis/case_identity.py` and `crates/joil-evidence/reports/EV12-capture-checklist.md`.
pub fn build_wave4_case_identity(case_dir: &Path) -> Result<Wave4CaseIdentity, String> {
    let prepared_path = safe_case_relpath(case_dir, "input-manifest.json")?;
    let prepared_bytes = fs::read(&prepared_path).map_err(|e| e.to_string())?;
    let prepared_val = joil_evidence::identity::strict_json(&prepared_bytes)?;
    let prepared = prepared_val
        .as_object()
        .ok_or_else(|| "input-manifest.json must contain a JSON object".to_string())?;
    let bundle = prepared
        .get("subject_bundle")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "prepared compiler subject bundle missing".to_string())?;

    let mut artifacts = BTreeMap::new();
    for name in ["design-program.json", "design-ir.json"] {
        let p = safe_case_relpath(case_dir, name)?;
        artifacts.insert(name.to_string(), digest_file(&p)?);
    }
    let surface_rel = prepared
        .get("surface_stl_relpath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "case artifact path must be a nonempty string".to_string())?;
    let surface_path = safe_case_relpath(case_dir, surface_rel)?;
    artifacts.insert(surface_rel.to_string(), digest_file(&surface_path)?);

    for (key, name) in [
        ("genome", "design-program.json"),
        ("ir", "design-ir.json"),
        ("surface_mesh", surface_rel),
    ] {
        let cref = require_content_ref(bundle, key)?;
        if cref.sha256 != artifacts[name].sha256 {
            return Err(format!(
                "prepared {key} digest differs from actual artifact"
            ));
        }
    }
    let prep_stl_sha = prepared
        .get("surface_stl_sha256")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if prep_stl_sha != artifacts[surface_rel].sha256 {
        return Err("prepared surface digest mismatch".into());
    }

    let canonical_genome = require_hash_id(prepared.get("design_sha256"))?;
    let canonical_ir = require_hash_id(prepared.get("design_ir_sha256"))?;

    let prog_path = safe_case_relpath(case_dir, "design-program.json")?;
    let prog_bytes = fs::read(&prog_path).map_err(|e| e.to_string())?;
    let prog_val = joil_evidence::identity::strict_json(&prog_bytes)?;
    if !prog_val.is_object() {
        return Err("design-program.json must contain a JSON object".into());
    }
    let ordered_prog: OrderedJson =
        serde_json::from_slice(&prog_bytes).map_err(|e| e.to_string())?;
    let unsorted_prog_compact = serde_json::to_vec(&ordered_prog).map_err(|e| e.to_string())?;
    let unsorted_prog_hash = joil_ir::sha256_hex(&unsorted_prog_compact);

    let ir_path = safe_case_relpath(case_dir, "design-ir.json")?;
    let ir_bytes = fs::read(&ir_path).map_err(|e| e.to_string())?;
    let ir_val = joil_evidence::identity::strict_json(&ir_bytes)?;
    let ir_obj = ir_val
        .as_object()
        .ok_or_else(|| "design-ir.json must contain a JSON object".to_string())?;

    let ir_source_genome = require_hash_id(ir_obj.get("program_sha256"))?;
    let ir_sha = ir_obj
        .get("ir_sha256")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let sorted_prog_compact = serde_json::to_vec(&prog_val).map_err(|e| e.to_string())?;
    let sorted_prog_hash = joil_ir::sha256_hex(&sorted_prog_compact);
    if ir_sha != canonical_ir
        || canonical_genome != unsorted_prog_hash
        || ir_source_genome != sorted_prog_hash
    {
        return Err("IR and prepared manifest disagree on canonical subjects".into());
    }
    if let Ok(dp) = serde_json::from_slice::<DesignProgram>(&prog_bytes) {
        if dp.canonical_sha256()? != canonical_genome {
            return Err("IR and prepared manifest disagree on canonical subjects".into());
        }
    }

    let mesh = build_inventory(case_dir, &["constant/polyMesh"], &[])?;
    for name in ["points", "faces", "owner", "neighbour", "boundary"] {
        let plain = format!("constant/polyMesh/{name}");
        let gz = format!("constant/polyMesh/{name}.gz");
        if !mesh.contains_key(&plain) && !mesh.contains_key(&gz) {
            return Err(format!("missing executed volume mesh component: {name}"));
        }
    }
    for name in [
        "0/U",
        "0/p",
        "system/controlDict",
        "system/fvSchemes",
        "system/fvSolution",
    ] {
        let p = safe_case_relpath(case_dir, name)?;
        if !p.is_file() {
            return Err(format!("missing solver input: {name}"));
        }
    }
    let inputs = build_inventory(
        case_dir,
        &["0", "constant", "system"],
        &["constant/polyMesh"],
    )?;

    let mesh_hash = joil_evidence::identity::scientific_hash(&serde_json::json!({
        "contract": WAVE4_CASE_IDENTITY_SCHEMA,
        "stage": "volume_mesh",
        "files": &mesh,
    }))?;

    let genome_hash = artifacts["design-program.json"].sha256.clone();
    let ir_hash = artifacts["design-ir.json"].sha256.clone();
    let geometry_hash = artifacts[surface_rel].sha256.clone();

    let mut compiler = BTreeMap::new();
    for key in ["compiler", "geometry_kernel", "compiler_config"] {
        compiler.insert(key.to_string(), require_content_ref(bundle, key)?);
    }

    let case_hash = joil_evidence::identity::scientific_hash(&serde_json::json!({
        "contract": WAVE4_CASE_IDENTITY_SCHEMA,
        "stage": "solver_case",
        "upstream": {
            "genome_hash": &genome_hash,
            "ir_hash": &ir_hash,
            "geometry_hash": &geometry_hash,
            "mesh_hash": &mesh_hash,
        },
        "compiler": &compiler,
        "files": &inputs,
    }))?;

    Ok(Wave4CaseIdentity {
        canonical_genome_hash: canonical_genome,
        canonical_ir_hash: canonical_ir,
        canonical_ir_source_genome_hash: ir_source_genome,
        case_hash,
        compiler,
        genome_hash,
        geometry_hash,
        hash_basis: "genome/IR/geometry exact file bytes; mesh/case domain-separated inventories"
            .to_string(),
        ir_hash,
        kind: "openfoam_artifact_identity_not_qualification".to_string(),
        mesh_files: mesh,
        mesh_hash,
        prepared_manifest: digest_file(&prepared_path)?,
        schema: WAVE4_CASE_IDENTITY_SCHEMA.to_string(),
        solver_input_files: inputs,
        source_artifacts: artifacts,
    })
}

fn run_ev_python_case_identity(
    case_dir: &Path,
    finalize_first: bool,
) -> Result<(Wave4CaseIdentity, Wave4CaseVerification), String> {
    let canon_case = check_no_symlink_case_root(case_dir)?;
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let script = if finalize_first {
        "import json, sys\n\
from analysis.case_identity import finalize_case_identity, verify_case_identity\n\
rec = finalize_case_identity(sys.argv[1])\n\
ver = verify_case_identity(sys.argv[1])\n\
print(json.dumps({'record': rec, 'verification': ver}))\n"
    } else {
        "import json, pathlib, sys\n\
from analysis.case_identity import MANIFEST, verify_case_identity\n\
from analysis.evidence import strict_json\n\
case = pathlib.Path(sys.argv[1])\n\
ver = verify_case_identity(case)\n\
rec = strict_json((case / MANIFEST).read_bytes())\n\
print(json.dumps({'record': rec, 'verification': ver}))\n"
    };
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&canon_case)
        .current_dir(&repo_root)
        .env("PYTHONPATH", &repo_root)
        .output()
        .map_err(|e| format!("failed to invoke EV case_identity Python API: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "EV analysis.case_identity failed ({}): {}{}",
            output.status, stderr, stdout
        ));
    }
    let payload: serde_json::Value = joil_evidence::identity::strict_json(&output.stdout)?;
    let rec: Wave4CaseIdentity = serde_json::from_value(payload["record"].clone())
        .map_err(|e| format!("decode EV Wave4CaseIdentity: {e}"))?;
    let ver: Wave4CaseVerification = serde_json::from_value(payload["verification"].clone())
        .map_err(|e| format!("decode EV Wave4CaseVerification: {e}"))?;
    Ok((rec, ver))
}

/// Finalize `case-identity.json` (`joil.wave4-case-identity.v1`) after the last
/// `mesh`/`0/`/`dictionary`/`RSM` edit and before solver execution or cleanup,
/// using EV's `analysis.case_identity.finalize_case_identity` and `verify_case_identity` API
/// (`crates/joil-evidence/reports/EV12-capture-checklist.md`) and cross-verifying in Rust.
pub fn finalize_wave4_case_identity(case_dir: &Path) -> Result<Wave4CaseIdentity, String> {
    let rust_expected = build_wave4_case_identity(case_dir)?;
    let sidecar_path = safe_case_relpath(case_dir, WAVE4_CASE_IDENTITY_FILE)?;
    if sidecar_path.exists() {
        let existing_bytes = fs::read(&sidecar_path).map_err(|e| e.to_string())?;
        let existing: Wave4CaseIdentity = joil_evidence::identity::strict_decode(&existing_bytes)?;
        if existing != rust_expected {
            return Err("existing case identity is stale; never overwrite it".into());
        }
    }
    let (py_record, py_ver) = run_ev_python_case_identity(case_dir, true)?;
    if py_record != rust_expected {
        return Err("Rust and EV Python joil.wave4-case-identity.v1 records diverged".into());
    }
    if py_ver.state != "exact_artifact_bytes_verified"
        || py_ver.case_hash != rust_expected.case_hash
        || py_ver.mesh_hash != rust_expected.mesh_hash
    {
        return Err("EV verify_case_identity did not confirm exact_artifact_bytes_verified".into());
    }
    Ok(py_record)
}

/// Verify an existing `case-identity.json` (`joil.wave4-case-identity.v1`) against
/// retained case artifacts using both Rust and EV's `analysis.case_identity.verify_case_identity`.
pub fn verify_wave4_case_identity(case_dir: &Path) -> Result<Wave4CaseVerification, String> {
    let sidecar_path = safe_case_relpath(case_dir, WAVE4_CASE_IDENTITY_FILE)?;
    if !sidecar_path.is_file() {
        return Err("missing case-identity.json sidecar".into());
    }
    let recorded_bytes = fs::read(&sidecar_path).map_err(|e| e.to_string())?;
    let recorded: Wave4CaseIdentity = joil_evidence::identity::strict_decode(&recorded_bytes)?;
    let actual = build_wave4_case_identity(case_dir)?;
    if recorded != actual {
        return Err("case identity differs from retained artifacts".into());
    }
    let (py_record, py_ver) = run_ev_python_case_identity(case_dir, false)?;
    if py_record != actual || py_ver.state != "exact_artifact_bytes_verified" {
        return Err("EV verify_case_identity failed".into());
    }
    Ok(py_ver)
}

fn collect_archived_case_inventory(
    case_dir: &Path,
    identity: &Wave4CaseIdentity,
) -> Result<BTreeMap<String, FileDigestEntry>, String> {
    let sidecar_digest = digest_file(&safe_case_relpath(case_dir, WAVE4_CASE_IDENTITY_FILE)?)?;
    let mut all_files: BTreeMap<String, FileDigestEntry> = BTreeMap::new();
    all_files.insert(
        "input-manifest.json".to_string(),
        identity.prepared_manifest.clone(),
    );
    all_files.insert(WAVE4_CASE_IDENTITY_FILE.to_string(), sidecar_digest);
    for inv in [
        &identity.source_artifacts,
        &identity.mesh_files,
        &identity.solver_input_files,
    ] {
        for (rel, dig) in inv {
            if let Some(prev) = all_files.insert(rel.clone(), dig.clone()) {
                if prev != *dig {
                    return Err(format!("conflicting inventory digest for {rel}"));
                }
            }
        }
    }
    Ok(all_files)
}

/// Archive all indexed case inputs (`input-manifest.json`, `case-identity.json`,
/// `source_artifacts`, `mesh_files`, `solver_input_files`) to a separate directory and
/// run `verify_wave4_case_identity` on the restored copy per `EV12-capture-checklist.md`.
pub fn archive_and_verify_wave4_case_inputs(
    case_dir: &Path,
    archive_dir: &Path,
) -> Result<Wave4ArchiveVerificationReceipt, String> {
    let canon_case = check_no_symlink_case_root(case_dir)?;
    if let Ok(meta) = fs::symlink_metadata(archive_dir) {
        if meta.file_type().is_symlink() {
            return Err("archive_dir cannot be a symlink".into());
        }
        return Err(format!(
            "archive_dir already exists; refusing to overwrite: {}",
            archive_dir.display()
        ));
    }
    let _ = verify_wave4_case_identity(case_dir)?;
    let identity = build_wave4_case_identity(case_dir)?;
    let all_files = collect_archived_case_inventory(case_dir, &identity)?;

    fs::create_dir_all(archive_dir).map_err(|e| e.to_string())?;
    let canon_arch = check_no_symlink_case_root(archive_dir)?;
    if canon_arch.starts_with(&canon_case) || canon_case.starts_with(&canon_arch) {
        let _ = fs::remove_dir_all(archive_dir);
        return Err("archive_dir must be disjoint from case_dir".into());
    }

    for (rel, expected_dig) in &all_files {
        let src = safe_case_relpath(case_dir, rel)?;
        let dst = archive_dir.join(rel);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(&src, &dst).map_err(|e| e.to_string())?;
        let copied_dig = digest_file(&dst)?;
        if copied_dig != *expected_dig {
            return Err(format!("archived file digest mismatch for {rel}"));
        }
    }

    let restored_ver = verify_wave4_case_identity(archive_dir)?;
    let source_ver = verify_wave4_case_identity(case_dir)?;
    if restored_ver.case_hash != source_ver.case_hash
        || restored_ver.mesh_hash != source_ver.mesh_hash
    {
        return Err("restored archive verification hashes differ from source case".into());
    }

    let total_bytes: u64 = all_files.values().map(|d| d.bytes).sum();
    let receipt = Wave4ArchiveVerificationReceipt {
        schema: "joil.wave4-case-archive-verification.v1".to_string(),
        source_case: case_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("case")
            .to_string(),
        archive_dir: archive_dir.display().to_string(),
        case_hash: identity.case_hash,
        mesh_hash: identity.mesh_hash,
        file_count: all_files.len(),
        total_bytes,
        restored_verification: restored_ver,
    };
    let receipt_json = serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?;
    fs::write(
        archive_dir.join("ev12-archive-verification-receipt.json"),
        format!("{receipt_json}\n"),
    )
    .map_err(|e| e.to_string())?;
    Ok(receipt)
}

/// Prune `constant/polyMesh` from `case_dir` ONLY after verifying that:
/// 1. `case_dir` has a valid, non-stale `case-identity.json` (`verify_wave4_case_identity(case_dir)`), and
/// 2. `archive_dir` is a disjoint directory whose restored copy passes `verify_wave4_case_identity(archive_dir)`
///    with identical `Wave4CaseIdentity` (`case_hash`, `mesh_hash`, and all file inventories).
///
/// Refuses to delete `constant/polyMesh` if either verification fails.
pub fn prune_polymesh_after_verified_archive(
    case_dir: &Path,
    archive_dir: &Path,
) -> Result<Wave4ArchiveVerificationReceipt, String> {
    let canon_case = check_no_symlink_case_root(case_dir)?;
    let canon_arch = check_no_symlink_case_root(archive_dir)
        .map_err(|e| format!("refusing to prune polyMesh without verified archive_dir: {e}"))?;
    if canon_arch.starts_with(&canon_case) || canon_case.starts_with(&canon_arch) {
        return Err(
            "refusing to prune polyMesh: archive_dir must be disjoint from case_dir".into(),
        );
    }

    let case_ver = verify_wave4_case_identity(case_dir)
        .map_err(|e| format!("refusing to prune polyMesh: source case unverified: {e}"))?;
    let arch_ver = verify_wave4_case_identity(archive_dir)
        .map_err(|e| format!("refusing to prune polyMesh: archive copy unverified: {e}"))?;

    let case_id = build_wave4_case_identity(case_dir)?;
    let arch_id = build_wave4_case_identity(archive_dir)?;
    if case_id != arch_id
        || case_ver.case_hash != arch_ver.case_hash
        || case_ver.mesh_hash != arch_ver.mesh_hash
    {
        return Err(
            "refusing to prune polyMesh: archive Wave4CaseIdentity does not match source case"
                .into(),
        );
    }

    let all_files = collect_archived_case_inventory(archive_dir, &arch_id)?;
    let file_count = all_files.len();
    let total_bytes: u64 = all_files.values().map(|d| d.bytes).sum();

    let poly_mesh_dir = safe_case_relpath(case_dir, "constant/polyMesh")?;
    fs::remove_dir_all(&poly_mesh_dir).map_err(|e| e.to_string())?;

    Ok(Wave4ArchiveVerificationReceipt {
        schema: "joil.wave4-case-archive-verification.v1".to_string(),
        source_case: case_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("case")
            .to_string(),
        archive_dir: archive_dir.display().to_string(),
        case_hash: arch_id.case_hash,
        mesh_hash: arch_id.mesh_hash,
        file_count,
        total_bytes,
        restored_verification: arch_ver,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::program::{DesignProgram, InletSplit3D};

    #[test]
    fn ir_r1_openfoam_cfd_stl_is_ir_compiled_mesh_and_slices_to_3_inlet_loops_per_duct() {
        let mut prog = DesignProgram::baseline_ct35();
        prog.inlet_split = InletSplit3D {
            enabled: true,
            vane_thickness_mm: 1.2,
            vane_z_fraction: 0.5,
            branches_per_port: 3,
            azimuthal_phase_deg: 12.0,
        };
        prog.skimmer.length_mm = 5.5;
        prog.skimmer.outer_radius_mm = 2.5;
        prog.skimmer.capture_flutes = 4;

        let tmp_dir =
            std::env::temp_dir().join(format!("joil_ir_r1_cfd_case_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp_dir);

        let cfg = CfdCaseConfig::default();
        let manifest = build_openfoam_candidate_case(&prog, &tmp_dir, &cfg)
            .expect("3-branch 6jet OpenFOAM case generation must succeed");

        let receipt = manifest
            .enactment_receipt
            .as_ref()
            .expect("enactment_receipt must be present");
        assert!(receipt.all_active_features_enacted);
        assert_eq!(receipt.measured_disjoint_inlet_paths, 6);
        assert_eq!(receipt.expected_disjoint_inlet_paths, 6);

        let taxon = manifest
            .derived_taxon
            .as_ref()
            .expect("derived_taxon must be present");
        assert_eq!(taxon.measured_inlet_channel_count, 6);
        assert_eq!(taxon.betti_1, 4);

        let stl_path = tmp_dir.join(&manifest.surface_stl_relpath);
        let stl_text = fs::read_to_string(&stl_path).expect("read constant/triSurface/ct35.stl");
        let (total_loops, loops_inlet1, loops_inlet2) =
            joil_ir::IrCompiledMesh::audit_ascii_stl_inlet_enactment(&stl_text);
        assert_eq!(loops_inlet1, 3, "inlet1 duct must slice to 3 closed loops");
        assert_eq!(loops_inlet2, 3, "inlet2 duct must slice to 3 closed loops");
        assert_eq!(total_loops, 6, "total disjoint inlet loops must equal 6");

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn int06_wave4_top_winners_cfd_stl_equals_ir_compiled_mesh_and_slice_loops() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let wave4_path = manifest_dir.join("../../docs/results/wave4-evolution-proof-v1.json");
        let wave4_val: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&wave4_path).expect("read wave4 json"))
                .expect("parse wave4 json");

        let cases = [
            (
                "wave4_top_elite_s20261108_g016_i000_8jet",
                &wave4_val["promoted_wave3_top_elite"]["program"],
                4usize,
            ),
            (
                "wave4_trifurcated_6jet_vortex_head",
                &wave4_val["promoted_wave3_elites_by_family"]["trifurcated-6jet-vortex-head"]
                    ["program"],
                3usize,
            ),
            (
                "wave4_counter_fluted_recovery_vortex_6jet",
                &wave4_val["promoted_wave3_elites_by_family"]["counter-fluted-recovery-vortex"]
                    ["program"],
                3usize,
            ),
            (
                "wave4_baseline_ct35_control_2jet",
                &wave4_val["baseline_b0_ct35"]["program"],
                1usize,
            ),
        ];

        let mut case_records = Vec::new();
        for (label, prog_val, expected_loops_per_duct) in cases {
            let prog: DesignProgram =
                serde_json::from_value(prog_val.clone()).expect("deserialize DesignProgram");
            let p_norm = prog.normalize().expect("normalize DesignProgram");

            let ir = crate::geometry::compile_design_ir(&p_norm).expect("compile DesignIrV1");
            let ir_mesh = joil_ir::IrCompiledMesh::compile_with_extension(
                &ir,
                joil_ir::BackendId::RustMultiPatchCfdSurface,
                0.0,
            )
            .expect("compile IrCompiledMesh");

            let tmp_dir =
                std::env::temp_dir().join(format!("joil_int06_{label}_{}", std::process::id()));
            let _ = fs::remove_dir_all(&tmp_dir);

            let cfg = CfdCaseConfig::default();
            let manifest = build_openfoam_candidate_case(&p_norm, &tmp_dir, &cfg)
                .expect("build_openfoam_candidate_case must succeed");

            // 1. Verify MultiPatchSurface::from_program writes the exact same STL mesh as IrCompiledMesh::compile_with_extension
            let direct_surface = MultiPatchSurface::from_program(&p_norm, 0.0).unwrap();
            assert_eq!(
                direct_surface.vertices_m, ir_mesh.vertices_m,
                "{label}: MultiPatchSurface vertices must equal IrCompiledMesh::compile_with_extension"
            );
            assert_eq!(
                direct_surface.triangles, ir_mesh.triangles,
                "{label}: MultiPatchSurface triangles must equal IrCompiledMesh::compile_with_extension"
            );

            // 2. Verify slice loops at x = +-0.045 m and x = +-0.050 m on both IrCompiledMesh and emitted OpenFOAM STL
            let loops_neg_045 = ir_mesh.count_cross_section_loops_at_x(-0.045);
            let loops_pos_045 = ir_mesh.count_cross_section_loops_at_x(0.045);
            let loops_neg_050 = ir_mesh.count_cross_section_loops_at_x(-0.050);
            let loops_pos_050 = ir_mesh.count_cross_section_loops_at_x(0.050);
            assert_eq!(
                loops_neg_045, expected_loops_per_duct,
                "{label}: x = -0.045 m must have {expected_loops_per_duct} closed loops"
            );
            assert_eq!(
                loops_pos_045, expected_loops_per_duct,
                "{label}: x = +0.045 m must have {expected_loops_per_duct} closed loops"
            );
            assert_eq!(
                loops_neg_050, expected_loops_per_duct,
                "{label}: x = -0.050 m must have {expected_loops_per_duct} closed loops"
            );
            assert_eq!(
                loops_pos_050, expected_loops_per_duct,
                "{label}: x = +0.050 m must have {expected_loops_per_duct} closed loops"
            );

            let stl_path = tmp_dir.join(&manifest.surface_stl_relpath);
            let stl_text = fs::read_to_string(&stl_path).expect("read ct35.stl");
            let (total_loops, loops_in1, loops_in2) =
                joil_ir::IrCompiledMesh::audit_ascii_stl_inlet_enactment(&stl_text);
            assert_eq!(loops_in1, expected_loops_per_duct);
            assert_eq!(loops_in2, expected_loops_per_duct);
            assert_eq!(total_loops, 2 * expected_loops_per_duct);

            let taxon = manifest.derived_taxon.as_ref().unwrap();
            case_records.push(serde_json::json!({
                "label": label,
                "topology_class": manifest.topology_class,
                "design_sha256": manifest.design_sha256,
                "design_ir_sha256": manifest.design_ir_sha256,
                "surface_stl_sha256": manifest.surface_stl_sha256,
                "surface_stl_bytes": manifest.surface_stl_bytes,
                "vertex_count": ir_mesh.vertices_m.len(),
                "triangle_count": ir_mesh.triangles.len(),
                "equals_ir_compiled_mesh_vertices": true,
                "equals_ir_compiled_mesh_triangles": true,
                "watertight_and_manifold": manifest.multi_patch_audit.watertight_and_manifold,
                "slice_loops_x_neg_0_045_m": loops_neg_045,
                "slice_loops_x_pos_0_045_m": loops_pos_045,
                "slice_loops_x_neg_0_050_m": loops_neg_050,
                "slice_loops_x_pos_0_050_m": loops_pos_050,
                "stl_audit_inlet1_loops": loops_in1,
                "stl_audit_inlet2_loops": loops_in2,
                "stl_audit_total_inlet_loops": total_loops,
                "expected_loops_per_duct": expected_loops_per_duct,
                "betti": [taxon.betti_0, taxon.betti_1, taxon.betti_2],
                "taxon_signature": taxon.taxon_signature,
                "derived_cluster": taxon.derived_cluster,
            }));

            let _ = fs::remove_dir_all(&tmp_dir);
        }

        let checks_dir = manifest_dir.join("../../.private/checks");
        let _ = fs::create_dir_all(&checks_dir);
        let receipt_path = checks_dir.join("int06-wave4-winner-cfd-stl-ir-parity-20261016.json");
        let receipt_doc = serde_json::json!({
            "schema": "joil.ir.int06_wave4_cfd_stl_parity_receipt.v1",
            "source_wave4_receipt": "docs/results/wave4-evolution-proof-v1.json",
            "generator_test": "joil_core::cfd_case::tests::int06_wave4_top_winners_cfd_stl_equals_ir_compiled_mesh_and_slice_loops",
            "cases": case_records,
        });
        let _ = fs::write(
            &receipt_path,
            format!("{}\n", serde_json::to_string_pretty(&receipt_doc).unwrap()),
        );
    }

    #[test]
    fn ev12_prepare_finalize_after_mesh_and_rsm_edits_and_refuse_polymesh_prune_until_archive_verified(
    ) {
        let mut prog = DesignProgram::baseline_ct35();
        prog.inlet_split = InletSplit3D {
            enabled: true,
            vane_thickness_mm: 1.2,
            vane_z_fraction: 0.5,
            branches_per_port: 3,
            azimuthal_phase_deg: 10.0,
        };
        let p_norm = prog.normalize().expect("normalize");

        let root_tmp = std::env::temp_dir().join(format!("joil_ev12_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root_tmp);
        fs::create_dir_all(&root_tmp).expect("create root_tmp");

        let case_dir = root_tmp.join("wave4-synthetic-rsm-continuation");
        let archive_dir = root_tmp.join("wave4-synthetic-rsm-continuation-archive");
        let cfg = CfdCaseConfig::default();

        // 1. Prepare candidate case via build_openfoam_candidate_case (prepare-cfd-case)
        let _manifest =
            build_openfoam_candidate_case(&p_norm, &case_dir, &cfg).expect("prepare case");
        assert!(case_dir.join("ev12-capture-workflow.sh").is_file());

        // 2. Finalizing BEFORE constant/polyMesh exists must fail closed
        assert!(
            finalize_wave4_case_identity(&case_dir).is_err(),
            "finalizing before volume mesh exists must fail closed"
        );

        // 3. Populate synthetic constant/polyMesh (points, faces, owner, neighbour, boundary + cellZones)
        //    and apply post-prepare dictionary + 0/ + RSM edits BEFORE solver start
        let poly_dir = case_dir.join("constant/polyMesh");
        fs::create_dir_all(&poly_dir).expect("create polyMesh");
        for name in [
            "points",
            "faces",
            "owner",
            "neighbour",
            "boundary",
            "cellZones",
        ] {
            fs::write(
                poly_dir.join(name),
                format!("synthetic polyMesh {name} payload\n").as_bytes(),
            )
            .expect("write polyMesh file");
        }
        // Dictionary edit (e.g., G1 grid sizing in system/blockMeshDict)
        let bm_path = case_dir.join("system/blockMeshDict");
        let bm_orig = fs::read_to_string(&bm_path).expect("read blockMeshDict");
        fs::write(&bm_path, format!("{bm_orig}\n// synthetic G1 grid edit\n"))
            .expect("write edited blockMeshDict");
        // 0/ and RSM field conversion edits (0/U, 0/R, 0/epsilon)
        for field in ["0/U", "0/R", "0/epsilon"] {
            let fpath = case_dir.join(field);
            let forig = fs::read_to_string(&fpath).expect("read 0/ field");
            fs::write(&fpath, format!("{forig}\n// synthetic RSM field edit\n"))
                .expect("write edited 0/ field");
        }

        // 4. Finalize case-identity.json (joil.wave4-case-identity.v1) after the last edit and before solver start
        let identity = finalize_wave4_case_identity(&case_dir)
            .expect("finalize_wave4_case_identity must succeed after mesh/0/dictionary/RSM edits");
        assert_eq!(identity.schema, WAVE4_CASE_IDENTITY_SCHEMA);
        assert!(case_dir.join(WAVE4_CASE_IDENTITY_FILE).is_file());
        assert!(identity
            .mesh_files
            .contains_key("constant/polyMesh/cellZones"));

        // Identical retry must be allowed and return the exact same identity
        let retry_identity =
            finalize_wave4_case_identity(&case_dir).expect("identical retry must succeed");
        assert_eq!(retry_identity, identity);

        let ver = verify_wave4_case_identity(&case_dir).expect("verify_wave4_case_identity");
        assert_eq!(ver.state, "exact_artifact_bytes_verified");
        assert_eq!(ver.case_hash, identity.case_hash);
        assert_eq!(ver.mesh_hash, identity.mesh_hash);

        // Post-finalization edits to mesh, 0/, or dictionary must be rejected by both verify and finalize
        for rel in ["constant/polyMesh/points", "0/U", "system/fvSchemes"] {
            let p = case_dir.join(rel);
            let orig = fs::read(&p).expect("read original");
            let mut mutated = orig.clone();
            mutated.extend_from_slice(b" tampered");
            fs::write(&p, &mutated).expect("write tampered");
            assert!(
                verify_wave4_case_identity(&case_dir).is_err(),
                "verify must reject tampered {rel}"
            );
            assert!(
                finalize_wave4_case_identity(&case_dir).is_err(),
                "finalize must refuse to overwrite stale sidecar after tampering {rel}"
            );
            fs::write(&p, &orig).expect("restore original");
        }

        // 5. Refuse to prune constant/polyMesh until the archive is verified
        assert!(
            prune_polymesh_after_verified_archive(&case_dir, &archive_dir).is_err(),
            "must refuse to prune polyMesh when archive_dir does not exist yet"
        );
        assert!(
            case_dir.join("constant/polyMesh/points").is_file(),
            "polyMesh must remain intact after refused prune"
        );

        // Archive and verify restored copy in archive_dir
        let arch_receipt = archive_and_verify_wave4_case_inputs(&case_dir, &archive_dir)
            .expect("archive_and_verify_wave4_case_inputs must succeed");
        assert_eq!(
            arch_receipt.restored_verification.state,
            "exact_artifact_bytes_verified"
        );
        assert_eq!(arch_receipt.case_hash, identity.case_hash);
        assert_eq!(arch_receipt.mesh_hash, identity.mesh_hash);

        // Corrupting the archive copy must cause prune_polymesh_after_verified_archive to refuse pruning
        let arch_points = archive_dir.join("constant/polyMesh/points");
        let arch_points_orig = fs::read(&arch_points).expect("read arch points");
        fs::write(&arch_points, b"corrupted archive mesh").expect("corrupt arch points");
        assert!(
            prune_polymesh_after_verified_archive(&case_dir, &archive_dir).is_err(),
            "must refuse to prune polyMesh when archive copy fails verification"
        );
        assert!(
            case_dir.join("constant/polyMesh/points").is_file(),
            "polyMesh must remain intact when archive verification fails"
        );
        fs::write(&arch_points, &arch_points_orig).expect("restore arch points");

        // Now that archive is verified, prune_polymesh_after_verified_archive succeeds and removes case_dir/constant/polyMesh
        let prune_receipt = prune_polymesh_after_verified_archive(&case_dir, &archive_dir)
            .expect("prune_polymesh_after_verified_archive must succeed after verified archive");
        assert_eq!(prune_receipt.case_hash, identity.case_hash);
        // EV12-WR-01: prune receipt must report exact deduplicated archive inventory including case-identity.json
        assert_eq!(
            prune_receipt.file_count, arch_receipt.file_count,
            "EV12-WR-01: prune receipt file_count must equal archive receipt file_count"
        );
        assert_eq!(
            prune_receipt.total_bytes, arch_receipt.total_bytes,
            "EV12-WR-01: prune receipt total_bytes must equal archive receipt total_bytes"
        );
        assert!(!case_dir.join("constant/polyMesh").exists());

        // The retained archive copy still passes full EV-12 verification
        let arch_ver_after = verify_wave4_case_identity(&archive_dir)
            .expect("retained archive copy must remain verifiable after source polyMesh prune");
        assert_eq!(arch_ver_after.state, "exact_artifact_bytes_verified");

        let _ = fs::remove_dir_all(&root_tmp);
    }

    #[test]
    fn ev12_wr01_to_wr05_regression_suite() {
        let mut prog = DesignProgram::baseline_ct35();
        prog.inlet_split = InletSplit3D {
            enabled: true,
            vane_thickness_mm: 1.2,
            vane_z_fraction: 0.5,
            branches_per_port: 3,
            azimuthal_phase_deg: 10.0,
        };
        let p_norm = prog.normalize().expect("normalize");
        let cfg = CfdCaseConfig::default();

        let root_tmp =
            std::env::temp_dir().join(format!("joil_ev12_wr_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root_tmp);
        fs::create_dir_all(&root_tmp).expect("create root_tmp");

        let populate_synthetic_polymesh = |case_dir: &Path| {
            let poly_dir = case_dir.join("constant/polyMesh");
            fs::create_dir_all(&poly_dir).expect("create polyMesh");
            for name in [
                "points",
                "faces",
                "owner",
                "neighbour",
                "boundary",
                "cellZones",
            ] {
                fs::write(
                    poly_dir.join(name),
                    format!("synthetic polyMesh {name} payload\n").as_bytes(),
                )
                .expect("write polyMesh file");
            }
        };

        // --- EV12-WR-03: build_wave4_case_identity and finalize_wave4_case_identity must reject forged canonical_genome on non-DesignProgram JSON ---
        {
            let forged_case = root_tmp.join("wr03-forged-canonical");
            build_openfoam_candidate_case(&p_norm, &forged_case, &cfg)
                .expect("prepare forged_case");
            populate_synthetic_polymesh(&forged_case);

            let prog_path = forged_case.join("design-program.json");
            fs::write(
                &prog_path,
                b"{\n  \"synthetic\": true,\n  \"z\": 2,\n  \"a\": 0\n}\n",
            )
            .expect("write synthetic non-DesignProgram JSON");
            let prog_bytes = fs::read(&prog_path).expect("read prog_path");
            let prog_sha = joil_ir::sha256_hex(&prog_bytes);
            let ordered_prog: OrderedJson =
                serde_json::from_slice(&prog_bytes).expect("parse OrderedJson");
            let valid_canonical_genome = joil_ir::sha256_hex(
                &serde_json::to_vec(&ordered_prog).expect("serialize OrderedJson"),
            );
            let sorted_prog: serde_json::Value =
                serde_json::from_slice(&prog_bytes).expect("parse Value");
            let valid_ir_source_genome =
                joil_ir::sha256_hex(&serde_json::to_vec(&sorted_prog).expect("serialize Value"));

            let ir_path = forged_case.join("design-ir.json");
            let mut ir_doc: serde_json::Value =
                serde_json::from_slice(&fs::read(&ir_path).expect("read ir")).expect("parse ir");
            ir_doc["program_sha256"] = serde_json::Value::String(valid_ir_source_genome);
            let ir_bytes =
                format!("{}\n", serde_json::to_string_pretty(&ir_doc).unwrap()).into_bytes();
            let ir_file_sha = joil_ir::sha256_hex(&ir_bytes);
            fs::write(&ir_path, &ir_bytes).expect("write updated design-ir.json");

            let im_path = forged_case.join("input-manifest.json");
            let mut im: serde_json::Value =
                serde_json::from_slice(&fs::read(&im_path).expect("read im")).expect("parse im");
            im["design_sha256"] = serde_json::Value::String("f".repeat(64));
            im["subject_bundle"]["genome"]["sha256"] = serde_json::Value::String(prog_sha);
            im["subject_bundle"]["ir"]["sha256"] = serde_json::Value::String(ir_file_sha);
            fs::write(
                &im_path,
                format!("{}\n", serde_json::to_string_pretty(&im).unwrap()),
            )
            .expect("write forged im");

            let err = build_wave4_case_identity(&forged_case)
                .expect_err("EV12-WR-03: build_wave4_case_identity must reject forged canonical_genome on non-DesignProgram JSON");
            assert!(
                err.contains("canonical subjects"),
                "EV12-WR-03: expected canonical subjects error, got: {err}"
            );
            assert!(
                finalize_wave4_case_identity(&forged_case).is_err(),
                "EV12-WR-03: finalize_wave4_case_identity must reject forged canonical_genome on non-DesignProgram JSON"
            );

            // Restoring the valid canonical_genome makes Rust builder accept it
            im["design_sha256"] = serde_json::Value::String(valid_canonical_genome);
            fs::write(
                &im_path,
                format!("{}\n", serde_json::to_string_pretty(&im).unwrap()),
            )
            .expect("write valid im");
            assert!(
                build_wave4_case_identity(&forged_case).is_ok(),
                "EV12-WR-03: build_wave4_case_identity must accept valid canonical_genome on non-DesignProgram JSON"
            );
        }

        // --- EV12-WR-02: relative case_dir from foreign working directory must resolve against caller cwd, not repo_root ---
        {
            let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(|p| p.parent())
                .expect("repo root")
                .to_path_buf();
            let rel_subpath = format!(
                ".private/ev12_wr02_rel_probe_{}/target_case",
                std::process::id()
            );
            let repo_rel_target = repo_root.join(&rel_subpath);
            let foreign_cwd = root_tmp.join("foreign_cwd");
            let foreign_target = foreign_cwd.join(&rel_subpath);

            let _ = fs::remove_dir_all(repo_root.join(format!(
                ".private/ev12_wr02_rel_probe_{}",
                std::process::id()
            )));
            build_openfoam_candidate_case(&p_norm, &repo_rel_target, &cfg)
                .expect("prepare repo_rel_target");
            populate_synthetic_polymesh(&repo_rel_target);
            build_openfoam_candidate_case(&p_norm, &foreign_target, &cfg)
                .expect("prepare foreign_target");
            populate_synthetic_polymesh(&foreign_target);

            let orig_cwd = std::env::current_dir().expect("get current_dir");
            std::env::set_current_dir(&foreign_cwd).expect("set_current_dir to foreign_cwd");
            let finalize_res = finalize_wave4_case_identity(Path::new(&rel_subpath));
            let verify_res = verify_wave4_case_identity(Path::new(&rel_subpath));
            std::env::set_current_dir(&orig_cwd).expect("restore orig_cwd");

            let _ =
                finalize_res.expect("EV12-WR-02: relative finalize from foreign cwd must succeed");
            let ver =
                verify_res.expect("EV12-WR-02: relative verify from foreign cwd must succeed");
            assert_eq!(ver.state, "exact_artifact_bytes_verified");
            assert!(
                foreign_target.join(WAVE4_CASE_IDENTITY_FILE).is_file(),
                "EV12-WR-02: foreign_target must receive case-identity.json"
            );
            assert!(
                !repo_rel_target.join(WAVE4_CASE_IDENTITY_FILE).exists(),
                "EV12-WR-02: repo_rel_target must NOT be touched when caller cwd is foreign_cwd"
            );

            let _ = fs::remove_dir_all(repo_root.join(format!(
                ".private/ev12_wr02_rel_probe_{}",
                std::process::id()
            )));
        }

        // --- EV12-WR-04 & EV12-WR-05: emitted ev12-capture-workflow.sh works from case cwd without PYTHONPATH, does not prune before solver, and prunes only with --prune-polymesh ---
        {
            let wf_case = root_tmp.join("wr04_05_workflow_case");
            let wf_archive = root_tmp.join("wr04_05_workflow_archive");
            build_openfoam_candidate_case(&p_norm, &wf_case, &cfg).expect("prepare wf_case");
            populate_synthetic_polymesh(&wf_case);

            // EV12-WR-04: Run `bash ev12-capture-workflow.sh` with no args from inside wf_case and without PYTHONPATH
            let out_no_args = std::process::Command::new("bash")
                .arg("ev12-capture-workflow.sh")
                .current_dir(&wf_case)
                .env_remove("PYTHONPATH")
                .output()
                .expect("spawn bash ev12-capture-workflow.sh");
            assert!(
                out_no_args.status.success(),
                "EV12-WR-04: ev12-capture-workflow.sh from case cwd without PYTHONPATH failed: stdout={} stderr={}",
                String::from_utf8_lossy(&out_no_args.stdout),
                String::from_utf8_lossy(&out_no_args.stderr)
            );
            assert!(wf_case.join(WAVE4_CASE_IDENTITY_FILE).is_file());
            assert!(wf_case.join("constant/polyMesh/points").is_file());

            // EV12-WR-05 (pre-solver capture with ARCHIVE_DIR): must archive + verify AND retain constant/polyMesh
            let out_capture = std::process::Command::new("bash")
                .arg(wf_case.join("ev12-capture-workflow.sh"))
                .arg(&wf_case)
                .arg(&wf_archive)
                .current_dir(&wf_case)
                .env_remove("PYTHONPATH")
                .output()
                .expect("spawn bash ev12-capture-workflow.sh with archive_dir");
            assert!(
                out_capture.status.success(),
                "EV12-WR-05: pre-solver archive capture failed: stdout={} stderr={}",
                String::from_utf8_lossy(&out_capture.stdout),
                String::from_utf8_lossy(&out_capture.stderr)
            );
            assert!(
                wf_archive.join(WAVE4_CASE_IDENTITY_FILE).is_file(),
                "EV12-WR-05: archive_dir must contain verified case-identity.json"
            );
            assert!(
                wf_case.join("constant/polyMesh/points").is_file(),
                "EV12-WR-05: pre-solver capture must NOT implicitly delete constant/polyMesh before solver runs"
            );

            // EV12-WR-05 (explicit post-solver --prune-polymesh): must verify archive and prune constant/polyMesh
            let out_prune = std::process::Command::new("bash")
                .arg(wf_case.join("ev12-capture-workflow.sh"))
                .arg(&wf_case)
                .arg(&wf_archive)
                .arg("--prune-polymesh")
                .current_dir(&wf_case)
                .env_remove("PYTHONPATH")
                .output()
                .expect("spawn bash ev12-capture-workflow.sh --prune-polymesh");
            assert!(
                out_prune.status.success(),
                "EV12-WR-05: explicit --prune-polymesh failed: stdout={} stderr={}",
                String::from_utf8_lossy(&out_prune.stdout),
                String::from_utf8_lossy(&out_prune.stderr)
            );
            assert!(
                !wf_case.join("constant/polyMesh").exists(),
                "EV12-WR-05: constant/polyMesh must be pruned when --prune-polymesh is explicitly passed"
            );
            let arch_ver = verify_wave4_case_identity(&wf_archive)
                .expect("retained archive must remain verified after --prune-polymesh");
            assert_eq!(arch_ver.state, "exact_artifact_bytes_verified");
        }

        let _ = fs::remove_dir_all(&root_tmp);
    }
}
