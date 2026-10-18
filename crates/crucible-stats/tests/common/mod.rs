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

/// Preregistered synthetic generator: SplitMix64 with 53-bit uniforms and Box-Muller normals.
pub struct SplitMix64(pub u64);
impl SplitMix64 {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    pub fn normal(&mut self) -> f64 {
        let u1 = 1.0 - self.uniform(); // (0, 1]
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

pub fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Binomial Monte Carlo standard error of an observed rate.
pub fn monte_carlo_se(rate: f64, trials: usize) -> f64 {
    (rate * (1.0 - rate) / trials as f64).sqrt()
}

/// Closed-form Cauchy (t with one degree of freedom) quantile.
pub fn cauchy_quantile(p: f64) -> f64 {
    (std::f64::consts::PI * (p - 0.5)).tan()
}
