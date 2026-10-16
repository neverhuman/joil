//! Hard F0 Red-Team Geometry, Interface, and Manufacturing Gate (`redteam.rs`).
//!
//! Modeled after `~/jarambit/crates/redteam`: every candidate and every Monte Carlo perturbation
//! must pass this deterministic gate before receiving any performance score.

use crate::geometry::baseline;
use crate::phenotype::{Phenotype3D, SolidMesh};
use crate::program::DesignProgram;
use serde::{Deserialize, Serialize};

/// Frozen research requirements from `requirements/RS-joil-research-v1.json`.
pub const MIN_WALL_CLEARANCE_MM: f64 = 1.0;
pub const MIN_PASSAGE_DIAMETER_MM: f64 = 1.4;
pub const MIN_SECTION_JACOBIAN: f64 = 0.10;
pub const MAX_COLLAR_TAIL_DIFF_MM3: f64 = 1e-5;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedTeamReject {
    NormalizationFailed(String),
    FrozenCollarOrTailModified,
    FrozenTerminalPortsModified,
    WallClearanceBelowMinimum,
    PassageDiameterBelowMinimum,
    SingularOrInvertedJacobian,
    NonManifoldOrInwardMesh,
    TrappedPowderOrFluidCavity,
    StructuralOrModalFailure(String),
}

/// Evaluate all hard F0 red-team geometric and interface constraints on a 3D candidate.
pub fn check(
    prog: &DesignProgram,
    pheno: &Phenotype3D,
    mesh: &SolidMesh,
) -> Result<(), Vec<RedTeamReject>> {
    let mut rejects = Vec::new();
    if let Err(msg) = prog.normalize() {
        rejects.push(RedTeamReject::NormalizationFailed(msg));
    }

    let base_dims = match baseline().derive() {
        Ok(d) => d,
        Err(msg) => {
            rejects.push(RedTeamReject::NormalizationFailed(msg));
            return Err(rejects);
        }
    };

    if !pheno.frozen_collar_and_tail_diff_mm3.is_finite()
        || pheno.frozen_collar_and_tail_diff_mm3 > MAX_COLLAR_TAIL_DIFF_MM3
    {
        rejects.push(RedTeamReject::FrozenCollarOrTailModified);
    }

    if (pheno.terminal_inlet_area_each_mm2 - base_dims.inlet_area_each_mm2).abs() > 1e-6
        || (pheno.terminal_overflow_diameter_mm - base_dims.overflow_diameter_mm).abs() > 1e-6
        || (pheno.terminal_underflow_z_mm - base_dims.z_underflow_mm).abs() > 1e-6
    {
        rejects.push(RedTeamReject::FrozenTerminalPortsModified);
    }

    if !pheno.minimum_wall_clearance_mm.is_finite()
        || pheno.minimum_wall_clearance_mm < MIN_WALL_CLEARANCE_MM
    {
        rejects.push(RedTeamReject::WallClearanceBelowMinimum);
    }

    if !pheno.minimum_passage_diameter_mm.is_finite()
        || pheno.minimum_passage_diameter_mm < MIN_PASSAGE_DIAMETER_MM
    {
        rejects.push(RedTeamReject::PassageDiameterBelowMinimum);
    }

    if !pheno.min_section_jacobian.is_finite() || pheno.min_section_jacobian < MIN_SECTION_JACOBIAN
    {
        rejects.push(RedTeamReject::SingularOrInvertedJacobian);
    }

    let census = mesh.edge_census();
    let signed_vol = mesh.signed_volume();
    if census.open_boundary_edges != 0
        || census.non_manifold_edges != 0
        || !signed_vol.is_finite()
        || signed_vol <= 0.0
    {
        rejects.push(RedTeamReject::NonManifoldOrInwardMesh);
    }

    if pheno.trapped_cavities != 0 {
        rejects.push(RedTeamReject::TrappedPowderOrFluidCavity);
    }

    match crate::structure::analytic_structural_screen(prog, pheno, 29468.22, 0.001111111) {
        Ok(sq) => {
            if !sq.passed {
                for r in sq.failure_reasons {
                    rejects.push(RedTeamReject::StructuralOrModalFailure(r));
                }
            }
        }
        Err(msg) => {
            rejects.push(RedTeamReject::StructuralOrModalFailure(msg));
        }
    }

    if rejects.is_empty() {
        Ok(())
    } else {
        Err(rejects)
    }
}
