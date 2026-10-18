//! Shared V5-04 test support: fixture loading and oracles computed by a different
//! method from the code under test. Synthetic values only.
#![allow(dead_code)]
use statrs::distribution::{ChiSquared, Continuous, ContinuousCDF, Normal};
use std::path::PathBuf;

pub fn fixture(name: &str) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/v5-statistics")
        .join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_slice(&bytes).unwrap()
}

pub fn f64s(value: &serde_json::Value) -> Vec<f64> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

pub fn strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

/// Exact power of the one-sided one-sample t test, P(T'(n-1, sqrt(n)*standardized) > c),
/// by integrating the conditional normal tail over the chi-square scale variable
/// (composite Simpson). Independent of the central-t planning iteration.
pub fn noncentral_t_power(n: usize, critical: f64, standardized_effect: f64) -> f64 {
    let df = (n - 1) as f64;
    let lambda = (n as f64).sqrt() * standardized_effect;
    let chi = ChiSquared::new(df).unwrap();
    let normal = Normal::new(0.0, 1.0).unwrap();
    let upper = df + 60.0 * (2.0 * df).sqrt() + 60.0;
    let steps = 40_000usize;
    let h = upper / steps as f64;
    let integrand = |v: f64| {
        if v <= 0.0 {
            return 0.0;
        }
        (1.0 - normal.cdf(critical * (v / df).sqrt() - lambda)) * chi.pdf(v)
    };
    let mut sum = integrand(0.0) + integrand(upper);
    for i in 1..steps {
        let w = if i % 2 == 1 { 4.0 } else { 2.0 };
        sum += w * integrand(i as f64 * h);
    }
    sum * h / 3.0
}

/// Closed-form Cauchy (t with one degree of freedom) quantile.
pub fn cauchy_quantile(p: f64) -> f64 {
    (std::f64::consts::PI * (p - 0.5)).tan()
}
