//! V5-13-05: coupon plan and as-built (CT) metrology budgets.
//!
//! The coupon plan is preparation only: it lists the records that must exist and who owns
//! each external dependency; it places no order and contacts no vendor. A CT voxel pitch is
//! a sampling interval, not a dimensional accuracy: accuracy needs a calibration artifact,
//! edge localization and repeatability, each declared, combined in quadrature with the
//! voxel quantization term.
use crate::{invalid, Field, JudgeError, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouponKind {
    Tensile,
    Fatigue,
    Density,
    Porosity,
    Roughness,
    CtCalibration,
}

pub const ALL_COUPONS: [CouponKind; 6] = [
    CouponKind::Tensile,
    CouponKind::Fatigue,
    CouponKind::Density,
    CouponKind::Porosity,
    CouponKind::Roughness,
    CouponKind::CtCalibration,
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalDependency {
    pub owner: String,
    pub description: String,
    pub blocks: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouponRecord {
    pub kind: CouponKind,
    pub purpose: String,
    pub process_condition: String,
    /// A testable rule of the form "measured X compared with threshold Y"; the threshold is a field.
    pub acceptance_rule: String,
    pub acceptance_threshold: Field<String>,
    pub minimum_specimens: Field<u32>,
    pub uncertainty_components: Vec<String>,
    pub external_dependency: ExternalDependency,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouponPlan {
    pub schema: String,
    pub passport_id: String,
    /// Preparation only. Both must be false in a plan: nothing is ordered or contacted.
    pub orders_placed: bool,
    pub vendor_contacted: bool,
    pub records: Vec<CouponRecord>,
}

impl CouponPlan {
    pub fn from_json(bytes: &[u8]) -> Result<Self, JudgeError> {
        let plan: Self = serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
        plan.check()?;
        Ok(plan)
    }

    pub fn check(&self) -> Result<(), JudgeError> {
        if self.schema != "joil.hydrocyclone.coupon-plan.v1" || self.passport_id.trim().is_empty() {
            return Err(invalid("unknown coupon plan schema or empty passport id"));
        }
        if self.orders_placed || self.vendor_contacted {
            return Err(invalid(
                "a preparation plan must not record orders or vendor contact",
            ));
        }
        let kinds: BTreeSet<_> = self.records.iter().map(|r| r.kind).collect();
        if kinds.len() != self.records.len() || kinds != ALL_COUPONS.iter().copied().collect() {
            return Err(invalid(
                "plan must hold exactly one record for each required coupon kind",
            ));
        }
        for r in &self.records {
            let text = [
                &r.purpose,
                &r.process_condition,
                &r.acceptance_rule,
                &r.external_dependency.owner,
                &r.external_dependency.description,
            ];
            if text.iter().any(|t| t.trim().is_empty())
                || r.uncertainty_components.is_empty()
                || r.uncertainty_components.iter().any(|c| c.trim().is_empty())
                || r.external_dependency.blocks.is_empty()
            {
                return Err(invalid("each coupon record needs purpose, process, rule, uncertainty components and an owned dependency"));
            }
        }
        Ok(())
    }
}

/// Declared CT uncertainty components in micrometres. `None` = not declared (unknown, not zero).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CtBudget {
    pub voxel_pitch_um: f64,
    pub calibration_artifact_uncertainty_um: Option<f64>,
    pub edge_localization_um: Option<f64>,
    pub repeatability_um: Option<f64>,
    pub calibration_artifact_source: Option<Sha256Hex>,
}

impl CtBudget {
    /// Expanded dimensional accuracy `k * sqrt(sum u^2)` in micrometres, or the missing components.
    pub fn dimensional_accuracy_um(&self, coverage_k: f64) -> Result<f64, Vec<String>> {
        let mut missing = Vec::new();
        if !(self.voxel_pitch_um.is_finite() && self.voxel_pitch_um > 0.0) {
            missing.push("voxel_pitch:invalid".to_string());
        }
        if !(coverage_k.is_finite() && coverage_k >= 1.0) {
            missing.push("coverage_factor:invalid".to_string());
        }
        for (name, v) in [
            (
                "calibration_artifact_uncertainty",
                self.calibration_artifact_uncertainty_um,
            ),
            ("edge_localization", self.edge_localization_um),
            ("repeatability", self.repeatability_um),
        ] {
            match v {
                Some(x) if x.is_finite() && x >= 0.0 => {}
                _ => missing.push(format!("{name}:not_declared")),
            }
        }
        if self.calibration_artifact_source.is_none() {
            missing.push("calibration_artifact_source:not_declared".into());
        }
        if !missing.is_empty() {
            return Err(missing);
        }
        let quantization = self.voxel_pitch_um / 12f64.sqrt();
        let squares = quantization.powi(2)
            + self
                .calibration_artifact_uncertainty_um
                .unwrap_or(0.0)
                .powi(2)
            + self.edge_localization_um.unwrap_or(0.0).powi(2)
            + self.repeatability_um.unwrap_or(0.0).powi(2);
        Ok(coverage_k * squares.sqrt())
    }
}

/// Does the budget meet an admitted dimensional-accuracy requirement? Pitch alone cannot.
pub fn verify_dimensional_accuracy(
    budget: &CtBudget,
    coverage_k: f64,
    required_um: &Field<f64>,
) -> Verdict {
    let Some(required) = required_um
        .admitted()
        .filter(|v| v.is_finite() && **v > 0.0)
    else {
        return Verdict::Unresolved(vec!["dimensional_accuracy_requirement:not_admitted".into()]);
    };
    match budget.dimensional_accuracy_um(coverage_k) {
        Err(missing) => Verdict::Unresolved(missing),
        Ok(accuracy) if accuracy <= *required => Verdict::Pass,
        Ok(accuracy) => Verdict::Fail(vec![format!(
            "dimensional_accuracy:{accuracy:.3}um_exceeds_{required:.3}um"
        )]),
    }
}

/// A wall is detectable only if it spans enough voxels AND exceeds the expanded accuracy.
pub fn verify_wall_detectable(
    budget: &CtBudget,
    coverage_k: f64,
    wall_um: f64,
    min_voxels_across: &Field<f64>,
) -> Verdict {
    let Some(min_voxels) = min_voxels_across
        .admitted()
        .filter(|v| v.is_finite() && **v >= 1.0)
    else {
        return Verdict::Unresolved(vec!["wall_detectability:min_voxels_not_admitted".into()]);
    };
    if !(wall_um.is_finite() && wall_um > 0.0) {
        return Verdict::Fail(vec!["wall_detectability:invalid_wall".into()]);
    }
    let accuracy = match budget.dimensional_accuracy_um(coverage_k) {
        Ok(a) => a,
        Err(missing) => return Verdict::Unresolved(missing),
    };
    let mut reasons = Vec::new();
    if wall_um < min_voxels * budget.voxel_pitch_um {
        reasons.push(format!(
            "wall_detectability:spans_fewer_than_{min_voxels}_voxels"
        ));
    }
    if wall_um <= accuracy {
        reasons.push("wall_detectability:wall_not_larger_than_expanded_accuracy".into());
    }
    if reasons.is_empty() {
        Verdict::Pass
    } else {
        Verdict::Fail(reasons)
    }
}
