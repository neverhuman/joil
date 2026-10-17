//! Source-bound v2 policy preparation. Valid policy is not physical qualification.
//! The application authenticates Registry reviews; submitted booleans never admit a source.
use crate::claims::Registry;
use crate::identity::{check_id, scientific_hash, strict_decode, unique_ids, ContentRef};
use crate::observation::Finite;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum DecisionError {
    Invalid(String),
    Unresolved(Vec<String>),
}
impl fmt::Display for DecisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DecisionError {}
fn invalid(message: impl Into<String>) -> DecisionError {
    DecisionError::Invalid(message.into())
}
fn id(value: &str) -> Result<(), DecisionError> {
    check_id(value).map_err(invalid)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionUnit {
    Pascal,
    JoulePerTreatedCubicMetre,
    CubicMetrePerHour,
    Millimetre,
    MgPerLitreDispersedOil,
    Kilogram,
    LoadMultiplier,
    Dimensionless,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundedValue {
    pub lower: Finite,
    pub upper: Finite,
    pub unit: DecisionUnit,
}
impl BoundedValue {
    pub fn check(&self) -> Result<(), DecisionError> {
        if self.lower > self.upper
            || [self.lower, self.upper]
                .iter()
                .any(|v| v.get() == 0.0 && v.get().is_sign_negative())
        {
            return Err(invalid("reversed_bound_or_negative_zero"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum RequirementValue<T> {
    Admitted {
        value: T,
        source: ContentRef,
        applicability: ContentRef,
    },
    Proposed {
        value: T,
        source: ContentRef,
        applicability: ContentRef,
        reason: String,
    },
    Unresolved {
        reason: String,
        owner: String,
        blocks: Vec<String>,
    },
}
impl<T> RequirementValue<T> {
    fn check(&self) -> Result<(), DecisionError> {
        match self {
            Self::Admitted {
                source,
                applicability,
                ..
            } => {
                source.check().map_err(invalid)?;
                applicability.check().map_err(invalid)
            }
            Self::Proposed {
                source,
                applicability,
                reason,
                ..
            } => {
                source.check().map_err(invalid)?;
                applicability.check().map_err(invalid)?;
                id(reason)
            }
            Self::Unresolved {
                reason,
                owner,
                blocks,
            } => {
                id(reason)?;
                id(owner)?;
                if blocks.is_empty() {
                    return Err(invalid("unresolved_input_needs_affected_claims"));
                }
                unique_ids(blocks.iter().map(String::as_str)).map_err(invalid)
            }
        }
    }
    pub fn is_admitted(&self) -> bool {
        matches!(self, Self::Admitted { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    AtMost,
    AtLeast,
    Within,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitSpec {
    pub id: String,
    pub comparison: Comparison,
    pub unit: DecisionUnit,
    pub mandatory_for: Vec<String>,
    pub value: RequirementValue<BoundedValue>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceValue {
    pub value: BoundedValue,
    pub comparison: Comparison,
    pub applicability: ContentRef,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub q_in_m3_h: Finite,
    pub cin_mg_l: Finite,
    pub d50_feed_um: Finite,
    pub viscosity_factor: Finite,
    pub dsd_basis: String,
    pub concentration_basis: String,
    pub source: ContentRef,
    pub source_pointer: String,
    pub basis_source: ContentRef,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateSpec {
    pub id: String,
    pub estimand: String,
    pub unit: DecisionUnit,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Alternative {
    pub source: ContentRef,
    pub pointer: String,
    pub interpretation: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conflict {
    pub id: String,
    pub alternatives: Vec<Alternative>,
    pub resolution: Option<ContentRef>,
    pub blocks: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictResolution {
    pub conflict_id: String,
    pub alternatives: Vec<Alternative>,
    pub applicability: ContentRef,
    pub resolution_statement: ContentRef,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalSource {
    pub input_id: String,
    pub artifact: ContentRef,
    pub applicability: ContentRef,
    pub claims: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalInput {
    pub id: String,
    pub owner: String,
    pub blocks: Vec<String>,
    pub independent_work: String,
    pub evidence: RequirementValue<ContentRef>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Admission {
    SandboxOnly,
    AdmittedForEvidenceReview,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementSet {
    pub schema_version: u32,
    pub id: String,
    pub revision: String,
    pub admission: Admission,
    pub baseline: ContentRef,
    pub preregistration: ContentRef,
    pub applicability: ContentRef,
    pub concentration_basis: String,
    pub claim_scopes: Vec<String>,
    pub scenarios: Vec<Scenario>,
    pub mandatory_gates: Vec<GateSpec>,
    pub limits: Vec<LimitSpec>,
    pub conflicts: Vec<Conflict>,
    pub external_inputs: Vec<ExternalInput>,
}
impl RequirementSet {
    pub fn from_toml(bytes: &[u8]) -> Result<Self, DecisionError> {
        let text = std::str::from_utf8(bytes).map_err(|e| invalid(e.to_string()))?;
        let value: Self = toml::from_str(text).map_err(|e| invalid(e.to_string()))?;
        value.check()?;
        Ok(value)
    }
    pub fn check(&self) -> Result<(), DecisionError> {
        if self.schema_version != 2
            || self.scenarios.is_empty()
            || self.mandatory_gates.is_empty()
            || self.limits.is_empty()
        {
            return Err(invalid("incomplete_v2_requirements"));
        }
        id(&self.id)?;
        id(&self.revision)?;
        for r in [&self.baseline, &self.preregistration, &self.applicability] {
            r.check().map_err(invalid)?;
        }
        if self.concentration_basis != "dispersed_oil_mg_per_litre" {
            return Err(invalid("ambiguous_concentration_basis"));
        }
        if self.claim_scopes.is_empty() {
            return Err(invalid("missing_claim_scopes"));
        }
        unique_ids(self.claim_scopes.iter().map(String::as_str)).map_err(invalid)?;
        unique_ids(self.scenarios.iter().map(|s| s.id.as_str())).map_err(invalid)?;
        unique_ids(self.mandatory_gates.iter().map(|g| g.id.as_str())).map_err(invalid)?;
        unique_ids(self.limits.iter().map(|l| l.id.as_str())).map_err(invalid)?;
        unique_ids(self.conflicts.iter().map(|c| c.id.as_str())).map_err(invalid)?;
        unique_ids(self.external_inputs.iter().map(|e| e.id.as_str())).map_err(invalid)?;
        for s in &self.scenarios {
            if s.q_in_m3_h.get() <= 0.0
                || s.cin_mg_l.get() < 0.0
                || s.d50_feed_um.get() <= 0.0
                || s.viscosity_factor.get() <= 0.0
                || s.dsd_basis != "oil_volume"
                || s.concentration_basis != self.concentration_basis
                || !s.source_pointer.starts_with('/')
            {
                return Err(invalid(format!("invalid_scenario:{}", s.id)));
            }
            s.source.check().map_err(invalid)?;
            s.basis_source.check().map_err(invalid)?;
        }
        for g in &self.mandatory_gates {
            id(&g.estimand)?;
        }
        for l in &self.limits {
            l.value.check()?;
            if l.mandatory_for.is_empty() {
                return Err(invalid("limit_without_claim_scope"));
            }
            unique_ids(l.mandatory_for.iter().map(String::as_str)).map_err(invalid)?;
            if l.mandatory_for
                .iter()
                .any(|s| !self.claim_scopes.contains(s))
            {
                return Err(invalid("unknown_limit_claim_scope"));
            }
            if let RequirementValue::Admitted { value, .. }
            | RequirementValue::Proposed { value, .. } = &l.value
            {
                value.check()?;
                if value.unit != l.unit {
                    return Err(invalid(format!("limit_unit:{}", l.id)));
                }
            }
        }
        for c in &self.conflicts {
            if c.alternatives.len() < 2 || c.blocks.is_empty() {
                return Err(invalid("incomplete_conflict"));
            }
            if c.blocks.iter().any(|s| !self.claim_scopes.contains(s)) {
                return Err(invalid("unknown_conflict_claim_scope"));
            }
            for a in &c.alternatives {
                a.source.check().map_err(invalid)?;
                id(&a.pointer)?;
                id(&a.interpretation)?;
            }
            if let Some(r) = &c.resolution {
                r.check().map_err(invalid)?;
            }
        }
        for e in &self.external_inputs {
            id(&e.owner)?;
            id(&e.independent_work)?;
            if e.blocks.is_empty() {
                return Err(invalid("external_input_without_affected_claims"));
            }
            if e.blocks.iter().any(|s| !self.claim_scopes.contains(s)) {
                return Err(invalid("unknown_external_claim_scope"));
            }
            e.evidence.check()?;
        }
        Ok(())
    }
    /// Missing/proposed values stay unresolved, never physical Fail or zero uncertainty.
    pub fn qualified_blockers(&self, claim: &str) -> Vec<String> {
        let mut out = Vec::new();
        if self.admission != Admission::AdmittedForEvidenceReview {
            out.push("requirements_sandbox_only".into());
        }
        for l in &self.limits {
            if l.mandatory_for.iter().any(|s| s == claim) && !l.value.is_admitted() {
                out.push(format!("limit:{}", l.id));
            }
        }
        for c in &self.conflicts {
            if c.blocks.iter().any(|s| s == claim) && c.resolution.is_none() {
                out.push(format!("conflict:{}", c.id));
            }
        }
        for e in &self.external_inputs {
            if e.blocks.iter().any(|s| s == claim) && !e.evidence.is_admitted() {
                out.push(format!("external:{}:{}", e.id, e.owner));
            }
        }
        out
    }
}

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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Estimand {
    pub id: String,
    pub unit: DecisionUnit,
    pub direction: Direction,
    pub kind: EstimandKind,
    pub population: ContentRef,
    pub formula: ContentRef,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstimandDefinition {
    pub kind: EstimandKind,
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
    fn check_bound(
        value: &RequirementValue<BoundedValue>,
        unit: DecisionUnit,
        registry: &Registry,
        requirements: &ContentRef,
        applicability: &ContentRef,
    ) -> Result<(), DecisionError> {
        value.check()?;
        let RequirementValue::Admitted {
            value,
            source,
            applicability: scope,
        } = value
        else {
            return Err(DecisionError::Unresolved(vec![
                "uncertainty_bound_not_admitted".into(),
            ]));
        };
        value.check()?;
        let stored: SourceValue = decode_reviewed(registry, source, requirements)?;
        if value.unit != unit
            || value.lower.get() < 0.0
            || scope != applicability
            || stored.value != *value
            || stored.applicability != *scope
            || stored.comparison != Comparison::AtMost
        {
            return Err(invalid("uncertainty_source_unit_value_or_scope"));
        }
        reviewed(registry, scope, requirements)?;
        Ok(())
    }
    fn check_inference(&self, registry: &Registry) -> Result<(), DecisionError> {
        let method = &self.statistical_method;
        let policy: FamilyPolicy =
            decode_reviewed(registry, &method.family_policy, &self.requirements)?;
        if policy.experiment_family_id != self.experiment_family_id
            || policy.requirements != self.requirements
            || policy.max_attempts == 0
            || self.attempt > policy.max_attempts
            || !(0.0 < policy.family_alpha.get() && policy.family_alpha.get() < 1.0)
            || policy.criterion != method.criterion
            || policy.procedure != method.procedure
            || policy.sidedness != method.sidedness
            || policy.claim_scope != self.claim_scope
            || policy.sampling_design != self.sampling_design
            || policy.joint_model != self.joint_model
            || policy.selection_policy != self.selection_policy
            || policy.stopping_policy != self.stopping_policy
            || policy.holdout_policy != self.holdout_policy
            || policy.operating_population != self.operating_population
        {
            return Err(invalid("inference_family_policy_or_reference_mismatch"));
        }
        if policy.selection == Selection::FixedCandidate
            && policy.fixed_subject.as_ref() != Some(&self.subject)
        {
            return Err(invalid("fixed_candidate_subject_changed"));
        }
        let sampling: SamplingDesign =
            decode_reviewed(registry, &self.sampling_design, &self.requirements)?;
        id(&sampling.independent_unit)?;
        id(&sampling.pairing)?;
        reviewed(
            registry,
            &sampling.correlation_treatment,
            &self.requirements,
        )?;
        let joint: JointModel = decode_reviewed(registry, &self.joint_model, &self.requirements)?;
        if joint.sampling_design != self.sampling_design || joint.assumptions.is_empty() {
            return Err(invalid("unjustified_joint_sampling_model"));
        }
        for r in &joint.assumptions {
            reviewed(registry, r, &self.requirements)?;
        }
        if sampling.kind == SamplingKind::FiniteEnvelope
            && self
                .estimands
                .iter()
                .any(|e| matches!(e.kind, EstimandKind::Mean | EstimandKind::Quantile))
        {
            return Err(invalid("finite_envelope_is_not_iid_tail_distribution"));
        }
        let expected: BTreeSet<_> = self.gate_scenario_family.iter().cloned().collect();
        let allocated: BTreeSet<_> = method
            .allocations
            .iter()
            .map(|a| a.member.clone())
            .collect();
        if expected != allocated
            || allocated.len() != method.allocations.len()
            || method
                .allocations
                .iter()
                .any(|a| !(0.0 < a.alpha.get() && a.alpha.get() < 1.0))
        {
            return Err(invalid("incomplete_alpha_allocation"));
        }
        let attempt_budget = policy.family_alpha.get() / f64::from(policy.max_attempts);
        match (method.criterion, method.procedure, policy.selection) {
            (
                ErrorCriterion::FixedCandidateConjunction,
                Procedure::IntersectionUnion,
                Selection::FixedCandidate,
            ) if method.sidedness != Sidedness::TwoSided => {
                if method
                    .allocations
                    .iter()
                    .any(|a| a.alpha.get() > attempt_budget)
                {
                    return Err(invalid("conjunction_attempt_alpha_exceeded"));
                }
            }
            (
                ErrorCriterion::SimultaneousCoverage | ErrorCriterion::AdaptiveWinnerQualification,
                Procedure::Bonferroni,
                _,
            ) => {
                if method
                    .allocations
                    .iter()
                    .map(|a| a.alpha.get())
                    .sum::<f64>()
                    > attempt_budget
                {
                    return Err(invalid("simultaneous_or_adaptive_alpha_exceeded"));
                }
            }
            _ => {
                return Err(invalid(
                    "unsupported_error_criterion_procedure_or_selection",
                ))
            }
        }
        Ok(())
    }
    pub fn qualified_admission(
        &self,
        requirements: &RequirementSet,
        registry: &Registry,
    ) -> Result<(), DecisionError> {
        self.validate(requirements, registry)?;
        let mut blockers = requirements.qualified_blockers(&self.claim_scope);
        for l in &requirements.limits {
            if !l.mandatory_for.contains(&self.claim_scope) {
                continue;
            }
            if let RequirementValue::Admitted {
                value,
                source,
                applicability,
            } = &l.value
            {
                let stored: SourceValue = decode_reviewed(registry, source, &self.requirements)?;
                if stored.value != *value
                    || stored.comparison != l.comparison
                    || stored.applicability != *applicability
                    || applicability != &self.applicability
                {
                    return Err(invalid(format!("limit_source_drift:{}", l.id)));
                }
            }
        }
        for e in &requirements.external_inputs {
            if e.blocks.contains(&self.claim_scope) {
                if let RequirementValue::Admitted {
                    value,
                    source,
                    applicability,
                } = &e.evidence
                {
                    let stored: ExternalSource =
                        decode_reviewed(registry, source, &self.requirements)?;
                    if &stored.artifact != value
                        || stored.input_id != e.id
                        || stored.claims != e.blocks
                        || stored.applicability != *applicability
                        || applicability != &self.applicability
                    {
                        return Err(invalid("external_source_scope_or_value"));
                    }
                    reviewed(registry, value, &self.requirements)?;
                }
            }
        }
        for c in &requirements.conflicts {
            if c.blocks.contains(&self.claim_scope) {
                if let Some(r) = &c.resolution {
                    let stored: ConflictResolution =
                        decode_reviewed(registry, r, &self.requirements)?;
                    if stored.conflict_id != c.id
                        || stored.alternatives != c.alternatives
                        || stored.applicability != self.applicability
                    {
                        return Err(invalid("conflict_resolution_source_or_scope"));
                    }
                    reviewed(registry, &stored.resolution_statement, &self.requirements)?;
                }
            }
        }
        // Evidence review readiness is not a physical verdict. Claim admission
        // still requires the trusted registry's complete observable gate closure.
        if blockers.is_empty() {
            Ok(())
        } else {
            blockers.sort();
            Err(DecisionError::Unresolved(blockers))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignMode {
    Sandbox,
    Qualified,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignRequest {
    pub schema_version: u32,
    pub mode: CampaignMode,
    pub synthetic_software_only: bool,
    pub requirements: ContentRef,
    pub decision_profile: Option<ContentRef>,
}
impl CampaignRequest {
    pub fn from_toml(bytes: &[u8]) -> Result<Self, DecisionError> {
        toml::from_str(std::str::from_utf8(bytes).map_err(|e| invalid(e.to_string()))?)
            .map_err(|e| invalid(e.to_string()))
    }
    pub fn preflight(&self, requirement_bytes: &[u8]) -> Result<RequirementSet, DecisionError> {
        let requirements = RequirementSet::from_toml(requirement_bytes)?;
        let actual =
            ContentRef::from_bytes(&requirements.id, &requirements.revision, requirement_bytes);
        if self.schema_version != 1 || self.requirements != actual {
            return Err(invalid("campaign_requirement_identity_mismatch"));
        }
        if self.mode == CampaignMode::Sandbox && self.synthetic_software_only {
            return Ok(requirements);
        }
        // This template is deliberately not an execution/admission shortcut.
        let mut blockers = requirements.qualified_blockers("computational_comparator");
        if self.decision_profile.is_none() {
            blockers.push("decision_profile_missing".into());
        }
        blockers.push("trusted_observable_gate_closure_required_before_execution".into());
        Err(DecisionError::Unresolved(blockers))
    }
}
