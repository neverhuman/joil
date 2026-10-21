//! Treated capacity at a fixed pressure budget, reject budget and quality limit.
//!
//! `Q*` is the largest *tested* feed flow at which the plant meets the quality limit with
//! a feed pressure inside the budget (SCIENCE.md: no interpolation across failed or
//! untested gaps). The pressure-limited flow `Q_p` is found by bisection on the cheap
//! hydraulic solve; quality is then probed at `Q_p ρ^j`. Treated capacity is
//! `Q* (1 - r_total)`. When no tested point complies, capacity is zero and a shortfall
//! (log ratio to the limit) is reported so that search can still rank near-misses.

use crate::fluid::FeedState;
use crate::network::{evaluate, feed_pressure, PlantEvaluation, Settings};
use crate::plant::Plant;
use crate::{JudgeError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub pressure_budget_pa: f64,
    pub reject_budget: f64,
    pub quality_limit_mg_l: f64,
    pub max_amplification: f64,
    pub q_min_m3_h: f64,
    pub q_max_m3_h: f64,
    pub quality_probe_ratio: f64,
    pub quality_probes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            pressure_budget_pa: 400_000.0,
            reject_budget: 0.02,
            quality_limit_mg_l: 30.0,
            max_amplification: 8.0,
            q_min_m3_h: 0.5,
            q_max_m3_h: 40.0,
            quality_probe_ratio: 0.85,
            quality_probes: 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Probe {
    pub flow_m3_h: f64,
    pub cout_mg_l: Option<f64>,
    /// Budget-relevant pressure (feed pressure plus pump work per feed volume).
    pub feed_pressure_pa: Option<f64>,
    pub compliant: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Capacity {
    pub q_pressure_limit_m3_h: Option<f64>,
    pub q_star_m3_h: Option<f64>,
    pub q_treated_m3_h: f64,
    pub reject_fraction: f64,
    pub amplification: f64,
    /// `max(0, min ln(Cu/limit))` over probes, or the pressure shortfall at `q_min`.
    pub shortfall: f64,
    pub probes: Vec<Probe>,
    /// Evaluation at `Q*`, or at the cleanest probe when none complies.
    pub reference: Option<PlantEvaluation>,
}

pub fn capacity(plant: &Plant, feed: &FeedState, s: &Settings, lim: &Limits) -> Result<Capacity> {
    let p = |q_h: f64| feed_pressure(plant, q_h / 3600.0, s);
    let flows = crate::flows::solve(plant, lim.q_min_m3_h / 3600.0)?;
    let reject_fraction = flows.q_reject / flows.q_feed;
    let amplification = flows.amplification;
    let p_lo = p(lim.q_min_m3_h)?;
    let mut out = Capacity {
        q_pressure_limit_m3_h: None,
        q_star_m3_h: None,
        q_treated_m3_h: 0.0,
        reject_fraction,
        amplification,
        shortfall: 0.0,
        probes: Vec::new(),
        reference: None,
    };
    if p_lo > lim.pressure_budget_pa {
        out.shortfall = (p_lo / lim.pressure_budget_pa).ln() + 1.0;
        return Ok(out);
    }
    let q_p = if p(lim.q_max_m3_h)? <= lim.pressure_budget_pa {
        lim.q_max_m3_h
    } else {
        let (mut lo, mut hi) = (lim.q_min_m3_h.ln(), lim.q_max_m3_h.ln());
        for _ in 0..18 {
            let mid = 0.5 * (lo + hi);
            if p(mid.exp())? <= lim.pressure_budget_pa {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo.exp()
    };
    out.q_pressure_limit_m3_h = Some(q_p);
    let mut best_ln = f64::INFINITY;
    let mut numerical = 0usize;
    let mut worsening = 0usize;
    let mut last_cout = f64::INFINITY;
    for j in 0..lim.quality_probes.max(1) {
        let q = q_p * lim.quality_probe_ratio.powi(j as i32);
        if q < lim.q_min_m3_h * (1.0 - 1e-12) {
            break;
        }
        match evaluate(plant, &feed.with_flow(q), s) {
            Ok(ev) => {
                let ok = ev.cout_mg_l <= lim.quality_limit_mg_l
                    && ev.equivalent_pressure_pa <= lim.pressure_budget_pa * (1.0 + 1e-9);
                let ln = (ev.cout_mg_l.max(1e-12) / lim.quality_limit_mg_l).ln();
                out.probes.push(Probe {
                    flow_m3_h: q,
                    cout_mg_l: Some(ev.cout_mg_l),
                    feed_pressure_pa: Some(ev.equivalent_pressure_pa),
                    compliant: ok,
                    error: None,
                });
                if ok {
                    out.q_star_m3_h = Some(q);
                    out.q_treated_m3_h = q * (1.0 - reject_fraction);
                    out.shortfall = 0.0;
                    out.reference = Some(ev);
                    return Ok(out);
                }
                // Lower flow rarely helps a swirl separator unless breakup dominates; stop
                // after two consecutive worsening probes.
                worsening = if ev.cout_mg_l > last_cout {
                    worsening + 1
                } else {
                    0
                };
                last_cout = ev.cout_mg_l;
                if ln < best_ln {
                    best_ln = ln;
                    out.reference = Some(ev);
                }
                if worsening >= 2 {
                    break;
                }
            }
            Err(e) => {
                if matches!(e, JudgeError::Numerical(_)) {
                    numerical += 1;
                } else if out.probes.is_empty() {
                    return Err(e);
                }
                out.probes.push(Probe {
                    flow_m3_h: q,
                    cout_mg_l: None,
                    feed_pressure_pa: None,
                    compliant: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }
    if out.reference.is_none() && numerical > 0 {
        return Err(JudgeError::Numerical(
            "every quality probe failed numerically".into(),
        ));
    }
    out.shortfall = best_ln.max(0.0);
    Ok(out)
}
