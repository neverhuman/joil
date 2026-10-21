//! Minimum feed pressure, throttles and pump lifts of a plant.
//!
//! Each active unit needs an inlet pressure high enough for every outlet stream to reach
//! its destination: `p_in(u) = max_k [Δp_k(u) + Δp_link + p_req(dest_k)]`. Unpumped links
//! propagate the destination requirement; a pumped link only needs its suction pressure.
//! The fixed point is found by Bellman–Ford relaxation. An unpumped cycle whose losses are
//! positive never converges and is reported as a passive loop. Surplus pressure on a stream
//! is dissipated in a throttle at the stream's arrival, which [`crate::network`] charges as
//! a high-shear breakup location.

use crate::flows::Flows;
use crate::plant::{Dest, Plant};
use crate::{JudgeError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terminals {
    pub product_pa: f64,
    pub reject_pa: f64,
    /// Minimum gauge pressure at a pump suction.
    pub pump_suction_min_pa: f64,
    pub pump_efficiency: f64,
}

impl Default for Terminals {
    fn default() -> Self {
        Self {
            product_pa: 0.0,
            reject_pa: 0.0,
            pump_suction_min_pa: 20_000.0,
            pump_efficiency: 0.6,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PressureSolution {
    /// Required gauge pressure at each unit inlet (Pa); 0 for inactive units.
    pub unit_inlet_pa: Vec<f64>,
    /// Minimum feed gauge pressure (Pa).
    pub feed_pa: f64,
    /// Pressure dissipated in a throttle on each outlet link (Pa).
    pub throttle_pa: Vec<Vec<f64>>,
    /// Pressure added by a pump on each outlet link (Pa).
    pub lift_pa: Vec<Vec<f64>>,
    pub pump_power_w: f64,
}

/// `port_drop[u][k]`: unit pressure drop to outlet `k`; `link_loss[u][k]`: channel loss.
pub fn solve(
    plant: &Plant,
    flows: &Flows,
    port_drop: &[Vec<f64>],
    link_loss: &[Vec<f64>],
    feed_loss: f64,
    terms: &Terminals,
) -> Result<PressureSolution> {
    let n = plant.units.len();
    let target = |dest: Dest, req: &[f64]| -> f64 {
        match dest {
            Dest::Unit(v) => req[v],
            Dest::Product => terms.product_pa,
            Dest::Reject => terms.reject_pa,
        }
    };
    let mut req = vec![0.0_f64; n];
    let mut converged = false;
    for _ in 0..=n + 1 {
        let mut changed = false;
        for u in 0..n {
            if !flows.active[u] {
                continue;
            }
            let mut need: f64 = 0.0;
            for (k, l) in plant.units[u].outlets.iter().enumerate() {
                if flows.link[u][k] <= 0.0 {
                    continue;
                }
                let downstream = if l.pumped {
                    terms.pump_suction_min_pa
                } else {
                    target(l.to, &req)
                };
                need = need.max(port_drop[u][k] + link_loss[u][k] + downstream);
            }
            if need > req[u] + 1e-9 * need.abs().max(1.0) {
                req[u] = need;
                changed = true;
            }
        }
        if !changed {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(JudgeError::PassiveLoop(
            "an unpumped recycle needs pressure gain; add a pump or reroute".into(),
        ));
    }
    let feed_pa = feed_loss + target(plant.feed.to, &req);
    let mut throttle = Vec::with_capacity(n);
    let mut lift = Vec::with_capacity(n);
    let mut power = 0.0;
    for u in 0..n {
        let mut t_row = vec![0.0; plant.units[u].outlets.len()];
        let mut l_row = vec![0.0; plant.units[u].outlets.len()];
        if flows.active[u] {
            for (k, l) in plant.units[u].outlets.iter().enumerate() {
                let arrival = req[u] - port_drop[u][k] - link_loss[u][k];
                let dest = target(l.to, &req);
                if l.pumped {
                    l_row[k] = (dest - arrival).max(0.0);
                    t_row[k] = (arrival - dest).max(0.0);
                    power += l_row[k] * flows.link[u][k] / terms.pump_efficiency;
                } else {
                    t_row[k] = (arrival - dest).max(0.0);
                }
            }
        }
        throttle.push(t_row);
        lift.push(l_row);
    }
    if !feed_pa.is_finite() || !power.is_finite() {
        return Err(JudgeError::Numerical("pressure solution not finite".into()));
    }
    Ok(PressureSolution {
        unit_inlet_pa: req,
        feed_pa,
        throttle_pa: throttle,
        lift_pa: lift,
        pump_power_w: power,
    })
}
