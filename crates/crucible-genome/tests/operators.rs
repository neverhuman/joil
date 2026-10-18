use crucible_genome::{
    operators::{attempt, classify_duplicate, OperatorBudget, OutcomeKind, Proposal},
    schema::*,
};
use std::collections::BTreeSet;

fn port(name: &str) -> TypedPort {
    TypedPort {
        name: name.into(),
        domain: Domain::Fluid,
        section: "circular".into(),
        frame: "frozen".into(),
        unit: "m".into(),
    }
}
fn module(id: u64, kind: ModuleKind, ports: &[&str]) -> Module {
    Module {
        innovation: InnovationId(id),
        kind,
        ports: ports.iter().map(|p| port(p)).collect(),
        field_roots: vec![],
    }
}
fn base() -> Genome {
    Genome {
        schema_version: 1,
        interface: InterfaceRef {
            schema: "synthetic-operator-test-v1".into(),
            semantic_sha256: "a".repeat(64),
        },
        modules: vec![
            module(10, ModuleKind::Junction, &["out_a", "out_b"]),
            module(11, ModuleKind::Junction, &["in_a", "in_b", "out"]),
            module(12, ModuleKind::Chamber, &["in"]),
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
                from_module: 1,
                from_port: "out".into(),
                to_module: 2,
                to_port: "in".into(),
            },
        ],
        fields: vec![FieldOp::Constant { value: 0.01 }],
        process: ProcessChoice::Unspecified,
    }
}
fn budget() -> OperatorBudget {
    OperatorBudget {
        max_modules: 20,
        max_edges: 40,
        max_fields: 40,
    }
}

#[test]
fn attempts_are_charged_local_and_replayable_with_stable_innovations() {
    let g = base();
    let p = Proposal::ParameterPerturb {
        field: 0,
        delta: 0.002,
    };
    let a = attempt(&g, p.clone(), budget());
    let b = attempt(&g, p, budget());
    assert_eq!(a, b);
    assert_eq!(a.kind, OutcomeKind::Accepted);
    assert_eq!(a.cost.attempts, 1);
    assert_eq!(a.locality, "field:0");
    assert_ne!(a.parent_id, a.proposal_id.clone().unwrap());
    assert_eq!(a.changed_observables, ["field_constant"]);

    let inserted = module(900, ModuleKind::Rib, &["in", "out"]);
    let split = attempt(&g, Proposal::SplitEdge { edge: 0, inserted }, budget());
    assert_eq!(split.kind, OutcomeKind::Accepted);
    assert_eq!(
        split
            .child
            .unwrap()
            .modules
            .iter()
            .find(|m| m.kind == ModuleKind::Rib)
            .unwrap()
            .innovation,
        InnovationId(13)
    );
}

#[test]
fn inapplicable_keepout_like_and_conformance_failures_are_charged() {
    let g = base();
    let missing = attempt(&g, Proposal::RemoveModule { module: 99 }, budget());
    assert_eq!(missing.kind, OutcomeKind::Rejected);
    assert_eq!(missing.cost.attempts, 1);
    assert!(missing.child.is_none());
    assert_eq!(
        missing.proposal,
        Some(Proposal::RemoveModule { module: 99 })
    );

    // Typed ports reject a reconnect that would cross the declared interface contract.
    let bad = attempt(
        &g,
        Proposal::Reconnect {
            edge: 0,
            from_module: 2,
            from_port: "in".into(),
            to_module: 1,
            to_port: "out".into(),
        },
        budget(),
    );
    assert_eq!(bad.kind, OutcomeKind::Rejected);
    assert_eq!(bad.cost.attempts, 1);
    assert!(bad.reason.unwrap().contains("proposal invalid"));

    let limited = attempt(
        &g,
        Proposal::DuplicateModule { module: 0 },
        OperatorBudget {
            max_modules: 3,
            max_edges: 40,
            max_fields: 40,
        },
    );
    assert_eq!(limited.kind, OutcomeKind::BudgetExceeded);
    assert!(limited.cost.over_budget);
    assert_eq!(limited.cost.attempts, 1);
}

#[test]
fn duplicate_neutral_and_crossover_are_separate_outcomes() {
    let g = base();
    let mut duplicate = attempt(
        &g,
        Proposal::ParameterPerturb {
            field: 0,
            delta: 0.001,
        },
        budget(),
    );
    let seen = BTreeSet::from([duplicate.proposal_id.clone().unwrap()]);
    classify_duplicate(&mut duplicate, &seen);
    assert_eq!(duplicate.kind, OutcomeKind::Duplicate);
    assert_eq!(duplicate.cost.attempts, 1);

    let neutral = attempt(
        &g,
        Proposal::HomologousCrossover { other: g.clone() },
        budget(),
    );
    assert_eq!(neutral.kind, OutcomeKind::Neutral);
    assert_eq!(neutral.cost.attempts, 1);
}

#[test]
fn equal_shell_different_interior_remains_distinct_subject() {
    let a = base();
    let mut b = a.clone();
    b.modules[1].kind = ModuleKind::Rib;
    let shell_a = a
        .modules
        .iter()
        .filter(|m| m.kind == ModuleKind::Shell)
        .count();
    let shell_b = b
        .modules
        .iter()
        .filter(|m| m.kind == ModuleKind::Shell)
        .count();
    assert_eq!(shell_a, shell_b);
    assert_ne!(
        a.semantic_identity().unwrap(),
        b.semantic_identity().unwrap()
    );
}

#[test]
fn structural_merge_and_graph_symmetry_are_explicit_topology_changes() {
    let g = base();
    let merged = attempt(&g, Proposal::MergeModule { module: 1 }, budget());
    assert_eq!(merged.kind, OutcomeKind::Accepted, "{:?}", merged.reason);
    assert_eq!(merged.child.as_ref().unwrap().modules.len(), 2);
    assert_eq!(merged.child.as_ref().unwrap().edges.len(), 1);

    let sym = attempt(
        &g,
        Proposal::ChangeGraphSymmetry {
            module: 2,
            enable: true,
        },
        budget(),
    );
    assert_eq!(sym.kind, OutcomeKind::Accepted);
    let mirrored = sym.child.unwrap();
    assert_eq!(
        mirrored.modules.last().unwrap().innovation,
        InnovationId(13)
    );
    let unsym = attempt(
        &mirrored,
        Proposal::ChangeGraphSymmetry {
            module: 2,
            enable: false,
        },
        budget(),
    );
    assert_eq!(unsym.kind, OutcomeKind::Accepted);
    assert_eq!(unsym.child.unwrap().modules.len(), g.modules.len());
}

#[test]
fn expressiveness_registry_is_frozen_and_not_campaign_seed_material() {
    let raw = include_str!("../../../tests/fixtures/v5-expressiveness/cases.json");
    let registry: serde_json::Value = serde_json::from_str(raw).unwrap();
    assert_eq!(registry["freeze"], "v5-12-expressiveness-v1");
    assert_eq!(registry["campaign_seeds"], false);
    assert_eq!(registry["cases"].as_array().unwrap().len(), 16);
    assert_eq!(registry["cases"][0]["id"], "ct35-round-trip");
    assert_eq!(registry["cases"][11]["id"], "narrow-throat-hidden-cavity");
    assert_eq!(
        registry["cases"][15]["id"],
        "equal-shell-different-interior"
    );
    assert!(registry["cases"]
        .as_array()
        .unwrap()
        .iter()
        .all(
            |c| c["geometry_status"] == "mpg_smoke" || c["geometry_status"] == "blocked_no_backend"
        ));
}

#[test]
fn operator_smoke_yield_denominator_reconciles_every_proposal() {
    let g = base();
    let proposal = Proposal::ParameterPerturb {
        field: 0,
        delta: 0.001,
    };
    let first = attempt(&g, proposal.clone(), budget());
    let mut repeated = attempt(&g, proposal, budget());
    classify_duplicate(
        &mut repeated,
        &BTreeSet::from([first.proposal_id.clone().unwrap()]),
    );
    let outcomes = [
        first,
        repeated,
        attempt(&g, Proposal::RemoveModule { module: 99 }, budget()),
        attempt(
            &g,
            Proposal::ParameterPerturb {
                field: 0,
                delta: 0.0,
            },
            budget(),
        ),
        attempt(
            &g,
            Proposal::DuplicateModule { module: 0 },
            OperatorBudget {
                max_modules: 3,
                max_edges: 40,
                max_fields: 40,
            },
        ),
    ];
    assert!(outcomes.iter().all(|o| o.cost.attempts == 1));
    assert_eq!(
        outcomes.len(),
        outcomes
            .iter()
            .map(|o| o.cost.attempts as usize)
            .sum::<usize>()
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| o.kind == OutcomeKind::Accepted)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| o.kind == OutcomeKind::Rejected)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| o.kind == OutcomeKind::Neutral)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| o.kind == OutcomeKind::Duplicate)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| o.kind == OutcomeKind::BudgetExceeded)
            .count(),
        1
    );
}

#[test]
fn bounded_operator_sequence_preserves_interface_process_and_graph_validity() {
    let mut g = base();
    g.process = ProcessChoice::Frozen {
        passport_sha256: "b".repeat(64),
    };
    let interface = g.interface.clone();
    let process = g.process.clone();
    let sequence = [
        Proposal::ParameterPerturb {
            field: 0,
            delta: 0.001,
        },
        Proposal::SplitEdge {
            edge: 0,
            inserted: module(700, ModuleKind::Rib, &["in", "out"]),
        },
        Proposal::Reconnect {
            edge: 0,
            from_module: 88,
            from_port: "missing".into(),
            to_module: 1,
            to_port: "in_a".into(),
        },
        Proposal::HomologousCrossover { other: g.clone() },
        Proposal::ChangeGraphSymmetry {
            module: 2,
            enable: true,
        },
    ];
    for proposal in sequence {
        let outcome = attempt(&g, proposal, budget());
        assert_eq!(outcome.cost.attempts, 1);
        if let Some(child) = outcome.child {
            child.validate().unwrap();
            assert_eq!(child.interface, interface);
            assert_eq!(child.process, process);
            g = child;
        }
    }
}
