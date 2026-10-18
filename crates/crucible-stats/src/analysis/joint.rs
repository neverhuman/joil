//! V5-04-03: joint decision bounds for a validated DecisionProfile.
//!
//! The reviewed profile, requirement set and registry are validated first
//! (`DecisionProfile::qualified_admission`); unresolved inputs abstain and invalid
//! ones block. Each family member gets a one-sided lower bound on its
//! positive-is-passing margin, L = mean - t(1-alpha_j, n-1) sd / sqrt(n) - b_j, where
//! b_j is the admitted adverse numerical bound plus the admitted model-discrepancy
//! bound for that estimand (stipulated separately, never estimated from the
//! confirmation data). Bonferroni simultaneous coverage and a fixed-candidate
//! intersection-union conjunction are reported as different claims. Paired blocks
//! keep their cross-margin correlations in the result; Bonferroni does not need
//! independence across margins. Normal margins with unknown variance are assumed.
use super::sample_moments;
use crate::distribution::student_t_quantile;
use crate::StatsError;
use joil_evidence::claims::Registry;
use joil_evidence::decision::{
    DecisionError, DecisionProfile, DecisionUnit, ErrorCriterion, FamilyMember, Procedure,
    RequirementSet, RequirementValue, Sidedness,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarginSummary {
    pub member: FamilyMember,
    pub unit: DecisionUnit,
    pub n: usize,
    pub mean: f64,
    pub sd: f64,
    /// Required for ratio margins such as `Q_candidate - 1.10 Q_baseline`.
    pub ratio_baseline_mean: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairedMargin {
    pub member: FamilyMember,
    pub unit: DecisionUnit,
    /// One margin value per independent block, in block order.
    pub values: Vec<f64>,
    /// Baseline values per block for ratio margins.
    pub ratio_baseline: Option<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarginData {
    /// Arithmetic-only summaries; pairing cannot be checked.
    Summaries(Vec<MarginSummary>),
    /// Independent paired blocks; every margin is observed on every block.
    PairedBlocks {
        block_ids: Vec<String>,
        margins: Vec<PairedMargin>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JointScope {
    SimultaneousLowerBounds,
    FixedCandidateConjunction,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemberBound {
    pub member: FamilyMember,
    pub unit: DecisionUnit,
    pub n: usize,
    pub mean: f64,
    pub sd: f64,
    pub alpha: f64,
    pub t_critical: f64,
    pub adverse_systematic_bound: f64,
    pub lower_bound: f64,
    pub passes: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum JointOutcome {
    Pass,
    Fail { failed: Vec<FamilyMember> },
    Abstain { reasons: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JointDecision {
    pub scope: Option<JointScope>,
    pub outcome: JointOutcome,
    pub bounds: Vec<MemberBound>,
    /// Pearson correlations between margins over paired blocks (row/column = bounds order).
    pub correlations: Option<Vec<Vec<f64>>>,
    pub interpretation: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum JointError {
    /// The reviewed profile, requirements or registry are invalid.
    Profile(String),
    Input(&'static str),
    Unsupported(&'static str),
    Stats(StatsError),
}

impl From<StatsError> for JointError {
    fn from(e: StatsError) -> Self {
        Self::Stats(e)
    }
}

fn admitted_upper(value: &RequirementValue<joil_evidence::decision::BoundedValue>) -> Option<f64> {
    match value {
        RequirementValue::Admitted { value, .. } => Some(value.upper.get()),
        _ => None,
    }
}

struct Summary {
    member: FamilyMember,
    unit: DecisionUnit,
    n: usize,
    mean: f64,
    sd: f64,
    ratio_baseline_mean: Option<f64>,
}

fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let cov: f64 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let va: f64 = a.iter().map(|x| (x - ma).powi(2)).sum();
    let vb: f64 = b.iter().map(|y| (y - mb).powi(2)).sum();
    cov / (va * vb).sqrt()
}

type CorrelationMatrix = Vec<Vec<f64>>;

fn summaries(data: &MarginData) -> Result<(Vec<Summary>, Option<CorrelationMatrix>), JointError> {
    match data {
        MarginData::Summaries(rows) => Ok((
            rows.iter()
                .map(|r| Summary {
                    member: r.member.clone(),
                    unit: r.unit,
                    n: r.n,
                    mean: r.mean,
                    sd: r.sd,
                    ratio_baseline_mean: r.ratio_baseline_mean,
                })
                .collect(),
            None,
        )),
        MarginData::PairedBlocks { block_ids, margins } => {
            let mut seen = BTreeSet::new();
            for id in block_ids {
                if joil_evidence::identity::check_id(id).is_err() || !seen.insert(id.as_str()) {
                    return Err(JointError::Input("invalid or duplicate block identity"));
                }
            }
            let mut out = Vec::new();
            for m in margins {
                if m.values.len() != block_ids.len()
                    || m.ratio_baseline
                        .as_ref()
                        .is_some_and(|b| b.len() != block_ids.len())
                {
                    return Err(JointError::Input(
                        "every margin must be observed on every paired block",
                    ));
                }
                let (mean, variance) = sample_moments(&m.values)?;
                let ratio_baseline_mean = match &m.ratio_baseline {
                    Some(b) => Some(sample_moments(b)?.0),
                    None => None,
                };
                out.push(Summary {
                    member: m.member.clone(),
                    unit: m.unit,
                    n: m.values.len(),
                    mean,
                    sd: variance.sqrt(),
                    ratio_baseline_mean,
                });
            }
            let correlations = margins
                .iter()
                .map(|a| {
                    margins
                        .iter()
                        .map(|b| pearson(&a.values, &b.values))
                        .collect()
                })
                .collect();
            Ok((out, Some(correlations)))
        }
    }
}

/// Validate the reviewed decision inputs, then compute the family's lower bounds.
pub fn joint_lower_bounds(
    profile: &DecisionProfile,
    requirements: &RequirementSet,
    registry: &Registry,
    data: &MarginData,
) -> Result<JointDecision, JointError> {
    match profile.qualified_admission(requirements, registry) {
        Ok(()) => {}
        Err(DecisionError::Unresolved(reasons)) => {
            return Ok(JointDecision {
                scope: None,
                outcome: JointOutcome::Abstain { reasons },
                bounds: vec![],
                correlations: None,
                interpretation:
                    "unresolved requirement or discrepancy: abstain, never pass or fail".into(),
            })
        }
        Err(DecisionError::Invalid(message)) => return Err(JointError::Profile(message)),
    }
    let method = &profile.statistical_method;
    if method.sidedness != Sidedness::Lower {
        return Err(JointError::Unsupported(
            "this routine bounds positive-is-passing margins from below",
        ));
    }
    let scope = match (method.criterion, method.procedure) {
        (ErrorCriterion::SimultaneousCoverage, Procedure::Bonferroni) => {
            JointScope::SimultaneousLowerBounds
        }
        (ErrorCriterion::FixedCandidateConjunction, Procedure::IntersectionUnion) => {
            JointScope::FixedCandidateConjunction
        }
        (ErrorCriterion::AdaptiveWinnerQualification, _) => {
            return Err(JointError::Unsupported(
                "adaptive winner qualification requires experiment-family ledger charges",
            ))
        }
        _ => {
            return Err(JointError::Unsupported(
                "unsupported criterion and procedure pair",
            ))
        }
    };
    let (rows, correlations) = summaries(data)?;
    let declared: BTreeSet<&FamilyMember> = profile.gate_scenario_family.iter().collect();
    let supplied: BTreeSet<&FamilyMember> = rows.iter().map(|r| &r.member).collect();
    if declared != supplied || supplied.len() != rows.len() {
        return Err(JointError::Input(
            "margin data do not match the declared gate-scenario family",
        ));
    }
    let mut bounds = Vec::with_capacity(rows.len());
    for r in &rows {
        let estimand = profile
            .estimands
            .iter()
            .find(|e| e.id == r.member.estimand)
            .ok_or(JointError::Input("margin estimand not declared"))?;
        if estimand.unit != r.unit {
            return Err(JointError::Input(
                "margin unit differs from its estimand unit",
            ));
        }
        if r.n < 2 || !r.mean.is_finite() || !r.sd.is_finite() || r.sd <= 0.0 {
            return Err(JointError::Input(
                "each margin needs n >= 2, a finite mean and a positive finite sd",
            ));
        }
        if let Some(base) = r.ratio_baseline_mean {
            if !base.is_finite() || base <= 0.0 {
                return Err(JointError::Input(
                    "ratio margin needs a strictly positive baseline mean",
                ));
            }
        }
        let alpha = method
            .allocations
            .iter()
            .find(|a| a.member == r.member)
            .map(|a| a.alpha.get())
            .ok_or(JointError::Input("member has no alpha allocation"))?;
        let bias = |inventory: &[joil_evidence::decision::UncertaintyBound]| {
            inventory
                .iter()
                .find(|b| b.estimand == r.member.estimand)
                .and_then(|b| admitted_upper(&b.bound))
        };
        let adverse = match (
            bias(&profile.numerical_bounds),
            bias(&profile.model_discrepancy),
        ) {
            (Some(numerical), Some(model)) => numerical + model,
            _ => {
                return Err(JointError::Input(
                    "adverse systematic bound missing or not admitted",
                ))
            }
        };
        let t_critical = student_t_quantile(1.0 - alpha, r.n - 1)?;
        let lower = r.mean - t_critical * r.sd / (r.n as f64).sqrt() - adverse;
        if !lower.is_finite() {
            return Err(JointError::Stats(StatsError::NumericalFailure(
                "nonfinite lower bound",
            )));
        }
        bounds.push(MemberBound {
            member: r.member.clone(),
            unit: r.unit,
            n: r.n,
            mean: r.mean,
            sd: r.sd,
            alpha,
            t_critical,
            adverse_systematic_bound: adverse,
            lower_bound: lower,
            passes: lower > 0.0,
        });
    }
    let failed: Vec<FamilyMember> = bounds
        .iter()
        .filter(|b| !b.passes)
        .map(|b| b.member.clone())
        .collect();
    let outcome = if failed.is_empty() {
        JointOutcome::Pass
    } else {
        JointOutcome::Fail { failed }
    };
    let interpretation = match scope {
        JointScope::SimultaneousLowerBounds => format!(
            "Bonferroni: all displayed lower bounds hold simultaneously with probability >= {:.4} under normal margins and valid adverse bounds; mean requirements only, not reliability",
            1.0 - method.allocations.iter().map(|a| a.alpha.get()).sum::<f64>()
        ),
        JointScope::FixedCandidateConjunction => "intersection-union: false qualification of this fixed candidate's conjunction <= max member alpha; the displayed bounds are NOT simultaneous intervals".into(),
    };
    Ok(JointDecision {
        scope: Some(scope),
        outcome,
        bounds,
        correlations,
        interpretation,
    })
}
