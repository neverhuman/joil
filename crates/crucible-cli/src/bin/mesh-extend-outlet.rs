#[allow(dead_code)]
#[path = "../foam_mesh.rs"]
mod foam_mesh;
#[path = "../foam_runtime.rs"]
mod foam_runtime;
#[allow(dead_code)]
#[path = "../ordered_json.rs"]
mod ordered_json;
#[allow(dead_code)]
#[path = "../port_support.rs"]
mod port_support;
use ordered_json::Ordered;
use port_support::*;
use serde_json::Value;
use std::{fs, path::Path, process::ExitCode};
fn copy_tree(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let dst = target.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &dst)?;
        } else {
            fs::copy(entry.path(), dst)?;
        }
    }
    Ok(())
}
fn hashes(folder: &Path) -> Result<Ordered> {
    let mut result = Vec::new();
    for name in ["points", "faces", "owner", "neighbour", "boundary"] {
        result.push((name, val(sha(fs::read(folder.join(name))?))));
    }
    Ok(obj(result))
}
fn length_label(value: f64) -> Result<String> {
    let mut buffer: [libc::c_char; 64] = [0; 64];
    // SAFETY: the fixed printf format consumes a double and the bounded buffer is writable.
    let n = unsafe { libc::snprintf(buffer.as_mut_ptr(), buffer.len(), c"%.15g".as_ptr(), value) };
    let n = usize::try_from(n)?;
    require(n < buffer.len(), "invalid extension length")?;
    Ok(buffer[..n]
        .iter()
        .map(|b| char::from(b.to_ne_bytes()[0]))
        .collect())
}
fn run() -> Result<()> {
    let o = foam_mesh::options(true)?;
    require(!o.case.exists(), "fresh case required")?;
    require(
        o.length > 0. && o.length <= 0.3 && (2..=300).contains(&o.layers),
        "extension outside pilot budget",
    )?;
    require(
        !o.source.join("domain-extension.json").exists(),
        "extend the original domain only",
    )?;
    let source_audit: Value = serde_json::from_slice(&fs::read(o.source.join("mesh-audit.json"))?)?;
    let area = source_audit["patches"]["overflow"]["area_m2"]
        .as_f64()
        .ok_or("missing outlet area")?;
    fs::create_dir_all(o.case.join("system"))?;
    fs::create_dir(o.case.join("constant"))?;
    let src = o.source.join("constant/polyMesh");
    let dst = o.case.join("constant/polyMesh");
    copy_tree(&src, &dst)?;
    fs::copy(
        o.source.join("system/controlDict"),
        o.case.join("system/controlDict"),
    )?;
    fs::write(o.case.join("system/controlDict"), "FoamFile {format ascii; class dictionary; object controlDict;}\napplication extrudeMesh; startFrom startTime; startTime 0; stopAt endTime; endTime 1; deltaT 1; writeControl timeStep; writeInterval 1;\n")?;
    let dictionary = format!("FoamFile {{format ascii; class dictionary; object extrudeMeshDict;}}\nconstructFrom mesh;\nsourceCase \".\";\nsourcePatches (overflow);\nflipNormals false;\nzoneNames (numericalOverflowExtension);\nextrudeModel\n{{\n    type linearDirection;\n    nLayers {};\n    expansionRatio 1;\n    direction (0 0 -1);\n    thickness {};\n}}\nmergeFaces false;\nmergeTol 0;\n",o.layers,length_label(o.length)?);
    fs::write(o.case.join("system/extrudeMeshDict"), &dictionary)?;
    let inputs = hashes(&src)?;
    let mut mesh_input = Ordered::parse(&o.source.join("mesh-input.json"))?;
    let original_json = value(mesh_input.field("analytic_volume_m3")?)?;
    let original = original_json.as_f64().ok_or("missing volume")?;
    mesh_input.put("analytic_volume_m3".into(), val(original + area * o.length))?;
    let overflow = mesh_input.field("patches")?.field("overflow")?;
    if let Ok(position) = overflow.field("plane_position_m") {
        *position = val(value(position)?.as_f64().ok_or("invalid plane position")? - o.length);
    }
    mesh_input.put(
        "numerical_domain_extension".into(),
        obj(vec![
            ("patch", val("overflow")),
            ("length_m", val(o.length)),
            ("layers", val(o.layers)),
            ("original_analytic_volume_m3", val(original_json)),
            (
                "extruded_polygon_area_m2",
                val(source_audit["patches"]["overflow"]["area_m2"].clone()),
            ),
            ("baseline_geometry_changed", val(false)),
            (
                "original_terminal_becomes_internal_reference_plane",
                val(true),
            ),
        ]),
    )?;
    fs::write(
        o.case.join("mesh-input.json"),
        foam_mesh::json_bytes(&mesh_input),
    )?;
    let lock: Value = serde_json::from_slice(&fs::read(
        o.root.join("physics/openfoam/environment/lock.json"),
    )?)?;
    foam_runtime::probe(
        &o.root,
        &o.case,
        &lock,
        "extension-0-extrudeMesh",
        &["extrudeMesh"],
        300,
    )?;
    let before = foam_mesh::points(&src)?;
    let after: std::collections::HashSet<_> = foam_mesh::points(&dst)?
        .iter()
        .map(foam_mesh::point_key)
        .collect();
    require(
        before
            .iter()
            .all(|p| !p.iter().any(|v| v.is_nan()) && after.contains(&foam_mesh::point_key(p))),
        "extrusion changed an original mesh point",
    )?;
    let receipt = obj(vec![
        ("kind", val("explicit_numerical_outlet_extension")),
        (
            "source_case",
            val(o
                .source
                .file_name()
                .ok_or("missing source name")?
                .to_string_lossy()
                .into_owned()),
        ),
        ("source_mesh_sha256", inputs),
        (
            "source_mesh_audit_sha256",
            val(sha(fs::read(o.source.join("mesh-audit.json"))?)),
        ),
        ("length_m", val(o.length)),
        ("layers", val(o.layers)),
        ("original_points_preserved", val(true)),
        ("baseline_geometry_changed", val(false)),
        (
            "source_audit_pass",
            val(source_audit
                .get("pilot_mesh_pass")
                .ok_or("missing source audit pass")?
                .clone()),
        ),
        ("new_mesh_sha256", hashes(&dst)?),
        (
            "generator_sha256",
            val(sha(fs::read(
                o.root.join("physics/openfoam/mesh/extend_outlet.py"),
            )?)),
        ),
        ("dictionary_sha256", val(sha(dictionary))),
        (
            "device_and_full_domain_energy_must_be_distinguished",
            val(true),
        ),
        ("campaign_admission", val(false)),
    ]);
    fs::write(
        o.case.join("domain-extension.json"),
        foam_mesh::json_bytes(&receipt),
    )?;
    foam_runtime::probe(
        &o.root,
        &o.case,
        &lock,
        "mesh-2-checkMesh",
        &["checkMesh", "-allTopology", "-allGeometry"],
        300,
    )?;
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
