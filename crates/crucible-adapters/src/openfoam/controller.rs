//! Split controller, Richardson convergence certification, and native case/restart evidence guards.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SplitControllerConfig {
    pub target_reject_fraction: f64,
    pub reachable_min: f64,
    pub reachable_max: f64,
    pub initial_backpressure_pa: f64,
    pub pressure_min_pa: f64,
    pub pressure_max_pa: f64,
    pub kp_pa_per_fraction: f64,
    pub ki_pa_per_fraction_s: f64,
    pub settle_tolerance: f64,
    pub settle_window: usize,
    pub sample_count: usize,
    pub sample_interval_s: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ControllerSample {
    pub time_s: f64,
    pub measured_reject_fraction: f64,
    pub backpressure_pa: f64,
    pub saturated: bool,
    pub settled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerRecord {
    pub config: SplitControllerConfig,
    pub history: Vec<ControllerSample>,
}

pub struct SplitController {
    config: SplitControllerConfig,
    integral: f64,
    history: Vec<ControllerSample>,
}

impl SplitController {
    pub fn new(config: SplitControllerConfig) -> Result<Self, String> {
        if !config.target_reject_fraction.is_finite()
            || !config.reachable_min.is_finite()
            || !config.reachable_max.is_finite()
            || config.reachable_min < 0.0
            || config.reachable_max > 1.0
            || config.reachable_min > config.reachable_max
            || config.target_reject_fraction < config.reachable_min
            || config.target_reject_fraction > config.reachable_max
            || config.pressure_min_pa >= config.pressure_max_pa
            || config.initial_backpressure_pa < config.pressure_min_pa
            || config.initial_backpressure_pa > config.pressure_max_pa
            || config.kp_pa_per_fraction <= 0.0
            || config.ki_pa_per_fraction_s < 0.0
            || config.settle_tolerance <= 0.0
            || config.settle_window == 0
            || config.sample_count == 0
            || config.sample_interval_s <= 0.0
            || ![
                config.pressure_min_pa,
                config.pressure_max_pa,
                config.initial_backpressure_pa,
                config.kp_pa_per_fraction,
                config.ki_pa_per_fraction_s,
                config.settle_tolerance,
                config.sample_interval_s,
            ]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err("invalid or unreachable split-controller configuration".into());
        }
        Ok(Self {
            config,
            integral: 0.0,
            history: Vec::new(),
        })
    }

    pub fn observe(&mut self, time_s: f64, measured: f64, dt_s: f64) -> Result<f64, String> {
        if !time_s.is_finite()
            || !measured.is_finite()
            || !dt_s.is_finite()
            || dt_s <= 0.0
            || !(0.0..=1.0).contains(&measured)
            || self.history.last().is_some_and(|p| time_s <= p.time_s)
        {
            return Err("invalid controller observation".into());
        }
        if measured < self.config.reachable_min || measured > self.config.reachable_max {
            return Err("observed split outside declared reachable range".into());
        }
        let error = self.config.target_reject_fraction - measured;
        let candidate_integral = self.integral + error * dt_s;
        // Raising reject outlet pressure reduces reject flow; positive split error
        // therefore commands lower backpressure.
        let raw = self.config.initial_backpressure_pa
            - self.config.kp_pa_per_fraction * error
            - self.config.ki_pa_per_fraction_s * candidate_integral;
        let next = raw.clamp(self.config.pressure_min_pa, self.config.pressure_max_pa);
        let saturated = raw != next;
        // Conditional integration prevents windup when error drives farther into a rail.
        if !saturated || (raw > next && error > 0.0) || (raw < next && error < 0.0) {
            self.integral = candidate_integral;
        }
        let n = self.history.len();
        let settled = n + 1 >= self.config.settle_window
            && self
                .history
                .iter()
                .rev()
                .take(self.config.settle_window.saturating_sub(1))
                .all(|s| {
                    (s.measured_reject_fraction - self.config.target_reject_fraction).abs()
                        <= self.config.settle_tolerance
                })
            && error.abs() <= self.config.settle_tolerance;
        self.history.push(ControllerSample {
            time_s,
            measured_reject_fraction: measured,
            backpressure_pa: next,
            saturated,
            settled,
        });
        Ok(next)
    }

    pub fn accepted_statistics(&self, expected_samples: usize) -> Result<(), String> {
        if self.history.iter().any(|s| s.saturated) {
            return Err("controller saturation blocks statistics".into());
        }
        if !self.history.last().is_some_and(|s| s.settled) {
            return Err("controller did not settle before sampling".into());
        }
        if expected_samples != self.config.sample_count {
            return Err("sampling settings differ from frozen controller contract".into());
        }
        Ok(())
    }
    pub fn history(&self) -> &[ControllerSample] {
        &self.history
    }

    pub fn record(&self) -> ControllerRecord {
        ControllerRecord {
            config: self.config.clone(),
            history: self.history.clone(),
        }
    }
}

/// Refuse certification when the ordered refinement sequence is not monotone.
pub fn richardson_certificate(
    values: &[f64],
    refinement_ratio: f64,
    budget: f64,
) -> Result<f64, String> {
    if values.len() < 3
        || !refinement_ratio.is_finite()
        || refinement_ratio <= 1.0
        || !budget.is_finite()
        || budget <= 0.0
        || !values.iter().all(|v| v.is_finite())
    {
        return Err("invalid refinement data or preallocated error budget".into());
    }
    let d0 = values[1] - values[0];
    let d1 = values[2] - values[1];
    if d0 == 0.0 || d1 == 0.0 || d0.signum() != d1.signum() || d1.abs() >= d0.abs() {
        return Err(
            "oscillatory or non-asymptotic sequence cannot receive Richardson certificate".into(),
        );
    }
    let p = (d0 / d1).ln() / refinement_ratio.ln();
    if !p.is_finite() || p <= 0.0 {
        return Err("invalid observed order".into());
    }
    let error = d1.abs() / (refinement_ratio.powf(p) - 1.0);
    if error > budget {
        return Err("estimated discretization error exceeds preallocated budget".into());
    }
    Ok(error)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCaseEvidence {
    pub requested_model: String,
    pub native_solver_exit_code: Option<i32>,
    pub field_write_complete: bool,
    pub absolute_pressure_reference_pa: Option<f64>,
    pub mesh_health_passed: bool,
    pub restart_state_sha256: Option<String>,
    pub statistics_state_sha256: Option<String>,
}

pub fn native_case_pass(e: &NativeCaseEvidence) -> Result<(), String> {
    if e.native_solver_exit_code != Some(0)
        || !e.field_write_complete
        || !e.mesh_health_passed
        || !e
            .absolute_pressure_reference_pa
            .is_some_and(|p| p.is_finite() && p > 0.0)
        || e.restart_state_sha256
            .as_ref()
            .is_none_or(|s| s.len() != 64)
        || e.statistics_state_sha256
            .as_ref()
            .is_none_or(|s| s.len() != 64)
    {
        return Err("native execution/restart evidence incomplete; cannot PASS".into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestartComparison {
    pub fields_complete_before: bool,
    pub fields_complete_after: bool,
    pub statistics_complete_before: bool,
    pub statistics_complete_after: bool,
    pub mesh_health_before: bool,
    pub mesh_health_after: bool,
    pub absolute_pressure_known: bool,
    pub observables_before: Vec<f64>,
    pub observables_after: Vec<f64>,
    pub observable_tolerances: Vec<f64>,
}

pub fn verify_restart(comparison: &RestartComparison) -> Result<(), String> {
    let n = comparison.observables_before.len();
    if !comparison.fields_complete_before
        || !comparison.fields_complete_after
        || !comparison.statistics_complete_before
        || !comparison.statistics_complete_after
        || !comparison.mesh_health_before
        || !comparison.mesh_health_after
        || !comparison.absolute_pressure_known
        || n == 0
        || comparison.observables_after.len() != n
        || comparison.observable_tolerances.len() != n
    {
        return Err("restart is missing complete fields/statistics, known pressure, healthy mesh, or observable pairs".into());
    }
    for ((before, after), tolerance) in comparison
        .observables_before
        .iter()
        .zip(&comparison.observables_after)
        .zip(&comparison.observable_tolerances)
    {
        if !before.is_finite()
            || !after.is_finite()
            || !tolerance.is_finite()
            || *tolerance <= 0.0
            || (before - after).abs() > *tolerance
        {
            return Err("restart observable differs beyond its declared tolerance".into());
        }
    }
    Ok(())
}
