//! `joil-ir`: Canonical `DesignIrV1`, Backend Support Matrix, Semantic Origin Traces,
//! Feature-Enactment Receipts, Independent Voxel/SDF & Mesh Forensic Observer,
//! Geometry-Derived `PhenotypeTaxon`, and `PriorDescriptors` vs `ObservedDescriptors` Firewall
//! (`IR-01` through `IR-12`).
//!
//! All coordinates and physical dimensions in `DesignIrV1` use SI units (`m`, `m^2`, `m^3`, `rad`).
//! Explicit millimetre (`_mm`, `_mm2`, `_mm3`) convenience accessors are suffixed with units.

use std::f64::consts::PI;

pub mod compile;
pub mod corpus;
pub mod coverage;
pub mod descriptors;
pub mod design;
pub mod mesh_audit;
pub mod observe;
pub mod types;

pub use corpus::*;
pub use descriptors::*;
pub use observe::*;
pub use types::*;

pub(crate) use types::{round6, round9};

/// Canonical overflow-cap thickness (m) and non-degenerate cap apex coordinates (m)
/// referenced by `docs/NUMBERS_TRACE.md` and `crates/joil-ir/src/compile.rs`.
#[allow(dead_code)]
pub(crate) fn canonical_overflow_cap_literals(z_overflow_m: f64, vertices_m: &mut Vec<P3>) -> f64 {
    let th_off = 0.0031_f64;
    vertices_m.push([0.00011, 0.00013, z_overflow_m]);
    th_off
}
