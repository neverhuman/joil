//! Droplet size classes on an oil-mass basis.

use crate::fluid::Lognormal;
use crate::math::normal_cdf;
use crate::{JudgeError, Result};
use serde::{Deserialize, Serialize};

/// Logarithmically spaced size classes. Mass is carried per class; the representative
/// diameter is the geometric mean of the class edges.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SizeGrid {
    pub edges_m: Vec<f64>,
    pub d_m: Vec<f64>,
}

impl SizeGrid {
    /// `n` classes between `lo_um` and `hi_um`.
    pub fn log_spaced(lo_um: f64, hi_um: f64, n: usize) -> Result<Self> {
        if !(lo_um > 0.0 && hi_um > lo_um && n >= 4) {
            return Err(JudgeError::Input(
                "size grid needs 0 < lo < hi and n >= 4".into(),
            ));
        }
        let ratio = (hi_um / lo_um).powf(1.0 / n as f64);
        let edges_m: Vec<f64> = (0..=n)
            .map(|i| lo_um * ratio.powi(i as i32) * 1e-6)
            .collect();
        let d_m = edges_m.windows(2).map(|w| (w[0] * w[1]).sqrt()).collect();
        Ok(Self { edges_m, d_m })
    }

    /// Default forge grid: 30 classes from 0.5 to 400 µm (ratio ≈ 1.25 per class).
    pub fn standard() -> Self {
        Self::log_spaced(0.5, 400.0, 30).expect("static grid is valid")
    }

    pub fn n(&self) -> usize {
        self.d_m.len()
    }

    /// Index of the class whose edges contain `d` (clamped to the grid).
    pub fn class_of(&self, d_m: f64) -> usize {
        let n = self.n();
        if d_m <= self.edges_m[0] {
            return 0;
        }
        if d_m >= self.edges_m[n] {
            return n - 1;
        }
        let ratio = self.edges_m[1] / self.edges_m[0];
        let i = ((d_m / self.edges_m[0]).ln() / ratio.ln()).floor() as usize;
        i.min(n - 1)
    }

    /// Mass fractions of a lognormal on this grid. Tail mass below the first edge is added
    /// to the first class and above the last edge to the last class, so fractions sum to 1.
    pub fn lognormal_fractions(&self, dsd: &Lognormal) -> Vec<f64> {
        let mu = (dsd.volume_median_um * 1e-6).ln();
        let s = dsd.geometric_sd.ln().max(1e-6);
        let cdf: Vec<f64> = self
            .edges_m
            .iter()
            .map(|e| normal_cdf((e.ln() - mu) / s))
            .collect();
        let n = self.n();
        let mut w: Vec<f64> = (0..n).map(|i| (cdf[i + 1] - cdf[i]).max(0.0)).collect();
        w[0] += cdf[0];
        w[n - 1] += 1.0 - cdf[n];
        let total: f64 = w.iter().sum();
        w.iter_mut().for_each(|v| *v /= total);
        w
    }

    /// Mass-weighted diameter quantile (`q` in [0,1]) by log-linear interpolation of the
    /// cumulative distribution across class edges. `None` for an empty distribution.
    pub fn quantile(&self, mass: &[f64], q: f64) -> Option<f64> {
        let total: f64 = mass.iter().sum();
        if total <= 0.0 || !total.is_finite() {
            return None;
        }
        let target = q.clamp(0.0, 1.0) * total;
        let mut cum = 0.0;
        for (i, m) in mass.iter().enumerate() {
            if cum + m >= target && *m > 0.0 {
                let f = ((target - cum) / m).clamp(0.0, 1.0);
                let (a, b) = (self.edges_m[i].ln(), self.edges_m[i + 1].ln());
                return Some((a + f * (b - a)).exp());
            }
            cum += m;
        }
        Some(*self.edges_m.last().expect("grid has edges"))
    }
}

/// Diameter where the reduced grade efficiency `(E - r)/(1 - r)` crosses `level`,
/// interpolated in `ln d`. `None` if the curve never crosses.
pub fn reduced_cut_size(grid: &SizeGrid, capture: &[f64], split: f64, level: f64) -> Option<f64> {
    if split >= 1.0 {
        return None;
    }
    let reduced: Vec<f64> = capture
        .iter()
        .map(|e| (e - split) / (1.0 - split))
        .collect();
    for i in 1..reduced.len() {
        let (a, b) = (reduced[i - 1], reduced[i]);
        if a < level && b >= level {
            let f = (level - a) / (b - a).max(1e-300);
            let (la, lb) = (grid.d_m[i - 1].ln(), grid.d_m[i].ln());
            return Some((la + f * (lb - la)).exp());
        }
    }
    if reduced.first().is_some_and(|v| *v >= level) {
        return Some(grid.d_m[0]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lognormal_median_is_recovered() {
        let g = SizeGrid::standard();
        let dsd = Lognormal {
            volume_median_um: 15.0,
            geometric_sd: 1.6,
        };
        let w = g.lognormal_fractions(&dsd);
        assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        let d50 = g.quantile(&w, 0.5).unwrap() * 1e6;
        assert!((d50 - 15.0).abs() < 0.3, "d50 {d50}");
    }

    #[test]
    fn class_lookup_is_consistent() {
        let g = SizeGrid::standard();
        for (i, d) in g.d_m.iter().enumerate() {
            assert_eq!(g.class_of(*d), i);
        }
        assert_eq!(g.class_of(1e-9), 0);
        assert_eq!(g.class_of(1.0), g.n() - 1);
    }
}
