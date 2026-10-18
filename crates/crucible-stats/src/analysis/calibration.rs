//! V5-04-05: preregistered repeated synthetic calibration.
//!
//! A preregistration fixes seeds, generators, unit and cluster structure, n, trial
//! counts, methods and acceptance rules before any trial runs. Its canonical
//! identity is recorded with every report so a tuned rerun cannot pose as the
//! registered one. Coverage, false-qualification and power rates are distinct
//! metrics, each reported with its binomial Monte Carlo standard error. Adaptive
//! attempt charges belong to the experiment-family ledger (crucible-runtime), not to
//! this module. Synthetic calibration is software verification, not physical evidence.
use crate::StatsError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Packet-wide bounds from the C007 assignment.
pub const MAX_TOTAL_TRIALS: u64 = 10_000;
pub const MAX_UNITS_PER_TRIAL: u64 = 128;
pub const MAX_CLUSTERS_PER_TRIAL: u64 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    /// Probability that the interval (or every interval of a family) covers the truth.
    Coverage,
    /// Probability of qualifying when a component sits exactly on its failure boundary.
    FalseQualification,
    /// Probability of qualifying under the declared useful alternative.
    Power,
    /// Behaviour under a deliberately violated assumption; never a method claim.
    AssumptionViolationDemonstration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Acceptance {
    AtLeast,
    AtMost,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationCase {
    pub id: String,
    pub method: String,
    pub metric: Metric,
    pub generator: serde_json::Value,
    pub seed: u64,
    pub trials: u64,
    pub independent_units_per_trial: u64,
    pub clusters_per_trial: u64,
    pub target_rate: f64,
    pub acceptance: Acceptance,
    /// One-sided normal multiplier on the binomial SE at the target rate.
    pub tolerance_z: f64,
    pub scope: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preregistration {
    pub schema: String,
    pub synthetic: bool,
    pub purpose: String,
    pub rng: String,
    /// Trials of this packet already spent elsewhere (V5-04-02 coverage).
    pub prior_packet_trials: u64,
    pub stopping: String,
    pub cases: Vec<CalibrationCase>,
}

impl Preregistration {
    /// Refuse unbounded, duplicate or unscoped registrations before any trial runs.
    pub fn check(&self) -> Result<String, StatsError> {
        if !self.synthetic || self.cases.is_empty() {
            return Err(StatsError::InvalidInput(
                "calibration must be a nonempty synthetic registration",
            ));
        }
        for text in [&self.schema, &self.purpose, &self.rng, &self.stopping] {
            if joil_evidence::identity::check_id(text).is_err() {
                return Err(StatsError::InvalidInput("missing registration statement"));
            }
        }
        let mut ids = BTreeSet::new();
        let mut total = self.prior_packet_trials;
        for c in &self.cases {
            if joil_evidence::identity::check_id(&c.id).is_err() || !ids.insert(c.id.as_str()) {
                return Err(StatsError::InvalidInput(
                    "invalid or duplicate case identity",
                ));
            }
            if joil_evidence::identity::check_id(&c.method).is_err()
                || joil_evidence::identity::check_id(&c.scope).is_err()
            {
                return Err(StatsError::InvalidInput(
                    "each case names its method and scope",
                ));
            }
            if c.trials == 0
                || c.independent_units_per_trial == 0
                || c.independent_units_per_trial > MAX_UNITS_PER_TRIAL
                || c.clusters_per_trial > MAX_CLUSTERS_PER_TRIAL
            {
                return Err(StatsError::InvalidInput(
                    "case outside the unit or cluster bound",
                ));
            }
            if !(c.target_rate > 0.0
                && c.target_rate < 1.0
                && c.tolerance_z > 0.0
                && c.tolerance_z <= 4.0)
            {
                return Err(StatsError::InvalidInput(
                    "target rate or tolerance outside its range",
                ));
            }
            total = total
                .checked_add(c.trials)
                .ok_or(StatsError::InvalidInput("trial count overflow"))?;
        }
        if total > MAX_TOTAL_TRIALS {
            return Err(StatsError::InvalidInput(
                "registered trials exceed the packet bound",
            ));
        }
        joil_evidence::identity::scientific_hash(self)
            .map_err(|_| StatsError::InvalidInput("registration is not canonically serializable"))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RateReport {
    pub case_id: String,
    pub metric: Metric,
    pub registration_identity: String,
    pub successes: u64,
    pub trials: u64,
    pub rate: f64,
    pub monte_carlo_se: f64,
    pub acceptance_bound: f64,
    pub passed: bool,
    pub scope: String,
}

/// Report one registered case. The trial count must equal the registered count.
pub fn report(
    registration: &Preregistration,
    case_id: &str,
    successes: u64,
    trials: u64,
) -> Result<RateReport, StatsError> {
    let registration_identity = registration.check()?;
    let case = registration
        .cases
        .iter()
        .find(|c| c.id == case_id)
        .ok_or(StatsError::InvalidInput("unregistered calibration case"))?;
    if trials != case.trials || successes > trials {
        return Err(StatsError::InvalidInput(
            "executed trials differ from the registration",
        ));
    }
    let rate = successes as f64 / trials as f64;
    let monte_carlo_se = (rate * (1.0 - rate) / trials as f64).sqrt();
    let target_se = (case.target_rate * (1.0 - case.target_rate) / trials as f64).sqrt();
    let (acceptance_bound, passed) = match case.acceptance {
        Acceptance::AtLeast => {
            let bound = case.target_rate - case.tolerance_z * target_se;
            (bound, rate >= bound)
        }
        Acceptance::AtMost => {
            let bound = case.target_rate + case.tolerance_z * target_se;
            (bound, rate <= bound)
        }
    };
    Ok(RateReport {
        case_id: case.id.clone(),
        metric: case.metric,
        registration_identity,
        successes,
        trials,
        rate,
        monte_carlo_se,
        acceptance_bound,
        passed,
        scope: case.scope.clone(),
    })
}
