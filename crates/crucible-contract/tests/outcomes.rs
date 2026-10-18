mod common;
use common::*;
use crucible_contract::outcomes::*;
use crucible_contract::subject::BlobDigest;
use joil_evidence::{claims::*, identity::*, observation::*, Verdict};
use std::collections::BTreeMap;

fn fixture() -> (ReviewFixture, ContentRef, ContentRef) {
    let mut r = ReviewFixture::new();
    let identity = ScientificIdentity {
        schema: 2,
        subject: SubjectRef::NativeCad {
            artifact: reference("synthetic-geometry"),
            geometry_state: reference("synthetic-state"),
            format: CadFormat::Stl,
            unit: LengthUnit::Metre,
        },
        requirements: vec![r.requirements.clone()],
        source: reference("synthetic-source"),
        build: reference("synthetic-build"),
        cargo_lock: reference("synthetic-lock"),
        config: reference("synthetic-config"),
        evaluator: reference("synthetic-evaluator"),
        scenario: reference("synthetic-scenario"),
        objective: reference("synthetic-objective"),
        descriptor: reference("synthetic-descriptor"),
        solver_build: reference("synthetic-solver"),
        model: reference("synthetic-model"),
        numerics: reference("synthetic-numerics"),
        process: reference("synthetic-process"),
        material: MaterialState::Unknown {
            reason: "synthetic software fixture only".into(),
        },
        dependencies: vec![],
    };
    let measurement = Measurement {
        value: f(1.0),
        unit: Unit::Dimensionless,
        quantity: "synthetic-scalar".into(),
        frame: "SI".into(),
        region: "software-fixture".into(),
        recovery: reference("synthetic-recovery"),
        fidelity: Fidelity::Identity,
        tier: EvidenceTier::SoftwareContract,
        origin: Origin::Synthetic,
        identity: identity.clone(),
        evidence: vec![reference("synthetic-raw")],
        applicability: reference("synthetic-applicability"),
        state: EvidenceState {
            execution: Execution::Succeeded,
            numerical: Numerical::Unverified,
            validation: Validation::NotPerformed,
        },
        uncertainty: vec![Uncertainty {
            component: "synthetic-exact".into(),
            absolute: f(0.0),
            unit: Unit::Dimensionless,
            method: "fixed scalar fixture".into(),
            dependence: "not a statistical sample".into(),
            coverage: "software only".into(),
        }],
    };
    let mut refs = identity.refs();
    refs.extend(measurement.evidence.iter());
    refs.extend([&measurement.recovery, &measurement.applicability]);
    for reference in refs {
        r.registry
            .insert_artifact(reference.clone(), reference.id.as_bytes().to_vec())
            .unwrap();
    }
    let node = ObservationNode {
        id: "synthetic-observation".into(),
        observation: Observation::Measured {
            measurement: Box::new(measurement),
        },
        dependencies: vec![],
    };
    let bytes = serde_json::to_vec(&node).unwrap();
    let observation = ContentRef::from_bytes("synthetic-observation", "1", &bytes);
    r.registry
        .insert_observation(observation.clone(), bytes)
        .unwrap();
    r.review(observation.clone(), ReviewAction::Approved);
    let mut claims = Vec::new();
    for (id, limit) in [("synthetic-pass", 2.0), ("synthetic-fail", 0.0)] {
        let inventory = ClaimInventory {
            id: id.into(),
            identity: identity.clone(),
            criteria: vec![Criterion {
                id: "scalar-limit".into(),
                observation: observation.clone(),
                quantity: "synthetic-scalar".into(),
                unit: Unit::Dimensionless,
                minimum_tier: EvidenceTier::SoftwareContract,
                at_most: f(limit),
            }],
            dependencies: vec![],
        };
        let bytes = serde_json::to_vec(&inventory).unwrap();
        let reference = ContentRef::from_bytes(id, "1", &bytes);
        r.registry
            .install_inventory(
                ApprovedInventory::from_approved_bytes(
                    reference.clone(),
                    std::slice::from_ref(&r.requirements),
                    &bytes,
                )
                .unwrap(),
            )
            .unwrap();
        r.review(reference.clone(), ReviewAction::Approved);
        claims.push(reference);
    }
    assert_eq!(r.registry.resolve(&claims[0]), Verdict::Pass);
    assert_eq!(r.registry.resolve(&claims[1]), Verdict::Fail);
    (r, claims[0].clone(), claims[1].clone())
}
fn policy(
    r: &mut ReviewFixture,
    id: &str,
    claim: ContentRef,
    na: Option<ContentRef>,
) -> (GatePolicy, ContentRef) {
    let record = GatePolicyRecord {
        id: id.into(),
        claim,
        not_applicable_rule: na,
    };
    let bytes = serde_json::to_vec(&record).unwrap();
    let reference = ContentRef::from_bytes(&format!("policy-{id}-{}", r.sequence), "1", &bytes);
    r.approve(reference.clone(), bytes);
    (
        GatePolicy::from_approved_registry(&r.registry, &reference, &r.requirements).unwrap(),
        reference,
    )
}
fn success() -> TaskOutcome {
    TaskOutcome::Succeeded {
        output_manifest: Some(BlobDigest::of_bytes(b"synthetic-output-manifest")),
    }
}
#[test]
fn execution_missing_invalid_unsupported_and_out_of_scope_never_become_physical_fail_or_pass() {
    let (mut r, pass, _) = fixture();
    let (p, _) = policy(&mut r, "synthetic-gate", pass.clone(), None);
    let measured = ObservationState::Measured { claim: pass };
    for (task, reason) in [
        (TaskOutcome::NotRun, NonPassingReason::NotRun),
        (
            TaskOutcome::NumericalFailure { diagnostic: None },
            NonPassingReason::NumericalFailure,
        ),
        (
            TaskOutcome::InfrastructureFailure { diagnostic: None },
            NonPassingReason::InfrastructureFailure,
        ),
        (TaskOutcome::Unsupported, NonPassingReason::Unsupported),
        (
            TaskOutcome::Succeeded {
                output_manifest: None,
            },
            NonPassingReason::AbsentOutput,
        ),
    ] {
        assert_eq!(
            p.decide(&task, &measured, &r.registry).state(),
            &GateState::Unknown { reason }
        );
    }
    for (observation, reason) in [
        (
            ObservationState::Missing,
            NonPassingReason::MissingObservation,
        ),
        (
            ObservationState::Invalid,
            NonPassingReason::InvalidObservation,
        ),
        (ObservationState::OutOfScope, NonPassingReason::OutOfScope),
        (
            ObservationState::NotApplicable {
                rule: reference("unregistered-rule"),
            },
            NonPassingReason::UnregisteredNotApplicable,
        ),
        (
            ObservationState::Measured {
                claim: reference("other-case"),
            },
            NonPassingReason::SourceMismatch,
        ),
    ] {
        assert_eq!(
            p.decide(&success(), &observation, &r.registry).state(),
            &GateState::Unknown { reason }
        );
    }
    assert!(matches!(
        p.decide(&success(), &measured, &r.registry).state(),
        GateState::Pass { .. }
    )); // software-only claim
}
#[test]
fn only_current_source_bound_registry_can_decide_pass_fail_and_not_applicable() {
    let (mut r, pass, fail) = fixture();
    let (pass_policy, policy_ref) = policy(&mut r, "pass", pass.clone(), Some(fail.clone()));
    assert_eq!(
        pass_policy
            .decide(
                &success(),
                &ObservationState::NotApplicable { rule: fail.clone() },
                &r.registry
            )
            .state(),
        &GateState::Unknown {
            reason: NonPassingReason::IncompleteEvidence
        }
    );
    let (fail_policy, _) = policy(&mut r, "fail", fail.clone(), Some(pass.clone()));
    assert!(matches!(
        fail_policy
            .decide(
                &success(),
                &ObservationState::Measured { claim: fail },
                &r.registry
            )
            .state(),
        GateState::Fail { .. }
    ));
    assert!(matches!(
        fail_policy
            .decide(
                &success(),
                &ObservationState::NotApplicable { rule: pass.clone() },
                &r.registry
            )
            .state(),
        GateState::NotApplicable { .. }
    ));
    let mut altered = pass.clone();
    altered.sha256 = "0".repeat(64);
    let (invalid, _) = policy(&mut r, "invalid", altered.clone(), None);
    assert_eq!(
        invalid
            .decide(
                &success(),
                &ObservationState::Measured { claim: altered },
                &r.registry
            )
            .state(),
        &GateState::Unknown {
            reason: NonPassingReason::InvalidEvidence
        }
    );
    let missing = reference("missing-claim");
    let (incomplete, _) = policy(&mut r, "missing", missing.clone(), None);
    assert_eq!(
        incomplete
            .decide(
                &success(),
                &ObservationState::Measured { claim: missing },
                &r.registry
            )
            .state(),
        &GateState::Unknown {
            reason: NonPassingReason::IncompleteEvidence
        }
    );
    r.review(pass.clone(), ReviewAction::Reopened);
    assert_eq!(
        pass_policy
            .decide(
                &success(),
                &ObservationState::Measured {
                    claim: pass.clone()
                },
                &r.registry
            )
            .state(),
        &GateState::Unknown {
            reason: NonPassingReason::ReopenedEvidence
        }
    );
    r.review(pass.clone(), ReviewAction::Revoked);
    assert_eq!(
        pass_policy
            .decide(
                &success(),
                &ObservationState::Measured { claim: pass },
                &r.registry
            )
            .state(),
        &GateState::Unknown {
            reason: NonPassingReason::RevokedEvidence
        }
    );
    r.review(policy_ref, ReviewAction::Revoked);
    assert_eq!(
        pass_policy
            .decide(&success(), &ObservationState::Missing, &r.registry)
            .state(),
        &GateState::Unknown {
            reason: NonPassingReason::UnreviewedPolicy
        }
    );
}
#[test]
fn cleared_missing_renamed_or_stale_gate_inventory_cannot_become_eligible() {
    let (mut r, pass, _) = fixture();
    let (p, policy_ref) = policy(&mut r, "required-gate", pass.clone(), None);
    let record = GateInventoryRecord {
        id: "synthetic-inventory".into(),
        requirements: r.requirements.clone(),
        gates: vec![GatePolicyRecord {
            id: p.id().into(),
            claim: pass.clone(),
            not_applicable_rule: None,
        }],
    };
    let bytes = serde_json::to_vec(&record).unwrap();
    let reference = ContentRef::from_bytes("synthetic-inventory", "1", &bytes);
    r.approve(reference.clone(), bytes);
    let inventory =
        ApprovedGateInventory::from_approved_registry(&r.registry, &reference, &r.requirements)
            .unwrap();
    assert!(matches!(
        ClaimEligibility::from_gate_decisions(&inventory, &BTreeMap::new(), &r.registry).state(),
        EligibilityState::Blocked { .. }
    ));
    let decisions = BTreeMap::from([(
        p.id().into(),
        p.decide(
            &success(),
            &ObservationState::Measured { claim: pass },
            &r.registry,
        ),
    )]);
    assert!(matches!(
        ClaimEligibility::from_gate_decisions(&inventory, &decisions, &r.registry).state(),
        EligibilityState::ReadyForEvidenceReview { .. }
    ));
    let renamed = BTreeMap::from([(
        "unregistered-alias".into(),
        decisions.values().next().unwrap().clone(),
    )]);
    assert!(matches!(
        ClaimEligibility::from_gate_decisions(&inventory, &renamed, &r.registry).state(),
        EligibilityState::Blocked { .. }
    ));
    r.review(policy_ref, ReviewAction::Revoked);
    assert!(matches!(
        ClaimEligibility::from_gate_decisions(&inventory, &decisions, &r.registry).state(),
        EligibilityState::Blocked { .. }
    ));
    r.review(reference, ReviewAction::Revoked);
    assert!(matches!(
        ClaimEligibility::from_gate_decisions(&inventory, &decisions, &r.registry).state(),
        EligibilityState::Blocked { .. }
    ));
}

#[test]
fn a_passing_claim_under_old_requirements_cannot_satisfy_a_new_policy() {
    let (mut r, pass, _) = fixture();
    let old = r.requirements.clone();
    r.requirements = reference("different-new-requirements");
    assert_eq!(r.registry.resolve(&pass), Verdict::Pass);
    assert_eq!(
        r.registry
            .resolve_in_scope(&pass, std::slice::from_ref(&old)),
        Verdict::Pass
    );
    assert_eq!(
        r.registry
            .resolve_in_scope(&pass, std::slice::from_ref(&r.requirements)),
        Verdict::Invalid
    );
    let (p, _) = policy(&mut r, "new-scope", pass.clone(), None);
    assert_eq!(
        p.decide(
            &success(),
            &ObservationState::Measured { claim: pass },
            &r.registry
        )
        .state(),
        &GateState::Unknown {
            reason: NonPassingReason::InvalidEvidence
        }
    );
}
