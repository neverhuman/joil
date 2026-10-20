//! Reference head/tail coupler fixture, applicability envelope, and frozen liner operator assessment.

use super::*;

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
