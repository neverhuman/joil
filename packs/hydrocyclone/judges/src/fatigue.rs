//! V5-20-02: finite-life fatigue estimands tied to a V5-13 material passport.
//!
//! Every statement here is a finite-life statement: a predicted number of cycles to failure for
//! one process/orientation/finish/defect curve, one Goodman mean-stress treatment and one declared
//! cycle-count spectrum, with model/data scatter carried as a lower bound. A target at or above
//! the declared run-out is a screen and is never reported as infinite life. A curve whose evidence
//! context differs from the passport's admitted context, key by key, cannot be used for that
//! passport, so no curve transfers to another chemistry or process.
use crate::material::{ClaimKind, ContextKey, MaterialPassport, ALL_CONTEXT};
use crate::{Field, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasquinCurve {
    /// Fatigue strength coefficient in Pa: stress amplitude at one half cycle.
    pub sigma_f_prime_pa: f64,
    /// Basquin exponent, dimensionless and negative.
    pub exponent_b: f64,
    /// One-sigma scatter of log10 life from the fit, dimensionless.
    pub log10_life_sigma: f64,
    /// Process context the curve data were generated under, key by key.
    pub evidence_context: BTreeMap<ContextKey, String>,
}

/// One cycle-count bin of a block: stress amplitude and mean in Pa, and cycles per block.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CycleBin {
    pub amplitude_pa: f64,
    pub mean_pa: f64,
    pub cycles: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FatigueInput {
    pub curve: Field<BasquinCurve>,
    /// Ultimate strength in Pa for the Goodman mean-stress correction.
    pub ultimate_pa: Field<f64>,
    /// Coverage factor k for the life lower bound `N * 10^(-k * sigma)`; must be at least one.
    pub coverage_k: Field<f64>,
    /// Cycle-count spectrum of one block. An RMS pressure is not a spectrum and cannot be used.
    pub spectrum: Field<Vec<CycleBin>>,
    /// Finite life target in cycles that the claim asks about.
    pub target_cycles: f64,
    /// Declared run-out target; a target at or above it is a screen, never infinite life.
    pub runout_cycles: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FiniteLifeReport {
    pub verdict: Verdict,
    pub predicted_cycles: Option<f64>,
    pub lower_bound_cycles: Option<f64>,
    /// Present only for a computed finite-life estimate; always names the passport.
    pub statement: Option<String>,
}

fn unresolved(reasons: Vec<String>) -> FiniteLifeReport {
    FiniteLifeReport {
        verdict: Verdict::Unresolved(reasons),
        predicted_cycles: None,
        lower_bound_cycles: None,
        statement: None,
    }
}

/// Goodman-corrected equivalent fully reversed amplitude `a / (1 - m / u)`.
pub fn goodman_amplitude(amplitude_pa: f64, mean_pa: f64, ultimate_pa: f64) -> f64 {
    amplitude_pa / (1.0 - mean_pa / ultimate_pa)
}

/// Basquin cycles to failure `N = 0.5 * (sigma / sigma_f')^(1/b)`.
pub fn basquin_cycles(equivalent_amplitude_pa: f64, curve: &BasquinCurve) -> f64 {
    0.5 * (equivalent_amplitude_pa / curve.sigma_f_prime_pa).powf(1.0 / curve.exponent_b)
}

fn curve_is_valid(curve: &BasquinCurve) -> bool {
    curve.sigma_f_prime_pa.is_finite()
        && curve.sigma_f_prime_pa > 0.0
        && curve.exponent_b.is_finite()
        && curve.exponent_b < 0.0
        && curve.log10_life_sigma.is_finite()
        && curve.log10_life_sigma >= 0.0
}

fn spectrum_is_valid(bins: &[CycleBin], ultimate_pa: f64) -> bool {
    !bins.is_empty()
        && bins.iter().all(|b| {
            b.amplitude_pa.is_finite()
                && b.amplitude_pa > 0.0
                && b.mean_pa.is_finite()
                && b.mean_pa.abs() < ultimate_pa
                && b.cycles.is_finite()
                && b.cycles >= 0.0
        })
        && bins.iter().map(|b| b.cycles).sum::<f64>() > 0.0
}

/// Judge a finite-life fatigue target for one passport. Missing or mismatched evidence is
/// unresolved, never a pass, and never a physical failure.
pub fn judge_finite_life(passport: &MaterialPassport, input: &FatigueInput) -> FiniteLifeReport {
    if let Err(error) = passport.check() {
        return unresolved(vec![format!("passport:invalid:{error}")]);
    }
    let passport_verdict = passport.qualify(ClaimKind::AsBuiltFatigue);
    if !passport_verdict.is_pass() {
        return FiniteLifeReport {
            verdict: passport_verdict,
            predicted_cycles: None,
            lower_bound_cycles: None,
            statement: None,
        };
    }
    let mut open = Vec::new();
    let curve = input.curve.admitted();
    let ultimate = input.ultimate_pa.admitted().copied();
    let coverage = input.coverage_k.admitted().copied();
    let spectrum = input.spectrum.admitted();
    if curve.is_none() {
        open.push("curve:not_admitted".into());
    }
    if ultimate.is_none() {
        open.push("ultimate:not_admitted".into());
    }
    if coverage.is_none() {
        open.push("coverage_k:not_admitted".into());
    }
    if spectrum.is_none() {
        open.push("spectrum:not_admitted".into());
    }
    if let Some(curve) = curve {
        if !curve_is_valid(curve) {
            open.push("curve:invalid_parameters".into());
        }
        for key in ALL_CONTEXT {
            let declared = passport.context.get(&key).and_then(Field::admitted);
            if curve.evidence_context.get(&key) != declared {
                open.push(format!("curve:context_mismatch:{}", key.name()));
            }
        }
    }
    if let Some(u) = ultimate {
        if !(u.is_finite() && u > 0.0) {
            open.push("ultimate:invalid".into());
        }
    }
    if let Some(k) = coverage {
        if !(k.is_finite() && k >= 1.0) {
            open.push("coverage_k:invalid".into());
        }
    }
    if let (Some(bins), Some(u)) = (spectrum, ultimate) {
        if !spectrum_is_valid(bins, u) {
            open.push("spectrum:invalid_or_empty".into());
        }
    }
    if !(input.target_cycles.is_finite() && input.target_cycles > 0.0) {
        open.push("target:invalid".into());
    }
    if !(input.runout_cycles.is_finite() && input.runout_cycles > 0.0) {
        open.push("runout:invalid".into());
    }
    if input.target_cycles >= input.runout_cycles {
        open.push("target:at_or_above_runout_is_screen_not_finite_life".into());
    }
    if !open.is_empty() {
        return unresolved(open);
    }
    let (Some(curve), Some(u), Some(k), Some(bins)) = (curve, ultimate, coverage, spectrum) else {
        return unresolved(open);
    };
    let damage: f64 = bins
        .iter()
        .map(|b| {
            let equivalent = goodman_amplitude(b.amplitude_pa, b.mean_pa, u);
            b.cycles / basquin_cycles(equivalent, curve)
        })
        .sum();
    let block_cycles: f64 = bins.iter().map(|b| b.cycles).sum();
    let predicted = block_cycles / damage;
    let lower_bound = predicted * 10f64.powf(-k * curve.log10_life_sigma);
    let verdict = if lower_bound >= input.target_cycles {
        Verdict::Pass
    } else if predicted < input.target_cycles {
        Verdict::Fail(vec!["life:predicted_below_target".into()])
    } else {
        Verdict::Unresolved(vec!["life:lower_bound_straddles_target".into()])
    };
    FiniteLifeReport {
        verdict,
        predicted_cycles: Some(predicted),
        lower_bound_cycles: Some(lower_bound),
        statement: Some(format!(
            "finite life: predicted {predicted:.4e} cycles, lower bound {lower_bound:.4e} cycles \
             (k={k}, sigma_log10={}), target {:.4e} cycles; passport {}; run-out {:.4e} cycles is \
             a declared screen, not infinite life; no transfer to other chemistry",
            curve.log10_life_sigma, input.target_cycles, passport.passport_id, input.runout_cycles
        )),
    }
}
