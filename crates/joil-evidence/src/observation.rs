//! Typed scientific scalars. Failure and missingness have no numerical projection.
use crate::identity::{check_id, ContentRef, MaterialState, ScientificIdentity, SubjectRef};
use crate::Verdict;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Adapted from Jarambit assurance.rs; nonfinite values cannot inhabit a scalar.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Finite(f64);
impl Finite {
    pub fn new(value: f64) -> Result<Self, String> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err("nonfinite_value".into())
        }
    }
    pub fn get(self) -> f64 {
        self.0
    }
}
impl Serialize for Finite {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.0)
    }
}
impl<'de> Deserialize<'de> for Finite {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Number;
        impl serde::de::Visitor<'_> for Number {
            type Value = Finite;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("finite binary64")
            }
            fn visit_f64<E: serde::de::Error>(self, x: f64) -> Result<Finite, E> {
                Finite::new(x).map_err(E::custom)
            }
            fn visit_u64<E: serde::de::Error>(self, x: u64) -> Result<Finite, E> {
                let f = x as f64;
                if f as u128 != x as u128 {
                    return Err(E::custom("inexact integer scalar"));
                }
                self.visit_f64(f)
            }
            fn visit_i64<E: serde::de::Error>(self, x: i64) -> Result<Finite, E> {
                let f = x as f64;
                if f as i128 != x as i128 {
                    return Err(E::custom("inexact integer scalar"));
                }
                self.visit_f64(f)
            }
        }
        deserializer.deserialize_any(Number)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Dimensionless,
    Pascal,
    MgPerLitreDispersedOil,
    CubicMetrePerHour,
    JoulePerTreatedCubicMetre,
    Metre,
    Millimetre,
    CubicMillimetre,
    Megapascal,
    Second,
    Hertz,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fidelity {
    Identity,
    F0,
    F1,
    F2,
    F3,
    F4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTier {
    SoftwareContract,
    Geometry,
    AnalyticScreen,
    CoupledScreen,
    NumericallyVerified,
    ModelChallenge,
    PhysicalValidation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Synthetic,
    Simulation,
    Measurement,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Execution {
    Succeeded,
    Failed,
    NotRun,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Numerical {
    Verified,
    Unverified,
    Invalid,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Validation {
    Validated,
    NotPerformed,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceState {
    pub execution: Execution,
    pub numerical: Numerical,
    pub validation: Validation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Uncertainty {
    pub component: String,
    pub absolute: Finite,
    pub unit: Unit,
    pub method: String,
    pub dependence: String,
    pub coverage: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub value: Finite,
    pub unit: Unit,
    pub quantity: String,
    pub frame: String,
    pub region: String,
    pub recovery: ContentRef,
    pub fidelity: Fidelity,
    pub tier: EvidenceTier,
    pub origin: Origin,
    pub identity: ScientificIdentity,
    pub evidence: Vec<ContentRef>,
    pub applicability: ContentRef,
    pub state: EvidenceState,
    pub uncertainty: Vec<Uncertainty>,
}

impl Measurement {
    pub fn check(&self) -> Result<(), String> {
        self.identity.check()?;
        for id in [&self.quantity, &self.frame, &self.region] {
            check_id(id)?;
        }
        self.recovery.check()?;
        self.applicability.check()?;
        if self.evidence.is_empty() || self.uncertainty.is_empty() {
            return Err("missing_source_or_uncertainty".into());
        }
        for reference in &self.evidence {
            reference.check()?;
        }
        if self.state.execution != Execution::Succeeded
            || self.state.numerical == Numerical::Invalid
            || self.state.validation == Validation::Invalid
        {
            return Err("invalid_measured_state".into());
        }
        for u in &self.uncertainty {
            if u.absolute.get() < 0.0 || u.unit != self.unit {
                return Err("invalid_uncertainty_units_or_sign".into());
            }
            for id in [&u.component, &u.method, &u.dependence, &u.coverage] {
                check_id(id)?;
            }
        }
        if self.origin == Origin::Synthetic && self.tier != EvidenceTier::SoftwareContract {
            return Err("synthetic_cannot_be_physical_evidence".into());
        }
        let fidelity_valid = match self.tier {
            EvidenceTier::SoftwareContract => true,
            EvidenceTier::Geometry => self.fidelity == Fidelity::F0,
            EvidenceTier::AnalyticScreen | EvidenceTier::CoupledScreen => {
                self.fidelity == Fidelity::F1
            }
            EvidenceTier::NumericallyVerified => self.fidelity == Fidelity::F2,
            EvidenceTier::ModelChallenge => self.fidelity == Fidelity::F3,
            EvidenceTier::PhysicalValidation => self.fidelity == Fidelity::F4,
        };
        if !fidelity_valid {
            return Err("tier_fidelity_mismatch".into());
        }
        if self.tier >= EvidenceTier::NumericallyVerified
            && self.state.numerical != Numerical::Verified
        {
            return Err("numerical_verification_missing".into());
        }
        if (self.state.validation == Validation::Validated
            || self.tier == EvidenceTier::PhysicalValidation)
            && (self.origin != Origin::Measurement
                || self.fidelity != Fidelity::F4
                || self.tier != EvidenceTier::PhysicalValidation
                || self.state.validation != Validation::Validated
                || !matches!(self.identity.subject, SubjectRef::Specimen { .. })
                || matches!(self.identity.material, MaterialState::Unknown { .. }))
        {
            return Err("physical_validation_requires_identified_specimen_measurement".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    Geometry,
    Numerical,
    Infrastructure,
    OutOfDomain,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observation {
    Measured {
        measurement: Box<Measurement>,
    },
    Failed {
        class: FailureClass,
        reason: String,
        diagnostics: Vec<ContentRef>,
    },
    Unknown {
        reason: String,
    },
    NotApplicable {
        reason: String,
    },
    Insufficient {
        reason: String,
        missing: Vec<String>,
    },
}

impl Observation {
    pub fn check(&self) -> Result<(), String> {
        match self {
            Self::Measured { measurement } => measurement.check(),
            Self::Failed {
                reason,
                diagnostics,
                ..
            } => {
                check_id(reason)?;
                for reference in diagnostics {
                    reference.check()?;
                }
                Ok(())
            }
            Self::Unknown { reason } | Self::NotApplicable { reason } => check_id(reason),
            Self::Insufficient { reason, missing } => {
                check_id(reason)?;
                if missing.is_empty() {
                    return Err("insufficient_needs_missing_inventory".into());
                }
                for item in missing {
                    check_id(item)?;
                }
                Ok(())
            }
        }
    }

    /// Point comparison only. Claim resolution additionally checks source bytes, identity,
    /// frozen inventory and reviews; no caller can turn this into physical validation.
    pub fn compare_at_most(&self, limit: Finite, unit: Unit, tier: EvidenceTier) -> Verdict {
        if self.check().is_err() {
            return Verdict::Invalid;
        }
        match self {
            Self::Measured { measurement: m } if m.unit == unit && m.tier >= tier => {
                if m.value.get() <= limit.get() {
                    Verdict::Pass
                } else {
                    Verdict::Fail
                }
            }
            Self::Measured { measurement } if measurement.unit != unit => Verdict::Invalid,
            _ => Verdict::Incomplete,
        }
    }
}

pub fn ingest_observation(bytes: &[u8]) -> Result<Observation, String> {
    let observation: Observation = crate::identity::strict_decode(bytes)?;
    observation.check()?;
    Ok(observation)
}
