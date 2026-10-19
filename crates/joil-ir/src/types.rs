use crate::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub type P3 = [f64; 3];

/// CT-35 reference envelope (`baselines/ct35/reference.json` `derived_mm`) in SI metres. The IR
/// record and the mesh compiler share these values (`V5-01-05`). IR schema 1 recorded D3 = 20 mm,
/// L2 = 1070 mm and L3 = 400 mm while the mesh used the reference; schema-1 hashes keep that
/// historical identity and schema-1 IRs are not recompiled.
pub const CT35_D1_M: f64 = 0.070;
pub const CT35_D2_M: f64 = 0.035;
pub const CT35_D3_M: f64 = 0.0175;
pub const CT35_L1_M: f64 = 0.070;
pub const CT35_L2_M: f64 = 0.770;
pub const CT35_L3_M: f64 = 0.700;
/// Current IR identity (`V5-01-05`): schema 2 records the reference envelope.
pub const IR_SCHEMA_VERSION: u32 = 2;
pub const IR_FAMILY: &str = "joil-design-ir-v2";

/// Canonical SHA-256 digest of a byte slice.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) fn round6(x: f64) -> f64 {
    (x * 1_000_000.0).round() / 1_000_000.0
}

pub(crate) fn round9(x: f64) -> f64 {
    (x * 1_000_000_000.0).round() / 1_000_000_000.0
}

// ============================================================================
// IR-01 & IR-02: Errors, Backend Support Matrix, and DesignIrV1 Nodes
// ============================================================================

/// Target geometry/evaluation backend identifier (`IR-02`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendId {
    /// Fast axisymmetric/modulated preview mesh (`SolidMesh`).
    RustFastRadialSurface,
    /// Full 3D multi-patch CFD/F0 surface with internal vanes/skimmer/ramps (`MultiPatchSurface`).
    RustMultiPatchCfdSurface,
    /// Python CadQuery B-rep compiler with NURBS lofting for smooth fields (`exotic_graph.py`).
    PythonCadQueryBrep,
    /// Strict analytical CSG-only B-rep compiler (rejects implicit TPMS/ribbon fields).
    PythonCadQueryStrictCsgOnly,
    /// Independent 3D voxel/SDF volumetric flood-fill and clearance observer.
    VoxelSdfObserver,
    /// Legacy pre-v4 unsplit-inlet CFD surface generator (does not enact splitter vanes/skimmer).
    LegacyUnenactedCfdSurface,
    /// CUDA reduced-order trajectory & SDF tensor evaluator.
    CudaReducedScreen,
}

impl BackendId {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackendId::RustFastRadialSurface => "rust_fast_radial_surface",
            BackendId::RustMultiPatchCfdSurface => "rust_multi_patch_cfd_surface",
            BackendId::PythonCadQueryBrep => "python_cadquery_brep",
            BackendId::PythonCadQueryStrictCsgOnly => "python_cadquery_strict_csg_only",
            BackendId::VoxelSdfObserver => "voxel_sdf_observer",
            BackendId::LegacyUnenactedCfdSurface => "legacy_unenacted_cfd_surface",
            BackendId::CudaReducedScreen => "cuda_reduced_screen",
        }
    }
}

/// Feature coverage level for a given `(feature, backend)` pair (`IR-02`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SupportLevel {
    /// Exact analytical or B-rep/mesh realization within declared numerical tolerance.
    Exact,
    /// Bounded conservative approximation with explicit approximation note.
    ConservativeApproximation { bound_note: String },
    /// Radial outer-wall corrugation proxy (`r_wall(theta, z)` modulation); does NOT bound volume,
    /// wetted area, or topology of free-standing internal vanes or a 3D TPMS lattice (`IR-R4`).
    WallCorrugationProxy { proxy_note: String },
    /// Visualization or low-order screening proxy only; cannot qualify physical geometry.
    ScreenOnly { reason: String },
    /// Feature is not supported by this backend; active use must fail closed.
    Unsupported { reason: String },
}

impl SupportLevel {
    pub fn is_supported_for_geometry_qualification(&self) -> bool {
        matches!(
            self,
            SupportLevel::Exact | SupportLevel::ConservativeApproximation { .. }
        )
    }
}

/// Typed compiler, backend-support, enactment, and F0 errors (`IR-01`..`IR-12`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error_kind", rename_all = "snake_case")]
pub enum IrError {
    SchemaOrFamilyUnsupported {
        schema_version: u32,
        family: String,
        reason: String,
    },
    BaselineHashMismatch {
        expected: String,
        actual: String,
    },
    InvalidParameter {
        gene_path: String,
        reason: String,
    },
    InvalidBranchCount {
        branches_per_port: u32,
        reason: String,
    },
    UnsupportedFeature {
        backend: BackendId,
        feature: String,
        reason: String,
    },
    UnenactedFeature {
        feature: String,
        ir_node_id: String,
        reason: String,
    },
    MissingSemanticOriginTrace {
        triangle_index: usize,
        reason: String,
    },
    F0AuditFailed {
        reasons: Vec<F0RejectionReason>,
    },
}

impl std::fmt::Display for IrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for IrError {}

/// TPMS basis in `DesignIrV1` (`IR-01`).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum TpmsBasisIr {
    #[default]
    None,
    Gyroid,
    SchwarzP,
    Diamond,
    CylindricalGyroid,
    UnsupportedCustom,
}

/// Typed `DesignIrV1` node (`IR-01`). Every active feature in the genome lowers to >=1 `IrNode`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "node_type", rename_all = "snake_case")]
pub enum IrNode {
    /// Immutable outer DN35 cartridge fluid envelope in SI metres.
    BodyEnvelope {
        node_id: String,
        d1_m: f64,
        d2_m: f64,
        d3_m: f64,
        l1_m: f64,
        lc_m: f64,
        l2_m: f64,
        l3_m: f64,
        do_m: f64,
        orifice_length_m: f64,
        plenum_radius_m: f64,
        plenum_length_m: f64,
        wall_thickness_m: f64,
        z_collar_start_m: f64,
        z_head_end_m: f64,
        z_underflow_m: f64,
    },
    /// Immutable terminal patch or frozen interface zone.
    TerminalPatch {
        node_id: String,
        patch_name: String,
        expected_area_m2: f64,
        expected_normal: [f64; 3],
        plane_axis: usize,
        plane_coord_m: f64,
        immutable: bool,
    },
    /// Layer A: Tangential inlet ramp (axial drop + radial slot contraction + superellipse section).
    InletRamp {
        node_id: String,
        axial_drop_m: f64,
        radial_slot_factor: f64,
        superellipse_exponent: f64,
    },
    /// Layer B: Multi-branch tangential manifold header ($2 \to 2K$ jets).
    InletBranchManifold {
        node_id: String,
        branches_per_port: u32,
        total_jets: u32,
        vane_thickness_m: f64,
        vane_z_fraction: f64,
        azimuthal_phase_rad: f64,
    },
    /// Individual branched inlet jet sub-channel (`p in {1,2}, b in 0..K`).
    InletJetChannel {
        node_id: String,
        port_index: u32,
        branch_index: u32,
        z_center_m: f64,
        sub_height_m: f64,
        width_m: f64,
        hydraulic_diameter_m: f64,
    },
    /// Physical solid flow-divider splitter vane inside an inlet duct.
    SplitterVaneSolid {
        node_id: String,
        port_index: u32,
        vane_index: u32,
        x_start_m: f64,
        x_end_m: f64,
        z_center_m: f64,
        thickness_m: f64,
    },
    /// Layer B2: Internal helical guide-ribbon cascade on the swirl chamber wall.
    HelicalRibbonCascade {
        node_id: String,
        vane_count: u32,
        axial_start_m: f64,
        axial_end_m: f64,
        helical_twist_rad: f64,
        thickness_m: f64,
        radial_chord_m: f64,
    },
    /// Layer C2: Spatially graded peripheral TPMS coalescer/rectifier shell.
    TpmsPeripheralShell {
        node_id: String,
        basis: TpmsBasisIr,
        radial_inner_fraction: f64,
        axial_start_m: f64,
        axial_end_m: f64,
        radial_wavelength_m: f64,
        axial_wavelength_m: f64,
        sheet_thickness_m: f64,
    },
    /// Layer C: Single non-axisymmetric azimuthal/helical Fourier mode on the swirl chamber wall.
    ChamberHarmonicMode {
        node_id: String,
        mode_index: usize,
        azimuthal_order: u32,
        cos_m: f64,
        sin_m: f64,
        amplitude_m: f64,
        twist_rad: f64,
        axial_start_m: f64,
        axial_end_m: f64,
    },
    /// Layer D: $C^2$-clamped S-curve reducing cone contour on $z \in [0.072, 0.166]\text{ m}$.
    ConeSplineProfile {
        node_id: String,
        mode1_m: f64,
        mode2_m: f64,
        mode3_m: f64,
        axial_start_m: f64,
        axial_end_m: f64,
    },
    /// Layer E: Axial vortex-finder skimmer sleeve protruding into the swirl chamber around the overflow bore.
    VortexFinderSkimmer {
        node_id: String,
        length_m: f64,
        inner_radius_m: f64,
        outer_radius_m: f64,
        tip_radius_m: f64,
        capture_flutes: u32,
    },
}

impl IrNode {
    pub fn node_id(&self) -> &str {
        match self {
            IrNode::BodyEnvelope { node_id, .. }
            | IrNode::TerminalPatch { node_id, .. }
            | IrNode::InletRamp { node_id, .. }
            | IrNode::InletBranchManifold { node_id, .. }
            | IrNode::InletJetChannel { node_id, .. }
            | IrNode::SplitterVaneSolid { node_id, .. }
            | IrNode::HelicalRibbonCascade { node_id, .. }
            | IrNode::TpmsPeripheralShell { node_id, .. }
            | IrNode::ChamberHarmonicMode { node_id, .. }
            | IrNode::ConeSplineProfile { node_id, .. }
            | IrNode::VortexFinderSkimmer { node_id, .. } => node_id,
        }
    }
}

/// Explicit mapping from every genome gene group to its lowered `DesignIrV1` nodes or `Inactive` state (`IR-01`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum GeneIrMapping {
    Active {
        node_ids: Vec<String>,
        semantic_boundary_ids: Vec<String>,
    },
    Inactive {
        reason: String,
        canonical_default_verified: bool,
    },
}

impl GeneIrMapping {
    pub fn is_active(&self) -> bool {
        matches!(self, GeneIrMapping::Active { .. })
    }
}

/// Semantic origin trace for a boundary face or internal mechanism surface (`IR-11`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticOriginTrace {
    pub origin_id: u16,
    pub semantic_id: String,
    pub gene_path: String,
    pub ir_node_id: String,
    pub compiler_operation: String,
    pub cfd_patch: String,
    pub immutable_interface: bool,
}

/// Canonical `DesignIrV1` representation (`IR-01`, `IR-11`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DesignIrV1 {
    pub ir_schema_version: u32,
    pub ir_family: String,
    pub source_genome_schema_version: u32,
    pub source_genome_family: String,
    pub units: String,
    pub baseline_file_sha256: String,
    pub program_sha256: String,
    pub ir_sha256: String,
    pub nodes: Vec<IrNode>,
    pub gene_mapping: BTreeMap<String, GeneIrMapping>,
    pub semantic_origins: Vec<SemanticOriginTrace>,
}

/// Per-feature coverage entry for a specific `BackendId` (`IR-02`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureBackendCoverage {
    pub gene_name: String,
    pub is_active: bool,
    pub node_ids: Vec<String>,
    pub backend: BackendId,
    pub support: SupportLevel,
}

/// Complete backend support report for a `DesignIrV1` (`IR-02`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendCoverageReport {
    pub ir_sha256: String,
    pub backend: BackendId,
    pub qualified_for_geometry: bool,
    pub feature_coverages: Vec<FeatureBackendCoverage>,
}
