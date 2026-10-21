//! Printed lattice coalescer bed (capture on struts, film drainage, release as large drops).
//!
//! * Pressure drop: Ergun with equivalent particle diameter `1.5 d_f` for struts `d_f`.
//! * Capture: fibrous-bed theory with Kuwabara flow, interception
//!   `η_R = (1-α)/Ku · R²/(1+R)` (`R = d/d_f`) plus inertial impaction
//!   `η_I = St²/(St+0.25)²`, bed efficiency `E = 1 - exp(-4 α η L / (π (1-α) d_f))`.
//! * Captured oil drains as drops of `c_rel d_f`, limited by the bed's own
//!   maximum stable diameter. Incoming drops above that limit break before capture.
//!
//! Steady state is assumed (no saturation, fouling or wettability drift). Struts below the
//! printable minimum are rejected.

use crate::breakup::{breakup_transfer, max_stable_diameter};
use crate::closure::Closure;
use crate::cyclone::MIN_FEATURE_M;
use crate::fluid::Fluids;
use crate::sizes::SizeGrid;
use crate::transfer::Transfer;
use crate::{JudgeError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoalescerSpec {
    pub cells: u32,
    pub diameter_m: f64,
    pub length_m: f64,
    pub porosity: f64,
    pub strut_m: f64,
}

impl CoalescerSpec {
    pub fn validate(&self) -> Result<()> {
        let ok = (1..=64).contains(&self.cells)
            && self.diameter_m >= 5.0 * MIN_FEATURE_M
            && self.length_m > 0.0
            && (0.4..=0.97).contains(&self.porosity)
            && self.strut_m >= MIN_FEATURE_M
            && self.diameter_m > 3.0 * self.strut_m;
        if ok {
            Ok(())
        } else {
            Err(JudgeError::Geometry(
                "coalescer bed outside printable bounds (strut >= 1 mm, porosity 0.4..0.97)".into(),
            ))
        }
    }

    pub fn cell_volume(&self) -> f64 {
        0.25 * std::f64::consts::PI * self.diameter_m * self.diameter_m * self.length_m
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoalescerResponse {
    pub dp_pa: f64,
    pub superficial_velocity_m_s: f64,
    pub eps_w_kg: f64,
    pub release_d_m: f64,
    pub capture: Vec<f64>,
    pub transfer: Transfer,
}

pub fn respond(
    spec: &CoalescerSpec,
    q_cell: f64,
    grid: &SizeGrid,
    fluids: &Fluids,
    closure: &Closure,
) -> Result<CoalescerResponse> {
    spec.validate()?;
    if q_cell.is_nan() || q_cell <= 0.0 {
        return Err(JudgeError::Input("coalescer flow must be positive".into()));
    }
    let rho = fluids.rho_water_kg_m3;
    let mu = fluids.mu_water_pa_s;
    let area = 0.25 * std::f64::consts::PI * spec.diameter_m * spec.diameter_m;
    let u = q_cell / area;
    let eps_p = spec.porosity;
    let alpha = 1.0 - eps_p;
    let dp_eq = 1.5 * spec.strut_m;
    let dp = spec.length_m
        * (150.0 * mu * alpha * alpha * u / (eps_p.powi(3) * dp_eq * dp_eq)
            + 1.75 * rho * alpha * u * u / (eps_p.powi(3) * dp_eq));
    let eps_bed = dp * u / (rho * eps_p * spec.length_m);
    let d_max = max_stable_diameter(eps_bed, fluids, closure.davies_c);
    let pre = breakup_transfer(grid, d_max, closure.satellite_fraction);
    let ku = -0.5 * alpha.ln() - 0.75 + alpha - 0.25 * alpha * alpha;
    let u_int = u / eps_p;
    let capture: Vec<f64> = grid
        .d_m
        .iter()
        .map(|d| {
            let r = d / spec.strut_m;
            let eta_r = (1.0 - alpha) / ku.max(1e-3) * r * r / (1.0 + r);
            let st = fluids.rho_oil_kg_m3 * d * d * u_int / (18.0 * mu * spec.strut_m);
            let eta_i = st * st / ((st + 0.25) * (st + 0.25));
            let eta = ((eta_r + eta_i) * closure.coalescer_capture_mult).min(1.0);
            let expo = 4.0 * alpha * eta * spec.length_m
                / (std::f64::consts::PI * (1.0 - alpha) * spec.strut_m);
            -(-expo).exp_m1()
        })
        .collect();
    let release_d = (closure.coalescer_release_factor * spec.strut_m).min(d_max);
    let release_class = grid.class_of(release_d);
    let cols = (0..grid.n())
        .map(|i| {
            let e = capture[i];
            if e <= 0.0 || release_class == i {
                vec![(i, 1.0)]
            } else {
                vec![(i, 1.0 - e), (release_class, e)]
            }
        })
        .collect();
    let transfer = pre.then(&Transfer { cols });
    Ok(CoalescerResponse {
        dp_pa: dp,
        superficial_velocity_m_s: u,
        eps_w_kg: eps_bed,
        release_d_m: release_d,
        capture,
        transfer,
    })
}
