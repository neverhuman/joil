mod mesh_support;
use mesh_support::*;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn fixture() -> PathBuf {
    let root = repo().join(format!(
        "runs/i021-prepare-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    put(
        &root,
        "prepare_shim.py",
        include_str!("mesh_support/prepare_shim.py"),
    );
    let script = r#"
import json, math, sys, struct
from pathlib import Path
sys.path.insert(0,str(Path(sys.argv[1])/'cad/ct35'))
from geometry import load
obj,d=load();p=obj['parameters'];root=Path(sys.argv[2])
# Synthetic disconnected closed boxes reproduce terminal areas/planes, not CT35 CAD.
boxes=[]
w=d['w']*.001;h=d['h']*.001
boxes.append(([-.06,0,0],[ -.05,w,h]))
boxes.append(([.05,0,0],[.06,w,h]))
r=math.sqrt(math.pi)*p['plenum_radius_mm']*.001
z=-(2*d['ro']*p['orifice_length_over_do']+p['plenum_length_mm'])*.001
boxes.append(([0,0,z],[r,r,z+.001]))
r=math.sqrt(math.pi)*d['r3']*.001;z=d['ze']*.001
boxes.append(([0,0,z-.001],[r,r,z]))
tri=[]
for lo,hi in boxes:
 v=[[hi[k] if bits[k] else lo[k] for k in range(3)] for bits in [(0,0,0),(1,0,0),(1,1,0),(0,1,0),(0,0,1),(1,0,1),(1,1,1),(0,1,1)]]
 for face in [(0,3,2,1),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)]:
  for ids in [(face[0],face[1],face[2]),(face[0],face[2],face[3])]:tri.append([v[i] for i in ids])
def ascii(name,ts):
 (root/name).write_text('solid synthetic\n'+''.join('facet normal 0 0 0\nouter loop\n'+''.join('vertex '+' '.join(map(str,v))+'\n' for v in t)+'endloop\nendfacet\n' for t in ts)+'endsolid synthetic\n')
ascii('surface.stl',tri);ascii('open.stl',tri[:-1]);ascii('inward.stl',[t[::-1] for t in tri]);ascii('length.stl',[[[v[0],v[1],v[2]*1.01] for v in t] for t in tri])
# Binary coordinates need a genome whose ze is representable in f32 for the strict absolute gate.
z32=struct.unpack('<f',struct.pack('<f',z))[0];p['l3_over_d3']=(z32*1000-d['zt'])/(2*d['r3'])
(root/'binary-genome.json').write_text(json.dumps(obj))
(root/'binary.stl').write_bytes(bytes(80)+struct.pack('<I',len(tri))+b''.join(struct.pack('<12fH',0,0,0,*[x for v in t for x in v],0) for t in tri))
obj['family']='invalid';(root/'bad-genome.json').write_text(json.dumps(obj))
"#;
    let out = Command::new("python3")
        .arg("-S")
        .arg("-c")
        .arg(script)
        .arg(repo())
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    root
}
fn invoke(
    root: &Path,
    python: bool,
    shim: bool,
    name: &str,
    surface: &str,
    level: &str,
    genome: Option<&str>,
) -> Output {
    let mut c = if python {
        let mut c = Command::new(if shim {
            "python3".to_owned()
        } else {
            std::env::var("JOIL_MESH_PYTHON").unwrap_or_else(|_| "python3".into())
        });
        if shim {
            c.arg("-S").arg("-c").arg("import prepare_shim,runpy,sys; p=sys.argv.pop(1);runpy.run_path(p,run_name='__main__')");
        }
        c.arg(repo().join("physics/openfoam/mesh/prepare.py"));
        c
    } else {
        Command::new(env!("CARGO_BIN_EXE_mesh-prepare"))
    };
    c.current_dir(root)
        .args(["--case", name, "--surface", surface, "--level", level]);
    if let Some(g) = genome {
        c.args(["--genome", g]);
    }
    c.env("PYTHONDONTWRITEBYTECODE", "1").output().unwrap()
}
fn compare(a: &Value, b: &Value) {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            assert!(
                (a - b).abs() <= 1e-11 * a.abs().max(b.abs()).max(1e-8),
                "{a} != {b}"
            );
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len());
            for (k, v) in a {
                compare(v, &b[k]);
            }
        }
        _ => assert_eq!(a, b),
    }
}
#[test]
fn synthetic_closed_surface_parity() {
    let installed =
        Command::new(std::env::var("JOIL_MESH_PYTHON").unwrap_or_else(|_| "python3".into()))
            .args(["-c", "import numpy,trimesh"])
            .output()
            .unwrap()
            .status
            .success();
    for shim in [true, false] {
        if !shim && !installed {
            continue;
        }
        for level in ["0", "1"] {
            for binary in [false, true] {
                let root = fixture();
                let (s, g) = if binary {
                    ("binary.stl", Some("binary-genome.json"))
                } else {
                    ("surface.stl", None)
                };
                let a = invoke(&root, true, shim, "python", s, level, g);
                let b = invoke(&root, false, false, "rust", s, level, g);
                assert!(a.status.success(), "{}", String::from_utf8_lossy(&a.stderr));
                assert!(b.status.success(), "{}", String::from_utf8_lossy(&b.stderr));
                let mut a: Value = serde_json::from_slice(&a.stdout).unwrap();
                let mut b: Value = serde_json::from_slice(&b.stdout).unwrap();
                for (v, folder) in [(&mut a, "python"), (&mut b, "rust")] {
                    assert_eq!(
                        v["labelled_surface_sha256"],
                        sha(
                            &fs::read(root.join(folder).join("constant/triSurface/ct35.stl"))
                                .unwrap()
                        )
                    );
                    v["labelled_surface_sha256"] = Value::Null;
                }
                compare(&a, &b);
                assert_eq!(
                    snapshot(&root.join("python/system")),
                    snapshot(&root.join("rust/system"))
                );
                let read_vertices = |folder: &str| {
                    fs::read_to_string(root.join(folder).join("constant/triSurface/ct35.stl"))
                        .unwrap()
                        .lines()
                        .filter(|l| l.trim().starts_with("vertex"))
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                };
                assert_eq!(read_vertices("python"), read_vertices("rust"));
            }
        }
    }
}
#[test]
fn negative_surface_parity() {
    for variant in ["outside", "exists", "open", "inward", "length", "genome"] {
        let root = fixture();
        let s = match variant {
            "open" => "open.stl",
            "inward" => "inward.stl",
            "length" => "length.stl",
            _ => "surface.stl",
        };
        for py in [true, false] {
            let name = if variant == "outside" {
                repo().join("target/i021-outside-case")
            } else {
                root.join(if py { "py-negative" } else { "rs-negative" })
            };
            if variant == "exists" {
                fs::create_dir(&name).unwrap();
            }
            let out = invoke(
                &root,
                py,
                true,
                name.to_str().unwrap(),
                s,
                "0",
                if variant == "genome" {
                    Some("bad-genome.json")
                } else {
                    None
                },
            );
            assert!(!out.status.success(), "accepted {variant}");
        }
    }
}

#[test]
fn genome_validation_parity() {
    for (key, value) in [
        ("n_inlets", 3.),
        ("cone_half_angle_deg", 89.),
        ("d2_mm", -1.),
        ("d1_over_d2", 0.5),
        ("inlet_area_ratio", 100.),
        ("inlet_z_offset_mm", -1.),
        ("plenum_radius_mm", 0.1),
        ("head_spigot_over_d2", 100.),
    ] {
        let root = fixture();
        let mut genome: Value =
            serde_json::from_slice(&fs::read(repo().join("baselines/ct35/genome.json")).unwrap())
                .unwrap();
        genome["parameters"][key] = serde_json::json!(value);
        put(&root, "invalid.json", genome.to_string());
        for py in [true, false] {
            let out = invoke(
                &root,
                py,
                true,
                if py { "python" } else { "rust" },
                "surface.stl",
                "0",
                Some("invalid.json"),
            );
            assert!(!out.status.success(), "accepted {key}");
        }
    }
}
