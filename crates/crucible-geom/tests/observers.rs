//! Synthetic software fixtures only. Authority is created explicitly for tests;
//! it is not a geometry acceptance or physical validation receipt.
use crucible_contract::subject::*;
use crucible_geom::observe::{
    self,
    clearance::{self, ClearanceGate, ManufacturingProfile},
    distance,
    enactment::{self, DevelopmentFailure, Enactment, ProbeStep},
    mesh, topology,
    volume::{self, Cell, PortSeeds, Volume},
    BoundVolume, Mesh, ObserveError,
};
use crucible_units::{CoordinateFrame, FrameTransform, Metres};
use joil_evidence::{
    claims::{Registry, ReviewAction, ReviewEvent},
    identity::ContentRef,
    observation::Finite,
};
use std::collections::BTreeMap;

fn near(a: f64, b: f64, t: f64) {
    assert!((a - b).abs() <= t, "{a} != {b} (tolerance {t})");
}
fn m(x: f64) -> Metres {
    Metres::new(x).unwrap()
}
fn f(x: f64) -> Finite {
    Finite::new(x).unwrap()
}
fn grid(d: [usize; 3], cell: impl Fn([usize; 3]) -> Cell) -> Volume {
    let n = d.iter().product();
    let mut g = Volume {
        encoding: "observer-cubical-volume/1".into(),
        dimensions: d,
        origin_m: [0.; 3],
        pitch_m: 0.001,
        cells: vec![Cell::Outside; n],
        ports: vec![],
    };
    for i in 0..n {
        g.cells[i] = cell(g.point(i));
    }
    g
}
/// Independent mesh construction by exposed voxel faces, used only as an oracle.
fn surface(g: &Volume, phase: Cell) -> Mesh {
    let sides: [([isize; 3], [[usize; 3]; 4]); 6] = [
        ([-1, 0, 0], [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]]),
        ([1, 0, 0], [[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 0, 1]]),
        ([0, -1, 0], [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]]),
        ([0, 1, 0], [[0, 1, 0], [0, 1, 1], [1, 1, 1], [1, 1, 0]]),
        ([0, 0, -1], [[0, 0, 0], [0, 1, 0], [1, 1, 0], [1, 0, 0]]),
        ([0, 0, 1], [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]]),
    ];
    let mut ids = BTreeMap::<[usize; 3], usize>::new();
    let mut vertices = vec![];
    let mut triangles = vec![];
    for (i, &c) in g.cells.iter().enumerate() {
        if c == phase {
            let p = g.point(i);
            for (dir, quad) in sides {
                let q: Vec<_> = (0..3).map(|a| p[a] as isize + dir[a]).collect();
                let interior = (0..3).all(|a| q[a] >= 0 && q[a] < g.dimensions[a] as isize);
                if interior
                    && g.cells[g.index([q[0] as usize, q[1] as usize, q[2] as usize])] == phase
                {
                    continue;
                }
                let face = quad.map(|v| {
                    let key = std::array::from_fn(|a| p[a] + v[a]);
                    *ids.entry(key).or_insert_with(|| {
                        let id = vertices.len();
                        vertices.push(std::array::from_fn(|a| {
                            g.origin_m[a] + key[a] as f64 * g.pitch_m
                        }));
                        id
                    })
                });
                triangles.push([face[0], face[1], face[2]]);
                triangles.push([face[0], face[2], face[3]]);
            }
        }
    }
    Mesh {
        vertices_m: vertices,
        triangles,
    }
}
fn box_mesh(size: [f64; 3]) -> Mesh {
    let mut g = grid([1; 3], |_| Cell::Solid);
    g.pitch_m = 1.;
    let mut mesh = surface(&g, Cell::Solid);
    for p in &mut mesh.vertices_m {
        for a in 0..3 {
            p[a] *= size[a];
        }
    }
    mesh
}
fn approve(
    registry: &mut Registry,
    requirements: &ContentRef,
    seq: &mut u64,
    namespace: &str,
    bytes: Vec<u8>,
) -> BlobDigest {
    let blob = BlobDigest::of_bytes(&bytes);
    let subject = blob.reference(namespace);
    registry.insert_artifact(subject.clone(), bytes).unwrap();
    *seq += 1;
    let event = ReviewEvent {
        sequence: *seq,
        subject,
        action: ReviewAction::Approved,
        reviewer: "synthetic-fixture-authority".into(),
        requirements: vec![requirements.clone()],
    };
    let bytes = serde_json::to_vec(&event).unwrap();
    registry
        .append_approved_review(
            "synthetic-fixture-authority",
            std::slice::from_ref(requirements),
            ContentRef::from_bytes(&format!("synthetic-review-{seq}"), "1", &bytes),
            bytes,
        )
        .unwrap();
    blob
}
fn subject(
    surface_bytes: Vec<u8>,
    v: Option<&Volume>,
    error: f64,
) -> (VerifiedSubject, BTreeMap<BlobDigest, Vec<u8>>) {
    let mut registry = Registry::default();
    let requirements = ContentRef::from_bytes("synthetic-observer-requirements", "1", b"synthetic");
    let mut seq = 0;
    let frame = CoordinateFrame::RightHandedSiZDownstream;
    let (phase, centre) = if let Some(g) = v {
        let i = g.ports[0].cells[0];
        (g.cells[i], g.centre(i))
    } else {
        (Cell::Fluid, [0.; 3])
    };
    let ports = vec![Port {
        tag: 7,
        purpose: if phase == Cell::Solid {
            PortPurpose::Support
        } else {
            PortPurpose::Inlet
        },
        frame,
        centre_m: centre.map(m),
        area_m2: f(1.),
        outward_normal: [f(1.), f(0.), f(0.)],
    }];
    let boundary = b"synthetic-frozen-region".to_vec();
    let frozen = vec![FrozenRegion {
        tag: 9,
        boundary: BlobDigest::of_bytes(&boundary),
    }];
    let interface = InterfaceContract {
        ports: ports.clone(),
        frozen_regions: frozen.clone(),
        spatial_tolerance_m: m(error.max(1e-8)),
    };
    let interface_id = approve(
        &mut registry,
        &requirements,
        &mut seq,
        "v5.interface",
        serde_json::to_vec(&interface).unwrap(),
    );
    let mut artifacts = Vec::new();
    let mut blobs = BTreeMap::from([(BlobDigest::of_bytes(&boundary), boundary)]);
    for (role, format, bytes) in [
        (
            ArtifactRole::Geometry,
            ArtifactFormat::TriangleSurfaceJson,
            surface_bytes.clone(),
        ),
        (
            ArtifactRole::SurfaceMesh,
            ArtifactFormat::TriangleSurfaceJson,
            surface_bytes,
        ),
    ]
    .into_iter()
    .chain(v.map(|v| {
        (
            ArtifactRole::VolumeMesh,
            ArtifactFormat::NativeVolumeMesh,
            serde_json::to_vec(v).unwrap(),
        )
    })) {
        let digest = BlobDigest::of_bytes(&bytes);
        let acceptance = ArtifactAcceptance {
            artifact: digest.clone(),
            format,
            length_unit: LengthUnit::Metre,
            approximation_bound_m: m(error),
            interface_contract: interface_id.clone(),
        };
        let acceptance = approve(
            &mut registry,
            &requirements,
            &mut seq,
            "v5.artifact-acceptance",
            serde_json::to_vec(&acceptance).unwrap(),
        );
        artifacts.push(ArtifactRef {
            role,
            format,
            blob: digest.clone(),
            bytes: bytes.len() as u64,
            length_unit: LengthUnit::Metre,
            approximation_bound_m: m(error),
            acceptance,
        });
        blobs.insert(digest, bytes);
    }
    let bundle = SubjectBundle {
        schema_version: 1,
        artifacts,
        transform: FrameTransform {
            from: frame,
            to: frame,
            translation_m: [m(0.); 3],
            rotation: [
                [f(1.), f(0.), f(0.)],
                [f(0.), f(1.), f(0.)],
                [f(0.), f(0.), f(1.)],
            ],
        },
        material: MaterialState::Unknown,
        roughness: RoughnessState::Unknown,
        interface_contract: interface_id,
        ports,
        frozen_regions: frozen,
    };
    let verified = bundle
        .verify_artifacts(&registry, &requirements, |id| {
            blobs
                .get(id)
                .cloned()
                .ok_or(crucible_contract::ContractError::ArtifactMismatch)
        })
        .unwrap();
    (verified, blobs)
}
fn bound(g: &Volume, error: f64) -> BoundVolume {
    let mut v = g.clone();
    let seed = v
        .cells
        .iter()
        .position(|&c| matches!(c, Cell::Solid | Cell::Fluid))
        .unwrap();
    v.ports = vec![PortSeeds {
        tag: 7,
        cells: vec![seed],
    }];
    let (s, blobs) = subject(
        serde_json::to_vec(&box_mesh([1.; 3])).unwrap(),
        Some(&v),
        error,
    );
    let a = s
        .bundle()
        .artifacts
        .iter()
        .find(|a| a.role == ArtifactRole::VolumeMesh)
        .unwrap();
    observe::read_volume(&s, s.identity(), &a.blob, &blobs[&a.blob]).unwrap()
}

#[test]
fn exact_byte_subject_binding_and_mutation_of_the_mesh_input() {
    let bytes = serde_json::to_vec(&box_mesh([1.; 3])).unwrap();
    let (s, blobs) = subject(bytes.clone(), None, 1e-7);
    let digest = BlobDigest::of_bytes(&bytes);
    let read = observe::read_surface(&s, s.identity(), &digest, &blobs[&digest]).unwrap();
    near(read.measure().unwrap().volume_m3, 1., 1e-12);
    assert_eq!(read.provenance().artifact, digest);
    let different = serde_json::to_vec(&box_mesh([2., 1., 1.])).unwrap();
    assert!(matches!(
        observe::read_surface(&s, s.identity(), &digest, &different),
        Err(ObserveError::Binding)
    ));
    assert!(matches!(
        observe::read_surface(
            &s,
            s.identity(),
            &BlobDigest::of_bytes(&different),
            &different
        ),
        Err(ObserveError::Binding)
    ));
    let wrong = SemanticId::of(&"another subject").unwrap();
    assert!(matches!(
        observe::read_surface(&s, &wrong, &digest, &bytes),
        Err(ObserveError::Binding)
    ));
    let (new, _) = subject(different.clone(), None, 1e-7);
    let changed = observe::read_surface(
        &new,
        new.identity(),
        &BlobDigest::of_bytes(&different),
        &different,
    )
    .unwrap();
    near(changed.measure().unwrap().volume_m3, 2., 1e-12);
    let faces_at = |value: &observe::BoundSurface, x: f64| {
        value
            .mesh()
            .triangles
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                t.iter()
                    .all(|&v| value.mesh().vertices_m[v][0] == x)
                    .then_some(i)
            })
            .collect::<Vec<_>>()
    };
    let x0 = faces_at(&read, 0.);
    let x1 = faces_at(&read, 1.);
    let gap = read.patch_gap(&x0, &x1).unwrap();
    assert!(gap.lower_m < 1. && gap.upper_m > 1.);
    near(gap.input_error_m, 1e-7, 0.);
    assert!(read.patch_gap(&x0, &x0).is_err());
    let detects_mutation = |observer: &dyn Fn(&Mesh) -> f64| {
        (observer(read.mesh()) - observer(changed.mesh())).abs() > 0.5
    };
    assert!(detects_mutation(&|m| mesh::measure(m).unwrap().volume_m3));
    assert!(!detects_mutation(&|_| 1.)); // a deliberately mesh-ignoring observer fails this oracle
}

#[test]
fn strict_wire_rejects_genotype_side_channels_and_malformed_meshes() {
    for bytes in [
        br#"{"vertices_m":[],"triangles":[],"genotype":{}}"#.to_vec(),
        br#"{"vertices_m":[],"vertices_m":[],"triangles":[]}"#.to_vec(),
        br#"{"vertices_m":[[NaN,0,0]],"triangles":[]}"#.to_vec(),
    ] {
        let (s, _) = subject(bytes.clone(), None, 1e-7);
        assert!(
            observe::read_surface(&s, s.identity(), &BlobDigest::of_bytes(&bytes), &bytes).is_err()
        );
    }
    let base = box_mesh([1.; 3]);
    let mut open = base.clone();
    open.triangles.pop();
    assert!(mesh::measure(&open).is_err());
    let mut backwards = base.clone();
    for t in &mut backwards.triangles {
        t.swap(1, 2);
    }
    assert!(mesh::measure(&backwards).is_err());
    let mut duplicate = base.clone();
    duplicate.triangles.push(base.triangles[0]);
    assert!(mesh::measure(&duplicate).is_err());
    let mut nonfinite = base;
    nonfinite.vertices_m[0][0] = f64::INFINITY;
    assert!(mesh::measure(&nonfinite).is_err());
}

#[test]
fn volume_binding_observes_only_the_changed_partition_bytes() {
    let a = grid([3; 3], |p| {
        if p == [1; 3] {
            Cell::Solid
        } else {
            Cell::Fluid
        }
    });
    let b = grid(
        [3; 3],
        |p| if p[0] == 1 { Cell::Solid } else { Cell::Fluid },
    );
    let bound_a = bound(&a, 1e-8);
    let bound_b = bound(&b, 1e-8);
    assert_ne!(bound_a.provenance().artifact, bound_b.provenance().artifact);
    near(
        volume::measure(bound_b.volume()).unwrap().solid_volume_m3
            / volume::measure(bound_a.volume()).unwrap().solid_volume_m3,
        9.,
        1e-12,
    );
    let mut with_port = a.clone();
    with_port.ports = vec![PortSeeds {
        tag: 7,
        cells: vec![0],
    }];
    let (s, blobs) = subject(
        serde_json::to_vec(&box_mesh([1.; 3])).unwrap(),
        Some(&with_port),
        1e-8,
    );
    let artifact = s.bundle().artifacts.last().unwrap();
    let mut bytes = blobs[&artifact.blob].clone();
    bytes.push(b' ');
    assert!(matches!(
        observe::read_volume(&s, s.identity(), &artifact.blob, &bytes),
        Err(ObserveError::Binding)
    ));
    with_port.encoding = "other-native-volume".into();
    let unsupported = serde_json::to_vec(&with_port).unwrap();
    let (s, _) = subject(
        serde_json::to_vec(&box_mesh([1.; 3])).unwrap(),
        Some(&with_port),
        1e-8,
    );
    assert!(matches!(
        observe::read_volume(
            &s,
            s.identity(),
            &BlobDigest::of_bytes(&unsupported),
            &unsupported
        ),
        Err(ObserveError::Unsupported(_))
    ));
}

#[test]
fn two_volume_complexes_and_independent_boundary_conventions() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/v5-geometry");
    let oracles: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("oracles.json")).unwrap()).unwrap();
    for case in oracles["fixtures"].as_array().unwrap() {
        let g: Volume = crucible_contract::strict_decode(
            &std::fs::read(root.join(case["file"].as_str().unwrap())).unwrap(),
        )
        .unwrap();
        let t = topology::volume(&g, Cell::Solid).unwrap();
        let expected: Vec<usize> = serde_json::from_value(case["volume_betti"].clone()).unwrap();
        assert_eq!(t.cubical.0.as_slice(), expected);
        assert!(t.agrees());
        assert_ne!(t.cubical_cells, t.tetrahedral_cells);
        let boundary = topology::boundary(&surface(&g, Cell::Solid)).unwrap();
        let expected: Vec<usize> = serde_json::from_value(case["boundary_betti"].clone()).unwrap();
        assert_eq!(boundary.0.as_slice(), expected);
        if case["file"] == "ball.json" {
            assert_eq!(t.cubical.0, [1, 0, 0]);
            assert_eq!(boundary.0, [1, 0, 1]);
            assert_ne!(t.cubical.0, boundary.0);
        }
    }
}

#[test]
fn all_two_by_two_closed_cube_unions_are_contractible_or_empty() {
    // Every nonempty subset of these eight cubes contains the common central
    // vertex and is star-shaped about it. This oracle needs no rank algorithm.
    for mask in 0_u16..256 {
        let mut g = grid([2; 3], |_| Cell::Outside);
        for (i, c) in g.cells.iter_mut().enumerate() {
            if mask & (1 << i) != 0 {
                *c = Cell::Solid;
            }
        }
        let observed = topology::volume(&g, Cell::Solid).unwrap();
        assert_eq!(
            observed.cubical.0,
            if mask == 0 { [0, 0, 0] } else { [1, 0, 0] },
            "mask {mask}"
        );
        assert!(observed.agrees());
    }
}

#[test]
fn port_reachability_solid_attachment_and_point_contact_are_different() {
    let g = grid([5, 3, 3], |p| {
        if p[1] == 1 && p[2] == 1 && p[0] != 3 {
            Cell::Fluid
        } else {
            Cell::Solid
        }
    });
    let seeds = [
        PortSeeds {
            tag: 14,
            cells: vec![g.index([0, 1, 1])],
        },
        PortSeeds {
            tag: 62,
            cells: vec![g.index([2, 1, 1])],
        },
    ];
    let c = volume::connectivity(&g, Cell::Fluid, &seeds).unwrap();
    assert_eq!(c.face_components, 2);
    assert_eq!(c.unseeded_components, 1);
    assert_eq!(c.unreachable_cells, vec![g.index([4, 1, 1])]);
    assert!(c.port_reachability[0][1]);
    let solids = grid([5, 3, 3], |p| {
        if p[1] == 1 && p[2] == 1 && p[0] != 3 {
            Cell::Solid
        } else {
            Cell::Outside
        }
    });
    assert_eq!(
        volume::connectivity(&solids, Cell::Solid, &seeds)
            .unwrap()
            .unseeded_components,
        1
    );
    assert!(volume::connectivity(&g, Cell::Fluid, &[]).is_err());
    let point = grid([2; 3], |p| {
        if p == [0; 3] || p == [1; 3] {
            Cell::Solid
        } else {
            Cell::Outside
        }
    });
    assert_eq!(
        topology::volume(&point, Cell::Solid).unwrap().cubical.0,
        [1, 0, 0]
    );
    assert_eq!(
        volume::connectivity(
            &point,
            Cell::Solid,
            &[PortSeeds {
                tag: 1,
                cells: vec![0]
            }]
        )
        .unwrap()
        .face_components,
        2
    );
    assert!(mesh::validate(&surface(&point, Cell::Solid)).is_err());
    let mut uncertain = g.clone();
    uncertain.cells[0] = Cell::Unresolved;
    assert!(
        !volume::connectivity(&uncertain, Cell::Fluid, &seeds)
            .unwrap()
            .resolved
    );
    assert!(matches!(
        topology::volume(&uncertain, Cell::Fluid),
        Err(ObserveError::Unsupported(_))
    ));
}

fn pocket(throat: usize) -> Volume {
    grid([13, 9, 9], |[x, y, z]| {
        let room = (1..=11).contains(&x) && x != 6 && (1..=7).contains(&y) && (1..=7).contains(&z);
        let passage = x == 6 && y.abs_diff(4) <= throat && z.abs_diff(4) <= throat;
        if room || passage {
            Cell::Fluid
        } else {
            Cell::Solid
        }
    })
}
fn profile(d: f64) -> ManufacturingProfile {
    ManufacturingProfile {
        profile_sha256: BlobDigest::of_bytes(b"synthetic manufacturing profile"),
        escape_diameter_m: d,
    }
}

#[test]
fn connected_powder_pocket_fails_clearance_and_wider_throat_has_a_path() {
    let tight = pocket(0);
    let seed = tight.index([3, 4, 4]);
    let target = tight.index([9, 4, 4]);
    let conn = volume::connectivity(
        &tight,
        Cell::Fluid,
        &[PortSeeds {
            tag: 1,
            cells: vec![seed],
        }],
    )
    .unwrap();
    assert_eq!(conn.face_components, 1);
    let blocked = clearance::escape(&tight, &profile(0.0012), &[seed], &[target], 1e-8).unwrap();
    assert_eq!(blocked.gate, ClearanceGate::Inaccessible);
    assert_eq!(blocked.isolated_void_components, 0);
    assert!(blocked.inaccessible_site_volume_m3 > 0.);
    assert!(blocked.paths[0].bottleneck_lower_m < 0.0012);
    let wide = pocket(2);
    let passed = clearance::escape(&wide, &profile(0.0012), &[seed], &[target], 1e-8).unwrap();
    assert_eq!(passed.gate, ClearanceGate::Accessible);
    let path = &passed.paths[0].path_cells;
    assert_eq!(path[0], target);
    assert_eq!(*path.last().unwrap(), seed);
    for pair in path.windows(2) {
        let a = wide.point(pair[0]);
        let b = wide.point(pair[1]);
        assert_eq!((0..3).map(|i| a[i].abs_diff(b[i])).sum::<usize>(), 1);
    }
    assert!(passed.paths[0].bottleneck_lower_m >= 0.0012);
    assert!(clearance::escape(&wide, &profile(0.), &[seed], &[target], 0.).is_err());
}

#[test]
fn isolated_void_and_subvoxel_throat_remain_failed_or_unresolved() {
    let mut g = pocket(0);
    let seed = g.index([3, 4, 4]);
    let target = g.index([9, 4, 4]);
    let throat = g.index([6, 4, 4]);
    g.cells[throat] = Cell::Solid;
    let closed = clearance::escape(&g, &profile(0.0012), &[seed], &[target], 1e-8).unwrap();
    assert_eq!(closed.isolated_void_components, 1);
    assert_eq!(closed.gate, ClearanceGate::Inaccessible);
    g.cells[throat] = Cell::Unresolved;
    let unknown = clearance::escape(&g, &profile(0.0012), &[seed], &[target], 1e-8).unwrap();
    assert_eq!(unknown.gate, ClearanceGate::Unresolved);
    assert_eq!(unknown.unresolved_cells, vec![throat]);
    assert_eq!(unknown.paths[0].gate, ClearanceGate::Unresolved);
}

fn square(z: f64) -> Vec<[[f64; 3]; 3]> {
    vec![
        [[0., 0., z], [2., 0., z], [2., 3., z]],
        [[0., 0., z], [2., 3., z], [0., 3., z]],
    ]
}
fn rotate(p: [f64; 3]) -> [f64; 3] {
    let (s, c) = 0.731_f64.sin_cos();
    [
        c * p[0] - s * p[2] + 0.043,
        p[1] + 0.017,
        s * p[0] + c * p[2] - 0.021,
    ]
}

#[test]
fn complete_patch_distances_rays_rotation_and_narrow_breakthrough() {
    let a = square(0.);
    let b = square(0.003);
    let gap = distance::patch_gap(&a, &b, 1e-7).unwrap();
    assert!(gap.lower_m <= 0.003 && gap.upper_m >= 0.003);
    near(gap.independent_ray_min_m.unwrap(), 0.003, 1e-12);
    let rotated_a: Vec<_> = a.iter().map(|t| t.map(rotate)).collect();
    let rotated_b: Vec<_> = b.iter().map(|t| t.map(rotate)).collect();
    let rotated = distance::patch_gap(&rotated_a, &rotated_b, 1e-7).unwrap();
    near(gap.lower_m, rotated.lower_m, 1e-12);
    near(rotated.independent_ray_min_m.unwrap(), 0.003, 1e-12);
    let mut narrow = b.clone();
    narrow.push([
        [0.33, 0.41, 0.00005],
        [0.34, 0.41, 0.00005],
        [0.33, 0.42, 0.00005],
    ]);
    let narrow = distance::patch_gap(&a, &narrow, 1e-7).unwrap();
    assert!(narrow.estimate_m < 0.000051);
    assert!(narrow.lower_m < 0.000051); // it cannot pass a minimum-wall gate
    let crossing = [[[0.2, 0.2, -0.001], [0.4, 0.2, 0.001], [0.3, 0.4, 0.001]]];
    let pierced = distance::patch_gap(&a, &crossing, 0.).unwrap();
    assert!(pierced.estimate_m < 1e-12);
    assert_eq!(pierced.lower_m, 0.);
    assert!(distance::patch_gap(&[], &b, 0.).is_err());
}

#[test]
fn analytic_tube_wall_error_is_bounded_by_the_actual_facets() {
    let n = 128;
    let mut inner = vec![];
    let mut outer = vec![];
    for k in 0..n {
        let a = std::f64::consts::TAU * k as f64 / n as f64;
        let b = std::f64::consts::TAU * (k + 1) as f64 / n as f64;
        for (r, patch) in [(0.01, &mut inner), (0.013, &mut outer)] {
            let q = [
                [r * a.cos(), r * a.sin(), 0.],
                [r * b.cos(), r * b.sin(), 0.],
                [r * b.cos(), r * b.sin(), 0.02],
                [r * a.cos(), r * a.sin(), 0.02],
            ];
            patch.push([q[0], q[1], q[2]]);
            patch.push([q[0], q[2], q[3]]);
        }
    }
    let e = 0.013 * (1. - (std::f64::consts::PI / n as f64).cos());
    let result = distance::patch_gap(&inner, &outer, e).unwrap();
    assert!(result.lower_m <= 0.003 && result.upper_m >= 0.003);
    assert!(result.upper_m - result.lower_m < 0.00003);
    near(
        result.independent_ray_min_m.unwrap(),
        0.003 * (std::f64::consts::PI / n as f64).cos(),
        1e-12,
    );
}

#[test]
fn asymmetric_integrals_moments_and_semantic_port_planes_survive_rotation() {
    let b = box_mesh([2., 3., 4.]);
    let result = mesh::measure(&b).unwrap();
    near(result.volume_m3, 24., 1e-12);
    near(result.area_m2, 52., 1e-12);
    assert_eq!(result.centroid_m, [1., 1.5, 2.]);
    for a in 0..3 {
        near(
            result.second_moment_about_vertex0_m5[a][a],
            24. * [4., 9., 16.][a] / 3.,
            1e-12,
        );
    }
    let mut rotated = b.clone();
    for p in &mut rotated.vertices_m {
        *p = rotate(*p);
    }
    let r = mesh::measure(&rotated).unwrap();
    near(r.volume_m3, 24., 1e-11);
    near(r.area_m2, 52., 1e-11);
    for (a, b) in r.centroid_m.into_iter().zip(rotate(result.centroid_m)) {
        near(a, b, 1e-12);
    }
    let (s, c) = 0.731_f64.sin_cos();
    let port = mesh::port_plane(
        &rotated,
        mesh::Plane {
            centre_m: rotate([2., 1.5, 2.]),
            normal: [c, 0., s],
        },
        1e-12,
    )
    .unwrap();
    near(port.0, 12., 1e-11);
    for (a, b) in port.1.into_iter().zip(rotate([2., 1.5, 2.])) {
        near(a, b, 1e-12);
    }
}

#[test]
fn frozen_region_difference_and_non_star_cavity_are_measured_from_cells() {
    let a = grid([5; 3], |p| {
        if p == [2; 3] {
            Cell::Fluid
        } else {
            Cell::Solid
        }
    });
    let mut b = a.clone();
    let hole = b.index([2, 2, 1]);
    b.cells[hole] = Cell::Fluid;
    let frozen = volume::frozen_difference(&a, &b, &[hole]).unwrap();
    near(frozen.changed_volume_m3, 1e-9, 1e-20);
    assert_eq!(
        topology::volume(&a, Cell::Solid).unwrap().cubical.0,
        [1, 0, 1]
    );
    // A deliberately wrong star/column-envelope proxy fills the enclosed cell.
    // It has the same outer envelope, yet fails the independent volume oracle.
    let mut radial_proxy = a.clone();
    radial_proxy.cells.fill(Cell::Solid);
    assert_ne!(
        volume::measure(&a).unwrap().solid_volume_m3,
        volume::measure(&radial_proxy).unwrap().solid_volume_m3
    );
    assert_ne!(
        topology::volume(&a, Cell::Solid).unwrap().cubical,
        topology::volume(&radial_proxy, Cell::Solid)
            .unwrap()
            .cubical
    );
    let other = a.index([0, 0, 0]);
    near(
        volume::frozen_difference(&a, &b, &[other])
            .unwrap()
            .changed_volume_m3,
        0.,
        0.,
    );
    let mut breakthrough = b.clone();
    let outer_hole = breakthrough.index([2, 2, 0]);
    breakthrough.cells[outer_hole] = Cell::Fluid;
    assert_eq!(
        topology::volume(&breakthrough, Cell::Solid)
            .unwrap()
            .cubical
            .0,
        [1, 0, 0]
    );
    b.cells[hole] = Cell::Unresolved;
    assert!(
        volume::frozen_difference(&a, &b, &[hole])
            .unwrap()
            .unresolved_volume_m3
            > 0.
    );
}

fn probe(name: &str) -> ProbeStep {
    ProbeStep {
        parameter: name.into(),
        unit: "dimensionless".into(),
        step: 0.2,
        parameter_resolution: 0.001,
    }
}
#[test]
fn active_perturbations_distinguish_neutral_unsupported_and_failed_development() {
    let g = grid([5; 3], |p| if p[0] < 2 { Cell::Solid } else { Cell::Fluid });
    let baseline = bound(&g, 1e-8);
    let mut calls = vec![];
    let report = enactment::probe_active(
        &baseline,
        &[
            probe("radial_inner_fraction"),
            probe("width"),
            probe("unavailable"),
            probe("broken"),
        ],
        |name, step| {
            calls.push((name.to_owned(), step));
            match name {
                "radial_inner_fraction" => Ok(bound(&g, 1e-8)),
                "width" => {
                    let v = grid([5; 3], |p| {
                        if p[0] < if step > 0. { 3 } else { 1 } {
                            Cell::Solid
                        } else {
                            Cell::Fluid
                        }
                    });
                    Ok(bound(&v, 1e-8))
                }
                "unavailable" => Err(DevelopmentFailure::Unsupported),
                _ => Err(DevelopmentFailure::Failed),
            }
        },
    )
    .unwrap();
    assert_eq!(calls.len(), 8);
    for pair in calls.chunks_exact(2) {
        assert!(pair[0].1 < 0. && pair[1].1 > 0.);
    }
    assert_eq!(
        report.iter().map(|r| r.state).collect::<Vec<_>>(),
        [
            Enactment::Neutral,
            Enactment::Changed,
            Enactment::Unsupported,
            Enactment::FailedDevelopment
        ]
    );
    let id = enactment::physical_id(&baseline).unwrap();
    assert_eq!(report[0].physical_ids, [Some(id.clone()), Some(id)]);
    assert_eq!(report[0].solid_volume_delta_m3, [Some(0.), Some(0.)]);
    assert!(report[1].resolved_witness_cells.iter().all(|&n| n > 0));
    let mut too_small = probe("tiny");
    too_small.step = too_small.parameter_resolution;
    assert!(
        enactment::probe_active(&baseline, &[too_small], |_, _| panic!(
            "underresolved probe must not develop"
        ))
        .is_err()
    );
    let wide_error = bound(&g, 0.001);
    let r = enactment::probe_active(&wide_error, &[probe("width")], |_, _| {
        Ok(bound(
            &grid([5; 3], |p| if p[0] < 3 { Cell::Solid } else { Cell::Fluid }),
            0.001,
        ))
    })
    .unwrap();
    assert_eq!(r[0].state, Enactment::Unsupported);
}

#[test]
fn tpms_mass_is_resolved_in_the_same_solid_and_no_op_gets_no_credit() {
    let gyroid = |threshold: f64| {
        grid([9; 3], |[x, y, z]| {
            let k = std::f64::consts::TAU / 8.;
            let (x, y, z) = (k * x as f64, k * y as f64, k * z as f64);
            if (x.sin() * y.cos() + y.sin() * z.cos() + z.sin() * x.cos()).abs() < threshold {
                Cell::Solid
            } else {
                Cell::Fluid
            }
        })
    };
    let before = bound(&gyroid(0.4), 1e-8);
    let after = bound(&gyroid(0.9), 1e-8);
    let density = BlobDigest::of_bytes(b"synthetic density 7800 kg/m3");
    let a = enactment::mass(&before, 7800., density.clone()).unwrap();
    let b = enactment::mass(&after, 7800., density.clone()).unwrap();
    let count = after
        .volume()
        .cells
        .iter()
        .filter(|&&c| c == Cell::Solid)
        .count();
    near(b.represented_mass_kg, count as f64 * 1e-9 * 7800., 1e-15);
    assert!(b.represented_mass_kg > a.represented_mass_kg);
    assert_eq!(b.provenance.artifact, after.provenance().artifact);
    let probes = enactment::probe_active(
        &before,
        &[probe("tpms_sheet"), probe("radial_inner_fraction")],
        |name, _| {
            Ok(bound(
                &gyroid(if name == "tpms_sheet" { 0.9 } else { 0.4 }),
                1e-8,
            ))
        },
    )
    .unwrap();
    assert_eq!(probes[0].state, Enactment::Changed);
    assert_eq!(probes[1].state, Enactment::Neutral);
    assert_eq!(
        enactment::mass(&bound(&gyroid(0.4), 1e-8), 7800., density)
            .unwrap()
            .represented_mass_kg,
        a.represented_mass_kg
    );
}

#[test]
fn numerical_and_representation_budgets_refuse_instead_of_fabricating_success() {
    let mut g = grid([2; 3], |_| Cell::Solid);
    g.pitch_m = f64::MIN_POSITIVE;
    assert!(g.validate().is_err());
    g.pitch_m = 0.001;
    g.origin_m = [1e50; 3];
    assert!(g.validate().is_err());
    let huge = grid([9; 3], |_| Cell::Solid);
    assert!(matches!(
        topology::volume(&huge, Cell::Solid),
        Err(ObserveError::Unsupported(_))
    ));
    let mut unresolved = grid([3; 3], |_| Cell::Solid);
    unresolved.cells[1] = Cell::Unresolved;
    assert!(enactment::mass(
        &bound(&unresolved, 1e-8),
        7800.,
        BlobDigest::of_bytes(b"density")
    )
    .is_err());
    let cube = grid([2; 3], |_| Cell::Solid);
    let measures = volume::measure(&cube).unwrap();
    near(measures.solid_volume_m3, 8e-9, 1e-20);
    for x in measures.solid_centroid_m.unwrap() {
        near(x, 0.001, 1e-16);
    }
    near(
        measures.solid_second_moment_m5[0][0],
        8e-9 * 0.002_f64.powi(2) / 3.,
        1e-25,
    );
}
