//! V5-25 optional frozen-liner response operator (software fixtures only).
//!
//! The operator stands in for the full-liner reference only inside its declared envelope, only
//! when the cut-plane interface closes, and only when the state's uncertainty is below an admitted
//! limit. Everything else dispatches the full-liner reference. Abstention is never a pass, and an
//! absent operator never blocks a smaller campaign. Every fixture is SYNTHETIC: the full-liner
//! reference is not run here, no downstream oil grade is qualified, and nothing in this module is
//! a physical validation.
use crate::{canonical_hash, invalid, Field, JudgeError, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f64::consts::PI;

/// Two-sided 95 % normal quantile for the Wilson interval.
const Z95: f64 = 1.959_963_984_540_054;
/// Initial caps for optional joil jobs (AGENTS.md). A new user-authorized allocation supersedes them.
pub const INITIAL_MAX_CORES: u32 = 4;
pub const INITIAL_MAX_MEMORY_GIB: f64 = 12.0;
pub const INITIAL_MAX_OUTPUT_GIB: f64 = 5.0;
/// Reason recorded when no operator is present. It routes to the full-liner reference and never
/// blocks the campaign.
pub const OPERATOR_ABSENT: &str = "operator:absent";

fn open(reason: impl Into<String>) -> Verdict {
    Verdict::Unresolved(vec![reason.into()])
}

fn finite(values: &[f64]) -> bool {
    values.iter().all(|v| v.is_finite())
}

/// Invalid values or limits cannot pass a relative-error/uncertainty check.
fn exceeds(value: f64, limit: f64) -> bool {
    !value.is_finite() || !limit.is_finite() || value < 0.0 || limit < 0.0 || value > limit
}

/// Relative difference with a symmetric scale. Equal values, including two zeros, agree exactly.
pub(crate) fn rel_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).abs();
    if d == 0.0 {
        0.0
    } else {
        d / a.abs().max(b.abs())
    }
}

// ------------------------------------------------------------------ V5-25-01 interface state

/// One radial node of an admitted cut plane. Velocities are m/s, turbulent kinetic energy is
/// m^2/s^2, and each oil entry is the oil mass flux (kg/m^2/s) crossing the plane in one size bin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CutNode {
    pub radius_m: f64,
    pub axial_ms: f64,
    pub radial_ms: f64,
    pub tangential_ms: f64,
    pub turbulence_k_m2_s2: f64,
    /// Must be set wherever the axial velocity is negative; an unflagged backflow is refused.
    pub reverse_core: bool,
    /// Aligned with `LinerCutPlaneState::oil_bins_m`.
    pub oil_flux_kg_m2_s: Vec<f64>,
}

/// Return flow the downstream boundary feeds back to the tail.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryFeedback {
    /// Reverse-core mass flow the boundary returns, kg/s, non-negative.
    pub reverse_core_inflow_kg_s: f64,
}

/// Complete interface state at one admitted cut plane.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinerCutPlaneState {
    pub station_m: f64,
    pub carrier_density_kg_m3: f64,
    /// Oil size-bin diameters in metres, strictly increasing and positive.
    pub oil_bins_m: Vec<f64>,
    /// Radially ordered nodes, at least three.
    pub nodes: Vec<CutNode>,
    pub boundary: BoundaryFeedback,
    /// One-sigma relative uncertainty on the integrated fluxes of this state.
    pub relative_uncertainty: f64,
}

/// Structural validity of a cut-plane state. An empty list means the state can be integrated.
pub fn validate_state(state: &LinerCutPlaneState) -> Vec<String> {
    let mut bad = Vec::new();
    let scalars = [
        state.station_m,
        state.carrier_density_kg_m3,
        state.boundary.reverse_core_inflow_kg_s,
        state.relative_uncertainty,
    ];
    if !finite(&scalars)
        || state.carrier_density_kg_m3 <= 0.0
        || state.boundary.reverse_core_inflow_kg_s < 0.0
        || state.relative_uncertainty < 0.0
    {
        bad.push("state:invalid_scalar".to_string());
    }
    let bins = &state.oil_bins_m;
    if bins.is_empty()
        || !bins.iter().all(|d| d.is_finite() && *d > 0.0)
        || !bins.windows(2).all(|w| w[0] < w[1])
    {
        bad.push("state:oil_bins_not_increasing_positive".to_string());
    }
    if state.nodes.len() < 3 {
        bad.push("state:needs_three_radial_nodes".to_string());
        return bad;
    }
    if !state
        .nodes
        .iter()
        .all(|n| n.radius_m.is_finite() && n.radius_m > 0.0)
        || !state
            .nodes
            .windows(2)
            .all(|w| w[0].radius_m < w[1].radius_m)
    {
        bad.push("state:radii_not_strictly_increasing_positive".to_string());
    }
    let nodes_ok = state.nodes.iter().all(|n| {
        finite(&[
            n.axial_ms,
            n.radial_ms,
            n.tangential_ms,
            n.turbulence_k_m2_s2,
        ]) && n.turbulence_k_m2_s2 >= 0.0
            && n.oil_flux_kg_m2_s.len() == bins.len()
            && finite(&n.oil_flux_kg_m2_s)
    });
    if !nodes_ok {
        bad.push("state:node_fields_invalid_or_bin_count".to_string());
    }
    if state
        .nodes
        .iter()
        .any(|n| n.axial_ms < 0.0 && !n.reverse_core)
    {
        bad.push("state:unflagged_reverse_flow".to_string());
    }
    bad
}

/// Trapezoid integral over the admitted nodes of `f(node) * 2 pi r` in radius. Exact for a
/// constant `f` (the area of the annulus); it covers only the radii the state supplies.
fn annulus_integral(nodes: &[CutNode], f: impl Fn(&CutNode) -> f64) -> f64 {
    nodes
        .windows(2)
        .map(|w| {
            let g0 = f(&w[0]) * 2.0 * PI * w[0].radius_m;
            let g1 = f(&w[1]) * 2.0 * PI * w[1].radius_m;
            0.5 * (g0 + g1) * (w[1].radius_m - w[0].radius_m)
        })
        .sum()
}

/// Fluxes a cut-plane state carries through the plane.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Integrated {
    pub axial_mass_kg_s: f64,
    pub axial_momentum_n: f64,
    pub angular_momentum_n_m: f64,
    pub oil_kg_s: Vec<f64>,
    /// Reverse-core mass flow through the plane, kg/s, positive when the core flows backwards.
    pub reverse_core_kg_s: f64,
}

/// Integrate a state. The caller must have checked `validate_state` first.
pub fn integrate(state: &LinerCutPlaneState) -> Integrated {
    let rho = state.carrier_density_kg_m3;
    let reverse = -rho
        * annulus_integral(&state.nodes, |n| {
            if n.reverse_core {
                n.axial_ms.min(0.0)
            } else {
                0.0
            }
        });
    Integrated {
        axial_mass_kg_s: rho * annulus_integral(&state.nodes, |n| n.axial_ms),
        axial_momentum_n: rho * annulus_integral(&state.nodes, |n| n.axial_ms * n.axial_ms),
        angular_momentum_n_m: rho
            * annulus_integral(&state.nodes, |n| n.tangential_ms * n.radius_m * n.axial_ms),
        oil_kg_s: (0..state.oil_bins_m.len())
            .map(|k| {
                annulus_integral(&state.nodes, |n| {
                    n.oil_flux_kg_m2_s.get(k).copied().unwrap_or(f64::NAN)
                })
            })
            .collect(),
        reverse_core_kg_s: reverse,
    }
}

/// Balance targets admitted for one cut plane.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BalanceTargets {
    pub axial_mass_kg_s: f64,
    pub axial_momentum_n: f64,
    pub angular_momentum_n_m: f64,
    /// Oil mass flow per size bin admitted for this plane.
    pub oil_kg_s: Vec<f64>,
}

/// Admitted relative tolerances for each balance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BalanceTolerances {
    pub mass_rel: f64,
    pub momentum_rel: f64,
    pub oil_rel: f64,
    pub reverse_core_rel: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BalanceReport {
    pub integrated: Option<Integrated>,
    pub verdict: Verdict,
}

fn residual_fail(fails: &mut Vec<String>, name: &str, actual: f64, target: f64, tol: f64) {
    let r = rel_diff(actual, target);
    if exceeds(r, tol) {
        fails.push(format!("balance:{name}_residual_{r:.3e}_exceeds_{tol:.3e}"));
    }
}

/// Independently check mass, axial and angular momentum, per-bin oil, and the reverse-core inflow
/// against admitted targets. A residual outside tolerance is `Fail`; missing or malformed inputs
/// are `Unresolved`. A closed balance on a state that is not valid is never `Pass`.
pub fn check_balances(
    state: &LinerCutPlaneState,
    targets: &Field<BalanceTargets>,
    tol: &Field<BalanceTolerances>,
) -> BalanceReport {
    let invalid_state = validate_state(state);
    if !invalid_state.is_empty() {
        return BalanceReport {
            integrated: None,
            verdict: Verdict::Unresolved(invalid_state),
        };
    }
    let got = integrate(state);
    let mut open_items = Vec::new();
    if targets.admitted().is_none() {
        open_items.push("balance_targets:not_admitted".to_string());
    }
    if tol.admitted().is_none() {
        open_items.push("balance_tolerances:not_admitted".to_string());
    }
    let (Some(t), Some(k)) = (targets.admitted(), tol.admitted()) else {
        return BalanceReport {
            integrated: Some(got),
            verdict: Verdict::Unresolved(open_items),
        };
    };
    if t.oil_kg_s.len() != got.oil_kg_s.len() {
        open_items.push("balance:oil_target_bin_count_mismatch".to_string());
        return BalanceReport {
            integrated: Some(got),
            verdict: Verdict::Unresolved(open_items),
        };
    }
    let mut fails = Vec::new();
    residual_fail(
        &mut fails,
        "axial_mass",
        got.axial_mass_kg_s,
        t.axial_mass_kg_s,
        k.mass_rel,
    );
    residual_fail(
        &mut fails,
        "axial_momentum",
        got.axial_momentum_n,
        t.axial_momentum_n,
        k.momentum_rel,
    );
    residual_fail(
        &mut fails,
        "angular_momentum",
        got.angular_momentum_n_m,
        t.angular_momentum_n_m,
        k.momentum_rel,
    );
    for (i, (g, e)) in got.oil_kg_s.iter().zip(&t.oil_kg_s).enumerate() {
        residual_fail(&mut fails, &format!("oil_bin_{i}"), *g, *e, k.oil_rel);
    }
    let fed = state.boundary.reverse_core_inflow_kg_s;
    if got.reverse_core_kg_s > 0.0 && fed == 0.0 {
        fails.push("balance:reverse_core_inflow_omitted".to_string());
    } else {
        residual_fail(
            &mut fails,
            "reverse_core",
            got.reverse_core_kg_s,
            fed,
            k.reverse_core_rel,
        );
    }
    let verdict = if !fails.is_empty() {
        Verdict::Fail(fails)
    } else if !open_items.is_empty() {
        Verdict::Unresolved(open_items)
    } else {
        Verdict::Pass
    };
    BalanceReport {
        integrated: Some(got),
        verdict,
    }
}

/// A candidate tail relation. Only the full downstream state may close the tail. A pressure-drop
/// relation is refused whatever its fit quality.
#[derive(Clone, Debug, PartialEq)]
pub enum TailRelation {
    PressureOnly { pressure_drop_pa: f64 },
    FullDownstream(Box<LinerCutPlaneState>),
}

/// `Pass` means only that the full state is admissible as a tail basis. Balances still have to be
/// checked with `check_balances`.
pub fn accept_tail_relation(relation: &TailRelation) -> Verdict {
    match relation {
        TailRelation::PressureOnly { .. } => {
            open("tail:pressure_only_relation_cannot_replace_full_downstream_physics")
        }
        TailRelation::FullDownstream(state) => {
            let bad = validate_state(state);
            if bad.is_empty() {
                Verdict::Pass
            } else {
                Verdict::Unresolved(bad)
            }
        }
    }
}

// ----------------------------------------------------- V5-25-02 preregistered case protocol

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Split {
    Training,
    Calibration,
    HeldOut,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseRecord {
    pub case_id: String,
    pub split: Split,
    pub geometry_family: String,
    /// Full accepted geometry, identified by its digest.
    pub geometry_sha256: Sha256Hex,
    pub carrier_evidence_sha256: Sha256Hex,
    pub transport_evidence_sha256: Sha256Hex,
    /// Mechanism and state-domain tags the case's generated states cover.
    pub domain_tags: Vec<String>,
    /// Generated fixtures are synthetic and can never close a scientific gate.
    pub synthetic: bool,
}

/// Compute resources measured for the case-generation and reference lease.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasuredLease {
    pub cpu_cores: u32,
    pub memory_gib: f64,
    pub output_gib: f64,
    pub gpu: bool,
    pub lease_sha256: Sha256Hex,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeclaredGap {
    pub tag: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preregistration {
    pub required_domain_tags: Vec<String>,
    pub declared_gaps: Vec<DeclaredGap>,
    pub cases: Vec<CaseRecord>,
    pub lease: Field<MeasuredLease>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreregistrationReport {
    /// Tags covered by at least one held-out case.
    pub held_out_tags: Vec<String>,
    /// Required tags that no held-out case covers and that are declared as gaps.
    pub declared_gaps: Vec<String>,
    pub held_out_cases: usize,
    pub synthetic_cases: usize,
    /// True only when the protocol passes and every case is non-synthetic.
    pub science_admissible: bool,
    pub verdict: Verdict,
}

/// Check the preregistered case protocol. Geometry-family separation and held-out independence
/// are `Fail`, because a reused head or family leaks the answer into validation. Missing evidence,
/// an unmeasured lease or an unexplained coverage gap is `Unresolved`.
pub fn judge_preregistration(pre: &Preregistration) -> PreregistrationReport {
    let mut fails = Vec::new();
    let mut open_items = Vec::new();
    match pre.lease.admitted() {
        Some(l) => {
            let within = l.cpu_cores >= 1
                && l.cpu_cores <= INITIAL_MAX_CORES
                && l.memory_gib > 0.0
                && l.memory_gib <= INITIAL_MAX_MEMORY_GIB
                && l.output_gib >= 0.0
                && l.output_gib <= INITIAL_MAX_OUTPUT_GIB
                && !l.gpu;
            if !within {
                fails.push("lease:exceeds_initial_joil_allocation".to_string());
            }
        }
        None => open_items.push("lease:not_measured_or_admitted".to_string()),
    }

    let mut ids = std::collections::BTreeSet::new();
    for c in &pre.cases {
        if !ids.insert(c.case_id.as_str()) {
            fails.push(format!("split:duplicate_case_id:{}", c.case_id));
        }
    }
    let training: Vec<&CaseRecord> = pre
        .cases
        .iter()
        .filter(|c| c.split != Split::HeldOut)
        .collect();
    let held: Vec<&CaseRecord> = pre
        .cases
        .iter()
        .filter(|c| c.split == Split::HeldOut)
        .collect();
    if held.is_empty() {
        open_items.push("split:no_held_out_cases".to_string());
    }
    for h in &held {
        for t in &training {
            if t.geometry_family == h.geometry_family {
                fails.push(format!("split:family_overlap:{}", h.geometry_family));
                break;
            }
        }
        if training.iter().any(|t| {
            t.geometry_sha256 == h.geometry_sha256
                || t.carrier_evidence_sha256 == h.carrier_evidence_sha256
                || t.transport_evidence_sha256 == h.transport_evidence_sha256
        }) {
            fails.push(format!(
                "split:held_out_reuses_training_or_calibration_head:{}",
                h.case_id
            ));
        }
    }

    let held_tags: std::collections::BTreeSet<&str> = held
        .iter()
        .flat_map(|c| c.domain_tags.iter().map(String::as_str))
        .collect();
    let mut declared = Vec::new();
    for tag in &pre.required_domain_tags {
        if held_tags.contains(tag.as_str()) {
            continue;
        }
        if pre.declared_gaps.iter().any(|g| &g.tag == tag) {
            declared.push(tag.clone());
        } else {
            open_items.push(format!("coverage:unexplained_gap:{tag}"));
        }
    }
    let synthetic_cases = pre.cases.iter().filter(|c| c.synthetic).count();
    let verdict = if !fails.is_empty() {
        Verdict::Fail(fails)
    } else if !open_items.is_empty() {
        Verdict::Unresolved(open_items)
    } else {
        Verdict::Pass
    };
    PreregistrationReport {
        held_out_tags: held_tags.iter().map(|t| t.to_string()).collect(),
        declared_gaps: declared,
        held_out_cases: held.len(),
        synthetic_cases,
        science_admissible: verdict.is_pass() && synthetic_cases == 0,
        verdict,
    }
}

/// Build a fixed 12-case substitute corpus with disjoint geometry families and evidence IDs.
/// The corpus is a protocol fixture only; its synthetic labels do not admit physical science.
pub fn build_preregistered_full_liner_corpus(
    lease: Field<MeasuredLease>,
    declared_gaps: Vec<DeclaredGap>,
) -> Result<(Preregistration, PreregistrationReport), JudgeError> {
    if lease.admitted().is_none() {
        return Err(invalid("full_liner:measured_lease_required"));
    }
    let plan = [
        ("cyl_conical_a", Split::Training, 4),
        ("cyl_conical_b", Split::Training, 4),
        ("tapered_swirl_c", Split::Calibration, 2),
        ("helical_liner_d", Split::HeldOut, 1),
        ("stepped_cone_e", Split::HeldOut, 1),
    ];
    let tags = [
        "reverse_core",
        "swirl",
        "oil_grade",
        "pressure_drop",
        "low_flow",
        "high_flow",
    ];
    let mut cases = Vec::new();
    let mut index = 0usize;
    for (family, split, count) in plan {
        for member in 0..count {
            let case_id = format!("full-liner-{family}-{member:02}");
            let digest = |kind: &str| Sha256Hex::of_bytes(format!("{case_id}:{kind}").as_bytes());
            let domain_tags = if split == Split::HeldOut {
                if family == "helical_liner_d" {
                    tags[..3].iter().map(|tag| tag.to_string()).collect()
                } else {
                    tags[3..].iter().map(|tag| tag.to_string()).collect()
                }
            } else {
                vec![
                    tags[index % tags.len()].to_string(),
                    tags[(index + 1) % tags.len()].to_string(),
                ]
            };
            cases.push(CaseRecord {
                case_id: case_id.clone(),
                split,
                geometry_family: family.to_string(),
                geometry_sha256: digest("geometry"),
                carrier_evidence_sha256: digest("carrier"),
                transport_evidence_sha256: digest("transport"),
                domain_tags,
                synthetic: true,
            });
            index += 1;
        }
    }
    let pre = Preregistration {
        required_domain_tags: tags
            .iter()
            .map(|x| x.to_string())
            .chain(std::iter::once("wet_liner_surface_chemistry".to_string()))
            .collect(),
        declared_gaps,
        cases,
        lease,
    };
    let report = judge_preregistration(&pre);
    if !report.verdict.is_pass() {
        return Err(invalid(format!(
            "full_liner:preregistration_rejected:{:?}",
            report.verdict
        )));
    }
    Ok((pre, report))
}

/// Parse and rejudge a persisted corpus before it is used by a consumer.
pub fn verify_full_liner_corpus_json(raw_json: &str) -> Result<PreregistrationReport, JudgeError> {
    let pre: Preregistration = serde_json::from_str(raw_json)
        .map_err(|e| invalid(format!("full_liner:invalid_json:{e}")))?;
    let report = judge_preregistration(&pre);
    if report.verdict.is_pass() {
        Ok(report)
    } else {
        Err(invalid(format!(
            "full_liner:verification_failed:{:?}",
            report.verdict
        )))
    }
}

// ---------------------------------------------------- V5-25-03 coupled head/tail iteration

/// Interface exchanged each iteration: oil mass flow per size bin and reverse-core mass flow.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Interface {
    pub oil_kg_s: Vec<f64>,
    pub reverse_core_kg_s: f64,
}

/// Downstream observables a coupled solve reports.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observables {
    pub reject_split: f64,
    pub treated_oil_mg_l: f64,
    pub pressure_drop_pa: f64,
}

/// What the tail returns to the head for one exchange.
#[derive(Clone, Debug, PartialEq)]
pub struct TailResponse {
    pub returned: Interface,
    /// Oil flux the tail's own state carries through the plane, per bin.
    pub integrated_oil_kg_s: Vec<f64>,
    /// Reverse-core flux the tail's own state carries, kg/s.
    pub integrated_reverse_kg_s: f64,
    pub observables: Observables,
    /// Physical infeasibility the tail reports, such as a choked core. Not an iteration failure.
    pub infeasible: Vec<String>,
}

pub trait HeadMap {
    /// Head response: the interface it sends to the tail for the tail's returned interface.
    fn respond(&self, returned: &Interface) -> Interface;
}

pub trait TailMap {
    /// Tail response to the interface the head sent.
    fn respond(&self, sent: &Interface) -> TailResponse;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CouplingConfig {
    pub max_exchanges: u32,
    /// Relaxation weight in (0, 1].
    pub relaxation: f64,
    /// Relative residual at which the exchange is converged.
    pub residual_tol: f64,
    /// Relative closure tolerance between the sent interface and the tail's own fluxes.
    pub closure_tol: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CouplingOutcome {
    Converged {
        exchanges: u32,
        residuals: Vec<f64>,
        observables: Observables,
        interface: Interface,
    },
    /// Iteration failure: non-finite values, a malformed response, or no convergence within the cap.
    NumericalFailure {
        exchanges: u32,
        residuals: Vec<f64>,
        reason: String,
    },
    /// The tail reports that the interface cannot be realised.
    PhysicalInfeasible {
        exchanges: u32,
        reasons: Vec<String>,
    },
    /// The converged interface does not close against the tail's flux: reverse-core inflow
    /// omitted, or oil flux unclosed in a bin.
    UnclosedInterface {
        exchanges: u32,
        reasons: Vec<String>,
    },
}

impl CouplingOutcome {
    /// Exchanges performed. The count is measured, not assumed.
    pub fn exchanges(&self) -> u32 {
        match self {
            Self::Converged { exchanges, .. }
            | Self::NumericalFailure { exchanges, .. }
            | Self::PhysicalInfeasible { exchanges, .. }
            | Self::UnclosedInterface { exchanges, .. } => *exchanges,
        }
    }

    /// Reasons that send the candidate to the full-liner reference. Empty only when converged.
    pub fn fallback_reasons(&self) -> Vec<String> {
        match self {
            Self::Converged { .. } => Vec::new(),
            Self::NumericalFailure { reason, .. } => vec![format!("coupling:numerical:{reason}")],
            Self::PhysicalInfeasible { reasons, .. } => reasons
                .iter()
                .map(|r| format!("coupling:infeasible:{r}"))
                .collect(),
            Self::UnclosedInterface { reasons, .. } => reasons
                .iter()
                .map(|r| format!("coupling:unclosed:{r}"))
                .collect(),
        }
    }
}

fn interface_finite(i: &Interface) -> bool {
    finite(&i.oil_kg_s) && i.reverse_core_kg_s.is_finite()
}

fn tail_response_finite(t: &TailResponse) -> bool {
    interface_finite(&t.returned)
        && finite(&t.integrated_oil_kg_s)
        && t.integrated_reverse_kg_s.is_finite()
        && t.observables.reject_split.is_finite()
        && t.observables.treated_oil_mg_l.is_finite()
        && t.observables.pressure_drop_pa.is_finite()
}

fn interface_residual(a: &Interface, b: &Interface) -> f64 {
    a.oil_kg_s
        .iter()
        .zip(&b.oil_kg_s)
        .map(|(x, y)| rel_diff(*x, *y))
        .fold(rel_diff(a.reverse_core_kg_s, b.reverse_core_kg_s), f64::max)
}

/// Blend the sent interface toward the head's proposal. The oil total is carried by the proposal:
/// the blended bins are rescaled to its total, so the blend neither creates nor loses oil.
fn conservative_relax(current: &Interface, proposal: &Interface, omega: f64) -> Interface {
    let mut oil: Vec<f64> = current
        .oil_kg_s
        .iter()
        .zip(&proposal.oil_kg_s)
        .map(|(c, p)| (1.0 - omega) * c + omega * p)
        .collect();
    let blended: f64 = oil.iter().sum();
    let target: f64 = proposal.oil_kg_s.iter().sum();
    if blended > 0.0 {
        let scale = target / blended;
        oil.iter_mut().for_each(|v| *v *= scale);
    }
    Interface {
        oil_kg_s: oil,
        reverse_core_kg_s: (1.0 - omega) * current.reverse_core_kg_s
            + omega * proposal.reverse_core_kg_s,
    }
}

fn closure_reasons(sent: &Interface, tail: &TailResponse, tol: f64) -> Vec<String> {
    let mut reasons = Vec::new();
    if tail.integrated_reverse_kg_s > 0.0 && sent.reverse_core_kg_s == 0.0 {
        reasons.push("reverse_core:inflow_omitted".to_string());
    } else if exceeds(
        rel_diff(sent.reverse_core_kg_s, tail.integrated_reverse_kg_s),
        tol,
    ) {
        reasons.push("reverse_core:inflow_unclosed".to_string());
    }
    if sent.oil_kg_s.len() != tail.integrated_oil_kg_s.len() {
        reasons.push("interface:oil_bin_count_mismatch".to_string());
    } else {
        for (i, (s, c)) in sent
            .oil_kg_s
            .iter()
            .zip(&tail.integrated_oil_kg_s)
            .enumerate()
        {
            if exceeds(rel_diff(*s, *c), tol) {
                reasons.push(format!("interface:oil_bin_{i}_unclosed"));
            }
        }
    }
    reasons
}

/// Bounded head/tail fixed-point iteration. The exchange count and the residual history are
/// measured. A converged interface must also close against the tail's own fluxes; otherwise it is
/// `UnclosedInterface`. Non-finite values and non-convergence are `NumericalFailure`. Physical
/// infeasibility is reported separately.
pub fn couple<H: HeadMap, T: TailMap>(
    head: &H,
    tail: &T,
    initial: Interface,
    cfg: &CouplingConfig,
) -> CouplingOutcome {
    let config_ok = cfg.max_exchanges >= 4
        && cfg.relaxation > 0.0
        && cfg.relaxation <= 1.0
        && cfg.residual_tol > 0.0
        && cfg.closure_tol > 0.0;
    if !config_ok {
        return CouplingOutcome::NumericalFailure {
            exchanges: 0,
            residuals: Vec::new(),
            reason: "config:invalid".to_string(),
        };
    }
    let mut sent = initial;
    let mut residuals = Vec::new();
    for exchange in 1..=cfg.max_exchanges {
        let tail_out = tail.respond(&sent);
        if !tail_response_finite(&tail_out) {
            return CouplingOutcome::NumericalFailure {
                exchanges: exchange,
                residuals,
                reason: "tail:nonfinite_response".to_string(),
            };
        }
        if !tail_out.infeasible.is_empty() {
            return CouplingOutcome::PhysicalInfeasible {
                exchanges: exchange,
                reasons: tail_out.infeasible,
            };
        }
        let proposal = head.respond(&tail_out.returned);
        if !interface_finite(&proposal) || proposal.oil_kg_s.len() != sent.oil_kg_s.len() {
            return CouplingOutcome::NumericalFailure {
                exchanges: exchange,
                residuals,
                reason: "head:nonfinite_or_bin_count_mismatch".to_string(),
            };
        }
        let residual = interface_residual(&sent, &proposal);
        residuals.push(residual);
        if exchange >= 4 && residual <= cfg.residual_tol {
            let reasons = closure_reasons(&sent, &tail_out, cfg.closure_tol);
            if !reasons.is_empty() {
                return CouplingOutcome::UnclosedInterface {
                    exchanges: exchange,
                    reasons,
                };
            }
            return CouplingOutcome::Converged {
                exchanges: exchange,
                residuals,
                observables: tail_out.observables,
                interface: sent,
            };
        }
        sent = conservative_relax(&sent, &proposal, cfg.relaxation);
    }
    CouplingOutcome::NumericalFailure {
        exchanges: cfg.max_exchanges,
        residuals,
        reason: "iteration:not_converged_within_cap".to_string(),
    }
}

/// Compare coupled observables with the held-out full-liner reference. Pressure agreement is one
/// observable among three; this function does not judge oil grade on its own.
pub fn compare_to_reference(
    coupled: &Observables,
    reference: &Observables,
    budget: &Field<DecisionBudget>,
) -> Verdict {
    let Some(b) = budget.admitted() else {
        return open("decision_budget:not_admitted");
    };
    let mut fails = Vec::new();
    for (name, got, want, tol) in [
        (
            "reject_split",
            coupled.reject_split,
            reference.reject_split,
            b.reject_split_rel,
        ),
        (
            "treated_oil",
            coupled.treated_oil_mg_l,
            reference.treated_oil_mg_l,
            b.treated_oil_rel,
        ),
        (
            "pressure_drop",
            coupled.pressure_drop_pa,
            reference.pressure_drop_pa,
            b.pressure_rel,
        ),
    ] {
        let r = rel_diff(got, want);
        if exceeds(r, tol) {
            fails.push(format!("observable:{name}_rel_{r:.4}_exceeds_{tol:.4}"));
        }
    }
    if fails.is_empty() {
        Verdict::Pass
    } else {
        Verdict::Fail(fails)
    }
}

/// Admitted decision budget for the coupled-to-reference comparison.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionBudget {
    pub reject_split_rel: f64,
    pub treated_oil_rel: f64,
    pub pressure_rel: f64,
}

/// Small reference fixture used to exercise the head/tail exchange contract with an explicit
/// per-bin oil partition and reverse-core feedback. It is not a fitted physical model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReferenceHeadTailCoupler {
    pub oil_partition: Vec<f64>,
    pub reverse_core_target_kg_s: f64,
    pub feedback_fraction: f64,
    pub observables: Observables,
    pub omit_reverse_core_in_sent: bool,
    pub oil_closure_scale: f64,
}

impl HeadMap for ReferenceHeadTailCoupler {
    fn respond(&self, returned: &Interface) -> Interface {
        let total: f64 = returned.oil_kg_s.iter().sum();
        let mut oil: Vec<f64> = self
            .oil_partition
            .iter()
            .enumerate()
            .map(|(i, fraction)| {
                let target = total * fraction;
                target
                    + self.feedback_fraction
                        * (returned.oil_kg_s.get(i).copied().unwrap_or(0.0) - target)
            })
            .map(|v| v.max(0.0))
            .collect();
        let sum: f64 = oil.iter().sum();
        if sum > 0.0 {
            for value in &mut oil {
                *value *= total / sum;
            }
        }
        Interface {
            oil_kg_s: oil,
            reverse_core_kg_s: if self.omit_reverse_core_in_sent {
                0.0
            } else {
                self.reverse_core_target_kg_s
                    + self.feedback_fraction
                        * (returned.reverse_core_kg_s - self.reverse_core_target_kg_s)
            },
        }
    }
}

impl TailMap for ReferenceHeadTailCoupler {
    fn respond(&self, sent: &Interface) -> TailResponse {
        TailResponse {
            returned: sent.clone(),
            integrated_oil_kg_s: sent
                .oil_kg_s
                .iter()
                .map(|v| v * self.oil_closure_scale)
                .collect(),
            integrated_reverse_kg_s: if self.omit_reverse_core_in_sent {
                self.reverse_core_target_kg_s
            } else {
                sent.reverse_core_kg_s
            },
            observables: self.observables.clone(),
            infeasible: Vec::new(),
        }
    }
}

/// Run the bounded exchange, preserve numerical versus physical outcomes, and compare only a
/// closed solution against the held-out full-liner observables using the admitted error budget.
pub fn evaluate_coupled_head_tail_against_reference(
    coupler: &ReferenceHeadTailCoupler,
    initial: Interface,
    cfg: &CouplingConfig,
    reference: &Observables,
    budget: &Field<DecisionBudget>,
) -> Result<(CouplingOutcome, Verdict), JudgeError> {
    if coupler.oil_partition.is_empty()
        || coupler
            .oil_partition
            .iter()
            .any(|x| !x.is_finite() || *x < 0.0)
        || (coupler.oil_partition.iter().sum::<f64>() - 1.0).abs() > 1e-10
        || !coupler.reverse_core_target_kg_s.is_finite()
        || !coupler.feedback_fraction.is_finite()
        || !(0.0..1.0).contains(&coupler.feedback_fraction)
        || !coupler.oil_closure_scale.is_finite()
    {
        return Err(invalid("coupling:reference_coupler_invalid"));
    }
    let outcome = couple(coupler, coupler, initial, cfg);
    let verdict = match &outcome {
        CouplingOutcome::Converged {
            exchanges,
            observables,
            ..
        } if *exchanges >= 4 => compare_to_reference(observables, reference, budget),
        _ => Verdict::Unresolved(outcome.fallback_reasons()),
    };
    Ok((outcome, verdict))
}

// ------------------------------------------------- V5-25-01/05 operator, envelope and features

/// Feature names used by the applicability envelope, in order.
pub const FEATURE_NAMES: [&str; 3] = ["reverse_fraction", "swirl_ratio", "turbulence_ratio"];

/// Features of a cut-plane state: reverse-core node fraction, mean |u_theta|/|u_a| swirl, and
/// peak turbulent kinetic energy normalised by mean axial kinetic energy.
pub fn features(state: &LinerCutPlaneState) -> [f64; 3] {
    let n = state.nodes.len().max(1) as f64;
    let reverse_fraction = state.nodes.iter().filter(|x| x.reverse_core).count() as f64 / n;
    let swirl_ratio = state
        .nodes
        .iter()
        .map(|x| x.tangential_ms.abs() / x.axial_ms.abs().max(1e-9))
        .sum::<f64>()
        / n;
    let mean_axial_sq = state
        .nodes
        .iter()
        .map(|x| x.axial_ms * x.axial_ms)
        .sum::<f64>()
        / n;
    let peak_k = state
        .nodes
        .iter()
        .map(|x| x.turbulence_k_m2_s2)
        .fold(0.0, f64::max);
    [
        reverse_fraction,
        swirl_ratio,
        peak_k / mean_axial_sq.max(1e-12),
    ]
}

/// Admitted box in feature space where the operator was trained. A box is only an abstention
/// heuristic: a point inside it is not proven to be in the operator's domain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub lower: [f64; 3],
    pub upper: [f64; 3],
}

impl Envelope {
    /// Normalised distance outside the box; zero means inside. Non-finite features are infinitely far.
    pub fn hull_distance(&self, f: &[f64; 3]) -> f64 {
        (0..3)
            .map(|i| {
                let (lo, hi, x) = (self.lower[i], self.upper[i], f[i]);
                if !x.is_finite() {
                    return f64::INFINITY;
                }
                let span = (hi - lo).max(1e-12);
                (lo - x).max(x - hi).max(0.0) / span
            })
            .fold(0.0, f64::max)
    }
}

/// Who produced an operator result. Admission and benchmark rows are bound to this identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperatorIdentity {
    pub operator_id: String,
    pub model_sha256: Sha256Hex,
    pub observer_id: String,
}

/// A frozen liner response: the tail response is fixed at one admitted cut-plane state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenLinerOperator {
    pub identity: OperatorIdentity,
    pub frozen: LinerCutPlaneState,
    pub frozen_observables: Observables,
    pub envelope: Envelope,
}

/// Evidence from assessing one candidate with the operator.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperatorEvidence {
    pub identity_sha256: Sha256Hex,
    pub candidate_sha256: Sha256Hex,
    pub features: [f64; 3],
    pub hull_distance: f64,
    pub relative_uncertainty: f64,
    pub coupling: CouplingOutcome,
}

impl FrozenLinerOperator {
    pub fn new(
        identity: OperatorIdentity,
        frozen: LinerCutPlaneState,
        frozen_observables: Observables,
        envelope: Envelope,
    ) -> Result<Self, JudgeError> {
        let bad = validate_state(&frozen);
        if !bad.is_empty() {
            return Err(invalid(bad.join(",")));
        }
        let ordered = (0..3).all(|i| envelope.lower[i] < envelope.upper[i]);
        if !(finite(&envelope.lower) && finite(&envelope.upper) && ordered) {
            return Err(invalid("envelope:bounds_not_finite_ordered"));
        }
        let o = &frozen_observables;
        if !finite(&[o.reject_split, o.treated_oil_mg_l, o.pressure_drop_pa]) {
            return Err(invalid("frozen_observables:not_finite"));
        }
        Ok(Self {
            identity,
            frozen,
            frozen_observables,
            envelope,
        })
    }

    /// Assess a candidate state: its envelope distance, its uncertainty and the coupled solve
    /// against this frozen tail. An invalid candidate is never routed to the operator.
    pub fn assess<H: HeadMap>(
        &self,
        head: &H,
        initial: Interface,
        cfg: &CouplingConfig,
        candidate: &LinerCutPlaneState,
    ) -> Result<OperatorEvidence, JudgeError> {
        let candidate_open = validate_state(candidate);
        let (feats, hull, coupling) = if candidate_open.is_empty() {
            let f = features(candidate);
            (
                f,
                self.envelope.hull_distance(&f),
                couple(head, self, initial, cfg),
            )
        } else {
            (
                [0.0; 3],
                f64::INFINITY,
                CouplingOutcome::UnclosedInterface {
                    exchanges: 0,
                    reasons: candidate_open,
                },
            )
        };
        Ok(OperatorEvidence {
            identity_sha256: canonical_hash(&self.identity)?,
            candidate_sha256: canonical_hash(candidate)?,
            features: feats,
            hull_distance: hull,
            relative_uncertainty: candidate.relative_uncertainty,
            coupling,
        })
    }
}

impl TailMap for FrozenLinerOperator {
    fn respond(&self, _sent: &Interface) -> TailResponse {
        // The response is frozen: the plane carries its own integrated flux whatever the head
        // sends. A mismatch with the head's interface is reported as unclosed, never absorbed.
        let got = integrate(&self.frozen);
        TailResponse {
            returned: Interface {
                oil_kg_s: got.oil_kg_s.clone(),
                reverse_core_kg_s: got.reverse_core_kg_s,
            },
            integrated_oil_kg_s: got.oil_kg_s,
            integrated_reverse_kg_s: got.reverse_core_kg_s,
            observables: self.frozen_observables.clone(),
            infeasible: Vec::new(),
        }
    }
}

// ---------------------------------------------------------- V5-25-04 ranking and random audits

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stratum {
    /// Promoted by the operator.
    Promoted,
    /// In-domain and ranked below the promotion cut.
    LowRanked,
    /// Outside the envelope: the operator abstains.
    OutOfDomain,
}

impl Stratum {
    pub fn name(self) -> &'static str {
        match self {
            Self::Promoted => "promoted",
            Self::LowRanked => "low_ranked",
            Self::OutOfDomain => "out_of_domain",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateRecord {
    pub candidate_id: String,
    pub stratum: Stratum,
    pub operator: Observables,
    pub reference: Observables,
    /// Whether the full-liner reference clears the downstream gate for this candidate.
    pub reference_clears_gate: bool,
}

/// Fixed acceptance criteria, admitted before any audit is drawn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditCriteria {
    pub max_false_omission_upper95: f64,
    pub max_pressure_rel_error: f64,
    pub max_oil_grade_rel_error: f64,
    pub min_audited_per_stratum: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditBenchmark {
    pub operator_identity_sha256: Sha256Hex,
    pub seed: u64,
    pub audited: usize,
    pub audited_per_stratum: BTreeMap<String, usize>,
    /// Operator-rejected audited candidates (low-ranked, in domain).
    pub rejected: usize,
    /// Rejected candidates that the full-liner reference would have cleared.
    pub false_omissions: usize,
    pub false_omission_rate: f64,
    /// Wilson 95 % upper bound on the false-omission rate.
    pub false_omission_upper95: f64,
    pub abstained: usize,
    pub abstention_rate: f64,
    pub max_pressure_rel_error: f64,
    pub max_oil_grade_rel_error: f64,
    pub verdict: Verdict,
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn uniform(state: &mut u64) -> f64 {
    (splitmix64(state) >> 11) as f64 / (1u64 << 53) as f64
}

/// Seeded Bernoulli audit draw. Every candidate consumes one draw in population order, so the
/// same seed and population always audit the same indices.
pub fn draw_audits(
    population: &[CandidateRecord],
    rates: &BTreeMap<Stratum, f64>,
    seed: u64,
) -> Vec<usize> {
    let mut state = seed;
    population
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let u = uniform(&mut state);
            let rate = rates.get(&c.stratum).copied().unwrap_or(0.0);
            (u < rate).then_some(i)
        })
        .collect()
}

/// Upper bound of the Wilson score interval for `k` successes in `n` trials. With no trials the
/// bound is 1, so an empty audit can never satisfy a criterion.
pub fn wilson_upper(k: usize, n: usize, z: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let (k, n) = (k as f64, n as f64);
    let p = k / n;
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let centre = p + z2 / (2.0 * n);
    let margin = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    ((centre + margin) / denom).min(1.0)
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 {
        0.0
    } else {
        a as f64 / b as f64
    }
}

fn observables_finite(o: &Observables) -> bool {
    finite(&[o.reject_split, o.treated_oil_mg_l, o.pressure_drop_pa])
}

fn judge_benchmark(
    criteria: &Field<AuditCriteria>,
    per_stratum: &BTreeMap<String, usize>,
    rejected: usize,
    upper: f64,
    pressure: f64,
    oil: f64,
    mut open_items: Vec<String>,
) -> Verdict {
    let Some(c) = criteria.admitted() else {
        open_items.push("audit:criteria_not_admitted".to_string());
        return Verdict::Unresolved(open_items);
    };
    let mut fails = Vec::new();
    for s in [Stratum::Promoted, Stratum::LowRanked, Stratum::OutOfDomain] {
        if per_stratum.get(s.name()).copied().unwrap_or(0) < c.min_audited_per_stratum {
            open_items.push(format!("audit:stratum_{}_under_sampled", s.name()));
        }
    }
    if rejected == 0 {
        open_items.push("audit:no_rejected_candidates_audited".to_string());
    } else if upper > c.max_false_omission_upper95 {
        fails.push(format!(
            "audit:false_omission_upper95_{upper:.4}_exceeds_{:.4}",
            c.max_false_omission_upper95
        ));
    }
    if pressure > c.max_pressure_rel_error {
        fails.push(format!(
            "audit:pressure_rel_error_{pressure:.4}_exceeds_{:.4}",
            c.max_pressure_rel_error
        ));
    }
    if oil > c.max_oil_grade_rel_error {
        fails.push(format!(
            "audit:oil_grade_rel_error_{oil:.4}_exceeds_{:.4}",
            c.max_oil_grade_rel_error
        ));
    }
    if !fails.is_empty() {
        Verdict::Fail(fails)
    } else if !open_items.is_empty() {
        Verdict::Unresolved(open_items)
    } else {
        Verdict::Pass
    }
}

/// Draw the random audit, then measure omission, abstention and prediction error on it. The
/// operator's own stratum decides what it did: promoted, rejected (low-ranked) or abstained (OOD).
pub fn run_audit(
    population: &[CandidateRecord],
    rates: &BTreeMap<Stratum, f64>,
    seed: u64,
    criteria: &Field<AuditCriteria>,
    identity: &OperatorIdentity,
) -> Result<AuditBenchmark, JudgeError> {
    let identity_sha256 = canonical_hash(identity)?;
    let audited = draw_audits(population, rates, seed);
    let mut per_stratum: BTreeMap<String, usize> = BTreeMap::new();
    let (mut rejected, mut false_omissions, mut abstained) = (0usize, 0usize, 0usize);
    let (mut max_pressure, mut max_oil) = (0.0_f64, 0.0_f64);
    let mut open_items = Vec::new();
    for &i in &audited {
        let c = &population[i];
        *per_stratum.entry(c.stratum.name().to_string()).or_insert(0) += 1;
        if !observables_finite(&c.operator) || !observables_finite(&c.reference) {
            open_items.push(format!("audit:nonfinite_observable:{}", c.candidate_id));
            continue;
        }
        match c.stratum {
            Stratum::Promoted => {}
            Stratum::LowRanked => {
                rejected += 1;
                if c.reference_clears_gate {
                    false_omissions += 1;
                }
            }
            Stratum::OutOfDomain => abstained += 1,
        }
        if c.stratum != Stratum::OutOfDomain {
            max_pressure = max_pressure.max(rel_diff(
                c.operator.pressure_drop_pa,
                c.reference.pressure_drop_pa,
            ));
            max_oil = max_oil.max(rel_diff(
                c.operator.treated_oil_mg_l,
                c.reference.treated_oil_mg_l,
            ));
        }
    }
    let upper = wilson_upper(false_omissions, rejected, Z95);
    let verdict = judge_benchmark(
        criteria,
        &per_stratum,
        rejected,
        upper,
        max_pressure,
        max_oil,
        open_items,
    );
    Ok(AuditBenchmark {
        operator_identity_sha256: identity_sha256,
        seed,
        audited: audited.len(),
        audited_per_stratum: per_stratum,
        rejected,
        false_omissions,
        false_omission_rate: ratio(false_omissions, rejected),
        false_omission_upper95: upper,
        abstained,
        abstention_rate: ratio(abstained, audited.len()),
        max_pressure_rel_error: max_pressure,
        max_oil_grade_rel_error: max_oil,
        verdict,
    })
}

/// Admission is bound to the operator identity that produced the benchmark. A different operator,
/// model or observer cannot inherit it.
pub fn admit(bench: &AuditBenchmark, identity: &OperatorIdentity) -> Result<Verdict, JudgeError> {
    if canonical_hash(identity)? != bench.operator_identity_sha256 {
        return Ok(open("admission:operator_identity_mismatch"));
    }
    Ok(bench.verdict.clone())
}

/// Evidence available for a downstream oil-grade claim.
#[derive(Clone, Debug, PartialEq)]
pub enum GradeEvidence {
    PressureOnly {
        pressure_rel_error: f64,
    },
    OilGrade {
        pressure_rel_error: f64,
        oil_grade_rel_error: f64,
    },
}

/// Scalar pressure agreement never qualifies a downstream oil grade. The oil-grade error must be
/// measured and within the admitted criteria.
pub fn judge_downstream_grade(
    evidence: &GradeEvidence,
    criteria: &Field<AuditCriteria>,
) -> Verdict {
    let GradeEvidence::OilGrade {
        pressure_rel_error,
        oil_grade_rel_error,
    } = evidence
    else {
        return open("downstream:oil_grade_not_compared_pressure_agreement_cannot_qualify");
    };
    let Some(c) = criteria.admitted() else {
        return open("audit:criteria_not_admitted");
    };
    let mut fails = Vec::new();
    if exceeds(*pressure_rel_error, c.max_pressure_rel_error) {
        fails.push("downstream:pressure_rel_error_exceeds_criteria".to_string());
    }
    if exceeds(*oil_grade_rel_error, c.max_oil_grade_rel_error) {
        fails.push("downstream:oil_grade_rel_error_exceeds_criteria".to_string());
    }
    if fails.is_empty() {
        Verdict::Pass
    } else {
        Verdict::Fail(fails)
    }
}

// ----------------------------------------------------- V5-25-05 automatic fallback and budget

/// Where a candidate's result comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    Operator { observables: Observables },
    FullLiner { reasons: Vec<String> },
}

/// Route one candidate. Out-of-envelope, unclosed, non-converged or high-uncertainty states go to
/// the full-liner reference. Only a converged, in-envelope, low-uncertainty state uses the operator.
pub fn route(evidence: Option<&OperatorEvidence>, max_relative_uncertainty: f64) -> Route {
    let Some(e) = evidence else {
        return Route::FullLiner {
            reasons: vec![OPERATOR_ABSENT.to_string()],
        };
    };
    let mut reasons = Vec::new();
    if e.hull_distance != 0.0 {
        reasons.push("ood:outside_envelope".to_string());
    }
    reasons.extend(e.coupling.fallback_reasons());
    if exceeds(e.relative_uncertainty, max_relative_uncertainty) {
        reasons.push("uncertainty:above_limit".to_string());
    }
    match &e.coupling {
        CouplingOutcome::Converged { observables, .. } if reasons.is_empty() => Route::Operator {
            observables: observables.clone(),
        },
        _ => Route::FullLiner { reasons },
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvaluationBudget {
    pub units_remaining: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampaignCase {
    pub candidate_id: String,
    /// Every source the result depends on: geometry, state, and operator identity digests.
    pub sources: Vec<Sha256Hex>,
    pub route: Route,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Disposition {
    Operator {
        observables: Observables,
    },
    FullLiner {
        reasons: Vec<String>,
    },
    /// The operator abstained and the budget cannot pay for the reference. Never a pass.
    Abstain {
        reasons: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct DispatchRecord {
    pub candidate_id: String,
    pub disposition: Disposition,
    pub sources: Vec<Sha256Hex>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampaignPlan {
    pub records: Vec<DispatchRecord>,
    /// Full-liner units spent on candidates the operator was not present for.
    pub base_units_spent: f64,
    /// Full-liner units spent because the operator abstained or failed; extra to a campaign without it.
    pub fallback_units_spent: f64,
    pub budget_remaining: f64,
    pub verdict: Verdict,
}

/// Dispatch a campaign against a budget. Each full-liner evaluation costs `full_cost_units`.
/// When the budget cannot pay for a fallback the candidate is `Abstain`, so the campaign is
/// `Unresolved` and the candidate is not assumed to pass. An absent operator routes everything
/// to the reference and does not block the campaign.
pub fn plan_campaign(
    cases: &[CampaignCase],
    full_cost_units: f64,
    budget: &mut EvaluationBudget,
) -> Result<CampaignPlan, JudgeError> {
    if !(full_cost_units.is_finite() && full_cost_units > 0.0) {
        return Err(invalid("campaign:full_cost_not_finite_positive"));
    }
    if !(budget.units_remaining.is_finite() && budget.units_remaining >= 0.0) {
        return Err(invalid("campaign:budget_not_finite_nonnegative"));
    }
    let (mut base, mut fallback) = (0.0, 0.0);
    let mut records = Vec::new();
    let mut open_items = Vec::new();
    for case in cases {
        let disposition = match &case.route {
            Route::Operator { observables } => Disposition::Operator {
                observables: observables.clone(),
            },
            Route::FullLiner { reasons } => {
                if budget.units_remaining >= full_cost_units {
                    budget.units_remaining -= full_cost_units;
                    if reasons.iter().any(|r| r.as_str() == OPERATOR_ABSENT) {
                        base += full_cost_units;
                    } else {
                        fallback += full_cost_units;
                    }
                    Disposition::FullLiner {
                        reasons: reasons.clone(),
                    }
                } else {
                    open_items.push(format!(
                        "budget:depleted_cannot_fallback:{}",
                        case.candidate_id
                    ));
                    let mut abstain = reasons.clone();
                    abstain.push("budget:depleted".to_string());
                    Disposition::Abstain { reasons: abstain }
                }
            }
        };
        records.push(DispatchRecord {
            candidate_id: case.candidate_id.clone(),
            disposition,
            sources: case.sources.clone(),
        });
    }
    let verdict = if open_items.is_empty() {
        Verdict::Pass
    } else {
        Verdict::Unresolved(open_items)
    };
    Ok(CampaignPlan {
        records,
        base_units_spent: base,
        fallback_units_spent: fallback,
        budget_remaining: budget.units_remaining,
        verdict,
    })
}

#[cfg(test)]
mod v5_25_remediation_tests {
    use super::*;

    #[test]
    fn v5_25_02_full_liner_corpus_family_separation_and_head_reuse_guard() {
        let raw = include_str!("../../../../experiments/datasets/v5-25-02-full-liner-corpus.json");
        let report = verify_full_liner_corpus_json(raw).unwrap();
        assert_eq!(report.held_out_cases, 2);
        assert_eq!(report.synthetic_cases, 12);
        assert!(report.held_out_tags.contains(&"oil_grade".to_string()));
        assert_eq!(report.declared_gaps, vec!["wet_liner_surface_chemistry"]);
        let lease = Field::Admitted {
            value: MeasuredLease {
                cpu_cores: 2,
                memory_gib: 6.0,
                output_gib: 1.0,
                gpu: false,
                lease_sha256: Sha256Hex::of_bytes(b"synthetic measured lease"),
            },
            source: Sha256Hex::of_bytes(b"synthetic lease source"),
        };
        let (_, built_report) = build_preregistered_full_liner_corpus(
            lease,
            vec![DeclaredGap {
                tag: "wet_liner_surface_chemistry".into(),
                reason: "physical wet-liner measurements are governed by V5-30".into(),
            }],
        )
        .unwrap();
        assert_eq!(
            built_report.declared_gaps,
            vec!["wet_liner_surface_chemistry"]
        );
        let mut pre: Preregistration = serde_json::from_str(raw).unwrap();
        let held = pre
            .cases
            .iter()
            .position(|c| c.split == Split::HeldOut)
            .unwrap();
        pre.cases[held].geometry_family = pre.cases[0].geometry_family.clone();
        assert!(verify_full_liner_corpus_json(&serde_json::to_string(&pre).unwrap()).is_err());
        pre.cases[held].geometry_family = "helical_liner_d".into();
        pre.cases[held].geometry_sha256 = pre.cases[0].geometry_sha256.clone();
        assert!(verify_full_liner_corpus_json(&serde_json::to_string(&pre).unwrap()).is_err());
    }

    #[test]
    fn v5_25_03_coupled_head_tail_reference_agreement_and_unclosed_fallback() {
        let reference = Observables {
            reject_split: 0.08,
            treated_oil_mg_l: 24.0,
            pressure_drop_pa: 42_000.0,
        };
        let coupler = ReferenceHeadTailCoupler {
            oil_partition: vec![0.65, 0.35],
            reverse_core_target_kg_s: 0.1,
            feedback_fraction: 0.3,
            observables: reference.clone(),
            omit_reverse_core_in_sent: false,
            oil_closure_scale: 1.0,
        };
        let cfg = CouplingConfig {
            max_exchanges: 20,
            relaxation: 1.0,
            residual_tol: 0.01,
            closure_tol: 1e-9,
        };
        let budget = Field::Admitted {
            value: DecisionBudget {
                reject_split_rel: 0.01,
                treated_oil_rel: 0.01,
                pressure_rel: 0.01,
            },
            source: Sha256Hex::of_bytes(b"synthetic decision budget"),
        };
        let (outcome, verdict) = evaluate_coupled_head_tail_against_reference(
            &coupler,
            Interface {
                oil_kg_s: vec![0.8, 0.2],
                reverse_core_kg_s: 0.2,
            },
            &cfg,
            &reference,
            &budget,
        )
        .unwrap();
        assert!(
            matches!(&outcome, CouplingOutcome::Converged { exchanges, .. } if *exchanges >= 4)
        );
        assert_eq!(verdict, Verdict::Pass);
        let mut omitted = coupler.clone();
        omitted.omit_reverse_core_in_sent = true;
        let (fallback, verdict) = evaluate_coupled_head_tail_against_reference(
            &omitted,
            Interface {
                oil_kg_s: vec![0.8, 0.2],
                reverse_core_kg_s: 0.2,
            },
            &cfg,
            &reference,
            &budget,
        )
        .unwrap();
        assert!(matches!(
            fallback,
            CouplingOutcome::UnclosedInterface { .. }
        ));
        assert!(matches!(verdict, Verdict::Unresolved(_)));
        let mut unclosed_oil = coupler.clone();
        unclosed_oil.oil_closure_scale = 0.9;
        let (fallback, _) = evaluate_coupled_head_tail_against_reference(
            &unclosed_oil,
            Interface {
                oil_kg_s: vec![0.8, 0.2],
                reverse_core_kg_s: 0.2,
            },
            &cfg,
            &reference,
            &budget,
        )
        .unwrap();
        assert!(matches!(
            fallback,
            CouplingOutcome::UnclosedInterface { .. }
        ));
        let too_short = CouplingConfig {
            max_exchanges: 3,
            ..cfg
        };
        assert!(matches!(
            couple(
                &omitted,
                &omitted,
                Interface {
                    oil_kg_s: vec![0.8, 0.2],
                    reverse_core_kg_s: 0.2
                },
                &too_short
            ),
            CouplingOutcome::NumericalFailure { .. }
        ));
    }
}
