use super::*;

// ---------------------------------------- V5-25-03/05 frozen operator, envelope and routing

fn envelope(upper_turbulence: f64) -> Envelope {
    Envelope {
        lower: [0.0; 3],
        upper: [0.1, 0.5, upper_turbulence],
    }
}

fn operator(upper_turbulence: f64) -> FrozenLinerOperator {
    FrozenLinerOperator::new(identity(), state(1.0), obs(), envelope(upper_turbulence))
        .expect("valid fixture")
}

/// Interface that matches the frozen state's own fluxes, so the closure holds at the first exchange.
fn frozen_interface(s: &LinerCutPlaneState) -> Interface {
    let got = integrate(s);
    Interface {
        oil_kg_s: got.oil_kg_s,
        reverse_core_kg_s: got.reverse_core_kg_s,
    }
}

fn evidence(coupling: CouplingOutcome, hull: f64, uncertainty: f64) -> OperatorEvidence {
    OperatorEvidence {
        identity_sha256: Sha256Hex::of_bytes(b"operator-a"),
        candidate_sha256: Sha256Hex::of_bytes(b"candidate-a"),
        features: [0.0; 3],
        hull_distance: hull,
        relative_uncertainty: uncertainty,
        coupling,
    }
}

fn converged() -> CouplingOutcome {
    CouplingOutcome::Converged {
        exchanges: 2,
        residuals: vec![1e-12, 1e-12],
        observables: obs(),
        interface: Interface {
            oil_kg_s: FROZEN_OIL.to_vec(),
            reverse_core_kg_s: 0.0,
        },
    }
}

fn reasons_of(r: Route) -> Vec<String> {
    match r {
        Route::FullLiner { reasons } => reasons,
        Route::Operator { .. } => Vec::new(),
    }
}

#[test]
fn in_envelope_candidate_with_closed_interface_uses_the_operator() {
    let op = operator(0.05);
    let cand = state(1.0);
    let ev = op
        .assess(
            &Identity,
            frozen_interface(&cand),
            &cfg(20, 1.0, 1e-9, 1e-6),
            &cand,
        )
        .expect("assess");
    assert!(ev.hull_distance.abs() < 1e-12);
    assert!(matches!(ev.coupling, CouplingOutcome::Converged { .. }));
    assert!(matches!(route(Some(&ev), 0.05), Route::Operator { .. }));
}

#[test]
fn candidate_outside_the_envelope_dispatches_full_liner() {
    let op = operator(0.001);
    let cand = state(1.0);
    let ev = op
        .assess(
            &Identity,
            frozen_interface(&cand),
            &cfg(20, 1.0, 1e-9, 1e-6),
            &cand,
        )
        .expect("assess");
    assert!(ev.hull_distance > 0.0);
    let r = reasons_of(route(Some(&ev), 0.05));
    assert!(r.contains(&"ood:outside_envelope".to_string()), "{r:?}");
}

#[test]
fn high_uncertainty_state_dispatches_full_liner() {
    let op = operator(0.05);
    let mut cand = state(1.0);
    cand.relative_uncertainty = 0.2;
    let ev = op
        .assess(
            &Identity,
            frozen_interface(&cand),
            &cfg(20, 1.0, 1e-9, 1e-6),
            &cand,
        )
        .expect("assess");
    let r = reasons_of(route(Some(&ev), 0.05));
    assert!(r.contains(&"uncertainty:above_limit".to_string()), "{r:?}");
}

#[test]
fn invalid_candidate_state_is_never_routed_to_the_operator() {
    let op = operator(0.05);
    let mut cand = state(1.0);
    cand.nodes.truncate(2);
    let ev = op
        .assess(
            &Identity,
            frozen_interface(&state(1.0)),
            &cfg(20, 1.0, 1e-9, 1e-6),
            &cand,
        )
        .expect("assess");
    let r = reasons_of(route(Some(&ev), 0.05));
    assert!(
        r.contains(&"coupling:unclosed:state:needs_three_radial_nodes".to_string()),
        "{r:?}"
    );
}

#[test]
fn absent_operator_routes_to_full_liner_with_its_own_reason() {
    assert_eq!(
        route(None, 0.05),
        Route::FullLiner {
            reasons: vec![OPERATOR_ABSENT.to_string()]
        }
    );
}

#[test]
fn unclosed_or_failed_coupling_routes_to_full_liner() {
    let unclosed = evidence(
        CouplingOutcome::UnclosedInterface {
            exchanges: 2,
            reasons: vec!["reverse_core:inflow_omitted".into()],
        },
        0.0,
        0.01,
    );
    let r = reasons_of(route(Some(&unclosed), 0.05));
    assert!(r.contains(&"coupling:unclosed:reverse_core:inflow_omitted".to_string()));

    let failed = evidence(
        CouplingOutcome::NumericalFailure {
            exchanges: 5,
            residuals: vec![0.1; 5],
            reason: "iteration:not_converged_within_cap".into(),
        },
        0.0,
        0.01,
    );
    assert!(!reasons_of(route(Some(&failed), 0.05)).is_empty());
}

#[test]
fn converged_in_domain_evidence_returns_the_coupled_observables() {
    let ok = evidence(converged(), 0.0, 0.01);
    assert!(matches!(
        route(Some(&ok), 0.05),
        Route::Operator { observables } if observables == obs()
    ));
}

#[test]
fn operator_construction_rejects_unordered_envelope() {
    let bad = Envelope {
        lower: [0.5, 0.0, 0.0],
        upper: [0.1, 0.5, 0.05],
    };
    assert!(FrozenLinerOperator::new(identity(), state(1.0), obs(), bad).is_err());
}

#[test]
fn envelope_distance_is_zero_inside_and_positive_outside() {
    let env = Envelope {
        lower: [0.0; 3],
        upper: [1.0; 3],
    };
    assert!(env.hull_distance(&[0.5, 0.5, 0.5]).abs() < 1e-12);
    assert!((env.hull_distance(&[2.0, 0.5, 0.5]) - 1.0).abs() < 1e-12);
    assert!(env.hull_distance(&[f64::NAN, 0.5, 0.5]).is_infinite());
    let f = features(&state(1.0));
    assert!(f[0].abs() < 1e-12);
    assert!((f[2] - 0.01).abs() < 1e-12);
}

// ------------------------------------------------ V5-25-04 ranking, omission and audits

fn obs_scaled(base: &Observables, k: f64) -> Observables {
    Observables {
        reject_split: base.reject_split * k,
        treated_oil_mg_l: base.treated_oil_mg_l * k,
        pressure_drop_pa: base.pressure_drop_pa * k,
    }
}

fn record(id: &str, stratum: Stratum, op_k: f64, clears: bool) -> CandidateRecord {
    let reference = obs();
    CandidateRecord {
        candidate_id: id.into(),
        stratum,
        operator: obs_scaled(&reference, op_k),
        reference,
        reference_clears_gate: clears,
    }
}

/// One promoted, four low-ranked (two would clear the gate), two out-of-domain.
fn population() -> Vec<CandidateRecord> {
    vec![
        record("p1", Stratum::Promoted, 1.01, true),
        record("l1", Stratum::LowRanked, 1.02, true),
        record("l2", Stratum::LowRanked, 1.02, true),
        record("l3", Stratum::LowRanked, 1.02, false),
        record("l4", Stratum::LowRanked, 1.02, false),
        record("o1", Stratum::OutOfDomain, 1.5, false),
        record("o2", Stratum::OutOfDomain, 1.5, false),
    ]
}

fn rates(promoted: f64, low: f64, out: f64) -> BTreeMap<Stratum, f64> {
    BTreeMap::from([
        (Stratum::Promoted, promoted),
        (Stratum::LowRanked, low),
        (Stratum::OutOfDomain, out),
    ])
}

fn criteria(min_per_stratum: usize, max_false_omission_upper95: f64) -> Field<AuditCriteria> {
    admitted(
        AuditCriteria {
            max_false_omission_upper95,
            max_pressure_rel_error: 0.05,
            max_oil_grade_rel_error: 0.05,
            min_audited_per_stratum: min_per_stratum,
        },
        "audit-criteria",
    )
}

#[test]
fn seeded_draws_are_deterministic_and_respect_rates() {
    let pop = population();
    assert_eq!(draw_audits(&pop, &rates(1.0, 1.0, 1.0), 7).len(), pop.len());
    assert!(draw_audits(&pop, &rates(0.0, 0.0, 0.0), 7).is_empty());
    assert_eq!(
        draw_audits(&pop, &rates(0.5, 0.5, 0.5), 7),
        draw_audits(&pop, &rates(0.5, 0.5, 0.5), 7)
    );
}

#[test]
fn full_audit_counts_false_omissions_and_abstentions() {
    let b = run_audit(
        &population(),
        &rates(1.0, 1.0, 1.0),
        1,
        &criteria(1, 0.5),
        &identity(),
    )
    .expect("audit");
    assert_eq!(b.audited, 7);
    assert_eq!(b.rejected, 4);
    assert_eq!(b.false_omissions, 2);
    assert!((b.false_omission_rate - 0.5).abs() < 1e-12);
    assert!(b.false_omission_upper95 > 0.5 && b.false_omission_upper95 < 0.9);
    assert_eq!(b.abstained, 2);
    assert!((b.abstention_rate - 2.0 / 7.0).abs() < 1e-12);
    assert!(b.max_pressure_rel_error < 0.05);
    assert!(mentions(&b.verdict, "false_omission_upper95"));
}

#[test]
fn wilson_upper_bound_matches_reference_value() {
    assert!((wilson_upper(0, 10, 1.959_963_984_540_054) - 0.2775).abs() < 1e-3);
    assert!((wilson_upper(0, 0, 1.96) - 1.0).abs() < 1e-12);
}

#[test]
fn admission_is_bound_to_the_operator_identity() {
    let b = run_audit(
        &population(),
        &rates(1.0, 1.0, 1.0),
        1,
        &criteria(1, 0.99),
        &identity(),
    )
    .expect("audit");
    assert!(b.verdict.is_pass(), "{:?}", b.verdict);
    assert!(admit(&b, &identity()).expect("hash").is_pass());
    let mut other = identity();
    other.operator_id = "frozen-liner-fixture-b".into();
    assert!(mentions(
        &admit(&b, &other).expect("hash"),
        "admission:operator_identity_mismatch"
    ));
}

#[test]
fn scalar_pressure_agreement_cannot_qualify_oil_grade() {
    let c = criteria(1, 0.99);
    let pressure_only = GradeEvidence::PressureOnly {
        pressure_rel_error: 0.0,
    };
    assert!(mentions(
        &judge_downstream_grade(&pressure_only, &c),
        "oil_grade_not_compared"
    ));
    let good = GradeEvidence::OilGrade {
        pressure_rel_error: 0.01,
        oil_grade_rel_error: 0.01,
    };
    assert!(judge_downstream_grade(&good, &c).is_pass());
    let bad = GradeEvidence::OilGrade {
        pressure_rel_error: 0.01,
        oil_grade_rel_error: 0.2,
    };
    assert!(matches!(judge_downstream_grade(&bad, &c), Verdict::Fail(_)));
}

#[test]
fn unsampled_out_of_domain_stratum_is_unresolved_not_pass() {
    let b = run_audit(
        &population(),
        &rates(1.0, 1.0, 0.0),
        1,
        &criteria(1, 0.99),
        &identity(),
    )
    .expect("audit");
    assert_eq!(b.abstained, 0);
    assert!(
        matches!(b.verdict, Verdict::Unresolved(_)),
        "{:?}",
        b.verdict
    );
    assert!(mentions(&b.verdict, "stratum_out_of_domain_under_sampled"));
}

// ------------------------------------------------------ V5-25-05 automatic fallback budget

fn full(id: &str, reasons: &[&str]) -> CampaignCase {
    CampaignCase {
        candidate_id: id.into(),
        sources: vec![Sha256Hex::of_bytes(id.as_bytes())],
        route: Route::FullLiner {
            reasons: reasons.iter().map(|r| r.to_string()).collect(),
        },
    }
}

fn op_case(id: &str) -> CampaignCase {
    CampaignCase {
        candidate_id: id.into(),
        sources: vec![Sha256Hex::of_bytes(id.as_bytes())],
        route: Route::Operator { observables: obs() },
    }
}

#[test]
fn fallback_is_paid_from_budget_and_keeps_its_sources() {
    let cases = vec![op_case("c0"), full("c1", &["ood:outside_envelope"])];
    let mut budget = EvaluationBudget {
        units_remaining: 10.0,
    };
    let plan = plan_campaign(&cases, 2.0, &mut budget).expect("plan");
    assert!(plan.verdict.is_pass(), "{:?}", plan.verdict);
    assert_eq!(plan.fallback_units_spent, 2.0);
    assert_eq!(plan.base_units_spent, 0.0);
    assert_eq!(plan.budget_remaining, 8.0);
    assert!(matches!(
        plan.records[0].disposition,
        Disposition::Operator { .. }
    ));
    assert_eq!(plan.records[1].sources, cases[1].sources);
}

#[test]
fn depleted_budget_abstains_and_never_assumes_a_pass() {
    let cases = vec![full("c1", &["ood:outside_envelope"])];
    let mut budget = EvaluationBudget {
        units_remaining: 1.0,
    };
    let plan = plan_campaign(&cases, 2.0, &mut budget).expect("plan");
    assert!(matches!(
        plan.records[0].disposition,
        Disposition::Abstain { .. }
    ));
    assert!(matches!(plan.verdict, Verdict::Unresolved(_)));
    assert!(mentions(
        &plan.verdict,
        "budget:depleted_cannot_fallback:c1"
    ));
    assert_eq!(plan.fallback_units_spent, 0.0);
    assert_eq!(budget.units_remaining, 1.0);
}

#[test]
fn absent_operator_does_not_block_a_smaller_campaign() {
    let cases = vec![
        full("c0", &[OPERATOR_ABSENT]),
        full("c1", &[OPERATOR_ABSENT]),
        full("c2", &[OPERATOR_ABSENT]),
    ];
    let mut budget = EvaluationBudget {
        units_remaining: 3.0,
    };
    let plan = plan_campaign(&cases, 1.0, &mut budget).expect("plan");
    assert!(plan.verdict.is_pass(), "{:?}", plan.verdict);
    assert_eq!(plan.base_units_spent, 3.0);
    assert_eq!(plan.fallback_units_spent, 0.0);
    assert_eq!(plan.budget_remaining, 0.0);
    assert!(plan
        .records
        .iter()
        .all(|r| matches!(r.disposition, Disposition::FullLiner { .. })));
}

#[test]
fn campaign_rejects_nonpositive_cost_and_bad_budget() {
    let cases = vec![op_case("c0")];
    let mut budget = EvaluationBudget {
        units_remaining: 1.0,
    };
    assert!(plan_campaign(&cases, 0.0, &mut budget).is_err());
    let mut nan_budget = EvaluationBudget {
        units_remaining: f64::NAN,
    };
    assert!(plan_campaign(&cases, 1.0, &mut nan_budget).is_err());
}

#[test]
fn g009_guard_invalid_tolerances_never_admit_comparison_or_operator_route() {
    let same = Observables {
        reject_split: 0.02,
        treated_oil_mg_l: 20.0,
        pressure_drop_pa: 1000.0,
    };
    for bad in [f64::NAN, f64::INFINITY, -1.0] {
        let budget = admitted(
            DecisionBudget {
                reject_split_rel: bad,
                treated_oil_rel: bad,
                pressure_rel: bad,
            },
            "invalid limits",
        );
        assert!(!compare_to_reference(&same, &same, &budget).is_pass());
        let e = evidence(converged(), 0.0, 0.01);
        assert!(matches!(route(Some(&e), bad), Route::FullLiner { .. }));
    }
}

/// Aggregate Wave 11 acceptance check. Fixtures are synthetic cutplane states, not native solver
/// cases; passing these checks never admits the reduced operator for physical use.
#[test]
fn v5_25_02_to_05_sc_dec_039_040_cutplane_corpus_physics_informed_loss_ood_and_fallback() {
    // SC-DEC-039/040: only a family-separated, explicitly synthetic corpus with declared gaps is
    // accepted as a preregistration; calibration evidence cannot be reused as held-out evidence.
    let corpus = judge_preregistration(&protocol(true));
    assert!(corpus.verdict.is_pass(), "{:?}", corpus.verdict);
    assert_eq!(corpus.held_out_cases, 2);
    assert_eq!(corpus.synthetic_cases, 4);
    assert!(!corpus.science_admissible);
    let mut contaminated = protocol(true);
    contaminated.cases[2].carrier_evidence_sha256 =
        contaminated.cases[0].carrier_evidence_sha256.clone();
    assert!(matches!(
        judge_preregistration(&contaminated).verdict,
        Verdict::Fail(_)
    ));

    // The physics-informed residual gate closes mass, momentum, angular momentum and oil flux.
    let balanced = check_balances(&state(1.0), &targets_for(1.0), &tolerances());
    assert!(balanced.verdict.is_pass());
    let mut incorrect_targets = targets_for(1.0).admitted().cloned().expect("targets");
    incorrect_targets.axial_mass_kg_s *= 1.1;
    let residual_penalty = check_balances(
        &state(1.0),
        &admitted(incorrect_targets, "deliberately-imbalanced-targets"),
        &tolerances(),
    );
    assert!(matches!(residual_penalty.verdict, Verdict::Fail(_)));
    assert!(mentions(&residual_penalty.verdict, "axial_mass"));

    // Out-of-domain states abstain to the exact full-liner route.
    let op = operator(0.001);
    let candidate = state(1.0);
    let evidence = op
        .assess(
            &Identity,
            frozen_interface(&candidate),
            &cfg(20, 1.0, 1e-9, 1e-6),
            &candidate,
        )
        .expect("synthetic candidate assessment");
    let reasons = reasons_of(route(Some(&evidence), 0.05));
    assert!(reasons.iter().any(|r| r == "ood:outside_envelope"));

    // Fallback cost is charged and source identity is preserved; depleted budget stays unresolved.
    let cases = vec![
        op_case("aggregate-operator"),
        full("aggregate-ood", &["ood:outside_envelope"]),
    ];
    let mut budget = EvaluationBudget {
        units_remaining: 10.0,
    };
    let plan = plan_campaign(&cases, 2.0, &mut budget).expect("fallback campaign");
    assert!(plan.verdict.is_pass());
    assert_eq!(plan.fallback_units_spent, 2.0);
    assert_eq!(plan.records[1].sources, cases[1].sources);
    let mut exhausted = EvaluationBudget {
        units_remaining: 1.0,
    };
    let refused = plan_campaign(
        &[full("aggregate-ood", &["ood:outside_envelope"])],
        2.0,
        &mut exhausted,
    )
    .expect("budget refusal is represented");
    assert!(matches!(refused.verdict, Verdict::Unresolved(_)));
    assert_eq!(refused.fallback_units_spent, 0.0);
}
