//! Synthetic numerical fixtures, not experimental or physical validation.
use crucible_geom::export::*;
use crucible_geom::surface::*;
use crucible_geom::tet::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
fn digest(s: &str) -> String {
    sha256(s.as_bytes())
}
fn policy() -> SurfacePolicy {
    SurfacePolicy {
        length_tolerance_m: 1e-11,
        minimum_area_m2: 1e-16,
        minimum_shape_ratio: 1e-8,
        maximum_deviation_m: 1e-6,
        volume_tolerance_m3: 1e-12,
    }
}
fn tet_policy() -> TetPolicy {
    TetPolicy {
        minimum_jacobian_m3: 1e-15,
        minimum_scaled_jacobian: 1e-9,
        maximum_projection_m: 3.0,
        volume_tolerance_m3: 1e-10,
    }
}
fn tag(patch: &str) -> BoundaryTag {
    BoundaryTag {
        patch: patch.into(),
        inside_material: "fluid".into(),
        outside_material: "liner".into(),
        roughness_m: 35e-6,
    }
}
fn boundary(points: Vec<Point>, triangles: Vec<[usize; 3]>, volume: f64) -> PartitionBoundary {
    PartitionBoundary {
        partition_sha256: digest("synthetic explicit partition"),
        acceptance_receipt_sha256: digest(
            "synthetic fixture upstream receipt, no production admission",
        ),
        compiler: "synthetic-facets/1".into(),
        kernel: "explicit-f64-facets/1".into(),
        vertices_m: points,
        facets: triangles
            .into_iter()
            .map(|vertices| Facet {
                vertices,
                tag: tag("wall"),
            })
            .collect(),
        active_features: BTreeSet::from([Feature::FacetedBoundary]),
        volume_bounds_m3: [volume * (1.0 - 1e-12), volume * (1.0 + 1e-12)],
        upstream_deviation_bound_m: 0.0,
    }
}
fn tetra() -> PartitionBoundary {
    boundary(
        vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![[1, 2, 3], [0, 3, 2], [0, 1, 3], [0, 2, 1]],
        1.0 / 6.0,
    )
}
fn cube(scale: Point) -> PartitionBoundary {
    let [x, y, z] = scale;
    let mut b = boundary(
        vec![
            [0., 0., 0.],
            [x, 0., 0.],
            [x, y, 0.],
            [0., y, 0.],
            [0., 0., z],
            [x, 0., z],
            [x, y, z],
            [0., y, z],
        ],
        vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
        x * y * z,
    );
    for (i, f) in b.facets.iter_mut().enumerate() {
        f.tag = tag(match i / 2 {
            0 => "feed-oblique-A",
            1 => "reject",
            _ => "wall",
        });
    }
    b
}
fn card() -> CoverageCard {
    CoverageCard {
        backend: "explicit-facet-export".into(),
        version: "1".into(),
        compiler: "synthetic-facets/1".into(),
        kernel: "explicit-f64-facets/1".into(),
        input_tolerance_m: 1e-6,
        supported_features: BTreeSet::from([Feature::FacetedBoundary, Feature::ThinShell]),
        admissible_consumers: BTreeSet::from(["display".into(), "fixture-fea".into()]),
        limitations: vec!["synthetic fixtures; V5-08 acceptance remains external".into()],
    }
}
fn run(name: &str) -> PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    let root = ROOT.get_or_init(|| {
        let root = std::env::var_os("V5_ARTIFACT_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.private/artifact-tests")
            });
        let path = root.join(format!(
            "artifacts-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    });
    root.join(name)
}
#[test]
fn rotated_asymmetric_ports_keep_authoritative_tags() {
    let mut b = cube([2., 0.7, 0.4]);
    let angle = 0.61_f64;
    let (s, c) = angle.sin_cos();
    for p in &mut b.vertices_m {
        let [x, y, z] = *p;
        let u = c * x - s * y;
        *p = [
            0.8 * u + 0.6 * z + 1.3,
            s * x + c * y - 0.2,
            -0.6 * u + 0.8 * z + 0.11,
        ];
    }
    let surface = extract(b.clone(), policy()).unwrap();
    assert!((surface.audit().volume_m3 - 0.56).abs() < 1e-12);
    assert_eq!(surface.boundary().facets, b.facets);
    let feed = &b.facets[0];
    let p = feed.vertices.map(|i| b.vertices_m[i]);
    // No axis-constant classifier can discover this oblique inlet.
    assert!(!(0..3).any(|k| p.iter().all(|q| (q[k] - p[0][k]).abs() < 1e-12)));
    assert!(surface.audit().patch_area_m2["feed-oblique-A"] > 0.0);
    let mut guessed = b;
    guessed.facets[0].tag = tag("guessed-wall");
    let changed = extract(guessed, policy()).unwrap();
    assert_ne!(changed.boundary_sha256(), surface.boundary_sha256());
}
#[test]
fn surface_negative_corpus() {
    let mut b = tetra();
    b.facets.pop();
    assert!(extract(b, policy()).is_err());
    let mut b = tetra();
    b.facets[0].vertices.swap(0, 1);
    assert!(extract(b, policy()).is_err());
    let mut b = tetra();
    b.facets.push(b.facets[0].clone());
    assert!(extract(b, policy()).is_err());
    let mut b = tetra();
    b.vertices_m[1][0] = f64::NAN;
    assert!(extract(b, policy()).is_err());
    let mut b = tetra();
    b.volume_bounds_m3 = [0.2, 0.3];
    assert!(extract(b, policy()).is_err());
    let mut b = tetra();
    b.upstream_deviation_bound_m = 1e-2;
    assert!(extract(b, policy()).is_err());
    let mut b = tetra();
    b.facets[0].tag.roughness_m = -1.;
    assert!(extract(b, policy()).is_err());
    let mut b = tetra();
    b.facets[1].tag.outside_material = "other".into();
    assert!(extract(b, policy()).is_err());
    let mut b = tetra();
    b.vertices_m[3] = [0.1, 0.1, 1e-15];
    assert!(extract(b, policy()).is_err());
}
#[test]
fn intersecting_and_disconnected_shells_do_not_pass_volume_checks() {
    // Closed connected octahedron with one pole pushed through its opposite.
    let points = vec![
        [1., 0., 0.],
        [0., 1., 0.],
        [-1., 0., 0.],
        [0., -1., 0.],
        [0., 0., 1.],
        [0.3, 0.3, 2.],
    ];
    let tris = vec![
        [0, 1, 4],
        [1, 2, 4],
        [2, 3, 4],
        [3, 0, 4],
        [1, 0, 5],
        [2, 1, 5],
        [3, 2, 5],
        [0, 3, 5],
    ];
    let mut b = boundary(points, tris, 1.);
    b.volume_bounds_m3 = [1e-12, 100.];
    assert!(extract(b, policy()).is_err());
    let mut b = tetra();
    let second = tetra();
    let offset = b.vertices_m.len();
    b.vertices_m.extend(
        second
            .vertices_m
            .into_iter()
            .map(|p| [p[0] + 2., p[1], p[2]]),
    );
    b.facets.extend(second.facets.into_iter().map(|mut f| {
        f.vertices = f.vertices.map(|i| i + offset);
        f
    }));
    b.volume_bounds_m3 = [0.3, 0.4];
    assert!(matches!(
        extract(b, policy()),
        Err(ArtifactError::Unsupported(_))
    ));
}
#[test]
fn p1_patch_thin_wall_and_distorted_boundaries_have_positive_jacobians() {
    for (name, scale) in [
        ("patch", [1., 1., 1.]),
        ("thin-wall", [1., 1., 0.001]),
        ("distorted", [0.2, 1.7, 0.31]),
    ] {
        let mut b = tetra();
        for p in &mut b.vertices_m {
            let [x, y, z] = *p;
            *p = [
                x * scale[0] + 0.13 * y,
                y * scale[1] + 0.2 * z,
                z * scale[2],
            ];
        }
        let volume = scale.iter().product::<f64>() / 6.;
        b.volume_bounds_m3 = [volume - 1e-14, volume + 1e-14];
        if name == "thin-wall" {
            b.active_features.insert(Feature::ThinShell);
        }
        let surface = extract(b, policy()).unwrap();
        let mesh = qualify_p1(
            &surface,
            surface.boundary().vertices_m.clone(),
            vec![[0, 1, 2, 3]],
            tet_policy(),
        )
        .unwrap();
        assert!(mesh.audit().minimum_jacobian_bound_m3 > 0.);
        let p2 = elevate_p2(&mesh, &BTreeMap::new()).unwrap();
        assert!(p2.audit().minimum_jacobian_bound_m3 > 0.);
        assert!((p2.audit().volume_m3 - volume).abs() < 1e-12);
    }
}
#[test]
fn p2_midpoint_projection_inverts_valid_p1_and_is_rejected() {
    let surface = extract(tetra(), policy()).unwrap();
    let mesh = qualify_p1(
        &surface,
        surface.boundary().vertices_m.clone(),
        vec![[0, 1, 2, 3]],
        tet_policy(),
    )
    .unwrap();
    let tangent_projection = BTreeMap::from([([0, 1], [0.52, 0.0, 0.0])]);
    let projected_positive = elevate_p2(&mesh, &tangent_projection).unwrap();
    assert!(projected_positive.audit().minimum_jacobian_bound_m3 > 0.9);
    assert!(projected_positive.audit().projection_bound_m > 0.0);
    let projected = BTreeMap::from([([0, 1], [0.5, 1.2, 1.2])]);
    assert!(matches!(
        elevate_p2(&mesh, &projected),
        Err(ArtifactError::NumericalFailure(_))
    ));
    let p2 = elevate_p2(&mesh, &BTreeMap::new()).unwrap();
    let values = jacobian_bernstein(p2.elements()[0].map(|i| p2.nodes_m()[i])).unwrap();
    assert_eq!(values.len(), 20);
    assert!(values.iter().all(|x| (*x - 1.).abs() < 1e-12));
    assert!(elevate_p2(&mesh, &BTreeMap::from([([9, 10], [0.; 3])])).is_err());
}
#[test]
fn tetrahedral_complex_requires_same_complete_oriented_boundary() {
    let surface = extract(tetra(), policy()).unwrap();
    assert!(qualify_p1(
        &surface,
        surface.boundary().vertices_m.clone(),
        vec![[1, 0, 2, 3]],
        tet_policy()
    )
    .is_err());
    assert!(qualify_p1(
        &surface,
        surface.boundary().vertices_m.clone(),
        vec![[0, 1, 2, 3]; 2],
        tet_policy()
    )
    .is_err());
    let mut changed = surface.boundary().vertices_m.clone();
    changed[1][0] = 1000.;
    assert!(qualify_p1(&surface, changed, vec![[0, 1, 2, 3]], tet_policy()).is_err());
    // Split one tetra at its interior centroid: an independent conforming seed.
    let mut nodes = surface.boundary().vertices_m.clone();
    nodes.push([0.25; 3]);
    let elements = vec![[4, 1, 2, 3], [0, 4, 2, 3], [0, 1, 4, 3], [0, 1, 2, 4]];
    let mesh = qualify_p1(&surface, nodes, elements, tet_policy()).unwrap();
    assert!((mesh.audit().volume_m3 - 1. / 6.).abs() < 1e-12);
    elevate_p2(&mesh, &BTreeMap::new()).unwrap();
}
#[test]
fn unsupported_active_features_and_fidelity_block_exports() {
    let mut b = tetra();
    b.active_features.insert(Feature::ResolvedLattice);
    let surface = extract(b, policy()).unwrap();
    let mut coverage = card();
    coverage
        .supported_features
        .insert(Feature::HomogenizedPorous);
    let path = run("blocked-lattice");
    assert!(export_bundle(
        &surface,
        &path,
        &coverage,
        "display",
        RigidTransform::default(),
        BuildUnit::Metre
    )
    .is_err());
    assert!(!path.exists());
    let surface = extract(tetra(), policy()).unwrap();
    assert!(card().admit(&surface, "reference-cfd-unqualified").is_err());
    let mut coverage = card();
    coverage.kernel = "different".into();
    assert!(coverage.admit(&surface, "display").is_err());
}
#[test]
fn frozen_liner_reuse_demands_interface_and_conservative_mapping() {
    let good = LinerReuse {
        source_interface_sha256: digest("interface"),
        target_interface_sha256: digest("interface"),
        source_topology_sha256: digest("topology"),
        target_topology_sha256: digest("topology"),
        source_areas_m2: vec![1., 2.],
        target_areas_m2: vec![1., 2.],
        weights: vec![vec![1., 0.], vec![0., 1.]],
        relative_tolerance: 1e-12,
    };
    assert!(good.verify().is_ok());
    let mut bad = good.clone();
    bad.target_interface_sha256 = digest("changed");
    assert!(bad.verify().is_err());
    let mut bad = good.clone();
    bad.weights[0][0] = 0.9;
    assert!(bad.verify().is_err());
    let mut bad = good.clone();
    bad.target_areas_m2[0] = 2.;
    assert!(bad.verify().is_err());
    let mut bad = good;
    bad.weights[0][0] = f64::NAN;
    assert!(bad.verify().is_err());
}
fn stl_points(path: &std::path::Path) -> Vec<Point> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let parts: Vec<_> = line.split_whitespace().collect();
            (parts.first() == Some(&"vertex")).then(|| {
                [
                    parts[1].parse().unwrap(),
                    parts[2].parse().unwrap(),
                    parts[3].parse().unwrap(),
                ]
            })
        })
        .collect()
}
fn glb_points(path: &std::path::Path) -> (Value, Vec<Point>) {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"glTF");
    let u32at = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
    assert_eq!(u32at(4), 2);
    assert_eq!(u32at(8), bytes.len());
    let json_len = u32at(12);
    let doc: Value = serde_json::from_slice(&bytes[20..20 + json_len]).unwrap();
    let start = 20 + json_len + 8;
    let count = doc["accessors"][0]["count"].as_u64().unwrap() as usize;
    let points = (0..count)
        .map(|i| {
            std::array::from_fn(|axis| {
                f32::from_le_bytes(
                    bytes[start + i * 12 + axis * 4..start + i * 12 + axis * 4 + 4]
                        .try_into()
                        .unwrap(),
                ) as f64
            })
        })
        .collect();
    (doc, points)
}
fn independently_check_3mf(path: &std::path::Path, expected: Point, unit: &str) {
    // Standard-library ZIP CRC validation and XML parser, independent of writer.
    let script = r#"import sys,zipfile,xml.etree.ElementTree as E
with zipfile.ZipFile(sys.argv[1]) as z:
 assert z.testzip() is None
 assert set(z.namelist())=={'[Content_Types].xml','_rels/.rels','3D/3dmodel.model'}
 r=E.fromstring(z.read('3D/3dmodel.model'))
 assert r.attrib['unit']==sys.argv[2]
 ns={'c':'http://schemas.microsoft.com/3dmanufacturing/core/2015/02'}
 v=[[float(p.attrib[k]) for k in ('x','y','z')] for p in r.findall('.//c:vertex',ns)]
 scale=1000 if sys.argv[2]=='millimeter' else 1
 for i in range(3):
  measured=(max(p[i] for p in v)-min(p[i] for p in v))/scale
  assert abs(measured-float(sys.argv[3+i]))<1e-11,(i,measured,sys.argv[3+i])
"#;
    let status = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(path)
        .arg(unit)
        .args(expected.map(|x| x.to_string()))
        .status()
        .unwrap();
    assert!(status.success());
}
#[test]
fn unit_cube_exports_have_independent_dimensions_tags_and_digests() {
    let surface = extract(cube([1.; 3]), policy()).unwrap();
    let path = run("unit-cube");
    let manifest = export_bundle(
        &surface,
        &path,
        &card(),
        "display",
        RigidTransform::default(),
        BuildUnit::Millimetre,
    )
    .unwrap();
    let mut stl = Vec::new();
    for f in &manifest.files {
        let bytes = fs::read(path.join(&f.path)).unwrap();
        assert_eq!(sha256(&bytes), f.sha256);
        if f.path.ends_with(".stl") {
            stl.extend(stl_points(&path.join(&f.path)));
        }
    }
    assert_eq!(bounding_box(&stl), [[0.; 3], [1.; 3]]);
    let (doc, glb) = glb_points(&path.join("scene.glb"));
    assert_eq!(bounding_box(&glb), [[0.; 3], [1.; 3]]);
    assert_eq!(doc["extras"]["boundary_sha256"], surface.boundary_sha256());
    assert_eq!(doc["meshes"][0]["primitives"].as_array().unwrap().len(), 3);
    independently_check_3mf(&path.join("build.3mf"), [1.; 3], "millimeter");
    assert!(!manifest.scientific_admission);
}
fn ct35_axial_body() -> PartitionBoundary {
    // Full axial length/dimensions from frozen public reference. This fixture
    // explicitly omits inlet ducts/overflow: it is NOT the accepted CT-35 B-rep.
    let reference: Value =
        serde_json::from_str(include_str!("../../../baselines/ct35/reference.json")).unwrap();
    let d = &reference["derived_mm"];
    let val = |name: &str| d[name].as_f64().unwrap() * 0.001;
    let sections = [
        (0., val("R1")),
        (val("L1"), val("R1")),
        (val("z_cone_end"), val("R2")),
        (val("z_taper_end"), val("R3")),
        (val("z_end"), val("R3")),
    ];
    let n = 16;
    let mut nodes = Vec::new();
    for (z, r) in sections {
        for k in 0..n {
            let a = k as f64 * std::f64::consts::TAU / n as f64;
            nodes.push([r * a.cos(), r * a.sin(), z]);
        }
    }
    let bottom = nodes.len();
    nodes.push([0., 0., 0.]);
    let top = nodes.len();
    nodes.push([0., 0., val("z_end")]);
    let mut triangles = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        triangles.push([bottom, j, i]);
        triangles.push([top, 4 * n + i, 4 * n + j]);
    }
    for s in 0..4 {
        for i in 0..n {
            let j = (i + 1) % n;
            let a = s * n + i;
            let b = s * n + j;
            let c = (s + 1) * n + j;
            let e = (s + 1) * n + i;
            triangles.extend([[a, b, c], [a, c, e]]);
        }
    }
    // Independent polygon-frustum integral, not divergence-theorem duplicate.
    let area_factor = (n as f64 / 2.) * (std::f64::consts::TAU / n as f64).sin();
    let volume = sections
        .windows(2)
        .map(|w| {
            let (za, ra) = w[0];
            let (zb, rb) = w[1];
            area_factor * (zb - za) * (ra * ra + ra * rb + rb * rb) / 3.
        })
        .sum::<f64>();
    let mut b = boundary(nodes, triangles, volume);
    b.partition_sha256 = digest("synthetic CT-35 axial-body-only dimensions, not full baseline");
    b
}
#[test]
fn ct35_source_bound_axial_dimension_export_control() {
    let surface = extract(ct35_axial_body(), policy()).unwrap();
    let path = run("ct35-axial-body-only");
    let manifest = export_bundle(
        &surface,
        &path,
        &card(),
        "display",
        RigidTransform::default(),
        BuildUnit::Metre,
    )
    .unwrap();
    let (_, points) = glb_points(&path.join("scene.glb"));
    let b = bounding_box(&points);
    let expected = [0.07, 0.07, 1.639_247_431_843_31];
    for axis in 0..3 {
        assert!((b[1][axis] - b[0][axis] - expected[axis]).abs() < 2e-7);
    }
    independently_check_3mf(&path.join("build.3mf"), expected, "meter");
    assert_eq!(
        manifest.partition_sha256,
        surface.boundary().partition_sha256
    );
}
#[test]
fn export_rejects_scaling_false_exact_step_and_existing_evidence() {
    let surface = extract(tetra(), policy()).unwrap();
    let mut transform = RigidTransform::default();
    transform.rotation[0][0] = 1000.;
    let path = run("bad-scale");
    assert!(export_bundle(
        &surface,
        &path,
        &card(),
        "display",
        transform,
        BuildUnit::Metre
    )
    .is_err());
    assert!(!path.exists());
    assert!(request_step(&surface, StepLabel::Exact).is_err());
    assert!(request_step(&surface, StepLabel::TessellatedApproximation).is_err());
    let path = run("existing");
    fs::create_dir(&path).unwrap();
    let evidence = path.join("governing.stl");
    fs::write(&evidence, b"must survive").unwrap();
    assert!(export_bundle(
        &surface,
        &path,
        &card(),
        "display",
        RigidTransform::default(),
        BuildUnit::Metre
    )
    .is_err());
    assert_eq!(fs::read(evidence).unwrap(), b"must survive");
    let path = run("existing-file");
    fs::write(&path, b"previous journal").unwrap();
    assert!(export_bundle(
        &surface,
        &path,
        &card(),
        "display",
        RigidTransform::default(),
        BuildUnit::Metre
    )
    .is_err());
    assert_eq!(fs::read(path).unwrap(), b"previous journal");
}
#[test]
fn fixed_corpus_retains_meshes_raw_inputs_cost_and_failure_diagnosis() {
    let corpus = [
        ("tetra-patch", tetra()),
        ("thin-wall", cube([0.1, 0.1, 0.001])),
        ("ct35-axial-body-only", ct35_axial_body()),
        ("open-negative", {
            let mut b = cube([1.; 3]);
            b.facets.pop();
            b
        }),
    ];
    let mut records = Vec::new();
    for (name, input) in corpus {
        let seed = match name {
            "tetra-patch" => [0.25; 3],
            "thin-wall" => [0.05, 0.05, 0.0005],
            "ct35-axial-body-only" => [0.0, 0.0, 0.03],
            _ => [0.5; 3],
        };
        let source_reference = if name == "ct35-axial-body-only" {
            serde_json::from_str::<Value>(include_str!("../../../baselines/ct35/reference.json"))
                .unwrap()
        } else {
            Value::Null
        };
        let bytes=serde_json::to_vec(&json!({"synthetic":true,"fixture":name,"partition_boundary":input,"surface_policy":policy(),"tet_policy":tet_policy(),"interior_seed_m":seed,"source_reference":source_reference,"compiler":"synthetic-facets/1","kernel":"explicit-f64-facets/1"})).unwrap();
        let path = run(&format!("benchmark-{name}"));
        let record = measure_attempt(
            name,
            &bytes,
            &path,
            RetainedRole::GoverningFeaMesh,
            |raw| {
                let raw: Value = serde_json::from_slice(raw).unwrap();
                let input: PartitionBoundary =
                    serde_json::from_value(raw["partition_boundary"].clone()).unwrap();
                let surface_policy: SurfacePolicy =
                    serde_json::from_value(raw["surface_policy"].clone()).unwrap();
                let tet_policy: TetPolicy =
                    serde_json::from_value(raw["tet_policy"].clone()).unwrap();
                let seed: Point = serde_json::from_value(raw["interior_seed_m"].clone()).unwrap();
                Ok((extract(input, surface_policy)?, seed, tet_policy))
            },
            |(surface, seed, tet_policy)| {
                let mesh = mesh_from_interior_seed(&surface, seed, tet_policy)?;
                serde_json::to_vec(&mesh).map_err(|e| ArtifactError::Invalid(e.to_string()))
            },
        )
        .unwrap();
        let required = BTreeSet::from([
            RetainedRole::RawInput,
            RetainedRole::Provenance,
            RetainedRole::GoverningFeaMesh,
        ]);
        let expected: BTreeMap<_, _> = record
            .retention
            .objects
            .iter()
            .filter(|o| matches!(o.role, RetainedRole::GoverningFeaMesh))
            .map(|o| (o.role.clone(), o.sha256.clone()))
            .collect();
        assert!(record.elapsed_ns > 0);
        assert!(!record.physical_infeasibility);
        if name == "open-negative" {
            assert!(matches!(
                record.outcome,
                AttemptOutcome::NumericalArtifactFailure(_)
            ));
            assert!(record
                .retention
                .verify(&path, &required, &expected)
                .is_err());
        } else {
            assert!(matches!(record.outcome, AttemptOutcome::AcceptedArtifact));
            record
                .retention
                .verify(&path, &required, &expected)
                .unwrap();
        }
        records.push(record);
    }
    fs::write(
        run("fixed-corpus-results.json"),
        serde_json::to_vec_pretty(
            &json!({"synthetic":true,"accepted":3,"attempted":4,"records":records}),
        )
        .unwrap(),
    )
    .unwrap();
}
#[test]
fn scalar_only_receipt_never_authorizes_mesh_deletion() {
    let path = run("scalar-only");
    fs::create_dir(&path).unwrap();
    fs::write(path.join("receipt.json"), b"{\"volume\":1}").unwrap();
    let closure = RetentionClosure {
        objects: vec![RetainedObject {
            path: "receipt.json".into(),
            sha256: sha256(b"{\"volume\":1}"),
            bytes: 12,
            role: RetainedRole::Provenance,
        }],
    };
    let required = BTreeSet::from([
        RetainedRole::RawInput,
        RetainedRole::Provenance,
        RetainedRole::GoverningFeaMesh,
    ]);
    assert!(closure
        .verify(
            &path,
            &required,
            &BTreeMap::from([(
                RetainedRole::GoverningFeaMesh,
                digest("accepted governing mesh")
            )])
        )
        .is_err());
    let surface = extract(tetra(), policy()).unwrap();
    let mesh = qualify_p1(
        &surface,
        surface.boundary().vertices_m.clone(),
        vec![[0, 1, 2, 3]],
        tet_policy(),
    )
    .unwrap();
    let mesh_digest = sha256(&serde_json::to_vec(&mesh).unwrap());
    let path = run("retained-fea");
    let raw = serde_json::to_vec(&surface).unwrap();
    let record = measure_attempt(
        "p1-fea",
        &raw,
        &path,
        RetainedRole::GoverningFeaMesh,
        |_| Ok(mesh),
        |m| Ok(serde_json::to_vec(&m).unwrap()),
    )
    .unwrap();
    let expected = BTreeMap::from([(RetainedRole::GoverningFeaMesh, mesh_digest)]);
    record
        .retention
        .verify(&path, &required, &expected)
        .unwrap();
    assert!(record
        .retention
        .verify(
            &path,
            &required,
            &BTreeMap::from([(RetainedRole::GoverningFeaMesh, digest("unrelated mesh"))])
        )
        .is_err());
    fs::write(path.join("governing-mesh.json"), b"tampered").unwrap();
    assert!(record
        .retention
        .verify(&path, &required, &expected)
        .is_err());
}

#[test]
fn interior_seed_generator_retains_boundary_and_rejects_bad_seed() {
    for scale in [[1.0, 1.0, 1.0], [0.2, 0.1, 0.001], [1.2, 0.7, 0.3]] {
        let surface = extract(cube(scale), policy()).unwrap();
        let mesh = mesh_from_interior_seed(&surface, scale.map(|x| x * 0.5), tet_policy()).unwrap();
        assert_eq!(mesh.elements().len(), 12);
        assert!((mesh.audit().volume_m3 - scale.iter().product::<f64>()).abs() < 1e-12);
        assert!(mesh.audit().minimum_jacobian_bound_m3 > 0.0);
        let serialized = serde_json::to_value(&mesh).unwrap();
        assert_eq!(
            serialized["provenance"]["compiler"],
            surface.boundary().compiler
        );
        assert!(
            serialized["provenance"]["admissible_consumers"]
                .as_array()
                .unwrap()
                .len()
                == 1
        );
        elevate_p2(&mesh, &BTreeMap::new()).unwrap();
        assert!(mesh_from_interior_seed(&surface, [-1.0; 3], tet_policy()).is_err());
    }
}

#[test]
fn v5_10_03_coverage_binds_compiler_kernel_tolerance_consumer_and_resolved_features() {
    let surface = extract(tetra(), policy()).unwrap();
    assert!(card().admit(&surface, "fixture-fea").is_ok());
    for mutation in 0..6 {
        let mut c = card();
        match mutation {
            0 => c.compiler = "other".into(),
            1 => c.kernel = "other".into(),
            2 => c.input_tolerance_m = f64::NAN,
            3 => c.admissible_consumers.clear(),
            4 => c.supported_features.clear(),
            _ => c.input_tolerance_m = -1.0,
        }
        assert!(
            c.admit(&surface, "fixture-fea").is_err(),
            "mutation {mutation}"
        );
    }
    let mut boundary = tetra();
    boundary.active_features.insert(Feature::ResolvedLattice);
    let surface = extract(boundary, policy()).unwrap();
    let mut c = card();
    c.supported_features.insert(Feature::HomogenizedPorous);
    assert!(c.admit(&surface, "fixture-fea").is_err());
    c.supported_features.insert(Feature::ResolvedLattice);
    assert!(c.admit(&surface, "fixture-fea").is_ok());
}
