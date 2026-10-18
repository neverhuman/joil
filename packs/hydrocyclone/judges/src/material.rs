//! V5-13-01: process-qualified material passports.
//!
//! A material name never implies a qualification. An allowable is usable for a claim only
//! when the passport's process context (alloy/product form, LPBF parameters, orientation,
//! heat treatment/HIP, finish, temperature/environment, defect assumptions) is admitted with
//! sources and the allowable's own evidence context equals that context, key by key.
use crate::{invalid, Field, JudgeError, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextKey {
    AlloyProductForm,
    LpbfParameters,
    Orientation,
    HeatTreatmentHip,
    Finish,
    TemperatureEnvironment,
    DefectAssumptions,
}

pub const ALL_CONTEXT: [ContextKey; 7] = [
    ContextKey::AlloyProductForm,
    ContextKey::LpbfParameters,
    ContextKey::Orientation,
    ContextKey::HeatTreatmentHip,
    ContextKey::Finish,
    ContextKey::TemperatureEnvironment,
    ContextKey::DefectAssumptions,
];

impl ContextKey {
    pub fn name(self) -> &'static str {
        match self {
            Self::AlloyProductForm => "alloy_product_form",
            Self::LpbfParameters => "lpbf_parameters",
            Self::Orientation => "orientation",
            Self::HeatTreatmentHip => "heat_treatment_hip",
            Self::Finish => "finish",
            Self::TemperatureEnvironment => "temperature_environment",
            Self::DefectAssumptions => "defect_assumptions",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowableKind {
    Yield,
    FatigueEndurance,
    SourServiceEligibility,
}

impl AllowableKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Yield => "yield",
            Self::FatigueEndurance => "fatigue_endurance",
            Self::SourServiceEligibility => "sour_service_eligibility",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimKind {
    MassDensity,
    StaticStrength,
    AsBuiltFatigue,
    SourService,
}

impl ClaimKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::MassDensity => "mass_density",
            Self::StaticStrength => "static_strength",
            Self::AsBuiltFatigue => "as_built_fatigue",
            Self::SourService => "sour_service",
        }
    }
    fn context(self) -> &'static [ContextKey] {
        match self {
            Self::MassDensity => &[
                ContextKey::AlloyProductForm,
                ContextKey::LpbfParameters,
                ContextKey::HeatTreatmentHip,
            ],
            Self::StaticStrength => &[
                ContextKey::AlloyProductForm,
                ContextKey::LpbfParameters,
                ContextKey::Orientation,
                ContextKey::HeatTreatmentHip,
                ContextKey::Finish,
            ],
            Self::AsBuiltFatigue | Self::SourService => &ALL_CONTEXT,
        }
    }
    fn allowable(self) -> Option<AllowableKind> {
        match self {
            Self::MassDensity => None,
            Self::StaticStrength => Some(AllowableKind::Yield),
            Self::AsBuiltFatigue => Some(AllowableKind::FatigueEndurance),
            Self::SourService => Some(AllowableKind::SourServiceEligibility),
        }
    }
}

/// Density bounds in kg/m^3 for the passport's process condition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub lower: f64,
    pub upper: f64,
}

impl Bounds {
    pub fn check(&self, name: &str) -> Result<(), JudgeError> {
        if self.lower.is_finite()
            && self.upper.is_finite()
            && 0.0 < self.lower
            && self.lower <= self.upper
        {
            Ok(())
        } else {
            Err(invalid(format!(
                "{name} needs finite positive ordered bounds"
            )))
        }
    }
}

/// An allowable and the exact process context its evidence was generated under.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Allowable {
    pub value: f64,
    pub unit: String,
    pub evidence_context: BTreeMap<ContextKey, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialPassport {
    pub schema: String,
    pub passport_id: String,
    pub synthetic: bool,
    pub context: BTreeMap<ContextKey, Field<String>>,
    pub density_kg_m3: Field<Bounds>,
    pub allowables: BTreeMap<AllowableKind, Field<Allowable>>,
}

impl MaterialPassport {
    pub fn from_json(bytes: &[u8]) -> Result<Self, JudgeError> {
        let passport: Self = serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
        passport.check()?;
        Ok(passport)
    }

    pub fn check(&self) -> Result<(), JudgeError> {
        if self.schema != "joil.hydrocyclone.material-passport.v1"
            || self.passport_id.trim().is_empty()
        {
            return Err(invalid("unknown passport schema or empty id"));
        }
        for key in ALL_CONTEXT {
            match self.context.get(&key) {
                None => {
                    return Err(invalid(format!(
                        "context {} missing; use an explicit unresolved field",
                        key.name()
                    )))
                }
                Some(Field::Admitted { value, .. } | Field::Proposed { value, .. })
                    if value.trim().is_empty() =>
                {
                    return Err(invalid(format!("context {} is empty", key.name())))
                }
                _ => {}
            }
        }
        if let Field::Admitted { value, .. } | Field::Proposed { value, .. } = &self.density_kg_m3 {
            value.check("density")?;
        }
        for (kind, field) in &self.allowables {
            if let Field::Admitted { value: a, .. } | Field::Proposed { value: a, .. } = field {
                if !a.value.is_finite() || a.value <= 0.0 || a.unit.trim().is_empty() {
                    return Err(invalid(format!(
                        "allowable {} needs a positive finite value and unit",
                        kind.name()
                    )));
                }
            }
        }
        Ok(())
    }

    /// Judge whether this passport supports a claim. Never `Fail`: missing process evidence
    /// is an unresolved input, not a physical failure.
    pub fn qualify(&self, claim: ClaimKind) -> Verdict {
        let mut blockers = Vec::new();
        for key in claim.context() {
            if self.context.get(key).and_then(Field::admitted).is_none() {
                blockers.push(format!("context:{}:not_admitted", key.name()));
            }
        }
        if matches!(claim, ClaimKind::MassDensity) && self.density_kg_m3.admitted().is_none() {
            blockers.push("density:not_admitted".into());
        }
        if let Some(kind) = claim.allowable() {
            match self.allowables.get(&kind).and_then(Field::admitted) {
                None => blockers.push(format!("allowable:{}:not_admitted", kind.name())),
                Some(allowable) => {
                    for key in claim.context() {
                        let Some(declared) = self.context.get(key).and_then(Field::admitted) else {
                            continue;
                        };
                        match allowable.evidence_context.get(key) {
                            None => blockers.push(format!(
                                "allowable:{}:evidence_context_missing:{}",
                                kind.name(),
                                key.name()
                            )),
                            Some(evidence) if evidence != declared => blockers.push(format!(
                                "allowable:{}:context_mismatch:{}",
                                kind.name(),
                                key.name()
                            )),
                            Some(_) => {}
                        }
                    }
                }
            }
        }
        Verdict::combine([if blockers.is_empty() {
            Verdict::Pass
        } else {
            Verdict::Unresolved(blockers)
        }])
    }
}
