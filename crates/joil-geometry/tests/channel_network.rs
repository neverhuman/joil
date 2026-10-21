//! Analytic and synthetic geometry controls; no manufacturing or CFD admission.
use joil_geometry::channel_network::*;
use joil_geometry::csg;
use joil_geometry::partition::{Material, MaterialPartition};
use joil_geometry::{EvaluationBudget, Point3, ReferenceInterpreter};

fn value(field: &csg::BoundedField, p: [f64; 3]) -> f64 {
    ReferenceInterpreter::compile(field.program().clone())
        .unwrap()
        .evaluate_with_budget(
            Point3::metres(p).unwrap(),
            &mut EvaluationBudget::new(1_000_000),
        )
        .unwrap()
}

fn sphere(id: &str, centre_m: [f64; 3], radius_m: f64) -> Chamber {
    Chamber {
        id: id.into(),
        centre_m,
        geometry: ChamberShape::Sphere { radius_m },
    }
}

fn channel(id: &str, from: &str, to: &str) -> Channel {
    Channel {
        id: id.into(),
        from_chamber: from.into(),
        to_chamber: to.into(),
        via_m: vec![],
        radius_m: 0.003,
    }
}

fn diamond() -> ChannelNetwork {
    ChannelNetwork {
        schema_version: SCHEMA_VERSION,
        chambers: vec![
            sphere("feed", [-0.04, 0., 0.], 0.006),
            sphere("upper", [0., 0.02, 0.01], 0.007),
            sphere("lower", [0., -0.02, -0.01], 0.005),
            sphere("product", [0.04, 0., 0.], 0.006),
        ],
        channels: vec![
            channel("feed-upper", "feed", "upper"),
            channel("feed-lower", "feed", "lower"),
            channel("upper-product", "upper", "product"),
            channel("lower-product", "lower", "product"),
        ],
    }
}

#[test]
fn arbitrary_axis_capsule_matches_an_independent_segment_distance() {
    let start = [-0.03, 0.02, -0.01];
    let end = [0.04, -0.01, 0.06];
    let field = capsule(start, end, 0.004).unwrap();
    let d: [f64; 3] = std::array::from_fn(|i| end[i] - start[i]);
    let squared_length: f64 = d.iter().map(|x| x * x).sum();
    for p in [start, end, [0., 0., 0.], [0.2, -0.1, 0.3], [-0.1, 0., 0.]] {
        let t = (p
            .iter()
            .zip(start)
            .zip(d)
            .map(|((x, a), v)| (x - a) * v)
            .sum::<f64>()
            / squared_length)
            .clamp(0.0, 1.0);
        let expected = (0..3)
            .map(|i| (p[i] - start[i] - t * d[i]).powi(2))
            .sum::<f64>()
            .sqrt()
            - 0.004;
        assert!((value(&field, p) - expected).abs() < 1e-14);
    }
    assert!(field.is_exact_distance());
    assert_eq!(field.lipschitz_bound(), Some(1.0));
}

#[test]
fn capsule_rotation_translation_scale_and_endpoint_reversal() {
    let a = [-0.02, 0.01, 0.];
    let b = [0.03, -0.01, 0.04];
    let transform = |p: [f64; 3]| [-3.0 * p[1] + 0.1, 3.0 * p[0] - 0.2, 3.0 * p[2] + 0.05];
    let original = capsule(a, b, 0.003).unwrap();
    let transformed = capsule(transform(a), transform(b), 0.009).unwrap();
    let reversed = capsule(b, a, 0.003).unwrap();
    for p in [[0., 0., 0.], [0.01, -0.02, 0.03], a, b] {
        assert!((value(&transformed, transform(p)) - 3.0 * value(&original, p)).abs() < 1e-13);
        assert!((value(&reversed, p) - value(&original, p)).abs() < 1e-13);
    }
}

#[test]
fn branch_rejoin_has_real_channels_and_keeps_the_intervening_solid() {
    let network = diamond();
    let compiled = network.compile().unwrap();
    assert_eq!(compiled.segment_count, 4);
    for c in &network.channels {
        let start = network
            .chambers
            .iter()
            .find(|x| x.id == c.from_chamber)
            .unwrap()
            .centre_m;
        let end = network
            .chambers
            .iter()
            .find(|x| x.id == c.to_chamber)
            .unwrap()
            .centre_m;
        for k in 0..=10 {
            let t = k as f64 / 10.0;
            let p = std::array::from_fn(|i| start[i] + t * (end[i] - start[i]));
            assert!(value(&compiled.fluid, p) < 0.0);
        }
    }
    assert!(value(&compiled.fluid, [0., 0., 0.]) > 0.0);
    let domain = csg::cuboid([0.; 3], [0.06; 3]).unwrap();
    let partition = MaterialPartition::new(
        domain.program().clone(),
        compiled.fluid.program().clone(),
        vec![],
    )
    .unwrap();
    let mut budget = EvaluationBudget::new(1_000_000);
    assert_eq!(
        partition
            .ownership(Point3::metres([0., 0., 0.]).unwrap(), &mut budget)
            .unwrap()
            .material,
        Material::Solid
    );
    assert_eq!(
        partition
            .ownership(Point3::metres([0., 0.02, 0.01]).unwrap(), &mut budget)
            .unwrap()
            .material,
        Material::Fluid
    );
}

#[test]
fn unequal_series_chambers_and_out_of_plane_return_are_enacted() {
    let mut n = ChannelNetwork {
        schema_version: 1,
        chambers: vec![
            sphere("a", [0., 0., 0.], 0.01),
            sphere("b", [0., 0., 0.06], 0.02),
        ],
        channels: vec![channel("series", "a", "b")],
    };
    let before = n.compile().unwrap();
    assert!(value(&before.fluid, [0.015, 0., 0.06]) < 0.0);
    assert!(value(&before.fluid, [0.015, 0., 0.]) > 0.0);
    n.channels.push(Channel {
        id: "return".into(),
        from_chamber: "b".into(),
        to_chamber: "a".into(),
        via_m: vec![[0.04, 0.03, 0.06], [0.04, 0.03, 0.]],
        radius_m: 0.002,
    });
    let after = n.compile().unwrap();
    assert!(value(&before.fluid, [0.04, 0.03, 0.03]) > 0.0);
    assert!(value(&after.fluid, [0.04, 0.03, 0.03]) < 0.0);
    assert_ne!(
        before.fluid.program().identity().unwrap(),
        after.fluid.program().identity().unwrap()
    );
}

#[test]
fn labels_do_not_change_geometry_and_radius_does() {
    let n = diamond();
    let original = n.compile().unwrap();
    let mut renamed = n.clone();
    renamed.channels[0].id = "unfamiliar-name".into();
    let renamed = renamed.compile().unwrap();
    assert_ne!(original.recipe_sha256, renamed.recipe_sha256);
    assert_eq!(
        original.fluid.program().identity().unwrap(),
        renamed.fluid.program().identity().unwrap()
    );
    let mut larger = n;
    larger.channels[0].radius_m *= 1.5;
    assert_ne!(
        original.fluid.program().identity().unwrap(),
        larger
            .compile()
            .unwrap()
            .fluid
            .program()
            .identity()
            .unwrap()
    );
}

#[test]
fn malformed_and_over_budget_geometry_is_refused() {
    for radius in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(capsule([0.; 3], [0., 0., 1.], radius).is_err());
    }
    assert!(capsule([0.; 3], [0.; 3], 0.01).is_err());
    assert!(capsule([f64::MAX, 0., 0.], [-f64::MAX, 0., 0.], 0.01).is_err());
    let n = diamond();
    let mut missing = n.clone();
    missing.channels[0].to_chamber = "absent".into();
    assert!(missing.compile().is_err());
    let mut duplicate = n.clone();
    duplicate.chambers[1].id = "feed".into();
    assert!(duplicate.compile().is_err());
    let mut bad_point = n.clone();
    bad_point.channels[0].via_m = vec![[f64::INFINITY, 0., 0.]];
    assert!(bad_point.compile().is_err());
    let mut too_many = n;
    too_many.channels[0].via_m = vec![[0.02, 0.01, 0.]; 17];
    assert!(too_many.compile().is_err());
}

#[test]
fn self_loop_changes_the_actual_fluid_volume() {
    let n = ChannelNetwork {
        schema_version: 1,
        chambers: vec![sphere("loop-root", [0.; 3], 0.005)],
        channels: vec![Channel {
            id: "loop".into(),
            from_chamber: "loop-root".into(),
            to_chamber: "loop-root".into(),
            via_m: vec![[0.02, 0., 0.], [0.02, 0.02, 0.], [0., 0.02, 0.]],
            radius_m: 0.002,
        }],
    };
    let c = n.compile().unwrap();
    assert!(value(&c.fluid, [0.02, 0.01, 0.]) < 0.0);
    assert!(value(&c.fluid, [0.01, 0.01, 0.]) > 0.0);
}

#[test]
fn committed_example_compiles_to_the_same_field_as_the_analytic_fixture() {
    let n: ChannelNetwork = serde_json::from_str(include_str!(
        "../../../tests/fixtures/process-network/branch-rejoin-geometry.json"
    ))
    .unwrap();
    assert_eq!(
        n.compile().unwrap().fluid.program().identity().unwrap(),
        diamond()
            .compile()
            .unwrap()
            .fluid
            .program()
            .identity()
            .unwrap()
    );
}
