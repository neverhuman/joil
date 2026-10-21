//! Fluid properties and feed states (SI units throughout).

use crate::{JudgeError, Result};
use serde::{Deserialize, Serialize};

/// Carrier water and dispersed oil properties.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fluids {
    pub rho_water_kg_m3: f64,
    pub mu_water_pa_s: f64,
    pub rho_oil_kg_m3: f64,
    pub mu_oil_pa_s: f64,
    pub interfacial_tension_n_m: f64,
}

impl Fluids {
    /// Provisional reference properties of `requirements/RS-joil-research-v1.json`.
    pub fn reference() -> Self {
        Self {
            rho_water_kg_m3: 998.0,
            mu_water_pa_s: 1.0e-3,
            rho_oil_kg_m3: 850.0,
            mu_oil_pa_s: 5.0e-3,
            interfacial_tension_n_m: 0.025,
        }
    }

    pub fn validate(&self) -> Result<()> {
        let ok = self.rho_water_kg_m3 > 0.0
            && self.mu_water_pa_s > 0.0
            && self.rho_oil_kg_m3 > 0.0
            && self.mu_oil_pa_s > 0.0
            && self.interfacial_tension_n_m > 0.0
            && self.rho_oil_kg_m3 < self.rho_water_kg_m3;
        if ok {
            Ok(())
        } else {
            Err(JudgeError::Input(
                "fluid properties must be positive with oil lighter than water".into(),
            ))
        }
    }

    pub fn delta_rho(&self) -> f64 {
        self.rho_water_kg_m3 - self.rho_oil_kg_m3
    }

    pub fn nu_water(&self) -> f64 {
        self.mu_water_pa_s / self.rho_water_kg_m3
    }
}

/// Oil-volume (equal density: oil-mass) basis lognormal drop size distribution.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lognormal {
    pub volume_median_um: f64,
    pub geometric_sd: f64,
}

/// A feed stream entering the plant.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedState {
    pub flow_m3_h: f64,
    pub cin_mg_l: f64,
    pub dsd: Lognormal,
}

impl FeedState {
    /// Reference duty point of `RS-joil-research-v1`.
    pub fn reference() -> Self {
        Self {
            flow_m3_h: 4.0,
            cin_mg_l: 500.0,
            dsd: Lognormal {
                volume_median_um: 15.0,
                geometric_sd: 1.6,
            },
        }
    }

    pub fn q_m3_s(&self) -> f64 {
        self.flow_m3_h / 3600.0
    }

    /// Oil mass flow in kg/s (`mg/L == g/m^3`).
    pub fn oil_kg_s(&self) -> f64 {
        self.cin_mg_l * self.q_m3_s() * 1.0e-3
    }

    pub fn with_flow(&self, flow_m3_h: f64) -> Self {
        Self { flow_m3_h, ..*self }
    }

    pub fn validate(&self) -> Result<()> {
        let ok = self.flow_m3_h.is_finite()
            && self.flow_m3_h > 0.0
            && self.cin_mg_l.is_finite()
            && self.cin_mg_l >= 0.0
            && self.dsd.volume_median_um > 0.0
            && self.dsd.geometric_sd > 1.0;
        if ok {
            Ok(())
        } else {
            Err(JudgeError::Input(
                "feed state must be finite and positive".into(),
            ))
        }
    }
}

/// Outlet concentration from raw capture and split (SCIENCE.md oracle form).
///
/// `Cu = Ci (1 - E) / (1 - r)`. Returns `None` when `r >= 1` (no treated water).
pub fn underflow_concentration(cin_mg_l: f64, capture: f64, split: f64) -> Option<f64> {
    if !(0.0..1.0).contains(&split) || !(0.0..=1.0).contains(&capture) {
        return None;
    }
    Some(cin_mg_l * (1.0 - capture) / (1.0 - split))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn science_contract_oracles() {
        let cu = underflow_concentration(500.0, 0.94, 0.02).unwrap();
        assert!((cu - 30.612_244_9).abs() < 1e-6);
        let cu = underflow_concentration(500.0, 0.95, 0.02).unwrap();
        assert!((cu - 25.510_204_1).abs() < 1e-6);
        // Passive tracer: E = r gives Cu = Ci.
        assert!((underflow_concentration(500.0, 0.02, 0.02).unwrap() - 500.0).abs() < 1e-9);
    }
}
