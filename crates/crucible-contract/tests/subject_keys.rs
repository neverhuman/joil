mod common;
use common::*;
use crucible_contract::keys::*;
use crucible_contract::subject::*;
use crucible_contract::{strict_decode, ContractError};
use joil_evidence::claims::ReviewAction;

#[test]
fn artifact_only_evaluator_uses_verified_geometry_without_genotype() {
    let (review, bundle, objects) = bundle();
    let view = bundle
        .verify_artifacts(&review.registry, &review.requirements, |r| {
            objects
                .get(r)
                .cloned()
                .ok_or(ContractError::ArtifactMismatch)
        })
        .unwrap();
    // Independent tetrahedron determinant: this judge has only a verified
    // physical view and a content-addressed reader, with no design program.
    fn judge(
        view: &VerifiedSubject,
        objects: &std::collections::BTreeMap<BlobDigest, Vec<u8>>,
    ) -> f64 {
        let artifact = view
            .bundle()
            .artifacts
            .iter()
            .find(|a| a.role == ArtifactRole::SurfaceMesh)
            .unwrap();
        let surface = TriangleSurface::from_json(&objects[&artifact.blob]).unwrap();
        let v: Vec<[f64; 3]> = surface
            .vertices_m
            .iter()
            .map(|p| p.map(|v| v.get()))
            .collect();
        surface
            .triangles
            .iter()
            .map(|&[a, b, c]| {
                let (a, b, c) = (v[a as usize], v[b as usize], v[c as usize]);
                (a[0] * (b[1] * c[2] - b[2] * c[1])
                    + a[1] * (b[2] * c[0] - b[0] * c[2])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum::<f64>()
    }
    assert!((judge(&view, &objects) - 1.0 / 6.0).abs() < 1e-15);
    assert_eq!(view.identity(), &bundle.identity().unwrap());
}
#[test]
fn changed_bytes_units_frozen_tail_unreviewed_and_revoked_acceptance_refuse() {
    let (mut review, original, objects) = bundle();
    assert!(original
        .verify_artifacts(&review.registry, &review.requirements, |_| Ok(
            b"different bytes".to_vec()
        ))
        .is_err());
    let mut wrong = original.clone();
    wrong.artifacts[1].length_unit = LengthUnit::Millimetre;
    assert!(wrong.check_structure().is_err());
    let mut wrong = original.clone();
    wrong.frozen_regions[0].boundary = BlobDigest::of_bytes(b"changed-downstream-tail");
    assert!(matches!(
        wrong.verify_artifacts(&review.registry, &review.requirements, |r| Ok(
            objects[r].clone()
        )),
        Err(ContractError::InterfaceMismatch)
    ));
    let mut wrong = original.clone();
    wrong.artifacts[0].approximation_bound_m = m(1e-4);
    assert!(wrong
        .verify_artifacts(&review.registry, &review.requirements, |r| Ok(
            objects[r].clone()
        ))
        .is_err());
    assert!(original
        .verify_artifacts(&review.registry, &review.requirements, |r| {
            if r == &original.frozen_regions[0].boundary {
                Err(ContractError::ArtifactMismatch)
            } else {
                Ok(objects[r].clone())
            }
        })
        .is_err());
    let r = original.artifacts[0]
        .acceptance
        .reference("v5.artifact-acceptance");
    review.review(r, ReviewAction::Revoked);
    assert!(original
        .verify_artifacts(&review.registry, &review.requirements, |r| Ok(
            objects[r].clone()
        ))
        .is_err());
}
#[test]
fn labels_expected_scores_duplicate_keys_and_bad_digest_cannot_leak_through_wire() {
    let (_, bundle, _) = bundle();
    let mut value = serde_json::to_value(&bundle).unwrap();
    for key in [
        "expected_mechanism_score",
        "family_bonus",
        "genome",
        "ancestry",
        "display_label",
    ] {
        value[key] = serde_json::json!("forbidden-side-channel");
        assert!(SubjectBundle::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
        value.as_object_mut().unwrap().remove(key);
    }
    value["artifacts"][0]["module_kind"] = serde_json::json!("expected-winner");
    assert!(SubjectBundle::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(SubjectBundle::from_json(b"{\"schema_version\":1,\"schema_version\":1}").is_err());
    assert!(strict_decode::<BlobDigest>(br#""not-a-digest""#).is_err());
    assert!(strict_decode::<crucible_units::Metres>(b"-0.0").is_err());
    let mut surface = serde_json::to_value(surface()).unwrap();
    surface["expected_family"] = serde_json::json!("elite");
    assert!(TriangleSurface::from_json(&serde_json::to_vec(&surface).unwrap()).is_err());
}
fn key() -> EvaluationKey {
    let (_, bundle, _) = bundle();
    let d = BlobDigest::of_bytes(b"synthetic-evaluation-input");
    EvaluationKey {
        schema_version: 1,
        subject: bundle,
        manufacturing_state: d.clone(),
        scenario: d.clone(),
        solver_build: d.clone(),
        container: d.clone(),
        numerics: d.clone(),
        mesh_policy: d.clone(),
        sampling_design: d.clone(),
        randomness: d.clone(),
        extractor: d,
        calibration: Calibration::Uncalibrated,
    }
}
#[test]
fn every_observation_dependency_invalidates_cache_while_policy_reranking_does_not() {
    let base = key();
    let identity = base.identity().unwrap();
    let changed = BlobDigest::of_bytes(b"independently changed dependency");
    let mut mutants = Vec::new();
    for field in 0..13 {
        let mut x = base.clone();
        match field {
            0 => x.manufacturing_state = changed.clone(),
            1 => x.scenario = changed.clone(),
            2 => x.solver_build = changed.clone(),
            3 => x.container = changed.clone(),
            4 => x.numerics = changed.clone(),
            5 => x.mesh_policy = changed.clone(),
            6 => x.sampling_design = changed.clone(),
            7 => x.randomness = changed.clone(),
            8 => x.extractor = changed.clone(),
            9 => {
                x.calibration = Calibration::Bound {
                    record: changed.clone(),
                }
            }
            10 => x.subject.artifacts[0].blob = changed.clone(),
            11 => x.subject.artifacts[0].approximation_bound_m = m(2e-7),
            12 => {
                x.subject.material = MaterialState::Identified {
                    passport: changed.clone(),
                }
            }
            _ => unreachable!(),
        };
        mutants.push(x);
    }
    for x in mutants {
        assert_ne!(identity, x.identity().unwrap());
    }
    let mut policy = DecisionKey {
        observation: identity.clone(),
        requirement: changed.clone(),
        decision_profile: changed.clone(),
        preference_policy: BlobDigest::of_bytes(b"weights-v1"),
    };
    let first = policy.identity().unwrap();
    policy.preference_policy = BlobDigest::of_bytes(b"weights-v2");
    assert_ne!(first, policy.identity().unwrap());
    assert_eq!(identity, base.identity().unwrap());
    assert_ne!(
        BlobDigest::of_bytes(b"1").as_str(),
        SemanticId::of(&1_u64).unwrap().as_str()
    );
}
