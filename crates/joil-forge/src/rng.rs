//! Deterministic SplitMix64 random streams. State is a single `u64`, so a checkpoint
//! restores every stream exactly.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x6A09_E667_F3BC_C909)
    }

    /// Independent stream derived from a seed and a list of tags (generation, index, ...).
    pub fn derive(seed: u64, tags: &[u64]) -> Self {
        let mut r = Rng::new(seed);
        for t in tags {
            r.0 ^= t.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            r.next_u64();
        }
        r
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    /// Log-uniform in `[lo, hi]` (both positive).
    pub fn log_range(&mut self, lo: f64, hi: f64) -> f64 {
        (lo.ln() + (hi.ln() - lo.ln()) * self.f64()).exp()
    }

    pub fn normal(&mut self) -> f64 {
        let u1 = self.f64().max(1e-300);
        let u2 = self.f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    /// Uniform integer in `[0, n)`; `n` must be positive.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }

    pub fn int(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below((hi - lo + 1) as usize) as u32
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.below(items.len())])
        }
    }
}

/// Latin hypercube sample of `n` points in `[0,1]^D`, deterministic in `seed`.
pub fn latin_hypercube<const D: usize>(n: usize, seed: u64) -> Vec<[f64; D]> {
    let mut r = Rng::derive(seed, &[0x0001_A71A]);
    let mut cols: Vec<Vec<f64>> = Vec::with_capacity(D);
    for _ in 0..D {
        let mut perm: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = r.below(i + 1);
            perm.swap(i, j);
        }
        cols.push(
            perm.iter()
                .map(|p| (*p as f64 + r.f64()) / n as f64)
                .collect(),
        );
    }
    (0..n)
        .map(|i| std::array::from_fn(|d| cols[d][i]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_are_reproducible_and_distinct() {
        let mut a = Rng::derive(7, &[1, 2]);
        let mut b = Rng::derive(7, &[1, 2]);
        let mut c = Rng::derive(7, &[2, 1]);
        let (x, y, z) = (a.next_u64(), b.next_u64(), c.next_u64());
        assert_eq!(x, y);
        assert_ne!(x, z);
    }

    #[test]
    fn lhs_strata_are_filled() {
        let pts = latin_hypercube::<3>(8, 11);
        for d in 0..3 {
            let mut bins: Vec<usize> = pts.iter().map(|p| (p[d] * 8.0) as usize).collect();
            bins.sort();
            assert_eq!(bins, (0..8).collect::<Vec<_>>());
        }
    }
}
