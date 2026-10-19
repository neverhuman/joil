use super::*;
use crate::{canonical_hash, invalid, Field, JudgeError, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};

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
