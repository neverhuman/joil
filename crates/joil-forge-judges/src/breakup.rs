//! Turbulent drop breakup.
//!
//! Maximum stable diameter (Davies 1985, Hinze–Kolmogorov with a dispersed-phase viscous
//! resistance term):
//!
//! `d_max = C [ (σ + 0.08 μ_d ε^{1/3} d_max^{1/3}) / ρ_c ]^{3/5} ε^{-2/5}`
//!
//! valid when `d_max` exceeds the Kolmogorov scale `η = (ν³/ε)^{1/4}`. Below it the drop
//! sits in the viscous subrange and the critical capillary number criterion
//! `d_max = 2 Ca_c σ / (μ_c γ)`, `γ = (ε/ν)^{1/2}`, `Ca_c = 0.5`, is used instead.
//! Breakup is applied as an equal-volume split into the fewest fragments that are
//! stable, plus a satellite fraction at `d_max / 6`. Mass moves between classes; it is
//! never created or lost.

use crate::fluid::Fluids;
use crate::sizes::SizeGrid;
use crate::transfer::Transfer;

/// Maximum stable drop diameter (m) at dissipation rate `eps` (W/kg). Infinite for `eps <= 0`.
pub fn max_stable_diameter(eps: f64, fluids: &Fluids, davies_c: f64) -> f64 {
    if eps <= 0.0 || !eps.is_finite() {
        return f64::INFINITY;
    }
    let rho = fluids.rho_water_kg_m3;
    let sigma = fluids.interfacial_tension_n_m;
    let mu_d = fluids.mu_oil_pa_s;
    let e13 = eps.cbrt();
    let mut d = davies_c * (sigma / rho).powf(0.6) * eps.powf(-0.4);
    for _ in 0..40 {
        let next =
            davies_c * ((sigma + 0.08 * mu_d * e13 * d.cbrt()) / rho).powf(0.6) * eps.powf(-0.4);
        if (next - d).abs() <= 1e-12 * d {
            d = next;
            break;
        }
        d = next;
    }
    let nu = fluids.nu_water();
    let eta_k = (nu * nu * nu / eps).powf(0.25);
    if d >= eta_k {
        d
    } else {
        let gamma = (eps / nu).sqrt();
        (2.0 * 0.5 * sigma / (fluids.mu_water_pa_s * gamma)).min(eta_k)
    }
}

/// Column-stochastic breakup operator for a stable limit `d_max`.
pub fn breakup_transfer(grid: &SizeGrid, d_max: f64, satellite_fraction: f64) -> Transfer {
    let n = grid.n();
    if !d_max.is_finite() {
        return Transfer::identity(n);
    }
    let sat = satellite_fraction.clamp(0.0, 0.5);
    let sat_class = grid.class_of(d_max / 6.0);
    let cols = grid
        .d_m
        .iter()
        .enumerate()
        .map(|(i, d)| {
            if *d <= d_max {
                return vec![(i, 1.0)];
            }
            let fragments = (d / d_max).powi(3).ceil().max(2.0);
            let child = grid.class_of(d / fragments.cbrt()).min(i.saturating_sub(1));
            if sat > 0.0 && sat_class != child {
                vec![(child, 1.0 - sat), (sat_class, sat)]
            } else {
                vec![(child, 1.0)]
            }
        })
        .collect();
    Transfer { cols }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmax_decreases_with_dissipation() {
        let f = Fluids::reference();
        let a = max_stable_diameter(100.0, &f, 0.66);
        let b = max_stable_diameter(1.0e4, &f, 0.66);
        assert!(a > b && b > 0.0);
        // Pure Hinze scaling within the inertial range: ratio ≈ 100^0.4 ≈ 6.3.
        let r = a / b;
        assert!(r > 5.0 && r < 7.5, "ratio {r}");
        assert!(max_stable_diameter(0.0, &f, 0.66).is_infinite());
    }

    #[test]
    fn breakup_conserves_mass_and_shrinks() {
        let g = SizeGrid::standard();
        let t = breakup_transfer(&g, 20e-6, 0.1);
        for s in t.column_sums() {
            assert!((s - 1.0).abs() < 1e-15);
        }
        let mut m = vec![0.0; g.n()];
        let big = g.class_of(100e-6);
        m[big] = 1.0;
        let out = t.apply(&m);
        let top = out.iter().rposition(|v| *v > 0.0).unwrap();
        assert!(g.d_m[top] <= 20e-6 * 1.26, "fragment class {}", g.d_m[top]);
    }
}
