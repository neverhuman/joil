use super::*;

type ReleaseMutation<T> = (&'static str, fn(&mut T));

fn release_check<T: Clone>(
    good: &T,
    verify: impl Fn(&T) -> Result<(), PaperError>,
    mutations: &[ReleaseMutation<T>],
) {
    assert!(verify(good).is_ok());
    println!("positive: complete synthetic metadata accepted");
    for (label, mutate) in mutations {
        let mut bad = good.clone();
        mutate(&mut bad);
        assert!(verify(&bad).is_err(), "accepted negative: {label}");
        println!("negative: {label} rejected");
    }
}

#[test]
fn v5_31_01_packet_integration_completeness_rejects_unscoped_paths_and_missing_logs() {
    let good: Vec<_> = (0..31)
        .map(|i| PacketIntegrationRecord {
            packet_id: format!("V5-{i:02}"),
            tested_commit: "a".repeat(40),
            changed_paths: vec!["crates/crucible-paper/src/lib.rs".into()],
            command_exit_codes: vec![0],
            non_summary_logs: true,
            positive_checks: 1,
            negative_checks: 1,
            next_dependency: "V5-31".into(),
        })
        .collect();
    release_check(
        &good,
        |r| verify_packet_integration_completeness(r),
        &[
            ("missing packet", |r| {
                r.pop();
            }),
            ("duplicate packet", |r| r[1].packet_id = "V5-00".into()),
            ("unknown packet", |r| r[0].packet_id = "V5-31".into()),
            ("short commit", |r| r[0].tested_commit = "a".repeat(39)),
            ("nonhex commit", |r| r[0].tested_commit = "z".repeat(40)),
            ("empty scope", |r| r[0].changed_paths.clear()),
            ("private scope", |r| {
                r[0].changed_paths = vec![".private/secret".into()]
            }),
            ("runs scope", |r| {
                r[0].changed_paths = vec!["runs/log".into()]
            }),
            ("artifacts scope", |r| {
                r[0].changed_paths = vec!["artifacts/data".into()]
            }),
            ("wildcard", |r| r[0].changed_paths = vec!["*".into()]),
            ("path traversal", |r| {
                r[0].changed_paths = vec!["docs/../.private/secret".into()]
            }),
            ("missing commands", |r| r[0].command_exit_codes.clear()),
            ("failed command", |r| r[0].command_exit_codes.push(1)),
            ("missing raw logs", |r| r[0].non_summary_logs = false),
            ("missing positive", |r| r[0].positive_checks = 0),
            ("missing negative", |r| r[0].negative_checks = 0),
            ("blank dependency", |r| r[0].next_dependency = " ".into()),
        ],
    );
}

#[test]
fn v5_31_02_legacy_migration_parity_rejects_scalar_match_without_mesh_provenance() {
    let good = vec![LegacyMigrationParityRecord {
        legacy_path: "legacy/script.py".into(),
        native_replacement: "native/replacement.rs".into(),
        golden_input_sha256: "b".repeat(64),
        mesh_inputs_present: true,
        max_rel_discrepancy: 0.01,
        tolerance: 0.01,
        unavailable_input_disposition: None,
        claims_restored_provenance: true,
    }];
    release_check(
        &good,
        |r| verify_legacy_migration_parity_and_provenance(r),
        &[
            ("empty records", |r| r.clear()),
            ("scalar match without mesh", |r| {
                r[0].mesh_inputs_present = false;
                r[0].max_rel_discrepancy = 0.0;
                r[0].unavailable_input_disposition = Some("missing mesh".into());
            }),
            ("absent disposition", |r| {
                r[0].mesh_inputs_present = false;
                r[0].claims_restored_provenance = false;
            }),
            ("blank disposition", |r| {
                r[0].mesh_inputs_present = false;
                r[0].claims_restored_provenance = false;
                r[0].unavailable_input_disposition = Some(" ".into());
            }),
            ("above tolerance", |r| r[0].max_rel_discrepancy = 0.010001),
            ("NaN discrepancy", |r| r[0].max_rel_discrepancy = f64::NAN),
            ("infinite tolerance", |r| r[0].tolerance = f64::INFINITY),
            ("negative tolerance", |r| r[0].tolerance = -0.01),
            ("negative discrepancy", |r| r[0].max_rel_discrepancy = -0.01),
            ("invalid golden identity", |r| {
                r[0].golden_input_sha256.clear()
            }),
        ],
    );
    let mut absent = good;
    absent[0].mesh_inputs_present = false;
    absent[0].claims_restored_provenance = false;
    absent[0].unavailable_input_disposition =
        Some("mesh unavailable; reproduction deferred".into());
    assert!(verify_legacy_migration_parity_and_provenance(&absent).is_ok());
    println!(
        "positive: explicit unavailable-input disposition accepted without restored provenance"
    );
}

#[test]
fn v5_31_03_ci_tiers_enforce_conservative_status_and_forbid_manual_qualification_flip() {
    let suites: Vec<_> = [
        "rust-tests",
        "python-tests",
        "ui",
        "clippy",
        "fmt",
        "claims-audit",
        "baseline-controls",
        "public-tree",
    ]
    .iter()
    .map(|id| CiTierSuiteRecord {
        suite_id: (*id).into(),
        head_sha: "a".repeat(40),
        passed: true,
    })
    .collect();
    let status = ScientificStatusSummary {
        head_sha: "a".repeat(40),
        structural_qualification: false,
        qualified_baseline: false,
        scientific_admission: false,
        physical_rig_evidence_admitted: false,
    };
    release_check(
        &(suites, status),
        |(s, t)| verify_ci_tiers_and_conservative_status_derivation(s, t),
        &[
            ("missing suite", |r| {
                r.0.pop();
            }),
            ("failed suite", |r| r.0[0].passed = false),
            ("stale suite", |r| r.0[0].head_sha = "b".repeat(40)),
            ("duplicate suite", |r| r.0.push(r.0[0].clone())),
            ("invalid head", |r| r.1.head_sha = "invalid".into()),
            ("manual structural flip", |r| {
                r.1.structural_qualification = true
            }),
            ("manual baseline flip", |r| r.1.qualified_baseline = true),
            ("manual scientific flip", |r| {
                r.1.scientific_admission = true
            }),
        ],
    );
}

#[test]
fn v5_31_04_four_track_release_scope_forbids_r4_without_physical_measurements() {
    let good = FourTrackReleaseScope {
        software_engine_level: "S1".into(),
        computational_comparator_level: "C0".into(),
        discovery_method_level: "D0".into(),
        physical_candidate_level: "R0".into(),
        physical_programme_complete: false,
        gpu_fallback_disposition: "CPU reference only".into(),
        operator_fallback_disposition: "reference operator".into(),
        absent_inputs: vec!["independent rig measurements".into()],
    };
    release_check(
        &good,
        reconcile_four_track_release_scope,
        &[
            ("R4 without physical programme", |r| {
                r.physical_candidate_level = "R4".into()
            }),
            ("wrong software track", |r| {
                r.software_engine_level = "R1".into()
            }),
            ("wrong comparator track", |r| {
                r.computational_comparator_level = "S1".into()
            }),
            ("wrong discovery track", |r| {
                r.discovery_method_level = "C1".into()
            }),
            ("bare prefix", |r| r.discovery_method_level = "D".into()),
            ("unknown physical level", |r| {
                r.physical_candidate_level = "R9".into()
            }),
            ("blank GPU fallback", |r| {
                r.gpu_fallback_disposition = " ".into()
            }),
            ("blank operator fallback", |r| {
                r.operator_fallback_disposition.clear()
            }),
            ("missing absent-input inventory", |r| {
                r.absent_inputs.clear()
            }),
            ("blank absent input", |r| r.absent_inputs = vec![" ".into()]),
        ],
    );
    let mut complete = good;
    complete.physical_candidate_level = "R4".into();
    complete.physical_programme_complete = true;
    assert!(reconcile_four_track_release_scope(&complete).is_ok());
    println!(
        "positive: synthetic completed-programme metadata accepted; no real physical admission"
    );
}

#[test]
fn v5_31_05_exact_head_handoff_rejects_stale_sha_and_unreconciled_tasks() {
    let good = ExactHeadReleaseHandoff {
        pr_head_sha: "a".repeat(40),
        ci_verified_sha: "a".repeat(40),
        review_approved_sha: "a".repeat(40),
        completed_tasks: 160,
        total_tasks: 160,
        unacknowledged_items: 0,
        immutable_receipt_count: 5,
    };
    release_check(
        &good,
        verify_exact_head_release_handoff,
        &[
            ("stale CI", |r| r.ci_verified_sha = "b".repeat(40)),
            ("stale review", |r| r.review_approved_sha = "b".repeat(40)),
            ("short SHA", |r| r.pr_head_sha = "a".repeat(39)),
            ("matching nonhex SHAs", |r| {
                r.pr_head_sha = "z".repeat(40);
                r.ci_verified_sha = r.pr_head_sha.clone();
                r.review_approved_sha = r.pr_head_sha.clone();
            }),
            ("incomplete tasks", |r| r.completed_tasks = 159),
            ("wrong total", |r| {
                r.completed_tasks = 159;
                r.total_tasks = 159;
            }),
            ("unacknowledged item", |r| r.unacknowledged_items = 1),
            ("no immutable receipts", |r| r.immutable_receipt_count = 0),
        ],
    );
}

fn bundle() -> PaperBundle {
    let observation = Observation {
        id: "obs-screen-001".into(),
        subject_sha256: "a".repeat(64),
        scenario_id: "synthetic-scenario-1".into(),
        estimand_id: "screen-score".into(),
        unit: "dimensionless".into(),
        denominator: "independent_search_seed".into(),
        independent_unit_ids: vec!["seed-01".into(), "seed-02".into()],
        tier: "F1_screen".into(),
        tier_current: true,
        value: "0.42".into(),
        interpretation: "synthetic search-screen result; not physical performance".into(),
    };
    let query = ClaimQuery {
        observation_id: observation.id.clone(),
        subject_sha256: observation.subject_sha256.clone(),
        scenario_id: observation.scenario_id.clone(),
        estimand_id: observation.estimand_id.clone(),
        unit: observation.unit.clone(),
        denominator: observation.denominator.clone(),
        tier: observation.tier.clone(),
        allowed_interpretation: observation.interpretation.clone(),
    };
    PaperBundle {
        schema_version: 1,
        observations: vec![observation],
        queries: vec![query],
        figures: vec![],
        attempted_finalist_ids: vec!["synthetic-arm".into()],
        cost_quality: vec![CostQualityPoint {
            finalist_id: "synthetic-arm".into(),
            attempt_id: "attempt-1".into(),
            confirmation_data_id: "synthetic-confirmation-only".into(),
            normalized_resource_cost: "1.0".into(),
            quality_estimate: "0.0".into(),
            uncertainty_low: "-0.1".into(),
            uncertainty_high: "0.1".into(),
            independent_unit_ids: vec!["seed-2301".into(), "seed-2302".into()],
            outcome: "negative".into(),
        }],
    }
}

#[test]
fn g009_guard_figure_only_observation_cannot_bypass_revocation() {
    let mut b = bundle();
    b.queries.clear();
    b.observations[0].tier_current = false;
    b.figures.push(FigureManifest {
        id: "figure".into(),
        observation_ids: vec![b.observations[0].id.clone()],
        generator: "synthetic generator".into(),
        source_sha256: "a".repeat(64),
    });
    assert!(b.validate().is_err());
}
#[test]
fn exact_typed_query_builds() {
    assert!(bundle().validate().is_ok());
}

#[test]
fn wrong_denominator_unit_subject_and_stale_tier_fail() {
    for mutate in 0..4 {
        let mut value = bundle();
        match mutate {
            0 => value.observations[0].denominator = "parcel".into(),
            1 => value.queries[0].unit = "mg/L".into(),
            2 => value.queries[0].subject_sha256 = "b".repeat(64),
            _ => value.observations[0].tier_current = false,
        }
        assert!(value.validate().is_err(), "mutation {mutate} passed");
    }
}

#[test]
fn parcels_and_frames_cannot_inflate_independent_n() {
    let mut value = bundle();
    value.observations[0].denominator = "parcel".into();
    value.observations[0].independent_unit_ids = vec!["frame-1".into(); 1000];
    assert!(value.validate().is_err());
    value.observations[0].denominator = "independent_search_seed".into();
    value.observations[0].independent_unit_ids = vec!["seed-1".into(), "seed-1".into()];
    assert!(value.validate().is_err());
}

#[test]
fn every_attempted_finalist_and_uncertainty_interval_is_reported() {
    let mut value = bundle();
    value.attempted_finalist_ids.push("unreported".into());
    assert!(value.validate().is_err());
    value.attempted_finalist_ids.pop();
    value.cost_quality[0].uncertainty_low = "0.2".into();
    assert!(value.validate().is_err());
}

#[test]
fn near_numeric_value_is_never_claim_identity() {
    let mut value = bundle();
    value.observations[0].value = "420".into();
    assert!(value.validate().is_ok());
    value.queries[0].unit = "mg/L".into();
    assert!(value.validate().is_err());
}

#[test]
fn figure_must_reference_source_manifest_observations() {
    let mut value = bundle();
    value.figures.push(FigureManifest {
        id: "fig-1".into(),
        observation_ids: vec!["missing".into()],
        generator: "tools/build_figures.py".into(),
        source_sha256: "c".repeat(64),
    });
    assert!(matches!(
        value.validate(),
        Err(PaperError::InvalidFigure(_))
    ));
}

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
