//! V5-18-01 and V5-18-03: exact concentration oracles and treated-volume mechanical energy.
//!
//! Outlet concentration keeps the split denominator `(1 - r)` and the unclipped reduced
//! efficiency `(E - r) / (1 - r)`. A negative reduced efficiency means oil left in the treated
//! stream and is reported as found, never clipped. Oil that does not close, or that remains
//! in the domain, is assigned to the adverse side of each bound. Energy is the integrated
//! mechanical flux loss per compliant treated cubic metre; the per-feed quantity is kept only
//! as a labelled historical basis and is refused for the common-duty objective.
//! Synthetic fixtures exercise the arithmetic only; nothing here is a physical result.
use crate::{invalid, JudgeError};
use serde::{Deserialize, Serialize};

/// 1 kg/m³ = 1000 mg/L.
const MG_L_PER_KG_M3: f64 = 1000.0;
/// Relative tolerance for weight normalisation and mass-row conservation.
const NORMALISATION_TOLERANCE: f64 = 1e-9;

fn fraction(name: &str, x: f64) -> Result<f64, JudgeError> {
    if x.is_finite() && (0.0..=1.0).contains(&x) {
        Ok(x)
    } else {
        Err(invalid(format!("{name} must be finite in [0, 1]")))
    }
}

fn nonnegative(name: &str, x: f64) -> Result<f64, JudgeError> {
    if x.is_finite() && x >= 0.0 {
        Ok(x)
    } else {
        Err(invalid(format!("{name} must be finite and nonnegative")))
    }
}

/// Overflow/feed volume split `r = Q_o / Q_i`, required in `[0, 1)`.
pub fn split(r: f64) -> Result<f64, JudgeError> {
    if r.is_finite() && (0.0..1.0).contains(&r) {
        Ok(r)
    } else {
        Err(invalid("split r must be finite in [0, 1)"))
    }
}

/// Reduced efficiency `(E - r) / (1 - r)`. Deliberately unclipped: `E < r` gives a negative value.
pub fn reduced_efficiency(capture: f64, r: f64) -> Result<f64, JudgeError> {
    let r = split(r)?;
    let capture = fraction("capture", capture)?;
    Ok((capture - r) / (1.0 - r))
}

/// Dilute outlet concentration `Cu = Ci (1 - E) / (1 - r)` in mg/L.
pub fn outlet_concentration_mg_l(ci_mg_l: f64, capture: f64, r: f64) -> Result<f64, JudgeError> {
    let ci = nonnegative("inlet concentration", ci_mg_l)?;
    let capture = fraction("capture", capture)?;
    let r = split(r)?;
    let cu = ci * (1.0 - capture) / (1.0 - r);
    if cu.is_finite() {
        Ok(cu)
    } else {
        Err(invalid("outlet concentration overflow"))
    }
}

/// Same oracle written through the reduced efficiency: `Cu = Ci (1 - E_reduced)`.
pub fn outlet_concentration_from_reduced_mg_l(
    ci_mg_l: f64,
    reduced: f64,
) -> Result<f64, JudgeError> {
    let ci = nonnegative("inlet concentration", ci_mg_l)?;
    if !reduced.is_finite() || reduced > 1.0 {
        return Err(invalid("reduced efficiency must be finite and at most 1"));
    }
    nonnegative("derived outlet concentration", ci * (1.0 - reduced))
}

/// Smallest raw capture `E` for which `Cu <= limit`: `E = 1 - limit (1 - r) / Ci`, floored at 0.
pub fn required_capture_for_limit(
    ci_mg_l: f64,
    r: f64,
    limit_mg_l: f64,
) -> Result<f64, JudgeError> {
    let ci = nonnegative("inlet concentration", ci_mg_l)?;
    let r = split(r)?;
    let limit = nonnegative("limit", limit_mg_l)?;
    if ci <= 0.0 {
        return Err(invalid(
            "required capture undefined for zero inlet concentration",
        ));
    }
    Ok((1.0 - limit * (1.0 - r) / ci).max(0.0))
}

/// Oil mass balance of one test in kg. `reject` leaves in the overflow; `treated` leaves in the
/// underflow; `in_domain` is retained or wall-deposited mass with no recorded exit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OilInventory {
    pub injected_kg: f64,
    pub reject_kg: f64,
    pub treated_kg: f64,
    pub in_domain_kg: f64,
}

/// Capture and concentration bounds. Unresolved mass is assigned adversely, so the upper
/// concentration uses the lower capture and vice versa.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaptureBounds {
    pub capture_lower: f64,
    pub capture_upper: f64,
    pub cu_upper_mg_l: f64,
    pub cu_lower_mg_l: f64,
    pub closure_fraction: f64,
}

impl OilInventory {
    pub fn bounds(&self, ci_mg_l: f64, r: f64) -> Result<CaptureBounds, JudgeError> {
        let injected = self.injected_kg;
        if !injected.is_finite() || injected <= 0.0 {
            return Err(invalid("injected oil mass must be finite and positive"));
        }
        for (name, x) in [
            ("reject", self.reject_kg),
            ("treated", self.treated_kg),
            ("in-domain", self.in_domain_kg),
        ] {
            nonnegative(name, x)?;
        }
        let closure = injected - (self.reject_kg + self.treated_kg + self.in_domain_kg);
        let residual = closure.abs();
        // Confirmed reject mass minus any unclosed residual is the adverse capture.
        let capture_lower = ((self.reject_kg - residual) / injected).clamp(0.0, 1.0);
        // Every unexplained kilogram is credited as capture only for the favourable bound.
        let capture_upper =
            ((self.reject_kg + self.in_domain_kg + residual) / injected).clamp(0.0, 1.0);
        Ok(CaptureBounds {
            capture_lower,
            capture_upper,
            cu_upper_mg_l: outlet_concentration_mg_l(ci_mg_l, capture_lower, r)?,
            cu_lower_mg_l: outlet_concentration_mg_l(ci_mg_l, capture_upper, r)?,
            closure_fraction: residual / injected,
        })
    }
}

/// A treated-stream concentration from one synchronised time window: oil and water are taken
/// over the same interval, so the two quantities cannot come from different windows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SynchronizedWindow {
    pub start_s: f64,
    pub end_s: f64,
    pub treated_oil_kg: f64,
    pub treated_water_m3: f64,
}

/// Transient outlet concentration `m_oil,treated / V_water,treated` in mg/L.
pub fn transient_outlet_concentration_mg_l(w: &SynchronizedWindow) -> Result<f64, JudgeError> {
    if !(w.start_s.is_finite() && w.end_s.is_finite() && w.end_s > w.start_s) {
        return Err(invalid("window must be finite with end after start"));
    }
    nonnegative("treated oil mass", w.treated_oil_kg)?;
    if !w.treated_water_m3.is_finite() || w.treated_water_m3 <= 0.0 {
        return Err(invalid("treated water volume must be finite and positive"));
    }
    nonnegative(
        "transient concentration",
        w.treated_oil_kg / w.treated_water_m3 * MG_L_PER_KG_M3,
    )
}

/// Basis of a size distribution. Only oil-mass and oil-volume weights feed the concentration
/// integral; number and area bases need an explicit volume-conserving conversion first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DsdBasis {
    OilMass,
    OilVolume,
    Number,
    Area,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DsdBin {
    pub lower_um: f64,
    pub upper_um: f64,
    pub weight: f64,
    /// Raw capture `E(d)` of this inlet class.
    pub capture: f64,
}

/// `Cu = Ci * sum_d (1 - E_reduced(d)) w(d)`, equal to `Ci (1 - sum_d E(d) w(d)) / (1 - r)`.
pub fn cohort_outlet_concentration_mg_l(
    ci_mg_l: f64,
    r: f64,
    basis: DsdBasis,
    bins: &[DsdBin],
) -> Result<f64, JudgeError> {
    if !matches!(basis, DsdBasis::OilMass | DsdBasis::OilVolume) {
        return Err(invalid(
            "concentration needs an oil-mass or oil-volume basis",
        ));
    }
    if bins.is_empty() {
        return Err(invalid("size distribution has no bins"));
    }
    let mut total = 0.0;
    let mut previous_upper: Option<f64> = None;
    for bin in bins {
        if !(bin.lower_um.is_finite() && bin.upper_um.is_finite() && bin.lower_um < bin.upper_um) {
            return Err(invalid("bin edges must be finite with lower below upper"));
        }
        if previous_upper.is_some_and(|u| bin.lower_um < u) {
            return Err(invalid("bins must be ordered and non-overlapping"));
        }
        previous_upper = Some(bin.upper_um);
        nonnegative("bin weight", bin.weight)?;
        total += bin.weight;
    }
    if (total - 1.0).abs() > NORMALISATION_TOLERANCE {
        return Err(invalid("bin weights must be normalised to one"));
    }
    let mut integral = 0.0;
    for bin in bins {
        let reduced = reduced_efficiency(bin.capture, r)?;
        integral += (1.0 - reduced) * bin.weight;
    }
    Ok(nonnegative("inlet concentration", ci_mg_l)? * integral)
}

/// Mass transfer row for one inlet bin: its injected mass leaves to the reject, the treated
/// stream, or stays in the domain. Daughters are outlet states of the parent, never new mass.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferRow {
    pub reject_kg: f64,
    pub treated_kg: f64,
    pub in_domain_kg: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferMatrix {
    pub parent_kg: Vec<f64>,
    pub rows: Vec<TransferRow>,
}

impl TransferMatrix {
    /// Every row must conserve its parent's injected mass exactly; any surplus outlet mass
    /// (a daughter counted as well as its parent) is refused.
    pub fn check(&self) -> Result<(), JudgeError> {
        if self.parent_kg.is_empty() || self.parent_kg.len() != self.rows.len() {
            return Err(invalid("one transfer row is required per parent bin"));
        }
        for (parent, row) in self.parent_kg.iter().zip(&self.rows) {
            nonnegative("parent mass", *parent)?;
            for (name, x) in [
                ("reject", row.reject_kg),
                ("treated", row.treated_kg),
                ("in-domain", row.in_domain_kg),
            ] {
                nonnegative(name, x)?;
            }
            let outlets = row.reject_kg + row.treated_kg + row.in_domain_kg;
            if !outlets.is_finite() || (outlets - parent).abs() > 64.0 * f64::EPSILON * parent {
                return Err(invalid("transfer row does not conserve its parent mass"));
            }
        }
        Ok(())
    }

    /// Injected mass is the parent sum only.
    pub fn inventory(&self) -> Result<OilInventory, JudgeError> {
        self.check()?;
        let sum = |f: fn(&TransferRow) -> f64| self.rows.iter().map(f).sum::<f64>();
        Ok(OilInventory {
            injected_kg: self.parent_kg.iter().sum(),
            reject_kg: sum(|r| r.reject_kg),
            treated_kg: sum(|r| r.treated_kg),
            in_domain_kg: sum(|r| r.in_domain_kg),
        })
    }
}

/// One port of the control volume. `alpha` is the kinetic-energy flux factor (`>= 1`); a
/// nonuniform profile needs its integrated flux, not a guessed factor.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Port {
    pub q_m3_s: f64,
    pub area_m2: f64,
    pub static_gauge_pa: f64,
    pub z_m: f64,
    pub alpha: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlVolume {
    pub density_kg_m3: f64,
    pub gravity_m_s2: f64,
    pub inlet: Port,
    pub overflow: Port,
    pub underflow: Port,
}

/// The frozen common duty. Every energy value is tied to this identity and split.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenDuty {
    pub id: String,
    pub feed_m3_h: f64,
    pub split: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnergyBasis {
    /// Integrated port-flux loss divided by the treated volume flow.
    TreatedVolumeFluxLoss,
    /// Legacy `static_pressure_work_per_feed_m3`: static pressure drop referenced to feed.
    StaticPressureWorkPerFeedM3Historical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnergyClaim {
    pub basis: EnergyBasis,
    pub duty_id: String,
    pub value_j_per_m3: f64,
}

impl Port {
    /// Mechanical energy flux `q (p + alpha rho v^2 / 2 + rho g z)` in W, with gauge pressure.
    fn flux_w(&self, rho: f64, g: f64) -> Result<f64, JudgeError> {
        if !(self.q_m3_s.is_finite() && self.q_m3_s >= 0.0) {
            return Err(invalid("port flow must be finite and nonnegative"));
        }
        if !(self.area_m2.is_finite() && self.area_m2 > 0.0) {
            return Err(invalid("port area must be finite and positive"));
        }
        if !(self.alpha.is_finite() && self.alpha >= 1.0) {
            return Err(invalid(
                "kinetic factor alpha must be finite and at least one",
            ));
        }
        if !(self.static_gauge_pa.is_finite() && self.z_m.is_finite()) {
            return Err(invalid("port pressure and elevation must be finite"));
        }
        let v = self.q_m3_s / self.area_m2;
        let total_pressure =
            self.static_gauge_pa + self.alpha * rho * v * v / 2.0 + rho * g * self.z_m;
        Ok(self.q_m3_s * total_pressure)
    }
}

/// Mechanical energy flux loss per treated cubic metre at the frozen common duty.
/// The loss is `inlet flux - overflow flux - underflow flux` and is divided by the underflow
/// (treated) flow, never by the feed flow.
pub fn energy_flux_loss_per_treated_m3(
    cv: &ControlVolume,
    duty: &FrozenDuty,
) -> Result<EnergyClaim, JudgeError> {
    if duty.id.trim().is_empty() || !duty.feed_m3_h.is_finite() || duty.feed_m3_h <= 0.0 {
        return Err(invalid(
            "frozen duty requires identity and finite positive feed",
        ));
    }
    split(duty.split)?;
    if !(cv.density_kg_m3.is_finite() && cv.density_kg_m3 > 0.0 && cv.gravity_m_s2.is_finite()) {
        return Err(invalid("density must be positive and gravity finite"));
    }
    let q_in = cv.inlet.q_m3_s;
    if !(q_in.is_finite() && q_in > 0.0) {
        return Err(invalid("feed flow must be finite and positive"));
    }
    let q_out = cv.overflow.q_m3_s + cv.underflow.q_m3_s;
    if (q_in - q_out).abs() > NORMALISATION_TOLERANCE * q_in {
        return Err(invalid(
            "port flows do not close the incompressible volume balance",
        ));
    }
    let feed_m3_h = q_in * 3600.0;
    if (feed_m3_h - duty.feed_m3_h).abs() > NORMALISATION_TOLERANCE * duty.feed_m3_h
        || (cv.overflow.q_m3_s / q_in - duty.split).abs() > NORMALISATION_TOLERANCE
    {
        return Err(invalid("port flows do not match the frozen common duty"));
    }
    let g = cv.gravity_m_s2;
    let rho = cv.density_kg_m3;
    let loss_w =
        cv.inlet.flux_w(rho, g)? - cv.overflow.flux_w(rho, g)? - cv.underflow.flux_w(rho, g)?;
    if !loss_w.is_finite() || loss_w < -NORMALISATION_TOLERANCE * q_in * rho {
        return Err(invalid(
            "negative flux loss needs a work input inside the boundary",
        ));
    }
    let value = loss_w.max(0.0) / cv.underflow.q_m3_s;
    if !value.is_finite() {
        return Err(invalid("energy per treated volume overflow"));
    }
    Ok(EnergyClaim {
        basis: EnergyBasis::TreatedVolumeFluxLoss,
        duty_id: duty.id.clone(),
        value_j_per_m3: value,
    })
}

/// Legacy label kept for comparison. It is never admitted as the treated-volume objective.
pub fn historical_static_pressure_work_per_feed_m3(
    dp_static_pa: f64,
    duty_id: &str,
) -> EnergyClaim {
    EnergyClaim {
        basis: EnergyBasis::StaticPressureWorkPerFeedM3Historical,
        duty_id: duty_id.to_string(),
        value_j_per_m3: dp_static_pa,
    }
}

/// Admits only a treated-volume energy claim. A per-feed value labelled as treated is refused.
pub fn admit_treated_energy(claim: &EnergyClaim) -> Result<f64, JudgeError> {
    if claim.basis != EnergyBasis::TreatedVolumeFluxLoss {
        return Err(invalid(
            "energy must be per treated cubic metre from integrated port fluxes",
        ));
    }
    if claim.duty_id.trim().is_empty() {
        return Err(invalid("energy claim must name its frozen duty"));
    }
    nonnegative("energy per treated m3", claim.value_j_per_m3)
}
