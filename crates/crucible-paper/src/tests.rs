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

pub(super) fn bundle() -> PaperBundle {
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

#[path = "tests_prospective.rs"]
mod tests_prospective;
