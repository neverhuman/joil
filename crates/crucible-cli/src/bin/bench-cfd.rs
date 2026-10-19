#[allow(dead_code)]
#[path = "../ordered_json.rs"]
mod ordered_json;
#[allow(dead_code)]
#[path = "../port_support.rs"]
mod port_support;
use port_support::*;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};
#[path = "../foam_runtime.rs"]
mod foam_runtime;
use foam_runtime::{probe, text, write};
// Python's default :g is six significant digits, including scientific exponents.
fn time_label(value: f64) -> Result<String> {
    let format = b"%.6g\0";
    let mut buffer: [libc::c_char; 64] = [0; 64];
    // SAFETY: fixed format expects one double; buffer capacity is supplied exactly.
    let n = unsafe {
        libc::snprintf(
            buffer.as_mut_ptr(),
            buffer.len(),
            format.as_ptr().cast(),
            value,
        )
    };
    require(n >= 0 && (n as usize) < buffer.len(), "invalid saved time")?;
    Ok(buffer[..n as usize]
        .iter()
        .map(|b| char::from(b.to_ne_bytes()[0]))
        .collect())
}
fn rewrite_boundary(case: &Path) -> Result<()> {
    let path = case.join("constant/polyMesh/boundary");
    let before = fs::read(&path)?;
    let source = std::str::from_utf8(&before)?;
    let mut matches = Vec::new();
    for (start, _) in source.match_indices("wall") {
        if start > 0
            && source[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let mut rest = &source[start + 4..];
        rest = match rest.trim_start().strip_prefix('{') {
            Some(v) => v,
            None => continue,
        };
        rest = match rest.trim_start().strip_prefix("type") {
            Some(v) => v,
            None => continue,
        };
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        rest = rest.trim_start();
        if let Some(tail) = rest.strip_prefix("patch") {
            if tail.trim_start().starts_with(';') {
                matches.push(source.len() - rest.len());
            }
        }
    }
    require(
        matches.len() == 1,
        "expected exactly one generic wall group from gmshToFoam",
    )?;
    let mut after = source.to_string();
    after.replace_range(matches[0]..matches[0] + 5, "wall");
    fs::write(&path, &after)?;
    write(
        case.join("boundary-type-receipt.json"),
        &obj(vec![
            (
                "operation",
                val("set wall physical group to OpenFOAM wall patch type"),
            ),
            ("before_sha256", val(sha(before))),
            ("after_sha256", val(sha(after))),
        ]),
    )
}
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let stage = args.next().ok_or("stage required")?;
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut case = None;
    let mut binary = None;
    let mut time = None;
    while let Some(arg) = args.next() {
        let v = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--root" => root = v.into(),
            "--case" => case = Some(PathBuf::from(v)),
            "--metrics-bin" => binary = Some(PathBuf::from(v)),
            "--time" => time = Some(v.parse::<f64>()?),
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    let root = root.canonicalize()?;
    let case = resolve(&case.ok_or("--case required")?)?;
    let label = match time {
        Some(t) => format!("-t{}", time_label(t)?),
        None => String::new(),
    };
    let mut stages: Vec<Vec<String>> = match stage.as_str() {
        "mesh" => vec![vec!["blockMesh"],vec!["snappyHexMesh","-overwrite"],vec!["checkMesh","-allTopology","-allGeometry"]],
        "tet" => vec![vec!["gmshToFoam","ct35.msh"]],
        "water" => vec![vec!["decomposePar","-force"],vec!["mpirun","--bind-to","none","-np","4","foamRun","-parallel"]],
        "reconstruct" => vec![vec!["reconstructPar","-latestTime"]],
        "centres" => vec![vec!["foamPostProcess","-func","writeCellCentres","-latestTime"]],
        "build-metrics" => vec![vec!["bash","-c","cp -r /work/physics/openfoam/carrier/hydraulicMetrics build && export FOAM_USER_APPBIN=/case/bin && mkdir -p \"$FOAM_USER_APPBIN\" && cd build && wmake"]],
        "metrics" => vec![], _ => return Err("invalid stage".into()),
    }.into_iter().map(|a| a.into_iter().map(str::to_owned).collect()).collect();
    if stage == "metrics" {
        let binary =
            resolve(&binary.ok_or("--metrics-bin must identify the compiled audited utility")?)?;
        require(
            binary.is_file() && binary.starts_with(root.join("runs")),
            "metrics binary must be a project run artifact",
        )?;
        let relative = binary.strip_prefix(&root)?.to_string_lossy().into_owned();
        stages.push(vec![format!("/work/{relative}"), "-latestTime".into()]);
        require(
            !case.join("hydraulic-integrals.json").exists(),
            "preserve existing native integral output before another extraction",
        )?;
        let metadata = case.join(format!("metrics-binary{label}.json"));
        require(
            !metadata.exists(),
            "metrics extraction already recorded for this saved time",
        )?;
        write(
            metadata,
            &obj(vec![
                ("path", val(relative)),
                ("sha256", val(sha(fs::read(binary)?))),
            ]),
        )?;
    }
    for (i, argv) in stages.iter_mut().enumerate() {
        if let Some(t) = time {
            if matches!(stage.as_str(), "reconstruct" | "centres" | "metrics") {
                argv.pop();
                argv.extend(["-time".into(), time_label(t)?]);
            }
        }
        if stage == "metrics" && case.join("device-plane.json").exists() {
            let plane: Value = serde_json::from_slice(&fs::read(case.join("device-plane.json"))?)?;
            require(
                sha(fs::read(
                    case.join("constant/polyMesh/sets/deviceOverflow"),
                )?) == text(&plane, "face_set_sha256")?,
                "original device measurement plane changed",
            )?;
            argv.push("-devicePlane".into());
        }
        let name = format!(
            "{stage}{label}-{i}-{}",
            argv[0].rsplit('/').next().ok_or("empty command")?
        );
        require(
            !case.join(format!("{name}.log")).exists(),
            "stage log exists; use a new named attempt/case",
        )?;
        let lock: Value = serde_json::from_slice(&fs::read(
            root.join("physics/openfoam/environment/lock.json"),
        )?)?;
        probe(
            &root,
            &case,
            &lock,
            &name,
            &argv.iter().map(String::as_str).collect::<Vec<_>>(),
            3600,
        )?;
    }
    if stage == "metrics" {
        if let Some(t) = time {
            let target = case.join(format!("hydraulic-integrals{label}.json"));
            require(!target.exists(), "integral receipt already exists")?;
            let native = case.join("hydraulic-integrals.json");
            let value: Value = serde_json::from_slice(&fs::read(&native)?)?;
            require(
                value["time"].as_f64() == Some(t),
                "wrong saved time extracted",
            )?;
            fs::rename(native, target)?;
        }
    }
    if stage == "tet" {
        rewrite_boundary(&case)?;
        let lock: Value = serde_json::from_slice(&fs::read(
            root.join("physics/openfoam/environment/lock.json"),
        )?)?;
        probe(
            &root,
            &case,
            &lock,
            "mesh-2-checkMesh",
            &["checkMesh", "-allTopology", "-allGeometry"],
            300,
        )?;
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
