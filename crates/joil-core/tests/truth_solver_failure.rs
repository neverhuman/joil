use joil_core::openfoam::{evaluate_measured_cfd_kpis, read_cfd_kpi_observation};
use joil_evidence::observation::{FailureClass, Observation};

#[test]
fn truth_solver_failure_cannot_become_performance() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/truth-solver-failure");
    std::fs::create_dir_all(&root).unwrap();
    for (name, receipt) in [
        (
            "failed-exit",
            serde_json::json!({"exit_code": 2, "stop_reason": null}),
        ),
        (
            "stopped",
            serde_json::json!({"exit_code": 0, "stop_reason": "timeout"}),
        ),
    ] {
        let case = root.join(name);
        std::fs::create_dir_all(&case).unwrap();
        // Synthetic stale good-looking numbers cannot override a failed solve.
        let integrals = case.join("integrals.json");
        std::fs::write(
            &integrals,
            r#"{"time":7,"hydraulic_loss_j_per_treated_m3":1.0}"#,
        )
        .unwrap();
        std::fs::write(
            case.join("foamRun.log.receipt.json"),
            serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
        assert!(evaluate_measured_cfd_kpis(&root, &integrals, 7).is_err());
        let failure = read_cfd_kpi_observation(&root, &integrals, 7).unwrap_err();
        failure.check().unwrap();
        assert!(matches!(
            failure,
            Observation::Failed {
                class: FailureClass::Numerical,
                ..
            }
        ));
        let json = serde_json::to_value(&failure).unwrap();
        assert_eq!(json["status"], "failed");
        assert!(json.get("measured_hydraulics").is_none());
        assert!(json.get("value").is_none());
    }
    let case = root.join("failed-marker-without-metrics");
    std::fs::create_dir_all(&case).unwrap();
    std::fs::write(
        case.join("numerical-failure.json"),
        br#"{"reason":"diverged"}"#,
    )
    .unwrap();
    assert!(matches!(
        read_cfd_kpi_observation(&root, &case.join("missing-integrals.json"), 7),
        Err(Observation::Failed { .. })
    ));
    let empty = root.join("not-run");
    std::fs::create_dir_all(&empty).unwrap();
    assert!(matches!(
        read_cfd_kpi_observation(&root, &empty.join("missing-integrals.json"), 7),
        Err(Observation::Insufficient { .. })
    ));
}
