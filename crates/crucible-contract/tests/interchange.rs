use crucible_contract::{keys::*, outcomes::*, strict_decode, subject::*};
use crucible_units::Expr;
use std::{collections::BTreeMap, path::Path, process::Command};

#[test]
fn frozen_wire_examples_roundtrip_and_reject_leaked_or_invalid_inputs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/v5/fixtures");
    let mut valid = 0;
    let mut invalid = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        if name.starts_with("valid-") {
            match name {
                "valid-subject.json" => {
                    SubjectBundle::from_json(&bytes).unwrap();
                }
                "valid-surface.json" => {
                    TriangleSurface::from_json(&bytes).unwrap();
                }
                "valid-evaluation-key.json" => {
                    strict_decode::<EvaluationKey>(&bytes)
                        .unwrap()
                        .identity()
                        .unwrap();
                }
                "valid-decision-key.json" => {
                    strict_decode::<DecisionKey>(&bytes)
                        .unwrap()
                        .identity()
                        .unwrap();
                }
                "valid-task-outcome.json" => {
                    strict_decode::<TaskOutcome>(&bytes).unwrap();
                }
                "valid-observation-state.json" => {
                    strict_decode::<ObservationState>(&bytes).unwrap();
                }
                "valid-expression.json" => {
                    let value = Expr::from_json(&bytes)
                        .unwrap()
                        .evaluate(&BTreeMap::new(), 32)
                        .unwrap();
                    assert_eq!(value.value.get(), 5000.0); // 10 W / .002 treated m3/s
                }
                _ => panic!("uncovered fixture {name}"),
            }
            valid += 1;
        } else if name.starts_with("invalid-subject-") {
            assert!(SubjectBundle::from_json(&bytes).is_err(), "{name}");
            invalid += 1;
        } else if name == "invalid-task-outcome-forged-pass.json" {
            assert!(strict_decode::<TaskOutcome>(&bytes).is_err());
            invalid += 1;
        } else if name == "invalid-expression-dimension.json" {
            assert!(Expr::from_json(&bytes)
                .unwrap()
                .evaluate(&BTreeMap::new(), 32)
                .is_err());
            invalid += 1;
        } else {
            panic!("uncovered fixture {name}");
        }
    }
    assert_eq!((valid, invalid), (7, 8));
}

#[test]
fn generated_interchange_is_current() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let result = Command::new("rtk")
        .args([
            "proxy",
            "python3",
            "schemas/v5/generate_contracts.py",
            "--check",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
