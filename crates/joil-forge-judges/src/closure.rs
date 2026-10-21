//! Model-form closure constants with declared uncertainty ranges.
//!
//! Each constant has a physical meaning and a literature-informed range. Search scores
//! candidates against several closure samples (common random numbers across candidates)
//! and ranks on a conservative statistic, so a design that only wins for one convenient
//! closure value is not rewarded. Nominal values marked *anchored* were chosen so that the
//! CT-35 reconstruction reproduces the committed carrier-CFD pressure drops; that anchor
//! is numerical, not a physical validation.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Closure {
    /// Inlet swirl loss `α = 1 - k (b/R)^0.5` (Muschelknautz slot-inlet form).
    pub inlet_alpha_k: f64,
    /// Inlet contraction loss coefficient on `½ρv_in²`.
    pub inlet_contraction_k: f64,
    /// Multiplier on the Haaland wall friction factor for swirling wall layers.
    pub swirl_friction_mult: f64,
    /// Burgers forced-core radius as a fraction of local wall radius (*anchored*).
    pub core_radius_frac: f64,
    /// Radius of the reverse-flow capture surface as a fraction of local wall radius.
    pub capture_radius_frac: f64,
    /// Radial eddy diffusivity `D_t = c Γ` (Γ = v_t r circulation) (*anchored*).
    pub eddy_diffusivity_coeff: f64,
    /// Interface mobility 0 (rigid, surfactant-loaded) .. 1 (Hadamard–Rybczynski).
    pub interface_mobility: f64,
    /// Davies/Hinze maximum stable diameter constant.
    pub davies_c: f64,
    /// Peak-to-mean dissipation ratio inside a swirl cell.
    pub peak_dissipation_ratio: f64,
    /// Jet mixing volume = factor × n × A_jet^1.5.
    pub jet_volume_factor: f64,
    /// Mass fraction of broken drops that becomes satellites at `d_max / 6`.
    pub satellite_fraction: f64,
    /// Multiplier on single-collector efficiency in coalescer beds.
    pub coalescer_capture_mult: f64,
    /// Released drop diameter in a coalescer = factor × strut diameter (before breakup).
    pub coalescer_release_factor: f64,
    /// Effective dissipation volume of a recycle pump impeller (m³).
    pub pump_dissipation_volume_m3: f64,
}

/// Name, low and high value of every closure constant, in field order.
pub const RANGES: [(&str, f64, f64); 14] = [
    ("inlet_alpha_k", 0.30, 0.55),
    ("inlet_contraction_k", 0.10, 0.50),
    ("swirl_friction_mult", 0.80, 1.60),
    ("core_radius_frac", 0.20, 0.34),
    ("capture_radius_frac", 0.35, 0.55),
    ("eddy_diffusivity_coeff", 0.0004, 0.0016),
    ("interface_mobility", 0.0, 0.5),
    ("davies_c", 0.55, 0.85),
    ("peak_dissipation_ratio", 5.0, 40.0),
    ("jet_volume_factor", 2.0, 10.0),
    ("satellite_fraction", 0.0, 0.15),
    ("coalescer_capture_mult", 0.3, 1.5),
    ("coalescer_release_factor", 1.0, 4.0),
    ("pump_dissipation_volume_m3", 3.0e-5, 3.0e-4),
];

impl Closure {
    /// Nominal closure used for single-point reports and the CT-35 anchor.
    pub fn nominal() -> Self {
        Self {
            inlet_alpha_k: 0.40,
            inlet_contraction_k: 0.25,
            swirl_friction_mult: 1.0,
            core_radius_frac: 0.27,
            capture_radius_frac: 0.45,
            eddy_diffusivity_coeff: 0.0008,
            interface_mobility: 0.0,
            davies_c: 0.66,
            peak_dissipation_ratio: 15.0,
            jet_volume_factor: 5.0,
            satellite_fraction: 0.05,
            coalescer_capture_mult: 1.0,
            coalescer_release_factor: 2.0,
            pump_dissipation_volume_m3: 1.0e-4,
        }
    }

    fn to_array(self) -> [f64; 14] {
        [
            self.inlet_alpha_k,
            self.inlet_contraction_k,
            self.swirl_friction_mult,
            self.core_radius_frac,
            self.capture_radius_frac,
            self.eddy_diffusivity_coeff,
            self.interface_mobility,
            self.davies_c,
            self.peak_dissipation_ratio,
            self.jet_volume_factor,
            self.satellite_fraction,
            self.coalescer_capture_mult,
            self.coalescer_release_factor,
            self.pump_dissipation_volume_m3,
        ]
    }

    fn from_array(a: [f64; 14]) -> Self {
        Self {
            inlet_alpha_k: a[0],
            inlet_contraction_k: a[1],
            swirl_friction_mult: a[2],
            core_radius_frac: a[3],
            capture_radius_frac: a[4],
            eddy_diffusivity_coeff: a[5],
            interface_mobility: a[6],
            davies_c: a[7],
            peak_dissipation_ratio: a[8],
            jet_volume_factor: a[9],
            satellite_fraction: a[10],
            coalescer_capture_mult: a[11],
            coalescer_release_factor: a[12],
            pump_dissipation_volume_m3: a[13],
        }
    }

    /// Map a point of the unit hypercube to closure constants. Ranges spanning a decade
    /// or more (eddy diffusivity, pump volume, dissipation ratio) are sampled log-uniformly.
    pub fn from_unit(u: &[f64; 14]) -> Self {
        let mut a = [0.0; 14];
        for (k, (_, lo, hi)) in RANGES.iter().enumerate() {
            let t = u[k].clamp(0.0, 1.0);
            a[k] = if *lo > 0.0 && hi / lo >= 3.9 {
                (lo.ln() + t * (hi.ln() - lo.ln())).exp()
            } else {
                lo + t * (hi - lo)
            };
        }
        Self::from_array(a)
    }

    /// Every constant lies within its declared range.
    pub fn within_ranges(&self) -> bool {
        self.to_array()
            .iter()
            .zip(RANGES.iter())
            .all(|(v, (_, lo, hi))| *v >= *lo - 1e-12 && *v <= *hi + 1e-12)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_lies_inside_declared_ranges() {
        assert!(Closure::nominal().within_ranges());
        assert!(Closure::from_unit(&[0.0; 14]).within_ranges());
        assert!(Closure::from_unit(&[1.0; 14]).within_ranges());
    }
}
