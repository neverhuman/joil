use super::*;

fn digest(c: char) -> String {
    std::iter::repeat_n(c, 64).collect()
}
fn unit(i: usize, b: f64) -> ConfirmationUnit {
    ConfirmationUnit {
        build_a1: format!("a{i}"),
        build_b: format!("b{i}"),
        build_a2: format!("c{i}"),
        fluid_batch: format!("f{i}"),
        a1: 100.,
        b,
        a2: 100.,
        duty: "d".into(),
        band: "band".into(),
        pressure_policy: "p".into(),
        decision_profile: "profile".into(),
        material_model: "mat-model".into(),
        scenarios_pass: vec![true, true],
    }
}
#[test]
fn v5_30_01_protocol_closure_and_unsigned_draft_guard() {
    let mut p = RigProtocolClosure {
        containment_reviewed: true,
        comparator_defined: true,
        pilot_pairs: 3,
        blinded_aba_blocks: 3,
        calibration_subjects: vec!["cal".into()],
        holdout_subjects: vec!["held".into()],
        calibration_batches: vec!["batch1".into()],
        holdout_batches: vec!["batch2".into()],
        owner_signed: false,
        described_as_registered_or_approved: false,
    };
    assert!(verify_rig_protocol_closure(&p).is_ok());
    p.described_as_registered_or_approved = true;
    assert!(verify_rig_protocol_closure(&p).is_err());
    p.described_as_registered_or_approved = false;
    p.holdout_batches = vec!["batch1".into()];
    assert!(verify_rig_protocol_closure(&p).is_err());
}
#[test]
fn v5_30_02_delayed_censored_stream_reduction() {
    let s = TelemetryStream {
        delay_s: 2.,
        calibrated: true,
        posthoc_filtered: false,
        points: vec![
            TelemetryPoint {
                time_s: 3.,
                value: 0.,
                lower: 0.,
                upper: 1.,
                censored: true,
            },
            TelemetryPoint {
                time_s: 4.,
                value: 0.,
                lower: 0.,
                upper: 2.,
                censored: false,
            },
        ],
    };
    let r = reduce_delayed_censored_streams(std::slice::from_ref(&s)).unwrap();
    assert_eq!(r[0][0].start_s, 1.);
    assert!(r[0][0].right_censored);
    assert_eq!(r[0][0].upper, 2.);
    let second = TelemetryStream {
        delay_s: 1.,
        calibrated: true,
        posthoc_filtered: false,
        points: vec![
            TelemetryPoint {
                time_s: 2.5,
                value: 0.,
                lower: 4.,
                upper: 5.,
                censored: false,
            },
            TelemetryPoint {
                time_s: 3.,
                value: 0.,
                lower: 3.,
                upper: 6.,
                censored: false,
            },
        ],
    };
    let aligned = reduce_delayed_censored_streams(&[s.clone(), second]).unwrap();
    assert_eq!(aligned[0][0].start_s, aligned[1][0].start_s);
    assert_eq!(aligned[0][0].end_s, aligned[1][0].end_s);
    let mut bad = s;
    bad.posthoc_filtered = true;
    assert!(reduce_delayed_censored_streams(&[bad]).is_err());
}
#[test]
fn v5_30_03_specimen_passport_deviation_reevaluation() {
    let p = SpecimenPassport {
        coupon_sha256: digest('a'),
        ct_sha256: digest('b'),
        ra_um: 2.,
        rz_um: 8.,
        orifice_mm: 1.4,
        proof_sha256: digest('c'),
        tap_sha256s: vec![digest('d')],
        as_built_subject_sha256: digest('e'),
        computational_subject_sha256: digest('f'),
    };
    assert_eq!(verify_specimen_passport_and_deviation_trigger(&p), Ok(true));
    let mut bad = p;
    bad.ct_sha256 = "bad".into();
    assert!(verify_specimen_passport_and_deviation_trigger(&bad).is_err());
}
#[test]
fn v5_30_04_preregistered_confirmation_bound_and_negatives() {
    let units = vec![unit(1, 112.), unit(2, 112.), unit(3, 112.)];
    let r = analyze_preregistered_confirmation_units(
        &units,
        2,
        "d",
        "band",
        "p",
        "profile",
        "mat-model",
    )
    .unwrap();
    assert!(r.claim_at_least_ten_pct);
    assert!((r.mean_pct - 12.).abs() < 1e-10);
    let noisy = vec![unit(1, 110.), unit(2, 110.), unit(3, 135.)];
    let r = analyze_preregistered_confirmation_units(
        &noisy,
        2,
        "d",
        "band",
        "p",
        "profile",
        "mat-model",
    )
    .unwrap();
    assert!(r.mean_pct > 10.);
    assert!(r.lower_bound_pct < 10.);
    assert!(!r.claim_at_least_ten_pct);
    let mut bad = units;
    bad[2].fluid_batch = "f1".into();
    assert!(analyze_preregistered_confirmation_units(
        &bad,
        2,
        "d",
        "band",
        "p",
        "profile",
        "mat-model"
    )
    .is_err());
}

#[test]
fn v5_30_05_private_dossier_and_allowlisted_public_export_gate() {
    let private = PrivateDossierRecord {
        classification: "private".into(),
        industrial_installation_authorized: false,
        private_fields: [("technical_record".into(), "held privately".into())].into(),
    };
    let stored = serde_json::to_string(&private).unwrap();
    assert_eq!(
        serde_json::from_str::<PrivateDossierRecord>(&stored).unwrap(),
        private
    );
    let mut fields: std::collections::BTreeMap<String, String> = PUBLIC_DOSSIER_ALLOWLIST
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                "reviewed synthetic summary".to_string(),
            )
        })
        .collect();
    fields.insert("subject_sha256".into(), digest('a'));
    let review = PublicExportReview {
        approved: true,
        reviewer: "human reviewer".into(),
        reviewed_fields: fields,
    };
    let export = verify_private_dossier_and_public_export_gate(&private, &review).unwrap();
    assert_eq!(export.fields.len(), 7);
    let mut rejected = review.clone();
    rejected
        .reviewed_fields
        .insert("duty_summary".into(), "customer site_id 7".into());
    assert_eq!(
        verify_private_dossier_and_public_export_gate(&private, &rejected),
        Err("public_export_contains_sensitive_identity_token")
    );
    let mut rejected = review.clone();
    rejected.reviewed_fields.remove("limitations");
    assert_eq!(
        verify_private_dossier_and_public_export_gate(&private, &rejected),
        Err("public_export_fields_must_match_exact_allowlist")
    );
    let mut rejected = review.clone();
    rejected.approved = false;
    assert_eq!(
        verify_private_dossier_and_public_export_gate(&private, &rejected),
        Err("approved_human_review_required")
    );
    let mut rejected = private;
    rejected.industrial_installation_authorized = true;
    assert_eq!(
        verify_private_dossier_and_public_export_gate(&rejected, &review),
        Err("software_export_gate_cannot_authorize_installation")
    );
}
