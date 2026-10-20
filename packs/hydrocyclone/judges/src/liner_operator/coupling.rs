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

#[path = "coupling/operator.rs"]
mod operator;
pub use operator::*;
