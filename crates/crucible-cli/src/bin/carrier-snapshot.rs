//! Immutable same-iteration carrier observations, always unqualified.
#[allow(dead_code)]
#[path = "../ordered_json.rs"]
mod ordered_json;
#[allow(dead_code)]
#[path = "../port_support.rs"]
mod port_support;
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
fn rows(data: &[u8]) -> Result<Vec<(f64, f64)>> {
    let mut rows = vec![];
    for line in std::str::from_utf8(data)?.split_inclusive('\n') {
        if !line.ends_with('\n') {
            continue;
        }
        let text = line.trim();
        if text.is_empty() || text.starts_with('#') {
            continue;
        }
        let v = text
            .split_whitespace()
            .map(str::parse::<f64>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        require(
            v.len() == 2 && v.iter().all(|x| x.is_finite()),
            "expected finite scalar time/value rows",
        )?;
        require(
            !rows.iter().any(|(t, _)| *t == v[0]),
            "duplicate observation time",
        )?;
        rows.push((v[0], v[1]));
    }
    require(!rows.is_empty(), "empty scalar observation stream")?;
    Ok(rows)
}
fn pressure_scale(text: &str, density: f64) -> Result<f64> {
    require(
        density.is_finite() && density > 0.,
        "positive density required",
    )?;
    let rest = text
        .split_once("dimensions")
        .ok_or("explicit pressure dimensions required")?
        .1
        .trim_start();
    let rest = rest
        .strip_prefix('[')
        .ok_or("explicit pressure dimensions required")?;
    let (dims, tail) = rest
        .split_once(']')
        .ok_or("explicit pressure dimensions required")?;
    require(
        tail.trim_start().starts_with(';'),
        "explicit pressure dimensions required",
    )?;
    let dims = dims
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if dims == [0., 2., -2., 0., 0., 0., 0.] {
        Ok(density)
    } else if dims == [1., -1., -2., 0., 0., 0., 0.] {
        Ok(1.)
    } else {
        Err("unrecognized pressure dimensions".into())
    }
}
fn run() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let (mut case, mut out, mut created) = (None, None, None);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let v = args.next().ok_or("missing option value")?;
        match a.as_str() {
            "--case" => case = Some(PathBuf::from(v)),
            "--out" => out = Some(PathBuf::from(v)),
            "--created-utc" => created = Some(v),
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    let case = resolve(&case.ok_or("--case required")?)?;
    let out = resolve(&out.ok_or("--out required")?)?;
    require(
        case.starts_with(root.join("runs")) && out.starts_with(root.join("runs")),
        "case and raw snapshot must stay in project runs",
    )?;
    fs::create_dir_all(out.parent().ok_or("missing parent")?)?;
    fs::create_dir(&out)?;
    let raw = out.join("source-data");
    fs::create_dir(&raw)?;
    let manifest = read(case.join("input-manifest.json"))?;
    for (name, expected) in manifest.as_object().ok_or("invalid input manifest")? {
        let p = resolve(&case.join(name))?;
        require(
            p.starts_with(&case)
                && sha(fs::read(&p)?) == expected.as_str().ok_or("invalid digest")?,
            &format!("run input changed or escaped its case: {name}"),
        )?;
    }
    let genome = sha(fs::read(root.join("baselines/ct35/genome.json"))?);
    require(
        read(case.join("mesh-input.json"))?["genome_file_sha256"] == genome,
        "case is not bound to the frozen CT35 baseline",
    )?;
    let ports = ["inlet1", "inlet2", "overflow", "underflow"];
    let mut streams = vec![];
    let mut files = Map::new();
    for n in ports {
        for suffix in ["flow", "pressure"] {
            let name = format!("{n}_{suffix}");
            let dir = case.join("postProcessing").join(&name).join("0");
            let paths = if dir.exists() {
                fs::read_dir(dir)?
                    .map(|e| e.map(|e| e.path()))
                    .collect::<std::io::Result<Vec<_>>>()?
                    .into_iter()
                    .filter(|p| p.extension().is_some_and(|e| e == "dat"))
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };
            require(
                paths.len() == 1,
                &format!("expected one uninterrupted observation stream for {name}"),
            )?;
            let data = fs::read(&paths[0])?;
            fs::write(raw.join(format!("{name}.dat")), &data)?;
            streams.push(rows(&data)?);
            files.insert(name, json!({"sha256":sha(&data),"bytes":data.len()}));
        }
    }
    let pressure = fs::read(case.join("0/p"))?;
    fs::write(raw.join("pressure-field-input"), &pressure)?;
    let reqpath = root.join("requirements/RS-joil-research-v1.json");
    let req = read(&reqpath)?;
    let scale = pressure_scale(
        std::str::from_utf8(&pressure)?,
        req["reference"]["water_density_kg_m3"]
            .as_f64()
            .ok_or("missing density")?,
    )?;
    let t = streams[0]
        .iter()
        .map(|(t, _)| *t)
        .filter(|t| streams.iter().all(|s| s.iter().any(|(u, _)| t == u)))
        .max_by(f64::total_cmp)
        .ok_or("no same-iteration observations across all ports")?;
    let at = |i: usize| {
        streams[i]
            .iter()
            .find(|(u, _)| *u == t)
            .expect("common timestamp")
            .1
    };
    let q = [at(0), at(2), at(4), at(6)];
    let p = [at(1) * scale, at(3) * scale, at(5) * scale, at(7) * scale];
    let qi = -q[0] - q[1];
    require(qi > 0., "no positive inlet total flow")?;
    let pin = (-q[0] * p[0] - q[1] * p[1]) / qi;
    let dp = pin - p[3];
    let reject = q[2] / qi;
    let orientation = q[3] > 0. && q[2] >= 0. && q[2] < qi;
    let closure = q.iter().sum::<f64>().abs() / qi;
    require(
        q.iter()
            .chain(p.iter())
            .copied()
            .chain([qi, pin, dp, reject, closure, qi * 3600., q[3] * 3600.])
            .all(f64::is_finite)
            && (dp <= 0. || ((pin - p[2]) / dp).is_finite()),
        "nonfinite derived observation",
    )?;
    let mut reasons = vec![
        "steady_initializer_not_transient_statistics",
        "spatial_and_temporal_error_unresolved",
        "oil_transport_not_included",
        "pressure_energy_and_absolute_pressure_limits_unresolved",
        "no_independent_physical_validation",
    ];
    if case.join("mesh-audit.json").exists() {
        if !read(case.join("mesh-audit.json"))?["pilot_mesh_pass"]
            .as_bool()
            .ok_or("missing mesh pass")?
        {
            reasons.push("mesh_qualification_failed");
        }
    } else {
        reasons.push("mesh_qualification_missing");
    }
    if closure > 0.001 {
        reasons.push("water_closure_exceeds_seed_0.1pct");
    }
    if (reject - 0.02).abs() > 0.0002 {
        reasons.push("target_split_not_met");
    }
    if !orientation {
        reasons.push("reverse_outlet_net_flow");
    }
    let mut fitness = json!({"policy":"fixed-duty-exploratory-screen-v1","minimize_screen_vector":null,"qualified_multiobjective_vector":null,"qualified_capacity_gain":null,"sampled_quality":"unknown","campaign_admission":"blocked_reverse_flow_and_unqualified_physics","remaining_qualification_reasons":reasons});
    if orientation {
        require(
            (qi * 3600.).is_finite() && reject.is_finite(),
            "invalid finite positive duty or split",
        )?;
        reasons.sort();
        reasons.dedup();
        fitness["remaining_qualification_reasons"] = json!(reasons);
        fitness["screen_units"] = json!(["mg/L dispersed oil", "J/m3 treated water"]);
        fitness["treated_flow_m3_h"] = json!(qi * 3600. * (1. - reject));
        fitness["campaign_admission"] = json!("not_implemented_fail_closed");
    }
    let model = if case.join("carrier-input.json").exists() {
        read(case.join("carrier-input.json"))?["model"]
            .as_str()
            .ok_or("missing model")?
            .to_owned()
    } else {
        "SSG".into()
    };
    let created = created.unwrap_or_else(|| {
        let ms = crucible_runtime::utc_millis().unwrap_or(0); // UTC formatting uses the system C library without spawning a process.
        let secs = (ms / 1000) as libc::time_t;
        let mut tm = std::mem::MaybeUninit::<libc::tm>::uninit();
        // SAFETY: valid input and writable tm storage; gmtime_r initializes tm on success.
        let ok = unsafe { libc::gmtime_r(&secs, tm.as_mut_ptr()) };
        if ok.is_null() {
            return "unknown".into();
        }
        // SAFETY: checked successful initialization above.
        let tm = unsafe { tm.assume_init() };
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}+00:00",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec,
            ms % 1000
        )
    });
    let flows: Map<String, Value> = ports
        .iter()
        .zip(q)
        .map(|(n, v)| (n.to_string(), json!(v)))
        .collect();
    let pressures: Map<String, Value> = ports
        .iter()
        .zip(p)
        .map(|(n, v)| (n.to_string(), json!(v)))
        .collect();
    let result = json!({"schema_version":1,"kind":"executed_unqualified_carrier_snapshot","run_id":case.file_name().ok_or("missing run name")?.to_string_lossy(),"created_utc":created,"fidelity":format!("F1 steady {model} initializer diagnostic"),"observations":{"pseudo_iteration":t,"physical_elapsed_time_s":null,"feed_m3_h":qi*3600.,"treated_m3_h":q[3]*3600.,"reject_fraction":reject,"flow_orientation_valid":orientation,"water_closure_fraction":closure,"static_tap_drop_pa":dp,"pdr":if dp>0.{Some((pin-p[2])/dp)}else{None},"port_flow_m3_s":flows,"port_area_average_static_pressure_pa":pressures},"oil_capture":null,"cout_mg_l":null,"hydraulic_loss_j_per_treated_m3":null,"fitness":fitness,"evidence":{"captured_streams":files,"pressure_input_sha256":sha(&pressure),"requirements_file_sha256":sha(fs::read(reqpath)?),"genome_file_sha256":genome,"extractor_sha256":sha(fs::read(root.join("analysis/carrier_snapshot.py"))?),"input_manifest_sha256":sha(fs::read(case.join("input-manifest.json"))?)},"uncertainty":{"iteration_samples_are_physical_time_samples":false,"sampling":null,"spatial":null,"model_form":null,"confidence_interval":null},"limitations":["Incomplete running initializer; no accepted baseline or performance ranking.","Pressure values are static patch taps, not mechanical-energy loss.","Raw observation bytes archived with this immutable snapshot."]});
    let bytes = serde_json::to_string_pretty(&result)?;
    fs::write(out.join("snapshot.json"), format!("{bytes}\n"))?;
    println!("{bytes}");
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
