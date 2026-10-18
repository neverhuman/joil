//! V5-15 judge tests: accession, preregistration, observable coverage, branch challenges and
//! reference admission. Every number is a SYNTHETIC fixture. None of these tests is a CT-35
//! measurement, a literature result, or a physical or scientific PASS.
use pack_hydrocyclone_judges::validation::{
    compare, coverage, flow_archive_enabled, judge_accession, judge_admission, judge_branches,
    judge_discrepancy, judge_pdr, judge_preregistration, judge_protocol, Accession, Branch,
    BranchStudy, Candidate, ComparisonLevel, Ct35Protocol, DatasetStatus, Discrepancy,
    MeasuredCoverage, ModelIdentity, Observable, PdrGate, PdrOrigin, Preregistration, Ramp,
    VortexAdmission,
};
use pack_hydrocyclone_judges::{Field, Sha256Hex, Verdict};
use std::collections::BTreeSet;

fn digest(tag: &str) -> Sha256Hex {
    Sha256Hex::of_bytes(tag.as_bytes())
}

fn admitted<T>(value: T, tag: &str) -> Field<T> {
    Field::Admitted {
        value,
        source: digest(tag),
    }
}

fn unresolved<T>(blocks: &str) -> Field<T> {
    Field::Unresolved {
        reason: "source not accessed".into(),
        owner: "validation owner".into(),
        blocks: vec![blocks.into()],
    }
}

fn strings(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn has(v: &Verdict, needle: &str) -> bool {
    match v {
        Verdict::Pass => false,
        Verdict::Fail(r) | Verdict::Unresolved(r) => r.iter().any(|x| x.contains(needle)),
    }
}

fn is_open(v: &Verdict) -> bool {
    matches!(v, Verdict::Unresolved(_))
}

fn is_fail(v: &Verdict) -> bool {
    matches!(v, Verdict::Fail(_))
}

fn model() -> ModelIdentity {
    ModelIdentity {
        geometry: "synthetic-geometry-A".into(),
        fluids: "synthetic-fluids-A".into(),
        boundary_conditions: "synthetic-bc-A".into(),
        stations: "synthetic-stations-A".into(),
        methods: "synthetic-methods-A".into(),
        uncertainty: "synthetic-uncertainty-A".into(),
    }
}

fn accession(candidate: Candidate) -> Accession {
    Accession {
        candidate,
        source: admitted(digest("synthetic-source"), "source"),
        license: admitted("synthetic-licence".to_string(), "licence"),
        geometry: admitted("synthetic-geometry-A".into(), "geometry"),
        fluids: admitted("synthetic-fluids-A".into(), "fluids"),
        boundary_conditions: admitted("synthetic-bc-A".into(), "bc"),
        stations: admitted("synthetic-stations-A".into(), "stations"),
        methods: admitted("synthetic-methods-A".into(), "methods"),
        uncertainty: admitted("synthetic-uncertainty-A".into(), "uncertainty"),
    }
}

// V5-15-01: accession of original benchmark evidence.

#[test]
fn v5_15_01_fully_admitted_matching_accession_is_quantitative() {
    let a = accession(Candidate::Bai);
    assert!(judge_accession(&a).is_pass());
    let card = compare(&a, &model());
    assert_eq!(card.level, ComparisonLevel::Quantitative);
    assert_eq!(card.matched.len(), 6);
    assert!(card.unresolved.is_empty());
}

#[test]
fn v5_15_01_overlay_without_geometry_and_uncertainty_stays_literature() {
    // Attractive overlay: the fluid and station labels agree, but no geometry or uncertainty is known.
    let mut a = accession(Candidate::Young);
    a.geometry = unresolved("geometry");
    a.uncertainty = unresolved("uncertainty");
    let card = compare(&a, &model());
    assert_eq!(card.level, ComparisonLevel::LiteratureOnly);
    assert!(card
        .unresolved
        .contains(&"geometry:not_admitted".to_string()));
    assert!(card
        .unresolved
        .contains(&"uncertainty:not_admitted".to_string()));
    assert!(is_open(&judge_accession(&a)));
}

#[test]
fn v5_15_01_different_published_geometry_is_literature_not_quantitative() {
    let mut a = accession(Candidate::Bai);
    a.geometry = admitted("published-geometry-B".into(), "geometry-b");
    let card = compare(&a, &model());
    assert_eq!(card.level, ComparisonLevel::LiteratureOnly);
    assert!(card
        .unresolved
        .contains(&"geometry:literature_differs_from_model".to_string()));
}

#[test]
fn v5_15_01_proposed_recalled_value_is_not_an_accession() {
    let mut a = accession(Candidate::Young);
    a.stations = Field::Proposed {
        value: "synthetic-stations-A".into(),
        reason: "recalled, not accessed".into(),
    };
    assert!(has(&judge_accession(&a), "stations_not_admitted"));
    assert_eq!(compare(&a, &model()).level, ComparisonLevel::LiteratureOnly);
}

#[test]
fn v5_15_01_zhou_and_hsieh_without_sources_stay_unresolved() {
    for candidate in [Candidate::Zhou, Candidate::Hsieh] {
        let a = Accession {
            candidate,
            source: unresolved("comparison"),
            license: unresolved("comparison"),
            geometry: unresolved("comparison"),
            fluids: unresolved("comparison"),
            boundary_conditions: unresolved("comparison"),
            stations: unresolved("comparison"),
            methods: unresolved("comparison"),
            uncertainty: unresolved("comparison"),
        };
        assert!(is_open(&judge_accession(&a)));
        assert_eq!(compare(&a, &model()).level, ComparisonLevel::LiteratureOnly);
    }
}

// V5-15-02: preregistered anomaly study.

fn study(arms: &[&str], hypotheses: &[&str]) -> Preregistration {
    Preregistration {
        frozen: strings(&["ct35_geometry", "taps", "duty", "analysis"]),
        frozen_before_outcomes: true,
        arms: arms.iter().map(|s| s.to_string()).collect(),
        hypotheses: strings(hypotheses),
    }
}

#[test]
fn v5_15_02_one_factor_arms_with_competing_explanations_pass() {
    let p = study(
        &[
            "convection",
            "wall_layers",
            "run_duration",
            "initialization",
            "outlet_orifice_treatment",
        ],
        &["vortex_present", "vortex_absent_numerical"],
    );
    assert!(judge_preregistration(&p).is_pass());
}

#[test]
fn v5_15_02_outcomes_inspected_before_freeze_fail() {
    let mut p = study(
        &["convection"],
        &["vortex_present", "vortex_absent_numerical"],
    );
    p.frozen_before_outcomes = false;
    assert!(has(
        &judge_preregistration(&p),
        "outcomes_inspected_before_freeze"
    ));
}

#[test]
fn v5_15_02_changing_a_frozen_item_fails() {
    let p = study(&["taps"], &["vortex_present", "vortex_absent_numerical"]);
    assert!(has(&judge_preregistration(&p), "frozen_item_changed:taps"));
}

#[test]
fn v5_15_02_unregistered_factor_fails() {
    let p = study(
        &["mesh_size"],
        &["vortex_present", "vortex_absent_numerical"],
    );
    assert!(has(
        &judge_preregistration(&p),
        "factor_not_preregistered:mesh_size"
    ));
}

#[test]
fn v5_15_02_single_explanation_cannot_assume_an_absent_vortex() {
    let p = study(&["convection"], &["vortex_absent"]);
    let v = judge_preregistration(&p);
    assert!(is_open(&v));
    assert!(has(&v, "competing_explanations_required"));
}

#[test]
fn v5_15_02_borrowed_commercial_pdr_cannot_force_a_passing_case() {
    // Commercial 1.4-2.6 range for other liners: diagnostic context only, even if admitted as text.
    let gate = PdrGate {
        origin: PdrOrigin::BorrowedCommercial,
        range: admitted((1.4, 2.6), "commercial-range"),
        tuned_after_pdr: false,
    };
    let v = judge_pdr(2.0, &gate);
    assert!(is_open(&v));
    assert!(!v.is_pass());
}

#[test]
fn v5_15_02_measured_pdr_range_passes_inside_fails_outside_and_refuses_tuning() {
    // SYNTHETIC range for the judge test only.
    let gate = PdrGate {
        origin: PdrOrigin::MeasuredCt35,
        range: admitted((1.5, 2.2), "synthetic-ct35-range"),
        tuned_after_pdr: false,
    };
    assert!(judge_pdr(1.8, &gate).is_pass());
    assert!(is_fail(&judge_pdr(2.5, &gate)));
    let tuned = PdrGate {
        tuned_after_pdr: true,
        ..gate.clone()
    };
    assert!(has(&judge_pdr(1.8, &tuned), "tuned_after_inspection"));
}

// V5-15-03: independent observable validation and coverage cards.

#[test]
fn v5_15_03_pressure_agreement_cannot_qualify_oil_grade() {
    let m = MeasuredCoverage {
        observables: BTreeSet::from([Observable::PressureSplit]),
        geometry_family: "family-A".into(),
    };
    assert!(coverage(&m, "family-A", Observable::PressureSplit, "steady").supported);
    let oil = coverage(&m, "family-A", Observable::OilGrade, "steady");
    assert!(!oil.supported);
    assert_eq!(oil.reason, "no_measurement_of_this_observable");
}

#[test]
fn v5_15_03_good_pressure_cannot_qualify_a_different_geometry_family() {
    let m = MeasuredCoverage {
        observables: BTreeSet::from([Observable::PressureSplit, Observable::OilGrade]),
        geometry_family: "family-B".into(),
    };
    let card = coverage(&m, "family-A", Observable::PressureSplit, "steady");
    assert!(!card.supported);
    assert_eq!(card.reason, "geometry_family_differs");
}

#[test]
fn v5_15_03_discrepancy_components_are_quantified_separately() {
    let full = Discrepancy {
        numerical: Some(0.10),
        input: Some(0.02),
        experimental: Some(0.05),
    };
    assert!(judge_discrepancy(&full).is_pass());
    let missing = Discrepancy {
        numerical: Some(0.10),
        input: None,
        experimental: Some(0.05),
    };
    assert!(has(
        &judge_discrepancy(&missing),
        "discrepancy_unquantified:input"
    ));
    let negative = Discrepancy {
        numerical: Some(-0.1),
        input: Some(0.0),
        experimental: Some(0.0),
    };
    assert!(is_open(&judge_discrepancy(&negative)));
}

// V5-15-04: multiple stable states, retained branches and held-out geometries.

fn branch(initialization: &str, ramp: Ramp, converged: bool) -> Branch {
    Branch {
        initialization: initialization.into(),
        ramp,
        duty: 1.0,
        converged,
    }
}

fn complete_study() -> BranchStudy {
    BranchStudy {
        branches: vec![
            branch("init-low", Ramp::Up, true),
            branch("init-high", Ramp::Up, true),
            branch("init-high", Ramp::Down, true),
            branch("init-low", Ramp::Down, true),
        ],
        startup_condition: Some("synthetic-startup-ramp".into()),
        best_of_selection: false,
        calibration: strings(&["geometry-cal"]),
        held_out: strings(&["geometry-held-out"]),
    }
}

#[test]
fn v5_15_04_converged_up_and_down_branches_from_two_initializations_pass() {
    assert!(judge_branches(&complete_study()).is_pass());
}

#[test]
fn v5_15_04_best_branch_selection_cannot_represent_general_operation() {
    let mut s = complete_study();
    s.best_of_selection = true;
    let v = judge_branches(&s);
    assert!(is_open(&v));
    assert!(has(&v, "best_branch_selection_cannot_represent_operation"));
}

#[test]
fn v5_15_04_single_initialization_or_missing_ramp_is_not_a_multiple_state_study() {
    let mut one_init = complete_study();
    one_init.branches.retain(|b| b.initialization == "init-low");
    assert!(has(
        &judge_branches(&one_init),
        "multiple_states_not_challenged"
    ));
    let mut one_ramp = complete_study();
    one_ramp.branches.retain(|b| b.ramp == Ramp::Up);
    assert!(has(
        &judge_branches(&one_ramp),
        "multiple_states_not_challenged"
    ));
}

#[test]
fn v5_15_04_unconverged_branch_is_kept_open_not_dropped() {
    let mut s = complete_study();
    s.branches[1].converged = false;
    assert!(has(
        &judge_branches(&s),
        "branch_not_converged:init-high:Up"
    ));
}

#[test]
fn v5_15_04_calibration_and_held_out_geometries_must_stay_separate() {
    let mut s = complete_study();
    s.held_out = strings(&["geometry-cal"]);
    assert!(is_fail(&judge_branches(&s)));
}

#[test]
fn v5_15_04_missing_startup_condition_is_open() {
    let mut s = complete_study();
    s.startup_condition = None;
    assert!(has(
        &judge_branches(&s),
        "startup_control_selection_missing"
    ));
}

// V5-15-05: reference admission, archive gate and the owned CT-35 protocol.

fn build() -> Sha256Hex {
    digest("synthetic-evaluator-build-1")
}

fn admission() -> VortexAdmission {
    VortexAdmission {
        evaluator_build: build(),
        scope: strings(&["vortex-static-synthetic"]),
        dataset: DatasetStatus::Available,
        evidence: Some(digest("synthetic-matching-evidence")),
    }
}

#[test]
fn v5_15_05_matching_admission_passes_and_only_then_enables_flow_archives() {
    let v = judge_admission(Some(&admission()), &build(), "vortex-static-synthetic");
    assert!(v.is_pass());
    assert!(flow_archive_enabled(&v));
}

#[test]
fn v5_15_05_evaluator_version_change_revokes_ranking_authority() {
    let v = judge_admission(
        Some(&admission()),
        &digest("synthetic-evaluator-build-2"),
        "vortex-static-synthetic",
    );
    assert!(has(&v, "revoked_by_evaluator_build_change"));
    assert!(!flow_archive_enabled(&v));
}

#[test]
fn v5_15_05_withdrawn_or_inaccessible_dataset_revokes_ranking() {
    let mut a = admission();
    a.dataset = DatasetStatus::Withdrawn;
    let v = judge_admission(Some(&a), &build(), "vortex-static-synthetic");
    assert!(has(&v, "dataset_withdrawn_revokes_ranking"));
    a.dataset = DatasetStatus::Inaccessible;
    let w = judge_admission(Some(&a), &build(), "vortex-static-synthetic");
    assert!(has(&w, "dataset_inaccessible"));
    assert!(!flow_archive_enabled(&w));
}

#[test]
fn v5_15_05_scope_outside_admission_missing_evidence_or_no_admission_stay_open() {
    let outside = judge_admission(Some(&admission()), &build(), "vortex-dynamic");
    assert!(has(&outside, "scope_outside_admission"));
    let mut a = admission();
    a.evidence = None;
    let v = judge_admission(Some(&a), &build(), "vortex-static-synthetic");
    assert!(has(&v, "matching_evidence_missing"));
    assert!(!flow_archive_enabled(&v));
    let none = judge_admission(None, &build(), "vortex-static-synthetic");
    assert!(has(&none, "gvtx:not_admitted"));
}

fn protocol() -> Ct35Protocol {
    Ct35Protocol {
        launched: false,
        replicates: 3,
        drift_conc_rel: 0.10,
        drift_dp_rel: 0.05,
        taps_frozen: true,
    }
}

#[test]
fn v5_15_05_ct35_protocol_is_defined_but_not_launched() {
    assert!(judge_protocol(&protocol()).is_pass());
}

#[test]
fn v5_15_05_ct35_protocol_refuses_launch_loosened_gates_and_thin_replication() {
    let launched = Ct35Protocol {
        launched: true,
        ..protocol()
    };
    assert!(is_fail(&judge_protocol(&launched)));
    let loose = Ct35Protocol {
        drift_conc_rel: 0.20,
        ..protocol()
    };
    assert!(has(&judge_protocol(&loose), "drift_gate_loosened"));
    let thin = Ct35Protocol {
        replicates: 2,
        ..protocol()
    };
    assert!(has(&judge_protocol(&thin), "replicates_below_minimum"));
}
