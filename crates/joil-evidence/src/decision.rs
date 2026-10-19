//! Source-bound v2 policy preparation. Valid policy is not physical qualification.
//! The application authenticates Registry reviews; submitted booleans never admit a source.
use crate::identity::{check_id, unique_ids, ContentRef};
use crate::observation::Finite;
use serde::{Deserialize, Serialize};
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
pub(crate) fn invalid(message: impl Into<String>) -> DecisionError {
    DecisionError::Invalid(message.into())
}
pub(crate) fn id(value: &str) -> Result<(), DecisionError> {
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
    pub(crate) fn check(&self) -> Result<(), DecisionError> {
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

/// A reviewed limit is bound to its exact feature/quantity as well as its value.
/// Generic uncertainty records cannot be reused as process or geometry limits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitSource {
    pub limit_id: String,
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

mod profile;
pub use profile::*;
