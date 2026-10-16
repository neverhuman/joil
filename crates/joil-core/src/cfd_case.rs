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
            "candidate failed P08 analytic structural/LPBF screen: {:?}",
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

    Ok(manifest)
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
}
