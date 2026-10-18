//! V5-13 acceptance on SYNTHETIC fixtures: material passports, mass from the accepted solid,
//! manufacturing gates, finish/orientation identity and coupon/CT metrology.
//! Synthetic admitted values only exercise decision logic; they are not physical data.
use pack_hydrocyclone_judges::manufacturing::*;
use pack_hydrocyclone_judges::mass::*;
use pack_hydrocyclone_judges::material::*;
use pack_hydrocyclone_judges::metrology::*;
use pack_hydrocyclone_judges::surface::*;
use pack_hydrocyclone_judges::{Field, Sha256Hex, Verdict};
use std::collections::BTreeMap;

const PASSPORT: &[u8] = include_bytes!("../../materials/ti64-lpbf-as-built.passport.json");
const THRESHOLDS: &[u8] =
    include_bytes!("../../materials/ti64-lpbf-as-built.process-thresholds.json");
const COUPONS: &[u8] = include_bytes!("../../materials/coupon-plan.json");

fn sha(label: &str) -> Sha256Hex {
    Sha256Hex::of_bytes(format!("synthetic-source:{label}").as_bytes())
}
fn adm<T>(value: T, label: &str) -> Field<T> {
    Field::Admitted {
        value,
        source: sha(label),
    }
}
fn unresolved<T>() -> Field<T> {
    Field::Unresolved {
        reason: "synthetic unresolved".into(),
        owner: "synthetic-owner".into(),
        blocks: vec!["physical_candidate".into()],
    }
}
fn reasons(v: &Verdict) -> Vec<String> {
    match v {
        Verdict::Pass => vec![],
        Verdict::Fail(r) | Verdict::Unresolved(r) => r.clone(),
    }
}

fn context_values() -> BTreeMap<ContextKey, String> {
    ALL_CONTEXT
        .iter()
        .map(|k| (*k, format!("synthetic {}", k.name())))
        .collect()
}

/// A fully admitted SYNTHETIC passport whose allowable evidence matches its own context.
fn synthetic_passport() -> MaterialPassport {
    let values = context_values();
    let allowable = |unit: &str, value: f64| {
        adm(
            Allowable {
                value,
                unit: unit.into(),
                evidence_context: values.clone(),
            },
            &format!("allowable-{unit}"),
        )
    };
    MaterialPassport {
        schema: "joil.hydrocyclone.material-passport.v1".into(),
        passport_id: "synthetic-passport".into(),
        synthetic: true,
        context: values
            .iter()
            .map(|(k, v)| (*k, adm(v.clone(), k.name())))
            .collect(),
        density_kg_m3: adm(
            Bounds {
                lower: 4400.0,
                upper: 4460.0,
            },
            "density",
        ),
        allowables: BTreeMap::from([
            (AllowableKind::Yield, allowable("MPa-yield", 700.0)),
            (
                AllowableKind::FatigueEndurance,
                allowable("MPa-endurance", 100.0),
            ),
            (
                AllowableKind::SourServiceEligibility,
                allowable("eligibility-index", 1.0),
            ),
        ]),
    }
}

// ---------------------------------------------------------------- V5-13-01

#[test]
fn v5_13_01_committed_draft_passport_blocks_every_claim_with_named_blockers() {
    let passport = MaterialPassport::from_json(PASSPORT).unwrap();
    for claim in [
        ClaimKind::MassDensity,
        ClaimKind::StaticStrength,
        ClaimKind::AsBuiltFatigue,
        ClaimKind::SourService,
    ] {
        let v = passport.qualify(claim);
        assert!(
            matches!(v, Verdict::Unresolved(_)),
            "{}: {v:?}",
            claim.name()
        );
    }
    let fatigue = reasons(&passport.qualify(ClaimKind::AsBuiltFatigue));
    for key in ALL_CONTEXT {
        assert!(
            fatigue.contains(&format!("context:{}:not_admitted", key.name())),
            "{key:?}"
        );
    }
    assert!(fatigue.contains(&"allowable:fatigue_endurance:not_admitted".to_string()));
    // The legacy density constant is only a proposal and cannot support a mass claim.
    assert!(passport.density_kg_m3.admitted().is_none());
    assert!(reasons(&passport.qualify(ClaimKind::MassDensity))
        .contains(&"density:not_admitted".to_string()));
}

#[test]
fn v5_13_01_fully_admitted_synthetic_passport_passes_only_with_matching_process_evidence() {
    let passport = synthetic_passport();
    for claim in [
        ClaimKind::MassDensity,
        ClaimKind::StaticStrength,
        ClaimKind::AsBuiltFatigue,
        ClaimKind::SourService,
    ] {
        assert_eq!(passport.qualify(claim), Verdict::Pass, "{}", claim.name());
    }
    // One unresolved process field blocks exactly the claims that need it.
    let mut p = synthetic_passport();
    p.context
        .insert(ContextKey::TemperatureEnvironment, unresolved());
    assert_eq!(p.qualify(ClaimKind::StaticStrength), Verdict::Pass);
    assert_eq!(
        p.qualify(ClaimKind::AsBuiltFatigue),
        Verdict::Unresolved(vec!["context:temperature_environment:not_admitted".into()])
    );
}

#[test]
fn v5_13_01_wrought_data_or_an_alloy_name_match_cannot_qualify_as_built_fatigue() {
    // SYNTHETIC wrought-bar evidence: same alloy name, different process.
    let mut wrought = context_values();
    wrought.insert(ContextKey::AlloyProductForm, "synthetic wrought bar".into());
    wrought.insert(
        ContextKey::LpbfParameters,
        "not applicable (wrought)".into(),
    );
    wrought.insert(ContextKey::Finish, "machined".into());
    let mut p = synthetic_passport();
    p.allowables.insert(
        AllowableKind::FatigueEndurance,
        adm(
            Allowable {
                value: 100.0,
                unit: "MPa-endurance".into(),
                evidence_context: wrought,
            },
            "wrought-fatigue",
        ),
    );
    let v = p.qualify(ClaimKind::AsBuiltFatigue);
    let r = reasons(&v);
    assert!(matches!(v, Verdict::Unresolved(_)));
    for key in ["alloy_product_form", "lpbf_parameters", "finish"] {
        assert!(
            r.contains(&format!(
                "allowable:fatigue_endurance:context_mismatch:{key}"
            )),
            "{key}: {r:?}"
        );
    }
    // A bare alloy-name match supplies no process context at all.
    let mut name_only = BTreeMap::new();
    name_only.insert(
        ContextKey::AlloyProductForm,
        "synthetic alloy_product_form".to_string(),
    );
    p.allowables.insert(
        AllowableKind::FatigueEndurance,
        adm(
            Allowable {
                value: 100.0,
                unit: "MPa-endurance".into(),
                evidence_context: name_only,
            },
            "name-only-fatigue",
        ),
    );
    let r = reasons(&p.qualify(ClaimKind::AsBuiltFatigue));
    assert!(
        r.contains(&"allowable:fatigue_endurance:evidence_context_missing:orientation".to_string())
    );
    assert!(r.contains(
        &"allowable:fatigue_endurance:evidence_context_missing:defect_assumptions".to_string()
    ));
    // Sour service has its own allowable; fatigue evidence does not transfer.
    p.allowables.remove(&AllowableKind::SourServiceEligibility);
    assert_eq!(
        p.qualify(ClaimKind::SourService),
        Verdict::Unresolved(vec![
            "allowable:sour_service_eligibility:not_admitted".into()
        ])
    );
}

#[test]
fn v5_13_01_sc_dec_024_public_handbook_ti64_passport_enforces_lpbf_knockdown_and_blocks_wrought_substitution(
) {
    // Synthetic handbook-style values exercise the binding rules; they are not allowables.
    let mut lpbf = synthetic_passport();
    let lpbf_context = context_values();
    lpbf.allowables.insert(
        AllowableKind::FatigueEndurance,
        adm(
            Allowable {
                value: 80.0,
                unit: "MPa-endurance (synthetic)".into(),
                evidence_context: lpbf_context.clone(),
            },
            "synthetic-lpbf-as-built-fatigue",
        ),
    );
    assert_eq!(lpbf.qualify(ClaimKind::AsBuiltFatigue), Verdict::Pass);
    assert_eq!(lpbf.qualify(ClaimKind::SourService), Verdict::Pass);

    // All seven process dimensions are explicit and bound to evidence sources.
    for key in ALL_CONTEXT {
        let Field::Admitted { value, source } = lpbf.context.get(&key).unwrap() else {
            panic!("{} must be admitted", key.name());
        };
        assert!(!value.trim().is_empty());
        assert_eq!(*source, sha(key.name()));
        assert_eq!(lpbf_context.get(&key), Some(value));
    }
    // The substituted wrought handbook value is higher but has the wrong process form.
    let mut wrought_context = lpbf_context;
    wrought_context.insert(
        ContextKey::AlloyProductForm,
        "synthetic Ti-6Al-4V wrought bar".into(),
    );
    wrought_context.insert(
        ContextKey::LpbfParameters,
        "not applicable (wrought)".into(),
    );
    let wrought_allowable = adm(
        Allowable {
            value: 120.0,
            unit: "MPa-endurance (synthetic)".into(),
            evidence_context: wrought_context,
        },
        "synthetic-wrought-fatigue",
    );
    assert!(wrought_allowable.admitted().unwrap().value > 80.0);
    lpbf.allowables
        .insert(AllowableKind::FatigueEndurance, wrought_allowable);
    let fatigue_reasons = reasons(&lpbf.qualify(ClaimKind::AsBuiltFatigue));
    assert!(fatigue_reasons
        .contains(&"allowable:fatigue_endurance:context_mismatch:alloy_product_form".into()));
    assert!(fatigue_reasons
        .contains(&"allowable:fatigue_endurance:context_mismatch:lpbf_parameters".into()));

    // A material-name-only record cannot support either process-sensitive claim.
    let mut name_only = BTreeMap::new();
    name_only.insert(ContextKey::AlloyProductForm, "Ti-6Al-4V".to_string());
    lpbf.allowables.insert(
        AllowableKind::FatigueEndurance,
        adm(
            Allowable {
                value: 120.0,
                unit: "MPa-endurance (synthetic)".into(),
                evidence_context: name_only.clone(),
            },
            "synthetic-name-only-fatigue",
        ),
    );
    lpbf.allowables.insert(
        AllowableKind::SourServiceEligibility,
        adm(
            Allowable {
                value: 1.0,
                unit: "eligibility-index".into(),
                evidence_context: name_only,
            },
            "synthetic-name-only-sour-service",
        ),
    );
    assert!(matches!(
        lpbf.qualify(ClaimKind::AsBuiltFatigue),
        Verdict::Unresolved(_)
    ));
    assert!(matches!(
        lpbf.qualify(ClaimKind::SourService),
        Verdict::Unresolved(_)
    ));
}

#[test]
fn v5_13_01_malformed_passports_are_refused() {
    let mut value: serde_json::Value = serde_json::from_slice(PASSPORT).unwrap();
    let mut with_unknown = value.clone();
    with_unknown["extra"] = serde_json::json!(1);
    assert!(MaterialPassport::from_json(&serde_json::to_vec(&with_unknown).unwrap()).is_err());
    value["context"].as_object_mut().unwrap().remove("finish");
    assert!(MaterialPassport::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut bad_density = synthetic_passport();
    bad_density.density_kg_m3 = adm(
        Bounds {
            lower: 4500.0,
            upper: 4400.0,
        },
        "reversed",
    );
    assert!(bad_density.check().is_err());
    let mut empty = synthetic_passport();
    empty
        .context
        .insert(ContextKey::Finish, adm("  ".to_string(), "blank"));
    assert!(empty.check().is_err());
    assert!(Sha256Hex::new("not-a-digest").is_err());
}

// ---------------------------------------------------------------- V5-13-02

fn accepted_solid() -> AcceptedSolid {
    let subject = sha("accepted-subject");
    let part = |id: &str, kind, volume_m3| Contribution {
        feature_id: id.into(),
        kind,
        subject_sha256: subject.clone(),
        volume_m3,
    };
    AcceptedSolid {
        subject_sha256: subject.clone(),
        solid_volume_m3: 1.2e-4,
        volume_relative_bound: 0.01,
        tetra_mesh_volume_m3: 1.195e-4,
        mesh_relative_tolerance: 0.02,
        contributions: vec![
            part("wall", FeatureKind::Wall, 1.0e-4),
            part("rib-ring", FeatureKind::Rib, 1.5e-5),
            part("internal-solid", FeatureKind::InternalSolid, 5.0e-6),
        ],
    }
}

#[test]
fn v5_13_02_mass_is_admitted_density_times_the_accepted_solid_with_hand_derived_bounds() {
    let density = adm(
        Bounds {
            lower: 4400.0,
            upper: 4460.0,
        },
        "density",
    );
    let m = integrate_nominal_printed_mass(&accepted_solid(), &density).unwrap();
    // V in [1.188e-4, 1.212e-4] m^3; mass in [0.52272, 0.540552] kg.
    assert!((m.nominal_printed_kg.lower - 0.52272).abs() < 1e-12);
    assert!((m.nominal_printed_kg.upper - 0.540552).abs() < 1e-12);
    assert_eq!(m.density_source, sha("density"));
    assert_eq!(
        m.basis,
        "admitted_density_times_integrated_accepted_solid_volume"
    );
    // Resolved ribs and internal solids are partition members of the same subject.
    assert_eq!(accepted_solid().contributions.len(), 3);
}

#[test]
fn v5_13_02_unadmitted_density_unresolved_mesh_disagreement_and_foreign_features_are_refused() {
    let solid = accepted_solid();
    for density in [
        unresolved(),
        Field::Proposed {
            value: Bounds {
                lower: 4430.0,
                upper: 4430.0,
            },
            reason: "legacy constant".into(),
        },
    ] {
        assert_eq!(
            integrate_nominal_printed_mass(&solid, &density),
            Err(MassError::Unresolved(vec!["density:not_admitted".into()]))
        );
    }
    let density = adm(
        Bounds {
            lower: 4400.0,
            upper: 4460.0,
        },
        "density",
    );
    let mut mesh = accepted_solid();
    mesh.tetra_mesh_volume_m3 = 1.3e-4;
    assert!(matches!(
        integrate_nominal_printed_mass(&mesh, &density),
        Err(MassError::MeshDisagrees { .. })
    ));
    let mut foreign = accepted_solid();
    foreign.contributions[1].subject_sha256 = sha("another-subject");
    assert_eq!(
        integrate_nominal_printed_mass(&foreign, &density),
        Err(MassError::SubjectMismatch)
    );
    let mut missing_rib = accepted_solid();
    missing_rib.contributions.remove(1);
    assert!(matches!(
        integrate_nominal_printed_mass(&missing_rib, &density),
        Err(MassError::Invalid(_))
    ));
    let mut nan = accepted_solid();
    nan.solid_volume_m3 = f64::NAN;
    assert!(matches!(
        integrate_nominal_printed_mass(&nan, &density),
        Err(MassError::Invalid(_))
    ));
}

/// Independent oracle: net solid-volume change of the corrugated wall, integrating
/// ((r0+d)^2 - r0^2)/2 over theta and z for d = envelope * 0.45 * sheet * wave (gyroid).
fn corrugation_solid_change_mm3(
    r0: f64,
    sheet: f64,
    wavelength_axial: f64,
    z0: f64,
    z1: f64,
) -> f64 {
    let (nt, nz) = (720usize, 800usize);
    let (dt, dz) = (
        2.0 * std::f64::consts::PI / nt as f64,
        (z1 - z0) / nz as f64,
    );
    let mut fluid_change = 0.0;
    for j in 0..nz {
        let z = z0 + (j as f64 + 0.5) * dz;
        let s = (z - z0) / (z1 - z0);
        let env = (std::f64::consts::PI * s).sin().powi(2);
        let kz = 2.0 * std::f64::consts::PI * (z - z0) / wavelength_axial;
        for i in 0..nt {
            let kt = 4.0 * (i as f64 + 0.5) * dt;
            let wave = 0.55 * (kt.sin() * kz.cos() + 0.5 * (kt + kz).sin());
            let d = env * 0.45 * sheet * wave;
            fluid_change += ((r0 + d).powi(2) - r0 * r0) / 2.0 * dt * dz;
        }
    }
    -fluid_change
}

#[test]
fn v5_13_02_tpms_wall_corrugation_receives_no_missing_annulus_mass_credit() {
    // Legacy phenotype.rs: annulus volume x clamp(sheet/wavelength, .10, .28) credited as solid.
    let (r_outer, rif, span, sheet, radial_wavelength) =
        (35.0_f64, 0.68_f64, 40.0_f64, 1.35_f64, 8.5_f64);
    let r_inner = r_outer * rif;
    let annulus = std::f64::consts::PI * (r_outer * r_outer - r_inner * r_inner) * span;
    let legacy_credit = annulus * (sheet / radial_wavelength).clamp(0.10, 0.28);
    assert!(
        legacy_credit > 13_000.0 && legacy_credit < 13_300.0,
        "{legacy_credit}"
    );
    // The meshed wall only corrugates: its net solid change is a few mm^3, not thousands.
    let real = corrugation_solid_change_mm3(r_outer, sheet, 9.0, 24.0, 64.0);
    assert!(real.abs() < 50.0, "{real}");
    // Accepted solid = baseline head solid + the actual (tiny) change.
    let baseline_mm3 = 128_835.217;
    let subject = sha("tpms-subject");
    let solid = AcceptedSolid {
        subject_sha256: subject.clone(),
        solid_volume_m3: (baseline_mm3 + real) * 1e-9,
        volume_relative_bound: 0.01,
        tetra_mesh_volume_m3: (baseline_mm3 + real) * 1e-9,
        mesh_relative_tolerance: 0.02,
        contributions: vec![Contribution {
            feature_id: "head".into(),
            kind: FeatureKind::Wall,
            subject_sha256: subject.clone(),
            volume_m3: (baseline_mm3 + real) * 1e-9,
        }],
    };
    let credited = (baseline_mm3 + legacy_credit) * 1e-9;
    // Declared as an analytic estimate (the legacy basis): refused outright.
    let legacy = ClaimedSolidVolume {
        basis: VolumeBasis::AnalyticEstimate("analytic_estimate_v2".into()),
        volume_m3: credited,
    };
    assert_eq!(
        check_claimed_volume(&legacy, &solid),
        Err(MassError::UnsupportedCredit(
            "analytic_estimate_is_not_integrated_from_the_accepted_solid".into()
        ))
    );
    // Relabelled as the accepted solid but still carrying the credit: refused numerically.
    let relabelled = ClaimedSolidVolume {
        basis: VolumeBasis::AcceptedSolid(subject.clone()),
        volume_m3: credited,
    };
    assert_eq!(
        check_claimed_volume(&relabelled, &solid),
        Err(MassError::UnsupportedCredit(
            "claimed_volume_differs_from_the_accepted_solid".into()
        ))
    );
    // The honest claim is accepted; a foreign subject is refused.
    let honest = ClaimedSolidVolume {
        basis: VolumeBasis::AcceptedSolid(subject),
        volume_m3: solid.solid_volume_m3,
    };
    assert_eq!(check_claimed_volume(&honest, &solid), Ok(()));
    let foreign = ClaimedSolidVolume {
        basis: VolumeBasis::AcceptedSolid(sha("other")),
        volume_m3: solid.solid_volume_m3,
    };
    assert_eq!(
        check_claimed_volume(&foreign, &solid),
        Err(MassError::SubjectMismatch)
    );
}

#[test]
fn v5_13_02_printed_support_machined_and_weighed_mass_stay_separate() {
    let iv = |lower, upper| Interval { lower, upper };
    let mut ledger = MassLedger {
        nominal_printed_kg: iv(0.52, 0.54),
        supports_allowances_kg: None,
        machined_removed_kg: None,
        weighed: Some(WeighedMass {
            kg: 0.55,
            uncertainty_kg: 0.002,
            condition: WeighCondition::AsPrintedWithSupports,
            scale_source: sha("scale"),
        }),
    };
    // Unplanned terms are unknown, not zero.
    assert_eq!(ledger.as_printed_with_supports_kg(), None);
    assert_eq!(
        ledger.weighed_consistency(),
        Verdict::Unresolved(vec!["weighed_mass:comparison_term_not_planned".into()])
    );
    ledger.supports_allowances_kg = Some(iv(0.01, 0.02));
    ledger.machined_removed_kg = Some(iv(0.004, 0.006));
    let with_supports = ledger.as_printed_with_supports_kg().unwrap();
    assert!(
        (with_supports.lower - 0.53).abs() < 1e-12 && (with_supports.upper - 0.56).abs() < 1e-12
    );
    let finished = ledger.finished_kg().unwrap();
    assert!((finished.lower - 0.514).abs() < 1e-12 && (finished.upper - 0.536).abs() < 1e-12);
    assert_eq!(ledger.nominal_printed_kg, iv(0.52, 0.54));
    assert_eq!(ledger.weighed_consistency(), Verdict::Pass);
    ledger.weighed.as_mut().unwrap().kg = 0.60;
    assert_eq!(
        ledger.weighed_consistency(),
        Verdict::Fail(vec!["weighed_mass:outside_expected_bounds".into()])
    );
    // A finished-condition weighing is compared with the finished expectation, not the printed one.
    ledger.weighed = Some(WeighedMass {
        kg: 0.55,
        uncertainty_kg: 0.001,
        condition: WeighCondition::Finished,
        scale_source: sha("scale"),
    });
    assert_eq!(
        ledger.weighed_consistency(),
        Verdict::Fail(vec!["weighed_mass:outside_expected_bounds".into()])
    );
    ledger.weighed = None;
    assert_eq!(
        ledger.weighed_consistency(),
        Verdict::Unresolved(vec!["weighed_mass:not_measured".into()])
    );
}

// ---------------------------------------------------------------- V5-13-03

fn thresholds() -> ProcessThresholds {
    ProcessThresholds {
        build_volume_m: adm([0.25, 0.25, 0.30], "build"),
        min_wall_m: adm(0.001, "wall"),
        max_overhang_deg_from_vertical: adm(45.0, "overhang"),
        max_unsupported_span_m: adm(0.005, "span"),
        min_support_access_m: adm(0.004, "support"),
        min_printed_passage_m: adm(0.003, "passage"),
        min_powder_escape_clearance_m: adm(0.0025, "escape"),
        tool_clearance_margin_m: adm(0.0002, "margin"),
    }
}

fn reamer(swept: Option<f64>) -> ToolAccess {
    ToolAccess {
        tool_id: "reamer-a".into(),
        tool_radius_m: 0.0015,
        swept_clear_radius_m: swept,
    }
}

fn good_measurements() -> SubjectMeasurements {
    SubjectMeasurements {
        subject_sha256: sha("accepted-subject"),
        bounding_box_m: Some([0.1, 0.1, 0.2]),
        min_wall_m: Some(0.0015),
        max_overhang_deg_from_vertical: Some(40.0),
        max_unsupported_span_m: Some(0.004),
        supports: vec![SupportRegion {
            id: "support-1".into(),
            access_diameter_m: Some(0.006),
        }],
        printed_channels: vec![PrintedChannel {
            id: "inlet-channel".into(),
            min_diameter_m: Some(0.004),
        }],
        cavities: vec![Cavity {
            id: "cavity-1".into(),
            resolved: true,
            escape_clearance_diameter_m: Some(0.003),
            reamer: None,
        }],
        machined_features: vec![MachinedFeature {
            id: "overflow-orifice".into(),
            plan: Some(AccessPlan {
                process_plan_source: sha("plan"),
                access: reamer(Some(0.003)),
            }),
        }],
    }
}

fn run(t: &ProcessThresholds, m: &SubjectMeasurements) -> GateReport {
    evaluate(&sha("accepted-subject"), t, m).unwrap()
}
fn gate<'a>(r: &'a GateReport, name: &str) -> &'a Verdict {
    &r.gates.iter().find(|(n, _)| n == name).unwrap().1
}

#[test]
fn v5_13_03_good_subject_passes_every_gate_against_admitted_passport_thresholds() {
    let report = run(&thresholds(), &good_measurements());
    assert_eq!(report.overall, Verdict::Pass, "{:?}", report.gates);
    assert_eq!(report.gates.len(), 8);
}

#[test]
fn v5_13_03_committed_draft_thresholds_are_all_unresolved_never_defaulted() {
    let t: ProcessThresholds = serde_json::from_slice(THRESHOLDS).unwrap();
    let report = run(&t, &good_measurements());
    assert!(
        matches!(report.overall, Verdict::Unresolved(_)),
        "{:?}",
        report.overall
    );
    for (name, verdict) in &report.gates {
        assert!(
            matches!(verdict, Verdict::Unresolved(_)),
            "{name}: {verdict:?}"
        );
    }
    // Printed minimum passage is not silently the 1.4 mm machined value or the 3 mm draft.
    assert!(t.min_printed_passage_m.admitted().is_none());
}

#[test]
fn v5_13_03_connected_cavity_without_finite_escape_or_reamer_access_fails() {
    let mut m = good_measurements();
    m.cavities = vec![Cavity {
        id: "trap".into(),
        resolved: true,
        escape_clearance_diameter_m: None,
        reamer: None,
    }];
    let report = run(&thresholds(), &m);
    let v = gate(&report, "powder_escape");
    assert_eq!(
        v,
        &Verdict::Fail(vec![
            "cavity:trap:no_finite_clearance_escape_path".into(),
            "cavity:trap:no_reamer_access".into()
        ])
    );
    assert!(matches!(report.overall, Verdict::Fail(_)));
    // A too-narrow escape fails; a clear reamer path rescues it; a blocked reamer does not.
    m.cavities = vec![Cavity {
        id: "slot".into(),
        resolved: true,
        escape_clearance_diameter_m: Some(0.001),
        reamer: None,
    }];
    assert!(matches!(
        gate(&run(&thresholds(), &m), "powder_escape"),
        Verdict::Fail(_)
    ));
    m.cavities[0].reamer = Some(reamer(Some(0.0031)));
    assert_eq!(
        gate(&run(&thresholds(), &m), "powder_escape"),
        &Verdict::Pass
    );
    m.cavities[0].reamer = Some(reamer(Some(0.0016)));
    assert!(matches!(
        gate(&run(&thresholds(), &m), "powder_escape"),
        Verdict::Fail(_)
    ));
    // An unresolved cavity is unknown, not a pass.
    m.cavities = vec![Cavity {
        id: "unseen".into(),
        resolved: false,
        escape_clearance_diameter_m: None,
        reamer: None,
    }];
    assert!(matches!(
        gate(&run(&thresholds(), &m), "powder_escape"),
        Verdict::Unresolved(_)
    ));
}

#[test]
fn v5_13_03_machined_orifice_needs_its_own_plan_and_its_exception_never_qualifies_printed_channels()
{
    let mut m = good_measurements();
    // A 1.4 mm printed channel fails the admitted 3 mm printed minimum even though a 1.4 mm
    // machined orifice with a plan passes.
    m.printed_channels.push(PrintedChannel {
        id: "small-printed".into(),
        min_diameter_m: Some(0.0014),
    });
    m.machined_features[0]
        .plan
        .as_mut()
        .unwrap()
        .access
        .swept_clear_radius_m = Some(0.0018);
    let report = run(&thresholds(), &m);
    assert_eq!(gate(&report, "machined_access"), &Verdict::Pass);
    assert!(
        matches!(gate(&report, "printed_channels"), Verdict::Fail(r) if r.iter().any(|x| x.starts_with("printed_channel:small-printed")))
    );
    // No explicit plan: fail. Blocked swept volume: fail.
    m.machined_features[0].plan = None;
    assert_eq!(
        gate(&run(&thresholds(), &m), "machined_access"),
        &Verdict::Fail(vec![
            "machined:overflow-orifice:no_explicit_process_access_plan".into()
        ])
    );
    m.machined_features[0].plan = Some(AccessPlan {
        process_plan_source: sha("plan"),
        access: reamer(Some(0.0010)),
    });
    assert!(matches!(
        gate(&run(&thresholds(), &m), "machined_access"),
        Verdict::Fail(_)
    ));
}

#[test]
fn v5_13_03_each_geometric_limit_fails_unmeasured_is_unresolved_and_wrong_subject_is_refused() {
    let t = thresholds();
    let mut m = good_measurements();
    m.bounding_box_m = Some([0.1, 0.1, 0.35]);
    assert!(matches!(gate(&run(&t, &m), "build_fit"), Verdict::Fail(_)));
    let mut m = good_measurements();
    m.min_wall_m = Some(0.0005);
    assert!(matches!(gate(&run(&t, &m), "min_wall"), Verdict::Fail(_)));
    let mut m = good_measurements();
    m.max_overhang_deg_from_vertical = Some(60.0);
    assert!(matches!(
        gate(&run(&t, &m), "internal_downskin"),
        Verdict::Fail(_)
    ));
    let mut m = good_measurements();
    m.max_unsupported_span_m = Some(0.02);
    assert!(matches!(
        gate(&run(&t, &m), "unsupported_span"),
        Verdict::Fail(_)
    ));
    let mut m = good_measurements();
    m.supports[0].access_diameter_m = Some(0.002);
    assert!(matches!(
        gate(&run(&t, &m), "support_accessibility"),
        Verdict::Fail(_)
    ));
    let mut m = good_measurements();
    m.min_wall_m = None;
    m.bounding_box_m = None;
    let report = run(&t, &m);
    assert!(matches!(gate(&report, "min_wall"), Verdict::Unresolved(_)));
    assert!(matches!(gate(&report, "build_fit"), Verdict::Unresolved(_)));
    let mut m = good_measurements();
    m.subject_sha256 = sha("another-subject");
    assert!(evaluate(&sha("accepted-subject"), &t, &m).is_err());
    let mut m = good_measurements();
    m.supports.push(m.supports[0].clone());
    assert!(evaluate(&sha("accepted-subject"), &t, &m).is_err());
    let mut m = good_measurements();
    m.min_wall_m = Some(f64::NAN);
    assert!(evaluate(&sha("accepted-subject"), &t, &m).is_err());
}

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
