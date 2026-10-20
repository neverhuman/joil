//! V5-19: candidate-specific structural loads, static FEA qualification, buckling and reserve.
//!
//! Nothing here computes a structural answer from a closed-form load. The analytic Lamé and
//! cantilever expressions are *oracles* a native solver has to reproduce before it may be used;
//! a run that rests on a fixed-load formula is refused. Every threshold, tolerance, budget and
//! required factor is a status-bearing [`Field`]; missing ones make the judgement `Unresolved`.
//! The draft 2.5 MPa vessel overpressure is `Proposed` and never decides anything.
//! Nothing in this module is a physical qualification.
use crate::{canonical_hash, invalid, positive, Field, JudgeError, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

type V3 = [f64; 3];

/// Legacy draft external differential (joil-core `DESIGN_VESSEL_DELTA_P_PA`): provisional.
pub const DRAFT_EXTERNAL_DELTA_P_PA: f64 = 2.5e6;
/// CT-35 reference pressure that earlier screens copied onto every candidate.
pub const COPIED_CT35_PRESSURE_PA: f64 = 29468.22;

fn norm(v: V3) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn finite3(v: &V3) -> bool {
    v.iter().all(|x| x.is_finite())
}
fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
fn cross(r: V3, f: V3) -> V3 {
    std::array::from_fn(|i| r[(i + 1) % 3] * f[(i + 2) % 3] - r[(i + 2) % 3] * f[(i + 1) % 3])
}
pub(super) fn prefixed(prefix: &str, v: &Verdict) -> Verdict {
    let tag = |r: &Vec<String>| r.iter().map(|s| format!("{prefix}:{s}")).collect();
    match v {
        Verdict::Pass => Verdict::Pass,
        Verdict::Fail(r) => Verdict::Fail(tag(r)),
        Verdict::Unresolved(r) => Verdict::Unresolved(tag(r)),
    }
}
pub(super) fn open(reason: impl Into<String>) -> Verdict {
    Verdict::Unresolved(vec![reason.into()])
}
pub(super) fn fail(reason: impl Into<String>) -> Verdict {
    Verdict::Fail(vec![reason.into()])
}

// ---------------------------------------------------------------- V5-19-01 load families

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadFamily {
    OperatingFlow,
    ExternalDifferentialUpset,
    SupportPreloadContact,
    Handling,
    Thermal,
}

pub const ALL_FAMILIES: [LoadFamily; 5] = [
    LoadFamily::OperatingFlow,
    LoadFamily::ExternalDifferentialUpset,
    LoadFamily::SupportPreloadContact,
    LoadFamily::Handling,
    LoadFamily::Thermal,
];

impl LoadFamily {
    pub fn name(self) -> &'static str {
        match self {
            Self::OperatingFlow => "operating_flow",
            Self::ExternalDifferentialUpset => "external_differential_upset",
            Self::SupportPreloadContact => "support_preload_contact",
            Self::Handling => "handling",
            Self::Thermal => "thermal",
        }
    }
}

/// One family's external demand. `reference` is the demand at multiplier 1 and belongs to the
/// installation, not to a design; `required_factor` is what the allowable multiplier must beat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyDemand {
    pub unit: String,
    pub description: String,
    pub reference: Field<f64>,
    pub required_factor: Field<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadFamilySet {
    pub schema: String,
    pub set_id: String,
    pub synthetic: bool,
    pub families: BTreeMap<LoadFamily, FamilyDemand>,
}

/// A load set whose identity hash is what every design and reserve must cite.
#[derive(Clone, Debug, PartialEq)]
pub struct FrozenLoads {
    pub sha256: Sha256Hex,
    pub set: LoadFamilySet,
}

impl LoadFamilySet {
    pub fn from_json(bytes: &[u8]) -> Result<Self, JudgeError> {
        let set: Self = serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
        set.check()?;
        Ok(set)
    }

    pub fn check(&self) -> Result<(), JudgeError> {
        if self.schema != "joil.hydrocyclone.load-family-set.v1" || self.set_id.trim().is_empty() {
            return Err(invalid("unknown load-family schema or empty id"));
        }
        for family in ALL_FAMILIES {
            let Some(d) = self.families.get(&family) else {
                return Err(invalid(format!(
                    "family {} missing; use explicit unresolved fields",
                    family.name()
                )));
            };
            if d.unit.trim().is_empty() {
                return Err(invalid(format!("family {} needs a unit", family.name())));
            }
            for f in [&d.reference, &d.required_factor] {
                if let Field::Admitted { value, .. } | Field::Proposed { value, .. } = f {
                    positive(family.name(), *value)?;
                }
            }
        }
        Ok(())
    }

    pub fn freeze(&self) -> Result<FrozenLoads, JudgeError> {
        self.check()?;
        Ok(FrozenLoads {
            sha256: canonical_hash(self)?,
            set: self.clone(),
        })
    }

    /// Unresolved while any reference or required factor is not admitted. Never Fail.
    pub fn verdict(&self) -> Verdict {
        let mut blockers = Vec::new();
        for (family, d) in &self.families {
            for (what, f) in [
                ("reference", &d.reference),
                ("required_factor", &d.required_factor),
            ] {
                if f.admitted().is_none() {
                    blockers.push(format!("load_family:{}:{what}:not_admitted", family.name()));
                }
            }
        }
        if blockers.is_empty() {
            Verdict::Pass
        } else {
            Verdict::Unresolved(blockers)
        }
    }
}

impl FrozenLoads {
    /// External demand of `family` at `multiplier`: identical for every design.
    pub fn demand_at(&self, family: LoadFamily, multiplier: f64) -> Result<f64, JudgeError> {
        positive("load multiplier", multiplier)?;
        let reference = self
            .set
            .families
            .get(&family)
            .and_then(|d| d.reference.admitted())
            .ok_or_else(|| invalid(format!("{} reference not admitted", family.name())))?;
        Ok(reference * multiplier)
    }
}

/// The reference loads a design says it was solved under.
#[derive(Clone, Debug, PartialEq)]
pub struct DesignLoadClaim {
    pub design_sha256: Sha256Hex,
    pub frozen_loads_sha256: Sha256Hex,
    pub reference_loads: BTreeMap<LoadFamily, f64>,
}

/// A design must cite the frozen set and use its exact reference values; a weaker design cannot
/// pick an easier reference load.
pub fn check_design_loads(frozen: &FrozenLoads, claim: &DesignLoadClaim) -> Verdict {
    let mut parts = vec![frozen.set.verdict()];
    if claim.frozen_loads_sha256 != frozen.sha256 {
        parts.push(fail("design_cites_a_different_load_set"));
    }
    for family in ALL_FAMILIES {
        let frozen_ref = frozen
            .set
            .families
            .get(&family)
            .and_then(|d| d.reference.admitted());
        match (claim.reference_loads.get(&family), frozen_ref) {
            (None, _) => parts.push(fail(format!("reference_load_missing:{}", family.name()))),
            (Some(v), _) if !v.is_finite() => {
                parts.push(fail(format!("reference_load_not_finite:{}", family.name())))
            }
            (Some(v), Some(r)) if v != r => parts.push(fail(format!(
                "reference_load_differs_from_frozen:{}",
                family.name()
            ))),
            _ => {}
        }
    }
    Verdict::combine(parts)
}

#[path = "structure/transfer.rs"]
mod transfer;
pub use transfer::*;

#[path = "structure/fixtures.rs"]
mod fixtures;
pub use fixtures::*;

mod qualification;
pub use qualification::*;
