//! Exact dependency pin, without the optional random or linear-algebra features.
//! Independent table checks cover the quantiles used by the v5 planning fixtures.
use crate::StatsError;
use statrs::distribution::{ContinuousCDF, Normal, StudentsT};

pub const DISTRIBUTION_IMPLEMENTATION: &str = "statrs/0.18.0";

fn probability(p: f64) -> Result<(), StatsError> {
    if !p.is_finite() || p <= 0.0 || p >= 1.0 {
        return Err(StatsError::InvalidInput("probability must be in (0,1)"));
    }
    Ok(())
}

pub fn student_t_quantile(p: f64, degrees_of_freedom: usize) -> Result<f64, StatsError> {
    probability(p)?;
    // Integers above 2^53 cannot be represented exactly by this f64 routine.
    if degrees_of_freedom == 0 || degrees_of_freedom as u128 > (1_u128 << 53) {
        return Err(StatsError::InvalidInput("unsupported degrees of freedom"));
    }
    let distribution = StudentsT::new(0.0, 1.0, degrees_of_freedom as f64)
        .map_err(|_| StatsError::InvalidInput("invalid t distribution"))?;
    let value = distribution.inverse_cdf(p);
    if !value.is_finite() {
        return Err(StatsError::NumericalFailure("nonfinite t quantile"));
    }
    Ok(value)
}

pub fn normal_quantile(p: f64) -> Result<f64, StatsError> {
    probability(p)?;
    let value = Normal::new(0.0, 1.0)
        .map_err(|_| StatsError::InvalidInput("invalid normal distribution"))?
        .inverse_cdf(p);
    if !value.is_finite() {
        return Err(StatsError::NumericalFailure("nonfinite normal quantile"));
    }
    Ok(value)
}
