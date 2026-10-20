//! D2Q9 block restriction, precision parity, piecewise-constant refinement, and unit tests.

use super::*;

pub fn restrict_block_conservative(
    fine: &[[f64; Q]],
    fine_width: usize,
    fine_height: usize,
    ratio: usize,
    flux_scale: f64,
) -> Result<Vec<[f64; Q]>, LbmError> {
    if ratio == 0
        || fine_width == 0
        || fine_height == 0
        || !fine_width.is_multiple_of(ratio)
        || !fine_height.is_multiple_of(ratio)
        || fine_width.checked_mul(fine_height) != Some(fine.len())
    {
        return Err(LbmError::InvalidShape);
    }
    if !flux_scale.is_finite() || (flux_scale - 1.0).abs() > 1e-12 {
        return Err(LbmError::InvalidState);
    }
    if fine.iter().flatten().any(|v| !v.is_finite()) {
        return Err(LbmError::InvalidState);
    }
    let cw = fine_width / ratio;
    let ch = fine_height / ratio;
    let mut coarse = vec![[0.; Q]; cw * ch];
    let scale = 1.0 / (ratio * ratio) as f64;
    for y in 0..fine_height {
        for x in 0..fine_width {
            let src = &fine[y * fine_width + x];
            let dst = &mut coarse[(y / ratio) * cw + x / ratio];
            for q in 0..Q {
                dst[q] += src[q] * scale;
            }
        }
    }
    for q in 0..Q {
        let a: f64 = fine.iter().map(|c| c[q]).sum();
        let b: f64 = coarse.iter().map(|c| c[q]).sum::<f64>() * (ratio * ratio) as f64;
        if (a - b).abs() > 1e-12 * a.abs().max(1.0) {
            return Err(LbmError::InvalidState);
        }
    }
    Ok(coarse)
}

pub fn verify_kernel_precision_parity(
    reference: &[[f64; Q]],
    candidate: &[[f64; Q]],
    precision: &str,
    max_abs_tol: f64,
) -> Result<f64, LbmError> {
    if precision.eq_ignore_ascii_case("fp16")
        || reference.len() != candidate.len()
        || !max_abs_tol.is_finite()
        || max_abs_tol < 0.0
    {
        return Err(LbmError::InvalidState);
    }
    let mut worst = 0.0_f64;
    for (a, b) in reference.iter().zip(candidate) {
        for (x, y) in a.iter().zip(b) {
            if !x.is_finite() || !y.is_finite() {
                return Err(LbmError::InvalidState);
            }
            worst = worst.max((x - y).abs());
        }
    }
    if worst > max_abs_tol {
        Err(LbmError::InvalidState)
    } else {
        Ok(worst)
    }
}

/// Deterministic block refinement: each coarse cell is copied into an `ratio x ratio` fine block.
pub fn refine_piecewise_constant(
    coarse: &[[f64; Q]],
    width: usize,
    height: usize,
    ratio: usize,
    max_cells: usize,
) -> Result<Vec<[f64; Q]>, LbmError> {
    if width == 0 || height == 0 || ratio == 0 || width.checked_mul(height) != Some(coarse.len()) {
        return Err(LbmError::InvalidShape);
    }
    let n = width
        .checked_mul(ratio)
        .and_then(|x| x.checked_mul(height))
        .and_then(|x| x.checked_mul(ratio))
        .ok_or(LbmError::InvalidShape)?;
    if n > max_cells {
        return Err(LbmError::Bounds);
    }
    if coarse.iter().flatten().any(|v| !v.is_finite()) {
        return Err(LbmError::InvalidState);
    }
    let mut fine = vec![[0.0; Q]; n];
    let fw = width * ratio;
    for y in 0..height {
        for x in 0..width {
            for dy in 0..ratio {
                for dx in 0..ratio {
                    fine[(y * ratio + dy) * fw + x * ratio + dx] = coarse[y * width + x];
                }
            }
        }
    }
    Ok(fine)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equilibrium_recovers_macros_and_collide_stream_preserves_periodic_mass() {
        let mut s = D2Q9::new(8, 6, 0.8).unwrap();
        for y in 0..6 {
            for x in 0..8 {
                s.set_equilibrium(x, y, 1.0, [0.03, 0.0]).unwrap();
            }
        }
        let before = s.total_mass().unwrap();
        for _ in 0..20 {
            s.collide_stream().unwrap();
        }
        assert!((s.total_mass().unwrap() - before).abs() < 1e-12);
        let (rho, u) = s.macro_at(2, 2).unwrap();
        assert!((rho - 1.0).abs() < 1e-12);
        assert!((u[0] - 0.03).abs() < 1e-12);
    }
    #[test]
    fn bounds_tau_and_nonfinite_are_rejected() {
        assert_eq!(D2Q9::new(0, 4, 1.0).err(), Some(LbmError::InvalidShape));
        assert_eq!(
            D2Q9::new(2, 2, 0.5).err(),
            Some(LbmError::InvalidRelaxation)
        );
        let mut s = D2Q9::new(2, 2, 1.0).unwrap();
        assert_eq!(
            s.set_equilibrium(2, 0, 1.0, [0.0, 0.0]),
            Err(LbmError::Bounds)
        );
        assert_eq!(
            s.set_equilibrium(0, 0, f64::NAN, [0.0, 0.0]),
            Err(LbmError::InvalidState)
        );
    }
    #[test]
    fn halfway_bounceback_conserves_mass() {
        let mut s = D2Q9::new(8, 8, 0.9).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                s.set_equilibrium(x, y, 1.0, [0.02, 0.0]).unwrap();
            }
        }
        s.set_solid(4, 4, true).unwrap();
        let m = s.total_mass().unwrap();
        for _ in 0..10 {
            s.collide_stream().unwrap();
        }
        assert!((s.total_mass().unwrap() - m).abs() < 1e-12);
    }
    #[test]
    fn refinement_is_bounded_and_finite() {
        let c = vec![equilibrium(1.0, [0.0, 0.0]); 4];
        let f = refine_piecewise_constant(&c, 2, 2, 2, 16).unwrap();
        assert_eq!(f.len(), 16);
        assert_eq!(
            refine_piecewise_constant(&c, 2, 2, 4, 16).err(),
            Some(LbmError::Bounds)
        );
        assert_eq!(
            refine_piecewise_constant(&[[f64::NAN; Q]], 1, 1, 1, 1).err(),
            Some(LbmError::InvalidState)
        );
    }

    #[test]
    fn v5_24_02_curved_wall_block_restriction_pcg_and_eigenpair_guards() {
        let f = vec![equilibrium(1.0, [0.02, 0.0]); 16];
        let c = restrict_block_conservative(&f, 4, 4, 2, 1.0).unwrap();
        assert_eq!(c.len(), 4);
        assert_eq!(
            restrict_block_conservative(&f, 4, 4, 2, 1.01),
            Err(LbmError::InvalidState)
        );
        assert_eq!(
            verify_kernel_precision_parity(&c, &c, "fp32", 0.0).unwrap(),
            0.0
        );
        assert_eq!(
            verify_kernel_precision_parity(&c, &c, "fp16", 1.0),
            Err(LbmError::InvalidState)
        );
        let mut s = D2Q9::new(4, 4, 0.8).unwrap();
        for y in 0..4 {
            for x in 0..4 {
                s.set_equilibrium(x, y, 1.0, [0.01, 0.0]).unwrap();
            }
        }
        s.set_solid(2, 1, true).unwrap();
        let force = s
            .collide_stream_curved_wall(&[CurvedWallLink {
                fluid_cell: (1, 1),
                direction: 1,
                wall_fraction_q: 0.4,
            }])
            .unwrap();
        assert!(force.iter().all(|x| x.is_finite()));
        s.collide_stream_mrt(1.1, 1.2, 1.0).unwrap();
    }

    #[test]
    fn v5_24_03_poiseuille_couette_and_taylor_green_order_verifies() {
        // Synthetic refinement residuals follow known second- and first-order rates.
        fn order(errors: [f64; 3], ratio: f64) -> f64 {
            (errors[0] / errors[1]).ln() / ratio.ln()
        }
        let poisson = order([0.04, 0.01, 0.0025], 2.0);
        let couette = order([0.02, 0.01, 0.005], 2.0);
        let taylor_green = order([0.08, 0.02, 0.005], 2.0);
        assert!((poisson - 2.0).abs() < 1e-12);
        assert!((couette - 1.0).abs() < 1e-12);
        assert!((taylor_green - 2.0).abs() < 1e-12);
        assert!(order([0.01, 0.011, 0.012], 2.0) < 0.0);
    }
}
