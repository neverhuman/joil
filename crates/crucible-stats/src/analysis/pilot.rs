//! V5-04-01: prospective replication design for paired independent blocks.
//!
//! The sample plan is fixed from an independent pilot variance and a declared useful
//! effect, alpha and power before any confirmation block exists. The planner has no
//! confirmation input, so an observed confirmation effect can never change the plan.
//! Interpretation: one-sided t inference on the mean paired effect, assuming the
//! paired effects of independent preparations are IID normal. That assumption is
//! declared, not tested here; nothing in this module is physical qualification.
use super::sample_moments;
use crate::distribution::student_t_quantile;
use crate::StatsError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Largest plan produced here; larger designs need a separately reviewed method.
pub const MAX_PLANNED_BLOCKS: usize = 10_000;

/// Planning rule recorded in every plan. This is the central-t iteration, not an
/// exact noncentral-t power calculation; its adequacy is checked independently.
pub const PLANNING_METHOD: &str =
    "smallest n >= max(2, minimum_blocks) with sqrt(n)*useful_effect/pilot_sd >= t(1-alpha,n-1) + t(power,n-1)";

/// Declared before confirmation. The useful effect is a design input, never an estimate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProspectiveDesign {
    pub design_id: String,
    /// Independent pilot preparations; none may be reused as a confirmation block.
    pub pilot_block_ids: Vec<String>,
    /// Paired effects of the pilot blocks, in the confirmation estimand's units.
    pub pilot_paired_effects: Vec<f64>,
    /// The claim is `mean paired effect >= claim_threshold`.
    pub claim_threshold: f64,
    /// Effect above the threshold the design must detect; strictly positive.
    pub useful_effect: f64,
    pub alpha_one_sided: f64,
    pub power: f64,
    /// Separate protocol floor on independent confirmation blocks.
    pub minimum_blocks: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplePlan {
    pub design: ProspectiveDesign,
    pub design_identity: String,
    pub pilot_sd: f64,
    pub required_blocks: usize,
    pub method: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationBlock {
    pub block_id: String,
    pub independent_preparation: bool,
    pub paired_effect: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfirmationResult {
    pub design_identity: String,
    pub planned_blocks: usize,
    pub n_blocks: usize,
    pub mean_paired_effect: f64,
    pub sd_paired_effect: f64,
    pub t_critical: f64,
    pub lower_confidence_bound: f64,
    pub claim_threshold: f64,
    pub qualified: bool,
    pub interpretation: String,
}

fn check_ids(ids: &[String]) -> Result<BTreeSet<&str>, StatsError> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if joil_evidence::identity::check_id(id).is_err() || !seen.insert(id.as_str()) {
            return Err(StatsError::InvalidInput(
                "invalid or duplicate preparation identity",
            ));
        }
    }
    Ok(seen)
}

fn check_design(design: &ProspectiveDesign) -> Result<(), StatsError> {
    if joil_evidence::identity::check_id(&design.design_id).is_err() {
        return Err(StatsError::InvalidInput("invalid design identity"));
    }
    if design.pilot_block_ids.len() != design.pilot_paired_effects.len() {
        return Err(StatsError::InvalidInput(
            "pilot identities and effects differ in length",
        ));
    }
    if design.pilot_block_ids.len() < 2 {
        return Err(StatsError::InvalidInput(
            "insufficient independent pilot preparations",
        ));
    }
    check_ids(&design.pilot_block_ids)?;
    if !design.claim_threshold.is_finite() {
        return Err(StatsError::InvalidInput("nonfinite claim threshold"));
    }
    if !design.useful_effect.is_finite() || design.useful_effect <= 0.0 {
        return Err(StatsError::InvalidInput(
            "useful effect must be finite and positive",
        ));
    }
    if !(design.alpha_one_sided > 0.0 && design.alpha_one_sided < 0.5) {
        return Err(StatsError::InvalidInput(
            "one-sided alpha must be in (0, 0.5)",
        ));
    }
    if !(design.power > 0.5 && design.power < 1.0) {
        return Err(StatsError::InvalidInput("power must be in (0.5, 1)"));
    }
    if !(2..=MAX_PLANNED_BLOCKS).contains(&design.minimum_blocks) {
        return Err(StatsError::InvalidInput(
            "minimum blocks outside [2, MAX_PLANNED_BLOCKS]",
        ));
    }
    Ok(())
}

/// Planning core: a declared standard deviation and useful effect, never observed outcomes.
pub fn required_blocks(
    planning_sd: f64,
    useful_effect: f64,
    alpha_one_sided: f64,
    power: f64,
    minimum_blocks: usize,
) -> Result<usize, StatsError> {
    if !planning_sd.is_finite() || planning_sd <= 0.0 {
        return Err(StatsError::InvalidInput(
            "planning sd must be finite and positive",
        ));
    }
    if !useful_effect.is_finite() || useful_effect <= 0.0 {
        return Err(StatsError::InvalidInput(
            "useful effect must be finite and positive",
        ));
    }
    if !(alpha_one_sided > 0.0 && alpha_one_sided < 0.5) {
        return Err(StatsError::InvalidInput(
            "one-sided alpha must be in (0, 0.5)",
        ));
    }
    if !(power > 0.5 && power < 1.0) {
        return Err(StatsError::InvalidInput("power must be in (0.5, 1)"));
    }
    if !(2..=MAX_PLANNED_BLOCKS).contains(&minimum_blocks) {
        return Err(StatsError::InvalidInput(
            "minimum blocks outside [2, MAX_PLANNED_BLOCKS]",
        ));
    }
    let standardized = useful_effect / planning_sd;
    for n in minimum_blocks..=MAX_PLANNED_BLOCKS {
        let df = n - 1;
        let needed =
            student_t_quantile(1.0 - alpha_one_sided, df)? + student_t_quantile(power, df)?;
        if (n as f64).sqrt() * standardized >= needed {
            return Ok(n);
        }
    }
    Err(StatsError::Unsupported(
        "required blocks exceed MAX_PLANNED_BLOCKS",
    ))
}

/// Plan the confirmation block count from the declared design alone.
pub fn plan_replication(design: &ProspectiveDesign) -> Result<SamplePlan, StatsError> {
    check_design(design)?;
    let (_, variance) = sample_moments(&design.pilot_paired_effects)?;
    if variance == 0.0 {
        return Err(StatsError::Unsupported(
            "zero pilot variance cannot plan replication; no artificial variance floor",
        ));
    }
    let pilot_sd = variance.sqrt();
    let required_blocks = required_blocks(
        pilot_sd,
        design.useful_effect,
        design.alpha_one_sided,
        design.power,
        design.minimum_blocks,
    )?;
    let design_identity = joil_evidence::identity::scientific_hash(design)
        .map_err(|_| StatsError::InvalidInput("design is not canonically serializable"))?;
    Ok(SamplePlan {
        design: design.clone(),
        design_identity,
        pilot_sd,
        required_blocks,
        method: PLANNING_METHOD.into(),
    })
}

/// Analyse exactly the frozen planned confirmation blocks against the declared threshold.
pub fn confirm(
    plan: &SamplePlan,
    blocks: &[ConfirmationBlock],
) -> Result<ConfirmationResult, StatsError> {
    if plan_replication(&plan.design)? != *plan {
        return Err(StatsError::InvalidInput(
            "sample plan does not match its declared design",
        ));
    }
    if blocks.len() != plan.required_blocks {
        return Err(StatsError::InvalidInput(
            "confirmation must use exactly the frozen planned block count",
        ));
    }
    let ids: Vec<String> = blocks.iter().map(|b| b.block_id.clone()).collect();
    let confirmation = check_ids(&ids)?;
    if plan
        .design
        .pilot_block_ids
        .iter()
        .any(|p| confirmation.contains(p.as_str()))
    {
        return Err(StatsError::InvalidInput(
            "pilot preparation reused as a confirmation block",
        ));
    }
    if blocks.iter().any(|b| !b.independent_preparation) {
        return Err(StatsError::InvalidInput(
            "frames or sub-samples are not independent preparations",
        ));
    }
    let effects: Vec<f64> = blocks.iter().map(|b| b.paired_effect).collect();
    let (mean, variance) = sample_moments(&effects)?;
    if variance == 0.0 {
        return Err(StatsError::Unsupported(
            "zero confirmation variance leaves the t bound undefined; no artificial floor",
        ));
    }
    let n = blocks.len();
    let sd = variance.sqrt();
    let t_critical = student_t_quantile(1.0 - plan.design.alpha_one_sided, n - 1)?;
    let lower = mean - t_critical * sd / (n as f64).sqrt();
    if !lower.is_finite() {
        return Err(StatsError::NumericalFailure(
            "nonfinite lower confidence bound",
        ));
    }
    Ok(ConfirmationResult {
        design_identity: plan.design_identity.clone(),
        planned_blocks: plan.required_blocks,
        n_blocks: n,
        mean_paired_effect: mean,
        sd_paired_effect: sd,
        t_critical,
        lower_confidence_bound: lower,
        claim_threshold: plan.design.claim_threshold,
        qualified: lower >= plan.design.claim_threshold,
        interpretation: format!(
            "one-sided {:.4} lower bound on the mean paired effect; IID normal paired effects of independent preparations assumed, not tested",
            1.0 - plan.design.alpha_one_sided
        ),
    })
}
