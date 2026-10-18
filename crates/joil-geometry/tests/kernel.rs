//! Synthetic analytic and artifact controls; no measured/physical qualification.
use joil_geometry::{
    certify::{self, ConformancePolicy, CubicalSubject},
    chamber_wall::{self, ct35, ChamberWallProposal, FourierMode},
    csg::{self, Blend, Boolean, BoundedField},
    interval::{self, Cell, Interval, Sign},
    partition::{self, FrozenRegion, Material, MaterialPartition, SemanticRole},
};
use joil_geometry::{
    Axis, EvaluationBudget, FieldProgram, GeometryError, ImplicitEval, Op, Point3,
    ReferenceInterpreter,
};
use std::collections::{BTreeMap, BTreeSet};
fn budget() -> EvaluationBudget {
    EvaluationBudget::new(20_000_000)
}
fn point(p: [f64; 3]) -> Point3 {
    Point3::metres(p).unwrap()
}
fn value(f: &BoundedField, p: [f64; 3]) -> f64 {
    ReferenceInterpreter::compile(f.program().clone())
        .unwrap()
        .evaluate(point(p))
        .unwrap()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-11, "{a} != {b}");
}
fn constant(x: f64) -> FieldProgram {
    FieldProgram::new(vec![Op::Constant(x)], 0).unwrap()
}

#[test]
fn normal_budget_counts_source_nodes_across_all_six_samples() {
    // Three operations per sample: x + 1 has the analytic normal [1, 0, 0].
    let p = FieldProgram::new(
        vec![Op::Coordinate(Axis::X), Op::Constant(1.0), Op::Add(0, 1)],
        2,
    )
    .unwrap();
    let mut field = ReferenceInterpreter::compile(p).unwrap();
    let mut short = EvaluationBudget::new(6);
    assert!(
        interval::estimate_normal(&mut field, point([0.0; 3]), 1e-3, 1e-9, &mut short).is_err()
    );
    assert_eq!(short.remaining(), 0);
    let mut exact = EvaluationBudget::new(18);
    let normal =
        interval::estimate_normal(&mut field, point([0.0; 3]), 1e-3, 1e-9, &mut exact).unwrap();
    assert_eq!(normal, [1.0, 0.0, 0.0]);
    assert_eq!(exact.remaining(), 0);
}

#[test]
fn analytic_primitive_values_match_hand_geometry() {
    let s = csg::sphere([1.0, 2.0, 3.0], 2.0).unwrap();
    near(value(&s, [1.0, 2.0, 3.0]), -2.0);
    near(value(&s, [4.0, 6.0, 3.0]), 3.0);
    let b = csg::cuboid([0.0; 3], [1.0, 2.0, 3.0]).unwrap();
    near(value(&b, [0.0; 3]), -1.0);
    near(value(&b, [4.0, 6.0, 3.0]), 5.0);
    let c = csg::cylinder([0.0; 3], 2.0, 3.0).unwrap();
    near(value(&c, [3.0, 0.0, 4.0]), 2.0f64.sqrt());
    near(value(&c, [0.0; 3]), -2.0);
    for f in [&s, &b, &c] {
        assert_eq!(f.lipschitz_bound(), Some(1.0));
        assert!(f.is_exact_distance());
    }
    assert!(csg::sphere([0.0; 3], 0.0).is_err());
    assert!(csg::cuboid([f64::NAN, 0.0, 0.0], [1.0; 3]).is_err());
    assert!(csg::cylinder([0.0; 3], 1.0, f64::INFINITY).is_err());
}
#[test]
fn exact_set_booleans_and_bounded_blends() {
    let a = csg::sphere([-0.5, 0.0, 0.0], 1.0).unwrap();
    let b = csg::sphere([0.5, 0.0, 0.0], 1.0).unwrap();
    for op in [Boolean::Union, Boolean::Intersection, Boolean::Difference] {
        let c = csg::boolean(&a, &b, op).unwrap();
        assert!(!c.is_exact_distance());
        for i in -20..=20 {
            for j in -10..=10 {
                let p = [i as f64 / 10.0, j as f64 / 10.0, 0.0];
                let in_a = (p[0] + 0.5).hypot(p[1]) < 1.0;
                let in_b = (p[0] - 0.5).hypot(p[1]) < 1.0;
                let expected = match op {
                    Boolean::Union => in_a || in_b,
                    Boolean::Intersection => in_a && in_b,
                    Boolean::Difference => in_a && !in_b,
                };
                if value(&a, p).abs() > 1e-10 && value(&b, p).abs() > 1e-10 {
                    assert_eq!(value(&c, p) < 0.0, expected);
                }
            }
        }
    }
    let exact = csg::boolean(&a, &b, Boolean::Union).unwrap();
    for kind in [Blend::Chamfer, Blend::Fillet] {
        let f = csg::blend_union(&a, &b, 0.2, kind).unwrap();
        for i in -30..30 {
            let p = [i as f64 / 10.0, 0.3, 0.0];
            let delta = value(&exact, p) - value(&f, p);
            assert!(delta >= -1e-12 && delta <= f.scalar_approximation_error() + 1e-12);
        }
        assert_eq!(f.lipschitz_bound(), Some(1.0));
        assert!(!f.is_exact_distance());
    }
    assert!(csg::blend_union(&a, &b, -1.0, Blend::Fillet).is_err());
}
#[test]
fn dag_guards_strict_wire_and_no_hidden_failure_truncation() {
    for (nodes, out) in [
        (vec![], 0),
        (vec![Op::Constant(1.0)], 1),
        (vec![Op::Add(0, 0)], 0),
        (vec![Op::Constant(f64::NAN)], 0),
        (vec![Op::Constant(1.0); 4097], 0),
    ] {
        assert!(FieldProgram::new(nodes, out).is_err());
    }
    assert!(Point3::metres([0.0, f64::INFINITY, 0.0]).is_err());
    for bytes in [
        br#"{"schema_version":1,"nodes":[{"op":"constant","args":1}],"output":0,"extra":1}"#
            .as_slice(),
        br#"{"schema_version":1,"schema_version":1,"nodes":[],"output":0}"#,
        br#"{"schema_version":1,"nodes":[{"op":"constant","args":1,"extra":1}],"output":0}"#,
    ] {
        assert!(FieldProgram::from_json(bytes).is_err());
    }
    let p = FieldProgram::new(
        vec![Op::Constant(1.0), Op::Constant(-1.0), Op::SquareRoot(1)],
        0,
    )
    .unwrap();
    let mut f = ReferenceInterpreter::compile(p).unwrap();
    assert!(matches!(
        f.evaluate(point([0.0; 3])),
        Err(GeometryError::NumericalFailure(_))
    ));
    let p = FieldProgram::new(vec![Op::Constant(f64::MAX), Op::Multiply(0, 0)], 1).unwrap();
    assert!(ReferenceInterpreter::compile(p)
        .unwrap()
        .evaluate(point([0.0; 3]))
        .is_err());
}
#[test]
fn structural_hashes_are_order_independent_and_budget_is_not_refilled() {
    let a = FieldProgram::new(
        vec![
            Op::Coordinate(Axis::X),
            Op::Constant(2.0),
            Op::Multiply(0, 1),
            Op::Coordinate(Axis::Y),
            Op::Add(2, 3),
        ],
        4,
    )
    .unwrap();
    let b = FieldProgram::new(
        vec![
            Op::Coordinate(Axis::Y),
            Op::Constant(2.0),
            Op::Coordinate(Axis::X),
            Op::Multiply(2, 1),
            Op::Add(3, 0),
        ],
        4,
    )
    .unwrap();
    assert_eq!(a.identity().unwrap(), b.identity().unwrap());
    assert_eq!(
        constant(0.0).identity().unwrap(),
        constant(-0.0).identity().unwrap()
    );
    let changed = FieldProgram::new(vec![Op::Constant(0.0), Op::Constant(2.0)], 0).unwrap();
    assert_ne!(
        changed.identity().unwrap(),
        constant(0.0).identity().unwrap()
    );
    let bytes = serde_json::to_vec(&a).unwrap();
    assert_eq!(a, FieldProgram::from_json(&bytes).unwrap());
    let mut f = ReferenceInterpreter::compile(a).unwrap();
    let mut gas = EvaluationBudget::new(9);
    near(
        f.evaluate_with_budget(point([1.0, 3.0, 0.0]), &mut gas)
            .unwrap(),
        5.0,
    );
    assert_eq!(gas.remaining(), 4);
    assert!(f.evaluate_with_budget(point([0.0; 3]), &mut gas).is_err());
    assert_eq!(gas.remaining(), 0);
    assert!(f.evaluate_with_budget(point([0.0; 3]), &mut gas).is_err());
}
#[test]
fn interval_and_affine_enclose_independent_samples_and_reject_unknown_domain() {
    let cell = Cell::new([-1.0, -2.0, -3.0], [1.0, 2.0, 3.0]).unwrap();
    let f = csg::sphere([0.0; 3], 0.5).unwrap();
    let bound = interval::evaluate(f.program(), cell, &mut budget()).unwrap();
    for x in [-1.0f64, 0.0, 0.3, 1.0] {
        for y in [-2.0, 0.0, 2.0] {
            for z in [-3.0, 0.0, 3.0] {
                assert!(bound.contains((x * x + y * y + z * z).sqrt() - 0.5));
            }
        }
    }
    let correlated = FieldProgram::new(
        vec![
            Op::Coordinate(Axis::X),
            Op::Constant(1.0),
            Op::Add(0, 1),
            Op::Subtract(2, 0),
        ],
        3,
    )
    .unwrap();
    let natural = interval::evaluate(&correlated, cell, &mut budget()).unwrap();
    let affine = interval::evaluate_affine(&correlated, cell, &mut budget()).unwrap();
    assert!(affine.contains(1.0));
    assert!(affine.width() < 1e-12);
    assert!(natural.width() > 3.0);
    let invalid = FieldProgram::new(vec![Op::Coordinate(Axis::X), Op::SquareRoot(0)], 1).unwrap();
    assert!(interval::evaluate(&invalid, cell, &mut budget()).is_err());
    assert!(Interval::new(1.0, -1.0).is_err());
    assert!(Interval::point(f64::MAX).unwrap().square().is_err());
    assert!(interval::evaluate(f.program(), cell, &mut EvaluationBudget::new(0)).is_err());
    let sine = FieldProgram::new(vec![Op::Coordinate(Axis::X), Op::Sine(0)], 1).unwrap();
    assert_eq!(
        interval::evaluate(&sine, cell, &mut budget()).unwrap(),
        Interval::new(-1.0, 1.0).unwrap()
    );
    assert_eq!(
        interval::classify(Interval::new(-1.0, 1.0).unwrap()),
        Sign::Unresolved
    );
}
#[test]
fn normal_estimates_refuse_degenerate_gradients_and_unresolvable_steps() {
    let f = csg::sphere([0.0; 3], 1.0).unwrap();
    let mut f = ReferenceInterpreter::compile(f.program().clone()).unwrap();
    let n = interval::estimate_normal(&mut f, point([2.0, 0.0, 0.0]), 1e-5, 1e-8, &mut budget())
        .unwrap();
    near(n[0], 1.0);
    near(n[1], 0.0);
    assert!(interval::estimate_normal(&mut f, point([0.0; 3]), 1e-5, 1e-8, &mut budget()).is_err());
    assert!(
        interval::estimate_normal(&mut f, point([1e30, 0.0, 0.0]), 1e-9, 1e-8, &mut budget())
            .is_err()
    );
    assert!(interval::estimate_normal(
        &mut f,
        point([2.0, 0.0, 0.0]),
        1e-5,
        1e-8,
        &mut EvaluationBudget::new(5)
    )
    .is_err());
}
fn partition_fixture() -> MaterialPartition {
    MaterialPartition::new(
        csg::cuboid([0.0; 3], [2.0; 3]).unwrap().program().clone(),
        csg::cuboid([0.0; 3], [1.0; 3]).unwrap().program().clone(),
        vec![],
    )
    .unwrap()
}
#[test]
fn shared_partition_rejects_independent_overlaps_gaps_and_frozen_drift() {
    let p = partition_fixture();
    for (x, labels, material) in [
        ([0.0; 3], [true, false, false], Material::Fluid),
        ([1.5, 0.0, 0.0], [false, true, false], Material::Solid),
        ([3.0, 0.0, 0.0], [false, false, true], Material::Void),
    ] {
        assert_eq!(
            p.ownership(point(x), &mut budget()).unwrap().material,
            material
        );
        p.verify_derived(point(x), labels, &mut budget()).unwrap();
    }
    for bad in [
        [true, true, false],
        [false, false, false],
        [false, true, false],
    ] {
        assert!(p
            .verify_derived(point([0.0; 3]), bad, &mut budget())
            .is_err());
    }
    assert_eq!(
        p.ownership(point([1.0, 0.0, 0.0]), &mut budget())
            .unwrap()
            .material,
        Material::Fluid
    );
    let frozen = FrozenRegion {
        id: "blade".into(),
        role: SemanticRole::FrozenSolid,
        material: Material::Solid,
        region: csg::sphere([0.0; 3], 0.1).unwrap().program().clone(),
    };
    let p = MaterialPartition::new(constant(-1.0), constant(-1.0), vec![frozen]).unwrap();
    assert!(p.ownership(point([0.0; 3]), &mut budget()).is_err());
    assert!(p
        .classify_cell(Cell::new([-0.01; 3], [0.01; 3]).unwrap(), &mut budget())
        .is_err());
}

#[test]
fn frozen_semantic_regions_share_one_owner_independently_of_module_names() {
    let regions = vec![
        (
            "frozen-fluid",
            SemanticRole::FrozenFluid,
            Material::Fluid,
            [0.0, 0.0, 0.0],
        ),
        (
            "inlet-port",
            SemanticRole::Port,
            Material::Fluid,
            [0.8, 0.0, 0.0],
        ),
        (
            "shell-wall",
            SemanticRole::Wall,
            Material::Solid,
            [1.5, 0.0, 0.0],
        ),
        (
            "keepout",
            SemanticRole::Keepout,
            Material::Void,
            [3.0, 0.0, 0.0],
        ),
    ];
    let frozen = regions
        .iter()
        .map(|(id, role, material, centre)| FrozenRegion {
            id: (*id).into(),
            role: role.clone(),
            material: *material,
            region: csg::sphere(*centre, 0.05).unwrap().program().clone(),
        })
        .collect();
    let p = MaterialPartition::new(
        csg::cuboid([0.0; 3], [2.0; 3]).unwrap().program().clone(),
        csg::cuboid([0.0; 3], [1.0; 3]).unwrap().program().clone(),
        frozen,
    )
    .unwrap();
    for (id, _, owner, centre) in regions {
        let result = p.ownership(point(centre), &mut budget()).unwrap();
        assert_eq!(result.material, owner);
        assert!(result.semantic_tags.contains(&id.to_string()));
        let lower = centre.map(|x| x - 0.01);
        let upper = centre.map(|x| x + 0.01);
        assert_eq!(
            p.classify_cell(Cell::new(lower, upper).unwrap(), &mut budget())
                .unwrap(),
            Some(owner)
        );
    }
}
#[test]
fn spatial_volume_bound_reports_unresolved_instead_of_grid_pitch_proof() {
    let p = partition_fixture();
    let full = Cell::new([-2.5; 3], [2.5; 3]).unwrap();
    let bound = partition::bound_fluid_volume(&p, full, 1, 0.001, &mut budget()).unwrap();
    assert!(bound.fluid_m3.contains(8.0));
    assert!(!bound.requested_error_met);
    assert!(bound.ambiguous_cells > 0);
    let interior = Cell::new([-0.5; 3], [0.5; 3]).unwrap();
    let bound = partition::bound_fluid_volume(&p, interior, 1, 0.001, &mut budget()).unwrap();
    assert!(bound.fluid_m3.contains(1.0));
    assert!(bound.requested_error_met);
    assert!(bound.fluid_m3.width() < 1e-12);
    assert!(
        partition::bound_fluid_volume(&p, full, 1, 0.001, &mut EvaluationBudget::new(0)).is_err()
    );
}
const GENOME: &[u8] = include_bytes!("../../../baselines/ct35/genome.json");
const REFERENCE: &[u8] = include_bytes!("../../../baselines/ct35/reference.json");
fn lower(p: &ChamberWallProposal) -> chamber_wall::LoweredWall {
    chamber_wall::lower(p, GENOME, REFERENCE, &ct35::frozen(), &mut budget()).unwrap()
}
#[test]
fn chamber_fourier_twist_matches_independent_polar_formula_and_freezes_interfaces() {
    let base = ChamberWallProposal::baseline();
    let baseline = lower(&base);
    let mut p = base.clone();
    p.modes.push(FourierMode {
        azimuthal_order: 3,
        cos_mm: 0.7,
        sin_mm: -0.4,
        twist_rad: -2.0,
    });
    let shaped = lower(&p);
    assert!(shaped.field.lipschitz_bound().is_none());
    assert!(!shaped.field.is_exact_distance());
    for s in [0.2, 0.5, 0.8] {
        for theta in [0.0f64, 0.7, 2.4, 5.0] {
            let phase = 3.0 * theta - 2.0 * s;
            let r = 0.035
                + (std::f64::consts::PI * s).sin().powi(2)
                    * (0.0007 * phase.cos() - 0.0004 * phase.sin());
            let z = 0.020 + 0.045 * s;
            near(
                value(&shaped.field, [r * theta.cos(), r * theta.sin(), z]),
                0.0,
            );
            assert!(
                value(
                    &shaped.field,
                    [(r - 1e-5) * theta.cos(), (r - 1e-5) * theta.sin(), z]
                ) < 0.0
            );
            assert!(
                value(
                    &shaped.field,
                    [(r + 1e-5) * theta.cos(), (r + 1e-5) * theta.sin(), z]
                ) > 0.0
            );
        }
    }
    for z in [
        -0.0228,
        0.0,
        0.015,
        0.070,
        0.152,
        0.16924743184330993,
        0.8,
        1.2,
        ct35::Z_END_M,
    ] {
        for r in [0.0, 0.00875, 0.0175, 0.035, 0.05] {
            assert_eq!(
                value(&baseline.field, [r, 0.0, z]),
                value(&shaped.field, [r, 0.0, z])
            );
        }
    }
    assert_eq!(shaped.frozen, ct35::frozen());
    assert_eq!([ct35::D3_M, ct35::L2_M, ct35::L3_M], [0.0175, 0.770, 0.700]);
    for terminal in ct35::terminals() {
        near(value(&baseline.field, terminal.centre_m), 0.0);
    }
    let mut zero = p;
    zero.modes[0].cos_mm = 0.0;
    zero.modes[0].sin_mm = 0.0;
    let zero = lower(&zero);
    assert_eq!(
        zero.field.program().identity().unwrap(),
        baseline.field.program().identity().unwrap()
    );
    assert_ne!(zero.proposal_sha256, baseline.proposal_sha256);
}
#[test]
fn chamber_rejects_wrong_tail_lengths_collar_sources_and_mode_budget() {
    let p = ChamberWallProposal::baseline();
    for (d, l2, l3, shift) in [
        (0.020, 0.770, 0.700, 0.0),
        (0.0175, 1.070, 0.700, 0.0),
        (0.0175, 0.770, 0.400, 0.0),
        (0.0175, 0.770, 0.700, 0.001),
    ] {
        let mut f = ct35::frozen();
        f.d3_m = d;
        f.l2_m = l2;
        f.l3_m = l3;
        f.collar_m = f.collar_m.map(|x| x + shift);
        assert!(chamber_wall::lower(&p, GENOME, REFERENCE, &f, &mut budget()).is_err());
    }
    let mut bytes = GENOME.to_vec();
    bytes.push(b' ');
    assert!(chamber_wall::lower(&p, &bytes, REFERENCE, &ct35::frozen(), &mut budget()).is_err());
    assert!(chamber_wall::lower(
        &p,
        GENOME,
        REFERENCE,
        &ct35::frozen(),
        &mut EvaluationBudget::new(0)
    )
    .is_err());
    for (order, cos, twist, n) in [
        (1, 0.1, 0.0, 1),
        (3, 2.0, 0.0, 1),
        (3, 0.1, 7.0, 1),
        (3, f64::NAN, 0.0, 1),
        (3, 0.1, 0.0, 9),
    ] {
        let mut p = p.clone();
        p.modes = vec![
            FourierMode {
                azimuthal_order: order,
                cos_mm: cos,
                sin_mm: 0.0,
                twist_rad: twist
            };
            n
        ];
        assert!(p.validate().is_err());
    }
}

fn artifact_fixture(protected: bool) -> (CubicalSubject, MaterialPartition) {
    // A 3x3x3 solid cube with a one-voxel attached protrusion. The filter may
    // remove the protrusion only when no reviewed feature witness protects it.
    let cube = csg::cuboid([2.5; 3], [1.5; 3]).unwrap();
    let nub = csg::cuboid([4.5, 2.5, 2.5], [0.5; 3]).unwrap();
    let solid = csg::boolean(&cube, &nub, Boolean::Union).unwrap();
    let fluid = csg::boolean(
        &csg::cuboid([3.0; 3], [3.0; 3]).unwrap(),
        &solid,
        Boolean::Difference,
    )
    .unwrap();
    let partition =
        MaterialPartition::new(constant(-1.0), fluid.program().clone(), vec![]).unwrap();
    let mut labels = vec![];
    for z in 0..6 {
        for y in 0..6 {
            for x in 0..6 {
                labels.push(
                    if (1..4).contains(&x) && (1..4).contains(&y) && (1..4).contains(&z)
                        || x == 4 && y == 2 && z == 2
                    {
                        Material::Solid
                    } else {
                        Material::Fluid
                    },
                );
            }
        }
    }
    let witnesses = if protected {
        BTreeMap::from([("blade-tip".into(), BTreeSet::from([4 + 6 * (2 + 6 * 2)]))])
    } else {
        BTreeMap::new()
    };
    let subject = CubicalSubject::new(
        [6; 3],
        [0.0; 3],
        1.0,
        labels,
        witnesses,
        &partition,
        &mut budget(),
    )
    .unwrap();
    (subject, partition)
}
fn policy() -> ConformancePolicy {
    ConformancePolicy {
        id: "synthetic-one-pass".into(),
        maximum_displacement_m: 2.0,
        maximum_changed_volume_m3: 1.01,
        required_neighbours: 5,
    }
}
#[test]
fn conformance_computes_changes_new_identity_and_refuses_vanished_feature_or_excess() {
    let (subject, _) = artifact_fixture(false);
    let (after, audit) = certify::conform(&subject, &policy(), &mut budget()).unwrap();
    assert_eq!(audit.changed_cells, 1);
    assert!(audit.changed_volume_m3[0] <= 1.0 && audit.changed_volume_m3[1] >= 1.0);
    assert!(audit.displacement_upper_bound_m > 1.0 && audit.displacement_upper_bound_m < 2.0);
    assert_ne!(
        audit.original_subject_sha256,
        audit.resulting_subject_sha256
    );
    assert_eq!(audit.resulting_subject_sha256, after.identity().unwrap());
    let (_, same) = certify::conform(&after, &policy(), &mut budget()).unwrap();
    assert_eq!(same.changed_cells, 0);
    assert_eq!(same.changed_volume_m3, [0.0; 2]);
    assert_eq!(same.original_subject_sha256, same.resulting_subject_sha256);
    let (protected, _) = artifact_fixture(true);
    assert!(certify::conform(&protected, &policy(), &mut budget()).is_err());
    let mut tiny = policy();
    tiny.maximum_displacement_m = 0.1;
    assert!(certify::conform(&subject, &tiny, &mut budget()).is_err());
    tiny = policy();
    tiny.maximum_changed_volume_m3 = 0.1;
    assert!(certify::conform(&subject, &tiny, &mut budget()).is_err());
    assert!(certify::conform(&subject, &policy(), &mut EvaluationBudget::new(0)).is_err());
}

#[test]
fn disappearing_material_component_and_stale_artifact_cannot_be_repaired() {
    let solid = csg::sphere([2.5; 3], 0.4).unwrap();
    let fluid = csg::boolean(
        &csg::cuboid([3.0; 3], [3.0; 3]).unwrap(),
        &solid,
        Boolean::Difference,
    )
    .unwrap();
    let partition =
        MaterialPartition::new(constant(-1.0), fluid.program().clone(), vec![]).unwrap();
    let mut labels = vec![Material::Fluid; 216];
    labels[2 + 6 * (2 + 6 * 2)] = Material::Solid;
    assert!(CubicalSubject::new(
        [6; 3],
        [0.0; 3],
        1.0,
        vec![Material::Fluid; 216],
        BTreeMap::new(),
        &partition,
        &mut budget()
    )
    .is_err());
    let subject = CubicalSubject::new(
        [6; 3],
        [0.0; 3],
        1.0,
        labels,
        BTreeMap::new(),
        &partition,
        &mut budget(),
    )
    .unwrap();
    assert!(matches!(
        certify::conform(&subject, &policy(), &mut budget()),
        Err(GeometryError::Unsupported(
            "conformance deletes/splits/merges a material component"
        ))
    ));
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn pinned_experimental_vm_jit_cover_baseline_branch_rejoin_lattice_and_thin_shell() {
    use joil_geometry::field::{candidate::FidgetKernel, compare_points};
    let baseline = lower(&ChamberWallProposal::baseline()).field;
    let a = csg::sphere([-0.01, 0.0, 0.0], 0.02).unwrap();
    let b = csg::sphere([0.01, 0.0, 0.0], 0.02).unwrap();
    let branch = csg::boolean(&a, &b, Boolean::Union).unwrap();
    let branch = csg::boolean(
        &branch,
        &csg::sphere([0.0; 3], 0.005).unwrap(),
        Boolean::Difference,
    )
    .unwrap();
    let mut lattice = csg::cylinder([0.0; 3], 0.001, 0.02).unwrap();
    for x in [-0.01, 0.01] {
        for y in [-0.01, 0.01] {
            lattice = csg::boolean(
                &lattice,
                &csg::cylinder([x, y, 0.0], 0.001, 0.02).unwrap(),
                Boolean::Union,
            )
            .unwrap();
        }
    }
    let thin = csg::boolean(
        &csg::sphere([0.0; 3], 0.0101).unwrap(),
        &csg::sphere([0.0; 3], 0.01).unwrap(),
        Boolean::Difference,
    )
    .unwrap();
    let mut points = vec![];
    for x in [-0.04, -0.01005, 0.0, 0.01005, 0.04] {
        for y in [-0.01, 0.0, 0.01] {
            for z in [-0.02, 0.0, 0.04, 0.16924743184330993, 1.0, ct35::Z_END_M] {
                points.push(point([x, y, z]));
            }
        }
    }
    for (name, field) in [
        ("baseline", baseline),
        ("branch-rejoin", branch),
        ("internal-lattice", lattice),
        ("thin-shell", thin),
    ] {
        let p = field.program().clone();
        let mut reference = ReferenceInterpreter::compile(p.clone()).unwrap();
        let start = std::time::Instant::now();
        let mut candidate = FidgetKernel::compile(p, 2e-6, &mut budget()).unwrap();
        let compile_ns = start.elapsed().as_nanos();
        assert!(candidate.identity().experimental);
        assert_eq!(candidate.identity().version, "0.5.1");
        let error = compare_points(&mut reference, &mut candidate, &points, 2e-6).unwrap();
        println!("candidate_case={name} compile_ns={compile_ns} backend_compile_ns={} points={} max_scalar_error={error:e} experimental=true global_error_certified=false",candidate.compilation_nanoseconds,points.len());
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn candidate_precision_collapse_dead_arithmetic_and_budget_fail_closed() {
    use joil_geometry::field::candidate::FidgetKernel;
    let p = FieldProgram::new(
        vec![
            Op::Coordinate(Axis::X),
            Op::Constant(1e-9),
            Op::Add(0, 1),
            Op::Subtract(2, 0),
        ],
        3,
    )
    .unwrap();
    let mut f = FidgetKernel::compile(p.clone(), 1e-12, &mut budget()).unwrap();
    assert!(f.evaluate(point([1.0, 0.0, 0.0])).is_err());
    assert!(FidgetKernel::compile(p, 1e-12, &mut EvaluationBudget::new(0)).is_err());
    assert!(FidgetKernel::compile(constant(1e-100), 1e-6, &mut budget()).is_err());
    let dead = FieldProgram::new(
        vec![Op::Constant(1.0), Op::Constant(-1.0), Op::SquareRoot(1)],
        0,
    )
    .unwrap();
    let mut dead = FidgetKernel::compile(dead, 1e-6, &mut budget()).unwrap();
    assert!(dead.evaluate(point([0.0; 3])).is_err());
    let mut x = FidgetKernel::compile(
        FieldProgram::new(vec![Op::Coordinate(Axis::X)], 0).unwrap(),
        1e-6,
        &mut budget(),
    )
    .unwrap();
    assert!(x.evaluate(point([1e100, 0.0, 0.0])).is_err());
}

#[test]
fn v5_08_02_partition_has_one_owner_including_interfaces_and_rejects_all_other_labels() {
    let partition = MaterialPartition::new(
        csg::sphere([0.0; 3], 2.0).unwrap().program().clone(),
        csg::sphere([0.0; 3], 1.0).unwrap().program().clone(),
        vec![],
    )
    .unwrap();
    for (radius, expected) in [(0.0, 0), (1.0, 0), (1.5, 1), (2.0, 1), (2.1, 2)] {
        for mask in 0..8 {
            let labels = std::array::from_fn(|i| mask & (1 << i) != 0);
            assert_eq!(
                partition
                    .verify_derived(point([radius, 0.0, 0.0]), labels, &mut budget())
                    .is_ok(),
                mask == 1 << expected,
                "radius={radius}, labels={labels:?}"
            );
        }
    }
    assert!(partition
        .ownership(point([0.0; 3]), &mut EvaluationBudget::new(0))
        .is_err());
}
