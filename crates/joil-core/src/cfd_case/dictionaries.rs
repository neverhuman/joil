//! OpenFOAM 14 dictionary writers and EV-12 capture workflow script generator.

use super::{write_dict, CfdCaseConfig};
use crate::geometry::Dimensions;
use crate::phenotype::CfdPatch;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(super) fn write_openfoam_dictionaries(
    case_dir: &Path,
    config: &CfdCaseConfig,
    d: &Dimensions,
    dict_hashes: &mut BTreeMap<String, String>,
) -> Result<(), String> {
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
        dict_hashes,
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
        dict_hashes,
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
        dict_hashes,
    )?;
    write_dict(
        case_dir,
        "constant/momentumTransport",
        "dictionary",
        &format!(
            "simulationType RAS; RAS {{model {}; turbulence on; printCoeffs on;}}",
            config.turbulence_model
        ),
        dict_hashes,
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
    write_dict(case_dir, "0/U", "volVectorField", &u_body, dict_hashes)?;

    let p_body = format!(
        "dimensions [0 2 -2 0 0 0 0]; internalField uniform 0;\n\
boundaryField {{inlet1 {{type zeroGradient;}} inlet2 {{type zeroGradient;}}\n\
 overflow {{type fixedValue; value uniform {:.15e};}}\n\
 underflow {{type fixedValue; value uniform 0;}} wall {{type zeroGradient;}}}}",
        config.reject_pressure_pa / 998.0
    );
    write_dict(case_dir, "0/p", "volScalarField", &p_body, dict_hashes)?;

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
            dict_hashes,
        )?;
    }

    write_dict(
        case_dir,
        "0/nut",
        "volScalarField",
        "dimensions [0 2 -1 0 0 0 0]; internalField uniform 0;\n\
boundaryField {\"(inlet[12]|overflow|underflow)\" {type calculated; value uniform 0;}\n\
 wall {type nutkWallFunction; value uniform 0;}}",
        dict_hashes,
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
        dict_hashes,
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
        dict_hashes,
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
        dict_hashes,
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
        dict_hashes,
    )?;

    write_dict(
        case_dir,
        "system/decomposeParDict",
        "dictionary",
        &format!(
            "numberOfSubdomains {}; method scotch;",
            config.n_subdomains.max(1)
        ),
        dict_hashes,
    )?;

    Ok(())
}

pub(super) fn write_ev12_capture_script(case_dir: &Path) -> Result<(), String> {
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
    Ok(())
}
