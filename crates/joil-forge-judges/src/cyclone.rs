//! Hydrocyclone cell geometry, swirl decay and energy-consistent pressure drops.
//!
//! Per cell, with feed `Q`, overflow split `r` and wall profile `R(z)`:
//!
//! * inlet jet velocity `v_in = Q / A_in`, initial circulation `Γ₀ = α v_in r_inj`,
//!   `α = 1 - k (b/R₀)^½` (slot width `b`; involute entries use `0.7 k`);
//! * forward flow `Q_f(z) = Q - rQ z/L` (the reverse core is fed uniformly along `L`);
//! * angular-momentum balance with wall shear `τ = ½ c_f ρ v_t²`, `v_t = Γ/R`:
//!   `dΓ/dz = -π c_f Γ² / Q_f`, integrated exactly per step for frozen `c_f, Q_f`;
//! * Burgers vortex `v_t(r) = Γ/r · (1 - e^{-r²/r_c²})`, `r_c = κ R`.
//!
//! Underflow pressure drop is a mechanical-energy budget: inlet contraction and jet
//! mixing (Borda–Carnot on `(1-α) v_in`), wall-friction dissipation `∫ π c_f ρ Γ³/R² dz / Q_f`,
//! axial friction, and the swirl and axial kinetic energy discharged at the underflow
//! (dissipated downstream, so not recovered). Overflow pressure drop adds the Burgers
//! core deficit `ρΓ₀²/(2r_c²) ∫(1-e^{-u})²/u² du` and the discharged orifice dynamic head.

use crate::closure::Closure;
use crate::fluid::Fluids;
use crate::math::{burgers_core_integral, burgers_energy_integral, darcy_friction};
use crate::{finite, JudgeError, Result};
use serde::{Deserialize, Serialize};

/// Physical description of one hydrocyclone unit made of `cells` identical parallel cells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CycloneSpec {
    pub cells: u32,
    /// Wall profile stations `[z, R]` in metres from the roof (`z = 0`) to the underflow exit.
    pub stations: Vec<[f64; 2]>,
    pub inlet_jets: u32,
    /// Total inlet area per cell (m²).
    pub inlet_area_m2: f64,
    /// Jet height / width.
    pub inlet_aspect: f64,
    pub involute: bool,
    pub overflow_d_m: f64,
    /// Overflow / inlet volumetric split set-point per cell.
    pub split: f64,
    pub roughness_m: f64,
}

/// Discretisation of the axial march and of the flow-fraction coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    pub nz: usize,
    pub ns: usize,
}

impl Default for Resolution {
    /// Search resolution. The capture of the CT-35 reconstruction changes by less than
    /// 0.2 % between this and a 16x finer march (see `tests/numerics.rs`).
    fn default() -> Self {
        Self { nz: 60, ns: 12 }
    }
}

impl Resolution {
    /// Resolution used for reports and final elite re-evaluation.
    pub fn report() -> Self {
        Self { nz: 160, ns: 24 }
    }
}

/// Minimum printed or machined dimension accepted anywhere in a cell (m).
pub const MIN_FEATURE_M: f64 = 1.0e-3;

/// Declared applicability of the swirl model: steepest converging wall (half-angle) and
/// steepest diverging wall. Beyond these, separation, vortex breakdown and contraction
/// losses that the model does not represent would dominate.
pub const MAX_CONTRACTION_DEG: f64 = 25.0;
pub const MAX_EXPANSION_DEG: f64 = 8.0;

impl CycloneSpec {
    pub fn length(&self) -> f64 {
        self.stations.last().map(|s| s[0]).unwrap_or(0.0)
    }

    pub fn roof_radius(&self) -> f64 {
        self.stations[0][1]
    }

    pub fn exit_radius(&self) -> f64 {
        self.stations.last().map(|s| s[1]).unwrap_or(0.0)
    }

    pub fn max_radius(&self) -> f64 {
        self.stations.iter().fold(0.0, |m, s| m.max(s[1]))
    }

    /// Jet width and height (m).
    pub fn jet_size(&self) -> (f64, f64) {
        let a_jet = self.inlet_area_m2 / self.inlet_jets.max(1) as f64;
        let w = (a_jet / self.inlet_aspect).sqrt();
        (w, w * self.inlet_aspect)
    }

    /// Wall radius at axial position `z` by linear interpolation.
    pub fn radius_at(&self, z: f64) -> f64 {
        let s = &self.stations;
        if z <= s[0][0] {
            return s[0][1];
        }
        for w in s.windows(2) {
            if z <= w[1][0] {
                let f = (z - w[0][0]) / (w[1][0] - w[0][0]);
                return w[0][1] + f * (w[1][1] - w[0][1]);
            }
        }
        s[s.len() - 1][1]
    }

    /// Fluid volume of one cell (m³), exact for the piecewise-linear profile.
    pub fn cell_volume(&self) -> f64 {
        self.stations
            .windows(2)
            .map(|w| {
                let (h, a, b) = (w[1][0] - w[0][0], w[0][1], w[1][1]);
                std::f64::consts::PI * h * (a * a + a * b + b * b) / 3.0
            })
            .sum()
    }

    /// Wetted wall area of one cell (m²), conical frusta.
    pub fn wall_area(&self) -> f64 {
        let lateral: f64 = self
            .stations
            .windows(2)
            .map(|w| {
                let (h, a, b) = (w[1][0] - w[0][0], w[0][1], w[1][1]);
                std::f64::consts::PI * (a + b) * (h * h + (a - b) * (a - b)).sqrt()
            })
            .sum();
        lateral + std::f64::consts::PI * self.roof_radius().powi(2)
    }

    /// Steepest converging and diverging wall half-angles (degrees).
    pub fn wall_angles(&self) -> (f64, f64) {
        let (mut conv, mut div) = (0.0_f64, 0.0_f64);
        for w in self.stations.windows(2) {
            let ang = ((w[1][1] - w[0][1]) / (w[1][0] - w[0][0]).max(1e-12))
                .atan()
                .to_degrees();
            if ang < 0.0 {
                conv = conv.max(-ang);
            } else {
                div = div.max(ang);
            }
        }
        (conv, div)
    }

    /// Applicability envelope of the reduced-order model.
    pub fn applicability(&self) -> Result<()> {
        let (conv, div) = self.wall_angles();
        if conv > MAX_CONTRACTION_DEG + 1e-9 || div > MAX_EXPANSION_DEG + 1e-9 {
            return Err(JudgeError::Applicability(format!(
                "wall half-angles {conv:.1}° converging / {div:.1}° diverging exceed {MAX_CONTRACTION_DEG}° / {MAX_EXPANSION_DEG}°"
            )));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        let geo = |m: &str| Err(JudgeError::Geometry(m.to_string()));
        if self.cells == 0 || self.cells > 64 {
            return geo("cyclone cells must be in 1..=64");
        }
        if self.stations.len() < 2 || self.stations[0][0] != 0.0 {
            return geo("cyclone profile needs >= 2 stations starting at z = 0");
        }
        for w in self.stations.windows(2) {
            if w[1][0].is_nan() || w[1][0] <= w[0][0] {
                return geo("cyclone stations must strictly increase in z");
            }
        }
        if self
            .stations
            .iter()
            .any(|s| s[1].is_nan() || s[1] < 1.5 * MIN_FEATURE_M || !s[0].is_finite())
        {
            return geo("cyclone wall radius below printable passage");
        }
        if self.inlet_jets == 0 || self.inlet_jets > 12 {
            return geo("inlet jets must be in 1..=12");
        }
        if self.inlet_area_m2.is_nan()
            || self.inlet_area_m2 <= 0.0
            || !(0.2..=8.0).contains(&self.inlet_aspect)
        {
            return geo("inlet area must be positive and aspect within [0.2, 8]");
        }
        let (w, h) = self.jet_size();
        if w < MIN_FEATURE_M || h < MIN_FEATURE_M {
            return geo("inlet jet narrower than the minimum feature");
        }
        if w > 0.6 * self.roof_radius() {
            return geo("inlet jet wider than 0.6 roof radius");
        }
        if h > self.stations[1][0] + 1e-12 {
            return geo("inlet jet taller than the first profile segment");
        }
        if !(MIN_FEATURE_M..=1.6 * self.roof_radius()).contains(&self.overflow_d_m) {
            return geo("overflow orifice outside [1 mm, 0.8 roof diameter]");
        }
        if !(self.split > 0.0 && self.split < 0.95) {
            return Err(JudgeError::Input(
                "cyclone split must lie in (0, 0.95)".into(),
            ));
        }
        if self.roughness_m.is_nan() || self.roughness_m < 0.0 {
            return Err(JudgeError::Input("roughness must be non-negative".into()));
        }
        Ok(())
    }
}

/// Axial swirl solution of one cell.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SwirlProfile {
    pub z_m: Vec<f64>,
    pub r_wall_m: Vec<f64>,
    pub gamma_m2_s: Vec<f64>,
    pub q_forward_m3_s: Vec<f64>,
}

/// Mechanical-energy pressure budget of one cell (Pa).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PressureBudget {
    pub inlet_contraction: f64,
    pub jet_mixing: f64,
    pub wall_friction: f64,
    pub exit_swirl: f64,
    pub exit_axial: f64,
    pub roof_swirl: f64,
    pub core_deficit: f64,
    pub orifice: f64,
}

/// Hydraulic state of one cell at a given cell feed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CycloneHydraulics {
    pub q_cell_m3_s: f64,
    pub inlet_velocity_m_s: f64,
    pub alpha: f64,
    pub gamma0_m2_s: f64,
    pub dp_underflow_pa: f64,
    pub dp_overflow_pa: f64,
    pub budget: PressureBudget,
    pub eps_inlet_w_kg: f64,
    pub eps_mean_w_kg: f64,
    pub eps_peak_w_kg: f64,
    pub residence_s: f64,
    pub cell_volume_m3: f64,
    /// Peak centripetal acceleration at the wall, in multiples of g.
    pub peak_wall_g: f64,
    pub swirl: SwirlProfile,
}

/// Axial nodes of the march: every profile station is a node, and steps are at most
/// `L / nz` and at most the local wall diameter, so throats and steep cones are resolved.
pub fn axial_nodes(spec: &CycloneSpec, nz: usize) -> Vec<f64> {
    let len = spec.length();
    let base = len / nz.max(8) as f64;
    let mut nodes = vec![0.0];
    for w in spec.stations.windows(2) {
        let (z0, z1) = (w[0][0], w[1][0]);
        let r_min = w[0][1].min(w[1][1]);
        let step = base.min(2.0 * r_min).max(1e-6);
        let n = ((z1 - z0) / step).ceil().clamp(1.0, 4000.0) as usize;
        for i in 1..=n {
            nodes.push(z0 + (z1 - z0) * i as f64 / n as f64);
        }
    }
    nodes
}

/// Solve swirl decay and the pressure budget of one cell at cell feed `q_cell`.
pub fn hydraulics(
    spec: &CycloneSpec,
    q_cell: f64,
    fluids: &Fluids,
    closure: &Closure,
    res: Resolution,
) -> Result<CycloneHydraulics> {
    spec.validate()?;
    spec.applicability()?;
    if !q_cell.is_finite() || q_cell <= 0.0 {
        return Err(JudgeError::Input(
            "cyclone cell flow must be positive".into(),
        ));
    }
    let rho = fluids.rho_water_kg_m3;
    let nu = fluids.nu_water();
    let pi = std::f64::consts::PI;
    let len = spec.length();
    let r0 = spec.roof_radius();
    let (w, _) = spec.jet_size();
    let v_in = q_cell / spec.inlet_area_m2;
    let k_alpha = closure.inlet_alpha_k * if spec.involute { 0.7 } else { 1.0 };
    let alpha = (1.0 - k_alpha * (w / r0).sqrt()).clamp(0.2, 0.98);
    let r_inj = (r0 - 0.5 * w).max(0.5 * r0);
    let gamma0 = alpha * v_in * r_inj;
    let q_over = spec.split * q_cell;
    let q_under = q_cell - q_over;
    let nodes = axial_nodes(spec, res.nz);
    let nz = nodes.len() - 1;

    let mut prof = SwirlProfile {
        z_m: Vec::with_capacity(nz + 1),
        r_wall_m: Vec::with_capacity(nz + 1),
        gamma_m2_s: Vec::with_capacity(nz + 1),
        q_forward_m3_s: Vec::with_capacity(nz + 1),
    };
    let mut gamma = gamma0;
    let (mut friction, mut axial) = (0.0, 0.0);
    let mut peak_g: f64 = 0.0;
    for k in 0..=nz {
        let z = nodes[k];
        let r = spec.radius_at(z);
        let qf = q_cell - q_over * z / len;
        prof.z_m.push(z);
        prof.r_wall_m.push(r);
        prof.gamma_m2_s.push(gamma);
        prof.q_forward_m3_s.push(qf);
        peak_g = peak_g.max((gamma / r).powi(2) / r / 9.80665);
        if k == nz {
            break;
        }
        let dz = nodes[k + 1] - z;
        let zm = z + 0.5 * dz;
        let rm = spec.radius_at(zm);
        let qm = q_cell - q_over * zm / len;
        let re_swirl = 2.0 * gamma / nu;
        let cf = closure.swirl_friction_mult
            * darcy_friction(re_swirl, spec.roughness_m / (2.0 * rm))
            / 4.0;
        let g_mid = gamma / (1.0 + 0.5 * pi * cf * gamma * dz / qm);
        friction += pi * cf * rho * g_mid.powi(3) / (rm * rm) * dz / qm;
        let vz = qm / (pi * rm * rm);
        let re_ax = vz * 2.0 * rm / nu;
        axial += darcy_friction(re_ax, spec.roughness_m / (2.0 * rm)) * dz / (2.0 * rm)
            * 0.5
            * rho
            * vz
            * vz;
        gamma /= 1.0 + pi * cf * gamma * dz / qm;
    }

    let r_exit = spec.exit_radius();
    let kappa = closure.core_radius_frac;
    let energy_factor = burgers_energy_integral(1.0 / (kappa * kappa));
    let exit_swirl = 0.5 * rho * (gamma / r_exit).powi(2) * energy_factor;
    let vz_exit = q_under / (pi * r_exit * r_exit);
    let dyn_in = 0.5 * rho * v_in * v_in;
    let rc0 = kappa * r0;
    let a_o = 0.25 * pi * spec.overflow_d_m * spec.overflow_d_m;
    let v_o = q_over / a_o;
    let budget = PressureBudget {
        inlet_contraction: dyn_in * closure.inlet_contraction_k,
        jet_mixing: dyn_in * (1.0 - alpha).powi(2),
        wall_friction: friction,
        exit_swirl,
        exit_axial: 0.5 * rho * vz_exit * vz_exit + axial,
        roof_swirl: 0.5 * rho * (gamma0 / r0).powi(2),
        core_deficit: rho * gamma0 * gamma0 / (2.0 * rc0 * rc0)
            * burgers_core_integral(1.0 / (kappa * kappa)),
        // Discharged orifice jet head; entry losses from the swirling core are neglected.
        orifice: 0.5 * rho * v_o * v_o,
    };
    let dp_under = budget.inlet_contraction
        + budget.jet_mixing
        + budget.wall_friction
        + budget.exit_swirl
        + budget.exit_axial;
    let dp_over = budget.inlet_contraction
        + budget.jet_mixing
        + budget.roof_swirl
        + budget.core_deficit
        + budget.orifice;

    let volume = spec.cell_volume();
    let a_jet = spec.inlet_area_m2 / spec.inlet_jets as f64;
    let v_jet = closure.jet_volume_factor * spec.inlet_jets as f64 * a_jet.powf(1.5);
    let eps_inlet = (budget.inlet_contraction + budget.jet_mixing) * q_cell / (rho * v_jet);
    let eps_mean = (dp_under * q_under + dp_over * q_over) / (rho * volume);
    let eps_peak = eps_inlet.max(closure.peak_dissipation_ratio * eps_mean);
    Ok(CycloneHydraulics {
        q_cell_m3_s: q_cell,
        inlet_velocity_m_s: v_in,
        alpha,
        gamma0_m2_s: gamma0,
        dp_underflow_pa: finite("dp_underflow", dp_under)?,
        dp_overflow_pa: finite("dp_overflow", dp_over)?,
        budget,
        eps_inlet_w_kg: eps_inlet,
        eps_mean_w_kg: eps_mean,
        eps_peak_w_kg: finite("eps_peak", eps_peak)?,
        residence_s: volume / q_cell,
        cell_volume_m3: volume,
        peak_wall_g: peak_g,
        swirl: prof,
    })
}
