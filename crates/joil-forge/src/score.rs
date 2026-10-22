//! Scoring: hard gates, conservative capacity across closure samples, and the weighted,
//! separation-first objective. Feasible means: geometry and print gates pass and the plant
//! meets the quality limit on the rating duty inside the pressure and reject budgets for
//! *every* sampled closure. The objective then rewards the oil removed on every duty.

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
    /// Weighted mean over duties of `ln(C_in / max(C_out, floor))` at the rated flow, each
    /// duty under its worst closure sample (0 when not feasible).
    pub separation: f64,
    /// Worst-closure outlet concentration of each duty at the rated flow (mg/L).
    pub duty_cout_mg_l: Vec<Option<f64>>,
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
            separation: 0.0,
            duty_cout_mg_l: Vec::new(),
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
        for u in latin_hypercube::<14>(c.closure_samples, c.closure_seed()) {
            closures.push(Closure::from_unit(&u));
        }
        Self {
            campaign: c.clone(),
            closures,
            grid: SizeGrid::standard(),
        }
    }

    /// Physics settings of the rating duty under closure sample `k`.
    pub fn settings(&self, k: usize, res: Resolution) -> Settings {
        self.duty_settings(0, k, res)
    }

    /// Physics settings of duty `d` under closure sample `k`.
    pub fn duty_settings(&self, d: usize, k: usize, res: Resolution) -> Settings {
        Settings {
            fluids: self.campaign.duties[d].fluids,
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
        if let Err(e) = c.validate() {
            return (
                Score::failed(100.0, format!("campaign: {e}"), units),
                None,
                None,
            );
        }
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
            // Graded: how far past the build height, bore and aspect limits it lies, so
            // search can climb back inside them.
            violation += 10.0 + 2.0 * ph.violations.len() as f64 + 5.0 * ph.excess;
            reasons.extend(ph.violations.iter().map(|v| format!("geometry: {v}")));
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
        let rating = &c.rating().feed;
        let nominal = match capacity(&ph.plant, rating, &self.settings(0, res), &c.limits) {
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
            separation: 0.0,
            duty_cout_mg_l: Vec::new(),
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
            match capacity(&ph.plant, rating, &self.settings(k, res), &c.limits) {
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
                rating,
                &self.settings(0, Resolution::report()),
                &c.limits,
            ) {
                Ok(cap) => {
                    q_min = q_min.min(cap.q_treated_m3_h);
                    robust_shortfall += cap.shortfall;
                }
                Err(e) => {
                    q_min = 0.0;
                    score.numerical_failure |= matches!(e, JudgeError::Numerical(_));
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
        // Separation on every duty at the rated (conservative) feed flow, each duty under
        // its worst closure sample. Pressure is fixed by the rating duty at this flow.
        let q_rated = q_min / (1.0 - nominal.reject_fraction).max(1e-9);
        let floor = c.objective.floor_mg_l;
        let (mut sep, mut wsum) = (0.0, 0.0);
        let mut duty_cout = Vec::with_capacity(c.duties.len());
        let mut failed_duty = false;
        for (d, duty) in c.duties.iter().enumerate() {
            let mut worst: Option<f64> = None;
            let mut failed = false;
            for k in 0..self.closures.len() {
                match evaluate(
                    &ph.plant,
                    &duty.feed.with_flow(q_rated),
                    &self.duty_settings(d, k, res),
                ) {
                    Ok(ev) => {
                        worst = Some(worst.map_or(ev.cout_mg_l, |w: f64| w.max(ev.cout_mg_l)));
                        // Closure-specific capacity endpoints need not imply a common
                        // feasible flow. Recheck the actual rated point for duty 0.
                        if d == 0
                            && (ev.cout_mg_l > c.limits.quality_limit_mg_l
                                || ev.equivalent_pressure_pa
                                    > c.limits.pressure_budget_pa * (1.0 + 1e-9))
                        {
                            failed_duty = true;
                            score.reasons.push(format!(
                                "duty 0, closure {k}: rated flow fails quality or pressure budget"
                            ));
                        }
                    }
                    Err(e) => {
                        score.numerical_failure |= matches!(e, JudgeError::Numerical(_));
                        score
                            .reasons
                            .push(format!("duty {d} ('{}'), closure {k}: {e}", duty.name));
                        failed = true;
                        break;
                    }
                }
            }
            if failed {
                failed_duty = true;
                duty_cout.push(None);
                continue;
            }
            let Some(cout) = worst else {
                failed_duty = true;
                duty_cout.push(None);
                score
                    .reasons
                    .push(format!("duty {d}: no evaluated closure"));
                continue;
            };
            sep += duty.weight * (duty.feed.cin_mg_l.max(floor) / cout.max(floor)).ln();
            wsum += duty.weight;
            duty_cout.push(Some(cout));
        }
        score.duty_cout_mg_l = duty_cout;
        if failed_duty {
            score.violation = 1.0;
            return (score, Some(ph), Some(nominal));
        }
        let separation = sep / wsum.max(1e-12);
        let w = &c.objective;
        let energy = score.specific_energy_kj_m3.unwrap_or(1e3).max(1e-6) * 1e3;
        score.feasible = true;
        score.violation = 0.0;
        score.q_treated_m3_h = q_min;
        score.q_feed_m3_h = q_rated;
        score.separation = separation;
        score.fitness = w.separation * separation + w.capacity * q_min.ln()
            - w.energy * (energy / 1e5).ln()
            - w.mass * score.shell_mass_kg.max(1e-3).ln()
            - w.per_unit * units as f64;
        if !score.fitness.is_finite() || !score.separation.is_finite() {
            score.feasible = false;
            score.fitness = 0.0;
            score.separation = 0.0;
            score.q_treated_m3_h = 0.0;
            score.violation = 1.0;
            score.numerical_failure = true;
            score
                .reasons
                .push("numerical: nonfinite objective arithmetic".into());
        }
        (score, Some(ph), Some(nominal))
    }
}
