//! V5-25 optional frozen-liner response operator (software fixtures only).
//!
//! The operator stands in for the full-liner reference only inside its declared envelope, only
//! when the cut-plane interface closes, and only when the state's uncertainty is below an admitted
//! limit. Everything else dispatches the full-liner reference. Abstention is never a pass, and an
//! absent operator never blocks a smaller campaign. Every fixture is SYNTHETIC: the full-liner
//! reference is not run here, no downstream oil grade is qualified, and nothing in this module is
//! a physical validation.
use crate::{invalid, Field, JudgeError, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Two-sided 95 % normal quantile for the Wilson interval.
pub(super) const Z95: f64 = 1.959_963_984_540_054;
/// Initial caps for optional joil jobs (AGENTS.md). A new user-authorized allocation supersedes them.
pub const INITIAL_MAX_CORES: u32 = 4;
pub const INITIAL_MAX_MEMORY_GIB: f64 = 12.0;
pub const INITIAL_MAX_OUTPUT_GIB: f64 = 5.0;
/// Reason recorded when no operator is present. It routes to the full-liner reference and never
/// blocks the campaign.
pub const OPERATOR_ABSENT: &str = "operator:absent";

pub(super) fn open(reason: impl Into<String>) -> Verdict {
    Verdict::Unresolved(vec![reason.into()])
}

pub(super) fn finite(values: &[f64]) -> bool {
    values.iter().all(|v| v.is_finite())
}

/// Invalid values or limits cannot pass a relative-error/uncertainty check.
pub(super) fn exceeds(value: f64, limit: f64) -> bool {
    !value.is_finite() || !limit.is_finite() || value < 0.0 || limit < 0.0 || value > limit
}

/// Relative difference with a symmetric scale. Equal values, including two zeros, agree exactly.
pub(crate) fn rel_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).abs();
    if d == 0.0 {
        0.0
    } else {
        d / a.abs().max(b.abs())
    }
}

// ------------------------------------------------------------------ V5-25-01 interface state

/// One radial node of an admitted cut plane. Velocities are m/s, turbulent kinetic energy is
/// m^2/s^2, and each oil entry is the oil mass flux (kg/m^2/s) crossing the plane in one size bin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CutNode {
    pub radius_m: f64,
    pub axial_ms: f64,
    pub radial_ms: f64,
    pub tangential_ms: f64,
    pub turbulence_k_m2_s2: f64,
    /// Must be set wherever the axial velocity is negative; an unflagged backflow is refused.
    pub reverse_core: bool,
    /// Aligned with `LinerCutPlaneState::oil_bins_m`.
    pub oil_flux_kg_m2_s: Vec<f64>,
}

/// Return flow the downstream boundary feeds back to the tail.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryFeedback {
    /// Reverse-core mass flow the boundary returns, kg/s, non-negative.
    pub reverse_core_inflow_kg_s: f64,
}

/// Complete interface state at one admitted cut plane.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinerCutPlaneState {
    pub station_m: f64,
    pub carrier_density_kg_m3: f64,
    /// Oil size-bin diameters in metres, strictly increasing and positive.
    pub oil_bins_m: Vec<f64>,
    /// Radially ordered nodes, at least three.
    pub nodes: Vec<CutNode>,
    pub boundary: BoundaryFeedback,
    /// One-sigma relative uncertainty on the integrated fluxes of this state.
    pub relative_uncertainty: f64,
}

/// Structural validity of a cut-plane state. An empty list means the state can be integrated.
pub fn validate_state(state: &LinerCutPlaneState) -> Vec<String> {
    let mut bad = Vec::new();
    let scalars = [
        state.station_m,
        state.carrier_density_kg_m3,
        state.boundary.reverse_core_inflow_kg_s,
        state.relative_uncertainty,
    ];
    if !finite(&scalars)
        || state.carrier_density_kg_m3 <= 0.0
        || state.boundary.reverse_core_inflow_kg_s < 0.0
        || state.relative_uncertainty < 0.0
    {
        bad.push("state:invalid_scalar".to_string());
    }
    let bins = &state.oil_bins_m;
    if bins.is_empty()
        || !bins.iter().all(|d| d.is_finite() && *d > 0.0)
        || !bins.windows(2).all(|w| w[0] < w[1])
    {
        bad.push("state:oil_bins_not_increasing_positive".to_string());
    }
    if state.nodes.len() < 3 {
        bad.push("state:needs_three_radial_nodes".to_string());
        return bad;
    }
    if !state
        .nodes
        .iter()
        .all(|n| n.radius_m.is_finite() && n.radius_m > 0.0)
        || !state
            .nodes
            .windows(2)
            .all(|w| w[0].radius_m < w[1].radius_m)
    {
        bad.push("state:radii_not_strictly_increasing_positive".to_string());
    }
    let nodes_ok = state.nodes.iter().all(|n| {
        finite(&[
            n.axial_ms,
            n.radial_ms,
            n.tangential_ms,
            n.turbulence_k_m2_s2,
        ]) && n.turbulence_k_m2_s2 >= 0.0
            && n.oil_flux_kg_m2_s.len() == bins.len()
            && finite(&n.oil_flux_kg_m2_s)
    });
    if !nodes_ok {
        bad.push("state:node_fields_invalid_or_bin_count".to_string());
    }
    if state
        .nodes
        .iter()
        .any(|n| n.axial_ms < 0.0 && !n.reverse_core)
    {
        bad.push("state:unflagged_reverse_flow".to_string());
    }
    bad
}

/// Trapezoid integral over the admitted nodes of `f(node) * 2 pi r` in radius. Exact for a
/// constant `f` (the area of the annulus); it covers only the radii the state supplies.
fn annulus_integral(nodes: &[CutNode], f: impl Fn(&CutNode) -> f64) -> f64 {
    nodes
        .windows(2)
        .map(|w| {
            let g0 = f(&w[0]) * 2.0 * PI * w[0].radius_m;
            let g1 = f(&w[1]) * 2.0 * PI * w[1].radius_m;
            0.5 * (g0 + g1) * (w[1].radius_m - w[0].radius_m)
        })
        .sum()
}

/// Fluxes a cut-plane state carries through the plane.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Integrated {
    pub axial_mass_kg_s: f64,
    pub axial_momentum_n: f64,
    pub angular_momentum_n_m: f64,
    pub oil_kg_s: Vec<f64>,
    /// Reverse-core mass flow through the plane, kg/s, positive when the core flows backwards.
    pub reverse_core_kg_s: f64,
}

/// Integrate a state. The caller must have checked `validate_state` first.
pub fn integrate(state: &LinerCutPlaneState) -> Integrated {
    let rho = state.carrier_density_kg_m3;
    let reverse = -rho
        * annulus_integral(&state.nodes, |n| {
            if n.reverse_core {
                n.axial_ms.min(0.0)
            } else {
                0.0
            }
        });
    Integrated {
        axial_mass_kg_s: rho * annulus_integral(&state.nodes, |n| n.axial_ms),
        axial_momentum_n: rho * annulus_integral(&state.nodes, |n| n.axial_ms * n.axial_ms),
        angular_momentum_n_m: rho
            * annulus_integral(&state.nodes, |n| n.tangential_ms * n.radius_m * n.axial_ms),
        oil_kg_s: (0..state.oil_bins_m.len())
            .map(|k| {
                annulus_integral(&state.nodes, |n| {
                    n.oil_flux_kg_m2_s.get(k).copied().unwrap_or(f64::NAN)
                })
            })
            .collect(),
        reverse_core_kg_s: reverse,
    }
}

/// Balance targets admitted for one cut plane.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BalanceTargets {
    pub axial_mass_kg_s: f64,
    pub axial_momentum_n: f64,
    pub angular_momentum_n_m: f64,
    /// Oil mass flow per size bin admitted for this plane.
    pub oil_kg_s: Vec<f64>,
}

/// Admitted relative tolerances for each balance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BalanceTolerances {
    pub mass_rel: f64,
    pub momentum_rel: f64,
    pub oil_rel: f64,
    pub reverse_core_rel: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BalanceReport {
    pub integrated: Option<Integrated>,
    pub verdict: Verdict,
}

fn residual_fail(fails: &mut Vec<String>, name: &str, actual: f64, target: f64, tol: f64) {
    let r = rel_diff(actual, target);
    if exceeds(r, tol) {
        fails.push(format!("balance:{name}_residual_{r:.3e}_exceeds_{tol:.3e}"));
    }
}

/// Independently check mass, axial and angular momentum, per-bin oil, and the reverse-core inflow
/// against admitted targets. A residual outside tolerance is `Fail`; missing or malformed inputs
/// are `Unresolved`. A closed balance on a state that is not valid is never `Pass`.
pub fn check_balances(
    state: &LinerCutPlaneState,
    targets: &Field<BalanceTargets>,
    tol: &Field<BalanceTolerances>,
) -> BalanceReport {
    let invalid_state = validate_state(state);
    if !invalid_state.is_empty() {
        return BalanceReport {
            integrated: None,
            verdict: Verdict::Unresolved(invalid_state),
        };
    }
    let got = integrate(state);
    let mut open_items = Vec::new();
    if targets.admitted().is_none() {
        open_items.push("balance_targets:not_admitted".to_string());
    }
    if tol.admitted().is_none() {
        open_items.push("balance_tolerances:not_admitted".to_string());
    }
    let (Some(t), Some(k)) = (targets.admitted(), tol.admitted()) else {
        return BalanceReport {
            integrated: Some(got),
            verdict: Verdict::Unresolved(open_items),
        };
    };
    if t.oil_kg_s.len() != got.oil_kg_s.len() {
        open_items.push("balance:oil_target_bin_count_mismatch".to_string());
        return BalanceReport {
            integrated: Some(got),
            verdict: Verdict::Unresolved(open_items),
        };
    }
    let mut fails = Vec::new();
    residual_fail(
        &mut fails,
        "axial_mass",
        got.axial_mass_kg_s,
        t.axial_mass_kg_s,
        k.mass_rel,
    );
    residual_fail(
        &mut fails,
        "axial_momentum",
        got.axial_momentum_n,
        t.axial_momentum_n,
        k.momentum_rel,
    );
    residual_fail(
        &mut fails,
        "angular_momentum",
        got.angular_momentum_n_m,
        t.angular_momentum_n_m,
        k.momentum_rel,
    );
    for (i, (g, e)) in got.oil_kg_s.iter().zip(&t.oil_kg_s).enumerate() {
        residual_fail(&mut fails, &format!("oil_bin_{i}"), *g, *e, k.oil_rel);
    }
    let fed = state.boundary.reverse_core_inflow_kg_s;
    if got.reverse_core_kg_s > 0.0 && fed == 0.0 {
        fails.push("balance:reverse_core_inflow_omitted".to_string());
    } else {
        residual_fail(
            &mut fails,
            "reverse_core",
            got.reverse_core_kg_s,
            fed,
            k.reverse_core_rel,
        );
    }
    let verdict = if !fails.is_empty() {
        Verdict::Fail(fails)
    } else if !open_items.is_empty() {
        Verdict::Unresolved(open_items)
    } else {
        Verdict::Pass
    };
    BalanceReport {
        integrated: Some(got),
        verdict,
    }
}

/// A candidate tail relation. Only the full downstream state may close the tail. A pressure-drop
/// relation is refused whatever its fit quality.
#[derive(Clone, Debug, PartialEq)]
pub enum TailRelation {
    PressureOnly { pressure_drop_pa: f64 },
    FullDownstream(Box<LinerCutPlaneState>),
}

/// `Pass` means only that the full state is admissible as a tail basis. Balances still have to be
/// checked with `check_balances`.
pub fn accept_tail_relation(relation: &TailRelation) -> Verdict {
    match relation {
        TailRelation::PressureOnly { .. } => {
            open("tail:pressure_only_relation_cannot_replace_full_downstream_physics")
        }
        TailRelation::FullDownstream(state) => {
            let bad = validate_state(state);
            if bad.is_empty() {
                Verdict::Pass
            } else {
                Verdict::Unresolved(bad)
            }
        }
    }
}

// ----------------------------------------------------- V5-25-02 preregistered case protocol

#[path = "liner_operator/prereg.rs"]
mod prereg;
pub use prereg::*;

mod audit;
mod coupling;
pub use audit::*;
pub use coupling::*;

#[cfg(test)]
#[path = "liner_operator/remediation_tests.rs"]
mod v5_25_remediation_tests;
