//! Benchmark summaries use complete attempts and qualified unique decisions.
//! These records do not assert CFD quality; eligibility requires bound physical
//! evidence and all declared acceptance flags.
use crate::{BlobDigest, Result, RuntimeError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const BENCHMARKS: [&str; 7] = [
    "BENCH-01", "BENCH-02", "BENCH-03", "BENCH-04", "BENCH-05", "BENCH-06", "BENCH-07",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheState {
    Cold,
    Warm,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptOutcome {
    Qualified,
    Rejected,
    NumericalFailure,
    InfrastructureFailure,
    Timeout,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkAttempt {
    pub attempt_id: String,
    pub decision_id: String,
    pub corpus: BlobDigest,
    pub benchmark: String,
    pub cache: CacheState,
    pub outcome: AttemptOutcome,
    pub wall_ms: u64,
    pub peak_rss_bytes: u64,
    pub disk_bytes: u64,
    pub resource_class: String,
    pub numerical_quality_pass: bool,
    pub physical_evidence: Option<BlobDigest>,
    pub synthetic_evaluator: bool,
    pub raw_cell_updates: Option<u64>,
    pub vram_occupancy_bytes: Option<u64>,
}
impl BenchmarkAttempt {
    pub fn is_qualified(&self) -> bool {
        self.outcome == AttemptOutcome::Qualified
            && self.numerical_quality_pass
            && self.physical_evidence.is_some()
            && !self.synthetic_evaluator
    }
    pub fn check(&self) -> Result<()> {
        if !BENCHMARKS.contains(&self.benchmark.as_str())
            || self.attempt_id.trim().is_empty()
            || self.decision_id.trim().is_empty()
            || self.resource_class.trim().is_empty()
            || self.wall_ms > 2_u64.pow(53)
            || self.peak_rss_bytes > 2_u64.pow(53)
            || self.disk_bytes > 2_u64.pow(53)
        {
            return Err(RuntimeError::Invalid("benchmark_attempt_identity_or_range"));
        }
        if self.synthetic_evaluator && self.is_qualified() {
            return Err(RuntimeError::Invalid("synthetic_evaluator_not_qualified"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Distribution {
    pub count: u64,
    pub p50: u64,
    pub p95: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkSummary {
    pub benchmark: String,
    pub corpus: BlobDigest,
    pub attempts: u64,
    pub failed_attempts: u64,
    pub qualified_unique_decisions: u64,
    pub wall_ms: Distribution,
    pub rss_bytes: Distribution,
    pub disk_bytes: Distribution,
    pub resource_class: String,
    pub eligible_throughput_per_wall_second: Option<f64>,
}
fn distribution(mut values: Vec<u64>) -> Result<Distribution> {
    if values.is_empty() {
        return Err(RuntimeError::Invalid("empty_benchmark_population"));
    }
    values.sort_unstable();
    let percentile = |p: usize| values[((values.len() - 1) * p).div_ceil(100)];
    Ok(Distribution {
        count: values.len() as u64,
        p50: percentile(50),
        p95: percentile(95),
    })
}
pub fn summarize(rows: &[BenchmarkAttempt]) -> Result<BenchmarkSummary> {
    let first = rows
        .first()
        .ok_or(RuntimeError::Invalid("empty_benchmark_population"))?;
    for row in rows {
        row.check()?;
    }
    if rows.iter().any(|r| {
        r.benchmark != first.benchmark
            || r.corpus != first.corpus
            || r.resource_class != first.resource_class
    }) {
        return Err(RuntimeError::Invalid("mixed_benchmark_cohort"));
    }
    let unique: BTreeSet<_> = rows
        .iter()
        .filter(|r| r.is_qualified())
        .map(|r| &r.decision_id)
        .collect();
    let wall = distribution(rows.iter().map(|r| r.wall_ms).collect())?;
    let elapsed = rows.iter().map(|r| r.wall_ms).sum::<u64>();
    let rate = if unique.is_empty() || elapsed == 0 {
        None
    } else {
        Some(unique.len() as f64 * 1000.0 / elapsed as f64)
    };
    Ok(BenchmarkSummary {
        benchmark: first.benchmark.clone(),
        corpus: first.corpus.clone(),
        attempts: rows.len() as u64,
        failed_attempts: rows
            .iter()
            .filter(|r| !matches!(r.outcome, AttemptOutcome::Qualified))
            .count() as u64,
        qualified_unique_decisions: unique.len() as u64,
        wall_ms: wall,
        rss_bytes: distribution(rows.iter().map(|r| r.peak_rss_bytes).collect())?,
        disk_bytes: distribution(rows.iter().map(|r| r.disk_bytes).collect())?,
        resource_class: first.resource_class.clone(),
        eligible_throughput_per_wall_second: rate,
    })
}
