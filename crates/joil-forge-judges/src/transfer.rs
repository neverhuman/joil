//! Sparse size-class transfer operators.
//!
//! A [`Transfer`] maps an inlet oil-mass vector (per size class) to an outlet vector.
//! Column `i` lists `(output class, fraction of inlet class i mass)`. A breakup step is
//! column-stochastic (each column sums to one); a separation port scales columns by its
//! capture fraction. Daughters are never new injected mass.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transfer {
    pub cols: Vec<Vec<(usize, f64)>>,
}

impl Transfer {
    pub fn identity(n: usize) -> Self {
        Self {
            cols: (0..n).map(|i| vec![(i, 1.0)]).collect(),
        }
    }

    pub fn diagonal(scale: &[f64]) -> Self {
        Self {
            cols: scale
                .iter()
                .enumerate()
                .map(|(i, s)| vec![(i, *s)])
                .collect(),
        }
    }

    pub fn n(&self) -> usize {
        self.cols.len()
    }

    /// `out += self * input`.
    pub fn apply_add(&self, input: &[f64], out: &mut [f64]) {
        for (i, col) in self.cols.iter().enumerate() {
            let m = input[i];
            if m == 0.0 {
                continue;
            }
            for (j, f) in col {
                out[*j] += f * m;
            }
        }
    }

    pub fn apply(&self, input: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n()];
        self.apply_add(input, &mut out);
        out
    }

    /// `after ∘ self`: first apply `self`, then `after`.
    pub fn then(&self, after: &Transfer) -> Transfer {
        let n = self.n();
        let mut cols = Vec::with_capacity(n);
        for col in &self.cols {
            let mut acc = vec![0.0; n];
            for (j, f) in col {
                for (k, g) in &after.cols[*j] {
                    acc[*k] += f * g;
                }
            }
            cols.push(
                acc.into_iter()
                    .enumerate()
                    .filter(|(_, v)| *v != 0.0)
                    .collect(),
            );
        }
        Transfer { cols }
    }

    /// Multiply every entry by `s`.
    pub fn scaled(&self, s: f64) -> Transfer {
        Transfer {
            cols: self
                .cols
                .iter()
                .map(|c| c.iter().map(|(j, f)| (*j, f * s)).collect())
                .collect(),
        }
    }

    /// Row-scale the output by a per-class factor: `diag(scale) * self`.
    pub fn scale_outputs(&self, scale: &[f64]) -> Transfer {
        Transfer {
            cols: self
                .cols
                .iter()
                .map(|c| c.iter().map(|(j, f)| (*j, f * scale[*j])).collect())
                .collect(),
        }
    }

    /// Column sums (fraction of each inlet class that leaves through this operator).
    pub fn column_sums(&self) -> Vec<f64> {
        self.cols
            .iter()
            .map(|c| c.iter().map(|(_, f)| f).sum())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_conserves_stochastic_columns() {
        let a = Transfer {
            cols: vec![vec![(0, 1.0)], vec![(0, 0.5), (1, 0.5)], vec![(1, 1.0)]],
        };
        let c = a.then(&a);
        for s in c.column_sums() {
            assert!((s - 1.0).abs() < 1e-15);
        }
        let m = c.apply(&[1.0, 2.0, 3.0]);
        assert!((m.iter().sum::<f64>() - 6.0).abs() < 1e-12);
    }
}
