//! Finite sample summaries. These do not assert independence, coverage or power.
use crate::StatsError;

/// Return the arithmetic mean and unbiased sample variance. No artificial
/// variance floor converts zero variation into observed information.
pub fn sample_moments(values: &[f64]) -> Result<(f64, f64), StatsError> {
    if values.len() < 2 || values.iter().any(|v| !v.is_finite()) {
        return Err(StatsError::InvalidInput("need at least two finite values"));
    }
    let mut mean = 0.0;
    let mut m2 = 0.0;
    for (i, value) in values.iter().enumerate() {
        let delta = value - mean;
        mean += delta / (i + 1) as f64;
        m2 += delta * (value - mean);
    }
    let variance = m2 / (values.len() - 1) as f64;
    if !mean.is_finite() || !variance.is_finite() || variance < 0.0 {
        return Err(StatsError::NumericalFailure("sample moments overflow"));
    }
    Ok((mean, variance))
}
