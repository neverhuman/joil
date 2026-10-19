use super::*;

// ---------------------------------------------------------------- V5-13-04

fn variant(orientation: &str, finish: &str, ra: Option<f64>) -> SurfaceVariant {
    SurfaceVariant {
        subject_sha256: sha("accepted-subject"),
        orientation: orientation.into(),
        faces: vec![
            FaceFinish {
                face_id: "bore".into(),
                finish_class: finish.into(),
                ra_um: ra,
            },
            FaceFinish {
                face_id: "outer".into(),
                finish_class: "as_printed".into(),
                ra_um: Some(8.0),
            },
        ],
    }
}

fn calibration(class: &str, points: u32) -> RaToKs {
    RaToKs {
        finish_class: class.into(),
        ra_min_um: 4.0,
        ra_max_um: 12.0,
        ks_over_ra_lower: 2.4,
        ks_over_ra_upper: 3.6,
        calibration_points: points,
    }
}

#[test]
fn v5_13_04_ra_and_equivalent_sand_roughness_are_different_and_ks_needs_a_calibration() {
    let face = FaceFinish {
        face_id: "bore".into(),
        finish_class: "as_printed".into(),
        ra_um: Some(6.5),
    };
    let calibrations = BTreeMap::from([(
        "as_printed".to_string(),
        adm(calibration("as_printed", 5), "ks-cal"),
    )]);
    match equivalent_sand_roughness(&face, &calibrations) {
        Roughness::Ks {
            lower_um,
            upper_um,
            calibration_source,
        } => {
            assert!((lower_um - 15.6).abs() < 1e-12 && (upper_um - 23.4).abs() < 1e-12);
            assert!(lower_um > 6.5, "ks is not Ra");
            assert_eq!(calibration_source, sha("ks-cal"));
        }
        other => panic!("{other:?}"),
    }
    let open = |c: &BTreeMap<String, Field<RaToKs>>, f: &FaceFinish| {
        matches!(equivalent_sand_roughness(f, c), Roughness::Unresolved(_))
    };
    // The legacy universal ks = 3.1 Ra: no calibration, a proposal, or a "universal" class all stay unresolved.
    assert!(open(&BTreeMap::new(), &face));
    let proposed = BTreeMap::from([(
        "as_printed".to_string(),
        Field::Proposed {
            value: RaToKs {
                ks_over_ra_lower: 3.1,
                ks_over_ra_upper: 3.1,
                ..calibration("as_printed", 5)
            },
            reason: "legacy 3.1 Ra".into(),
        },
    )]);
    assert!(open(&proposed, &face));
    let universal = BTreeMap::from([(
        "as_printed".to_string(),
        adm(calibration("universal", 5), "x"),
    )]);
    assert!(open(&universal, &face));
    assert!(calibration("universal", 5).check().is_err());
    // Too few points, wrong class, Ra outside the calibrated range and unmeasured Ra do not convert.
    let few = BTreeMap::from([(
        "as_printed".to_string(),
        adm(calibration("as_printed", 1), "x"),
    )]);
    assert!(open(&few, &face));
    let other_class = FaceFinish {
        finish_class: "machined".into(),
        ..face.clone()
    };
    assert!(open(&calibrations, &other_class));
    let high = FaceFinish {
        ra_um: Some(30.0),
        ..face.clone()
    };
    assert!(open(&calibrations, &high));
    let unmeasured = FaceFinish {
        ra_um: None,
        ..face
    };
    assert!(open(&calibrations, &unmeasured));
}

#[test]
fn v5_13_04_same_variant_serves_flow_structure_and_export_and_a_change_makes_them_stale() {
    let v = variant("z-up", "as_printed", Some(6.5));
    let id = v.variant_id().unwrap();
    let evals = |id: &Sha256Hex| {
        [
            EvaluationKind::Flow,
            EvaluationKind::Structure,
            EvaluationKind::Export,
        ]
        .map(|kind| Evaluation {
            kind,
            variant_id: id.clone(),
        })
    };
    assert_eq!(check_same_variant(&v, &evals(&id)).unwrap(), Verdict::Pass);
    // Face order does not change identity.
    let mut reordered = v.clone();
    reordered.faces.reverse();
    assert_eq!(reordered.variant_id().unwrap(), id);
    // Orientation, finish class or Ra each produce a new identity and stale the old evaluations.
    for changed in [
        variant("z-45", "as_printed", Some(6.5)),
        variant("z-up", "machined", Some(6.5)),
        variant("z-up", "as_printed", Some(7.0)),
    ] {
        assert_ne!(changed.variant_id().unwrap(), id);
        let verdict = check_same_variant(&changed, &evals(&id)).unwrap();
        assert_eq!(
            verdict,
            Verdict::Fail(vec![
                "stale:export".into(),
                "stale:flow".into(),
                "stale:structure".into()
            ])
        );
    }
    // Only some evaluations stale: only those are named; missing ones are unresolved.
    let mut partial = evals(&id).to_vec();
    partial[1].variant_id = sha("old-variant");
    assert_eq!(
        check_same_variant(&v, &partial).unwrap(),
        Verdict::Fail(vec!["stale:structure".into()])
    );
    assert_eq!(
        check_same_variant(&v, &evals(&id)[..2]).unwrap(),
        Verdict::Unresolved(vec!["missing:export".into()])
    );
    let mut bad = v.clone();
    bad.faces[0].ra_um = Some(-1.0);
    assert!(bad.variant_id().is_err());
    bad.faces = vec![];
    assert!(bad.check().is_err());
}

// ---------------------------------------------------------------- V5-13-05

#[test]
fn v5_13_05_coupon_plan_lists_every_required_record_with_owned_dependencies_and_no_orders() {
    let plan = CouponPlan::from_json(COUPONS).unwrap();
    assert_eq!(plan.records.len(), ALL_COUPONS.len());
    assert!(!plan.orders_placed && !plan.vendor_contacted);
    for r in &plan.records {
        assert!(!r.uncertainty_components.is_empty() && !r.acceptance_rule.is_empty());
        assert!(
            r.acceptance_threshold.admitted().is_none(),
            "no invented thresholds: {:?}",
            r.kind
        );
        assert!(r.minimum_specimens.admitted().is_none());
    }
    let mut value: serde_json::Value = serde_json::from_slice(COUPONS).unwrap();
    let mut ordered = value.clone();
    ordered["orders_placed"] = serde_json::json!(true);
    assert!(CouponPlan::from_json(&serde_json::to_vec(&ordered).unwrap()).is_err());
    value["records"].as_array_mut().unwrap().pop();
    assert!(CouponPlan::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut empty_budget: serde_json::Value = serde_json::from_slice(COUPONS).unwrap();
    empty_budget["records"][0]["uncertainty_components"] = serde_json::json!([]);
    assert!(CouponPlan::from_json(&serde_json::to_vec(&empty_budget).unwrap()).is_err());
}

fn ct(pitch: f64) -> CtBudget {
    CtBudget {
        voxel_pitch_um: pitch,
        calibration_artifact_uncertainty_um: Some(3.0),
        edge_localization_um: Some(5.0),
        repeatability_um: Some(2.0),
        calibration_artifact_source: Some(sha("artifact")),
    }
}

#[test]
fn v5_13_05_ct_accuracy_is_a_budget_not_a_voxel_pitch() {
    // Hand: u^2 = 20^2/12 + 3^2 + 5^2 + 2^2 = 71.3333; U = 2 * 8.4459 = 16.892 um.
    let full = ct(20.0);
    let u = full.dimensional_accuracy_um(2.0).unwrap();
    assert!((u - 2.0 * (20.0_f64.powi(2) / 12.0 + 9.0 + 25.0 + 4.0).sqrt()).abs() < 1e-12);
    assert!((u - 16.892).abs() < 1e-3);
    assert_eq!(
        verify_dimensional_accuracy(&full, 2.0, &adm(25.0, "req")),
        Verdict::Pass
    );
    assert!(matches!(
        verify_dimensional_accuracy(&full, 2.0, &adm(15.0, "req")),
        Verdict::Fail(_)
    ));
    // Pitch alone, however fine, cannot satisfy any requirement.
    let pitch_only = CtBudget {
        voxel_pitch_um: 1.0,
        calibration_artifact_uncertainty_um: None,
        edge_localization_um: None,
        repeatability_um: None,
        calibration_artifact_source: None,
    };
    let v = verify_dimensional_accuracy(&pitch_only, 2.0, &adm(25.0, "req"));
    assert_eq!(
        v,
        Verdict::Unresolved(vec![
            "calibration_artifact_uncertainty:not_declared".into(),
            "edge_localization:not_declared".into(),
            "repeatability:not_declared".into(),
            "calibration_artifact_source:not_declared".into(),
        ])
    );
    assert!(matches!(
        verify_dimensional_accuracy(&full, 2.0, &unresolved()),
        Verdict::Unresolved(_)
    ));
    assert!(pitch_only.dimensional_accuracy_um(0.5).is_err());
}

#[test]
fn v5_13_05_wall_detectability_needs_voxels_and_a_wall_larger_than_the_expanded_accuracy() {
    let full = ct(20.0);
    let voxels = adm(3.0, "min-voxels");
    assert_eq!(
        verify_wall_detectable(&full, 2.0, 100.0, &voxels),
        Verdict::Pass
    );
    // 50 um spans 2.5 voxels (< 3) although it is larger than the 16.9 um accuracy.
    assert_eq!(
        verify_wall_detectable(&full, 2.0, 50.0, &voxels),
        Verdict::Fail(vec!["wall_detectability:spans_fewer_than_3_voxels".into()])
    );
    let thin = verify_wall_detectable(&full, 2.0, 15.0, &voxels);
    assert!(
        matches!(&thin, Verdict::Fail(r) if r.len() == 2),
        "{thin:?}"
    );
    // A very fine pitch with no uncertainty budget still cannot declare a wall detectable.
    let pitch_only = CtBudget {
        voxel_pitch_um: 0.1,
        calibration_artifact_uncertainty_um: None,
        edge_localization_um: None,
        repeatability_um: None,
        calibration_artifact_source: None,
    };
    assert!(matches!(
        verify_wall_detectable(&pitch_only, 2.0, 100.0, &voxels),
        Verdict::Unresolved(_)
    ));
    assert!(matches!(
        verify_wall_detectable(&full, 2.0, 100.0, &unresolved()),
        Verdict::Unresolved(_)
    ));
    assert!(matches!(
        verify_wall_detectable(&full, 2.0, f64::NAN, &voxels),
        Verdict::Fail(_)
    ));
}

#[test]
fn v5_13_02_to_05_sc_dec_025_027_cad_mass_manufacturing_gates_roughness_and_ct_metrology() {
    // Synthetic accepted CAD solid: density bounds propagate through integrated volume.
    let mass =
        integrate_nominal_printed_mass(&accepted_solid(), &synthetic_passport().density_kg_m3)
            .unwrap();
    assert!(mass.nominal_printed_kg.lower < mass.nominal_printed_kg.upper);
    assert_eq!(
        mass.basis,
        "admitted_density_times_integrated_accepted_solid_volume"
    );

    // Passport thresholds admit the nominal wall/bore/orientation; a thin wall fails.
    let good = good_measurements();
    let report = run(&thresholds(), &good);
    assert_eq!(report.overall, Verdict::Pass);
    let mut thin_wall = good.clone();
    thin_wall.min_wall_m = Some(0.0009);
    assert!(matches!(
        gate(&run(&thresholds(), &thin_wall), "min_wall"),
        Verdict::Fail(_)
    ));
    let mut steep_downskin = good.clone();
    steep_downskin.max_overhang_deg_from_vertical = Some(50.0);
    assert!(matches!(
        gate(&run(&thresholds(), &steep_downskin), "internal_downskin"),
        Verdict::Fail(_)
    ));
    let mut undersized_bore = good;
    undersized_bore.printed_channels[0].min_diameter_m = Some(0.002);
    assert!(matches!(
        gate(&run(&thresholds(), &undersized_bore), "printed_channels"),
        Verdict::Fail(_)
    ));

    // Finish class maps measured Ra to a calibrated equivalent-sand ks interval.
    let face = FaceFinish {
        face_id: "bore".into(),
        finish_class: "as_printed".into(),
        ra_um: Some(6.5),
    };
    let ks = equivalent_sand_roughness(
        &face,
        &BTreeMap::from([(
            "as_printed".into(),
            adm(calibration("as_printed", 5), "ks-cal"),
        )]),
    );
    assert!(
        matches!(ks, Roughness::Ks { lower_um, upper_um, .. } if (lower_um - 15.6).abs() < 1e-12 && (upper_um - 23.4).abs() < 1e-12)
    );
    assert!(matches!(
        equivalent_sand_roughness(&face, &BTreeMap::new()),
        Roughness::Unresolved(_)
    ));
    let original_variant = variant("z-up", "as_printed", Some(6.5));
    let orientation_change = variant("z-45", "as_printed", Some(6.5));
    assert_ne!(
        original_variant.variant_id().unwrap(),
        orientation_change.variant_id().unwrap()
    );

    // Coupon preparation and CT uncertainty are distinct from voxel pitch.
    let plan = CouponPlan::from_json(COUPONS).unwrap();
    assert!(
        !plan.orders_placed && !plan.vendor_contacted && plan.records.len() == ALL_COUPONS.len()
    );
    let budget = ct(20.0);
    assert!(matches!(
        verify_dimensional_accuracy(&budget, 2.0, &adm(25.0, "req")),
        Verdict::Pass
    ));
    assert!(matches!(
        verify_dimensional_accuracy(&ct(1.0), 2.0, &adm(25.0, "req")),
        Verdict::Pass
    ));
    let pitch_only = CtBudget {
        voxel_pitch_um: 1.0,
        calibration_artifact_uncertainty_um: None,
        edge_localization_um: None,
        repeatability_um: None,
        calibration_artifact_source: None,
    };
    assert!(matches!(
        verify_dimensional_accuracy(&pitch_only, 2.0, &adm(25.0, "req")),
        Verdict::Unresolved(_)
    ));
}
