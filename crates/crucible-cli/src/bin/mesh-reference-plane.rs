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
use serde_json::{json, Value};
use std::{fs, io::Write, process::ExitCode};
fn run() -> Result<()> {
    let o = foam_mesh::options(false)?;
    let extension: Value =
        serde_json::from_slice(&fs::read(o.case.join("domain-extension.json"))?)?;
    require(
        extension["source_case"].as_str() == o.source.file_name().and_then(|s| s.to_str()),
        "extension/source mismatch",
    )?;
    let src = o.source.join("constant/polyMesh");
    let dst = o.case.join("constant/polyMesh");
    let sp = foam_mesh::points(&src)?;
    let sf = foam_mesh::faces(&src)?;
    let dp = foam_mesh::points(&dst)?;
    let df = foam_mesh::faces(&dst)?;
    let boundary = foam_mesh::tokens(&src.join("boundary"))?;
    let start = boundary
        .windows(2)
        .position(|w| w == ["overflow", "{"])
        .ok_or("missing overflow patch")?
        + 2;
    let end = boundary[start..]
        .iter()
        .position(|s| s == "}")
        .ok_or("missing patch end")?
        + start;
    let patch = &boundary[start..end];
    let number = |name: &str| -> Result<usize> {
        let i = patch
            .iter()
            .position(|s| s == name)
            .ok_or("missing patch field")?;
        Ok(patch.get(i + 1).ok_or("missing patch value")?.parse()?)
    };
    let first = number("startFace")?;
    let count = number("nFaces")?;
    let mut wanted = std::collections::HashSet::new();
    for f in sf.iter().skip(first).take(count) {
        let key = foam_mesh::key(&sp, f)?;
        wanted.insert(key);
    }
    let mut ids = Vec::new();
    for (id, face) in df.iter().enumerate() {
        let key = foam_mesh::key(&dp, face)?;
        if !key.iter().flatten().any(|v| f64::from_bits(*v).is_nan()) && wanted.contains(&key) {
            ids.push(id);
        }
    }
    let neighbour = foam_mesh::tokens(&dst.join("neighbour"))?;
    let internal: usize = neighbour
        .first()
        .ok_or("missing neighbour count")?
        .parse()?;
    require(
        wanted.len() == count && ids.len() == count && ids.iter().all(|&i| i < internal),
        "original outlet faces not recovered uniquely as internal faces",
    )?;
    fs::create_dir_all(dst.join("sets"))?;
    let out = dst.join("sets/deviceOverflow");
    let bytes = format!(
        "FoamFile {{format ascii; class faceSet; object deviceOverflow;}}\n{count}\n(\n{}\n)\n",
        ids.iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&out)?
        .write_all(bytes.as_bytes())?;
    let receipt = obj(vec![
        ("kind", val("exact_original_outlet_internal_face_identity")),
        ("faces", val(count)),
        (
            "source_mesh_face_sha256",
            val(extension
                .get("source_mesh_sha256")
                .and_then(|v| v.get("faces"))
                .ok_or("missing source mesh face hash")?
                .clone()),
        ),
        (
            "new_mesh_face_sha256",
            val(sha(fs::read(dst.join("faces"))?)),
        ),
        ("face_set_sha256", val(sha(bytes))),
        (
            "generator_sha256",
            val(sha(fs::read(
                o.root.join("physics/openfoam/mesh/reference_plane.py"),
            )?)),
        ),
        (
            "coordinate_matching",
            val("exact original vertex coordinates and polygon connectivity, order-independent"),
        ),
        ("outward_direction", val(json!([0, 0, -1]))),
    ]);
    let bytes = receipt.pretty(0) + "\n";
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(o.case.join("device-plane.json"))?
        .write_all(bytes.as_bytes())?;
    print!("{bytes}");
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
