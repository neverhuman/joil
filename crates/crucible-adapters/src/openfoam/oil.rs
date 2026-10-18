//! Source-bound field adapters for synthetic/native oil transport post-processing.
//! The routines consume already extracted OpenFOAM values; they do not certify CFD.
use crucible_particles::{
    forces::{acceleration, DropState, FluidState},
    inject::{integrate_inlet, InletFace},
    inventory::{grade_qualified, plan_sampling, Closure, Inventory, SamplingPlan},
};

#[derive(Clone, Debug)]
pub struct OilTransportEvidence {
    pub pressure_convention: &'static str,
    pub drag_model: &'static str,
    pub drag_regime_max_re: f64,
    pub separate_centrifugal_force: bool,
}

pub fn force_adapter(
    drop: DropState,
    fluid: FluidState,
) -> Result<([f64; 3], OilTransportEvidence), String> {
    let a = acceleration(drop, fluid)?;
    let rel = fluid
        .velocity
        .iter()
        .zip(drop.velocity)
        .map(|(u, v)| (u - v).powi(2))
        .sum::<f64>()
        .sqrt();
    Ok((a, OilTransportEvidence { pressure_convention: "full static -grad(p) including hydrostatics, plus virtual mass Cvm=0.5 and rho_drop*g body force; no additional buoyancy subtraction",
        drag_model: "Schiller-Naumann", drag_regime_max_re: fluid.rho*rel*drop.diameter/fluid.mu, separate_centrifugal_force: false }))
}

pub fn face_injection(
    faces: &[InletFace],
    dt_s: f64,
) -> Result<Vec<crucible_particles::inject::InjectionStratum>, String> {
    integrate_inlet(faces, dt_s)
}

pub fn restart_inventory(inventory: Inventory) -> Result<Inventory, String> {
    inventory.validate_restart()?;
    Ok(inventory)
}

pub fn grade_decision(completed: bool, closure: &Closure, threshold_mg_l: f64) -> bool {
    grade_qualified(completed, closure, threshold_mg_l)
}

pub fn residence_sampling(
    margin_mg_l: f64,
    concentration_mg_l: f64,
    tail_s: f64,
    tail_quantile_complete: bool,
    base_parcels: usize,
) -> Result<SamplingPlan, String> {
    plan_sampling(
        margin_mg_l,
        concentration_mg_l,
        tail_s,
        tail_quantile_complete,
        base_parcels,
    )
}
