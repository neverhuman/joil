use crate::Sha256Hex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 1;

/// No admitted/qualified variant exists: a linear response needs independent
/// geometry, hydraulic and physical evidence before any scientific use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    SyntheticFixture,
    UnqualifiedLinearResponse,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cohort {
    /// Bulk liquid volume. Reprocessing this cohort does not create fresh feed.
    pub liquid_volume_m3: f64,
    /// Oil MASS in each diameter interval, never droplet counts or number weights.
    pub oil_mass_kg: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Destination {
    Stage { stage_id: String },
    Product,
    Reject,
    Retained,
    Wall,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub id: String,
    pub destination: Destination,
    /// Fraction of stage-inlet bulk volume assigned to this route.
    pub liquid_fraction: f64,
    /// [inlet bin][outlet bin]: fraction of INLET oil mass reaching this route
    /// and outlet size. These are unconditional, not multiplied by liquid_fraction.
    pub oil_mass_transfer: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    pub id: String,
    /// Identity of the supplied response, not a certificate of physical validity.
    pub response_source_sha256: Sha256Hex,
    pub routes: Vec<Route>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feed {
    pub stage_id: String,
    pub cohort: Cohort,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportBudget {
    pub max_steps: usize,
    pub max_stage_visits: u64,
    pub max_matrix_terms: u64,
    /// Bounds the still-circulating volume AND oil mass relative to fresh input.
    /// Reaching this threshold retains the actual tail; it never renormalizes it.
    pub relative_tail_tolerance: f64,
}

impl Default for TransportBudget {
    fn default() -> Self {
        Self {
            max_steps: 256,
            max_stage_visits: 10_000,
            max_matrix_terms: 10_000_000,
            relative_tail_tolerance: 1e-10,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkRequest {
    pub schema_version: u32,
    pub evidence_class: EvidenceClass,
    /// Shared increasing diameter edges in metres, preserving bin meaning on every edge.
    pub diameter_edges_m: Vec<f64>,
    pub stages: Vec<Stage>,
    pub feeds: Vec<Feed>,
    pub budget: TransportBudget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Termination {
    Complete,
    TailBoundReached,
    WorkBudgetExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkReport {
    pub schema_version: u32,
    pub request_sha256: Sha256Hex,
    pub evidence_class: EvidenceClass,
    /// Always unresolved for physical use; conservation alone grants no admission.
    pub physical_verdict: crate::Verdict,
    pub termination: Termination,
    pub steps: usize,
    pub stage_visits: u64,
    pub matrix_terms: u64,
    pub fresh_feed: Cohort,
    pub product: Cohort,
    pub reject: Cohort,
    pub retained: Cohort,
    pub wall: Cohort,
    pub circulating: Cohort,
    /// Includes all visits, so recycle exposure is distinguishable from fresh feed.
    pub processed_by_stage: BTreeMap<String, Cohort>,
    /// Signed, explicitly retained floating-point balance errors.
    pub liquid_balance_error_m3: f64,
    pub oil_balance_error_kg: f64,
    /// Adverse allocation of circulating, retained and wall inventory.
    /// None means no product volume has yet been observed (no finite upper bound).
    pub product_concentration_mg_l: Option<Bounds>,
    pub external_reject_fraction: Bounds,
}
