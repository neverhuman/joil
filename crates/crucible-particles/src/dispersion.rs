//! Stochastic dispersion controls for anisotropic, inhomogeneous turbulence.
//!
//! These routines are deterministic software fixtures, not a native solver adapter or
//! physical qualification. Covariance inputs are kinematic diffusivities in m²/s.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Matrix3(pub [[f64; 3]; 3]);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RsmDispersion {
    /// Diffusivity tensor derived as R_ij * tau_c, in m²/s.
    pub diffusivity_m2_s: Matrix3,
    pub correlation_time_s: f64,
    /// Diagonal repair applied to the diffusivity for positive definiteness, in m²/s.
    pub realizability_repair_m2_s: f64,
}

/// Convert a Reynolds-stress velocity covariance (m²/s²) into a covariance
/// consistent stochastic diffusivity (m²/s) using an explicit correlation time.
pub fn rsm_covariance_to_dispersion(
    reynolds_stress_m2_s2: Matrix3,
    correlation_time_s: f64,
) -> Result<RsmDispersion, String> {
    if !correlation_time_s.is_finite() || correlation_time_s <= 0.0 {
        return Err("RSM dispersion correlation time must be finite and positive".into());
    }
    let mut diffusivity = reynolds_stress_m2_s2;
    for value in diffusivity.0.iter_mut().flatten() {
        *value *= correlation_time_s;
    }
    let (diffusivity_m2_s, realizability_repair_m2_s) = diffusivity.repair_realizability()?;
    Ok(RsmDispersion {
        diffusivity_m2_s,
        correlation_time_s,
        realizability_repair_m2_s,
    })
}

impl Matrix3 {
    fn cholesky(self) -> Result<[[f64; 3]; 3], String> {
        let a = self.0;
        if !a.iter().flatten().all(|v| v.is_finite()) {
            return Err("dispersion covariance must be finite".into());
        }
        for (i, row) in a.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                if (*value - a[j][i]).abs() > 1e-12 * value.abs().max(a[j][i].abs()).max(1.0) {
                    return Err("dispersion covariance must be symmetric".into());
                }
            }
        }
        let mut l = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..=i {
                let residual = a[i][j] - (0..j).map(|k| l[i][k] * l[j][k]).sum::<f64>();
                if i == j {
                    if residual <= 0.0 {
                        return Err("dispersion covariance is not positive definite".into());
                    }
                    l[i][j] = residual.sqrt();
                } else {
                    l[i][j] = residual / l[j][j];
                }
            }
        }
        Ok(l)
    }

    /// Return a conservative SPD repair by adding the smallest diagonal shift needed
    /// for Cholesky positivity, plus the recorded repair magnitude in m²/s.
    pub fn repair_realizability(self) -> Result<(Self, f64), String> {
        if !self.0.iter().flatten().all(|v| v.is_finite()) {
            return Err("covariance must be finite".into());
        }
        for (i, row) in self.0.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                if (*value - self.0[j][i]).abs()
                    > 1e-10 * value.abs().max(self.0[j][i].abs()).max(1.0)
                {
                    return Err("covariance asymmetry exceeds realizability tolerance".into());
                }
            }
        }
        let mut out = self;
        let mut shift = 0.0;
        for _ in 0..16 {
            if out.cholesky().is_ok() {
                return Ok((out, shift));
            }
            shift = if shift == 0.0 { 1e-12 } else { shift * 10.0 };
            for i in 0..3 {
                out.0[i][i] = self.0[i][i] + shift;
            }
        }
        Err("covariance cannot be repaired within bounded diagonal shift".into())
    }

    pub fn sample_increment(
        self,
        standard_normal: [f64; 3],
        dt_s: f64,
    ) -> Result<[f64; 3], String> {
        if !dt_s.is_finite() || dt_s <= 0.0 || !standard_normal.iter().all(|v| v.is_finite()) {
            return Err("invalid stochastic increment input".into());
        }
        let l = self.cholesky()?;
        let z = [standard_normal[0], standard_normal[1], standard_normal[2]];
        let scale = (2.0 * dt_s).sqrt();
        let result: [f64; 3] =
            std::array::from_fn(|i| scale * (0..=i).map(|j| l[i][j] * z[j]).sum::<f64>());
        if result.iter().any(|v| !v.is_finite()) {
            return Err("stochastic increment overflow".into());
        }
        Ok(result)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LesPartition {
    pub total_tke_m2_s2: f64,
    pub resolved_tke_m2_s2: f64,
    pub unresolved_tke_m2_s2: f64,
}

impl LesPartition {
    pub fn validate(self, modeled_dispersion_tke_m2_s2: f64) -> Result<(), String> {
        if ![
            self.total_tke_m2_s2,
            self.resolved_tke_m2_s2,
            self.unresolved_tke_m2_s2,
            modeled_dispersion_tke_m2_s2,
        ]
        .iter()
        .all(|v| v.is_finite() && *v >= 0.0)
        {
            return Err("LES energy components must be finite and nonnegative".into());
        }
        let tol = 1e-10 * self.total_tke_m2_s2.max(1.0);
        if (self.total_tke_m2_s2 - self.resolved_tke_m2_s2 - self.unresolved_tke_m2_s2).abs() > tol
        {
            return Err("LES resolved plus unresolved energy does not close".into());
        }
        if modeled_dispersion_tke_m2_s2 > self.unresolved_tke_m2_s2 + tol {
            return Err(
                "LES modeled dispersion double counts resolved turbulence or exceeds SGS energy"
                    .into(),
            );
        }
        Ok(())
    }
}

/// An analytic periodic one-dimensional invariant-density fixture. `D(x)` is
/// kinematic diffusivity in m²/s. Uniform density requires Ito drift `dD/dx`.
/// `apply_drift=false` is the intentionally wrong-drift negative control.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeriodicFixture {
    pub tracer_count: usize,
    pub steps: usize,
    pub dt_s: f64,
    pub length_m: f64,
    pub base_diffusivity_m2_s: f64,
    pub modulation: f64,
    pub seed: u64,
}

impl PeriodicFixture {
    /// Run with a declared inlet density perturbation for inlet-sensitivity checks.
    /// `inlet_skew` magnitude below one keeps the initial coordinate map monotone.
    pub fn evolve(self, apply_drift: bool, inlet_skew: f64) -> Result<Vec<f64>, String> {
        let Self {
            tracer_count: count,
            steps,
            dt_s,
            length_m,
            base_diffusivity_m2_s,
            modulation,
            seed,
        } = self;
        if count < 2
            || steps == 0
            || !dt_s.is_finite()
            || dt_s <= 0.0
            || !length_m.is_finite()
            || length_m <= 0.0
            || !base_diffusivity_m2_s.is_finite()
            || base_diffusivity_m2_s <= 0.0
            || !modulation.is_finite()
            || modulation.abs() >= 1.0
            || !inlet_skew.is_finite()
            || inlet_skew.abs() >= 1.0
        {
            return Err("invalid periodic tracer fixture parameters".into());
        }
        let mut x: Vec<f64> = (0..count)
            .map(|i| {
                let u = (i as f64 + 0.5) / count as f64;
                length_m
                    * (u + inlet_skew * (std::f64::consts::TAU * u).sin() / std::f64::consts::TAU)
            })
            .collect();
        let mut state = seed;
        for _ in 0..steps {
            for p in &mut x {
                let phase = std::f64::consts::TAU * *p / length_m;
                let d = base_diffusivity_m2_s * (1.0 + modulation * phase.cos());
                let drift = if apply_drift {
                    -base_diffusivity_m2_s * modulation * std::f64::consts::TAU / length_m
                        * phase.sin()
                } else {
                    0.0
                };
                let z = normal_pair(&mut state).0;
                *p = (*p + drift * dt_s + (2.0 * d * dt_s).sqrt() * z).rem_euclid(length_m);
            }
        }
        Ok(x)
    }
}

fn normal_pair(state: &mut u64) -> (f64, f64) {
    fn uniform(s: &mut u64) -> f64 {
        *s = s.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = *s;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^= z >> 31;
        ((z >> 11) as f64 + 0.5) / ((1u64 << 53) as f64)
    }
    let u1 = uniform(state).max(f64::MIN_POSITIVE);
    let u2 = uniform(state);
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = std::f64::consts::TAU * u2;
    (r * theta.cos(), r * theta.sin())
}

pub fn periodic_density_deviation(
    positions_m: &[f64],
    length_m: f64,
    bins: usize,
) -> Result<f64, String> {
    if positions_m.is_empty()
        || bins < 2
        || !length_m.is_finite()
        || length_m <= 0.0
        || positions_m
            .iter()
            .any(|x| !x.is_finite() || *x < 0.0 || *x >= length_m)
    {
        return Err("invalid invariant-density sample".into());
    }
    let mut counts = vec![0usize; bins];
    for x in positions_m {
        counts[((x / length_m * bins as f64) as usize).min(bins - 1)] += 1;
    }
    let expected = positions_m.len() as f64 / bins as f64;
    Ok(counts
        .iter()
        .map(|c| (*c as f64 - expected).abs())
        .sum::<f64>()
        / (2.0 * positions_m.len() as f64))
}
