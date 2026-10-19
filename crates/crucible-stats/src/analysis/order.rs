//! V5-04-04: multiplicity, rank effect size, distribution-free tails and censoring.
//!
//! Holm step-down adjustment, Cliff's delta (ties count zero) with an exact small-
//! sample permutation test, and exact binomial order-statistic intervals for
//! quantiles. Tail intervals require declared IID draws on distinct random streams;
//! independent blocks, quasi-random points, correlated draws and finite scenario
//! envelopes are refused rather than given a probabilistic tail. A value below the
//! limit of quantification is a left-censored observation, never a measured zero:
//! an order statistic in the censored range is reported only as "at most LOQ".
use crate::StatsError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Holm step-down adjusted p-values, in input order. Ties keep their input order.
pub fn holm_adjusted(p_values: &[f64]) -> Result<Vec<f64>, StatsError> {
    if p_values.is_empty()
        || p_values
            .iter()
            .any(|p| !p.is_finite() || !(0.0..=1.0).contains(p))
    {
        return Err(StatsError::InvalidInput(
            "p-values must be a nonempty list in [0, 1]",
        ));
    }
    let m = p_values.len();
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&i, &j| p_values[i].total_cmp(&p_values[j]).then(i.cmp(&j)));
    let mut adjusted = vec![0.0; m];
    let mut running = 0.0_f64;
    for (rank, &i) in order.iter().enumerate() {
        running = running.max(((m - rank) as f64 * p_values[i]).min(1.0));
        adjusted[i] = running;
    }
    Ok(adjusted)
}

/// Familywise rejections at `alpha` under Holm's procedure.
pub fn holm_rejections(p_values: &[f64], alpha: f64) -> Result<Vec<bool>, StatsError> {
    if !(alpha > 0.0 && alpha < 1.0) {
        return Err(StatsError::InvalidInput("alpha must be in (0, 1)"));
    }
    Ok(holm_adjusted(p_values)?
        .into_iter()
        .map(|p| p <= alpha)
        .collect())
}

fn finite_samples(a: &[f64], b: &[f64]) -> Result<(), StatsError> {
    if a.is_empty() || b.is_empty() || a.iter().chain(b).any(|x| !x.is_finite()) {
        return Err(StatsError::InvalidInput(
            "two nonempty finite samples required",
        ));
    }
    Ok(())
}

/// Cliff's delta P(A > B) - P(A < B) over all pairs; ties contribute zero.
/// Numeric comparison, not `total_cmp`: `-0.0` and `+0.0` are equal and must tie.
pub fn cliffs_delta(a: &[f64], b: &[f64]) -> Result<f64, StatsError> {
    finite_samples(a, b)?;
    let mut dominance = 0i64;
    for x in a {
        for y in b {
            dominance += if x > y {
                1
            } else if x < y {
                -1
            } else {
                0
            };
        }
    }
    Ok(dominance as f64 / (a.len() * b.len()) as f64)
}

/// Largest pooled sample for which every relabelling is enumerated.
pub const MAX_EXACT_PERMUTATION_SAMPLE: usize = 20;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PermutationTest {
    pub delta: f64,
    /// Exact one-sided p-value P(delta under relabelling >= observed delta).
    pub p_greater: f64,
    pub relabellings: u64,
    pub interpretation: String,
}

/// Exact permutation test of Cliff's delta under exchangeability of the pooled sample.
pub fn cliffs_delta_permutation(a: &[f64], b: &[f64]) -> Result<PermutationTest, StatsError> {
    finite_samples(a, b)?;
    let pooled: Vec<f64> = a.iter().chain(b).copied().collect();
    if pooled.len() > MAX_EXACT_PERMUTATION_SAMPLE {
        return Err(StatsError::Unsupported(
            "exact permutation limited to 20 pooled observations",
        ));
    }
    let observed = cliffs_delta(a, b)?;
    let (n, k) = (pooled.len(), a.len());
    let (mut total, mut extreme) = (0u64, 0u64);
    for mask in 0u32..(1u32 << n) {
        if mask.count_ones() as usize != k {
            continue;
        }
        let (mut x, mut y) = (Vec::with_capacity(k), Vec::with_capacity(n - k));
        for (i, v) in pooled.iter().enumerate() {
            if mask & (1 << i) != 0 {
                x.push(*v);
            } else {
                y.push(*v);
            }
        }
        total += 1;
        // Compare in exact integer dominance counts to avoid ties lost to rounding.
        if cliffs_delta(&x, &y)? * (k * (n - k)) as f64 >= observed * (k * (n - k)) as f64 - 1e-9 {
            extreme += 1;
        }
    }
    Ok(PermutationTest {
        delta: observed,
        p_greater: extreme as f64 / total as f64,
        relabellings: total,
        interpretation: "exact relabelling distribution under exchangeability of all pooled observations; not valid for dependent or clustered observations".into(),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OrderObservation {
    Measured {
        value: f64,
    },
    /// Left-censored: the value lies below this limit of quantification.
    BelowLoq {
        loq: f64,
    },
}
pub type Observation = OrderObservation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sampling {
    Iid,
    IndependentBlocks,
    FiniteEnvelope,
    QuasiRandom,
    Correlated,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draw {
    pub draw_id: String,
    /// Identity of the random stream that produced this draw; IID draws need distinct streams.
    pub stream: String,
    pub observation: Observation,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Endpoint {
    Measured {
        value: f64,
    },
    /// The order statistic is censored: known only to be at most this LOQ.
    AtMostLoq {
        loq: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantileInterval {
    pub quantile: f64,
    pub confidence: f64,
    pub n: usize,
    pub lower_rank: usize,
    pub upper_rank: usize,
    pub lower: Endpoint,
    pub upper: Endpoint,
    pub exact_coverage: f64,
    pub censored_observations: usize,
    pub interpretation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TailOutcome {
    Interval(QuantileInterval),
    Insufficient {
        n: usize,
        minimum_n: usize,
        reason: String,
    },
}

fn check_level(quantile: f64, confidence: f64) -> Result<(), StatsError> {
    if !(quantile > 0.0 && quantile < 1.0 && confidence > 0.0 && confidence < 1.0) {
        return Err(StatsError::InvalidInput(
            "quantile and confidence must be in (0, 1)",
        ));
    }
    Ok(())
}

/// Binomial(n, q) cumulative masses P(X <= k), k = 0..n, accumulated in log space.
fn binomial_cdf(n: usize, q: f64) -> Vec<f64> {
    let mut log_mass = n as f64 * (1.0 - q).ln();
    let mut masses = Vec::with_capacity(n + 1);
    for k in 0..=n {
        masses.push(log_mass.exp());
        if k < n {
            log_mass += ((n - k) as f64).ln() - ((k + 1) as f64).ln() + q.ln() - (1.0 - q).ln();
        }
    }
    let total: f64 = masses.iter().sum();
    let mut cumulative = 0.0;
    masses
        .iter()
        .map(|m| {
            cumulative += m / total;
            cumulative
        })
        .collect()
}

/// Exact order-statistic ranks (lower, upper) for a two-sided interval, if both exist.
pub fn order_statistic_ranks(
    n: usize,
    quantile: f64,
    confidence: f64,
) -> Result<Option<(usize, usize, f64)>, StatsError> {
    check_level(quantile, confidence)?;
    if n == 0 {
        return Ok(None);
    }
    let a = (1.0 - confidence) / 2.0;
    let cdf = binomial_cdf(n, quantile);
    let (mut lower, mut upper) = (None, None);
    for r in 1..=n {
        let f = cdf[r - 1];
        if f <= a {
            lower = Some(r);
        }
        if f >= 1.0 - a && upper.is_none() {
            upper = Some(r);
        }
    }
    Ok(match (lower, upper) {
        (Some(l), Some(u)) => Some((l, u, cdf[u - 1] - cdf[l - 1])),
        _ => None,
    })
}

/// Smallest IID sample size for which both exact ranks exist.
pub fn minimum_iid_draws(quantile: f64, confidence: f64) -> Result<usize, StatsError> {
    for n in 1..=100_000 {
        if order_statistic_ranks(n, quantile, confidence)?.is_some() {
            return Ok(n);
        }
    }
    Err(StatsError::Unsupported(
        "minimum sample size exceeds 100000",
    ))
}

/// Distribution-free interval for a quantile from declared IID draws.
pub fn quantile_interval(
    draws: &[Draw],
    sampling: Sampling,
    quantile: f64,
    confidence: f64,
) -> Result<TailOutcome, StatsError> {
    check_level(quantile, confidence)?;
    match sampling {
        Sampling::Iid => {}
        Sampling::IndependentBlocks => {
            return Err(StatsError::Unsupported(
                "order-statistic tail intervals need IID draws, not block summaries",
            ))
        }
        Sampling::FiniteEnvelope => {
            return Err(StatsError::Unsupported(
                "a finite scenario envelope is not a probability distribution",
            ))
        }
        Sampling::QuasiRandom => {
            return Err(StatsError::Unsupported(
                "quasi-random points are not IID order-statistic samples",
            ))
        }
        Sampling::Correlated => {
            return Err(StatsError::Unsupported(
                "correlated draws cannot support an IID tail interval",
            ))
        }
    }
    let (mut ids, mut streams) = (BTreeSet::new(), BTreeSet::new());
    for d in draws {
        if joil_evidence::identity::check_id(&d.draw_id).is_err() || !ids.insert(d.draw_id.as_str())
        {
            return Err(StatsError::InvalidInput(
                "invalid or duplicate draw identity",
            ));
        }
        if joil_evidence::identity::check_id(&d.stream).is_err()
            || !streams.insert(d.stream.as_str())
        {
            return Err(StatsError::Unsupported(
                "draws sharing a random stream are not independent",
            ));
        }
        match d.observation {
            Observation::Measured { value } if value.is_finite() => {}
            Observation::BelowLoq { loq } if loq.is_finite() && loq > 0.0 => {}
            _ => {
                return Err(StatsError::InvalidInput(
                    "nonfinite measurement or nonpositive LOQ",
                ))
            }
        }
    }
    let n = draws.len();
    let minimum_n = minimum_iid_draws(quantile, confidence)?;
    let Some((lower_rank, upper_rank, exact_coverage)) =
        order_statistic_ranks(n, quantile, confidence)?
    else {
        return Ok(TailOutcome::Insufficient {
            n,
            minimum_n,
            reason: "too few IID draws for two finite exact order-statistic bounds".into(),
        });
    };
    let mut measured: Vec<f64> = draws
        .iter()
        .filter_map(|d| match d.observation {
            Observation::Measured { value } => Some(value),
            Observation::BelowLoq { .. } => None,
        })
        .collect();
    measured.sort_by(f64::total_cmp);
    let censored = n - measured.len();
    let max_loq = draws
        .iter()
        .filter_map(|d| match d.observation {
            Observation::BelowLoq { loq } => Some(loq),
            Observation::Measured { .. } => None,
        })
        .fold(f64::NEG_INFINITY, f64::max);
    if censored > 0 && measured.first().is_some_and(|m| *m < max_loq) {
        return Err(StatsError::Unsupported(
            "censored observations interleave measured values; order statistics are not identified",
        ));
    }
    let endpoint = |rank: usize| {
        if rank <= censored {
            Endpoint::AtMostLoq { loq: max_loq }
        } else {
            Endpoint::Measured {
                value: measured[rank - censored - 1],
            }
        }
    };
    Ok(TailOutcome::Interval(QuantileInterval {
        quantile,
        confidence,
        n,
        lower_rank,
        upper_rank,
        lower: endpoint(lower_rank),
        upper: endpoint(upper_rank),
        exact_coverage,
        censored_observations: censored,
        interpretation: format!(
            "two-sided exact binomial order-statistic interval for the {quantile} quantile assuming IID draws (exact coverage {exact_coverage:.6}); censored ranks are reported as at most LOQ, never as zero"
        ),
    }))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FiniteEnvelope {
    pub scenarios: usize,
    pub minimum: f64,
    pub maximum: f64,
    pub interpretation: String,
}

/// Deterministic envelope over named scenarios. It is never a probability or reliability.
pub fn finite_envelope(values: &[(String, f64)]) -> Result<FiniteEnvelope, StatsError> {
    let mut seen = BTreeSet::new();
    for (scenario, value) in values {
        if joil_evidence::identity::check_id(scenario).is_err() || !seen.insert(scenario.as_str()) {
            return Err(StatsError::InvalidInput(
                "invalid or duplicate scenario identity",
            ));
        }
        if !value.is_finite() {
            return Err(StatsError::InvalidInput("nonfinite scenario value"));
        }
    }
    if values.is_empty() {
        return Err(StatsError::InvalidInput("no scenarios"));
    }
    Ok(FiniteEnvelope {
        scenarios: values.len(),
        minimum: values.iter().map(|v| v.1).fold(f64::INFINITY, f64::min),
        maximum: values.iter().map(|v| v.1).fold(f64::NEG_INFINITY, f64::max),
        interpretation: "finite scenario envelope over exactly the listed scenarios; not a probability, quantile or reliability".into(),
    })
}
