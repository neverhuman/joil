//! Small dependency-free numerical kernels.

use crate::{JudgeError, Result};

/// Error function, Abramowitz & Stegun 7.1.26 (absolute error below 1.5e-7).
pub fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let poly = t
        * (0.254_829_592
            + t * (-0.284_496_736
                + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    sign * (1.0 - poly * (-x * x).exp())
}

/// Standard normal cumulative distribution function.
pub fn normal_cdf(z: f64) -> f64 {
    0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2))
}

/// Solve a tridiagonal system in place with the Thomas algorithm.
///
/// `lower[i]` multiplies `x[i-1]`, `diag[i]` multiplies `x[i]`, `upper[i]` multiplies
/// `x[i+1]`. The right-hand side is overwritten with the solution. The matrices built by
/// the separation kernel are diagonally dominant M-matrices, for which no pivoting is needed.
pub fn thomas(lower: &[f64], diag: &[f64], upper: &[f64], rhs: &mut [f64], scratch: &mut [f64]) {
    let n = diag.len();
    debug_assert!(lower.len() == n && upper.len() == n && rhs.len() == n && scratch.len() >= n);
    let mut beta = diag[0];
    rhs[0] /= beta;
    for i in 1..n {
        scratch[i] = upper[i - 1] / beta;
        beta = diag[i] - lower[i] * scratch[i];
        rhs[i] = (rhs[i] - lower[i] * rhs[i - 1]) / beta;
    }
    for i in (0..n - 1).rev() {
        rhs[i] -= scratch[i + 1] * rhs[i + 1];
    }
}

/// Solve a dense linear system `a x = b` by Gaussian elimination with partial pivoting.
/// `a` is row-major `n x n`.
pub fn dense_solve(mut a: Vec<f64>, mut b: Vec<f64>) -> Result<Vec<f64>> {
    let n = b.len();
    if a.len() != n * n {
        return Err(JudgeError::Input("dense_solve dimension mismatch".into()));
    }
    let scale = a.iter().fold(0.0_f64, |m, v| m.max(v.abs())).max(1.0);
    for col in 0..n {
        let (piv, pmax) =
            (col..n)
                .map(|r| (r, a[r * n + col].abs()))
                .fold(
                    (col, -1.0),
                    |best, cur| if cur.1 > best.1 { cur } else { best },
                );
        if pmax <= 1e-13 * scale {
            return Err(JudgeError::Hydraulic(
                "singular flow network: a closed loop retains flow with no exit".into(),
            ));
        }
        if piv != col {
            for k in 0..n {
                a.swap(col * n + k, piv * n + k);
            }
            b.swap(col, piv);
        }
        let d = a[col * n + col];
        for r in (col + 1)..n {
            let f = a[r * n + col] / d;
            if f != 0.0 {
                for k in col..n {
                    a[r * n + k] -= f * a[col * n + k];
                }
                b[r] -= f * b[col];
            }
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let mut s = b[r];
        for k in (r + 1)..n {
            s -= a[r * n + k] * x[k];
        }
        x[r] = s / a[r * n + r];
    }
    if x.iter().all(|v| v.is_finite()) {
        Ok(x)
    } else {
        Err(JudgeError::Numerical(
            "dense solve produced non-finite values".into(),
        ))
    }
}

/// Composite Simpson integration of `f` on `[a, b]` with an even number of panels.
pub fn simpson(f: impl Fn(f64) -> f64, a: f64, b: f64, panels: usize) -> f64 {
    let n = panels.max(2) + panels % 2;
    let h = (b - a) / n as f64;
    let mut s = f(a) + f(b);
    for i in 1..n {
        let w = if i % 2 == 1 { 4.0 } else { 2.0 };
        s += w * f(a + h * i as f64);
    }
    s * h / 3.0
}

/// Darcy friction factor from the explicit Haaland correlation, with a laminar branch.
pub fn darcy_friction(re: f64, rel_roughness: f64) -> f64 {
    let re = re.max(1.0);
    if re < 2300.0 {
        return 64.0 / re;
    }
    let t = (rel_roughness.max(0.0) / 3.7).powf(1.11) + 6.9 / re;
    let inv_sqrt = -1.8 * t.log10();
    1.0 / (inv_sqrt * inv_sqrt)
}

/// `∫_0^U (1 - e^{-u})² / u² du`, the Burgers-vortex core pressure integral.
/// Tends to `2 ln 2` for large `U`.
pub fn burgers_core_integral(u_max: f64) -> f64 {
    // Integrate in log space for accuracy near zero and over decades.
    let lo = 1e-6_f64;
    if u_max <= lo {
        return u_max; // integrand -> 1 as u -> 0
    }
    let g = |t: f64| {
        let u = t.exp();
        let one_minus = -(-u).exp_m1();
        one_minus * one_minus / u // (1-e^-u)^2/u^2 * u (Jacobian du = u dt)
    };
    lo + simpson(g, lo.ln(), u_max.ln(), 400)
}

/// `∫_0^U (1 - e^{-u})² / u du`, used for the area-averaged swirl energy of a Burgers vortex.
pub fn burgers_energy_integral(u_max: f64) -> f64 {
    let lo = 1e-6_f64;
    if u_max <= lo {
        return 0.5 * u_max * u_max;
    }
    let g = |t: f64| {
        let u = t.exp();
        let one_minus = -(-u).exp_m1();
        one_minus * one_minus // (1-e^-u)^2/u * u
    };
    simpson(g, lo.ln(), u_max.ln(), 400)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erf_reference_values() {
        assert!((erf(0.5) - 0.520_499_877_8).abs() < 2e-7);
        assert!((erf(-1.0) + 0.842_700_792_9).abs() < 2e-7);
        assert!((normal_cdf(1.96) - 0.975_002_1).abs() < 2e-6);
    }

    #[test]
    fn thomas_matches_dense() {
        let lower = [0.0, -1.0, -1.0, -1.0];
        let diag = [2.0, 2.0, 2.0, 2.0];
        let upper = [-1.0, -1.0, -1.0, 0.0];
        let mut rhs = [1.0, 0.0, 0.0, 1.0];
        let mut scratch = [0.0; 4];
        thomas(&lower, &diag, &upper, &mut rhs, &mut scratch);
        for v in rhs {
            assert!((v - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn burgers_integrals_have_known_limits() {
        assert!((burgers_core_integral(1e4) - 2.0 * std::f64::consts::LN_2).abs() < 2e-3);
        let e = burgers_energy_integral(100.0);
        // ∫ (1-e^-u)^2/u du grows like ln U; check finite, positive, monotone.
        assert!(e > burgers_energy_integral(10.0) && e.is_finite());
    }

    #[test]
    fn singular_loop_is_hydraulic_error() {
        let err = dense_solve(vec![1.0, -1.0, -1.0, 1.0], vec![1.0, 0.0]).unwrap_err();
        assert!(matches!(err, JudgeError::Hydraulic(_)));
    }
}
