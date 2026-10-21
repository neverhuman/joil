//! Binary STL (millimetres, design frame: `z` downstream) and binary glTF (metres, Y up).

use super::mesh::Mesh;
use std::io::Write;
use std::path::Path;

/// Unit colours: the validated categorical slots (light), in fixed order, as linear RGB.
pub const SERIES_HEX: [u32; 8] = [
    0x2a78d6, 0xeb6834, 0x1baf7a, 0xeda100, 0xe87ba4, 0x008300, 0x6250d6, 0xe34948,
];
pub const CHANNEL_HEX: u32 = 0x9aa3ad;

pub fn linear(hex: u32) -> [f32; 3] {
    let f = |c: u32| {
        let v = c as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    [f((hex >> 16) & 255), f((hex >> 8) & 255), f(hex & 255)]
}

pub fn owner_color(o: u16) -> [f32; 3] {
    if o == u16::MAX {
        linear(CHANNEL_HEX)
    } else {
        linear(SERIES_HEX[o as usize % SERIES_HEX.len()])
    }
}

pub fn write_stl(path: &Path, m: &Mesh, name: &str) -> Result<(), String> {
    let mut buf = Vec::with_capacity(84 + 50 * m.f.len());
    let mut header = [0u8; 80];
    let tag = format!("joil-forge {name} mm");
    header[..tag.len().min(80)].copy_from_slice(&tag.as_bytes()[..tag.len().min(80)]);
    buf.extend_from_slice(&header);
    buf.extend_from_slice(&(m.f.len() as u32).to_le_bytes());
    for t in &m.f {
        let (a, b, c) = (m.v[t[0] as usize], m.v[t[1] as usize], m.v[t[2] as usize]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let mut n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-20);
        n.iter_mut().for_each(|x| *x /= l);
        for p in [n, a, b, c] {
            for x in p {
                buf.extend_from_slice(&x.to_le_bytes());
            }
        }
        buf.extend_from_slice(&[0, 0]);
    }
    std::fs::write(path, buf).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Area-weighted vertex normals.
pub fn vertex_normals(m: &Mesh) -> Vec<[f32; 3]> {
    let mut n = vec![[0.0f32; 3]; m.v.len()];
    for t in &m.f {
        let (a, b, c) = (m.v[t[0] as usize], m.v[t[1] as usize], m.v[t[2] as usize]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let f = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for &i in t {
            for k in 0..3 {
                n[i as usize][k] += f[k];
            }
        }
    }
    for x in &mut n {
        let l = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt().max(1e-20);
        x.iter_mut().for_each(|c| *c /= l);
    }
    n
}

pub struct GlbPart<'a> {
    pub name: &'a str,
    pub mesh: &'a Mesh,
    pub base: [f32; 4],
    pub vertex_colors: bool,
    pub metallic: f32,
}

/// Write a binary glTF with one node per part. Design millimetres (z downstream) become
/// metres with Y up and the cartridge centred on the origin.
pub fn write_glb(path: &Path, parts: &[GlbPart], length_mm: f64) -> Result<(), String> {
    let mut bin: Vec<u8> = Vec::new();
    let (mut views, mut accessors, mut meshes, mut nodes, mut materials) =
        (vec![], vec![], vec![], vec![], vec![]);
    let push_view = |bin: &mut Vec<u8>,
                     views: &mut Vec<serde_json::Value>,
                     bytes: &[u8],
                     target: u32|
     -> usize {
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        let off = bin.len();
        bin.extend_from_slice(bytes);
        views.push(serde_json::json!({"buffer": 0, "byteOffset": off, "byteLength": bytes.len(), "target": target}));
        views.len() - 1
    };
    let half = (length_mm / 2.0) as f32;
    for (pi, part) in parts.iter().enumerate() {
        let m = part.mesh;
        if m.f.is_empty() {
            continue;
        }
        let normals = vertex_normals(m);
        let (mut pos, mut nor, mut col) = (Vec::new(), Vec::new(), Vec::new());
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for (i, v) in m.v.iter().enumerate() {
            let p = [v[0] / 1000.0, (half - v[2]) / 1000.0, v[1] / 1000.0];
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
            p.iter()
                .for_each(|x| pos.extend_from_slice(&x.to_le_bytes()));
            let n = normals[i];
            [n[0], -n[2], n[1]]
                .iter()
                .for_each(|x| nor.extend_from_slice(&x.to_le_bytes()));
            owner_color(m.owner.get(i).copied().unwrap_or(u16::MAX))
                .iter()
                .for_each(|x| col.extend_from_slice(&x.to_le_bytes()));
        }
        let idx: Vec<u8> =
            m.f.iter()
                .flat_map(|t| t.iter().flat_map(|i| i.to_le_bytes()))
                .collect();
        let vp = push_view(&mut bin, &mut views, &pos, 34962);
        let vn = push_view(&mut bin, &mut views, &nor, 34962);
        let vc = push_view(&mut bin, &mut views, &col, 34962);
        let vi = push_view(&mut bin, &mut views, &idx, 34963);
        let a0 = accessors.len();
        accessors.push(serde_json::json!({"bufferView": vp, "componentType": 5126, "count": m.v.len(), "type": "VEC3", "min": lo, "max": hi}));
        accessors.push(serde_json::json!({"bufferView": vn, "componentType": 5126, "count": m.v.len(), "type": "VEC3"}));
        accessors.push(serde_json::json!({"bufferView": vc, "componentType": 5126, "count": m.v.len(), "type": "VEC3"}));
        accessors.push(serde_json::json!({"bufferView": vi, "componentType": 5125, "count": m.f.len() * 3, "type": "SCALAR"}));
        let mut attrs = serde_json::json!({"POSITION": a0, "NORMAL": a0 + 1});
        if part.vertex_colors {
            attrs["COLOR_0"] = serde_json::json!(a0 + 2);
        }
        materials.push(serde_json::json!({
            "name": part.name,
            "pbrMetallicRoughness": {"baseColorFactor": part.base, "metallicFactor": part.metallic, "roughnessFactor": 0.45},
            "alphaMode": if part.base[3] < 1.0 { "BLEND" } else { "OPAQUE" },
            "doubleSided": true
        }));
        meshes.push(serde_json::json!({"name": part.name, "primitives": [{"attributes": attrs, "indices": a0 + 3, "material": materials.len() - 1}]}));
        nodes.push(serde_json::json!({"name": part.name, "mesh": meshes.len() - 1}));
        let _ = pi;
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let json = serde_json::json!({
        "asset": {"version": "2.0", "generator": "joil-forge"},
        "scene": 0,
        "scenes": [{"nodes": (0..nodes.len()).collect::<Vec<_>>()}],
        "nodes": nodes, "meshes": meshes, "materials": materials,
        "accessors": accessors, "bufferViews": views,
        "buffers": [{"byteLength": bin.len()}]
    });
    let mut js = serde_json::to_vec(&json).map_err(|e| e.to_string())?;
    while !js.len().is_multiple_of(4) {
        js.push(b' ');
    }
    let total = 12 + 8 + js.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(js.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&js);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);
    let mut f =
        std::fs::File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    f.write_all(&out).map_err(|e| e.to_string())
}
