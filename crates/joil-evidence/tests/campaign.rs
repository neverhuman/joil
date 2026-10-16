use joil_evidence::{campaign::*, identity::*, observation::Finite, Verdict};
use std::collections::BTreeMap;

fn requirements() -> Requirements {
    Requirements::from_toml(include_bytes!(
        "../../../requirements/joil-hydrocyclone-v1.toml"
    ))
    .unwrap()
}

#[test]
fn runtime_objective_is_exact_preregistered_contract() {
    let r = requirements();
    let values = BTreeMap::from([
        ("cout_p95_mg_l".into(), Finite::new(25.0).unwrap()),
        ("specific_energy_j_m3".into(), Finite::new(500.0).unwrap()),
        ("grade_20um_p05".into(), Finite::new(0.9).unwrap()),
        ("d50_p95_um".into(), Finite::new(12.0).unwrap()),
        ("dp_u_pa".into(), Finite::new(30_000.0).unwrap()),
    ]);
    let executed = r.evaluate_objectives(&values).unwrap();
    assert_eq!(executed.len(), 5);
    assert!((executed[2].get() - 0.1).abs() < 1e-14);
    let hash = r.objective_hash().unwrap();
    r.verify_executed_objectives(&r.objectives, &hash).unwrap();
    let mut wrong = r.objectives.clone();
    wrong.swap(0, 1);
    assert!(r.verify_executed_objectives(&wrong, &hash).is_err());
    assert!(r
        .verify_executed_objectives(&r.objectives, &"0".repeat(64))
        .is_err());
    assert!(r.evaluate_objectives(&BTreeMap::new()).is_err());
}

#[test]
fn sandbox_does_not_admit_claim_bearing_campaign() {
    let r = requirements();
    let hash = r.objective_hash().unwrap();
    admit_campaign(CampaignMode::Sandbox, &r, &r.objectives, &hash, &[], |_| {
        Verdict::Incomplete
    })
    .unwrap();
    assert!(admit_campaign(
        CampaignMode::ClaimBearing,
        &r,
        &r.objectives,
        &hash,
        &[],
        |_| Verdict::Pass
    )
    .is_err());
    let mut fake = r;
    fake.admission = AdmissionState::Admitted;
    assert!(fake.check().is_err());
}

#[test]
fn every_mandatory_p0_gate_must_resolve() {
    // Synthetic policy exercises the admission algorithm, not real campaign approval.
    let mut r = requirements();
    r.admission = AdmissionState::Admitted;
    r.blockers.clear();
    r.thresholds.pressure_budget_pa = Some(Finite::new(40_000.0).unwrap());
    r.thresholds.energy_budget_j_per_treated_m3 = Some(Finite::new(50_000.0).unwrap());
    r.thresholds.absolute_pressure_min_pa = Some(Finite::new(100_000.0).unwrap());
    r.thresholds.duty_band_m3_h = Some([Finite::new(2.6).unwrap(), Finite::new(5.4).unwrap()]);
    let gates: Vec<_> = REQUIRED_P0_GATES
        .iter()
        .map(|id| GateReceipt {
            id: (*id).into(),
            evidence: ContentRef::from_bytes(id, "synthetic", id.as_bytes()),
        })
        .collect();
    let hash = r.objective_hash().unwrap();
    admit_campaign(
        CampaignMode::ClaimBearing,
        &r,
        &r.objectives,
        &hash,
        &gates,
        |_| Verdict::Pass,
    )
    .unwrap();
    for verdict in [
        Verdict::Fail,
        Verdict::Incomplete,
        Verdict::Invalid,
        Verdict::Reopened,
        Verdict::Revoked,
    ] {
        assert!(admit_campaign(
            CampaignMode::ClaimBearing,
            &r,
            &r.objectives,
            &hash,
            &gates,
            |_| verdict
        )
        .is_err());
    }
    assert!(admit_campaign(
        CampaignMode::ClaimBearing,
        &r,
        &r.objectives,
        &hash,
        &gates[1..],
        |_| Verdict::Pass
    )
    .is_err());
}

#[test]
fn sandbox_statistics_and_hypervolume_policy_are_explicit_and_frozen() {
    let mut r = requirements();
    assert_eq!(r.hypervolume.reference_point.len(), 5);
    assert_eq!(r.quantile_policy.min_n_p05_p95, 72);
    assert_eq!(r.quantile_policy.min_n_p01_p99, 368);
    r.hypervolume.normalization[0] = Finite::new(0.0).unwrap();
    assert!(r.check().is_err());
    r = requirements();
    r.quantile_policy.min_n_p05_p95 = 3;
    assert!(r.check().is_err());
}
