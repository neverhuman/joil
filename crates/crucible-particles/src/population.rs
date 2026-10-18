//! Exposure-integrated population changes with explicit calibration status.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataStatus {
    Synthetic,
    Uncalibrated,
    Candidate,
    Calibrated,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DaughterClass {
    pub diameter_m: f64,
    pub mass_fraction: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BreakupModel {
    pub data_status: DataStatus,
    pub rate_per_s: f64,
    pub relative_rate_uncertainty: f64,
    pub daughters: Vec<DaughterClass>,
    pub model_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BreakupBounds {
    pub probability_lower: f64,
    pub probability_nominal: f64,
    pub probability_upper: f64,
    pub daughter_mass_fraction: f64,
    pub mass_closure_relative_error: f64,
    pub rate_data_status: DataStatus,
    pub calibrated: bool,
    pub hinze_diagnostic_diameter_m: f64,
}

impl BreakupModel {
    pub fn evaluate(
        &self,
        exposure_s: f64,
        parent_mass_kg: f64,
        epsilon_m2_s3: f64,
        density_kg_m3: f64,
        surface_tension_n_m: f64,
    ) -> Result<BreakupBounds, String> {
        if self.model_id.trim().is_empty()
            || !exposure_s.is_finite()
            || exposure_s < 0.0
            || !parent_mass_kg.is_finite()
            || parent_mass_kg <= 0.0
            || !self.rate_per_s.is_finite()
            || self.rate_per_s < 0.0
            || !self.relative_rate_uncertainty.is_finite()
            || !(0.0..1.0).contains(&self.relative_rate_uncertainty)
            || self.daughters.is_empty()
            || self.daughters.iter().any(|d| {
                !d.diameter_m.is_finite()
                    || d.diameter_m <= 0.0
                    || !d.mass_fraction.is_finite()
                    || d.mass_fraction < 0.0
            })
        {
            return Err("invalid breakup model or exposure inputs".into());
        }
        let daughter_mass_fraction: f64 = self.daughters.iter().map(|d| d.mass_fraction).sum();
        let closure = (daughter_mass_fraction - 1.0).abs();
        if !daughter_mass_fraction.is_finite() || closure > 1e-12 {
            return Err(
                "daughter mass fractions must close to one; clipping cannot repair breakup mass"
                    .into(),
            );
        }
        let probability = |rate: f64| -(-rate * exposure_s).exp_m1();
        let lo = probability(self.rate_per_s * (1.0 - self.relative_rate_uncertainty));
        let hi = probability(self.rate_per_s * (1.0 + self.relative_rate_uncertainty));
        let hinze = hinze_diameter(epsilon_m2_s3, density_kg_m3, surface_tension_n_m)?;
        Ok(BreakupBounds {
            probability_lower: lo,
            probability_nominal: probability(self.rate_per_s),
            probability_upper: hi,
            daughter_mass_fraction,
            mass_closure_relative_error: closure,
            rate_data_status: self.data_status,
            calibrated: self.data_status == DataStatus::Calibrated,
            hinze_diagnostic_diameter_m: hinze,
        })
    }

    /// Universal instantaneous diameter clipping is not a calibrated exposure model.
    pub fn from_hinze_clipping(_hinze_diameter_m: f64) -> Result<Self, String> {
        Err(
            "instantaneous universal Hinze clipping is a diagnostic bound, not calibrated breakup"
                .into(),
        )
    }
}

pub fn hinze_diameter(
    epsilon_m2_s3: f64,
    density_kg_m3: f64,
    surface_tension_n_m: f64,
) -> Result<f64, String> {
    if !epsilon_m2_s3.is_finite()
        || epsilon_m2_s3 <= 0.0
        || !density_kg_m3.is_finite()
        || density_kg_m3 <= 0.0
        || !surface_tension_n_m.is_finite()
        || surface_tension_n_m <= 0.0
    {
        return Err("Hinze diagnostic inputs must be finite and positive".into());
    }
    // Hinze scaling; the prefactor remains a declared diagnostic constant, not calibration.
    let diameter =
        0.725 * (surface_tension_n_m / density_kg_m3).powf(0.6) * epsilon_m2_s3.powf(-0.4);
    if !diameter.is_finite() || diameter <= 0.0 {
        return Err("Hinze diameter is not representable".into());
    }
    Ok(diameter)
}

/// Direct explicit midpoint integration of the constant-kernel number equation.
/// Returns N(t); the analytic oracle is N0/(1+K*N0*t/2).
pub fn smoluchowski_constant_kernel(
    n0_m3: f64,
    kernel_m3_s: f64,
    duration_s: f64,
    steps: usize,
) -> Result<(f64, f64), String> {
    if !n0_m3.is_finite()
        || n0_m3 < 0.0
        || !kernel_m3_s.is_finite()
        || kernel_m3_s < 0.0
        || !duration_s.is_finite()
        || duration_s < 0.0
        || steps == 0
    {
        return Err("invalid Smoluchowski inputs".into());
    }
    let dt = duration_s / steps as f64;
    let mut n = n0_m3;
    for _ in 0..steps {
        let f = |x: f64| -0.5 * kernel_m3_s * x * x;
        let k1 = f(n);
        let k2 = f(n + 0.5 * dt * k1);
        n += dt * k2;
        if !n.is_finite() || n < 0.0 {
            return Err("Smoluchowski integration lost positivity or finiteness".into());
        }
    }
    let exact = n0_m3 / (1.0 + 0.5 * kernel_m3_s * n0_m3 * duration_s);
    Ok((n, exact))
}

/// Number density in equally spaced diameter classes. Refinement preserves the
/// supplied mass fractions; callers must provide source-supported DSD data.
pub fn refine_mass_classes(mass_fractions: &[f64], factor: usize) -> Result<Vec<f64>, String> {
    if mass_fractions.is_empty()
        || factor == 0
        || mass_fractions.iter().any(|x| !x.is_finite() || *x < 0.0)
    {
        return Err("invalid DSD refinement input".into());
    }
    let sum: f64 = mass_fractions.iter().sum();
    if (sum - 1.0).abs() > 1e-12 {
        return Err("DSD mass fractions must sum to one".into());
    }
    let mut refined = Vec::with_capacity(mass_fractions.len() * factor);
    for fraction in mass_fractions {
        refined.extend(std::iter::repeat_n(*fraction / factor as f64, factor));
    }
    Ok(refined)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct VelocityGroup {
    pub velocity_m_s: [f64; 3],
    pub mass_fraction: f64,
}

/// Split quadrature weights without collapsing distinct velocity groups to one shared velocity.
pub fn refine_velocity_groups(
    groups: &[VelocityGroup],
    factor: usize,
) -> Result<Vec<VelocityGroup>, String> {
    if groups.is_empty()
        || factor == 0
        || groups.iter().any(|g| {
            !g.mass_fraction.is_finite()
                || g.mass_fraction < 0.0
                || !g.velocity_m_s.iter().all(|v| v.is_finite())
        })
    {
        return Err("invalid velocity-group refinement input".into());
    }
    let total: f64 = groups.iter().map(|g| g.mass_fraction).sum();
    if (total - 1.0).abs() > 1e-12 {
        return Err("velocity-group mass weights must sum to one".into());
    }
    let mut out = Vec::with_capacity(groups.len() * factor);
    for group in groups {
        out.extend(std::iter::repeat_n(
            VelocityGroup {
                velocity_m_s: group.velocity_m_s,
                mass_fraction: group.mass_fraction / factor as f64,
            },
            factor,
        ));
    }
    Ok(out)
}
