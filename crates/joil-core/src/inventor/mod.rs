//! Inventor-Layer Heuristic Mechanism Priors (`IR-07`).
//!
//! Strictly separates genome-derived heuristic priors (`PriorDescriptors`) from judge-admissible
//! geometry and physics observations (`GeometryObservedDescriptors`, `PhysicsObservedDescriptors`,
//! `ObservedDescriptors`).
//!
//! `PriorDescriptors` may be used ONLY for search proposal generation or explicitly labeled
//! sandbox prior diagnostics. Calling `require_observed_descriptors_for_qualification` enforces
//! a compile-time and runtime firewall preventing `PriorDescriptors` from entering qualified scoring.

use crate::program::DesignProgram;
pub use joil_ir::{
    require_observed_descriptors_for_qualification, GeometryObservedDescriptors,
    ObservedDescriptors, PhysicsObservedDescriptors, PriorDescriptors,
};

/// Compute heuristic genome-level `PriorDescriptors` from a `DesignProgram` (`IR-07`).
/// These are inventor-side priors, never physical or geometric observations.
pub fn compute_prior_descriptors(prog: &DesignProgram) -> Result<PriorDescriptors, String> {
    let d = prog.descriptors()?;
    Ok(PriorDescriptors {
        provenance_kind: "genome_heuristic_prior_not_observed".to_string(),
        swirl_flux_prior: d.swirl_flux_index,
        axial_pitch_sin_prior: d.axial_pitch_sin,
        azimuthal_symmetry_prior: d.azimuthal_symmetry_order,
        shear_mitigation_prior: d.shear_mitigation_factor,
        cone_accel_persistence_prior: d.cone_accel_persistence,
        core_shield_ratio_prior: d.core_shield_ratio,
        wall_rifling_prior: d.wall_rifling_stabilization,
        hydraulic_resistance_prior: d.hydraulic_resistance_factor,
    })
}
