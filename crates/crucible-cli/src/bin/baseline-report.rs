#[allow(dead_code)]
#[path = "../ordered_json.rs"]
mod ordered_json;
#[allow(dead_code)]
#[path = "../port_support.rs"]
mod port_support;
use port_support::*;
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
fn read(path: impl AsRef<Path>) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn digest(path: impl AsRef<Path>) -> Result<String> {
    Ok(sha(fs::read(path)?))
}
fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or("expected string".into())
}
fn number(v: &Value) -> Result<f64> {
    v.as_f64().ok_or("expected number".into())
}
fn time_label(value: f64) -> Result<String> {
    let mut buffer = [0 as libc::c_char; 64];
    // SAFETY: fixed format expects one double and the buffer size is exact.
    let n = unsafe { libc::snprintf(buffer.as_mut_ptr(), buffer.len(), c"%.6g".as_ptr(), value) };
    require(n >= 0 && (n as usize) < buffer.len(), "invalid saved time")?;
    Ok(buffer[..n as usize]
        .iter()
        .map(|b| *b as u8 as char)
        .collect())
}
fn point(root: &Path, case: &Path, snapshot: &str) -> Result<Value> {
    let case = resolve(case)?;
    let snap = case.join(snapshot).join("snapshot.json");
    require(case.starts_with(root.join("runs")), "project runs only")?;
    let s = read(&snap)?;
    let obs = &s["observations"];
    let time = time_label(number(&obs["pseudo_iteration"])?)?;
    let req = root.join("requirements/RS-joil-research-v1.json");
    require(
        s["evidence"]["requirements_file_sha256"] == digest(&req)?,
        "requirements changed",
    )?;
    require(
        s["evidence"]["input_manifest_sha256"] == digest(case.join("input-manifest.json"))?,
        "input manifest changed",
    )?;
    for (name, hash) in read(case.join("input-manifest.json"))?
        .as_object()
        .ok_or("invalid input manifest")?
    {
        let path = resolve(&case.join(name))?;
        require(
            path.starts_with(&case) && digest(path)? == string(hash)?,
            "changed/escaped run input",
        )?;
    }
    for (name, item) in s["evidence"]["captured_streams"]
        .as_object()
        .ok_or("invalid captured streams")?
    {
        require(
            digest(
                case.join(snapshot)
                    .join("source-data")
                    .join(format!("{name}.dat")),
            )? == string(&item["sha256"])?,
            "raw snapshot observation changed",
        )?;
    }
    let native_path = case.join(format!("hydraulic-integrals-t{time}.json"));
    let native = read(&native_path)?;
    require(
        number(&obs["pseudo_iteration"])? == number(&native["time"])?,
        "observation/integral saved times differ",
    )?;
    for (name, q) in obs["port_flow_m3_s"]
        .as_object()
        .ok_or("invalid port flows")?
    {
        let q = number(q)?;
        let flow = number(&native["patches"][name]["signed_flow_m3_s"])?;
        require(
            flow.is_finite() && (q - flow).abs() <= 1e-12f64.max(q.abs() * 1e-8),
            "native energy and pressure snapshot use different fluxes",
        )?;
    }
    let energy = number(&native["hydraulic_loss_j_per_treated_m3"])?;
    require(
        energy.is_finite() && energy >= 0.,
        "nonphysical mechanical loss",
    )?;
    require(
        number(&native["rho_kg_m3"])? == 998.
            && native["gravity_m_s2"]
                .as_array()
                .is_some_and(|v| v.len() == 3 && v.iter().all(|x| x.as_f64() == Some(0.))),
        "unsupported energy density/gravity contract",
    )?;
    let metrics = read(case.join(format!("metrics-binary-t{time}.json")))?;
    require(
        digest(root.join(string(&metrics["path"])?))? == string(&metrics["sha256"])?,
        "native metrics binary changed",
    )?;
    let mut receipts = serde_json::Map::new();
    for name in [
        "water-1-mpirun.log.receipt.json".to_string(),
        format!("metrics-t{time}-0-joilHydraulicMetrics.log.receipt.json"),
    ] {
        let path = case.join(&name);
        let receipt = read(&path)?;
        let log = case.join(
            name.strip_suffix(".receipt.json")
                .ok_or("invalid receipt name")?,
        );
        require(
            number(&receipt["exit_code"])? == 0. && digest(log)? == string(&receipt["log_sha256"])?,
            "failed or changed execution receipt",
        )?;
        receipts.insert(path.strip_prefix(root)?.to_string_lossy().into_owned(),json!({"sha256":digest(&path)?,"wall_seconds":receipt["wall_seconds"],"image_id":receipt["image_id"],"exit_code":receipt["exit_code"]}));
    }
    let mut reasons = s["fitness"]["remaining_qualification_reasons"]
        .as_array()
        .ok_or("missing qualification reasons")?
        .iter()
        .map(|v| string(v).map(str::to_owned))
        .collect::<Result<Vec<_>>>()?;
    let mut reverse = serde_json::Map::new();
    for n in ["overflow", "underflow"] {
        reverse.insert(n.into(), native["patches"][n]["reverse_flow_m3_s"].clone());
    }
    if reverse
        .values()
        .any(|v| v.as_f64().is_some_and(|n| n > 1e-11))
    {
        reasons.push("external_outlet_backflow_requires_resolution".into());
    }
    let requirement = read(&req)?;
    let limits = &requirement["limits"]["pdr_draft_range"];
    if obs["pdr"].is_null()
        || !(number(&limits[0])? <= number(&obs["pdr"])?
            && number(&obs["pdr"])? <= number(&limits[1])?)
    {
        reasons.push("outside_draft_pdr_range".into());
    }
    let flow = number(&obs["feed_m3_h"])?;
    let reject = number(&obs["reject_fraction"])?;
    require(
        flow.is_finite() && flow > 0. && reject.is_finite() && (0. ..1.).contains(&reject),
        "invalid finite positive duty or split",
    )?;
    reasons.sort();
    reasons.dedup();
    let fitness = json!({"policy":"fixed-duty-exploratory-screen-v1","minimize_screen_vector":null,"screen_units":["mg/L dispersed oil","J/m3 treated water"],"sampled_quality":"unknown","treated_flow_m3_h":flow*(1.-reject),"qualified_multiobjective_vector":null,"qualified_capacity_gain":null,"campaign_admission":"not_implemented_fail_closed","remaining_qualification_reasons":reasons});
    let extension = if case.join("domain-extension.json").exists() {
        read(case.join("domain-extension.json"))?["length_m"].clone()
    } else {
        json!(0)
    };
    Ok(
        json!({"run_id":case.file_name().ok_or("missing run name")?.to_string_lossy(),"kind":"measured_unqualified_F1_initializer","model":read(case.join("carrier-input.json"))?["model"],"numerical_outlet_extension_m":extension,"observations":obs,"full_domain_hydraulic_loss_w":native["hydraulic_loss_w"],"full_domain_hydraulic_loss_j_per_treated_m3":energy,"external_outlet_reverse_flow_m3_s":reverse,"original_device_overflow":native.get("original_device_overflow"),"oil_capture":null,"cout_mg_l":null,"fitness":fitness,"evidence":{"snapshot":{"path":snap.strip_prefix(root)?.to_string_lossy(),"sha256":digest(&snap)?},"native_integrals":{"path":native_path.strip_prefix(root)?.to_string_lossy(),"sha256":digest(&native_path)?},"native_metrics_binary_sha256":metrics["sha256"],"execution":receipts}}),
    )
}
fn ascii_json(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii() && c != '\u{7f}' {
            out.push(c);
        } else {
            for unit in c.encode_utf16(&mut [0; 2]) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut specs = Vec::new();
    let mut out = None;
    while let Some(arg) = args.next() {
        let v = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--root" => root = v.into(),
            "--point" => specs.push((
                PathBuf::from(v),
                args.next().ok_or("snapshot directory required")?,
            )),
            "--out" => out = Some(PathBuf::from(v)),
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    require(!specs.is_empty(), "--point required")?;
    let out = out.ok_or("--out required")?;
    let root = root.canonicalize()?;
    let points = specs
        .iter()
        .map(|(c, s)| point(&root, c, s))
        .collect::<Result<Vec<_>>>()?;
    let date = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%S.%6N+00:00"])
        .output()?;
    require(date.status.success(), "UTC clock query failed")?;
    let result = json!({"schema_version":1,"kind":"executed_baseline_hydraulic_diagnostics","created_utc":String::from_utf8(date.stdout)?.trim(),"baseline_genome_sha256":digest(root.join("baselines/ct35/genome.json"))?,"requirements_sha256":digest(root.join("requirements/RS-joil-research-v1.json"))?,"extractor_sha256":digest(root.join("analysis/baseline_report.py"))?,"points":points,"qualified_baseline":false,"physical_validation":false,"qualified_capacity_gain":null,"limitations":[
        "Steady pseudo-iterations are not physical sampling times or confidence intervals.",
        "SST is an initializer; transient Reynolds-stress, grid/time/model and oil qualification remain open.",
        "The seed mesh fails the all-geometry concave-cell check and has no wall-layer qualification.",
        "Outlet extension changes the numerical domain, not the fixed device genotype.",
        "Static pressure taps and mechanical energy flux are distinct; pump/system energy is unknown.",
        "Oil-removal, compliance, structural suitability and superiority are unknown."]});
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&out)?;
    writeln!(file, "{}", serde_json::to_string_pretty(&result)?)?;
    println!(
        "{}",
        ascii_json(
            &obj(vec![
                (
                    "path",
                    val(out
                        .components()
                        .filter(|c| !matches!(c, std::path::Component::CurDir))
                        .collect::<PathBuf>()
                        .to_string_lossy()
                        .as_ref())
                ),
                ("points", val(points.len())),
                ("qualified_baseline", val(false))
            ])
            .pretty(0)
        )
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
