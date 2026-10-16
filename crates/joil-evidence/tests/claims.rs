mod common;
use joil_evidence::{claims::*, identity::*, observation::*, Verdict};

fn add_review(registry: &mut Registry, subject: ContentRef, action: ReviewAction, sequence: u64) {
    let event = ReviewEvent {
        sequence,
        subject,
        action,
        reviewer: "reviewer".into(),
        requirements: common::identity().requirements,
    };
    let bytes = serde_json::to_vec(&event).unwrap();
    registry
        .append_approved_review(
            "reviewer",
            &event.requirements,
            ContentRef::from_bytes(&format!("review-{sequence}"), "1", &bytes),
            bytes,
        )
        .unwrap();
}

fn fixture() -> (Registry, ContentRef, ContentRef, ContentRef) {
    let mut registry = Registry::default();
    let measurement = common::measurement();
    let mut refs = measurement.identity.refs();
    refs.extend(measurement.evidence.iter());
    refs.extend([&measurement.recovery, &measurement.applicability]);
    for reference in refs {
        registry
            .insert_artifact(reference.clone(), reference.id.as_bytes().to_vec())
            .unwrap();
    }
    let node = ObservationNode {
        id: "observation".into(),
        observation: Observation::Measured {
            measurement: Box::new(measurement),
        },
        dependencies: vec![],
    };
    let bytes = serde_json::to_vec(&node).unwrap();
    let observed = ContentRef::from_bytes("observation", "1", &bytes);
    registry
        .insert_observation(observed.clone(), bytes)
        .unwrap();
    let mut claims = vec![];
    for id in ["base-claim", "derived-claim"] {
        let inventory = ClaimInventory {
            id: id.into(),
            identity: common::identity(),
            criteria: vec![Criterion {
                id: "quality".into(),
                observation: observed.clone(),
                quantity: "cout".into(),
                unit: Unit::MgPerLitreDispersedOil,
                minimum_tier: EvidenceTier::NumericallyVerified,
                at_most: Finite::new(30.0).unwrap(),
            }],
            dependencies: claims.clone(),
        };
        let bytes = serde_json::to_vec(&inventory).unwrap();
        let reference = ContentRef::from_bytes(id, "1", &bytes);
        registry
            .install_inventory(
                ApprovedInventory::from_approved_bytes(
                    reference.clone(),
                    &inventory.identity.requirements,
                    &bytes,
                )
                .unwrap(),
            )
            .unwrap();
        claims.push(reference);
    }
    add_review(&mut registry, observed.clone(), ReviewAction::Approved, 1);
    add_review(&mut registry, claims[0].clone(), ReviewAction::Approved, 2);
    add_review(&mut registry, claims[1].clone(), ReviewAction::Approved, 3);
    (registry, observed, claims[0].clone(), claims[1].clone())
}

#[test]
fn revoked_dependency_revokes_every_downstream_claim() {
    let (mut registry, observation, base, derived) = fixture();
    assert_eq!(registry.resolve(&base), Verdict::Pass);
    assert_eq!(registry.resolve(&derived), Verdict::Pass);
    add_review(&mut registry, observation, ReviewAction::Revoked, 4);
    assert_eq!(registry.resolve(&base), Verdict::Revoked);
    assert_eq!(registry.resolve(&derived), Verdict::Revoked);
}

#[test]
fn mutation_missing_source_and_review_authority_fail_closed() {
    let (mut registry, _, base, derived) = fixture();
    let mut mutant = derived.clone();
    mutant.sha256 = "0".repeat(64);
    assert_eq!(registry.resolve(&mutant), Verdict::Invalid);
    registry = Registry::default();
    assert_eq!(registry.resolve(&base), Verdict::Incomplete);
    assert!(registry
        .insert_artifact(common::reference("bytes"), b"tampered".to_vec())
        .is_err());
    let event = ReviewEvent {
        sequence: 1,
        subject: base,
        action: ReviewAction::Approved,
        reviewer: "self".into(),
        requirements: common::identity().requirements,
    };
    let bytes = serde_json::to_vec(&event).unwrap();
    assert!(registry
        .append_approved_review(
            "trusted",
            &event.requirements,
            ContentRef::from_bytes("review", "1", &bytes),
            bytes
        )
        .is_err());
}
