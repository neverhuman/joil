//! Synthetic fixtures exercise admission plumbing only; no physical dataset is asserted.
mod common;
use joil_evidence::{claims::*, identity::*, observation::*, training::*};

fn review(registry: &mut Registry, subject: ContentRef, action: ReviewAction, sequence: &mut u64) {
    *sequence += 1;
    let event = ReviewEvent {
        sequence: *sequence,
        subject,
        action,
        reviewer: "synthetic-fixture-reviewer".into(),
        requirements: common::identity().requirements,
    };
    let bytes = serde_json::to_vec(&event).unwrap();
    registry
        .append_approved_review(
            "synthetic-fixture-reviewer",
            &event.requirements,
            ContentRef::from_bytes(&format!("review-{sequence}"), "test", &bytes),
            bytes,
        )
        .unwrap();
}

fn artifact<T: serde::Serialize>(
    registry: &mut Registry,
    id: &str,
    value: &T,
    sequence: &mut u64,
) -> ContentRef {
    let bytes = serde_json::to_vec(value).unwrap();
    let reference = ContentRef::from_bytes(id, "test", &bytes);
    registry.insert_artifact(reference.clone(), bytes).unwrap();
    review(
        registry,
        reference.clone(),
        ReviewAction::Approved,
        sequence,
    );
    reference
}

fn observed(
    registry: &mut Registry,
    id: &str,
    measurement: Measurement,
    sequence: &mut u64,
) -> ContentRef {
    let node = ObservationNode {
        id: id.into(),
        observation: Observation::Measured {
            measurement: Box::new(measurement),
        },
        dependencies: vec![],
    };
    let bytes = serde_json::to_vec(&node).unwrap();
    let reference = ContentRef::from_bytes(id, "test", &bytes);
    registry
        .insert_observation(reference.clone(), bytes)
        .unwrap();
    review(
        registry,
        reference.clone(),
        ReviewAction::Approved,
        sequence,
    );
    reference
}

#[test]
fn independent_admission_requires_measured_features_review_and_live_dependencies() {
    let mut registry = Registry::default();
    let mut sequence = 0;
    let m = common::measurement();
    let mut refs = m.identity.refs();
    refs.extend(m.evidence.iter());
    refs.extend([&m.recovery, &m.applicability]);
    for r in refs {
        registry
            .insert_artifact(r.clone(), r.id.as_bytes().to_vec())
            .unwrap();
    }
    let label = observed(&mut registry, "label", m.clone(), &mut sequence);
    let mut specs = vec![];
    let mut features = vec![];
    for i in 0..12 {
        let quantity = format!("observed-geometry-{i}");
        let mut feature = m.clone();
        feature.quantity = quantity.clone();
        feature.unit = Unit::Metre;
        feature.uncertainty[0].unit = Unit::Metre;
        feature.tier = EvidenceTier::Geometry;
        feature.fidelity = Fidelity::F0;
        features.push(observed(
            &mut registry,
            &format!("feature-{i}"),
            feature,
            &mut sequence,
        ));
        specs.push(ObservedFeatureSpec {
            quantity,
            unit: Unit::Metre,
            offset: Finite::new(0.0).unwrap(),
            scale: Finite::new(1.0).unwrap(),
        });
    }
    let contract = artifact(
        &mut registry,
        "observed-feature-contract",
        &FeatureContract {
            id: "observed-feature-contract".into(),
            features: specs,
        },
        &mut sequence,
    );
    let observer = artifact(
        &mut registry,
        "observer-code",
        &"synthetic-test-observer",
        &mut sequence,
    );
    let mut group = ObservedGroups {
        kind: GroupKind::GeometryFamily,
        identity_sha256: m.identity.sha256().unwrap(),
        labels: vec!["observed-fixture-cluster".into()],
        observer,
        observations: vec![features[0].clone()],
    };
    let geometry_groups = artifact(&mut registry, "geometry-group", &group, &mut sequence);
    group.kind = GroupKind::Mechanism;
    let mechanism_groups = artifact(&mut registry, "mechanism-group", &group, &mut sequence);
    let input = TrainingInput {
        id: "software-fixture".into(),
        label: label.clone(),
        features: features.try_into().unwrap(),
        geometry_groups,
        mechanism_groups,
    };
    let sample = admit_sample(&registry, &m.identity.requirements, &contract, &input).unwrap();
    assert_eq!(sample.target().get(), 25.0);
    assert_eq!(sample.features(), &[25.0; 12]);
    let mut missing = input.clone();
    missing.features[0] = common::reference("absent");
    assert!(admit_sample(&registry, &m.identity.requirements, &contract, &missing).is_err());
    let mut prior = m.clone();
    prior.tier = EvidenceTier::AnalyticScreen;
    prior.fidelity = Fidelity::F1;
    let mut wrong = input.clone();
    wrong.label = observed(&mut registry, "prior-label", prior, &mut sequence);
    assert!(admit_sample(&registry, &m.identity.requirements, &contract, &wrong).is_err());
    review(&mut registry, label, ReviewAction::Revoked, &mut sequence);
    assert!(admit_sample(&registry, &m.identity.requirements, &contract, &input).is_err());
}
