//! Frozen objectives and explicit sandbox/claim-bearing admission.
use crate::identity::{check_id, scientific_hash, ContentRef};
use crate::observation::{Finite, Unit};
use crate::Verdict;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const REQUIRED_P0_GATES: &[&str] = &[
    "variation_no_fallback",
    "schema_preserved",
    "geometry_parity",
    "measured_f0",
    "independent_judge",
    "gci_valid",
    "conservative_load_transfer",
    "candidate_structure",
    "requirements_admitted",
    "objective_contract_matches",
    "independent_labels",
    "claim_inventory_complete",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignMode {
    Sandbox,
    ClaimBearing,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionState {
    SandboxOnly,
    Admitted,
    Suspended,
    Superseded,
    Closed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sense {
    Minimize,
    Maximize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transform {
    Identity,
    OneMinus,
    Negate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectiveSpec {
    pub id: String,
    pub source_id: String,
    pub sense: Sense,
    pub unit: Unit,
    pub transform: Transform,
    pub reference_point: Option<Finite>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thresholds {
    pub quality_limit_mg_l: Finite,
    pub stretch_limit_mg_l: Finite,
    /// Historical search policy, applicable to sandbox screens only.
    pub sandbox_pressure_ratio_max: Finite,
    pub sandbox_pdr_min: Finite,
    pub pressure_budget_pa: Option<Finite>,
    pub energy_budget_j_per_treated_m3: Option<Finite>,
    pub absolute_pressure_min_pa: Option<Finite>,
    pub duty_band_m3_h: Option<[Finite; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirements {
    pub schema: u32,
    pub id: String,
    pub revision: String,
    pub admission: AdmissionState,
    pub scope: String,
    pub concentration_basis: String,
    pub coordinate_frame: String,
    pub baseline: ContentRef,
    pub preregistration: ContentRef,
    pub scenarios: Vec<ContentRef>,
    pub objectives: Vec<ObjectiveSpec>,
    pub thresholds: Thresholds,
    pub blockers: Vec<String>,
}

impl Requirements {
    pub fn from_toml(bytes: &[u8]) -> Result<Self, String> {
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let requirements: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        requirements.check()?;
        Ok(requirements)
    }

    pub fn check(&self) -> Result<(), String> {
        if self.schema != 1 || self.objectives.is_empty() || self.scenarios.is_empty() {
            return Err("incomplete_requirement_contract".into());
        }
        for id in [
            &self.id,
            &self.revision,
            &self.scope,
            &self.coordinate_frame,
        ] {
            check_id(id)?;
        }
        if self.concentration_basis != "dispersed_oil_mg_per_litre" {
            return Err("ambiguous_concentration_basis".into());
        }
        self.baseline.check()?;
        self.preregistration.check()?;
        crate::identity::unique_ids(self.scenarios.iter().map(|s| s.id.as_str()))?;
        for scenario in &self.scenarios {
            scenario.check()?;
        }
        crate::identity::unique_ids(self.objectives.iter().map(|o| o.id.as_str()))?;
        for objective in &self.objectives {
            check_id(&objective.id)?;
            check_id(&objective.source_id)?;
        }
        let t = &self.thresholds;
        if t.quality_limit_mg_l.get() < 0.0
            || t.stretch_limit_mg_l.get() < 0.0
            || t.stretch_limit_mg_l > t.quality_limit_mg_l
            || t.sandbox_pressure_ratio_max.get() <= 0.0
            || t.sandbox_pdr_min.get() <= 0.0
        {
            return Err("invalid_requirement_threshold".into());
        }
        for limit in [
            t.pressure_budget_pa,
            t.energy_budget_j_per_treated_m3,
            t.absolute_pressure_min_pa,
        ]
        .into_iter()
        .flatten()
        {
            if limit.get() <= 0.0 {
                return Err("invalid_physical_limit".into());
            }
        }
        if let Some([low, high]) = t.duty_band_m3_h {
            if low.get() <= 0.0 || low > high {
                return Err("invalid_duty_band".into());
            }
        }
        if self.admission == AdmissionState::Admitted
            && (!self.blockers.is_empty()
                || t.pressure_budget_pa.is_none()
                || t.energy_budget_j_per_treated_m3.is_none()
                || t.absolute_pressure_min_pa.is_none()
                || t.duty_band_m3_h.is_none())
        {
            return Err("cannot_admit_unresolved_requirements".into());
        }
        Ok(())
    }

    pub fn objective_hash(&self) -> Result<String, String> {
        self.check()?;
        scientific_hash(&self.objectives)
    }

    /// The runtime consumes this exact specification. Missing or nonfinite observables error.
    pub fn evaluate_objectives(
        &self,
        values: &BTreeMap<String, Finite>,
    ) -> Result<Vec<Finite>, String> {
        self.check()?;
        self.objectives
            .iter()
            .map(|spec| {
                let raw = values
                    .get(&spec.source_id)
                    .ok_or_else(|| format!("missing_objective:{}", spec.source_id))?
                    .get();
                let transformed = match spec.transform {
                    Transform::Identity => raw,
                    Transform::OneMinus => 1.0 - raw,
                    Transform::Negate => -raw,
                };
                Finite::new(if spec.sense == Sense::Maximize {
                    -transformed
                } else {
                    transformed
                })
            })
            .collect()
    }

    pub fn verify_executed_objectives(
        &self,
        executed: &[ObjectiveSpec],
        receipt_hash: &str,
    ) -> Result<(), String> {
        if executed != self.objectives || self.objective_hash()? != receipt_hash {
            return Err("preregistered_executed_objective_mismatch".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateReceipt {
    pub id: String,
    pub evidence: ContentRef,
}

/// Application-supplied resolver must use the trusted evidence registry; submitted booleans
/// are not evidence. The full mandatory inventory must be present even if it is unresolved.
pub fn admit_campaign(
    mode: CampaignMode,
    requirements: &Requirements,
    executed: &[ObjectiveSpec],
    objective_hash: &str,
    gates: &[GateReceipt],
    resolve: impl Fn(&ContentRef) -> Verdict,
) -> Result<(), String> {
    requirements.check()?;
    requirements.verify_executed_objectives(executed, objective_hash)?;
    if mode == CampaignMode::Sandbox {
        return if matches!(
            requirements.admission,
            AdmissionState::SandboxOnly | AdmissionState::Admitted
        ) {
            Ok(())
        } else {
            Err("campaign_inactive".into())
        };
    }
    if requirements.admission != AdmissionState::Admitted {
        return Err("claim_bearing_requires_admitted_requirements".into());
    }
    let ids: BTreeSet<_> = gates.iter().map(|g| g.id.as_str()).collect();
    if gates.len() != ids.len() || ids != REQUIRED_P0_GATES.iter().copied().collect() {
        return Err("mandatory_p0_inventory_incomplete".into());
    }
    for gate in gates {
        gate.evidence.check()?;
        if resolve(&gate.evidence) != Verdict::Pass {
            return Err(format!("p0_gate_open:{}", gate.id));
        }
    }
    Ok(())
}
