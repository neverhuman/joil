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

#[path = "liner_operator/routing_audit.rs"]
mod routing_audit;
