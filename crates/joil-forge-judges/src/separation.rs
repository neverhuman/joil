//! Size-resolved separation in a swirl cell by radial migration and turbulent diffusion.
//!
//! The forward (downward) flow occupies the annulus `r_cap(z) ≤ r ≤ R(z)` with
//! `r_cap = λR`; the reverse core inside `r_cap` carries the overflow. We march along `z`
//! in the flow-fraction coordinate `s = (r² - r_cap²)/(R² - r_cap²) ∈ [0, 1]` with
//! `n(s, z)` = oil mass flux per unit `s` of one size class:
//!
//! `∂n/∂z = -∂/∂s[(a_f + a_d) n] + ∂/∂s[K ∂n/∂s]`
//!
//! * fluid drift toward the core feeding the overflow: `a_f = -(1-s) q'/Q_f`, `q' = rQ/L`;
//! * drop slip (centrifugal buoyancy, Stokes with Schiller–Naumann and an optional
//!   Hadamard–Rybczynski interface mobility): `a_d = -2π r v_slip / Q_f`;
//! * turbulent diffusion: `K = 4π r² D_t / ((R² - r_cap²) Q_f)`, `D_t = c Γ(z)`.
//!
//! The core boundary `s = 0` is an advective outflow (captured mass); the wall `s = 1`
//! has zero flux. Implicit upwind finite volumes conserve mass to round-off, and with the
//! implicit forward flow a passive tracer reports exactly `r` to the overflow.

use crate::closure::Closure;
use crate::cyclone::{CycloneHydraulics, CycloneSpec};
use crate::fluid::Fluids;
use crate::math::thomas;
use crate::sizes::SizeGrid;
use crate::{JudgeError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeparationResult {
    /// Fraction of each class's mass (entering the separation zone) that reports to overflow.
    pub capture: Vec<f64>,
    /// Same march for a passive tracer (should equal the split).
    pub tracer_capture: f64,
    /// Largest per-class `|captured + remaining - 1|`.
    pub mass_error: f64,
}

/// Terminal slip speed (m/s) of a drop of diameter `d` in centripetal acceleration `a`.
fn slip_speed(stokes_coeff: f64, a: f64, d: f64, rho: f64, mu: f64) -> f64 {
    let v_st = stokes_coeff * a;
    let re_st = rho * v_st * d / mu;
    if re_st < 0.05 {
        return v_st;
    }
    // Newton on v (1 + 0.15 Re^0.687) = v_st.
    let mut v = v_st / (1.0 + 0.15 * re_st.powf(0.687));
    for _ in 0..4 {
        let re = rho * v * d / mu;
        let g = v * (1.0 + 0.15 * re.powf(0.687)) - v_st;
        let dg = 1.0 + 0.15 * 1.687 * re.powf(0.687);
        v = (v - g / dg).max(0.0);
    }
    v
}

/// Run the separation march of one cell for every size class.
pub fn separate(
    spec: &CycloneSpec,
    hyd: &CycloneHydraulics,
    grid: &SizeGrid,
    fluids: &Fluids,
    closure: &Closure,
    ns: usize,
) -> Result<SeparationResult> {
    let ns = ns.max(4);
    let pi = std::f64::consts::PI;
    let rho = fluids.rho_water_kg_m3;
    let mu = fluids.mu_water_pa_s;
    let kmu = fluids.mu_oil_pa_s / mu;
    let hadamard = 3.0 * (1.0 + kmu) / (2.0 + 3.0 * kmu);
    let mobility = 1.0 + closure.interface_mobility.clamp(0.0, 1.0) * (hadamard - 1.0);
    let lam = closure.capture_radius_frac;
    let kappa = closure.core_radius_frac;
    let q_over = spec.split * hyd.q_cell_m3_s;
    let q_prime = q_over / spec.length();
    let ds = 1.0 / ns as f64;
    let nclass = grid.n();
    // classes plus one passive tracer
    let ncol = nclass + 1;
    let stokes: Vec<f64> = grid
        .d_m
        .iter()
        .map(|d| fluids.delta_rho() * d * d / (18.0 * mu) * mobility)
        .collect();

    let mut n = vec![1.0_f64; ncol * ns];
    let mut captured = vec![0.0_f64; ncol];
    // Classes whose remaining mass is negligible stop marching (their capture is final).
    let mut alive = vec![true; ncol];
    let (mut r_face, mut a_f, mut k_face, mut acc) = (
        vec![0.0; ns + 1],
        vec![0.0; ns + 1],
        vec![0.0; ns + 1],
        vec![0.0; ns + 1],
    );
    let (mut lower, mut diag, mut upper, mut scratch) =
        (vec![0.0; ns], vec![0.0; ns], vec![0.0; ns], vec![0.0; ns]);
    let (mut p, mut m) = (vec![0.0; ns + 1], vec![0.0; ns + 1]);
    let sw = &hyd.swirl;
    for k in 0..sw.z_m.len() - 1 {
        let dz = sw.z_m[k + 1] - sw.z_m[k];
        let r_w = sw.r_wall_m[k + 1];
        let gamma = sw.gamma_m2_s[k + 1];
        let qf = sw.q_forward_m3_s[k + 1];
        let r_cap = lam * r_w;
        let span = r_w * r_w - r_cap * r_cap;
        let rc = kappa * r_w;
        let d_t = closure.eddy_diffusivity_coeff * gamma;
        for j in 0..=ns {
            let s = j as f64 * ds;
            let r = (r_cap * r_cap + s * span).sqrt();
            r_face[j] = r;
            a_f[j] = -(1.0 - s) * q_prime / qf;
            k_face[j] = 4.0 * pi * r * r * d_t / (span * qf);
            let vt = gamma / r * (-(-(r * r) / (rc * rc)).exp_m1());
            acc[j] = vt * vt / r;
        }
        let c = dz / ds;
        for col in 0..ncol {
            if !alive[col] {
                continue;
            }
            for j in 0..=ns {
                let slip = if col < nclass {
                    slip_speed(stokes[col], acc[j], grid.d_m[col], rho, mu)
                } else {
                    0.0
                };
                let a = a_f[j] - 2.0 * pi * r_face[j] * slip / qf;
                let kd = if j == 0 || j == ns {
                    0.0
                } else {
                    k_face[j] / ds
                };
                if j == ns {
                    p[j] = 0.0;
                    m[j] = 0.0;
                } else if j == 0 {
                    p[j] = 0.0;
                    m[j] = a.min(0.0);
                } else {
                    p[j] = a.max(0.0) + kd;
                    m[j] = a.min(0.0) - kd;
                }
            }
            for i in 0..ns {
                lower[i] = -c * p[i];
                diag[i] = 1.0 + c * (p[i + 1] - m[i]);
                upper[i] = c * m[i + 1];
            }
            let cells = &mut n[col * ns..(col + 1) * ns];
            thomas(&lower, &diag, &upper, cells, &mut scratch);
            captured[col] += -m[0] * cells[0] * dz;
            if cells.iter().sum::<f64>() * ds < 1e-12 {
                alive[col] = false;
            }
        }
    }
    let mut capture = Vec::with_capacity(nclass);
    let mut mass_error: f64 = 0.0;
    for col in 0..ncol {
        let remaining: f64 = n[col * ns..(col + 1) * ns].iter().sum::<f64>() * ds;
        let total = captured[col] + remaining;
        if !total.is_finite() {
            return Err(JudgeError::Numerical(
                "separation march produced non-finite mass".into(),
            ));
        }
        mass_error = mass_error.max((total - 1.0).abs());
        if col < nclass {
            capture.push(captured[col].clamp(0.0, 1.0));
        }
    }
    if mass_error > 1e-8 {
        return Err(JudgeError::Numerical(format!(
            "separation mass balance error {mass_error:e}"
        )));
    }
    Ok(SeparationResult {
        capture,
        tracer_capture: captured[nclass],
        mass_error,
    })
}
