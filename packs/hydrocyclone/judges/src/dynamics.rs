//! V5-20-01 and V5-20-05: wet bending modes and independent structural confirmation.
//!
//! A wet cantilever mode is judged only from its own admitted inputs: the beam, an admitted
//! fluid added mass per unit length, an admitted support record and an admitted broadband
//! wall-pressure spectrum that covers the mode band. A single copied frequency (the legacy
//! 418 Hz field) is a one-bin spectrum and a nominal Strouhal ratio is not a spectrum at all,
//! so neither can authorize dynamics. The forced-response judgement is a band-energy screen,
//! not a modal forced-response solve.
use crate::{Field, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// First root of cos(b)cosh(b) = -1 for a clamped-free Euler-Bernoulli beam.
pub const CLAMPED_FREE_BETA1_L: f64 = 1.875_104_068_711_961;

/// Admitted value, or a `not_admitted` reason appended to `open`.
fn admitted<'a, T>(field: &'a Field<T>, name: &str, open: &mut Vec<String>) -> Option<&'a T> {
    let value = field.admitted();
    if value.is_none() {
        open.push(format!("{name}:not_admitted"));
    }
    value
}

fn finite_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn finite_non_negative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportKind {
    /// Built-in root and free tip: the only support with a closed-form oracle in this module.
    ClampedFree,
    /// Any other support needs a modal solve on the actual supports, which is not admitted here.
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cantilever {
    pub length_m: f64,
    pub bending_stiffness_nm2: f64,
    /// Dry structural mass per unit length, kg/m.
    pub mass_per_length_kg_m: f64,
    /// Fluid added mass per unit length, kg/m, from an admitted fluid-structure source.
    pub added_mass_kg_m: Field<f64>,
    pub support: Field<SupportKind>,
}

impl Cantilever {
    /// First wet bending frequency `f = (beta L)^2 / (2 pi L^2) * sqrt(EI / (m + m_a))`.
    /// Returns the open reasons instead of a number when any input is not admitted or invalid.
    pub fn first_wet_frequency_hz(&self) -> Result<f64, Vec<String>> {
        let mut open = Vec::new();
        let geometry = [
            self.length_m,
            self.bending_stiffness_nm2,
            self.mass_per_length_kg_m,
        ];
        if !geometry.iter().all(|v| finite_positive(*v)) {
            open.push("beam:invalid_geometry".into());
        }
        let added = admitted(&self.added_mass_kg_m, "added_mass", &mut open);
        if let Some(m_a) = added {
            if !finite_non_negative(*m_a) {
                open.push("added_mass:invalid".into());
            }
        }
        match admitted(&self.support, "support", &mut open) {
            Some(SupportKind::ClampedFree) | None => {}
            Some(SupportKind::Other) => open.push("support:no_closed_form_oracle".into()),
        }
        match added {
            Some(m_a) if open.is_empty() => {
                let (l, ei, m) = (
                    self.length_m,
                    self.bending_stiffness_nm2,
                    self.mass_per_length_kg_m,
                );
                Ok(CLAMPED_FREE_BETA1_L.powi(2) / (2.0 * PI * l * l) * (ei / (m + *m_a)).sqrt())
            }
            _ => Err(open),
        }
    }
}

/// One one-sided wall-pressure PSD bin.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PsdBin {
    pub hz: f64,
    pub psd_pa2_hz: f64,
}

/// Broadband one-sided wall-pressure PSD in increasing frequency order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressureSpectrum {
    pub bins: Vec<PsdBin>,
}

impl PressureSpectrum {
    /// A broadband spectrum needs at least three ordered bins; one copied frequency is refused.
    fn is_broadband(&self) -> bool {
        self.bins.len() >= 3
            && self
                .bins
                .iter()
                .all(|b| finite_positive(b.hz) && finite_non_negative(b.psd_pa2_hz))
            && self.bins.windows(2).all(|w| w[0].hz < w[1].hz)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WetModeInput {
    pub cantilever: Cantilever,
    /// Modal damping ratio, dimensionless, admitted with its source.
    pub damping_ratio: Field<f64>,
    pub spectrum: Field<PressureSpectrum>,
    /// Admitted limit on the share of broadband PSD energy inside the mode half-power band.
    pub max_band_energy_fraction: Field<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WetModeScreen {
    pub mode_hz: Option<f64>,
    pub band_energy_fraction: Option<f64>,
    pub verdict: Verdict,
}

/// Trapezoid integral of PSD over frequency for `(hz, psd)` points.
fn trapezoid(points: &[(f64, f64)]) -> f64 {
    points
        .windows(2)
        .map(|w| 0.5 * (w[0].1 + w[1].1) * (w[1].0 - w[0].0))
        .sum()
}

/// Screen one wet bending mode against an admitted broadband spectrum. `Pass` means the
/// admitted spectrum puts no more energy in the mode band than the admitted limit; it is not
/// a forced-response qualification and it never follows from a single frequency.
pub fn screen_wet_mode(input: &WetModeInput) -> WetModeScreen {
    let unresolved = |open: Vec<String>| WetModeScreen {
        mode_hz: None,
        band_energy_fraction: None,
        verdict: Verdict::Unresolved(open),
    };
    let mut open = Vec::new();
    let mode = input.cantilever.first_wet_frequency_hz();
    if let Err(reasons) = &mode {
        open.extend(reasons.iter().cloned());
    }
    let zeta = admitted(&input.damping_ratio, "damping", &mut open).copied();
    if let Some(z) = zeta {
        if !(z.is_finite() && z > 0.0 && z < 1.0) {
            open.push("damping:invalid".into());
        }
    }
    let limit = admitted(
        &input.max_band_energy_fraction,
        "band_energy_limit",
        &mut open,
    )
    .copied();
    if let Some(l) = limit {
        if !(l.is_finite() && (0.0..=1.0).contains(&l)) {
            open.push("band_energy_limit:invalid".into());
        }
    }
    let spectrum = admitted(&input.spectrum, "spectrum", &mut open);
    let (Ok(f), Some(z), Some(limit), Some(spectrum)) = (mode, zeta, limit, spectrum) else {
        return unresolved(open);
    };
    if !spectrum.is_broadband() {
        open.push("spectrum:invalid_or_single_frequency".into());
    }
    if spectrum.bins.first().is_none_or(|b| b.hz > 0.5 * f)
        || spectrum.bins.last().is_none_or(|b| b.hz < 2.0 * f)
    {
        open.push("spectrum:does_not_cover_mode_band".into());
    }
    if !open.is_empty() {
        return unresolved(open);
    }
    let all: Vec<(f64, f64)> = spectrum.bins.iter().map(|b| (b.hz, b.psd_pa2_hz)).collect();
    let (lo, hi) = (f * (1.0 - z), f * (1.0 + z));
    let inside: Vec<(f64, f64)> = all
        .iter()
        .copied()
        .filter(|(hz, _)| *hz >= lo && *hz <= hi)
        .collect();
    if inside.len() < 2 {
        return unresolved(vec!["spectrum:band_resolution_insufficient".into()]);
    }
    let total = trapezoid(&all);
    if !total.is_finite() || total <= 0.0 {
        return unresolved(vec!["spectrum:zero_energy".into()]);
    }
    let fraction = trapezoid(&inside) / total;
    let verdict = if fraction <= limit {
        Verdict::Pass
    } else {
        Verdict::Fail(vec![format!(
            "forced_response:band_energy_fraction_{fraction:.4}_exceeds_{limit:.4}"
        )])
    };
    WetModeScreen {
        mode_hz: Some(f),
        band_energy_fraction: Some(fraction),
        verdict,
    }
}

/// Modal mesh series, coarse to fine, for one mode.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshSeries {
    /// Characteristic element size in metres, strictly decreasing.
    pub element_size_m: Vec<f64>,
    pub mode_hz: Vec<f64>,
}

/// Modal mesh check: at least three refinements, changes that do not grow under refinement,
/// and a last relative change within an admitted tolerance. Malformed series are unresolved;
/// a series that refines badly fails.
pub fn check_mesh_convergence(series: &MeshSeries, rel_tol: &Field<f64>) -> Verdict {
    let mut open = Vec::new();
    let Some(tol) = admitted(rel_tol, "mesh_tolerance", &mut open).copied() else {
        return Verdict::Unresolved(open);
    };
    if !finite_positive(tol) {
        return Verdict::Unresolved(vec!["mesh_tolerance:invalid".into()]);
    }
    let n = series.mode_hz.len();
    if n < 3 || series.element_size_m.len() != n {
        return Verdict::Unresolved(vec!["mesh_series:needs_three_refinements".into()]);
    }
    if !series.element_size_m.iter().all(|h| finite_positive(*h))
        || !series.element_size_m.windows(2).all(|w| w[1] < w[0])
    {
        return Verdict::Unresolved(vec!["mesh_series:sizes_not_strictly_refining".into()]);
    }
    if !series.mode_hz.iter().all(|f| finite_positive(*f)) {
        return Verdict::Unresolved(vec!["mesh_series:nonfinite_frequency".into()]);
    }
    let deltas: Vec<f64> = series
        .mode_hz
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .collect();
    let last_relative = deltas[deltas.len() - 1] / series.mode_hz[n - 1];
    let mut reasons = Vec::new();
    if deltas.windows(2).any(|d| d[1] > d[0]) {
        reasons.push("mesh:changes_grow_under_refinement".into());
    }
    if last_relative > tol {
        reasons.push(format!(
            "mesh:last_relative_change_{last_relative:.3e}_exceeds_{tol:.3e}"
        ));
    }
    if reasons.is_empty() {
        Verdict::Pass
    } else {
        Verdict::Fail(reasons)
    }
}

/// One independent route's value of a governing observable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteResult {
    pub route_id: String,
    pub observable: String,
    pub value: f64,
    /// Digest of the boundary and load file this route consumed.
    pub input_sha256: Sha256Hex,
}

/// What a cross-check does to the claim that uses the observable.
#[derive(Clone, Debug, PartialEq)]
pub enum ClaimEffect {
    /// The observable is supported; the route discrepancy is carried as an uncertainty term.
    Supported { discrepancy_uncertainty: f64 },
    /// Disagreement, shared inputs or missing evidence: the claim is not supported.
    NotSupported,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CrossCheck {
    pub verdict: Verdict,
    pub effect: ClaimEffect,
}

/// Compare two routes on one observable. Agreement within an admitted relative tolerance
/// supports the claim; disagreement blocks it. Two routes that consumed the same boundary or
/// load file are not independent, so they cannot qualify the input both share, however well
/// they agree.
pub fn cross_check(a: &RouteResult, b: &RouteResult, rel_tol: &Field<f64>) -> CrossCheck {
    let not_supported = |open: Vec<String>| CrossCheck {
        verdict: Verdict::Unresolved(open),
        effect: ClaimEffect::NotSupported,
    };
    if a.observable != b.observable || a.route_id == b.route_id {
        return not_supported(vec![
            "cross_check:routes_must_be_distinct_on_one_observable".into(),
        ]);
    }
    if !(a.value.is_finite() && b.value.is_finite()) {
        return not_supported(vec!["cross_check:nonfinite_route_value".into()]);
    }
    if a.input_sha256 == b.input_sha256 {
        return not_supported(vec![format!(
            "cross_check:{}:shared_boundary_load_input_not_independent",
            a.observable
        )]);
    }
    let Some(tol) = admitted(rel_tol, "cross_check_tolerance", &mut Vec::new())
        .copied()
        .filter(|t| finite_positive(*t))
    else {
        return not_supported(vec!["cross_check_tolerance:not_admitted_or_invalid".into()]);
    };
    let difference = (a.value - b.value).abs();
    let scale = a.value.abs().max(b.value.abs());
    let relative = if scale == 0.0 {
        0.0
    } else {
        difference / scale
    };
    if relative <= tol {
        CrossCheck {
            verdict: Verdict::Pass,
            effect: ClaimEffect::Supported {
                discrepancy_uncertainty: difference,
            },
        }
    } else {
        CrossCheck {
            verdict: Verdict::Fail(vec![format!(
                "cross_check:{}:discrepancy_{relative:.4}_exceeds_{tol:.4}",
                a.observable
            )]),
            effect: ClaimEffect::NotSupported,
        }
    }
}
