//! Report records of a plant evaluation.

use crate::cyclone::PressureBudget;
use crate::fluid::FeedState;
use crate::plant::Dest;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnitReport {
    pub index: usize,
    pub kind: String,
    pub cells: u32,
    pub q_in_m3_s: f64,
    pub inlet_pa: f64,
    pub port_drop_pa: Vec<f64>,
    pub eps_peak_w_kg: f64,
    pub d_max_um: f64,
    pub residence_s: f64,
    pub inlet_velocity_m_s: f64,
    pub peak_wall_g: f64,
    /// Overflow capture per size class (cyclones) or bed capture (coalescers).
    pub capture: Vec<f64>,
    pub reduced_d50_um: Option<f64>,
    pub tracer_error: f64,
    pub budget: Option<PressureBudget>,
    pub oil_in_kg_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinkReport {
    pub from: Option<usize>,
    pub port: usize,
    pub to: Dest,
    pub flow_m3_s: f64,
    pub velocity_m_s: f64,
    pub channel_loss_pa: f64,
    pub throttle_pa: f64,
    pub lift_pa: f64,
    pub d_max_um: f64,
    pub oil_kg_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlantEvaluation {
    pub model_version: String,
    pub evidence_class: String,
    pub feed: FeedState,
    pub q_product_m3_s: f64,
    pub q_reject_m3_s: f64,
    pub reject_fraction: f64,
    pub cout_mg_l: f64,
    pub oil_capture: f64,
    pub feed_pressure_pa: f64,
    /// Feed pressure plus pump shaft work per unit feed volume. Budgets apply to this, so
    /// an inter-stage booster cannot buy an extra pressure allowance.
    pub equivalent_pressure_pa: f64,
    pub pump_power_w: f64,
    pub specific_energy_j_m3: f64,
    pub amplification: f64,
    pub oil_balance_rel_error: f64,
    pub oil_iterations: usize,
    pub min_d_max_um: f64,
    pub feed_dsd: Vec<f64>,
    pub product_dsd: Vec<f64>,
    pub reject_dsd: Vec<f64>,
    pub units: Vec<UnitReport>,
    pub links: Vec<LinkReport>,
}
