//! Conditional analytic stress, modal and manufacturing screens, plus numerical oracles.
//!
//! Implements deterministic verification and screening for Packet `P08` (`STRUCTURE-01`..`STRUCTURE-05`,
//! `CAD-08`, `CAD-09`):
//! - `STRUCTURE-01`: Analytical Lamé thick-wall cylinder elasticity oracle + 3D harmonic-lobe & S-cone
//!   stress concentration ($K_t$) von Mises stress under external vessel pressure ($\Delta p = 2.5\,\text{MPa}$).
//! - `STRUCTURE-02`: Independent integration of supplied traction and solved reaction fields.
//! - `STRUCTURE-03`: Four-level axial integration order-stability diagnostic.
//! - `STRUCTURE-04`: Internal splitter-vane hydrodynamic bending + Strouhal vortex-shedding (VIV) frequency
//!   lock-in margin ($f_1 / f_{\text{shed}} \ge 3.0$), and vortex-finder skimmer cantilever tip deflection
//!   ($\delta_{\text{tip}} \le 0.010\,\text{mm}$) + precessing-vortex-core (PVC) modal margin.
//! - `STRUCTURE-05` / `CAD-08` / `CAD-09`: LPBF Ti-6Al-4V overhang angle ($\le 45^\circ$ from vertical $+z$),
//!   powder-evacuation bore check, and explicit sour-service / as-built qualification boundary.

use crate::geometry::baseline;
use crate::phenotype::Phenotype3D;
use crate::program::DesignProgram;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::f64::consts::PI;

/// Provisional LPBF Ti-6Al-4V (Grade 5, stress-relieved / HIPed) reference properties.
pub const TI64_YIELD_STRENGTH_MPA: f64 = 825.0;
pub const TI64_HCF_ENDURANCE_SEAWATER_MPA: f64 = 120.0;
pub const TI64_YOUNGS_MODULUS_PA: f64 = 113.8e9;
pub const TI64_POISSON_RATIO: f64 = 0.342;
pub const TI64_DENSITY_KG_M3: f64 = 4430.0;
pub const WATER_DENSITY_KG_M3: f64 = 998.0;

/// Design external vessel overpressure (2.5 MPa) and minimum structural safety factor.
pub const DESIGN_VESSEL_DELTA_P_PA: f64 = 2.5e6;
pub const MIN_YIELD_SAFETY_FACTOR: f64 = 2.5;
pub const MIN_MODAL_FREQUENCY_RATIO: f64 = 3.0;
pub const MAX_SKIMMER_TIP_DEFLECTION_MM: f64 = 0.010;
pub const MAX_LPBF_OVERHANG_FROM_VERTICAL_DEG: f64 = 45.0;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GciState {
    Monotone,
    Oscillatory,
    Divergent,
    Asymptotic,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GciReport {
    pub state: GciState,
    pub observed_order: Option<f64>,
    pub gci_fine_rel: Option<f64>,
    pub reason: String,
}

/// Equal-ratio three-grid diagnostic, ordered coarse to fine. Three values alone
/// cannot independently establish an asymptotic range, so no GCI is certified.
pub fn three_grid_order(values: [f64; 3], refinement_ratio: f64) -> Result<GciReport, String> {
    if values.iter().any(|x| !x.is_finite())
        || !refinement_ratio.is_finite()
        || refinement_ratio <= 1.0
    {
        return Err("finite grid values and refinement ratio > 1 required".into());
    }
    let coarse_difference = values[0] - values[1];
    let fine_difference = values[1] - values[2];
    let floor = 64.0
        * f64::EPSILON
        * values
            .iter()
            .copied()
            .map(f64::abs)
            .fold(f64::MIN_POSITIVE, f64::max);
    let (state, order, reason) =
        if coarse_difference.abs() <= floor || fine_difference.abs() <= floor {
            (
                GciState::Indeterminate,
                None,
                "grid differences are not resolved above roundoff",
            )
        } else if coarse_difference.is_sign_positive() != fine_difference.is_sign_positive() {
            (GciState::Oscillatory, None, "grid differences change sign")
        } else {
            let order =
                (coarse_difference.abs().ln() - fine_difference.abs().ln()) / refinement_ratio.ln();
            if !order.is_finite() || order <= 0.0 {
                (
                    GciState::Divergent,
                    Some(order),
                    "grid differences do not decrease",
                )
            } else {
                (
                    GciState::Monotone,
                    Some(order),
                    "asymptotic range needs an independent fourth grid",
                )
            }
        };
    Ok(GciReport {
        state,
        observed_order: order,
        gci_fine_rel: None,
        reason: reason.into(),
    })
}

/// Test order stability on overlapping triples before issuing a conditional GCI.
/// Sampling, temporal and model errors remain outside this spatial diagnostic.
pub fn four_grid_gci(
    values: [f64; 4],
    refinement_ratio: f64,
    relative_order_tolerance: f64,
) -> Result<GciReport, String> {
    if !relative_order_tolerance.is_finite() || relative_order_tolerance <= 0.0 {
        return Err("positive finite order-stability tolerance required".into());
    }
    let previous = three_grid_order([values[0], values[1], values[2]], refinement_ratio)?;
    let mut fine = three_grid_order([values[1], values[2], values[3]], refinement_ratio)?;
    if previous.state == GciState::Monotone && fine.state == GciState::Monotone {
        let old_order = previous.observed_order.unwrap();
        let order = fine.observed_order.unwrap();
        if (old_order - order).abs() <= relative_order_tolerance * order.abs() && values[3] != 0.0 {
            let gci = 1.25 * (values[3] - values[2]).abs()
                / (values[3].abs() * (refinement_ratio.powf(order) - 1.0));
            if gci.is_finite() {
                fine.state = GciState::Asymptotic;
                fine.gci_fine_rel = Some(gci);
                fine.reason = "order agrees on overlapping triples within declared tolerance; spatial diagnostic only".into();
            }
        } else {
            fine.reason =
                "observed order is not stable on overlapping triples or fine value is zero".into();
        }
    }
    Ok(fine)
}

/// Source quadrature points in a common Cartesian frame, SI units. The normal
/// defines traction `-pressure * normal + viscous_traction` applied to the solid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceTraction {
    pub position_m: [f64; 3],
    pub normal: [f64; 3],
    pub area_m2: f64,
    pub pressure_pa: f64,
    pub viscous_traction_pa: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodalReaction {
    pub position_m: [f64; 3],
    pub force_n: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadTransferAudit {
    pub source_force_n: [f64; 3],
    pub source_moment_n_m: [f64; 3],
    pub reaction_force_n: [f64; 3],
    pub reaction_moment_n_m: [f64; 3],
    pub force_residual_n: f64,
    pub moment_residual_n_m: f64,
    pub passed: bool,
}

fn resultant(
    samples: impl Iterator<Item = ([f64; 3], [f64; 3])>,
    origin_m: [f64; 3],
) -> ([f64; 3], [f64; 3]) {
    let mut force = [0.0; 3];
    let mut moment = [0.0; 3];
    for (position, applied) in samples {
        let r: [f64; 3] = std::array::from_fn(|i| position[i] - origin_m[i]);
        for i in 0..3 {
            force[i] += applied[i];
            moment[i] +=
                r[(i + 1) % 3] * applied[(i + 2) % 3] - r[(i + 2) % 3] * applied[(i + 1) % 3];
        }
    }
    (force, moment)
}

/// Independently integrate supplied source tractions and solved nodal reactions.
/// No reaction values are synthesized here. Positive tolerances are caller policy.
pub fn audit_load_transfer(
    source: &[SurfaceTraction],
    reactions: &[NodalReaction],
    origin_m: [f64; 3],
    relative_tolerance: f64,
    force_absolute_tolerance_n: f64,
    moment_absolute_tolerance_n_m: f64,
) -> Result<LoadTransferAudit, String> {
    if source.is_empty()
        || reactions.is_empty()
        || origin_m.iter().any(|x| !x.is_finite())
        || [
            relative_tolerance,
            force_absolute_tolerance_n,
            moment_absolute_tolerance_n_m,
        ]
        .iter()
        .any(|x| !x.is_finite() || *x < 0.0)
        || source.iter().any(|s| {
            !s.area_m2.is_finite()
                || s.area_m2 <= 0.0
                || !s.pressure_pa.is_finite()
                || s.position_m
                    .iter()
                    .chain(&s.normal)
                    .chain(&s.viscous_traction_pa)
                    .any(|x| !x.is_finite())
                || (s.normal.iter().map(|x| x * x).sum::<f64>() - 1.0).abs() > 1e-9
        })
        || reactions.iter().any(|r| {
            r.position_m
                .iter()
                .chain(&r.force_n)
                .any(|x| !x.is_finite())
        })
    {
        return Err(
            "finite SI traction/reaction fields, unit normals and nonnegative tolerances required"
                .into(),
        );
    }
    let (source_force_n, source_moment_n_m) = resultant(
        source.iter().map(|s| {
            (
                s.position_m,
                std::array::from_fn(|i| {
                    (-s.pressure_pa * s.normal[i] + s.viscous_traction_pa[i]) * s.area_m2
                }),
            )
        }),
        origin_m,
    );
    let (reaction_force_n, reaction_moment_n_m) = resultant(
        reactions.iter().map(|r| (r.position_m, r.force_n)),
        origin_m,
    );
    let norm = |v: [f64; 3]| v[0].hypot(v[1]).hypot(v[2]);
    let force_residual_n = norm(std::array::from_fn(|i| {
        source_force_n[i] + reaction_force_n[i]
    }));
    let moment_residual_n_m = norm(std::array::from_fn(|i| {
        source_moment_n_m[i] + reaction_moment_n_m[i]
    }));
    if [
        force_residual_n,
        moment_residual_n_m,
        norm(source_force_n),
        norm(source_moment_n_m),
    ]
    .iter()
    .any(|x| !x.is_finite())
    {
        return Err("load integration overflow".into());
    }
    Ok(LoadTransferAudit {
        source_force_n,
        source_moment_n_m,
        reaction_force_n,
        reaction_moment_n_m,
        force_residual_n,
        moment_residual_n_m,
        passed: force_residual_n
            <= force_absolute_tolerance_n + relative_tolerance * norm(source_force_n)
            && moment_residual_n_m
                <= moment_absolute_tolerance_n_m + relative_tolerance * norm(source_moment_n_m),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LameCylinderState {
    pub r_eval_m: f64,
    pub sigma_r_pa: f64,
    pub sigma_theta_pa: f64,
    pub sigma_z_pa: f64,
    pub von_mises_pa: f64,
    pub radial_displacement_m: f64,
}

/// Version label of the analytic estimate (`V5-01-05`).
pub const ANALYTIC_STRUCTURAL_ESTIMATE_KIND: &str =
    "analytic_fixed_load_structural_estimate_v2_not_solid_solve_or_qualification";
/// No buckling analysis exists; a passing estimate says nothing about buckling (`V5-01-05`).
pub const BUCKLING_NOT_EVALUATED: &str = "not_evaluated_no_buckling_analysis";

fn legacy_structural_estimate_kind() -> String {
    "legacy_record_analytic_fixed_load_estimate_kind_unrecorded".to_string()
}

fn buckling_not_evaluated() -> String {
    BUCKLING_NOT_EVALUATED.to_string()
}

/// Closed-form wall, beam and modal estimates under a fixed external demand. `passed` means only
/// that these estimates cleared their screen limits; it is not structural qualification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticStructuralScreen {
    pub genome_sha256: String,
    pub load_case_sha256: String,
    pub passed: bool,
    #[serde(default = "legacy_structural_estimate_kind")]
    pub estimate_kind: String,
    #[serde(default = "buckling_not_evaluated")]
    pub buckling_status: String,
    /// Hydraulic pressure drop supplied by the caller; several callers pass the CT-35 value for
    /// every candidate, so it is recorded rather than implied to be candidate-specific.
    #[serde(default)]
    pub hydraulic_dp_u_pa_input: Option<f64>,
    pub material_designation: String,
    pub sour_service_status: String,
    pub screen_status: String,
    pub design_vessel_delta_p_pa: f64,
    pub max_wall_von_mises_mpa: f64,
    pub min_yield_safety_factor: f64,
    pub stress_gci_fine_rel: Option<f64>,
    pub stress_convergence: GciReport,
    pub estimated_axial_pressure_thrust_n: f64,
    pub estimated_swirl_torque_n_m: f64,
    pub load_transfer_audit: Option<LoadTransferAudit>,
    pub load_transfer_status: String,
    pub vane_bending_stress_mpa: f64,
    pub vane_natural_freq_hz: f64,
    pub vane_strouhal_shedding_freq_hz: f64,
    pub vane_modal_freq_ratio: f64,
    pub skimmer_tip_deflection_mm: f64,
    pub skimmer_root_stress_mpa: f64,
    pub skimmer_natural_freq_hz: f64,
    pub pvc_excitation_freq_hz: f64,
    pub skimmer_modal_freq_ratio: f64,
    pub max_lpbf_overhang_from_vertical_deg: f64,
    pub min_powder_evacuation_dh_mm: f64,
    pub failure_reasons: Vec<String>,
}

pub fn structural_load_case_hash(
    genome_sha256: &str,
    dp_u_pa: f64,
    q_in_m3_s: f64,
) -> Result<String, String> {
    if !dp_u_pa.is_finite() || dp_u_pa <= 0.0 || !q_in_m3_s.is_finite() || q_in_m3_s <= 0.0 {
        return Err("finite positive candidate hydraulic loads required".into());
    }
    let wire = serde_json::to_vec(&(
        "analytic-structural-load-v1",
        genome_sha256,
        dp_u_pa,
        q_in_m3_s,
        DESIGN_VESSEL_DELTA_P_PA,
    ))
    .map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(wire)))
}

/// Four-level axial integration order-stability (`[int_128, int_256, int_512, int_1024]`) and structural screen.
pub mod screen;
pub use screen::*;
