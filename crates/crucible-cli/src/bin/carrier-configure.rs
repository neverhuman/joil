//! Foundation 14 dictionaries; F1 pseudo-iteration initializer only.
#[allow(dead_code)]
#[path = "../ordered_json.rs"]
pub(crate) mod ordered_json;
#[allow(dead_code)]
#[path = "../port_support.rs"]
pub(crate) mod port_support;
use port_support::*;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};
pub struct Options {
    pub root: PathBuf,
    pub case: PathBuf,
    pub flow: f64,
    pub pressure: f64,
    pub steps: i64,
    pub model: String,
}
pub fn g(value: f64, digits: i32) -> String {
    let mut b = [0i8; 128]; // SAFETY: buffer and printf arguments match the format.
    let n = unsafe { libc::snprintf(b.as_mut_ptr(), b.len(), c"%.*g".as_ptr(), digits, value) };
    assert!(n >= 0 && (n as usize) < b.len());
    String::from_utf8(b[..n as usize].iter().map(|&c| c as u8).collect()).expect("ASCII number")
}
fn py(v: f64) -> String {
    let s = if v != 0. && (v.abs() < 1e-4 || v.abs() >= 1e16) {
        format!("{v:e}")
    } else {
        format!("{v:?}")
    };
    match s.split_once('e') {
        Some((m, e)) => format!("{m}e{:+03}", e.parse::<i32>().expect("exponent")),
        None => s,
    }
}
fn dictionary(case: &Path, name: &str, text: &str, cls: &str) -> Result<()> {
    let p = case.join(name);
    fs::create_dir_all(p.parent().ok_or("missing dictionary parent")?)?;
    fs::write(
        &p,
        format!(
            "FoamFile {{ format ascii; class {cls}; object {}; }}\n{text}\n",
            p.file_name()
                .ok_or("missing dictionary name")?
                .to_string_lossy()
        ),
    )?;
    Ok(())
}
fn inlet(root: &Path) -> Result<(f64, f64, f64)> {
    let obj: Value = serde_json::from_slice(&fs::read(root.join("baselines/ct35/genome.json"))?)?;
    require(
        obj["schema_version"] == 1 && obj["family"] == "ct35-parametric-v1",
        "unsupported genome",
    )?;
    let p = obj["parameters"].as_object().ok_or("missing parameters")?;
    for (key, v) in p {
        let n = v.as_f64().ok_or("invalid parameter")?;
        require(n.is_finite(), "nonfinite parameter")?;
        require(
            if key == "inlet_z_offset_mm" {
                n >= 0.
            } else {
                n > 0.
            },
            "invalid dimension",
        )?;
    }
    let get = |k: &str| -> Result<f64> {
        p.get(k)
            .and_then(Value::as_f64)
            .ok_or(format!("missing parameter {k}").into())
    };
    let angle = get("cone_half_angle_deg")?;
    require(
        get("n_inlets")? == 2. && angle > 0. && angle < 89.,
        "invalid inlet count or cone angle",
    )?;
    let d2 = get("d2_mm")?;
    let d1 = d2 * get("d1_over_d2")?;
    let d3 = d2 * get("d3_over_d2")?;
    require(d1 > d2 && d2 > d3 && d3 > 0., "invalid diameters")?;
    let r1 = d1 / 2.;
    let area = get("inlet_area_ratio")? * std::f64::consts::PI * r1.powi(2) / 2.;
    let w = (area / get("inlet_h_over_w")?).sqrt();
    let h = w * get("inlet_h_over_w")?;
    require(
        w < r1
            && get("inlet_z_offset_mm")? + h < d1 * get("l1_over_d1")?
            && get("inlet_length_cfd_mm")?.min(get("inlet_length_part_mm")?) > r1
            && get("head_spigot_over_d2")? * d2 < d2 * get("l2_over_d2")?
            && get("plenum_radius_mm")? > d2 * get("do_over_d2")? / 2.,
        "inlet, collar or plenum does not fit",
    )?;
    Ok((area, w, h))
}
pub fn configure(o: &Options) -> Result<()> {
    require(
        resolve(&o.case)?.starts_with(o.root.join("runs"))
            && !o.case.join("input-manifest.json").exists(),
        "configure only a fresh project run; preserve previously bound inputs",
    )?;
    require(
        o.flow.is_finite() && o.flow > 0. && o.pressure.is_finite() && o.steps >= 1,
        "finite positive flow, pressure and iteration budget required",
    )?;
    require(
        ["SSG", "LRR", "kOmegaSST"].contains(&o.model.as_str()),
        "unsupported model identity",
    )?;
    let (area, w, h) = inlet(&o.root)?;
    let qi = o.flow / 3600.;
    let velocity = qi / (2. * area * 1e-6);
    let k = 1.5 * (0.05 * velocity).powi(2);
    let dh = 2. * w * h / (w + h) * 0.001;
    let epsilon = 0.09f64.powf(0.75) * k.powf(1.5) / (0.07 * dh);
    require(
        k.is_finite() && k > 0. && epsilon.is_finite() && (epsilon / (0.09 * k)).is_finite(),
        "nonfinite or underflowed inlet turbulence",
    )?;
    dictionary(
        &o.case,
        "constant/physicalProperties",
        &format!(r#"viscosityModel constant; nu {};"#, g(0.001 / 998., 15)),
        "dictionary",
    )?;
    dictionary(
        &o.case,
        "constant/momentumTransport",
        &format!(
            r#"simulationType RAS; RAS {{model {}; turbulence on; printCoeffs on;}}"#,
            o.model
        ),
        "dictionary",
    )?;
    dictionary(
        &o.case,
        "0/U",
        &format!(
            r#"dimensions [0 1 -1 0 0 0 0]; internalField uniform (0 0 0);
boundaryField {{
 inlet1 {{type flowRateInletVelocity; volumetricFlowRate constant {}; value uniform ({} 0 0);}}
 inlet2 {{type flowRateInletVelocity; volumetricFlowRate constant {}; value uniform ({} 0 0);}}
 "(overflow|underflow)" {{type pressureInletOutletVelocity; value uniform (0 0 0);}}
 wall {{type noSlip;}}
}}"#,
            g(qi / 2., 15),
            py(velocity),
            g(qi / 2., 15),
            py(-velocity)
        ),
        "volVectorField",
    )?;
    dictionary(
        &o.case,
        "0/p",
        &format!(
            r#"dimensions [0 2 -2 0 0 0 0]; internalField uniform 0;
boundaryField {{inlet1 {{type zeroGradient;}} inlet2 {{type zeroGradient;}}
 overflow {{type fixedValue; value uniform {};}}
 underflow {{type fixedValue; value uniform 0;}} wall {{type zeroGradient;}}}}"#,
            g(o.pressure / 998., 15)
        ),
        "volScalarField",
    )?;
    for (name, value, dimensions, walltype) in [
        ("k", k, "0 2 -2", "kqRWallFunction"),
        ("epsilon", epsilon, "0 2 -3", "epsilonWallFunction"),
        ("omega", epsilon / (0.09 * k), "0 0 -1", "omegaWallFunction"),
    ] {
        dictionary(
            &o.case,
            &("0/".to_string() + name),
            &format!(
                r#"dimensions [{} 0 0 0 0]; internalField uniform {};
boundaryField {{"inlet[12]" {{type fixedValue; value uniform {};}}
 "(overflow|underflow)" {{type inletOutlet; inletValue uniform {}; value uniform {};}}
 wall {{type {}; value uniform {};}}}}"#,
                dimensions,
                py(value),
                py(value),
                py(value),
                py(value),
                walltype,
                py(value)
            ),
            "volScalarField",
        )?;
    }
    dictionary(
        &o.case,
        "0/nut",
        r#"dimensions [0 2 -1 0 0 0 0]; internalField uniform 0;
boundaryField {"(inlet[12]|overflow|underflow)" {type calculated; value uniform 0;}
 wall {type nutkWallFunction; value uniform 0;}}"#,
        "volScalarField",
    )?;
    let r = format!(
        r#"({} 0 0 {} 0 {})"#,
        py(2. * k / 3.),
        py(2. * k / 3.),
        py(2. * k / 3.)
    );
    dictionary(
        &o.case,
        "0/R",
        &format!(
            r#"dimensions [0 2 -2 0 0 0 0]; internalField uniform {};
boundaryField {{"inlet[12]" {{type fixedValue; value uniform {};}}
 "(overflow|underflow)" {{type inletOutlet; inletValue uniform {}; value uniform {};}}
 wall {{type kqRWallFunction; value uniform {};}}}}"#,
            r, r, r, r, r
        ),
        "volSymmTensorField",
    )?;
    let mut functions = Vec::new();
    for name in ["inlet1", "inlet2", "overflow", "underflow"] {
        for (suffix, operation, field) in [("flow", "sum", "phi"), ("pressure", "areaAverage", "p")]
        {
            functions.push(format!(
                r#"{}_{} {{type surfaceFieldValue; libs ("libfieldFunctionObjects.so");
writeControl timeStep; writeInterval 1; log false; patch {};
operation {}; fields ({}); writeFields false;}}"#,
                name, suffix, name, operation, field
            ));
        }
    }
    let fields = if o.model == "SSG" || o.model == "LRR" {
        "p U R epsilon"
    } else {
        "p U k omega"
    };
    functions.push(format!(
        r#"residuals {{type residuals; libs ("libutilityFunctionObjects.so");
writeControl timeStep; writeInterval 1; fields ({});}}"#,
        fields
    ));
    dictionary(
        &o.case,
        "system/controlDict",
        &format!(
            r#"application foamRun; solver incompressibleFluid;
startFrom latestTime; startTime 0; stopAt endTime; endTime {}; deltaT {};
writeControl timeStep; writeInterval {}; purgeWrite 2;
writeFormat ascii; writePrecision 12; writeCompression off; timePrecision 10;
runTimeModifiable false;
functions {{ {} }}"#,
            o.steps,
            1,
            (o.steps / 2).max(1),
            functions.join(" ")
        ),
        "dictionary",
    )?;
    dictionary(
        &o.case,
        "system/fvSchemes",
        &format!(
            r#"ddtSchemes {{default {};}}
gradSchemes {{default cellLimited Gauss linear 1;}}
divSchemes {{default none;
 div(phi,U) bounded Gauss upwind;
 div(phi,R) bounded Gauss upwind;
 div(phi,k) bounded Gauss upwind;
 div(phi,epsilon) bounded Gauss upwind;
 div(phi,omega) bounded Gauss upwind;
 div((nuEff*dev2(T(grad(U))))) Gauss linear;
 div(R) Gauss linear;
}}
laplacianSchemes {{default Gauss linear limited .5;}}
interpolationSchemes {{default linear;}} snGradSchemes {{default limited .5;}}
wallDist {{method meshWave;}}"#,
            "steadyState"
        ),
        "dictionary",
    )?;
    dictionary(
        &o.case,
        "system/fvSolution",
        r#"solvers {
 p {solver GAMG; tolerance 1e-8; relTol .05; smoother GaussSeidel;}
 pFinal {$p; relTol 0;}
 pcorr {solver GAMG; tolerance 1e-8; relTol 0; smoother GaussSeidel;}
 "(U|R|k|epsilon|omega)" {solver smoothSolver; smoother symGaussSeidel; tolerance 1e-8; relTol .05;}
 "(U|R|k|epsilon|omega)Final" {solver smoothSolver; smoother symGaussSeidel; tolerance 1e-8; relTol 0;}
}
SIMPLE {nNonOrthogonalCorrectors 1; consistent yes;}
PIMPLE {nOuterCorrectors 1; nCorrectors 1; nNonOrthogonalCorrectors 1; consistent yes;}
relaxationFactors {fields {p .2;} equations {U .4; R .1; k .3; epsilon .3; omega .3;}}
"#,
        "dictionary",
    )?;
    dictionary(
        &o.case,
        "system/decomposeParDict",
        r#"numberOfSubdomains 4; method scotch;"#,
        "dictionary",
    )?;
    // Compatibility provenance binds the unchanged Python configuration oracle.
    // The native implementation identity is recorded separately in the packet receipt.
    let input = json!({"schema_version":1,"kind":"F1_steady_initializer_only","model":o.model,"feed_m3_h":o.flow,"reject_pressure_pa":o.pressure,"water_outlet_pressure_pa":0.,"rho_kg_m3":998.,"mu_pa_s":0.001,"gravity_m_s2":[0,0,0],"iterations":o.steps,"time_is_pseudo_iteration":true,"inlet_turbulent_intensity":0.05,"inlet_length_scale_m":0.07*dh,"genome_file_sha256":sha(fs::read(o.root.join("baselines/ct35/genome.json"))?),"configuration_source_sha256":sha(fs::read(o.root.join("physics/openfoam/carrier/configure.py"))?)});
    fs::write(
        o.case.join("carrier-input.json"),
        serde_json::to_string_pretty(&input)? + "\n",
    )?;
    Ok(())
}
fn run() -> Result<()> {
    let mut o = Options {
        root: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()?,
        case: PathBuf::new(),
        flow: 4.,
        pressure: -200.,
        steps: 500,
        model: "SSG".into(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let v = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--root" => o.root = PathBuf::from(v).canonicalize()?,
            "--case" => o.case = v.into(),
            "--flow" => o.flow = v.parse()?,
            "--reject-pressure-pa" => o.pressure = v.parse()?,
            "--steps" => o.steps = v.parse()?,
            "--model" => o.model = v,
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    require(!o.case.as_os_str().is_empty(), "--case required")?;
    configure(&o)
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("FAIL: {e}");
            ExitCode::FAILURE
        }
    }
}
