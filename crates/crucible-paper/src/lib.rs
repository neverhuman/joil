use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub id: String,
    pub subject_sha256: String,
    pub scenario_id: String,
    pub estimand_id: String,
    pub unit: String,
    pub denominator: String,
    pub independent_unit_ids: Vec<String>,
    pub tier: String,
    pub tier_current: bool,
    pub value: String,
    pub interpretation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimQuery {
    pub observation_id: String,
    pub subject_sha256: String,
    pub scenario_id: String,
    pub estimand_id: String,
    pub unit: String,
    pub denominator: String,
    pub tier: String,
    pub allowed_interpretation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FigureManifest {
    pub id: String,
    pub observation_ids: Vec<String>,
    pub generator: String,
    pub source_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostQualityPoint {
    pub finalist_id: String,
    pub attempt_id: String,
    pub confirmation_data_id: String,
    pub normalized_resource_cost: String,
    pub quality_estimate: String,
    pub uncertainty_low: String,
    pub uncertainty_high: String,
    pub independent_unit_ids: Vec<String>,
    pub outcome: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaperBundle {
    pub schema_version: u32,
    pub observations: Vec<Observation>,
    pub queries: Vec<ClaimQuery>,
    pub figures: Vec<FigureManifest>,
    pub attempted_finalist_ids: Vec<String>,
    pub cost_quality: Vec<CostQualityPoint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaperError {
    Version,
    DuplicateObservation(String),
    MissingObservation(String),
    WrongBinding(String),
    StaleTier(String),
    Interpretation(String),
    InvalidFigure(String),
}

impl std::fmt::Display for PaperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PaperError {}

fn validate_observation_context(observation: &Observation) -> Result<(), PaperError> {
    if !observation.tier_current {
        return Err(PaperError::StaleTier(observation.id.clone()));
    }
    let units: std::collections::BTreeSet<_> = observation.independent_unit_ids.iter().collect();
    if !matches!(
        observation.denominator.as_str(),
        "independent_search_seed" | "independent_preparation_and_build"
    ) || observation.independent_unit_ids.is_empty()
        || units.len() != observation.independent_unit_ids.len()
        || observation
            .independent_unit_ids
            .iter()
            .any(|id| id.trim().is_empty())
    {
        return Err(PaperError::WrongBinding(observation.id.clone()));
    }
    Ok(())
}

impl PaperBundle {
    pub fn validate(&self) -> Result<(), PaperError> {
        if self.schema_version != 1 {
            return Err(PaperError::Version);
        }
        let mut observations = BTreeMap::new();
        for observation in &self.observations {
            if observations.insert(&observation.id, observation).is_some() {
                return Err(PaperError::DuplicateObservation(observation.id.clone()));
            }
        }
        for query in &self.queries {
            let Some(observation) = observations.get(&query.observation_id) else {
                return Err(PaperError::MissingObservation(query.observation_id.clone()));
            };
            if observation.subject_sha256 != query.subject_sha256
                || observation.scenario_id != query.scenario_id
                || observation.estimand_id != query.estimand_id
                || observation.unit != query.unit
                || observation.denominator != query.denominator
                || observation.tier != query.tier
            {
                return Err(PaperError::WrongBinding(query.observation_id.clone()));
            }
            validate_observation_context(observation)?;
            if observation.interpretation != query.allowed_interpretation {
                return Err(PaperError::Interpretation(query.observation_id.clone()));
            }
        }
        for figure in &self.figures {
            if figure.generator.trim().is_empty()
                || figure.source_sha256.len() != 64
                || figure.observation_ids.is_empty()
                || figure
                    .observation_ids
                    .iter()
                    .any(|id| !observations.contains_key(id))
            {
                return Err(PaperError::InvalidFigure(figure.id.clone()));
            }
        }
        for figure in &self.figures {
            for id in &figure.observation_ids {
                validate_observation_context(observations[id])?;
            }
        }
        let finalists: std::collections::BTreeSet<_> = self.attempted_finalist_ids.iter().collect();
        if finalists.len() != self.attempted_finalist_ids.len() {
            return Err(PaperError::WrongBinding("duplicate finalist".into()));
        }
        let mut represented = std::collections::BTreeSet::new();
        let mut attempts = std::collections::BTreeSet::new();
        for point in &self.cost_quality {
            let parsed = [
                point.normalized_resource_cost.parse::<f64>(),
                point.quality_estimate.parse::<f64>(),
                point.uncertainty_low.parse::<f64>(),
                point.uncertainty_high.parse::<f64>(),
            ];
            if !finalists.contains(&point.finalist_id)
                || point.confirmation_data_id.trim().is_empty()
                || point.independent_unit_ids.is_empty()
                || !matches!(point.outcome.as_str(), "negative" | "feasible" | "failed")
                || !attempts.insert(&point.attempt_id)
                || parsed.iter().any(Result::is_err)
            {
                return Err(PaperError::WrongBinding(point.finalist_id.clone()));
            }
            let [Ok(cost), Ok(estimate), Ok(low), Ok(high)] = parsed else {
                unreachable!()
            };
            if !cost.is_finite()
                || cost < 0.0
                || !estimate.is_finite()
                || !low.is_finite()
                || !high.is_finite()
                || low > estimate
                || estimate > high
            {
                return Err(PaperError::WrongBinding(point.finalist_id.clone()));
            }
            let units: std::collections::BTreeSet<_> = point.independent_unit_ids.iter().collect();
            if units.len() != point.independent_unit_ids.len() {
                return Err(PaperError::WrongBinding(point.finalist_id.clone()));
            }
            represented.insert(&point.finalist_id);
        }
        if finalists != represented {
            return Err(PaperError::WrongBinding(
                "attempted finalist missing from cost-quality report".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlantedErrorAuditReport {
    pub baseline_valid: bool,
    pub planted_mutations_tested: usize,
    pub planted_mutations_rejected: usize,
    pub rejected_classes: Vec<&'static str>,
}

/// query_id is the exact observation_id of a registered typed query, never prose similarity.
pub fn verify_exact_claim_token(
    bundle: &PaperBundle,
    query_id: &str,
    prose_value: &str,
    prose_unit: &str,
) -> Result<(), PaperError> {
    bundle.validate()?;
    let query = bundle
        .queries
        .iter()
        .find(|q| q.observation_id == query_id)
        .ok_or_else(|| PaperError::MissingObservation(query_id.into()))?;
    if query.observation_id.trim().is_empty()
        || query.estimand_id.trim().is_empty()
        || query.unit.trim().is_empty()
        || query.subject_sha256.trim().is_empty()
    {
        return Err(PaperError::WrongBinding(query_id.into()));
    }
    let observation = bundle
        .observations
        .iter()
        .find(|o| o.id == query.observation_id)
        .ok_or_else(|| PaperError::MissingObservation(query_id.into()))?;
    if observation.value != prose_value || observation.unit != prose_unit {
        return Err(PaperError::WrongBinding(query_id.into()));
    }
    Ok(())
}

/// Execute mutation controls against a validated baseline; inability to plant is an error.
pub fn verify_planted_error_negative_corpus(
    bundle: &PaperBundle,
) -> Result<PlantedErrorAuditReport, PaperError> {
    bundle.validate()?;
    let query = bundle
        .queries
        .first()
        .ok_or_else(|| PaperError::MissingObservation("negative_corpus_query".into()))?;
    let index = bundle
        .observations
        .iter()
        .position(|o| o.id == query.observation_id)
        .ok_or_else(|| PaperError::MissingObservation(query.observation_id.clone()))?;
    let observation = &bundle.observations[index];
    verify_exact_claim_token(
        bundle,
        &query.observation_id,
        &observation.value,
        &observation.unit,
    )?;
    let value = observation
        .value
        .parse::<f64>()
        .map_err(|_| PaperError::WrongBinding("negative_corpus_numeric_value".into()))?;
    if !value.is_finite() || value == 0.0 || query.unit == "mg/L" {
        return Err(PaperError::WrongBinding(
            "negative_corpus_inapplicable_baseline".into(),
        ));
    }
    let mut report = PlantedErrorAuditReport {
        baseline_valid: true,
        planted_mutations_tested: 0,
        planted_mutations_rejected: 0,
        rejected_classes: Vec::new(),
    };
    for class in [
        "wrong_denominator",
        "wrong_unit",
        "wrong_subject",
        "stale_tier",
        "fuzzy_1000x_or_0_15pct_prose_match",
    ] {
        let mut mutated = bundle.clone();
        let rejected = match class {
            "wrong_denominator" => {
                mutated.observations[index].denominator = "parcel".into();
                mutated.validate().is_err()
            }
            "wrong_unit" => {
                mutated.queries[0].unit = "mg/L".into();
                mutated.validate().is_err()
            }
            "wrong_subject" => {
                mutated.queries[0].subject_sha256 = if query.subject_sha256 == "b".repeat(64) {
                    "c".repeat(64)
                } else {
                    "b".repeat(64)
                };
                mutated.validate().is_err()
            }
            "stale_tier" => {
                mutated.observations[index].tier_current = false;
                mutated.validate().is_err()
            }
            _ => {
                [value * 1000.0, value * 1.0015].iter().all(|wrong| {
                    verify_exact_claim_token(
                        &mutated,
                        &query.observation_id,
                        &wrong.to_string(),
                        &observation.unit,
                    )
                    .is_err()
                }) && verify_exact_claim_token(
                    &mutated,
                    "unbound-prose-number",
                    &observation.value,
                    &observation.unit,
                )
                .is_err()
            }
        };
        report.planted_mutations_tested += 1;
        if !rejected {
            return Err(PaperError::WrongBinding(format!(
                "planted_error_accepted:{class}"
            )));
        }
        report.planted_mutations_rejected += 1;
        report.rejected_classes.push(class);
    }
    Ok(report)
}

mod release;
pub use self::release::*;

#[cfg(test)]
mod tests;
