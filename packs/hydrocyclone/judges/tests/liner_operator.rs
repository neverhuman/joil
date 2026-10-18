//! V5-25 liner operator judge tests. Every number is a SYNTHETIC fixture, checked against an
//! analytic oracle or a hand-derived case. None is a full-liner result or a physical qualification.
use pack_hydrocyclone_judges::liner_operator::{
    accept_tail_relation, admit, check_balances, compare_to_reference, couple, draw_audits,
    features, integrate, judge_downstream_grade, judge_preregistration, plan_campaign, route,
    run_audit, validate_state, wilson_upper, AuditCriteria, BalanceTargets, BalanceTolerances,
    BoundaryFeedback, CampaignCase, CandidateRecord, CaseRecord, CouplingConfig, CouplingOutcome,
    CutNode, DecisionBudget, DeclaredGap, Disposition, Envelope, EvaluationBudget,
    FrozenLinerOperator, GradeEvidence, HeadMap, Interface, LinerCutPlaneState, MeasuredLease,
    Observables, OperatorEvidence, OperatorIdentity, Preregistration, Route, Split, Stratum,
    TailMap, TailRelation, TailResponse, OPERATOR_ABSENT,
};
use pack_hydrocyclone_judges::{Field, Sha256Hex, Verdict};
use std::collections::BTreeMap;
use std::f64::consts::PI;

const PREREG_TOML: &str = include_str!("../../../../experiments/v5-liner-operator.toml");

fn admitted<T>(value: T, tag: &str) -> Field<T> {
    Field::Admitted {
        value,
        source: Sha256Hex::of_bytes(tag.as_bytes()),
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

// ------------------------------------------------------------- shared fixtures

const RHO: f64 = 998.0;
const RADII: [f64; 3] = [0.01, 0.02, 0.03];
const BINS: [f64; 2] = [5e-6, 1e-5];
const FROZEN_OIL: [f64; 2] = [1e-6, 2e-6];

/// Annulus between the outer and inner nodes. A constant flux integrates to flux x area exactly
/// under the trapezoid rule, because `f * 2 pi r` is linear in r.
fn area() -> f64 {
    PI * (0.03_f64.powi(2) - 0.01_f64.powi(2))
}

fn node(r: f64, u: f64, theta: f64, reverse: bool) -> CutNode {
    CutNode {
        radius_m: r,
        axial_ms: u,
        radial_ms: 0.0,
        tangential_ms: theta,
        turbulence_k_m2_s2: 0.01,
        reverse_core: reverse,
        oil_flux_kg_m2_s: vec![1e-3, 2e-3],
    }
}

/// Uniform state: axial speed `u`, no swirl, flagged as reversed when `u` is negative.
fn state(u: f64) -> LinerCutPlaneState {
    LinerCutPlaneState {
        station_m: 0.2,
        carrier_density_kg_m3: RHO,
        oil_bins_m: BINS.to_vec(),
        nodes: RADII.iter().map(|r| node(*r, u, 0.0, u < 0.0)).collect(),
        boundary: BoundaryFeedback {
            reverse_core_inflow_kg_s: 0.0,
        },
        relative_uncertainty: 0.02,
    }
}

fn swirling_state(u: f64, theta: f64) -> LinerCutPlaneState {
    let mut s = state(u);
    for n in &mut s.nodes {
        n.tangential_ms = theta;
    }
    s
}

fn targets_for(u: f64) -> Field<BalanceTargets> {
    let a = area();
    admitted(
        BalanceTargets {
            axial_mass_kg_s: RHO * u * a,
            axial_momentum_n: RHO * u * u * a,
            angular_momentum_n_m: 0.0,
            oil_kg_s: vec![1e-3 * a, 2e-3 * a],
        },
        "balance-targets",
    )
}

fn tolerances() -> Field<BalanceTolerances> {
    admitted(
        BalanceTolerances {
            mass_rel: 1e-9,
            momentum_rel: 1e-9,
            oil_rel: 1e-9,
            reverse_core_rel: 1e-9,
        },
        "balance-tolerances",
    )
}

fn obs() -> Observables {
    Observables {
        reject_split: 0.02,
        treated_oil_mg_l: 150.0,
        pressure_drop_pa: 40_000.0,
    }
}

fn cfg(max: u32, omega: f64, tol: f64, closure: f64) -> CouplingConfig {
    CouplingConfig {
        max_exchanges: max,
        relaxation: omega,
        residual_tol: tol,
        closure_tol: closure,
    }
}

fn identity() -> OperatorIdentity {
    OperatorIdentity {
        operator_id: "frozen-liner-fixture-a".into(),
        model_sha256: Sha256Hex::of_bytes(b"model-a"),
        observer_id: "observer-fixture".into(),
    }
}

// ------------------------------------------------------ V5-25-01 state and conservation

#[test]
fn consistent_state_closes_every_balance_against_analytic_targets() {
    let s = state(1.0);
    assert!(validate_state(&s).is_empty());
    let r = check_balances(&s, &targets_for(1.0), &tolerances());
    assert!(r.verdict.is_pass(), "{:?}", r.verdict);
    let got = r.integrated.expect("valid state integrates");
    assert!((got.axial_mass_kg_s - RHO * area()).abs() < 1e-9 * RHO * area());
    assert_eq!(got.reverse_core_kg_s, 0.0);
}

#[test]
fn mass_imbalance_fails_the_balance() {
    let mut t = targets_for(1.0).admitted().cloned().expect("admitted");
    t.axial_mass_kg_s *= 1.1;
    let r = check_balances(&state(1.0), &admitted(t, "bad-mass"), &tolerances());
    assert!(matches!(r.verdict, Verdict::Fail(_)), "{:?}", r.verdict);
    assert!(mentions(&r.verdict, "axial_mass"));
}

#[test]
fn angular_momentum_mismatch_fails_the_balance() {
    // Swirl is present but the admitted target says none: the angular balance must catch it.
    let r = check_balances(&swirling_state(1.0, 0.5), &targets_for(1.0), &tolerances());
    assert!(matches!(r.verdict, Verdict::Fail(_)), "{:?}", r.verdict);
    assert!(mentions(&r.verdict, "angular_momentum"));
}

#[test]
fn reverse_core_inflow_omission_is_refused() {
    // Every node flows backwards but the boundary returns no reverse-core inflow.
    let r = check_balances(&state(-0.2), &targets_for(-0.2), &tolerances());
    assert!(matches!(r.verdict, Verdict::Fail(_)), "{:?}", r.verdict);
    assert!(mentions(&r.verdict, "reverse_core_inflow_omitted"));
}

#[test]
fn unflagged_reverse_flow_is_an_invalid_state() {
    let mut s = state(1.0);
    s.nodes[1].axial_ms = -0.2;
    let r = check_balances(&s, &targets_for(1.0), &tolerances());
    assert!(
        matches!(r.verdict, Verdict::Unresolved(_)),
        "{:?}",
        r.verdict
    );
    assert!(mentions(&r.verdict, "unflagged_reverse_flow"));
    assert!(r.integrated.is_none());
}

#[test]
fn unordered_radii_are_an_invalid_state() {
    let mut s = state(1.0);
    s.nodes.swap(0, 2);
    let r = check_balances(&s, &targets_for(1.0), &tolerances());
    assert!(
        matches!(r.verdict, Verdict::Unresolved(_)),
        "{:?}",
        r.verdict
    );
    assert!(mentions(&r.verdict, "radii"));
}

#[test]
fn missing_balance_targets_are_unresolved_never_pass() {
    let open = Field::Unresolved {
        reason: "not yet measured".into(),
        owner: "reduced-model owner".into(),
        blocks: vec!["V5-25-01".into()],
    };
    let r = check_balances(&state(1.0), &open, &tolerances());
    assert!(
        matches!(r.verdict, Verdict::Unresolved(_)),
        "{:?}",
        r.verdict
    );
    assert!(mentions(&r.verdict, "balance_targets:not_admitted"));
}

#[test]
fn pressure_only_tail_relation_cannot_replace_full_downstream_physics() {
    let v = accept_tail_relation(&TailRelation::PressureOnly {
        pressure_drop_pa: 40_000.0,
    });
    assert!(matches!(v, Verdict::Unresolved(_)), "{v:?}");
    assert!(mentions(&v, "pressure_only"));
    let full = accept_tail_relation(&TailRelation::FullDownstream(Box::new(state(1.0))));
    assert!(full.is_pass(), "{full:?}");
}

#[test]
fn coupled_observables_must_match_reference_within_budget() {
    let budget = admitted(
        DecisionBudget {
            reject_split_rel: 0.05,
            treated_oil_rel: 0.05,
            pressure_rel: 0.05,
        },
        "decision-budget",
    );
    let close = Observables {
        reject_split: 0.0205,
        treated_oil_mg_l: 152.0,
        pressure_drop_pa: 41_000.0,
    };
    assert!(compare_to_reference(&close, &obs(), &budget).is_pass());
    let oil_off = Observables {
        treated_oil_mg_l: 180.0,
        ..obs()
    };
    assert!(matches!(
        compare_to_reference(&oil_off, &obs(), &budget),
        Verdict::Fail(_)
    ));
}

// ------------------------------------------------------- V5-25-02 preregistered protocol

const TAGS: [&str; 4] = [
    "reverse_core_present",
    "reverse_core_absent",
    "high_swirl",
    "low_swirl",
];

fn case(
    id: &str,
    split: Split,
    family: &str,
    geom: &str,
    evidence: &str,
    tags: &[&str],
    synthetic: bool,
) -> CaseRecord {
    CaseRecord {
        case_id: id.into(),
        split,
        geometry_family: family.into(),
        geometry_sha256: Sha256Hex::of_bytes(geom.as_bytes()),
        carrier_evidence_sha256: Sha256Hex::of_bytes(format!("carrier-{evidence}").as_bytes()),
        transport_evidence_sha256: Sha256Hex::of_bytes(format!("transport-{evidence}").as_bytes()),
        domain_tags: tags.iter().map(|t| t.to_string()).collect(),
        synthetic,
    }
}

fn lease(cores: u32, gpu: bool) -> Field<MeasuredLease> {
    admitted(
        MeasuredLease {
            cpu_cores: cores,
            memory_gib: 6.0,
            output_gib: 2.5,
            gpu,
            lease_sha256: Sha256Hex::of_bytes(b"lease-claude-v5-25"),
        },
        "lease-receipt",
    )
}

fn protocol(synthetic: bool) -> Preregistration {
    Preregistration {
        required_domain_tags: TAGS.iter().map(|t| t.to_string()).collect(),
        declared_gaps: vec![DeclaredGap {
            tag: "low_swirl".into(),
            reason: "no held-out low-swirl family is generated yet".into(),
        }],
        cases: vec![
            case(
                "t1",
                Split::Training,
                "A",
                "gA",
                "eA",
                &["reverse_core_absent", "low_swirl"],
                synthetic,
            ),
            case(
                "c1",
                Split::Calibration,
                "B",
                "gB",
                "eB",
                &["reverse_core_absent", "high_swirl"],
                synthetic,
            ),
            case(
                "h1",
                Split::HeldOut,
                "C",
                "gC",
                "eC",
                &["reverse_core_present", "reverse_core_absent"],
                synthetic,
            ),
            case(
                "h2",
                Split::HeldOut,
                "D",
                "gD",
                "eD",
                &["high_swirl"],
                synthetic,
            ),
        ],
        lease: lease(2, false),
    }
}

#[test]
fn synthetic_protocol_passes_with_declared_gap_and_is_not_science() {
    let r = judge_preregistration(&protocol(true));
    assert!(r.verdict.is_pass(), "{:?}", r.verdict);
    assert_eq!(r.held_out_cases, 2);
    assert_eq!(r.synthetic_cases, 4);
    assert!(!r.science_admissible);
    assert_eq!(r.declared_gaps, vec!["low_swirl".to_string()]);
    assert_eq!(
        r.held_out_tags,
        ["high_swirl", "reverse_core_absent", "reverse_core_present"].map(String::from)
    );
}

#[test]
fn non_synthetic_protocol_that_passes_is_science_admissible() {
    let r = judge_preregistration(&protocol(false));
    assert!(r.verdict.is_pass(), "{:?}", r.verdict);
    assert!(r.science_admissible);
}

#[test]
fn held_out_head_reused_as_calibration_closes_nothing() {
    let mut pre = protocol(true);
    // h1 is given the geometry of calibration case c1.
    pre.cases[2].geometry_sha256 = Sha256Hex::of_bytes(b"gB");
    let r = judge_preregistration(&pre);
    assert!(matches!(r.verdict, Verdict::Fail(_)), "{:?}", r.verdict);
    assert!(mentions(
        &r.verdict,
        "held_out_reuses_training_or_calibration_head:h1"
    ));
    assert!(!r.science_admissible);
}

#[test]
fn held_out_carrier_evidence_reused_from_training_is_refused() {
    let mut pre = protocol(true);
    pre.cases[2].carrier_evidence_sha256 = pre.cases[0].carrier_evidence_sha256.clone();
    let r = judge_preregistration(&pre);
    assert!(mentions(
        &r.verdict,
        "held_out_reuses_training_or_calibration_head:h1"
    ));
}

#[test]
fn geometry_family_overlap_is_refused() {
    let mut pre = protocol(true);
    pre.cases[3].geometry_family = "A".into();
    let r = judge_preregistration(&pre);
    assert!(matches!(r.verdict, Verdict::Fail(_)), "{:?}", r.verdict);
    assert!(mentions(&r.verdict, "family_overlap:A"));
}

#[test]
fn unexplained_coverage_gap_is_unresolved() {
    let mut pre = protocol(true);
    pre.declared_gaps.clear();
    let r = judge_preregistration(&pre);
    assert!(
        matches!(r.verdict, Verdict::Unresolved(_)),
        "{:?}",
        r.verdict
    );
    assert!(mentions(&r.verdict, "coverage:unexplained_gap:low_swirl"));
}

#[test]
fn lease_beyond_initial_allocation_fails() {
    let mut pre = protocol(true);
    pre.lease = lease(8, false);
    assert!(mentions(
        &judge_preregistration(&pre).verdict,
        "lease:exceeds_initial_joil_allocation"
    ));
    pre.lease = lease(2, true);
    assert!(mentions(
        &judge_preregistration(&pre).verdict,
        "lease:exceeds_initial_joil_allocation"
    ));
}

#[test]
fn unmeasured_lease_is_unresolved() {
    let mut pre = protocol(true);
    pre.lease = Field::Unresolved {
        reason: "lease not measured".into(),
        owner: "claude".into(),
        blocks: vec!["V5-25-02".into()],
    };
    let r = judge_preregistration(&pre);
    assert!(
        matches!(r.verdict, Verdict::Unresolved(_)),
        "{:?}",
        r.verdict
    );
    assert!(mentions(&r.verdict, "lease:not_measured_or_admitted"));
}

#[test]
fn protocol_without_held_out_cases_is_unresolved() {
    let mut pre = protocol(true);
    pre.cases.retain(|c| c.split != Split::HeldOut);
    let r = judge_preregistration(&pre);
    assert!(mentions(&r.verdict, "split:no_held_out_cases"));
    assert_eq!(r.held_out_cases, 0);
}

#[test]
fn experiment_file_declares_synthetic_fixture_and_no_admission() {
    assert!(PREREG_TOML.contains("status = \"preregistered_synthetic_fixture\""));
    assert!(PREREG_TOML.contains("scientific_admission = false"));
    assert!(PREREG_TOML.contains("[held_out]"));
    assert!(PREREG_TOML.contains("reuse_as_validation = \"forbidden\""));
}

// ----------------------------------------------- V5-25-03 coupled head/tail iteration

/// Identity head: returns what it was given.
struct Identity;

impl HeadMap for Identity {
    fn respond(&self, returned: &Interface) -> Interface {
        returned.clone()
    }
}

/// Head that scales the tail's interface by a fixed gain.
struct Scaled(f64);

impl HeadMap for Scaled {
    fn respond(&self, returned: &Interface) -> Interface {
        Interface {
            oil_kg_s: returned.oil_kg_s.iter().map(|x| self.0 * x).collect(),
            reverse_core_kg_s: self.0 * returned.reverse_core_kg_s,
        }
    }
}

/// Head that drops reverse-core inflow entirely.
struct NoReverse;

impl HeadMap for NoReverse {
    fn respond(&self, returned: &Interface) -> Interface {
        Interface {
            oil_kg_s: returned.oil_kg_s.clone(),
            reverse_core_kg_s: 0.0,
        }
    }
}

/// Synthetic tail: the response to the sent interface is `frozen + contraction * (sent - frozen)`.
/// Its own fluxes are the frozen values.
struct TestTail {
    frozen_oil: Vec<f64>,
    frozen_reverse: f64,
    contraction: f64,
    infeasible: Vec<String>,
    poison: bool,
}

impl TailMap for TestTail {
    fn respond(&self, sent: &Interface) -> TailResponse {
        let c = self.contraction;
        let oil: Vec<f64> = self
            .frozen_oil
            .iter()
            .zip(&sent.oil_kg_s)
            .map(|(f, s)| f + c * (s - f))
            .collect();
        let oil = if self.poison {
            vec![f64::NAN; oil.len()]
        } else {
            oil
        };
        TailResponse {
            returned: Interface {
                oil_kg_s: oil,
                reverse_core_kg_s: self.frozen_reverse
                    + c * (sent.reverse_core_kg_s - self.frozen_reverse),
            },
            integrated_oil_kg_s: self.frozen_oil.clone(),
            integrated_reverse_kg_s: self.frozen_reverse,
            observables: obs(),
            infeasible: self.infeasible.clone(),
        }
    }
}

fn test_tail(contraction: f64, frozen_reverse: f64) -> TestTail {
    TestTail {
        frozen_oil: FROZEN_OIL.to_vec(),
        frozen_reverse,
        contraction,
        infeasible: Vec::new(),
        poison: false,
    }
}

#[test]
fn coupling_converges_and_reports_measured_exchange_count() {
    let initial = Interface {
        oil_kg_s: vec![0.0; 2],
        reverse_core_kg_s: 0.0,
    };
    let out = couple(
        &Identity,
        &test_tail(0.7, 0.05),
        initial,
        &cfg(500, 0.5, 1e-6, 1e-4),
    );
    match out {
        CouplingOutcome::Converged {
            exchanges,
            residuals,
            observables,
            ..
        } => {
            assert!(
                exchanges > 3,
                "exchange count must be measured, got {exchanges}"
            );
            assert_eq!(residuals.len(), exchanges as usize);
            assert!(residuals.last().is_some_and(|r| *r <= 1e-6));
            assert_eq!(observables, obs());
        }
        other => panic!("expected convergence, got {other:?}"),
    }
}

#[test]
fn reverse_core_inflow_omission_forces_fallback() {
    let initial = Interface {
        oil_kg_s: FROZEN_OIL.to_vec(),
        reverse_core_kg_s: 0.0,
    };
    let out = couple(
        &NoReverse,
        &test_tail(0.0, 0.05),
        initial,
        &cfg(50, 1.0, 1e-9, 1e-6),
    );
    match &out {
        CouplingOutcome::UnclosedInterface { reasons, .. } => {
            assert!(reasons.contains(&"reverse_core:inflow_omitted".to_string()));
        }
        other => panic!("expected unclosed interface, got {other:?}"),
    }
    assert!(!out.fallback_reasons().is_empty());
}

#[test]
fn unclosed_oil_flux_forces_fallback() {
    let initial = Interface {
        oil_kg_s: FROZEN_OIL.to_vec(),
        reverse_core_kg_s: 0.0,
    };
    let out = couple(
        &Scaled(0.5),
        &test_tail(0.0, 0.0),
        initial,
        &cfg(50, 1.0, 1e-9, 1e-6),
    );
    match out {
        CouplingOutcome::UnclosedInterface { reasons, .. } => {
            assert!(reasons.contains(&"interface:oil_bin_0_unclosed".to_string()));
            assert!(reasons.contains(&"interface:oil_bin_1_unclosed".to_string()));
        }
        other => panic!("expected unclosed interface, got {other:?}"),
    }
}

#[test]
fn physical_infeasibility_is_reported_apart_from_numerical_failure() {
    let mut tail = test_tail(0.0, 0.0);
    tail.infeasible = vec!["core:choked".to_string()];
    let initial = Interface {
        oil_kg_s: FROZEN_OIL.to_vec(),
        reverse_core_kg_s: 0.0,
    };
    match couple(&Identity, &tail, initial, &cfg(50, 1.0, 1e-9, 1e-6)) {
        CouplingOutcome::PhysicalInfeasible { exchanges, reasons } => {
            assert_eq!(exchanges, 1);
            assert_eq!(reasons, vec!["core:choked".to_string()]);
        }
        other => panic!("expected physical infeasibility, got {other:?}"),
    }
}

#[test]
fn non_finite_tail_response_is_a_numerical_failure() {
    let mut tail = test_tail(0.0, 0.0);
    tail.poison = true;
    let initial = Interface {
        oil_kg_s: FROZEN_OIL.to_vec(),
        reverse_core_kg_s: 0.0,
    };
    match couple(&Identity, &tail, initial, &cfg(50, 1.0, 1e-9, 1e-6)) {
        CouplingOutcome::NumericalFailure { reason, .. } => {
            assert_eq!(reason, "tail:nonfinite_response");
        }
        other => panic!("expected numerical failure, got {other:?}"),
    }
}

#[test]
fn divergent_iteration_stops_at_the_exchange_cap_as_numerical_failure() {
    let initial = Interface {
        oil_kg_s: FROZEN_OIL.to_vec(),
        reverse_core_kg_s: 0.0,
    };
    match couple(
        &Scaled(2.0),
        &test_tail(1.0, 0.0),
        initial,
        &cfg(10, 1.0, 1e-9, 1e-6),
    ) {
        CouplingOutcome::NumericalFailure {
            exchanges,
            residuals,
            reason,
        } => {
            assert_eq!(exchanges, 10);
            assert_eq!(residuals.len(), 10);
            assert_eq!(reason, "iteration:not_converged_within_cap");
        }
        other => panic!("expected numerical failure, got {other:?}"),
    }
}

#[test]
fn invalid_coupling_configuration_is_a_numerical_failure() {
    let initial = Interface {
        oil_kg_s: FROZEN_OIL.to_vec(),
        reverse_core_kg_s: 0.0,
    };
    match couple(
        &Identity,
        &test_tail(0.0, 0.0),
        initial,
        &cfg(0, 1.0, 1e-9, 1e-6),
    ) {
        CouplingOutcome::NumericalFailure {
            exchanges, reason, ..
        } => {
            assert_eq!(exchanges, 0);
            assert_eq!(reason, "config:invalid");
        }
        other => panic!("expected numerical failure, got {other:?}"),
    }
}

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
