use super::*;
use crate::{canonical_hash, invalid, Field, JudgeError, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
