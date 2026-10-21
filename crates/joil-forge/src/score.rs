//! Scoring: hard gates, conservative capacity across closure samples, and the weighted
//! objective. Feasible means: geometry gates pass and the plant meets the quality limit
//! inside the pressure and reject budgets for *every* sampled closure.

use crate::campaign::Campaign;
use crate::develop::{develop, Behavior, Phenotype};
use crate::genome::Genome;
use crate::rng::latin_hypercube;
use joil_forge_judges::capacity::{capacity, Capacity};
use joil_forge_judges::closure::Closure;
use joil_forge_judges::cyclone::Resolution;
use joil_forge_judges::network::{evaluate, Settings};
use joil_forge_judges::sizes::SizeGrid;
use joil_forge_judges::JudgeError;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub feasible: bool,
    /// Weighted objective; meaningful only when `feasible` (0 otherwise).
    pub fitness: f64,
    pub violation: f64,
    /// Treated capacity, minimum over closure samples (m³/h).
    pub q_treated_m3_h: f64,
    pub q_treated_nominal_m3_h: f64,
    pub q_feed_m3_h: f64,
    /// Outlet concentration at `Q*`, or at the cleanest probe when none complies.
    pub cout_mg_l: Option<f64>,
    /// Feed pressure plus pump work per feed volume, the budget-relevant pressure.
    pub feed_pressure_kpa: Option<f64>,
    pub specific_energy_kj_m3: Option<f64>,
    pub shell_mass_kg: f64,
    pub reject_fraction: Option<f64>,
    /// Worst feed-scenario margin `min(1, limit / C_out)` at the rated flow (1 = compliant).
    pub robustness: f64,
    pub units: usize,
    pub per_closure_q_m3_h: Vec<f64>,
    pub reasons: Vec<String>,
    pub numerical_failure: bool,
}

impl Score {
    fn failed(violation: f64, reason: String, units: usize) -> Self {
        Self {
            feasible: false,
            fitness: 0.0,
            violation,
            q_treated_m3_h: 0.0,
            q_treated_nominal_m3_h: 0.0,
            q_feed_m3_h: 0.0,
            cout_mg_l: None,
            feed_pressure_kpa: None,
            specific_energy_kj_m3: None,
            shell_mass_kg: 0.0,
            reject_fraction: None,
            robustness: 0.0,
            units,
            per_closure_q_m3_h: Vec::new(),
            reasons: vec![reason],
            numerical_failure: false,
        }
    }

    /// Ranking: feasible before infeasible; then higher fitness, or lower violation.
    pub fn compare(&self, other: &Score) -> Ordering {
        match (self.feasible, other.feasible) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (true, true) => other.fitness.total_cmp(&self.fitness),
            (false, false) => self.violation.total_cmp(&other.violation),
        }
    }
}

/// Evaluation context shared by every candidate of a run (common closure samples).
pub struct Evaluator {
    pub campaign: Campaign,
    pub closures: Vec<Closure>,
    grid: SizeGrid,
}

impl Evaluator {
    pub fn new(c: &Campaign) -> Self {
        let mut closures = vec![Closure::nominal()];
        for u in latin_hypercube::<14>(c.closure_samples, c.seed) {
            closures.push(Closure::from_unit(&u));
        }
        Self {
            campaign: c.clone(),
            closures,
            grid: SizeGrid::standard(),
        }
    }

    pub fn settings(&self, k: usize, res: Resolution) -> Settings {
        Settings {
            fluids: self.campaign.fluids,
            grid: self.grid.clone(),
            closure: self.closures[k],
            resolution: res,
            terminals: self.campaign.terminals,
        }
    }

    pub fn evaluate(&self, g: &Genome) -> (Score, Option<Behavior>) {
        let (s, p, _) = self.evaluate_full(g, self.campaign.resolution);
        (s, p.map(|p| p.behavior))
    }

    /// Score plus phenotype and the nominal-closure capacity record (for reports).
    pub fn evaluate_full(
        &self,
        g: &Genome,
        res: Resolution,
    ) -> (Score, Option<Phenotype>, Option<Capacity>) {
        let c = &self.campaign;
        let units = g.units.len();
        let ph = match develop(g, &c.envelope) {
            Ok(p) => p,
            Err(e) => {
                return (
                    Score::failed(100.0, format!("development: {e}"), units),
                    None,
                    None,
                )
            }
        };
        let mut violation = 0.0;
        let mut reasons = Vec::new();
        if !ph.violations.is_empty() {
            violation += 10.0 + 2.0 * ph.violations.len() as f64;
            reasons.extend(ph.violations.iter().map(|v| format!("geometry: {v}")));
            if ph.layout.length_mm > c.envelope.length_mm {
                violation += 5.0 * (ph.layout.length_mm / c.envelope.length_mm - 1.0);
            }
        }
        let pumped = ph
            .plant
            .units
            .iter()
            .flat_map(|u| u.outlets.iter())
            .any(|l| l.pumped);
        if pumped && !c.allow_pumps {
            violation += 10.0;
            reasons.push("policy: recycle pumps are not allowed in this campaign".into());
        }
        let nominal = match capacity(&ph.plant, &c.feed, &self.settings(0, res), &c.limits) {
            Ok(cap) => cap,
            Err(e) => {
                let mut s = Score::failed(
                    violation
                        + if matches!(e, JudgeError::Numerical(_)) {
                            30.0
                        } else {
                            20.0
                        },
                    format!("physics: {e}"),
                    units,
                );
                s.numerical_failure = matches!(e, JudgeError::Numerical(_));
                s.shell_mass_kg = ph.stats.shell_mass_kg;
                s.reasons.extend(reasons);
                return (s, Some(ph), None);
            }
        };
        if nominal.reject_fraction > c.limits.reject_budget * (1.0 + 1e-9) {
            violation += 5.0 + 10.0 * (nominal.reject_fraction / c.limits.reject_budget - 1.0);
            reasons.push(format!(
                "reject fraction {:.4} exceeds budget {:.4}",
                nominal.reject_fraction, c.limits.reject_budget
            ));
        }
        if nominal.amplification > c.limits.max_amplification {
            violation += 5.0 + nominal.amplification / c.limits.max_amplification - 1.0;
            reasons.push(format!(
                "recycle amplification {:.2} exceeds {:.2}",
                nominal.amplification, c.limits.max_amplification
            ));
        }
        violation += nominal.shortfall;
        if nominal.q_star_m3_h.is_none() {
            reasons.push(format!(
                "no compliant tested flow (shortfall {:.3})",
                nominal.shortfall
            ));
        }
        let reference = nominal.reference.as_ref();
        let mut score = Score {
            feasible: false,
            fitness: 0.0,
            violation,
            q_treated_m3_h: 0.0,
            q_treated_nominal_m3_h: nominal.q_treated_m3_h,
            q_feed_m3_h: nominal.q_star_m3_h.unwrap_or(0.0),
            cout_mg_l: reference.map(|r| r.cout_mg_l),
            feed_pressure_kpa: reference.map(|r| r.equivalent_pressure_pa / 1e3),
            specific_energy_kj_m3: reference.map(|r| r.specific_energy_j_m3 / 1e3),
            shell_mass_kg: ph.stats.shell_mass_kg,
            reject_fraction: Some(nominal.reject_fraction),
            robustness: 0.0,
            units,
            per_closure_q_m3_h: vec![nominal.q_treated_m3_h],
            reasons,
            numerical_failure: false,
        };
        if violation > 0.0 || nominal.q_star_m3_h.is_none() {
            return (score, Some(ph), Some(nominal));
        }
        // Robust tier: every closure sample must comply.
        let mut q_min = nominal.q_treated_m3_h;
        let mut robust_shortfall = 0.0;
        for k in 1..self.closures.len() {
            match capacity(&ph.plant, &c.feed, &self.settings(k, res), &c.limits) {
                Ok(cap) => {
                    score.per_closure_q_m3_h.push(cap.q_treated_m3_h);
                    q_min = q_min.min(cap.q_treated_m3_h);
                    robust_shortfall += cap.shortfall;
                }
                Err(e) => {
                    score.per_closure_q_m3_h.push(0.0);
                    q_min = 0.0;
                    robust_shortfall += 1.0;
                    score.numerical_failure |= matches!(e, JudgeError::Numerical(_));
                    score.reasons.push(format!("closure {k}: {e}"));
                }
            }
        }
        // Discretisation guard: the nominal closure again at the finer report resolution.
        if q_min > 0.0 && res != Resolution::report() {
            match capacity(
                &ph.plant,
                &c.feed,
                &self.settings(0, Resolution::report()),
                &c.limits,
            ) {
                Ok(cap) => {
                    q_min = q_min.min(cap.q_treated_m3_h);
                    robust_shortfall += cap.shortfall;
                }
                Err(e) => {
                    q_min = 0.0;
                    score.reasons.push(format!("refined resolution: {e}"));
                }
            }
        }
        if q_min <= 0.0 {
            score.violation = robust_shortfall.max(1e-3);
            score
                .reasons
                .push("not compliant for every closure sample".into());
            return (score, Some(ph), Some(nominal));
        }
        // Off-design feeds at the rated flow. Continuous: a scenario that misses the limit by
        // a factor k keeps 1/k of the capacity credit, so there is no cliff at the limit.
        let q_rated = nominal.q_star_m3_h.unwrap_or(0.0);
        let mut worst = 1.0_f64;
        for sc in &c.scenarios {
            let margin = evaluate(&ph.plant, &sc.with_flow(q_rated), &self.settings(0, res))
                .map(|ev| (c.limits.quality_limit_mg_l / ev.cout_mg_l.max(1e-9)).min(1.0))
                .unwrap_or(0.0);
            worst = worst.min(margin);
        }
        let w = &c.objective;
        let energy = score.specific_energy_kj_m3.unwrap_or(1e3).max(1e-6) * 1e3;
        score.feasible = true;
        score.violation = 0.0;
        score.q_treated_m3_h = q_min;
        score.robustness = worst;
        score.fitness = w.capacity * q_min.ln()
            - w.energy * (energy / 1e5).ln()
            - w.mass * score.shell_mass_kg.max(1e-3).ln()
            + w.robustness * worst.max(1e-3).ln()
            - w.per_unit * units as f64;
        (score, Some(ph), Some(nominal))
    }
}
