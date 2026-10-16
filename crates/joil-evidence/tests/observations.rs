mod common;
use joil_evidence::{observation::*, Verdict};

#[test]
fn unknown_never_passes() {
    let states = [
        Observation::Unknown {
            reason: "not run".into(),
        },
        Observation::Failed {
            class: FailureClass::Numerical,
            reason: "diverged".into(),
            diagnostics: vec![],
        },
        Observation::NotApplicable {
            reason: "no oil".into(),
        },
        Observation::Insufficient {
            reason: "too few samples".into(),
            missing: vec!["replicates".into()],
        },
    ];
    for observation in states {
        assert_eq!(
            observation.compare_at_most(
                Finite::new(30.0).unwrap(),
                Unit::MgPerLitreDispersedOil,
                EvidenceTier::NumericallyVerified
            ),
            Verdict::Incomplete
        );
    }
}

#[test]
fn measured_value_requires_units_uncertainty_and_correct_tier() {
    let mut measurement = common::measurement();
    let observation = Observation::Measured {
        measurement: Box::new(measurement.clone()),
    };
    assert_eq!(
        observation.compare_at_most(
            Finite::new(30.0).unwrap(),
            Unit::MgPerLitreDispersedOil,
            EvidenceTier::NumericallyVerified
        ),
        Verdict::Pass
    );
    assert_eq!(
        observation.compare_at_most(
            Finite::new(30.0).unwrap(),
            Unit::Pascal,
            EvidenceTier::NumericallyVerified
        ),
        Verdict::Invalid
    );
    assert_eq!(
        observation.compare_at_most(
            Finite::new(30.0).unwrap(),
            Unit::MgPerLitreDispersedOil,
            EvidenceTier::PhysicalValidation
        ),
        Verdict::Incomplete
    );
    measurement.uncertainty.clear();
    assert!(measurement.check().is_err());
    measurement = common::measurement();
    measurement.origin = Origin::Synthetic;
    assert!(measurement.check().is_err());
    measurement = common::measurement();
    measurement.state.execution = Execution::Failed;
    assert!(measurement.check().is_err());
    measurement = common::measurement();
    measurement.state.validation = Validation::Validated;
    assert!(measurement.check().is_err());
    assert!(Finite::new(f64::NAN).is_err());
    assert!(Finite::new(f64::INFINITY).is_err());
}

#[test]
fn strict_observation_wire_rejects_unknown_fields_and_values() {
    let observation = Observation::Measured {
        measurement: Box::new(common::measurement()),
    };
    let bytes = serde_json::to_vec(&observation).unwrap();
    assert_eq!(ingest_observation(&bytes).unwrap(), observation);
    assert!(ingest_observation(br#"{"status":"unknown","reason":"missing","value":0}"#).is_err());
    assert!(ingest_observation(br#"{"status":"pass","value":0}"#).is_err());
}
