//! Explicitly synthetic wire fixtures; run only to regenerate reviewed examples.
#[path = "../tests/common/mod.rs"]
mod common;
use crucible_contract::{keys::*, outcomes::*, subject::*};
use crucible_units::{Expr, Quantity, Unit};
use std::path::Path;
fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/v5/fixtures");
    std::fs::create_dir_all(&root).unwrap();
    let (_, bundle, _) = common::bundle();
    let d = BlobDigest::of_bytes(b"synthetic software fixture input");
    let eval = EvaluationKey {
        schema_version: 1,
        subject: bundle.clone(),
        manufacturing_state: d.clone(),
        scenario: d.clone(),
        solver_build: d.clone(),
        container: d.clone(),
        numerics: d.clone(),
        mesh_policy: d.clone(),
        sampling_design: d.clone(),
        randomness: d.clone(),
        extractor: d.clone(),
        calibration: Calibration::Uncalibrated,
    };
    let decision = DecisionKey {
        observation: eval.identity().unwrap(),
        requirement: d.clone(),
        decision_profile: d.clone(),
        preference_policy: d.clone(),
    };
    let expression = Expr::EnergyPerTreatedVolume {
        mechanical_loss: Box::new(Expr::Constant {
            quantity: Quantity {
                value: common::f(10.0),
                unit: Unit::MechanicalLossWatt,
            },
        }),
        treated_flow: Box::new(Expr::Constant {
            quantity: Quantity {
                value: common::f(0.002),
                unit: Unit::TreatedCubicMetrePerSecond,
            },
        }),
    };
    for (name, value) in [
        ("subject", serde_json::to_value(bundle).unwrap()),
        ("surface", serde_json::to_value(common::surface()).unwrap()),
        ("evaluation-key", serde_json::to_value(eval).unwrap()),
        ("decision-key", serde_json::to_value(decision).unwrap()),
        ("expression", serde_json::to_value(expression).unwrap()),
        (
            "task-outcome",
            serde_json::to_value(TaskOutcome::Succeeded {
                output_manifest: Some(d),
            })
            .unwrap(),
        ),
        (
            "observation-state",
            serde_json::to_value(ObservationState::Missing).unwrap(),
        ),
    ] {
        std::fs::write(
            root.join(format!("valid-{name}.json")),
            format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
        )
        .unwrap();
    }
}
