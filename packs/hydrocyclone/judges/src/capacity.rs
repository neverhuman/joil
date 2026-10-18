//! V5-18-02: common-flow operating maps.
//!
//! Every required scenario is evaluated at the same feed points. A common point is feasible
//! only when every scenario passes there; a failure or an unevaluated scenario (`Gap`) is kept
//! in the map and is never interpolated over. Each transition between adjacent feed points is
//! reported as a bracket. A continuous band is claimed only when admitted regularity evidence
//! exists and one feasible run is bounded by tested points; otherwise only sampled capacity is
//! reported. Nothing here is a physical capacity claim.
use crate::{invalid, Field, JudgeError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScenarioOutcome {
    /// Every gate of this scenario passes. The conservative (largest) reject split is kept for
    /// the treated lower bound.
    Pass {
        reject_fraction_upper: f64,
    },
    Fail {
        reasons: Vec<String>,
    },
    /// Not evaluated or not resolved. A gap is never a pass.
    Gap {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedPoint {
    pub feed_m3_h: f64,
    pub outcomes: BTreeMap<String, ScenarioOutcome>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatingMap {
    pub required: Vec<String>,
    pub points: Vec<FeedPoint>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointKind {
    Feasible,
    Infeasible,
    Gap,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampledCapacity {
    pub feed_m3_h: f64,
    /// `min_s Q (1 - r_s,upper)` at this same feed.
    pub treated_lower_m3_h: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bracket {
    pub below_feed_m3_h: f64,
    pub above_feed_m3_h: f64,
    pub below: PointKind,
    pub above: PointKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeasibleRun {
    pub lower_feed_m3_h: f64,
    pub upper_feed_m3_h: f64,
    pub tested_points: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "claim", rename_all = "snake_case", deny_unknown_fields)]
pub enum BandClaim {
    SampledOnly {
        reason: String,
    },
    Band {
        lower_feed_m3_h: f64,
        upper_feed_m3_h: f64,
    },
}

/// Regularity admitted for interpolation between adjacent tested points.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regularity {
    pub max_spacing_m3_h: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonCapacityReport {
    /// Largest tested feed where all scenarios pass. `None` when no common duty exists.
    pub sampled: Option<SampledCapacity>,
    pub failures_m3_h: Vec<f64>,
    pub gaps_m3_h: Vec<f64>,
    pub brackets: Vec<Bracket>,
    pub runs: Vec<FeasibleRun>,
    pub band: BandClaim,
}

fn kind(outcomes: &BTreeMap<String, ScenarioOutcome>, required: &[String]) -> PointKind {
    let mut failed = false;
    let mut gapped = false;
    for name in required {
        match outcomes.get(name) {
            Some(ScenarioOutcome::Pass { .. }) => {}
            Some(ScenarioOutcome::Fail { .. }) => failed = true,
            Some(ScenarioOutcome::Gap { .. }) | None => gapped = true,
        }
    }
    if failed {
        PointKind::Infeasible
    } else if gapped {
        PointKind::Gap
    } else {
        PointKind::Feasible
    }
}

impl OperatingMap {
    fn check(&self) -> Result<(), JudgeError> {
        let names: BTreeSet<&str> = self.required.iter().map(String::as_str).collect();
        if self.required.is_empty() || names.len() != self.required.len() || names.contains("") {
            return Err(invalid(
                "required scenarios must be nonempty, unique and named",
            ));
        }
        let mut feeds = Vec::new();
        for point in &self.points {
            if !(point.feed_m3_h.is_finite() && point.feed_m3_h > 0.0)
                || feeds.contains(&point.feed_m3_h)
            {
                return Err(invalid("feed points must be finite, positive and distinct"));
            }
            feeds.push(point.feed_m3_h);
            for (name, outcome) in &point.outcomes {
                if !names.contains(name.as_str()) {
                    return Err(invalid(format!("unexpected scenario {name}")));
                }
                if let ScenarioOutcome::Pass {
                    reject_fraction_upper,
                } = outcome
                {
                    if !(reject_fraction_upper.is_finite()
                        && (0.0..1.0).contains(reject_fraction_upper))
                    {
                        return Err(invalid("pass requires a reject split in [0, 1)"));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Evaluates the common-flow map. `regularity` is the only route to a band claim.
pub fn evaluate_common_map(
    map: &OperatingMap,
    regularity: &Field<Regularity>,
) -> Result<CommonCapacityReport, JudgeError> {
    map.check()?;
    let mut points: Vec<&FeedPoint> = map.points.iter().collect();
    points.sort_by(|a, b| a.feed_m3_h.total_cmp(&b.feed_m3_h));

    let mut sampled: Option<SampledCapacity> = None;
    let mut failures = Vec::new();
    let mut gaps = Vec::new();
    let mut kinds = Vec::new();
    for point in &points {
        let k = kind(&point.outcomes, &map.required);
        match k {
            PointKind::Feasible => {
                let treated_lower = map
                    .required
                    .iter()
                    .map(|name| match point.outcomes.get(name) {
                        Some(ScenarioOutcome::Pass {
                            reject_fraction_upper,
                        }) => point.feed_m3_h * (1.0 - reject_fraction_upper),
                        _ => f64::NAN,
                    })
                    .fold(f64::INFINITY, f64::min);
                sampled = Some(SampledCapacity {
                    feed_m3_h: point.feed_m3_h,
                    treated_lower_m3_h: treated_lower,
                });
            }
            PointKind::Infeasible => failures.push(point.feed_m3_h),
            PointKind::Gap => gaps.push(point.feed_m3_h),
        }
        kinds.push(k);
    }

    let mut brackets = Vec::new();
    for (pair, kinds_pair) in points.windows(2).zip(kinds.windows(2)) {
        if kinds_pair[0] != kinds_pair[1] {
            brackets.push(Bracket {
                below_feed_m3_h: pair[0].feed_m3_h,
                above_feed_m3_h: pair[1].feed_m3_h,
                below: kinds_pair[0],
                above: kinds_pair[1],
            });
        }
    }

    // Runs are maximal sequences of adjacent feasible points: a failure or gap breaks a run.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for (i, k) in kinds.iter().enumerate() {
        match (k, start) {
            (PointKind::Feasible, None) => start = Some(i),
            (PointKind::Feasible, Some(_)) => {}
            (_, Some(s)) => {
                runs.push((s, i - 1));
                start = None;
            }
            (_, None) => {}
        }
    }
    if let Some(s) = start {
        runs.push((s, kinds.len() - 1));
    }
    let runs_out: Vec<FeasibleRun> = runs
        .iter()
        .map(|&(s, e)| FeasibleRun {
            lower_feed_m3_h: points[s].feed_m3_h,
            upper_feed_m3_h: points[e].feed_m3_h,
            tested_points: e - s + 1,
        })
        .collect();

    let band = match regularity {
        Field::Admitted { value, .. } => {
            let spacing_ok = |&(s, e): &(usize, usize)| {
                (s..e).all(|i| {
                    points[i + 1].feed_m3_h - points[i].feed_m3_h <= value.max_spacing_m3_h
                })
            };
            match runs.as_slice() {
                [only] if spacing_ok(only) => BandClaim::Band {
                    lower_feed_m3_h: points[only.0].feed_m3_h,
                    upper_feed_m3_h: points[only.1].feed_m3_h,
                },
                [_] => BandClaim::SampledOnly {
                    reason: "feasible run has spacing above admitted regularity".into(),
                },
                [] => BandClaim::SampledOnly {
                    reason: "no common feasible run".into(),
                },
                _ => BandClaim::SampledOnly {
                    reason: "multiple feasible runs separated by failures or gaps".into(),
                },
            }
        }
        _ => BandClaim::SampledOnly {
            reason: "interpolation regularity not admitted".into(),
        },
    };

    Ok(CommonCapacityReport {
        sampled,
        failures_m3_h: failures,
        gaps_m3_h: gaps,
        brackets,
        runs: runs_out,
        band,
    })
}
