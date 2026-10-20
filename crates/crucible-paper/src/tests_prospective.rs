use super::*;

#[test]
fn v5_28_29_30_sc_dec_041_to_052_prospective_study_arms_winner_leakage_typed_queries_and_public_bundle(
) {
    use serde_json::Value;

    let preregistration: Value = serde_json::from_str(include_str!(
        "../../../experiments/v5-study/preregistration-v1.json"
    ))
    .expect("prospective study protocol is valid JSON");
    assert!(preregistration["status"]
        .as_str()
        .unwrap()
        .contains("frozen-prospective"));
    assert_eq!(preregistration["family_policy"], "all-attempts-retained; no post-result endpoint substitution; any endpoint change creates a new protocol identity");
    let arms = preregistration["arms"].as_array().unwrap();
    for required_arm in ["R0", "R1", "E1", "E2", "A1", "A2"] {
        assert!(arms.iter().any(|arm| arm["id"] == required_arm));
    }
    assert_eq!(preregistration["independent_seeds"]["disjoint"], true);
    assert_eq!(preregistration["confirmation"]["fresh_data"], true);
    assert_eq!(
        preregistration["unavailable_l4_negative_control"]["forbidden"],
        "replace endpoint after inspecting winners"
    );

    let interventions: Value = serde_json::from_str(include_str!(
        "../../../experiments/v5-study/mechanism-interventions-v1.json"
    ))
    .expect("mechanistic intervention plan is valid JSON");
    for required_intervention in [
        "intact",
        "knockout",
        "sham",
        "rescue",
        "matched-volume",
        "matched-area",
    ] {
        assert!(interventions["interventions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["id"] == required_intervention));
    }

    let fixture_bytes = include_bytes!("../../../paper/fixtures/negative-result.json");
    let report: PaperBundle =
        serde_json::from_slice(fixture_bytes).expect("synthetic review bundle deserializes");
    report
        .validate()
        .expect("exactly bound typed queries and all attempted outcomes validate");
    assert!(report
        .cost_quality
        .iter()
        .any(|point| point.outcome == "negative"));
    assert_eq!(
        report.attempted_finalist_ids.len(),
        report
            .cost_quality
            .iter()
            .map(|point| point.finalist_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );

    let mut fuzzy = report.clone();
    fuzzy.queries[0].observation_id = "obs-synthetic-search-001-with-suffix".into();
    assert!(matches!(
        fuzzy.validate(),
        Err(PaperError::MissingObservation(_))
    ));
    let mut winner_leakage = report.clone();
    winner_leakage.cost_quality[0].confirmation_data_id = "".into();
    assert!(winner_leakage.validate().is_err());
    let mut inflated_n = report.clone();
    inflated_n.cost_quality[0].independent_unit_ids = vec!["frame-1"; 1000]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert!(inflated_n.validate().is_err());

    let public_bundle = serde_json::to_string(&report).expect("bundle serializes");
    for private_marker in ["/home/", "/Users/", ".private/", "customer", "facility"] {
        assert!(
            !public_bundle.contains(private_marker),
            "public bundle leaked private marker {private_marker}"
        );
    }
}

#[test]
fn v5_28_04_planted_error_negative_corpus_rejects_denominator_unit_subject_stale_tier_and_fuzzy_numeric_matches(
) {
    let baseline = bundle();
    let report = verify_planted_error_negative_corpus(&baseline).unwrap();
    assert!(report.baseline_valid);
    assert_eq!(report.planted_mutations_tested, 5);
    assert_eq!(report.planted_mutations_rejected, 5);
    assert_eq!(
        report.rejected_classes,
        vec![
            "wrong_denominator",
            "wrong_unit",
            "wrong_subject",
            "stale_tier",
            "fuzzy_1000x_or_0_15pct_prose_match"
        ]
    );
    let id = &baseline.queries[0].observation_id;
    assert!(verify_exact_claim_token(&baseline, id, "0.42", "dimensionless").is_ok());
    for value in ["420", "0.42063", "0.420", "NaN"] {
        assert!(verify_exact_claim_token(&baseline, id, value, "dimensionless").is_err());
    }
    assert!(verify_exact_claim_token(&baseline, id, "0.42", "mg/L").is_err());
    assert!(verify_exact_claim_token(&baseline, "absent", "0.42", "dimensionless").is_err());
    let mut bad = baseline.clone();
    bad.observations[0].tier_current = false;
    assert!(verify_planted_error_negative_corpus(&bad).is_err());
    let mut bad = baseline.clone();
    bad.queries.clear();
    assert!(verify_planted_error_negative_corpus(&bad).is_err());
    let mut bad = baseline.clone();
    bad.queries[0].estimand_id.clear();
    assert!(verify_exact_claim_token(&bad, id, "0.42", "dimensionless").is_err());
    assert_eq!(baseline, bundle());
}

#[test]
fn v5_28_02_confirmation_campaign_enforces_fresh_confirmation_seeds_and_retains_negative_finalists()
{
    let text = include_str!("../../../experiments/datasets/v5-28-02-confirmation-campaign.json");
    let ledger: ConfirmationCampaignLedger = serde_json::from_str(text).unwrap();
    let audit = verify_confirmation_campaign_json(text).unwrap();
    assert_eq!(audit.search_runs, 12);
    assert_eq!(audit.confirmation_runs, 24);
    assert_eq!(audit.finalists_retained, 12);
    assert_eq!(audit.negative_or_null_or_failed, 20);
    assert_eq!(audit.preserved_cost_quality, ledger.reported_runs);
    assert!(!audit.scientific_admission);
    for mutation in 0..13 {
        let mut bad = ledger.clone();
        match mutation {
            0 => bad.confirmation_seed_ids[0] = bad.search_seed_ids[0],
            1 => {
                bad.reported_runs.pop();
            }
            2 => {
                bad.confirmation_runs[0].confirmation_data_id =
                    bad.search_runs[0].search_data_id.clone()
            }
            3 => bad.scientific_admission = true,
            4 => {
                bad.search_runs.pop();
            }
            5 => {
                bad.confirmation_runs.pop();
            }
            6 => bad.reported_runs[0].quality_interval[1] = 0.05,
            7 => bad.reported_runs[0].outcome = "feasible".into(),
            8 => bad.confirmation_runs[0].estimand_id = "post-hoc".into(),
            9 => {
                bad.attempted_finalist_ids.pop();
            }
            10 => bad.confirmation_runs[0].cost_interval[0] = f64::NAN,
            11 => bad.frozen_before_search = false,
            _ => {
                bad.confirmation_runs[0].confirmation_data_id =
                    bad.confirmation_runs[1].confirmation_data_id.clone()
            }
        }
        assert!(
            evaluate_confirmation_campaign_without_winner_leakage(&bad).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn v5_28_01_prospective_estimand_and_all_arm_seed_matrix_are_frozen() {
    let text = include_str!("../../../experiments/datasets/v5-28-02-confirmation-campaign.json");
    let mut ledger: ConfirmationCampaignLedger = serde_json::from_str(text).unwrap();
    let protocol: serde_json::Value = serde_json::from_str(include_str!(
        "../../../experiments/v5-study/preregistration-v1.json"
    ))
    .unwrap();
    assert!(protocol["estimands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["id"] == ledger.estimand_id));
    for run in &ledger.search_runs {
        assert!(protocol["arms"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["id"] == run.arm));
        assert!(protocol["independent_seeds"]["pilot"]
            .as_array()
            .unwrap()
            .contains(&run.search_seed_id.into()));
    }
    assert!(evaluate_confirmation_campaign_without_winner_leakage(&ledger).is_ok());
    ledger.frozen_before_search = false;
    assert!(evaluate_confirmation_campaign_without_winner_leakage(&ledger).is_err());
    ledger.frozen_before_search = true;
    ledger.confirmation_runs[0].estimand_id = "substituted-after-winners".into();
    ledger.reported_runs[0] = ledger.confirmation_runs[0].clone();
    assert!(evaluate_confirmation_campaign_without_winner_leakage(&ledger).is_err());
}
