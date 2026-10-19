use super::*;

// ------------------------------------------------------------------------- V5-19-04

fn eigen(multipliers: &[f64]) -> Vec<EigenSolve> {
    multipliers
        .iter()
        .enumerate()
        .map(|(i, m)| EigenSolve {
            mesh_sha256: sha(&format!("eigen-mesh-{i}")),
            element_count: 10_000 * (i as u64 + 1),
            multiplier: *m,
        })
        .collect()
}
fn collapse(
    label: &str,
    amplitude: f64,
    contact: bool,
    converged: bool,
    limit: f64,
) -> CollapseSolve {
    CollapseSolve {
        imperfection_label: label.into(),
        imperfection_amplitude_m: amplitude,
        imperfection_shape_sha256: sha("mode-1"),
        contact_declared: contact,
        mesh_sha256: sha("collapse-mesh"),
        converged,
        limit_multiplier: limit,
        solve_sha256: sha(&format!("collapse-{label}")),
    }
}
fn cylinder(length_m: f64) -> CylinderReference {
    CylinderReference {
        youngs_pa: 113.8e9,
        poisson: 0.342,
        wall_m: 0.002,
        radius_m: 0.05,
        length_m,
    }
}
fn buckling_input() -> BucklingInput {
    BucklingInput {
        finalist: true,
        contact_applicable: true,
        demand_reference_pa: 1.0e6,
        eigen: eigen(&[2.1, 2.08, 2.09]),
        collapse: vec![collapse("mode1-1e-4", 1e-4, true, true, 1.4)],
        reference: Some(cylinder(1.0)),
        mesh_spread_tolerance: adm(0.02, "mesh-spread"),
    }
}

#[test]
fn v5_19_04_governing_multiplier_is_reported_with_method_and_validity() {
    // hand value: E/(4(1-nu^2)) (t/R)^3 = 2.062e6 Pa for t/R = 0.04, valid for L >= 0.785 m
    let p = cylinder(1.0).long_tube_collapse_pa().unwrap();
    assert!(close(p, 2.062e6, 1e-3), "{p}");
    assert!(cylinder(0.2).long_tube_collapse_pa().is_none());
    let r = judge_buckling(&buckling_input()).unwrap();
    assert!(r.verdict.is_pass(), "{:?}", r.verdict);
    assert_eq!(r.governing_multiplier, Some(1.4));
    assert_eq!(r.method, "eigenbuckling_screen_then_nonlinear_collapse");
    assert!(r
        .validity
        .iter()
        .any(|v| v.contains("nonlinear_collapse_with_declared_imperfections")));
    assert!(close(r.reference_ratio.unwrap(), 1.4 / (p / 1.0e6), 1e-12));
    // a short cylinder is outside the reference envelope: reported, ratio absent
    let mut i = buckling_input();
    i.reference = Some(cylinder(0.2));
    let r = judge_buckling(&i).unwrap();
    assert!(r.reference_ratio.is_none());
    assert!(r
        .validity
        .iter()
        .any(|v| v == "long_tube_reference_outside_envelope"));
    // collapse above the eigen value never raises the governing multiplier
    let mut i = buckling_input();
    i.collapse = vec![collapse("stiffening", 1e-4, true, true, 3.0)];
    assert_eq!(judge_buckling(&i).unwrap().governing_multiplier, Some(2.08));
}

#[test]
fn v5_19_04_eigen_screen_alone_failed_solves_and_thin_evidence_never_pass() {
    let screen = |f: &dyn Fn(&mut BucklingInput)| {
        let mut i = buckling_input();
        f(&mut i);
        judge_buckling(&i).unwrap()
    };
    let r = screen(&|i| i.collapse.clear());
    assert!(has(&r.verdict, "nonlinear_collapse:not_run"));
    assert_eq!(r.method, "eigenbuckling_screen_only");
    assert!(r
        .validity
        .iter()
        .any(|v| v == "screen_not_collapse_validation"));
    let r = screen(&|i| i.finalist = false);
    assert!(has(&r.verdict, "eigen_screen_only_not_collapse_validated"));
    let r = screen(&|i| i.collapse = vec![collapse("diverged", 1e-4, true, false, 0.0)]);
    assert!(is_open(&r.verdict) && has(&r.verdict, "collapse:not_converged:diverged"));
    assert!(has(
        &screen(&|i| i.collapse[0].imperfection_amplitude_m = 0.0).verdict,
        "no_declared_imperfection"
    ));
    assert!(has(
        &screen(&|i| i.collapse[0].contact_declared = false).verdict,
        "contact_not_modelled"
    ));
    assert!(has(
        &screen(&|i| i.eigen.truncate(2)).verdict,
        "fewer_than_three_meshes"
    ));
    assert!(has(
        &screen(&|i| i.eigen = eigen(&[2.1, 1.6, 2.05])).verdict,
        "spread_exceeds_budget"
    ));
    assert!(has(
        &screen(&|i| i.mesh_spread_tolerance = Field::Proposed {
            value: 0.02,
            reason: "d".into()
        })
        .verdict,
        "tolerance:not_admitted"
    ));
    assert!(has(
        &screen(&|i| i.eigen.clear()).verdict,
        "eigenbuckling:not_run"
    ));
    // buckling below the frozen demand is a physical fail
    let r = screen(&|i| i.collapse = vec![collapse("weak", 1e-4, true, true, 0.8)]);
    assert!(is_fail(&r.verdict) && has(&r.verdict, "buckling_multiplier_below_frozen_demand"));
    assert!(judge_buckling(&BucklingInput {
        demand_reference_pa: 0.0,
        ..buckling_input()
    })
    .is_err());
    let mut i = buckling_input();
    i.eigen[0].multiplier = f64::NAN;
    assert!(judge_buckling(&i).is_err());
}

// ------------------------------------------------------------------------- V5-19-05

fn limit(family: LoadFamily, kind: LimitKind, allowable: f64) -> Limit {
    Limit {
        family,
        kind,
        response: Response::Linear,
        derivation: Derivation::ScaledSingleSolve,
        allowable_multiplier: allowable,
        load_basis_sha256: synthetic_frozen().sha256,
        multiplier_definition: None,
    }
}
fn buckling_limit(allowable: f64, resolved: f64) -> Limit {
    Limit {
        response: Response::Nonlinear,
        derivation: Derivation::ExplicitResolves {
            solve_sha256: sha("collapse"),
            resolved_multiplier: resolved,
        },
        ..limit(
            LoadFamily::ExternalDifferentialUpset,
            LimitKind::Buckling,
            allowable,
        )
    }
}
fn base_limits() -> Vec<Limit> {
    let mut v: Vec<Limit> = ALL_FAMILIES
        .iter()
        .map(|f| limit(*f, LimitKind::Yield, 3.0))
        .collect();
    v.push(buckling_limit(1.8, 1.8));
    v
}
fn separate(modal_value: f64, fatigue_value: f64) -> Vec<SeparateRequirement> {
    vec![
        SeparateRequirement {
            kind: LimitKind::Modal,
            name: "frequency_ratio".into(),
            value: modal_value,
            required: adm(2.2, "modal"),
        },
        SeparateRequirement {
            kind: LimitKind::Fatigue,
            name: "fatigue_safety_factor".into(),
            value: fatigue_value,
            required: adm(2.0, "fatigue"),
        },
    ]
}

#[test]
fn v5_19_05_reserve_is_allowable_over_required_per_family_with_a_common_governing_limit() {
    let r = compute_reserve(&synthetic_frozen(), &base_limits(), &separate(3.1, 2.4)).unwrap();
    assert!(r.verdict.is_pass(), "{:?}", r.verdict);
    // yield 3.0 / 1.5 = 2.0 everywhere; external buckling 1.8 / 1.5 = 1.2 governs
    assert!(close(
        r.family_reserve[&LoadFamily::OperatingFlow],
        2.0,
        1e-12
    ));
    assert!(close(
        r.family_reserve[&LoadFamily::ExternalDifferentialUpset],
        1.2,
        1e-12
    ));
    assert_eq!(
        r.governing.map(|g| (g.0, g.1)),
        Some((LoadFamily::ExternalDifferentialUpset, LimitKind::Buckling))
    );
    assert!(close(r.governing.unwrap().2, 1.2, 1e-12));
    assert!(r.separate.iter().all(|(_, v)| v.is_pass()));
}

#[test]
fn v5_19_05_high_yield_margin_cannot_mask_untested_external_pressure_buckling() {
    let mut limits = base_limits();
    limits.pop();
    limits[1].allowable_multiplier = 50.0;
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(is_open(&r.verdict) && has(&r.verdict, "external_pressure_buckling:untested"));
    assert!(close(
        r.family_reserve[&LoadFamily::ExternalDifferentialUpset],
        50.0 / 1.5,
        1e-12
    ));
    // a buckling limit whose re-solve never reached its claimed multiplier does not count
    let mut limits = base_limits();
    *limits.last_mut().unwrap() = buckling_limit(1.8, 1.2);
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(has(&r.verdict, "resolve_does_not_reach_claimed_multiplier"));
    assert!(has(&r.verdict, "external_pressure_buckling:untested"));
}

#[test]
fn v5_19_05_frequency_ratio_and_fatigue_factor_are_not_min_combined_as_load_multipliers() {
    // a frequency ratio of 1.1 and a fatigue factor of 0.9 fail their own requirements only
    let r = compute_reserve(&synthetic_frozen(), &base_limits(), &separate(1.1, 0.9)).unwrap();
    assert!(is_fail(&r.verdict));
    assert!(has(
        &r.verdict,
        "frequency_ratio:modal_below_own_requirement"
    ));
    assert!(has(
        &r.verdict,
        "fatigue_safety_factor:fatigue_below_own_requirement"
    ));
    assert!(
        close(r.governing.unwrap().2, 1.2, 1e-12),
        "separate numbers must not enter the min"
    );
    // presenting them as multiplier limits without a definition is refused and excluded
    for kind in [LimitKind::Modal, LimitKind::Fatigue] {
        let mut limits = base_limits();
        limits.push(limit(LoadFamily::OperatingFlow, kind, 0.4));
        let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
        assert!(is_open(&r.verdict) && has(&r.verdict, "not_a_load_multiplier_without_definition"));
        assert!(close(
            r.family_reserve[&LoadFamily::OperatingFlow],
            2.0,
            1e-12
        ));
    }
    // a justified, admitted multiplier definition makes the limit usable
    let mut limits = base_limits();
    let mut modal = limit(LoadFamily::OperatingFlow, LimitKind::Modal, 1.65);
    modal.multiplier_definition = Some(adm(
        "synthetic: pressure scale at which f1 reaches the excitation band".to_string(),
        "defn",
    ));
    limits.push(modal);
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(close(
        r.family_reserve[&LoadFamily::OperatingFlow],
        1.1,
        1e-12
    ));
}

#[test]
fn v5_19_05_nonlinear_limits_need_resolves_and_every_limit_cites_the_frozen_demand() {
    let mut limits = base_limits();
    let mut scaled = limit(LoadFamily::SupportPreloadContact, LimitKind::Contact, 9.0);
    scaled.response = Response::Nonlinear;
    limits.push(scaled);
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(has(
        &r.verdict,
        "contact:nonlinear_limit_needs_explicit_resolves"
    ));
    let mut limits = base_limits();
    limits[2].load_basis_sha256 = sha("easier-load-set");
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(is_fail(&r.verdict) && has(&r.verdict, "load_basis_differs_from_frozen_set"));
    let mut limits = base_limits();
    limits[3].allowable_multiplier = 1.2;
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(is_fail(&r.verdict) && has(&r.verdict, "family:handling:reserve_below_1:yield"));
    let r = compute_reserve(&synthetic_frozen(), &base_limits()[..3], &[]).unwrap();
    assert!(has(&r.verdict, "family:handling:no_usable_limit"));
    // the unresolved draft set can never produce a pass
    let draft = LoadFamilySet::from_json(DRAFT).unwrap().freeze().unwrap();
    let mut limits = base_limits();
    for l in &mut limits {
        l.load_basis_sha256 = draft.sha256.clone();
    }
    let r = compute_reserve(&draft, &limits, &[]).unwrap();
    assert!(has(&r.verdict, "required_factor:not_admitted"));
    assert!(!r.verdict.is_pass() && r.governing.is_none());
    assert!(compute_reserve(
        &synthetic_frozen(),
        &[limit(LoadFamily::Thermal, LimitKind::Yield, f64::NAN)],
        &[]
    )
    .is_err());
}

#[test]
fn v5_19_01_to_05_sc_dec_031_032_installation_loads_cfd_pressure_shear_native_fea_buckling_and_reserve(
) {
    // Frozen installation demands are design-independent and cannot be eased by a candidate.
    v5_19_01_draft_set_keeps_2_5_mpa_provisional_and_everything_else_unresolved();
    v5_19_01_multiplier_one_is_the_same_external_demand_for_every_design();
    v5_19_01_a_weaker_design_cannot_choose_an_easier_reference_load();

    // Candidate-bound pressure and shear mapping preserves patch coverage, normals, mesh and
    // independent force/moment reactions; copied pressure and missing loads are rejected.
    v5_19_02_conservative_transfer_fits_the_preallocated_budget();
    v5_19_02_copied_ct35_pressure_wrong_normals_and_missing_shear_or_patches_fail();
    v5_19_02_identity_budget_and_reactions_are_checked();

    // Native static FEA qualification requires pinned solver evidence and all oracle fixtures;
    // analytic formulas remain checks, not qualification evidence.
    v5_19_03_oracles_match_hand_derived_values();
    v5_19_03_solver_qualifies_only_when_every_oracle_fixture_passes();
    v5_19_03_process_adapter_is_pinned_and_isolated();
    v5_19_03_static_run_is_qualified_only_on_a_native_solid_solve_with_evidence();
    v5_19_03_fixed_load_formula_and_singular_point_stress_cannot_qualify();

    // Eigenvalue screening does not replace imperfection/contact collapse challenges; reserve
    // combines compatible load multipliers only and keeps modal/fatigue metrics separate.
    v5_19_04_governing_multiplier_is_reported_with_method_and_validity();
    v5_19_04_eigen_screen_alone_failed_solves_and_thin_evidence_never_pass();
    v5_19_05_reserve_is_allowable_over_required_per_family_with_a_common_governing_limit();
    v5_19_05_high_yield_margin_cannot_mask_untested_external_pressure_buckling();
    v5_19_05_frequency_ratio_and_fatigue_factor_are_not_min_combined_as_load_multipliers();
    v5_19_05_nonlinear_limits_need_resolves_and_every_limit_cites_the_frozen_demand();
}
