use crucible_genome::{
    develop::{develop, DevelopmentBudget, DevelopmentError, DevelopmentRequest, Rewrite},
    legacy::{lower_ct35, lower_legacy_layers, Lowering},
    schema::*,
};

fn iface() -> InterfaceRef {
    InterfaceRef {
        schema: "synthetic-test-v1".into(),
        semantic_sha256: "a".repeat(64),
    }
}
fn port(name: &str) -> TypedPort {
    TypedPort {
        name: name.into(),
        domain: Domain::Fluid,
        section: "circular".into(),
        frame: "si_z_downstream".into(),
        unit: "m".into(),
    }
}
fn genome() -> Genome {
    Genome {
        schema_version: 1,
        interface: iface(),
        modules: vec![
            Module {
                innovation: InnovationId(10),
                kind: ModuleKind::Junction,
                ports: vec![port("out_a"), port("out_b")],
                field_roots: vec![],
            },
            Module {
                innovation: InnovationId(11),
                kind: ModuleKind::Junction,
                ports: vec![port("in_a"), port("in_b"), port("out")],
                field_roots: vec![],
            },
            Module {
                innovation: InnovationId(12),
                kind: ModuleKind::Chamber,
                ports: vec![port("in")],
                field_roots: vec![],
            },
        ],
        edges: vec![
            ModuleEdge {
                relation: EdgeRelation::Construction,
                from_module: 0,
                from_port: "out_a".into(),
                to_module: 1,
                to_port: "in_a".into(),
            },
            ModuleEdge {
                relation: EdgeRelation::Construction,
                from_module: 0,
                from_port: "out_b".into(),
                to_module: 1,
                to_port: "in_b".into(),
            },
            ModuleEdge {
                relation: EdgeRelation::Construction,
                from_module: 1,
                from_port: "out".into(),
                to_module: 2,
                to_port: "in".into(),
            },
        ],
        fields: vec![
            FieldOp::Coordinate { axis: Axis::X },
            FieldOp::Constant { value: 0.01 },
            FieldOp::Add { a: 0, b: 1 },
        ],
        process: ProcessChoice::Unspecified,
    }
}
fn budget() -> DevelopmentBudget {
    DevelopmentBudget {
        max_modules: 32,
        max_edges: 64,
        max_field_nodes: 64,
        max_coefficients: 128,
        max_rewrite_applications: 16,
        max_depth: 32,
        max_spatial_frequency_m_inv: 8.,
        min_resolved_feature_m: 1e-4,
    }
}

#[test]
fn strict_canonical_roundtrip_and_branch_rejoin() {
    let g = genome();
    g.validate().unwrap();
    let bytes = g.canonical_json().unwrap();
    let decoded = Genome::strict_from_json(&bytes).unwrap();
    assert_eq!(bytes, decoded.canonical_json().unwrap());
    assert_eq!(
        g.semantic_identity().unwrap(),
        decoded.semantic_identity().unwrap()
    );
    let with_unknown = String::from_utf8(bytes).unwrap().replace(
        "\"schema_version\":1",
        "\"surprise\":true,\"schema_version\":1",
    );
    assert!(Genome::strict_from_json(with_unknown.as_bytes()).is_err());
}
#[test]
fn rejects_forward_construction_and_cyclic_evaluation() {
    let mut g = genome();
    g.edges[0].from_module = 2;
    assert!(g.validate().is_err());
    let mut g = genome();
    g.fields[2] = FieldOp::Add { a: 0, b: 2 };
    assert!(g.validate().is_err());
}
#[test]
fn fluid_recirculation_is_not_a_construction_cycle() {
    let mut g = genome();
    g.edges.push(ModuleEdge {
        relation: EdgeRelation::FluidFlow,
        from_module: 2,
        from_port: "in".into(),
        to_module: 0,
        to_port: "out_a".into(),
    });
    assert!(g.validate().is_ok());
}
#[test]
fn stable_development_identity_and_labels_are_sidecar_only() {
    let req = DevelopmentRequest {
        genome: genome(),
        rewrites: vec![],
        mechanism_labels: Default::default(),
    };
    let a = develop(req.clone(), &budget()).unwrap();
    let b = develop(req, &budget()).unwrap();
    assert_eq!(a.origin, b.origin);
    assert_eq!(a.partition_id, b.partition_id);
    let c = develop(
        DevelopmentRequest {
            genome: genome(),
            rewrites: vec![],
            mechanism_labels: [(0, "different label".into())].into(),
        },
        &budget(),
    )
    .unwrap();
    assert_eq!(a.partition_id, c.partition_id);
    assert_ne!(a.render.labels, c.render.labels);
    assert_eq!(a.judge_input(), c.judge_input());
    let bytes = serde_json::to_value(c.judge_input()).unwrap();
    assert!(bytes.get("origin").is_none());
    assert!(bytes.get("render").is_none());
}
#[test]
fn budget_exhaustion_fails_closed_without_baseline_substitution() {
    let mut b = budget();
    b.max_rewrite_applications = 0;
    let result = develop(
        DevelopmentRequest {
            genome: genome(),
            rewrites: vec![Rewrite::AddField {
                field: FieldOp::Constant { value: 1.0 },
            }],
            mechanism_labels: Default::default(),
        },
        &b,
    );
    assert!(matches!(
        result,
        Err(DevelopmentError::Budget {
            resource: "rewrite_applications",
            ..
        })
    ));
}
#[test]
fn staged_fields_reject_neural_high_frequency_and_bad_resolution() {
    let mut g = genome();
    g.fields.push(FieldOp::NeuralField { weights: vec![0.0] });
    assert!(matches!(g.validate(), Err(SchemaError::Unsupported(_))));
    let mut g = genome();
    g.fields.push(FieldOp::Primitive {
        kind: ModuleKind::Shell,
        inputs: vec![],
        coefficients: vec![1.],
        max_spatial_frequency_m_inv: 9.,
        min_feature_m: 1e-3,
    });
    assert!(matches!(
        develop(
            DevelopmentRequest {
                genome: g,
                rewrites: vec![],
                mechanism_labels: Default::default()
            },
            &budget()
        ),
        Err(DevelopmentError::ResolutionBudget {
            resource: "spatial_frequency_m_inv",
            ..
        })
    ));
    let mut g = genome();
    g.fields.push(FieldOp::Primitive {
        kind: ModuleKind::Shell,
        inputs: vec![],
        coefficients: vec![1.],
        max_spatial_frequency_m_inv: 1.,
        min_feature_m: 0.,
    });
    assert!(g.validate().is_err());
}
#[test]
fn ct35_migration_preserves_all_genes_and_marks_reference_unqualified() {
    let bytes = include_bytes!("../../../baselines/ct35/genome.json");
    let migration = lower_ct35(bytes, iface()).unwrap();
    assert_eq!(
        migration.retained_fields["parameters"]
            .as_object()
            .unwrap()
            .len(),
        20
    );
    assert_eq!(migration.certified_baseline_error_bounds, None);
    assert_eq!(
        migration.zero_mutation_contract_sha256,
        joil_evidence::identity::scientific_hash(&iface()).unwrap()
    );
}
#[test]
fn unsupported_legacy_exotic_is_not_lowered_to_radial_proxy() {
    let input = br#"{"tpms_basis":"gyroid","score":1.0}"#;
    let m = lower_legacy_layers(4, input, iface()).unwrap();
    assert!(matches!(m.lowering, Lowering::Unsupported { .. }));
    assert_eq!(m.retained_fields.len(), 2);
}
