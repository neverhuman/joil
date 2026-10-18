//! V5-20 judge tests: wet dynamics, finite-life fatigue, as-built identity and correlated bounds.
//! Every number here is a SYNTHETIC fixture checked against a hand-derived oracle; none is a
//! physical qualification.
use pack_hydrocyclone_judges::as_built::{
    build_partition, corner_bounds, dimensional_claim, judge_registration, p95_claim,
    roughness_claim, BoundedFactor, FeatureMeasurement, Masses, Registration, SampleBasis,
    SubjectIdentity, Uncertainty,
};
use pack_hydrocyclone_judges::dynamics::{
    check_mesh_convergence, cross_check, screen_wet_mode, Cantilever, ClaimEffect, MeshSeries,
    PressureSpectrum, PsdBin, RouteResult, SupportKind, WetModeInput,
};
use pack_hydrocyclone_judges::fatigue::{judge_finite_life, BasquinCurve, CycleBin, FatigueInput};
use pack_hydrocyclone_judges::material::{ContextKey, MaterialPassport, ALL_CONTEXT};
use pack_hydrocyclone_judges::metrology::CtBudget;
use pack_hydrocyclone_judges::surface::{FaceFinish, RaToKs, Roughness};
use pack_hydrocyclone_judges::{Field, Sha256Hex, Verdict};
use std::collections::BTreeMap;

fn admitted<T>(value: T, tag: &str) -> Field<T> {
    Field::Admitted {
        value,
        source: Sha256Hex::of_bytes(tag.as_bytes()),
    }
}

fn proposed<T>(value: T) -> Field<T> {
    Field::Proposed {
        value,
        reason: "test proposal, not admitted".into(),
    }
}

fn reasons(verdict: &Verdict) -> Vec<String> {
    match verdict {
        Verdict::Pass => Vec::new(),
        Verdict::Fail(r) | Verdict::Unresolved(r) => r.clone(),
    }
}

fn mentions(verdict: &Verdict, needle: &str) -> bool {
    reasons(verdict).iter().any(|r| r.contains(needle))
}

// ---------------------------------------------------------------- V5-20-01 dynamics

/// Unit clamped-free beam: L = EI = m = 1, so f = beta_1^2 / (2 pi) = 0.5595912 Hz.
const UNIT_DRY_FREQUENCY_HZ: f64 = 0.5595912;

fn unit_beam(added: Field<f64>, support: Field<SupportKind>) -> Cantilever {
    Cantilever {
        length_m: 1.0,
        bending_stiffness_nm2: 1.0,
        mass_per_length_kg_m: 1.0,
        added_mass_kg_m: added,
        support,
    }
}

fn clamped() -> Field<SupportKind> {
    admitted(SupportKind::ClampedFree, "support-record")
}

/// Log-spaced-looking but uniform 0.01 f grid from 0.1 f to 10 f with a flat PSD of 1 Pa^2/Hz.
fn broadband_around(f: f64, psd_in_band: f64) -> PressureSpectrum {
    let bins = (10..=1000)
        .map(|k| {
            let hz = f * f64::from(k) * 0.01;
            let psd = if (hz - f).abs() <= 0.02 * f {
                psd_in_band
            } else {
                1.0
            };
            PsdBin {
                hz,
                psd_pa2_hz: psd,
            }
        })
        .collect();
    PressureSpectrum { bins }
}

fn wet_input(cantilever: Cantilever, spectrum: Field<PressureSpectrum>) -> WetModeInput {
    WetModeInput {
        cantilever,
        damping_ratio: admitted(0.02, "damping-source"),
        spectrum,
        max_band_energy_fraction: admitted(0.01, "band-limit-source"),
    }
}

#[test]
fn v5_20_01_cantilever_oracle_matches_clamped_free_closed_form() {
    let f = beam_frequency(admitted(0.0, "no-added-mass"));
    assert!(
        ((f - UNIT_DRY_FREQUENCY_HZ) / UNIT_DRY_FREQUENCY_HZ).abs() < 1e-5,
        "dry frequency {f}"
    );
}

fn beam_frequency(added: Field<f64>) -> f64 {
    unit_beam(added, clamped())
        .first_wet_frequency_hz()
        .expect("admitted fixture")
}

#[test]
fn v5_20_01_added_mass_lowers_frequency_by_square_root_of_total_mass() {
    let dry = beam_frequency(admitted(0.0, "no-added-mass"));
    let wet = beam_frequency(admitted(3.0, "fluid-added-mass"));
    assert!(((wet / dry) - 0.5).abs() < 1e-12, "ratio {}", wet / dry);
}

#[test]
fn v5_20_01_length_scales_frequency_inversely_with_square_of_length() {
    let mut long = unit_beam(admitted(0.0, "no-added-mass"), clamped());
    long.length_m = 2.0;
    let f_long = long.first_wet_frequency_hz().expect("admitted fixture");
    let f_unit = beam_frequency(admitted(0.0, "no-added-mass"));
    assert!(((f_long / f_unit) - 0.25).abs() < 1e-12);
}

#[test]
fn v5_20_01_unadmitted_added_mass_or_support_blocks_the_frequency() {
    let proposed_mass = unit_beam(proposed(3.0), clamped());
    let reasons = proposed_mass
        .first_wet_frequency_hz()
        .expect_err("unadmitted added mass");
    assert!(reasons.iter().any(|r| r == "added_mass:not_admitted"));
    let other_support = unit_beam(
        admitted(0.0, "no-added-mass"),
        admitted(SupportKind::Other, "support-record"),
    );
    let reasons = other_support
        .first_wet_frequency_hz()
        .expect_err("support without oracle");
    assert!(reasons.iter().any(|r| r == "support:no_closed_form_oracle"));
}

#[test]
fn v5_20_01_mesh_series_that_converges_passes() {
    let series = MeshSeries {
        element_size_m: vec![0.01, 0.005, 0.0025],
        mode_hz: vec![100.0, 100.5, 100.6],
    };
    assert_eq!(
        check_mesh_convergence(&series, &admitted(2e-3, "mesh-tolerance")),
        Verdict::Pass
    );
}

#[test]
fn v5_20_01_mesh_series_that_diverges_under_refinement_fails() {
    let series = MeshSeries {
        element_size_m: vec![0.01, 0.005, 0.0025],
        mode_hz: vec![100.0, 110.0, 130.0],
    };
    let verdict = check_mesh_convergence(&series, &admitted(2e-3, "mesh-tolerance"));
    assert!(matches!(verdict, Verdict::Fail(_)));
    assert!(mentions(&verdict, "changes_grow_under_refinement"));
}

#[test]
fn v5_20_01_broadband_spectrum_below_admitted_band_limit_passes_screen() {
    let f = beam_frequency(admitted(0.0, "no-added-mass"));
    let screen = screen_wet_mode(&wet_input(
        unit_beam(admitted(0.0, "no-added-mass"), clamped()),
        admitted(broadband_around(f, 1.0), "pressure-spectrum-source"),
    ));
    assert_eq!(screen.verdict, Verdict::Pass, "{:?}", screen.verdict);
    let fraction = screen.band_energy_fraction.expect("computed fraction");
    assert!(fraction > 0.0 && fraction < 0.01, "fraction {fraction}");
}

#[test]
fn v5_20_01_band_energy_above_admitted_limit_fails_forced_response_screen() {
    let f = beam_frequency(admitted(0.0, "no-added-mass"));
    let screen = screen_wet_mode(&wet_input(
        unit_beam(admitted(0.0, "no-added-mass"), clamped()),
        admitted(broadband_around(f, 1000.0), "pressure-spectrum-source"),
    ));
    assert!(matches!(screen.verdict, Verdict::Fail(_)));
    assert!(mentions(
        &screen.verdict,
        "forced_response:band_energy_fraction"
    ));
}

#[test]
fn v5_20_01_copied_418hz_single_frequency_cannot_authorize_dynamics() {
    let single_bin = PressureSpectrum {
        bins: vec![PsdBin {
            hz: 418.0,
            psd_pa2_hz: 1.0,
        }],
    };
    let screen = screen_wet_mode(&wet_input(
        unit_beam(admitted(0.0, "no-added-mass"), clamped()),
        admitted(single_bin, "copied-418hz-field"),
    ));
    assert!(matches!(screen.verdict, Verdict::Unresolved(_)));
    assert!(mentions(
        &screen.verdict,
        "spectrum:invalid_or_single_frequency"
    ));
    assert!(mentions(
        &screen.verdict,
        "spectrum:does_not_cover_mode_band"
    ));
    assert_eq!(screen.mode_hz, None);
}

#[test]
fn v5_20_01_proposed_spectrum_or_support_is_unresolved_not_pass() {
    let f = beam_frequency(admitted(0.0, "no-added-mass"));
    let proposed_spectrum = screen_wet_mode(&wet_input(
        unit_beam(admitted(0.0, "no-added-mass"), clamped()),
        proposed(broadband_around(f, 1.0)),
    ));
    assert!(mentions(
        &proposed_spectrum.verdict,
        "spectrum:not_admitted"
    ));
    let proposed_support = screen_wet_mode(&wet_input(
        unit_beam(
            admitted(0.0, "no-added-mass"),
            proposed(SupportKind::ClampedFree),
        ),
        admitted(broadband_around(f, 1.0), "pressure-spectrum-source"),
    ));
    assert!(mentions(&proposed_support.verdict, "support:not_admitted"));
    assert!(!proposed_support.verdict.is_pass());
}

// ---------------------------------------------------------------- V5-20-02 fatigue

fn context_value(alloy: &str, key: ContextKey) -> String {
    format!("{alloy}:{}", key.name())
}

fn context_map(alloy: &str) -> BTreeMap<ContextKey, String> {
    ALL_CONTEXT
        .iter()
        .map(|k| (*k, context_value(alloy, *k)))
        .collect()
}

fn passport(alloy: &str, fatigue_admitted: bool) -> MaterialPassport {
    let source = Sha256Hex::of_bytes(b"passport-source").as_str().to_string();
    let context: serde_json::Map<String, serde_json::Value> = ALL_CONTEXT
        .iter()
        .map(|k| {
            (
                k.name().to_string(),
                serde_json::json!({
                    "status": "admitted",
                    "value": context_value(alloy, *k),
                    "source": source,
                }),
            )
        })
        .collect();
    let fatigue = if fatigue_admitted {
        serde_json::json!({
            "status": "admitted",
            "value": {
                "value": 5.0e8,
                "unit": "Pa",
                "evidence_context": context_map(alloy),
            },
            "source": source,
        })
    } else {
        serde_json::json!({
            "status": "unresolved",
            "reason": "no as-built fatigue data",
            "owner": "materials_engineer",
            "blocks": ["physical_candidate"],
        })
    };
    let document = serde_json::json!({
        "schema": "joil.hydrocyclone.material-passport.v1",
        "passport_id": "test-passport-v5-20-02",
        "synthetic": true,
        "context": context,
        "density_kg_m3": {
            "status": "unresolved",
            "reason": "not used by the fatigue claim",
            "owner": "materials_engineer",
            "blocks": ["physical_candidate"],
        },
        "allowables": { "fatigue_endurance": fatigue },
    });
    MaterialPassport::from_json(document.to_string().as_bytes()).expect("valid fixture passport")
}

fn curve(alloy: &str) -> BasquinCurve {
    BasquinCurve {
        sigma_f_prime_pa: 1.0e9,
        exponent_b: -0.1,
        log10_life_sigma: 0.2,
        evidence_context: context_map(alloy),
    }
}

fn single_bin(amplitude_pa: f64, mean_pa: f64) -> Vec<CycleBin> {
    vec![CycleBin {
        amplitude_pa,
        mean_pa,
        cycles: 1.0,
    }]
}

fn fatigue_input(alloy: &str, bins: Vec<CycleBin>, target_cycles: f64) -> FatigueInput {
    FatigueInput {
        curve: admitted(curve(alloy), "curve-source"),
        ultimate_pa: admitted(1.0e9, "ultimate-source"),
        coverage_k: admitted(2.0, "coverage-source"),
        spectrum: admitted(bins, "cycle-count-source"),
        target_cycles,
        runout_cycles: 1.0e7,
    }
}

/// N = 0.5 * (0.3)^(-10) = 84675.1 cycles for a 300 MPa amplitude with sigma_f' = 1 GPa, b = -0.1.
const PREDICTED_CYCLES: f64 = 84_675.1;

#[test]
fn v5_20_02_finite_life_pass_names_passport_and_is_not_infinite_life() {
    let report = judge_finite_life(
        &passport("A", true),
        &fatigue_input("A", single_bin(3.0e8, 0.0), 3.0e4),
    );
    assert_eq!(report.verdict, Verdict::Pass, "{:?}", report.verdict);
    let predicted = report.predicted_cycles.expect("predicted life");
    assert!(((predicted - PREDICTED_CYCLES) / PREDICTED_CYCLES).abs() < 1e-5);
    let lower = report.lower_bound_cycles.expect("lower bound");
    assert!(lower < predicted, "scatter must lower the bound");
    let statement = report.statement.expect("finite-life statement");
    assert!(statement.starts_with("finite life:"));
    assert!(statement.contains("test-passport-v5-20-02"));
    assert!(statement.contains("not infinite life"));
}

#[test]
fn v5_20_02_mean_stress_treatment_shortens_predicted_life_by_goodman_factor() {
    let zero_mean = judge_finite_life(
        &passport("A", true),
        &fatigue_input("A", single_bin(3.0e8, 0.0), 1.0),
    )
    .predicted_cycles
    .expect("zero-mean life");
    // Goodman: 3e8 / (1 - 5e8/1e9) = 6e8, so life falls by (0.6/0.3)^10 = 2^10.
    let tensile_mean = judge_finite_life(
        &passport("A", true),
        &fatigue_input("A", single_bin(3.0e8, 5.0e8), 1.0),
    )
    .predicted_cycles
    .expect("mean-stress life");
    assert!((tensile_mean / zero_mean - 1.0 / 1024.0).abs() < 1e-9);
}

#[test]
fn v5_20_02_target_at_runout_is_screen_never_infinite_life() {
    let report = judge_finite_life(
        &passport("A", true),
        &fatigue_input("A", single_bin(3.0e8, 0.0), 1.0e7),
    );
    assert!(matches!(report.verdict, Verdict::Unresolved(_)));
    assert!(mentions(
        &report.verdict,
        "at_or_above_runout_is_screen_not_finite_life"
    ));
    assert_eq!(report.statement, None);
}

#[test]
fn v5_20_02_lower_bound_straddling_target_is_unresolved() {
    let report = judge_finite_life(
        &passport("A", true),
        &fatigue_input("A", single_bin(3.0e8, 0.0), 5.0e4),
    );
    assert!(mentions(
        &report.verdict,
        "life:lower_bound_straddles_target"
    ));
}

#[test]
fn v5_20_02_predicted_life_below_target_fails() {
    let report = judge_finite_life(
        &passport("A", true),
        &fatigue_input("A", single_bin(3.0e8, 0.0), 1.0e6),
    );
    assert!(matches!(report.verdict, Verdict::Fail(_)));
    assert!(mentions(&report.verdict, "life:predicted_below_target"));
}

#[test]
fn v5_20_02_curve_from_other_chemistry_cannot_transfer() {
    let report = judge_finite_life(
        &passport("B", true),
        &fatigue_input("A", single_bin(3.0e8, 0.0), 3.0e4),
    );
    assert!(matches!(report.verdict, Verdict::Unresolved(_)));
    assert!(mentions(
        &report.verdict,
        "curve:context_mismatch:alloy_product_form"
    ));
    assert_eq!(report.statement, None);
}

#[test]
fn v5_20_02_passport_without_admitted_fatigue_allowable_blocks_statement() {
    let report = judge_finite_life(
        &passport("A", false),
        &fatigue_input("A", single_bin(3.0e8, 0.0), 3.0e4),
    );
    assert!(mentions(
        &report.verdict,
        "allowable:fatigue_endurance:not_admitted"
    ));
    assert_eq!(report.statement, None);
}

#[test]
fn v5_20_02_unadmitted_coverage_factor_is_unresolved() {
    let mut input = fatigue_input("A", single_bin(3.0e8, 0.0), 3.0e4);
    input.coverage_k = proposed(2.0);
    let report = judge_finite_life(&passport("A", true), &input);
    assert!(mentions(&report.verdict, "coverage_k:not_admitted"));
}

#[test]
fn v5_20_02_representable_tiny_amplitude_keeps_finite_life_pass() {
    // 1e-3 Pa / 1e9 Pa = 1e-12; N = 0.5 * (1e-12)^(-10) = 5e119, finite and far above the 1 cycle target.
    let report = judge_finite_life(
        &passport("A", true),
        &fatigue_input("A", single_bin(1.0e-3, 0.0), 1.0),
    );
    assert_eq!(report.verdict, Verdict::Pass, "{:?}", report.verdict);
    let predicted = report.predicted_cycles.expect("predicted life");
    assert!(
        ((predicted - 5.0e119) / 5.0e119).abs() < 1e-9,
        "{predicted}"
    );
}

#[test]
fn v5_20_02_finite_tiny_amplitude_never_passes_on_infinite_life() {
    // 1e-300 Pa / 1e9 Pa = 1e-309 is subnormal; (1e-309)^(-10) overflows, so N = +inf.
    let report = judge_finite_life(
        &passport("A", true),
        &fatigue_input("A", single_bin(1.0e-300, 0.0), 3.0e4),
    );
    assert!(!report.verdict.is_pass(), "{:?}", report.verdict);
    assert!(report.predicted_cycles.is_none_or(f64::is_finite));
    assert!(report.lower_bound_cycles.is_none_or(f64::is_finite));
}

#[test]
fn v5_20_02_subnormal_cycle_count_never_passes_on_underflowed_damage() {
    // N = 84675 cycles at 3e8 Pa, so a 1e-320 cycle count gives damage 1e-325, which underflows to 0.0.
    let mut input = fatigue_input("A", single_bin(3.0e8, 0.0), 3.0e4);
    input.spectrum = admitted(
        vec![CycleBin {
            amplitude_pa: 3.0e8,
            mean_pa: 0.0,
            cycles: 1.0e-320,
        }],
        "cycle-count-source",
    );
    let report = judge_finite_life(&passport("A", true), &input);
    assert!(!report.verdict.is_pass(), "{:?}", report.verdict);
    assert!(report.predicted_cycles.is_none_or(f64::is_finite));
    assert!(report.lower_bound_cycles.is_none_or(f64::is_finite));
}

// ---------------------------------------------------------------- V5-20-03 as-built

fn identity() -> SubjectIdentity {
    SubjectIdentity {
        nominal_sha256: Sha256Hex::of_bytes(b"nominal-design"),
        as_built_sha256: Sha256Hex::of_bytes(b"as-built-scan-1"),
    }
}

fn feature(id: &str, nominal: f64, as_built: f64, threshold: Field<f64>) -> FeatureMeasurement {
    FeatureMeasurement {
        feature_id: id.into(),
        nominal_mm: nominal,
        as_built_mm: as_built,
        rerun_threshold_mm: threshold,
    }
}

#[test]
fn v5_20_03_identities_masses_and_deviations_stay_separate() {
    let masses = Masses {
        nominal_kg: 0.52,
        as_built_kg: 0.524,
    };
    let features = [
        feature("bore", 10.0, 10.05, admitted(0.2, "threshold-source")),
        feature("wall", 1.0, 0.7, admitted(0.2, "threshold-source")),
    ];
    let partition = build_partition(&identity(), &masses, &features).expect("valid partition");
    assert_ne!(
        partition.identity.nominal_sha256,
        partition.identity.as_built_sha256
    );
    assert_eq!(partition.nominal_kg, 0.52);
    assert_eq!(partition.as_built_kg, 0.524);
    assert!((partition.mass_deviation_kg - 0.004).abs() < 1e-12);
    assert!((partition.deviations_mm["wall"] + 0.3).abs() < 1e-12);
    assert_eq!(partition.physics_reruns, vec!["wall".to_string()]);
    assert!(mentions(&partition.verdict, "physics_rerun_required:wall"));
}

#[test]
fn v5_20_03_as_built_subject_cannot_reuse_the_nominal_identity() {
    let same = SubjectIdentity {
        nominal_sha256: Sha256Hex::of_bytes(b"nominal-design"),
        as_built_sha256: Sha256Hex::of_bytes(b"nominal-design"),
    };
    let masses = Masses {
        nominal_kg: 0.52,
        as_built_kg: 0.52,
    };
    assert!(build_partition(&same, &masses, &[]).is_err());
}

#[test]
fn v5_20_03_unadmitted_rerun_threshold_is_unresolved_not_pass() {
    let masses = Masses {
        nominal_kg: 0.52,
        as_built_kg: 0.52,
    };
    let features = [feature("bore", 10.0, 10.0, proposed(0.2))];
    let partition = build_partition(&identity(), &masses, &features).expect("valid partition");
    assert!(mentions(
        &partition.verdict,
        "bore:rerun_threshold_not_admitted"
    ));
    assert!(!partition.verdict.is_pass());
}

#[test]
fn v5_20_03_finite_positive_threshold_keeps_unchanged_feature_passing() {
    let masses = Masses {
        nominal_kg: 0.52,
        as_built_kg: 0.52,
    };
    let features = [feature(
        "bore",
        10.0,
        10.0,
        admitted(0.2, "threshold-source"),
    )];
    let partition = build_partition(&identity(), &masses, &features).expect("valid partition");
    assert_eq!(partition.verdict, Verdict::Pass, "{:?}", partition.verdict);
    assert!(partition.physics_reruns.is_empty());
    assert_eq!(partition.deviations_mm["bore"], 0.0);
}

#[test]
fn v5_20_03_nonfinite_or_nonpositive_admitted_threshold_is_unresolved() {
    let masses = Masses {
        nominal_kg: 0.52,
        as_built_kg: 0.52,
    };
    for threshold in [f64::NAN, f64::INFINITY, 0.0, -0.2] {
        let features = [feature(
            "bore",
            10.0,
            10.0,
            admitted(threshold, "threshold-source"),
        )];
        let partition = build_partition(&identity(), &masses, &features).expect("valid partition");
        assert!(
            !partition.verdict.is_pass(),
            "threshold {threshold}: {:?}",
            partition.verdict
        );
        assert!(
            mentions(
                &partition.verdict,
                "bore:rerun_threshold_not_finite_positive"
            ),
            "threshold {threshold}: {:?}",
            partition.verdict
        );
    }
}

#[test]
fn v5_20_03_ct_voxel_pitch_alone_is_not_dimensional_accuracy() {
    let pitch_only = CtBudget {
        voxel_pitch_um: 50.0,
        calibration_artifact_uncertainty_um: None,
        edge_localization_um: None,
        repeatability_um: None,
        calibration_artifact_source: None,
    };
    let verdict = dimensional_claim(&identity(), &pitch_only, 2.0, &admitted(100.0, "req"))
        .expect("valid identity");
    assert!(matches!(verdict, Verdict::Unresolved(_)));
    assert!(!verdict.is_pass());
}

#[test]
fn v5_20_03_declared_ct_components_give_accuracy_and_bound_the_claim() {
    // 2 * sqrt((50/sqrt 12)^2 + 5^2 + 10^2 + 3^2) = 2 * sqrt(342.33) = 37.0 um.
    let budget = CtBudget {
        voxel_pitch_um: 50.0,
        calibration_artifact_uncertainty_um: Some(5.0),
        edge_localization_um: Some(10.0),
        repeatability_um: Some(3.0),
        calibration_artifact_source: Some(Sha256Hex::of_bytes(b"ct-calibration")),
    };
    let pass = dimensional_claim(&identity(), &budget, 2.0, &admitted(100.0, "req"))
        .expect("valid identity");
    assert_eq!(pass, Verdict::Pass);
    let fail = dimensional_claim(&identity(), &budget, 2.0, &admitted(30.0, "req"))
        .expect("valid identity");
    assert!(matches!(fail, Verdict::Fail(_)));
}

#[test]
fn v5_20_03_measured_ra_never_becomes_ks_without_admitted_calibration() {
    let face = FaceFinish {
        face_id: "bore".into(),
        finish_class: "lpbf_as_built_bore".into(),
        ra_um: Some(6.3),
    };
    let none = roughness_claim(&identity(), &face, &BTreeMap::new()).expect("valid identity");
    assert!(matches!(none, Roughness::Unresolved(_)));
    let mut proposed_map = BTreeMap::new();
    proposed_map.insert("lpbf_as_built_bore".to_string(), proposed(calibration()));
    let proposed_result =
        roughness_claim(&identity(), &face, &proposed_map).expect("valid identity");
    assert!(matches!(proposed_result, Roughness::Unresolved(_)));
    let mut admitted_map = BTreeMap::new();
    admitted_map.insert(
        "lpbf_as_built_bore".to_string(),
        admitted(calibration(), "finish-calibration"),
    );
    match roughness_claim(&identity(), &face, &admitted_map).expect("valid identity") {
        Roughness::Ks {
            lower_um, upper_um, ..
        } => {
            assert!((lower_um - 12.6).abs() < 1e-12);
            assert!((upper_um - 25.2).abs() < 1e-12);
        }
        other => panic!("expected admitted ks, got {other:?}"),
    }
}

fn calibration() -> RaToKs {
    RaToKs {
        finish_class: "lpbf_as_built_bore".into(),
        ra_min_um: 3.0,
        ra_max_um: 12.0,
        ks_over_ra_lower: 2.0,
        ks_over_ra_upper: 4.0,
        calibration_points: 5,
    }
}

#[test]
fn v5_20_03_registration_residual_is_judged_against_admitted_limit() {
    let pass = judge_registration(&Registration {
        residual_rms_mm: 0.02,
        limit_mm: admitted(0.05, "registration-limit"),
    });
    assert_eq!(pass, Verdict::Pass);
    let fail = judge_registration(&Registration {
        residual_rms_mm: 0.08,
        limit_mm: admitted(0.05, "registration-limit"),
    });
    assert!(matches!(fail, Verdict::Fail(_)));
    let open = judge_registration(&Registration {
        residual_rms_mm: 0.02,
        limit_mm: proposed(0.05),
    });
    assert!(matches!(open, Verdict::Unresolved(_)));
}

// ---------------------------------------------------------------- V5-20-04 correlated bounds

fn factor(name: &str, group: &str, low: f64, high: f64) -> BoundedFactor {
    BoundedFactor {
        name: name.into(),
        group: group.into(),
        low,
        high,
        kind: Uncertainty::Deterministic,
    }
}

#[test]
fn v5_20_04_correlated_groups_move_together_and_bound_the_response() {
    let factors = [
        factor("wall_offset", "geometry", 1.0, 2.0),
        factor("load_scale", "geometry", 10.0, 20.0),
        factor("modulus_offset", "material", 0.0, 1.0),
    ];
    let bounds = corner_bounds(&factors, |x| x[0] * x[1] + 100.0 * x[2]).expect("bounded");
    assert_eq!(bounds.groups, 2);
    assert_eq!(bounds.corners, 4);
    assert_eq!(bounds.lower, 10.0);
    assert_eq!(bounds.upper, 140.0);
}

#[test]
fn v5_20_04_probabilistic_factor_cannot_enter_corner_enumeration() {
    let mut random = factor("wall_offset", "geometry", 1.0, 2.0);
    random.kind = Uncertainty::Probabilistic;
    assert!(corner_bounds(&[random], |x| x[0]).is_err());
}

#[test]
fn v5_20_04_eight_selected_offsets_cannot_claim_distribution_free_p95() {
    assert!(matches!(
        p95_claim(SampleBasis::Corners(8)),
        Verdict::Unresolved(_)
    ));
    let eight_draws = p95_claim(SampleBasis::IidDraws {
        n: 8,
        admitted: true,
    });
    assert!(mentions(
        &eight_draws,
        "distribution_free_95_95_needs_59_draws_has_8"
    ));
}

#[test]
fn v5_20_04_distribution_free_p95_needs_59_admitted_iid_draws() {
    assert!(matches!(
        p95_claim(SampleBasis::IidDraws {
            n: 58,
            admitted: true
        }),
        Verdict::Unresolved(_)
    ));
    assert_eq!(
        p95_claim(SampleBasis::IidDraws {
            n: 59,
            admitted: true
        }),
        Verdict::Pass
    );
    assert!(matches!(
        p95_claim(SampleBasis::IidDraws {
            n: 1000,
            admitted: false
        }),
        Verdict::Unresolved(_)
    ));
}

// ---------------------------------------------------------------- V5-20-05 cross-check

fn route(id: &str, input: &str, observable: &str, value: f64) -> RouteResult {
    RouteResult {
        route_id: id.into(),
        observable: observable.into(),
        value,
        input_sha256: Sha256Hex::of_bytes(input.as_bytes()),
    }
}

#[test]
fn v5_20_05_agreement_within_tolerance_supports_claim_with_discrepancy_uncertainty() {
    let check = cross_check(
        &route("solver_a", "bc-a", "tip_deflection_m", 100.0),
        &route("solver_b", "bc-b", "tip_deflection_m", 101.0),
        &admitted(0.02, "cross-check-tolerance"),
    );
    assert_eq!(check.verdict, Verdict::Pass);
    assert_eq!(
        check.effect,
        ClaimEffect::Supported {
            discrepancy_uncertainty: 1.0
        }
    );
}

#[test]
fn v5_20_05_disagreement_beyond_tolerance_blocks_the_claim() {
    let check = cross_check(
        &route("solver_a", "bc-a", "tip_deflection_m", 100.0),
        &route("solver_b", "bc-b", "tip_deflection_m", 110.0),
        &admitted(0.02, "cross-check-tolerance"),
    );
    assert!(matches!(check.verdict, Verdict::Fail(_)));
    assert_eq!(check.effect, ClaimEffect::NotSupported);
}

#[test]
fn v5_20_05_two_solvers_sharing_one_wrong_boundary_file_cannot_qualify_it() {
    let check = cross_check(
        &route("solver_a", "same-wrong-bc", "tip_deflection_m", 100.0),
        &route("solver_b", "same-wrong-bc", "tip_deflection_m", 100.0),
        &admitted(0.02, "cross-check-tolerance"),
    );
    assert!(mentions(
        &check.verdict,
        "shared_boundary_load_input_not_independent"
    ));
    assert_eq!(check.effect, ClaimEffect::NotSupported);
}

#[test]
fn v5_20_05_unadmitted_tolerance_or_mismatched_observable_is_not_supported() {
    let open = cross_check(
        &route("solver_a", "bc-a", "tip_deflection_m", 100.0),
        &route("solver_b", "bc-b", "tip_deflection_m", 100.0),
        &proposed(0.02),
    );
    assert!(matches!(open.verdict, Verdict::Unresolved(_)));
    assert_eq!(open.effect, ClaimEffect::NotSupported);
    let mismatched = cross_check(
        &route("solver_a", "bc-a", "tip_deflection_m", 100.0),
        &route("solver_b", "bc-b", "peak_stress_pa", 100.0),
        &admitted(0.02, "cross-check-tolerance"),
    );
    assert!(matches!(mismatched.verdict, Verdict::Unresolved(_)));
}
