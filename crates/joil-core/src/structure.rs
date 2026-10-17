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

/// Exact analytical Lamé thick-walled cylinder solution (`STRUCTURE-01` oracle).
///
/// - `a_m`: inner radius (m)
/// - `b_m`: outer radius (m)
/// - `p_i_pa`: internal gauge pressure (Pa)
/// - `p_o_pa`: external gauge pressure (Pa)
/// - `r_m`: evaluation radius in `[a_m, b_m]` (m)
pub fn lame_thick_cylinder_oracle(
    a_m: f64,
    b_m: f64,
    p_i_pa: f64,
    p_o_pa: f64,
    r_m: f64,
) -> Result<LameCylinderState, String> {
    if !a_m.is_finite() || !b_m.is_finite() || !r_m.is_finite() || a_m <= 0.0 || b_m <= a_m {
        return Err("invalid Lamé cylinder radii: require 0 < a_m < b_m".to_string());
    }
    if r_m < a_m * (1.0 - 1e-12) || r_m > b_m * (1.0 + 1e-12) {
        return Err("evaluation radius r_m outside [a_m, b_m]".to_string());
    }
    if !p_i_pa.is_finite() || !p_o_pa.is_finite() {
        return Err("non-finite pressure in Lamé cylinder oracle".to_string());
    }
    let a2 = a_m * a_m;
    let b2 = b_m * b_m;
    let r2 = r_m.clamp(a_m, b_m).powi(2);
    let denom = b2 - a2;

    let c1 = (a2 * p_i_pa - b2 * p_o_pa) / denom;
    let c2 = (a2 * b2 * (p_i_pa - p_o_pa)) / (denom * r2);

    let sigma_r_pa = c1 - c2;
    let sigma_theta_pa = c1 + c2;
    let sigma_z_pa = c1;

    let d1 = sigma_theta_pa - sigma_r_pa;
    let d2 = sigma_r_pa - sigma_z_pa;
    let d3 = sigma_z_pa - sigma_theta_pa;
    let von_mises_pa = (0.5 * (d1 * d1 + d2 * d2 + d3 * d3)).sqrt();

    let e = TI64_YOUNGS_MODULUS_PA;
    let nu = TI64_POISSON_RATIO;
    let u_r = (r_m / e) * ((1.0 - 2.0 * nu) * c1 + (1.0 + nu) * c2);

    Ok(LameCylinderState {
        r_eval_m: r_m,
        sigma_r_pa,
        sigma_theta_pa,
        sigma_z_pa,
        von_mises_pa,
        radial_displacement_m: u_r,
    })
}

/// Evaluates peak wall von Mises stress (MPa) across `n_z` axial stations for convergence auditing (`STRUCTURE-03`).
fn evaluate_axial_wall_stress_profile_mpa(
    prog: &DesignProgram,
    delta_p_vessel_pa: f64,
    n_z: usize,
) -> Result<(f64, f64), String> {
    let base = baseline();
    let dims = base.derive()?;
    let r1 = 0.5 * dims.d1_mm;
    let r2 = 0.5 * dims.d2_mm;
    let r3 = 0.5 * dims.d3_mm;
    let l1 = dims.chamber_length_mm;
    let zc = dims.z_cone_end_mm;
    let zt = dims.z_taper_end_mm;
    let z_max = dims.z_underflow_mm;
    let wall_nom_mm = base.parameters.wall_thickness_mm;

    let mut max_vm_pa = 0.0_f64;
    let mut integrated_vm_pa_m = 0.0_f64;
    let dz_mm = z_max / (n_z.max(16) as f64);

    for iz in 0..n_z.max(16) {
        let z = (iz as f64 + 0.5) * dz_mm;
        let (r_in_mm, wall_eff_mm, kt_local) = if z <= l1 {
            let s = (z / l1).clamp(0.0, 1.0);
            let env = (PI * s).sin().powi(2);
            let mut lobe_amp_sum = 0.0_f64;
            let mut lobe_kt = 1.0_f64;
            for mode in &prog.chamber_modes {
                let amp = env * mode.cos_mm.hypot(mode.sin_mm);
                lobe_amp_sum += amp;
                let m = mode.azimuthal_order as f64;
                lobe_kt += 0.35 * (m * m - 1.0).max(1.0) * (amp / wall_nom_mm);
            }
            let w_eff = (wall_nom_mm - lobe_amp_sum).max(0.8);
            (r1 + lobe_amp_sum, w_eff, lobe_kt)
        } else if z <= zc {
            let t = (z - l1) / (zc - l1);
            let r_lin = r1 + (r2 - r1) * t;
            let s = ((z - 72.0) / (166.0 - 72.0)).clamp(0.0, 1.0);
            let env = if (72.0..=166.0).contains(&z) {
                (PI * s).sin().powi(2)
            } else {
                0.0
            };
            let dr = env
                * (prog.cone_profile.mode1_mm * (PI * s).sin()
                    + prog.cone_profile.mode2_mm * (2.0 * PI * s).sin());
            let w_eff = (wall_nom_mm - dr.max(0.0)).max(0.8);
            let kt_cone = 1.0
                + 0.22
                    * (prog.cone_profile.mode1_mm.abs() + 2.0 * prog.cone_profile.mode2_mm.abs())
                    / wall_nom_mm;
            (r_lin + dr, w_eff, kt_cone)
        } else if z <= zt {
            let t = (z - zc) / (zt - zc);
            let r_lin = r2 + (r3 - r2) * t;
            (r_lin, wall_nom_mm, 1.02)
        } else {
            (r3, wall_nom_mm, 1.0)
        };

        let a_m = (r_in_mm * 1e-3).max(1e-4);
        let b_m = a_m + (wall_eff_mm * 1e-3).max(5e-4);
        let state = lame_thick_cylinder_oracle(a_m, b_m, 0.0, delta_p_vessel_pa, a_m)?;
        let vm_local = state.von_mises_pa * kt_local;
        max_vm_pa = max_vm_pa.max(vm_local);
        integrated_vm_pa_m += vm_local * (dz_mm * 1e-3);
    }

    Ok((max_vm_pa * 1e-6, integrated_vm_pa_m * 1e-6))
}

/// Evaluate conditional analytic stress/modal/LPBF bounds. This is not a solid
/// solve; absent reaction and converged-mesh evidence remain explicitly unknown.
pub fn analytic_structural_screen(
    prog: &DesignProgram,
    pheno: &Phenotype3D,
    dp_u_pa: f64,
    q_in_m3_s: f64,
) -> Result<AnalyticStructuralScreen, String> {
    let genome_sha256 = prog.canonical_sha256()?;
    if pheno.design_sha256 != genome_sha256 {
        return Err("structural phenotype/genome identity mismatch".into());
    }
    let load_case_sha256 = structural_load_case_hash(&genome_sha256, dp_u_pa, q_in_m3_s)?;
    if !dp_u_pa.is_finite()
        || !q_in_m3_s.is_finite()
        || dp_u_pa <= 0.0
        || q_in_m3_s <= 0.0
        || !DESIGN_VESSEL_DELTA_P_PA.is_finite()
    {
        return Err("invalid non-finite or non-positive hydraulic load inputs".to_string());
    }

    let base = baseline();
    let dims = base.derive()?;

    // 1. STRUCTURE-01 & STRUCTURE-03: 3-grid axial stress convergence (Nz = 256, 512, 1024)
    let (vm_256, int_256) =
        evaluate_axial_wall_stress_profile_mpa(prog, DESIGN_VESSEL_DELTA_P_PA, 256)?;
    let (vm_512, int_512) =
        evaluate_axial_wall_stress_profile_mpa(prog, DESIGN_VESSEL_DELTA_P_PA, 512)?;
    let (vm_1024, int_1024) =
        evaluate_axial_wall_stress_profile_mpa(prog, DESIGN_VESSEL_DELTA_P_PA, 1024)?;

    let (_, int_128) = evaluate_axial_wall_stress_profile_mpa(prog, DESIGN_VESSEL_DELTA_P_PA, 128)?;
    let stress_convergence = four_grid_gci([int_128, int_256, int_512, int_1024], 2.0, 0.01)?;
    let stress_gci_fine_rel = stress_convergence.gci_fine_rel;
    let max_wall_von_mises_mpa = vm_1024.max(vm_512).max(vm_256);
    let min_yield_safety_factor = TI64_YIELD_STRENGTH_MPA / max_wall_von_mises_mpa.max(1e-6);

    // 2. STRUCTURE-02: Load transfer conservation (axial pressure thrust + swirl angular momentum torque)
    let a_head_m2 = PI * (0.5_f64 * dims.d1_mm * 1e-3).powi(2);
    let a_tail_m2 = PI * (0.5_f64 * dims.d3_mm * 1e-3).powi(2);
    let fluid_axial_load_n = dp_u_pa * (a_head_m2 - a_tail_m2);

    let a_inlet_total_m2 =
        2.0 * dims.inlet_area_each_mm2 * 1e-6 * prog.inlet_ramp.radial_slot_factor;
    let u_inlet_m_s = q_in_m3_s / a_inlet_total_m2.max(1e-7);
    let r_arm_m = (0.5 * dims.d1_mm - 0.5 * dims.inlet_width_mm) * 1e-3;
    let swirl_reaction_torque_n_m = WATER_DENSITY_KG_M3 * q_in_m3_s * u_inlet_m_s * r_arm_m;
    // CFD traction fields and independent solid reactions are absent in this
    // analytical screen. Estimated thrust/torque cannot verify load mapping.
    let load_transfer_audit = None;

    // 3. STRUCTURE-04a: Inlet splitter-vane hydrodynamic jet bending & Strouhal VIV resonance
    let q_dyn_inlet_pa = 0.5 * WATER_DENSITY_KG_M3 * u_inlet_m_s * u_inlet_m_s;
    let (vane_bending_stress_mpa, vane_natural_freq_hz, vane_strouhal_shedding_freq_hz) =
        if prog.inlet_split.enabled && prog.inlet_split.branches_per_port >= 2 {
            let t_v_m = (prog.inlet_split.vane_thickness_mm * 1e-3).max(2e-4);
            let h_v_m = dims.inlet_height_mm * 1e-3; // 12.23 mm span
            let c_v_m = 0.018_f64; // 18 mm chord
                                   // Clamped-clamped span under transverse jet incidence differential (C_L ~ 0.38)
            let p_transverse_pa = 0.38 * q_dyn_inlet_pa;
            let sigma_bend_pa = 0.5 * p_transverse_pa * (h_v_m / t_v_m).powi(2);
            // Clamped-clamped fundamental frequency in water (with added mass)
            let i_area = c_v_m * t_v_m.powi(3) / 12.0;
            let m_lin = c_v_m * t_v_m * TI64_DENSITY_KG_M3
                + 0.25 * PI * WATER_DENSITY_KG_M3 * c_v_m * t_v_m;
            let beta1_l = 4.73004074_f64;
            let omega1 =
                (beta1_l / h_v_m).powi(2) * (TI64_YOUNGS_MODULUS_PA * i_area / m_lin).sqrt();
            let f1_hz = omega1 / (2.0 * PI);
            let f_shed_hz = 0.21 * u_inlet_m_s / t_v_m;
            (sigma_bend_pa * 1e-6, f1_hz, f_shed_hz)
        } else {
            (0.0, 50_000.0, 100.0)
        };
    let vane_modal_freq_ratio = vane_natural_freq_hz / vane_strouhal_shedding_freq_hz.max(1e-3);

    // 4. STRUCTURE-04b: Vortex-finder skimmer cantilever deflection & PVC resonance
    let pvc_excitation_freq_hz = 0.48 * u_inlet_m_s / (PI * dims.d1_mm * 1e-3);
    let (skimmer_tip_deflection_mm, skimmer_root_stress_mpa, skimmer_natural_freq_hz) =
        if prog.skimmer.length_mm > 0.1 {
            let l_s_m = prog.skimmer.length_mm * 1e-3;
            let r_i_m = (0.5 * dims.overflow_diameter_mm + 0.5) * 1e-3;
            let r_o_m = (prog.skimmer.outer_radius_mm * 1e-3).max(r_i_m + 5e-4);
            let i_ann = 0.25 * PI * (r_o_m.powi(4) - r_i_m.powi(4));
            let a_ann = PI * (r_o_m.powi(2) - r_i_m.powi(2));
            // Lateral PVC pressure load per unit length q_lat (N/m)
            let p_pvc_pa = (0.045 * dp_u_pa).max(800.0);
            let q_lat_n_m = p_pvc_pa * (2.0 * r_o_m);
            let delta_tip_m =
                (q_lat_n_m * l_s_m.powi(4)) / (8.0 * TI64_YOUNGS_MODULUS_PA * i_ann.max(1e-16));
            let m_root_n_m = 0.5 * q_lat_n_m * l_s_m.powi(2);
            let sigma_root_pa = m_root_n_m * r_o_m / i_ann.max(1e-16);
            // Cantilever first mode (beta1*L = 1.875104) with submerged water added mass
            let m_lin = a_ann * TI64_DENSITY_KG_M3 + PI * r_o_m.powi(2) * WATER_DENSITY_KG_M3;
            let beta1_l = 1.87510407_f64;
            let omega1 =
                (beta1_l / l_s_m).powi(2) * (TI64_YOUNGS_MODULUS_PA * i_ann / m_lin).sqrt();
            (delta_tip_m * 1e3, sigma_root_pa * 1e-6, omega1 / (2.0 * PI))
        } else {
            (0.0, 0.0, 50_000.0)
        };
    let skimmer_modal_freq_ratio = skimmer_natural_freq_hz / pvc_excitation_freq_hz.max(1e-3);

    // 5. STRUCTURE-05 / CAD-08 / CAD-09: LPBF overhang & powder-evacuation manufacturability
    let ramp_overhang_deg = prog.inlet_ramp.axial_drop_mm.abs().atan2(32.0).to_degrees();
    let cone_slope_deg = 10.0
        + ((prog.cone_profile.mode1_mm.abs() + 2.0 * prog.cone_profile.mode2_mm.abs()) / 30.0)
            .atan()
            .to_degrees();
    let max_lpbf_overhang_from_vertical_deg = ramp_overhang_deg.max(cone_slope_deg);
    let min_powder_evacuation_dh_mm = pheno.minimum_passage_diameter_mm;

    let mut failure_reasons = Vec::new();
    if min_yield_safety_factor < MIN_YIELD_SAFETY_FACTOR {
        failure_reasons.push(format!(
            "yield_safety_factor_below_min({min_yield_safety_factor:.3} < {MIN_YIELD_SAFETY_FACTOR:.2})"
        ));
    }
    if let Some(gci) = stress_gci_fine_rel {
        if gci > 0.01 {
            failure_reasons.push(format!("stress_convergence_gci_exceeded({gci:.5} > 0.01)"));
        }
    }
    if vane_bending_stress_mpa > TI64_HCF_ENDURANCE_SEAWATER_MPA {
        failure_reasons.push(format!(
            "vane_bending_stress_exceeds_endurance({vane_bending_stress_mpa:.2} > {TI64_HCF_ENDURANCE_SEAWATER_MPA:.1} MPa)"
        ));
    }
    if vane_modal_freq_ratio < MIN_MODAL_FREQUENCY_RATIO {
        failure_reasons.push(format!(
            "vane_viv_lockin_risk({vane_modal_freq_ratio:.2} < {MIN_MODAL_FREQUENCY_RATIO:.1})"
        ));
    }
    if skimmer_tip_deflection_mm > MAX_SKIMMER_TIP_DEFLECTION_MM {
        failure_reasons.push(format!(
            "skimmer_tip_deflection_exceeded({skimmer_tip_deflection_mm:.5} > {MAX_SKIMMER_TIP_DEFLECTION_MM:.3} mm)"
        ));
    }
    if skimmer_root_stress_mpa > TI64_HCF_ENDURANCE_SEAWATER_MPA {
        failure_reasons.push(format!(
            "skimmer_root_stress_exceeds_endurance({skimmer_root_stress_mpa:.2} > {TI64_HCF_ENDURANCE_SEAWATER_MPA:.1} MPa)"
        ));
    }
    if skimmer_modal_freq_ratio < MIN_MODAL_FREQUENCY_RATIO {
        failure_reasons.push(format!(
            "skimmer_pvc_resonance_risk({skimmer_modal_freq_ratio:.2} < {MIN_MODAL_FREQUENCY_RATIO:.1})"
        ));
    }
    if max_lpbf_overhang_from_vertical_deg > MAX_LPBF_OVERHANG_FROM_VERTICAL_DEG {
        failure_reasons.push(format!(
            "lpbf_overhang_exceeded({max_lpbf_overhang_from_vertical_deg:.2} > {MAX_LPBF_OVERHANG_FROM_VERTICAL_DEG:.1} deg)"
        ));
    }
    if min_powder_evacuation_dh_mm < 1.4 {
        failure_reasons.push(format!(
            "powder_evacuation_bore_below_min({min_powder_evacuation_dh_mm:.3} < 1.400 mm)"
        ));
    }

    Ok(AnalyticStructuralScreen {
        genome_sha256,
        load_case_sha256,
        passed: failure_reasons.is_empty(),
        estimate_kind: ANALYTIC_STRUCTURAL_ESTIMATE_KIND.to_string(),
        buckling_status: buckling_not_evaluated(),
        hydraulic_dp_u_pa_input: Some(dp_u_pa),
        material_designation: "LPBF_Ti-6Al-4V_Grade5_Provisional_StressRelieved".to_string(),
        sour_service_status: "unknown_material_process_and_service_eligibility".into(),
        screen_status:
            "analytical_and_axial_slice_structural_modal_screen_not_3d_solid_fea_or_nace_mr0175"
                .to_string(),
        design_vessel_delta_p_pa: DESIGN_VESSEL_DELTA_P_PA,
        max_wall_von_mises_mpa,
        min_yield_safety_factor,
        stress_gci_fine_rel,
        stress_convergence,
        estimated_axial_pressure_thrust_n: fluid_axial_load_n,
        estimated_swirl_torque_n_m: swirl_reaction_torque_n_m,
        load_transfer_audit,
        load_transfer_status: "unknown_no_cfd_tractions_or_solid_reactions".into(),
        vane_bending_stress_mpa,
        vane_natural_freq_hz,
        vane_strouhal_shedding_freq_hz,
        vane_modal_freq_ratio,
        skimmer_tip_deflection_mm,
        skimmer_root_stress_mpa,
        skimmer_natural_freq_hz,
        pvc_excitation_freq_hz,
        skimmer_modal_freq_ratio,
        max_lpbf_overhang_from_vertical_deg,
        min_powder_evacuation_dh_mm,
        failure_reasons,
    })
}
