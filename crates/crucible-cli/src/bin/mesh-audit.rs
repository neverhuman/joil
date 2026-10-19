//! Independent ASCII mesh audit; geometric pilot evidence only.
#[allow(dead_code)]
#[path = "../foam_mesh.rs"]
mod foam_mesh;
#[allow(dead_code)]
#[path = "../ordered_json.rs"]
mod ordered_json;
#[allow(dead_code)]
#[path = "../port_support.rs"]
mod port_support;
use port_support::*;
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::Path,
    process::ExitCode,
};
fn number(v: &Value) -> Result<f64> {
    v.as_f64().ok_or("missing numeric value".into())
}
fn norm(v: [f64; 3]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}
fn section(points: &[[f64; 3]], faces: &[Vec<usize>]) -> Result<Value> {
    let z = -0.0014;
    let mut edges: BTreeMap<[i64; 2], BTreeSet<[i64; 2]>> = BTreeMap::new();
    let mut coords = BTreeMap::new();
    for face in faces {
        let poly = face
            .iter()
            .map(|&i| points.get(i).copied().ok_or("face vertex out of bounds"))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let lo = poly.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min);
        let hi = poly.iter().map(|p| p[2]).fold(f64::NEG_INFINITY, f64::max);
        if lo > z || hi < z {
            continue;
        }
        let mut hits = BTreeMap::new();
        for (a, b) in poly
            .iter()
            .zip(poly.iter().cycle().skip(1))
            .take(poly.len())
        {
            if (a[2] - z) * (b[2] - z) > 0. || (a[2] - b[2]).abs() < 1e-15 {
                continue;
            }
            let t = (z - a[2]) / (b[2] - a[2]);
            if !(-1e-12..=1. + 1e-12).contains(&t) {
                continue;
            }
            let xy = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
            require(xy.iter().all(|x| x.is_finite()), "nonfinite section")?;
            hits.insert(xy.map(|x| (x * 1e11).round_ties_even() as i64), xy);
        }
        if hits.is_empty() || (hits.len() == 1 && (lo >= z || hi <= z)) {
            continue;
        }
        require(hits.len() == 2, "ambiguous or nonplanar wall-section face")?;
        let keys: Vec<_> = hits.keys().copied().collect();
        edges.entry(keys[0]).or_default().insert(keys[1]);
        edges.entry(keys[1]).or_default().insert(keys[0]);
        coords.extend(hits);
    }
    require(
        edges.len() >= 3 && edges.values().all(|v| v.len() == 2),
        "bore section is not a closed unbranched loop",
    )?;
    let start = *edges.keys().next().ok_or("missing section")?;
    let mut ring = vec![start];
    let mut previous = None;
    let mut current = start;
    loop {
        let next = *edges[&current]
            .iter()
            .find(|&&p| Some(p) != previous)
            .ok_or("broken section")?;
        if next == start {
            break;
        }
        require(
            !ring.contains(&next),
            "section self-reconnects before closing",
        )?;
        ring.push(next);
        previous = Some(current);
        current = next;
    }
    require(
        ring.len() == edges.len(),
        "multiple wall-section components",
    )?;
    let mut area = 0.;
    let mut perimeter = 0.;
    for (a, b) in ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
    {
        let a = coords[a];
        let b = coords[b];
        area += a[0] * b[1] - a[1] * b[0];
        perimeter += ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    }
    area = area.abs() * 0.5;
    Ok(
        json!({"plane_z_m":z,"vertices":ring.len(),"area_m2":area,"perimeter_m":perimeter,"hydraulic_diameter_m":4.*area/perimeter,"area_equivalent_diameter_m":2.*(area/std::f64::consts::PI).sqrt(),"relative_analytic_area_error":area/(std::f64::consts::PI*0.0007f64.powi(2))-1.}),
    )
}
fn log_number(log: &str, marker: &str) -> Option<f64> {
    log.split_once(marker)?
        .1
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}
fn audit(case: &Path, name: &str) -> Result<bool> {
    let root = case.join("constant/polyMesh");
    let points = foam_mesh::points(&root)?;
    let pt = foam_mesh::tokens(&root.join("points"))?;
    require(
        pt.first().ok_or("missing point count")?.parse::<usize>()? == points.len(),
        "mesh point count mismatch",
    )?;
    require(
        points.iter().flatten().all(|v| v.is_finite()),
        "nonfinite mesh coordinates",
    )?;
    let faces = foam_mesh::faces(&root)?;
    let expected: Value = serde_json::from_slice(&fs::read(case.join("mesh-input.json"))?)?;
    let required = expected["patches"].as_object().ok_or("missing patches")?;
    let boundary = foam_mesh::tokens(&root.join("boundary"))?;
    let mut patches = Map::new();
    let mut wall = Vec::new();
    for i in 1..boundary.len() {
        if boundary[i] != "{" {
            continue;
        }
        let name = &boundary[i - 1];
        let end = boundary[i..]
            .iter()
            .position(|s| s == "}")
            .ok_or("missing boundary end")?
            + i;
        let field = |key: &str| -> Result<usize> {
            let k = boundary[i..end]
                .iter()
                .position(|s| s == key)
                .ok_or("missing boundary field")?
                + i;
            Ok(boundary[k + 1].parse()?)
        };
        let count = field("nFaces")?;
        let start = field("startFace")?;
        if count == 0 {
            continue;
        }
        let subset = faces
            .get(start..start.checked_add(count).ok_or("face count overflow")?)
            .ok_or("patch faces out of bounds")?;
        if name == "wall" {
            wall = subset.to_vec()
        }
        let mut net = [0.; 3];
        let mut area = 0.;
        let mut allpts = Vec::new();
        for f in subset {
            let poly = f
                .iter()
                .map(|&j| points.get(j).copied().ok_or("face vertex out of bounds"))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            allpts.extend(poly.iter().copied());
            let mut v = [0.; 3];
            for (a, b) in poly
                .iter()
                .zip(poly.iter().cycle().skip(1))
                .take(poly.len())
            {
                for k in 0..3 {
                    v[k] +=
                        (a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3]) * 0.5;
                }
            }
            area += norm(v);
            for k in 0..3 {
                net[k] += v[k]
            }
        }
        let mut rec = json!({"faces":count,"area_m2":area,"net_area_vector_m2":net});
        if let Some(e) = required.get(name) {
            require(area > 0., "zero terminal area")?;
            rec["relative_surface_area_error"] = json!(area / number(&e["area_m2"])? - 1.);
            let normal = e["normal"].as_array().ok_or("missing normal")?;
            require(normal.len() == 3, "invalid normal")?;
            rec["normal_alignment"] = json!(
                net.iter()
                    .zip(normal)
                    .map(|(a, b)| Ok(a * number(b)?))
                    .collect::<Result<Vec<f64>>>()?
                    .iter()
                    .sum::<f64>()
                    / area
            );
            if e.get("plane_axis").is_some() && e.get("plane_position_m").is_some() {
                let axis = e["plane_axis"].as_u64().ok_or("invalid plane axis")? as usize;
                require(axis < 3, "invalid plane axis")?;
                let pos = number(&e["plane_position_m"])?;
                let err = allpts
                    .iter()
                    .map(|p| (p[axis] - pos).abs())
                    .fold(0., f64::max);
                rec["max_terminal_plane_error_m"] = json!(err);
                rec["terminal_position_pass"] =
                    json!(err <= number(&e["plane_tolerance_m"])?.max(1e-6));
            }
        }
        patches.insert(name.clone(), rec);
    }
    let log = fs::read_to_string(case.join("mesh-2-checkMesh.log"))?
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n");
    let cells = log_number(&log, "cells:").ok_or("missing cell count")? as u64;
    let volume = log_number(&log, "Total volume =").ok_or("missing total volume")?;
    let relative = volume / number(&expected["analytic_volume_m3"])? - 1.;
    let bore = section(&points, &wall)?;
    let names: BTreeSet<_> = patches.keys().map(String::as_str).collect();
    let mut want: BTreeSet<_> = required.keys().map(String::as_str).collect();
    want.insert("wall");
    let mut checks = json!({"checkMesh_OK":log.contains("Mesh OK."),"terminal_names_present":names==want,"no_background_boundary":!patches.contains_key("background"),"volume_relative_error_lt_2pct":relative.abs()<0.02,"terminal_area_relative_error_lt_2pct":required.keys().filter_map(|k|patches.get(k)).all(|p|p["relative_surface_area_error"].as_f64().unwrap_or(1.).abs()<0.02),"terminal_normals":required.keys().filter_map(|k|patches.get(k)).all(|p|p["normal_alignment"].as_f64().unwrap_or(0.)>0.999),"terminal_positions":required.keys().all(|k|patches.get(k).is_some_and(|p|p["terminal_position_pass"]==true)),"bore_area_error_lt_2pct":number(&bore["relative_analytic_area_error"])?.abs()<0.02,"cell_cap":cells<=1200000});
    let mut metrics = Map::new();
    for (key, marker, limit, min) in [
        (
            "max_nonorthogonality_deg",
            "Mesh non-orthogonality Max:",
            65.,
            false,
        ),
        ("max_skewness", "Max skewness =", 4., false),
        ("min_determinant", "Cell determinant", 0.001, true),
        ("min_face_weight", "Face interpolation weight", 0.05, true),
        ("min_volume_ratio", "Face volume ratio", 0.01, true),
    ] {
        let value = if min {
            log.split_once(marker)
                .and_then(|(_, s)| s.lines().next())
                .and_then(|s| log_number(s, "minimum:"))
        } else {
            log_number(&log, marker)
        };
        checks[key] = json!(value.is_some_and(|v| if min { v >= limit } else { v <= limit }));
        metrics.insert(key.into(), json!(value));
    }
    let pass = checks
        .as_object()
        .ok_or("invalid checks")?
        .values()
        .all(|v| v == true);
    let result = json!({"kind":"exploratory_full_liner_mesh_audit","cells":cells,"volume_m3":volume,"relative_volume_error":relative,"patches":patches,"bore_section":bore,"quality_metrics":metrics,"checks":checks,"pilot_mesh_pass":pass,"qualification":"geometric pilot only; no mesh convergence or wall-treatment qualification"});
    require(
        Path::new(name).file_name().and_then(|s| s.to_str()) == Some(name),
        "audit output must be a case-local filename",
    )?;
    let text = serde_json::to_string_pretty(&result)? + "\n";
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(case.join(name))?
        .write_all(text.as_bytes())?;
    print!("{text}");
    Ok(pass)
}
fn run() -> Result<bool> {
    let mut args = std::env::args().skip(1);
    let mut case = None;
    let mut name = "mesh-audit.json".to_string();
    while let Some(arg) = args.next() {
        let v = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--case" => case = Some(v),
            "--output-name" => name = v,
            _ => return Err(format!("unknown option {arg}").into()),
        }
    }
    audit(Path::new(&case.ok_or("--case required")?), &name)
}
fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("FAIL: {e}");
            ExitCode::FAILURE
        }
    }
}
