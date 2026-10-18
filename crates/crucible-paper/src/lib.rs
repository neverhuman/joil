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

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle() -> PaperBundle {
        let observation = Observation {
            id: "obs-screen-001".into(),
            subject_sha256: "a".repeat(64),
            scenario_id: "synthetic-scenario-1".into(),
            estimand_id: "screen-score".into(),
            unit: "dimensionless".into(),
            denominator: "independent_search_seed".into(),
            independent_unit_ids: vec!["seed-01".into(), "seed-02".into()],
            tier: "F1_screen".into(),
            tier_current: true,
            value: "0.42".into(),
            interpretation: "synthetic search-screen result; not physical performance".into(),
        };
        let query = ClaimQuery {
            observation_id: observation.id.clone(),
            subject_sha256: observation.subject_sha256.clone(),
            scenario_id: observation.scenario_id.clone(),
            estimand_id: observation.estimand_id.clone(),
            unit: observation.unit.clone(),
            denominator: observation.denominator.clone(),
            tier: observation.tier.clone(),
            allowed_interpretation: observation.interpretation.clone(),
        };
        PaperBundle {
            schema_version: 1,
            observations: vec![observation],
            queries: vec![query],
            figures: vec![],
            attempted_finalist_ids: vec!["synthetic-arm".into()],
            cost_quality: vec![CostQualityPoint {
                finalist_id: "synthetic-arm".into(),
                attempt_id: "attempt-1".into(),
                confirmation_data_id: "synthetic-confirmation-only".into(),
                normalized_resource_cost: "1.0".into(),
                quality_estimate: "0.0".into(),
                uncertainty_low: "-0.1".into(),
                uncertainty_high: "0.1".into(),
                independent_unit_ids: vec!["seed-2301".into(), "seed-2302".into()],
                outcome: "negative".into(),
            }],
        }
    }

    #[test]
    fn g009_guard_figure_only_observation_cannot_bypass_revocation() {
        let mut b = bundle();
        b.queries.clear();
        b.observations[0].tier_current = false;
        b.figures.push(FigureManifest {
            id: "figure".into(),
            observation_ids: vec![b.observations[0].id.clone()],
            generator: "synthetic generator".into(),
            source_sha256: "a".repeat(64),
        });
        assert!(b.validate().is_err());
    }
    #[test]
    fn exact_typed_query_builds() {
        assert!(bundle().validate().is_ok());
    }

    #[test]
    fn wrong_denominator_unit_subject_and_stale_tier_fail() {
        for mutate in 0..4 {
            let mut value = bundle();
            match mutate {
                0 => value.observations[0].denominator = "parcel".into(),
                1 => value.queries[0].unit = "mg/L".into(),
                2 => value.queries[0].subject_sha256 = "b".repeat(64),
                _ => value.observations[0].tier_current = false,
            }
            assert!(value.validate().is_err(), "mutation {mutate} passed");
        }
    }

    #[test]
    fn parcels_and_frames_cannot_inflate_independent_n() {
        let mut value = bundle();
        value.observations[0].denominator = "parcel".into();
        value.observations[0].independent_unit_ids = vec!["frame-1".into(); 1000];
        assert!(value.validate().is_err());
        value.observations[0].denominator = "independent_search_seed".into();
        value.observations[0].independent_unit_ids = vec!["seed-1".into(), "seed-1".into()];
        assert!(value.validate().is_err());
    }

    #[test]
    fn every_attempted_finalist_and_uncertainty_interval_is_reported() {
        let mut value = bundle();
        value.attempted_finalist_ids.push("unreported".into());
        assert!(value.validate().is_err());
        value.attempted_finalist_ids.pop();
        value.cost_quality[0].uncertainty_low = "0.2".into();
        assert!(value.validate().is_err());
    }

    #[test]
    fn near_numeric_value_is_never_claim_identity() {
        let mut value = bundle();
        value.observations[0].value = "420".into();
        assert!(value.validate().is_ok());
        value.queries[0].unit = "mg/L".into();
        assert!(value.validate().is_err());
    }

    #[test]
    fn figure_must_reference_source_manifest_observations() {
        let mut value = bundle();
        value.figures.push(FigureManifest {
            id: "fig-1".into(),
            observation_ids: vec!["missing".into()],
            generator: "tools/build_figures.py".into(),
            source_sha256: "c".repeat(64),
        });
        assert!(matches!(
            value.validate(),
            Err(PaperError::InvalidFigure(_))
        ));
    }

    #[test]
    fn v5_28_29_30_sc_dec_041_to_052_prospective_study_arms_winner_leakage_typed_queries_and_public_bundle(
    ) {
        use serde_json::Value;

        let preregistration: Value = serde_json::from_str(include_str!(
            "../../../experiments/v5-study/preregistration-v1.json"
        ))
        .expect("prospective study protocol is valid JSON");
        assert!(preregistration["status"]
            .as_str()
            .unwrap()
            .contains("frozen-prospective"));
        assert_eq!(preregistration["family_policy"], "all-attempts-retained; no post-result endpoint substitution; any endpoint change creates a new protocol identity");
        let arms = preregistration["arms"].as_array().unwrap();
        for required_arm in ["R0", "R1", "E1", "E2", "A1", "A2"] {
            assert!(arms.iter().any(|arm| arm["id"] == required_arm));
        }
        assert_eq!(preregistration["independent_seeds"]["disjoint"], true);
        assert_eq!(preregistration["confirmation"]["fresh_data"], true);
        assert_eq!(
            preregistration["unavailable_l4_negative_control"]["forbidden"],
            "replace endpoint after inspecting winners"
        );

        let interventions: Value = serde_json::from_str(include_str!(
            "../../../experiments/v5-study/mechanism-interventions-v1.json"
        ))
        .expect("mechanistic intervention plan is valid JSON");
        for required_intervention in [
            "intact",
            "knockout",
            "sham",
            "rescue",
            "matched-volume",
            "matched-area",
        ] {
            assert!(interventions["interventions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == required_intervention));
        }

        let fixture_bytes = include_bytes!("../../../paper/fixtures/negative-result.json");
        let report: PaperBundle =
            serde_json::from_slice(fixture_bytes).expect("synthetic review bundle deserializes");
        report
            .validate()
            .expect("exactly bound typed queries and all attempted outcomes validate");
        assert!(report
            .cost_quality
            .iter()
            .any(|point| point.outcome == "negative"));
        assert_eq!(
            report.attempted_finalist_ids.len(),
            report
                .cost_quality
                .iter()
                .map(|point| point.finalist_id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );

        let mut fuzzy = report.clone();
        fuzzy.queries[0].observation_id = "obs-synthetic-search-001-with-suffix".into();
        assert!(matches!(
            fuzzy.validate(),
            Err(PaperError::MissingObservation(_))
        ));
        let mut winner_leakage = report.clone();
        winner_leakage.cost_quality[0].confirmation_data_id = "".into();
        assert!(winner_leakage.validate().is_err());
        let mut inflated_n = report.clone();
        inflated_n.cost_quality[0].independent_unit_ids = vec!["frame-1"; 1000]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert!(inflated_n.validate().is_err());

        let public_bundle = serde_json::to_string(&report).expect("bundle serializes");
        for private_marker in ["/home/", "/Users/", ".private/", "customer", "facility"] {
            assert!(
                !public_bundle.contains(private_marker),
                "public bundle leaked private marker {private_marker}"
            );
        }
    }

    #[test]
    fn v5_28_04_planted_error_negative_corpus_rejects_denominator_unit_subject_stale_tier_and_fuzzy_numeric_matches(
    ) {
        let baseline = bundle();
        let report = verify_planted_error_negative_corpus(&baseline).unwrap();
        assert!(report.baseline_valid);
        assert_eq!(report.planted_mutations_tested, 5);
        assert_eq!(report.planted_mutations_rejected, 5);
        assert_eq!(
            report.rejected_classes,
            vec![
                "wrong_denominator",
                "wrong_unit",
                "wrong_subject",
                "stale_tier",
                "fuzzy_1000x_or_0_15pct_prose_match"
            ]
        );
        let id = &baseline.queries[0].observation_id;
        assert!(verify_exact_claim_token(&baseline, id, "0.42", "dimensionless").is_ok());
        for value in ["420", "0.42063", "0.420", "NaN"] {
            assert!(verify_exact_claim_token(&baseline, id, value, "dimensionless").is_err());
        }
        assert!(verify_exact_claim_token(&baseline, id, "0.42", "mg/L").is_err());
        assert!(verify_exact_claim_token(&baseline, "absent", "0.42", "dimensionless").is_err());
        let mut bad = baseline.clone();
        bad.observations[0].tier_current = false;
        assert!(verify_planted_error_negative_corpus(&bad).is_err());
        let mut bad = baseline.clone();
        bad.queries.clear();
        assert!(verify_planted_error_negative_corpus(&bad).is_err());
        let mut bad = baseline.clone();
        bad.queries[0].estimand_id.clear();
        assert!(verify_exact_claim_token(&bad, id, "0.42", "dimensionless").is_err());
        assert_eq!(baseline, bundle());
    }
}
