use super::*;
use crate::claims::Registry;
use crate::identity::{scientific_hash, strict_decode, unique_ids, ContentRef};
use crate::observation::Finite;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Minimize,
    Maximize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimandKind {
    Mean,
    Quantile,
    WorstCase,
    Point,
}
/// Input-domain obligations are separate from the statistical functional.
/// Omitted legacy domains carry no inference authority.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimandInputDomain {
    #[default]
    Unspecified,
    RealValued,
    StrictlyPositiveBaseline,
}
impl EstimandInputDomain {
    fn is_unspecified(&self) -> bool {
        *self == Self::Unspecified
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Estimand {
    pub id: String,
    pub unit: DecisionUnit,
    pub direction: Direction,
    pub kind: EstimandKind,
    #[serde(default, skip_serializing_if = "EstimandInputDomain::is_unspecified")]
    pub input_domain: EstimandInputDomain,
    pub population: ContentRef,
    pub formula: ContentRef,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstimandDefinition {
    pub kind: EstimandKind,
    #[serde(default, skip_serializing_if = "EstimandInputDomain::is_unspecified")]
    pub input_domain: EstimandInputDomain,
    pub unit: DecisionUnit,
    pub direction: Direction,
    pub population: ContentRef,
    pub expression: ContentRef,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyMember {
    pub gate: String,
    pub scenario: String,
    pub estimand: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UncertaintyBound {
    pub estimand: String,
    pub bound: RequirementValue<BoundedValue>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCriterion {
    SimultaneousCoverage,
    FixedCandidateConjunction,
    AdaptiveWinnerQualification,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Procedure {
    Bonferroni,
    IntersectionUnion,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sidedness {
    Lower,
    Upper,
    TwoSided,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Selection {
    FixedCandidate,
    AdaptiveSearch,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlphaAllocation {
    pub member: FamilyMember,
    pub alpha: Finite,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticalMethod {
    pub criterion: ErrorCriterion,
    pub procedure: Procedure,
    pub sidedness: Sidedness,
    pub allocations: Vec<AlphaAllocation>,
    pub family_policy: ContentRef,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyPolicy {
    pub experiment_family_id: String,
    pub requirements: ContentRef,
    pub family_alpha: Finite,
    pub max_attempts: u32,
    pub criterion: ErrorCriterion,
    pub procedure: Procedure,
    pub sidedness: Sidedness,
    pub selection: Selection,
    pub fixed_subject: Option<ContentRef>,
    pub claim_scope: String,
    pub sampling_design: ContentRef,
    pub joint_model: ContentRef,
    pub selection_policy: ContentRef,
    pub stopping_policy: ContentRef,
    pub holdout_policy: ContentRef,
    pub operating_population: ContentRef,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplingKind {
    Iid,
    IndependentBlocks,
    FiniteEnvelope,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplingDesign {
    pub kind: SamplingKind,
    pub independent_unit: String,
    pub pairing: String,
    pub correlation_treatment: ContentRef,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointModel {
    pub sampling_design: ContentRef,
    pub assumptions: Vec<ContentRef>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningPrecision {
    pub estimand: String,
    pub useful_effect: BoundedValue,
    pub maximum_uncertainty_fraction: Finite,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownPolicy {
    Abstain,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionProfile {
    pub schema_version: u32,
    pub experiment_family_id: String,
    pub attempt: u32,
    pub requirements: ContentRef,
    pub subject: ContentRef,
    pub baseline: ContentRef,
    pub applicability: ContentRef,
    pub claim_scope: String,
    pub estimands: Vec<Estimand>,
    pub gate_scenario_family: Vec<FamilyMember>,
    pub sampling_design: ContentRef,
    pub joint_model: ContentRef,
    pub numerical_bounds: Vec<UncertaintyBound>,
    pub model_discrepancy: Vec<UncertaintyBound>,
    pub statistical_method: StatisticalMethod,
    pub selection_policy: ContentRef,
    pub stopping_policy: ContentRef,
    pub holdout_policy: ContentRef,
    pub operating_population: ContentRef,
    pub unknown_policy: UnknownPolicy,
    pub planning_precision: Vec<PlanningPrecision>,
    pub executable_acceptance_fixtures: Vec<ContentRef>,
}

fn reviewed<'a>(
    registry: &'a Registry,
    reference: &ContentRef,
    requirements: &ContentRef,
) -> Result<&'a [u8], DecisionError> {
    registry
        .checked_artifact(reference, std::slice::from_ref(requirements))
        .map_err(|e| invalid(format!("source:{}:{e}", reference.id)))
}
fn decode_reviewed<T: serde::de::DeserializeOwned>(
    registry: &Registry,
    r: &ContentRef,
    requirements: &ContentRef,
) -> Result<T, DecisionError> {
    strict_decode(reviewed(registry, r, requirements)?).map_err(invalid)
}

impl DecisionProfile {
    pub fn from_json(bytes: &[u8]) -> Result<Self, DecisionError> {
        strict_decode(bytes).map_err(invalid)
    }
    pub fn identity(&self) -> Result<String, DecisionError> {
        scientific_hash(self).map_err(invalid)
    }
    pub fn check_continuation(&self, previous: &Self) -> Result<(), DecisionError> {
        if self.experiment_family_id != previous.experiment_family_id
            || self.statistical_method.family_policy != previous.statistical_method.family_policy
            || self.requirements != previous.requirements
            || previous.attempt.checked_add(1) != Some(self.attempt)
        {
            return Err(invalid("fork_cannot_reset_family_attempt_or_alpha_policy"));
        }
        Ok(())
    }
    /// Checks the declared inference procedure and exact reviewed inputs only.
    /// Measurement sufficiency, actual alpha spending and physical gates remain separate.
    pub fn validate(
        &self,
        requirements: &RequirementSet,
        registry: &Registry,
    ) -> Result<(), DecisionError> {
        requirements.check()?;
        id(&self.experiment_family_id)?;
        id(&self.claim_scope)?;
        if !requirements.claim_scopes.contains(&self.claim_scope) {
            return Err(invalid("unknown_claim_scope"));
        }
        let approved =
            RequirementSet::from_toml(reviewed(registry, &self.requirements, &self.requirements)?)?;
        if &approved != requirements {
            return Err(invalid("requirement_identity_or_family_changed"));
        }
        if self.schema_version != 1
            || self.attempt == 0
            || self.baseline != requirements.baseline
            || self.applicability != requirements.applicability
        {
            return Err(invalid("profile_schema_subject_or_scope"));
        }
        for r in [
            &self.subject,
            &self.baseline,
            &self.applicability,
            &self.sampling_design,
            &self.joint_model,
            &self.selection_policy,
            &self.stopping_policy,
            &self.holdout_policy,
            &self.operating_population,
        ] {
            reviewed(registry, r, &self.requirements)?;
        }
        if self.estimands.is_empty()
            || self.executable_acceptance_fixtures.is_empty()
            || self.planning_precision.is_empty()
        {
            return Err(invalid("empty_estimands_fixtures_or_precision"));
        }
        unique_ids(self.estimands.iter().map(|e| e.id.as_str())).map_err(invalid)?;
        for e in &self.estimands {
            reviewed(registry, &e.population, &self.requirements)?;
            let definition: EstimandDefinition =
                decode_reviewed(registry, &e.formula, &self.requirements)?;
            if definition.kind != e.kind
                || definition.input_domain != e.input_domain
                || definition.unit != e.unit
                || definition.direction != e.direction
                || definition.population != e.population
            {
                return Err(invalid("estimand_definition_or_population_drift"));
            }
            reviewed(registry, &definition.expression, &self.requirements)?;
        }
        let expected: BTreeSet<_> = requirements
            .scenarios
            .iter()
            .flat_map(|s| {
                requirements
                    .mandatory_gates
                    .iter()
                    .map(move |g| FamilyMember {
                        gate: g.id.clone(),
                        scenario: s.id.clone(),
                        estimand: g.estimand.clone(),
                    })
            })
            .collect();
        let actual: BTreeSet<_> = self.gate_scenario_family.iter().cloned().collect();
        if actual != expected || actual.len() != self.gate_scenario_family.len() {
            return Err(invalid("incomplete_or_duplicate_gate_scenario_family"));
        }
        for g in &requirements.mandatory_gates {
            if !self
                .estimands
                .iter()
                .any(|e| e.id == g.estimand && e.unit == g.unit)
            {
                return Err(invalid(format!("estimand_unit_or_reference:{}", g.id)));
            }
        }
        let estimand_ids: BTreeSet<_> = self.estimands.iter().map(|e| e.id.as_str()).collect();
        if estimand_ids
            != requirements
                .mandatory_gates
                .iter()
                .map(|g| g.estimand.as_str())
                .collect()
        {
            return Err(invalid("unregistered_estimand_family"));
        }
        for bounds in [&self.numerical_bounds, &self.model_discrepancy] {
            let ids: BTreeSet<_> = bounds.iter().map(|b| b.estimand.as_str()).collect();
            if ids != estimand_ids || ids.len() != bounds.len() {
                return Err(invalid("empty_or_incomplete_uncertainty_inventory"));
            }
            for bound in bounds {
                let expected_unit = self
                    .estimands
                    .iter()
                    .find(|e| e.id == bound.estimand)
                    .unwrap()
                    .unit;
                Self::check_bound(
                    &bound.bound,
                    expected_unit,
                    registry,
                    &self.requirements,
                    &self.applicability,
                )?;
            }
        }
        for p in &self.planning_precision {
            p.useful_effect.check()?;
            if !self
                .estimands
                .iter()
                .any(|e| e.id == p.estimand && e.unit == p.useful_effect.unit)
                || p.useful_effect.lower.get() <= 0.0
                || !(0.0 < p.maximum_uncertainty_fraction.get()
                    && p.maximum_uncertainty_fraction.get() <= 1.0 / 3.0)
            {
                return Err(invalid("invalid_planning_precision_not_inference"));
            }
        }
        for r in &self.executable_acceptance_fixtures {
            reviewed(registry, r, &self.requirements)?;
        }
        for s in &requirements.scenarios {
            let source: serde_json::Value =
                decode_reviewed(registry, &s.source, &self.requirements)?;
            let row = source
                .pointer(&s.source_pointer)
                .ok_or_else(|| invalid("scenario_source_pointer"))?;
            for (key, value) in [
                ("q_in_m3_h", s.q_in_m3_h),
                ("cin_mg_l", s.cin_mg_l),
                ("d50_feed_um", s.d50_feed_um),
                ("viscosity_factor", s.viscosity_factor),
            ] {
                if row.get(key).and_then(|v| v.as_f64()) != Some(value.get()) {
                    return Err(invalid(format!("scenario_source_value:{}:{key}", s.id)));
                }
            }
            if row.get("duty_id").and_then(|v| v.as_str()) != Some(s.id.as_str()) {
                return Err(invalid("scenario_source_id"));
            }
            let basis: serde_json::Value =
                decode_reviewed(registry, &s.basis_source, &self.requirements)?;
            if basis
                .pointer("/reference/dsd_basis")
                .and_then(|v| v.as_str())
                != Some(s.dsd_basis.as_str())
            {
                return Err(invalid("number_volume_basis_source_mismatch"));
            }
        }
        self.check_inference(registry)?;
        Ok(())
    }
}

#[path = "profile_inference.rs"]
mod profile_inference;
pub use profile_inference::*;
