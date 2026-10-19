//! Content-bound F1 operating point with identical-mesh warm-start checks.
#[allow(dead_code)]
#[path = "carrier-configure.rs"]
mod configure;
use configure::{g, Options};
use configure::{ordered_json, port_support};
use port_support::*;
use serde_json::{json, Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};
fn read(p: impl AsRef<Path>) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(p)?)?)
}
fn save(p: impl AsRef<Path>, v: &Value) -> Result<()> {
    fs::write(p, serde_json::to_string_pretty(v)? + "\n")?;
    Ok(())
}
fn copytree(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.path().is_dir() {
            copytree(&e.path(), &to)?
        } else {
            fs::copy(e.path(), to)?;
        }
    }
    Ok(())
}
// Match the Python oracle's single flat overflow block and numeric value token.
fn pressure(text: &str, value: f64) -> Result<String> {
    let mut spans = Vec::new();
    let mut offset = 0;
    while let Some(i) = text[offset..].find("overflow") {
        let start = offset + i;
        offset = start + 8;
        if start > 0
            && text[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let after = text[offset..].trim_start();
        if !after.starts_with('{') {
            continue;
        }
        let body_start = text.len() - after.len() + 1;
        let end = text[body_start..]
            .find('}')
            .ok_or("reject pressure boundary not uniquely identified")?
            + body_start;
        let body = &text[body_start..end];
        if body.contains('{') {
            continue;
        }
        let mut at = 0;
        while let Some(j) = body[at..].find("value") {
            let j = j + at;
            at = j + 5;
            if j > 0
                && body[..j]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_')
            {
                continue;
            }
            let rest = &body[at..];
            if !rest.starts_with(char::is_whitespace) {
                continue;
            }
            let rest = rest.trim_start();
            let Some(rest) = rest.strip_prefix("uniform") else {
                continue;
            };
            if !rest.starts_with(char::is_whitespace) {
                continue;
            }
            let rest = rest.trim_start();
            let n = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || "-+.eE".contains(*c))
                .count();
            if n == 0 || !rest[n..].trim_start().starts_with(';') {
                continue;
            }
            let a = body_start + body.len() - rest.len();
            spans.push((a, a + n));
            break;
        }
    }
    require(
        spans.len() == 1,
        "reject pressure boundary not uniquely identified",
    )?;
    let (a, b) = spans[0];
    Ok(format!(
        "{}{}{}",
        &text[..a],
        g(value / 998., 15),
        &text[b..]
    ))
}
fn run() -> Result<()> {
    let mut o = Options {
        root: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()?,
        case: PathBuf::new(),
        flow: 4.,
        pressure: 0.,
        steps: 300,
        model: "kOmegaSST".into(),
    };
    let mut mesh = None;
    let mut diagnostic = false;
    let mut warm = None;
    let mut time = None;
    let mut pressure_given = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--diagnostic" {
            diagnostic = true;
            continue;
        }
        let v = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--root" => o.root = PathBuf::from(v).canonicalize()?,
            "--case" => o.case = v.into(),
            "--mesh-case" => mesh = Some(PathBuf::from(v)),
            "--flow" => o.flow = v.parse()?,
            "--reject-pressure-pa" => {
                o.pressure = v.parse()?;
                pressure_given = true
            }
            "--iterations" => o.steps = v.parse()?,
            "--model" => o.model = v,
            "--warm-case" => warm = Some(PathBuf::from(v)),
            "--warm-time" => time = Some(v.parse::<f64>()?),
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    require(
        !o.case.as_os_str().is_empty() && pressure_given,
        "--case and --reject-pressure-pa required",
    )?;
    o.case = resolve(&o.case)?;
    let mesh = resolve(&mesh.ok_or("--mesh-case required")?)?;
    require(
        o.case.starts_with(o.root.join("runs")) && mesh.starts_with(o.root.join("runs")),
        "project runs only",
    )?;
    require(!o.case.exists(), "new case path required")?;
    let audit = read(mesh.join("mesh-audit.json"))?;
    let pass = audit["pilot_mesh_pass"]
        .as_bool()
        .ok_or("missing mesh pass")?;
    require(
        pass || diagnostic,
        "mesh qualification failed; only explicit unqualified diagnostics allowed",
    )?;
    fs::create_dir_all(&o.case)?;
    copytree(
        &mesh.join("constant/polyMesh"),
        &o.case.join("constant/polyMesh"),
    )?;
    for name in [
        "mesh-input.json",
        "mesh-audit.json",
        "domain-extension.json",
    ] {
        if name != "domain-extension.json" || mesh.join(name).exists() {
            fs::copy(mesh.join(name), o.case.join(name))?;
        }
    }
    configure::configure(&o)?;
    if let Some(warm) = warm {
        let warm = resolve(&warm)?;
        require(
            warm.starts_with(o.root.join("runs")) && time.is_some(),
            "explicit project warm-start case and saved time required",
        )?;
        require(
            !warm.join("numerical-failure.json").exists(),
            "cannot seed from a failed numerical run",
        )?;
        require(
            read(warm.join("carrier-input.json"))?["model"] == o.model,
            "cross-model warm start requires a separate field conversion contract",
        )?;
        require(
            read(warm.join("water-1-mpirun.log.receipt.json"))?["exit_code"] == 0,
            "source run did not complete successfully",
        )?;
        for name in ["points", "faces", "owner", "neighbour", "boundary"] {
            require(
                sha(fs::read(warm.join("constant/polyMesh").join(name))?)
                    == sha(fs::read(o.case.join("constant/polyMesh").join(name))?),
                "cannot map fields across a changed mesh without qualified conservative mapping",
            )?;
        }
        let time = time.ok_or("missing warm time")?;
        let src = warm.join(g(time, 6));
        let mut transferred = Map::new();
        for name in ["U", "p", "phi", "k", "omega", "epsilon", "R", "nut"] {
            let f = src.join(name);
            if f.exists() {
                transferred.insert(name.into(), json!(sha(fs::read(&f)?)));
                fs::copy(f, o.case.join("0").join(name))?;
            }
        }
        require(
            ["U", "p", "phi", "k", "nut"]
                .iter()
                .all(|n| transferred.contains_key(*n)),
            "incomplete saved carrier fields",
        )?;
        let p = o.case.join("0/p");
        fs::write(&p, pressure(&fs::read_to_string(&p)?, o.pressure)?)?;
        save(
            o.case.join("warm-start.json"),
            &json!({"source_case":warm.file_name().ok_or("missing source name")?.to_string_lossy(),"source_saved_pseudo_time":time,"source_field_sha256":transferred,"method":"identical mesh direct copy, reject boundary pressure revised","physical_time_continuation":false}),
        )?;
    }
    save(
        o.case.join("research-status.json"),
        &json!({"purpose":if diagnostic{"unqualified F1 operating-point diagnostic"}else{"pilot carrier point"},"fidelity":"F1 steady initializer","mesh_pilot_pass":pass,"physical_validation":false,"campaign_admission":false,"qualified_fitness":null}),
    )?;
    let mut manifest = Map::new();
    for p in files(&o.case)? {
        manifest.insert(
            p.strip_prefix(&o.case)?.to_string_lossy().into_owned(),
            json!(sha(fs::read(&p)?)),
        );
    }
    save(o.case.join("input-manifest.json"), &Value::Object(manifest))?;
    println!("{}", o.case.display());
    Ok(())
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
