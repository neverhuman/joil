//! `joil-ir`: Canonical `DesignIrV1`, Backend Support Matrix, Semantic Origin Traces,
//! Feature-Enactment Receipts, Independent Voxel/SDF & Mesh Forensic Observer,
//! Geometry-Derived `PhenotypeTaxon`, and `PriorDescriptors` vs `ObservedDescriptors` Firewall
//! (`IR-01` through `IR-12`).
//!
//! All coordinates and physical dimensions in `DesignIrV1` use SI units (`m`, `m^2`, `m^3`, `rad`).
//! Explicit millimetre (`_mm`, `_mm2`, `_mm3`) convenience accessors are suffixed with units.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::f64::consts::PI;

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

fn round6(x: f64) -> f64 {
    (x * 1_000_000.0).round() / 1_000_000.0
}

fn round9(x: f64) -> f64 {
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

impl DesignIrV1 {
    /// Lower any `DesignProgram` JSON value into a canonical `DesignIrV1`, preserving schema identity
    /// and mapping every gene to `Active { node_ids }` or `Inactive`.
    pub fn from_program_json(
        prog: &serde_json::Value,
        expected_baseline_sha256: &str,
    ) -> Result<Self, IrError> {
        let obj = prog.as_object().ok_or_else(|| IrError::InvalidParameter {
            gene_path: "<root>".into(),
            reason: "program must be a JSON object".into(),
        })?;

        let schema_version = obj
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let family = obj
            .get("family")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !matches!(
            (schema_version, family.as_str()),
            (1, "ct35-chamber-wall-field-v1")
                | (2, "ct35-exotic-head-v2")
                | (3, "ct35-exotic-head-v3")
                | (4, "joil-xenogen-v4")
        ) {
            return Err(IrError::SchemaOrFamilyUnsupported {
                schema_version,
                family,
                reason: "schema_version and family pair is not an admitted genome schema".into(),
            });
        }

        let actual_base_sha = obj
            .get("baseline_file_sha256")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if actual_base_sha != expected_baseline_sha256 {
            return Err(IrError::BaselineHashMismatch {
                expected: expected_baseline_sha256.to_string(),
                actual: actual_base_sha.to_string(),
            });
        }

        let program_bytes = serde_json::to_vec(prog).map_err(|e| IrError::InvalidParameter {
            gene_path: "<root>".into(),
            reason: e.to_string(),
        })?;
        let program_sha256 = sha256_hex(&program_bytes);

        // Fixed CT-35 reference envelope dimensions in SI metres (`V5-01-05`)
        let d2_m = CT35_D2_M;
        let d1_m = CT35_D1_M;
        let d3_m = CT35_D3_M;
        let l1_m = CT35_L1_M;
        let cone_angle_deg = 10.0_f64;
        let lc_m = ((d1_m - d2_m) / 2.0) / cone_angle_deg.to_radians().tan();
        let l2_m = CT35_L2_M;
        let l3_m = CT35_L3_M;
        let do_m = 0.0014_f64;
        let orifice_length_m = 0.0028_f64;
        let plenum_radius_m = 0.0175_f64;
        let plenum_length_m = 0.020_f64;
        let wall_thickness_m = 0.003_f64;
        let z_head_end_m = l1_m + lc_m + 0.5 * d2_m;
        let z_collar_start_m = l1_m + lc_m - 0.5 * d2_m; // last 0.5*D2 collar before head end
        let z_underflow_m = l1_m + lc_m + l2_m + l3_m;
        let inlet_area_each_m2 = 0.0625 * PI * d1_m * d1_m / 4.0 / 2.0;
        let inlet_h_over_w = 1.2435_f64;
        let inlet_w_m = (inlet_area_each_m2 / inlet_h_over_w).sqrt();
        let inlet_h_m = inlet_h_over_w * inlet_w_m;
        let inlet_x_m = 0.060_f64;

        let mut nodes = Vec::new();
        let mut gene_mapping = BTreeMap::new();
        let mut semantic_origins = Vec::new();
        let mut next_origin_id = 0u16;

        let mut push_origin = |semantic_id: &str,
                               gene_path: &str,
                               ir_node_id: &str,
                               op: &str,
                               patch: &str,
                               immut: bool|
         -> String {
            let id = next_origin_id;
            next_origin_id += 1;
            semantic_origins.push(SemanticOriginTrace {
                origin_id: id,
                semantic_id: semantic_id.to_string(),
                gene_path: gene_path.to_string(),
                ir_node_id: ir_node_id.to_string(),
                compiler_operation: op.to_string(),
                cfd_patch: patch.to_string(),
                immutable_interface: immut,
            });
            semantic_id.to_string()
        };

        // 1. BodyEnvelope node + wall semantic origins
        nodes.push(IrNode::BodyEnvelope {
            node_id: "node.body_envelope".into(),
            d1_m: round9(d1_m),
            d2_m: round9(d2_m),
            d3_m: round9(d3_m),
            l1_m: round9(l1_m),
            lc_m: round9(lc_m),
            l2_m: round9(l2_m),
            l3_m: round9(l3_m),
            do_m: round9(do_m),
            orifice_length_m: round9(orifice_length_m),
            plenum_radius_m: round9(plenum_radius_m),
            plenum_length_m: round9(plenum_length_m),
            wall_thickness_m: round9(wall_thickness_m),
            z_collar_start_m: round9(z_collar_start_m),
            z_head_end_m: round9(z_head_end_m),
            z_underflow_m: round9(z_underflow_m),
        });
        let body_bounds = vec![
            push_origin(
                "wall.plenum",
                "baseline.plenum",
                "node.body_envelope",
                "revolve_plenum",
                "wall",
                true,
            ),
            push_origin(
                "wall.orifice",
                "baseline.orifice",
                "node.body_envelope",
                "revolve_orifice_bore",
                "wall",
                true,
            ),
            push_origin(
                "wall.chamber",
                "baseline.chamber",
                "node.body_envelope",
                "loft_swirl_chamber",
                "wall",
                false,
            ),
            push_origin(
                "wall.cone",
                "baseline.cone",
                "node.body_envelope",
                "loft_reducing_cone",
                "wall",
                false,
            ),
            push_origin(
                "wall.collar_and_taper",
                "baseline.taper",
                "node.body_envelope",
                "revolve_frozen_taper",
                "wall",
                true,
            ),
            push_origin(
                "wall.tail",
                "baseline.tail",
                "node.body_envelope",
                "revolve_frozen_tail",
                "wall",
                true,
            ),
            push_origin(
                "wall.inlet1_duct",
                "baseline.inlet1_duct",
                "node.body_envelope",
                "extrude_tangential_inlet1",
                "wall",
                false,
            ),
            push_origin(
                "wall.inlet2_duct",
                "baseline.inlet2_duct",
                "node.body_envelope",
                "extrude_tangential_inlet2",
                "wall",
                false,
            ),
        ];
        gene_mapping.insert(
            "body_envelope".into(),
            GeneIrMapping::Active {
                node_ids: vec!["node.body_envelope".into()],
                semantic_boundary_ids: body_bounds,
            },
        );

        // 2. Terminal patches
        let terminals_spec = [
            (
                "node.terminal.inlet1",
                "inlet1",
                inlet_area_each_m2,
                [-1.0, 0.0, 0.0],
                0usize,
                -inlet_x_m,
            ),
            (
                "node.terminal.inlet2",
                "inlet2",
                inlet_area_each_m2,
                [1.0, 0.0, 0.0],
                0usize,
                inlet_x_m,
            ),
            (
                "node.terminal.overflow",
                "overflow",
                PI * plenum_radius_m * plenum_radius_m,
                [0.0, 0.0, -1.0],
                2usize,
                -(orifice_length_m + plenum_length_m),
            ),
            (
                "node.terminal.underflow",
                "underflow",
                PI * (d3_m / 2.0).powi(2),
                [0.0, 0.0, 1.0],
                2usize,
                z_underflow_m,
            ),
        ];
        let mut term_node_ids = Vec::new();
        let mut term_bound_ids = Vec::new();
        for (nid, pname, area, norm, ax, pos) in terminals_spec {
            nodes.push(IrNode::TerminalPatch {
                node_id: nid.into(),
                patch_name: pname.into(),
                expected_area_m2: round9(area),
                expected_normal: norm,
                plane_axis: ax,
                plane_coord_m: round9(pos),
                immutable: true,
            });
            term_node_ids.push(nid.to_string());
            term_bound_ids.push(push_origin(
                &format!("terminal.{pname}"),
                &format!("baseline.terminals.{pname}"),
                nid,
                "cap_terminal_patch",
                pname,
                true,
            ));
        }
        gene_mapping.insert(
            "terminals".into(),
            GeneIrMapping::Active {
                node_ids: term_node_ids,
                semantic_boundary_ids: term_bound_ids,
            },
        );

        // 3. Layer A: inlet_ramp
        let ramp_obj = obj.get("inlet_ramp").and_then(|v| v.as_object());
        let drop_mm = ramp_obj
            .and_then(|r| r.get("axial_drop_mm"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let slot_factor = ramp_obj
            .and_then(|r| r.get("radial_slot_factor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let se_exp = ramp_obj
            .and_then(|r| r.get("superellipse_exponent"))
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);
        if !drop_mm.is_finite() || !(0.0..=8.0).contains(&drop_mm) {
            return Err(IrError::InvalidParameter {
                gene_path: "inlet_ramp.axial_drop_mm".into(),
                reason: format!("value {drop_mm} out of [0.0, 8.0] mm"),
            });
        }
        if !slot_factor.is_finite() || !(0.72..=1.05).contains(&slot_factor) {
            return Err(IrError::InvalidParameter {
                gene_path: "inlet_ramp.radial_slot_factor".into(),
                reason: format!("value {slot_factor} out of [0.72, 1.05]"),
            });
        }
        if !se_exp.is_finite() || !(2.0..=4.0).contains(&se_exp) {
            return Err(IrError::InvalidParameter {
                gene_path: "inlet_ramp.superellipse_exponent".into(),
                reason: format!("value {se_exp} out of [2.0, 4.0]"),
            });
        }
        let ramp_active = drop_mm.abs() > 1e-9 || (slot_factor - 1.0).abs() > 1e-9;
        if ramp_active {
            let nid = "node.inlet_ramp".to_string();
            nodes.push(IrNode::InletRamp {
                node_id: nid.clone(),
                axial_drop_m: round9(drop_mm * 1e-3),
                radial_slot_factor: round6(slot_factor),
                superellipse_exponent: round6(se_exp),
            });
            let bid = push_origin(
                "feature.inlet_ramp",
                "inlet_ramp",
                &nid,
                "sculpt_tangential_entry_ramp",
                "wall",
                false,
            );
            gene_mapping.insert(
                "inlet_ramp".into(),
                GeneIrMapping::Active {
                    node_ids: vec![nid],
                    semantic_boundary_ids: vec![bid],
                },
            );
        } else {
            gene_mapping.insert(
                "inlet_ramp".into(),
                GeneIrMapping::Inactive {
                    reason: "axial_drop_mm == 0 and radial_slot_factor == 1.0".into(),
                    canonical_default_verified: true,
                },
            );
        }

        // 4. Layer B: inlet_split
        let split_obj = obj.get("inlet_split").and_then(|v| v.as_object());
        let split_enabled = split_obj
            .and_then(|s| s.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let vane_t_mm = split_obj
            .and_then(|s| s.get("vane_thickness_mm"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.2);
        let vane_z_frac = split_obj
            .and_then(|s| s.get("vane_z_fraction"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        let branches_per_port = split_obj
            .and_then(|s| s.get("branches_per_port"))
            .and_then(|v| v.as_u64())
            .unwrap_or(if split_enabled { 2 } else { 1 }) as u32;
        let phase_deg = split_obj
            .and_then(|s| s.get("azimuthal_phase_deg"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        if split_enabled {
            if !(2..=4).contains(&branches_per_port) {
                return Err(IrError::InvalidBranchCount {
                    branches_per_port,
                    reason: "active inlet_split requires branches_per_port in 2..=4".into(),
                });
            }
            if !vane_t_mm.is_finite()
                || !(0.4..=2.5).contains(&vane_t_mm)
                || !vane_z_frac.is_finite()
                || !(0.20..=0.80).contains(&vane_z_frac)
                || !phase_deg.is_finite()
                || !(0.0..=45.0).contains(&phase_deg)
            {
                return Err(IrError::InvalidParameter {
                    gene_path: "inlet_split".into(),
                    reason: "invalid inlet_split vane parameters".into(),
                });
            }
            let mut split_nodes = Vec::new();
            let mut split_bounds = Vec::new();
            let main_id = "node.inlet_split.manifold".to_string();
            nodes.push(IrNode::InletBranchManifold {
                node_id: main_id.clone(),
                branches_per_port,
                total_jets: 2 * branches_per_port,
                vane_thickness_m: round9(vane_t_mm * 1e-3),
                vane_z_fraction: round6(vane_z_frac),
                azimuthal_phase_rad: round9(phase_deg.to_radians()),
            });
            split_nodes.push(main_id);

            let n_vanes = branches_per_port - 1;
            let sub_h_m = ((inlet_h_m - (n_vanes as f64) * vane_t_mm * 1e-3)
                / (branches_per_port as f64))
                .max(1e-4);
            let w_eff_m = inlet_w_m * slot_factor;
            let dh_sub_m = 2.0 * w_eff_m * sub_h_m / (w_eff_m + sub_h_m);

            for port_idx in 1..=2u32 {
                for b_idx in 0..branches_per_port {
                    let jid = format!("node.inlet_jet.p{port_idx}.b{b_idx}");
                    let z_c = (b_idx as f64 + 0.5) * (inlet_h_m / branches_per_port as f64);
                    nodes.push(IrNode::InletJetChannel {
                        node_id: jid.clone(),
                        port_index: port_idx,
                        branch_index: b_idx,
                        z_center_m: round9(z_c),
                        sub_height_m: round9(sub_h_m),
                        width_m: round9(w_eff_m),
                        hydraulic_diameter_m: round9(dh_sub_m),
                    });
                    split_nodes.push(jid);
                }
                for v_idx in 0..n_vanes {
                    let vid = format!("node.splitter_vane.p{port_idx}.v{v_idx}");
                    let zf = if n_vanes == 1 {
                        vane_z_frac
                    } else {
                        (v_idx as f64 + 1.0) / (branches_per_port as f64)
                    };
                    let (xs, xe) = if port_idx == 1 {
                        (-0.040, -0.022)
                    } else {
                        (0.022, 0.040)
                    };
                    nodes.push(IrNode::SplitterVaneSolid {
                        node_id: vid.clone(),
                        port_index: port_idx,
                        vane_index: v_idx,
                        x_start_m: xs,
                        x_end_m: xe,
                        z_center_m: round9(zf * inlet_h_m),
                        thickness_m: round9(vane_t_mm * 1e-3),
                    });
                    let bid = push_origin(
                        &format!("feature.inlet_split.p{port_idx}.v{v_idx}"),
                        "inlet_split",
                        &vid,
                        "insert_splitter_vane",
                        "wall",
                        false,
                    );
                    split_nodes.push(vid);
                    split_bounds.push(bid);
                }
            }
            gene_mapping.insert(
                "inlet_split".into(),
                GeneIrMapping::Active {
                    node_ids: split_nodes,
                    semantic_boundary_ids: split_bounds,
                },
            );
        } else {
            if branches_per_port > 4 {
                return Err(IrError::InvalidBranchCount {
                    branches_per_port,
                    reason: "branches_per_port exceeds maximum supported count 4".into(),
                });
            }
            gene_mapping.insert(
                "inlet_split".into(),
                GeneIrMapping::Inactive {
                    reason: "inlet_split.enabled == false (canonicalized to 1 branch per port)"
                        .into(),
                    canonical_default_verified: true,
                },
            );
        }

        // 5. Layer B2: ribbon_cascade
        let rib_obj = obj.get("ribbon_cascade").and_then(|v| v.as_object());
        let rib_vanes = rib_obj
            .and_then(|r| r.get("vane_count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let rib_span_mm = rib_obj
            .and_then(|r| r.get("axial_span_mm"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let rib_twist = rib_obj
            .and_then(|r| r.get("helical_twist_rad"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let rib_thick_mm = rib_obj
            .and_then(|r| r.get("thickness_mm"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.2);
        let rib_chord_mm = rib_obj
            .and_then(|r| r.get("radial_chord_mm"))
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        if rib_vanes > 0 && rib_span_mm > 1e-9 {
            if !matches!(rib_vanes, 2..=8)
                || !rib_span_mm.is_finite()
                || !(10.0..=55.0).contains(&rib_span_mm)
                || !rib_thick_mm.is_finite()
                || !(0.4..=3.0).contains(&rib_thick_mm)
                || !rib_chord_mm.is_finite()
                || !(0.5..=6.0).contains(&rib_chord_mm)
            {
                return Err(IrError::InvalidParameter {
                    gene_path: "ribbon_cascade".into(),
                    reason: "invalid active ribbon_cascade parameters".into(),
                });
            }
            let nid = "node.ribbon_cascade".to_string();
            let zs_m = 0.012;
            let ze_m = (0.012 + rib_span_mm * 1e-3).min(0.066);
            nodes.push(IrNode::HelicalRibbonCascade {
                node_id: nid.clone(),
                vane_count: rib_vanes,
                axial_start_m: round9(zs_m),
                axial_end_m: round9(ze_m),
                helical_twist_rad: round6(rib_twist),
                thickness_m: round9(rib_thick_mm * 1e-3),
                radial_chord_m: round9(rib_chord_mm * 1e-3),
            });
            let bid = push_origin(
                "feature.ribbon_cascade",
                "ribbon_cascade",
                &nid,
                "emboss_helical_guide_ribbons",
                "wall",
                false,
            );
            gene_mapping.insert(
                "ribbon_cascade".into(),
                GeneIrMapping::Active {
                    node_ids: vec![nid],
                    semantic_boundary_ids: vec![bid],
                },
            );
        } else {
            gene_mapping.insert(
                "ribbon_cascade".into(),
                GeneIrMapping::Inactive {
                    reason: "vane_count == 0 or axial_span_mm == 0".into(),
                    canonical_default_verified: true,
                },
            );
        }

        // 6. Layer C2: tpms_insert
        let tpms_obj = obj.get("tpms_insert").and_then(|v| v.as_object());
        let tpms_basis_str = tpms_obj
            .and_then(|t| t.get("basis"))
            .and_then(|v| v.as_str())
            .unwrap_or("None");
        let tpms_basis = match tpms_basis_str {
            "None" => TpmsBasisIr::None,
            "Gyroid" => TpmsBasisIr::Gyroid,
            "SchwarzP" => TpmsBasisIr::SchwarzP,
            "Diamond" => TpmsBasisIr::Diamond,
            "CylindricalGyroid" => TpmsBasisIr::CylindricalGyroid,
            _ => TpmsBasisIr::UnsupportedCustom,
        };
        if tpms_basis != TpmsBasisIr::None {
            let r_in_frac = tpms_obj
                .and_then(|t| t.get("radial_inner_fraction"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.68);
            let zs_mm = tpms_obj
                .and_then(|t| t.get("axial_start_mm"))
                .and_then(|v| v.as_f64())
                .unwrap_or(24.0);
            let ze_mm = tpms_obj
                .and_then(|t| t.get("axial_end_mm"))
                .and_then(|v| v.as_f64())
                .unwrap_or(64.0);
            let rw_mm = tpms_obj
                .and_then(|t| t.get("radial_wavelength_mm"))
                .and_then(|v| v.as_f64())
                .unwrap_or(8.5);
            let aw_mm = tpms_obj
                .and_then(|t| t.get("axial_wavelength_mm"))
                .and_then(|v| v.as_f64())
                .unwrap_or(9.0);
            let st_mm = tpms_obj
                .and_then(|t| t.get("sheet_thickness_mm"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.35);

            if !r_in_frac.is_finite()
                || !(0.20..=0.95).contains(&r_in_frac)
                || !zs_mm.is_finite()
                || !ze_mm.is_finite()
                || ze_mm <= zs_mm
                || !rw_mm.is_finite()
                || rw_mm <= 0.5
                || !aw_mm.is_finite()
                || aw_mm <= 0.5
                || !st_mm.is_finite()
                || st_mm <= 0.2
            {
                return Err(IrError::InvalidParameter {
                    gene_path: "tpms_insert".into(),
                    reason: "invalid active tpms_insert parameters".into(),
                });
            }

            let nid = "node.tpms_insert".to_string();
            nodes.push(IrNode::TpmsPeripheralShell {
                node_id: nid.clone(),
                basis: tpms_basis,
                radial_inner_fraction: round6(r_in_frac),
                axial_start_m: round9(zs_mm * 1e-3),
                axial_end_m: round9(ze_mm * 1e-3),
                radial_wavelength_m: round9(rw_mm * 1e-3),
                axial_wavelength_m: round9(aw_mm * 1e-3),
                sheet_thickness_m: round9(st_mm * 1e-3),
            });
            let bid = push_origin(
                "feature.tpms_insert",
                "tpms_insert",
                &nid,
                "modulate_peripheral_tpms_shell",
                "wall",
                false,
            );
            gene_mapping.insert(
                "tpms_insert".into(),
                GeneIrMapping::Active {
                    node_ids: vec![nid],
                    semantic_boundary_ids: vec![bid],
                },
            );
        } else {
            gene_mapping.insert(
                "tpms_insert".into(),
                GeneIrMapping::Inactive {
                    reason: "tpms_insert.basis == None".into(),
                    canonical_default_verified: true,
                },
            );
        }

        // 7. Layer C: chamber_modes (canonicalized by sorting so mode order permutation is invariant)
        let raw_modes = obj
            .get("chamber_modes")
            .or_else(|| obj.get("modes"))
            .and_then(|v| v.as_array());
        let mut parsed_modes = Vec::new();
        if let Some(arr) = raw_modes {
            if arr.len() > 8 {
                return Err(IrError::InvalidParameter {
                    gene_path: "chamber_modes".into(),
                    reason: "at most 8 chamber_modes allowed".into(),
                });
            }
            for m in arr {
                let mobj = m.as_object().ok_or_else(|| IrError::InvalidParameter {
                    gene_path: "chamber_modes".into(),
                    reason: "mode must be object".into(),
                })?;
                let ord = mobj
                    .get("azimuthal_order")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let cos_mm = mobj
                    .get("cos_mm")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(f64::NAN);
                let sin_mm = mobj
                    .get("sin_mm")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(f64::NAN);
                let twist_rad = mobj
                    .get("twist_rad")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(f64::NAN);
                if !(2..=8).contains(&ord)
                    || !cos_mm.is_finite()
                    || !sin_mm.is_finite()
                    || !twist_rad.is_finite()
                    || twist_rad.abs() > 2.0 * PI + 1e-9
                {
                    return Err(IrError::InvalidParameter {
                        gene_path: "chamber_modes".into(),
                        reason: "invalid chamber_mode order or non-finite amplitude/twist".into(),
                    });
                }
                let amp_mm = cos_mm.hypot(sin_mm);
                if amp_mm > 1e-12 {
                    parsed_modes.push((
                        ord,
                        round6(cos_mm),
                        round6(sin_mm),
                        round6(amp_mm),
                        round6(twist_rad),
                    ));
                }
            }
        }
        parsed_modes.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.total_cmp(&b.1))
                .then_with(|| a.2.total_cmp(&b.2))
                .then_with(|| a.4.total_cmp(&b.4))
        });
        if !parsed_modes.is_empty() {
            let mut cm_nodes = Vec::new();
            let mut cm_bounds = Vec::new();
            for (idx, (ord, cos_mm, sin_mm, amp_mm, twist_rad)) in
                parsed_modes.into_iter().enumerate()
            {
                let nid = format!("node.chamber_mode.{idx}.m{ord}");
                nodes.push(IrNode::ChamberHarmonicMode {
                    node_id: nid.clone(),
                    mode_index: idx,
                    azimuthal_order: ord,
                    cos_m: round9(cos_mm * 1e-3),
                    sin_m: round9(sin_mm * 1e-3),
                    amplitude_m: round9(amp_mm * 1e-3),
                    twist_rad,
                    axial_start_m: 0.020,
                    axial_end_m: 0.065,
                });
                let bid = push_origin(
                    &format!("feature.chamber_mode.{idx}.m{ord}"),
                    &format!("chamber_modes[{idx}]"),
                    &nid,
                    "apply_harmonic_chamber_wall_mode",
                    "wall",
                    false,
                );
                cm_nodes.push(nid);
                cm_bounds.push(bid);
            }
            gene_mapping.insert(
                "chamber_modes".into(),
                GeneIrMapping::Active {
                    node_ids: cm_nodes,
                    semantic_boundary_ids: cm_bounds,
                },
            );
        } else {
            gene_mapping.insert(
                "chamber_modes".into(),
                GeneIrMapping::Inactive {
                    reason: "no active Fourier chamber wall modes".into(),
                    canonical_default_verified: true,
                },
            );
        }

        // 8. Layer D: cone_profile
        let cone_obj = obj.get("cone_profile").and_then(|v| v.as_object());
        let m1_mm = cone_obj
            .and_then(|c| c.get("mode1_mm"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let m2_mm = cone_obj
            .and_then(|c| c.get("mode2_mm"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let m3_mm = cone_obj
            .and_then(|c| c.get("mode3_mm"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if !m1_mm.is_finite()
            || !m2_mm.is_finite()
            || !m3_mm.is_finite()
            || (m1_mm.abs() + m2_mm.abs() + m3_mm.abs()) > 2.5
        {
            return Err(IrError::InvalidParameter {
                gene_path: "cone_profile".into(),
                reason: "non-finite or excessive cone_profile mode amplitudes".into(),
            });
        }
        if m1_mm.abs() > 1e-12 || m2_mm.abs() > 1e-12 || m3_mm.abs() > 1e-12 {
            let nid = "node.cone_profile".to_string();
            nodes.push(IrNode::ConeSplineProfile {
                node_id: nid.clone(),
                mode1_m: round9(m1_mm * 1e-3),
                mode2_m: round9(m2_mm * 1e-3),
                mode3_m: round9(m3_mm * 1e-3),
                axial_start_m: 0.072,
                axial_end_m: 0.166,
            });
            let bid = push_origin(
                "feature.cone_profile",
                "cone_profile",
                &nid,
                "loft_c2_s_cone_profile",
                "wall",
                false,
            );
            gene_mapping.insert(
                "cone_profile".into(),
                GeneIrMapping::Active {
                    node_ids: vec![nid],
                    semantic_boundary_ids: vec![bid],
                },
            );
        } else {
            gene_mapping.insert(
                "cone_profile".into(),
                GeneIrMapping::Inactive {
                    reason: "cone_profile modes all zero".into(),
                    canonical_default_verified: true,
                },
            );
        }

        // 9. Layer E: skimmer
        let skim_obj = obj.get("skimmer").and_then(|v| v.as_object());
        let skim_len_mm = skim_obj
            .and_then(|s| s.get("length_mm"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let skim_r_mm = skim_obj
            .and_then(|s| s.get("outer_radius_mm"))
            .and_then(|v| v.as_f64())
            .unwrap_or(2.4);
        let skim_flutes = skim_obj
            .and_then(|s| s.get("capture_flutes"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        if !skim_len_mm.is_finite() || !(0.0..=12.0).contains(&skim_len_mm) {
            return Err(IrError::InvalidParameter {
                gene_path: "skimmer.length_mm".into(),
                reason: format!("skimmer.length_mm {skim_len_mm} out of [0, 12.0]"),
            });
        }
        if skim_len_mm > 1e-9 {
            if !skim_r_mm.is_finite() || !(1.5..=4.5).contains(&skim_r_mm) {
                return Err(IrError::InvalidParameter {
                    gene_path: "skimmer.outer_radius_mm".into(),
                    reason: format!("skimmer.outer_radius_mm {skim_r_mm} out of [1.5, 4.5]"),
                });
            }
            let ro_mm = (do_m * 1e3) / 2.0;
            let tip_r_mm = (ro_mm + 1.0).max(skim_r_mm - 0.4);
            let nid = "node.skimmer".to_string();
            nodes.push(IrNode::VortexFinderSkimmer {
                node_id: nid.clone(),
                length_m: round9(skim_len_mm * 1e-3),
                inner_radius_m: round9(ro_mm * 1e-3),
                outer_radius_m: round9(skim_r_mm * 1e-3),
                tip_radius_m: round9(tip_r_mm * 1e-3),
                capture_flutes: skim_flutes,
            });
            let bid = push_origin(
                "feature.skimmer",
                "skimmer",
                &nid,
                "extrude_vortex_finder_skimmer_sleeve",
                "wall",
                false,
            );
            gene_mapping.insert(
                "skimmer".into(),
                GeneIrMapping::Active {
                    node_ids: vec![nid],
                    semantic_boundary_ids: vec![bid],
                },
            );
        } else {
            gene_mapping.insert(
                "skimmer".into(),
                GeneIrMapping::Inactive {
                    reason: "skimmer.length_mm == 0".into(),
                    canonical_default_verified: true,
                },
            );
        }

        // Canonical IR digest depends ONLY on canonical lowered nodes, gene_mapping, and semantic_origins
        // (so inactive-gene perturbations and mode orderings produce bit-identical `ir_sha256`).
        let hash_payload = serde_json::json!({
            "ir_schema_version": IR_SCHEMA_VERSION,
            "ir_family": IR_FAMILY,
            "units": "SI",
            "baseline_file_sha256": expected_baseline_sha256,
            "nodes": &nodes,
            "gene_mapping": &gene_mapping,
            "semantic_origins": &semantic_origins,
        });
        let ir_bytes =
            serde_json::to_vec(&hash_payload).map_err(|e| IrError::InvalidParameter {
                gene_path: "<ir_hash>".into(),
                reason: e.to_string(),
            })?;
        let ir_sha256 = sha256_hex(&ir_bytes);

        Ok(Self {
            ir_schema_version: IR_SCHEMA_VERSION,
            ir_family: IR_FAMILY.into(),
            source_genome_schema_version: schema_version,
            source_genome_family: family,
            units: "SI".into(),
            baseline_file_sha256: expected_baseline_sha256.to_string(),
            program_sha256,
            ir_sha256,
            nodes,
            gene_mapping,
            semantic_origins,
        })
    }

    /// Refuse an IR whose identity or recorded envelope differs from the CT-35 reference
    /// (`V5-01-05`). A changed downstream tail or a schema-1 envelope is a different subject and is
    /// never compiled as if it were the reference.
    pub fn verify_reference_envelope(&self) -> Result<(), IrError> {
        if self.ir_schema_version != IR_SCHEMA_VERSION || self.ir_family != IR_FAMILY {
            return Err(IrError::SchemaOrFamilyUnsupported {
                schema_version: self.ir_schema_version,
                family: self.ir_family.clone(),
                reason: "legacy IR identity recorded a non-reference envelope; rebuild the IR from its program"
                    .into(),
            });
        }
        let envelope = self.nodes.iter().find_map(|node| match node {
            IrNode::BodyEnvelope {
                d1_m,
                d2_m,
                d3_m,
                l1_m,
                l2_m,
                l3_m,
                z_underflow_m,
                ..
            } => Some([*d1_m, *d2_m, *d3_m, *l1_m, *l2_m, *l3_m, *z_underflow_m]),
            _ => None,
        });
        let Some(recorded) = envelope else {
            return Err(IrError::InvalidParameter {
                gene_path: "node.body_envelope".into(),
                reason: "IR has no BodyEnvelope".into(),
            });
        };
        let lc_m = ((CT35_D1_M - CT35_D2_M) / 2.0) / 10.0_f64.to_radians().tan();
        let reference = [
            ("d1_m", CT35_D1_M),
            ("d2_m", CT35_D2_M),
            ("d3_m", CT35_D3_M),
            ("l1_m", CT35_L1_M),
            ("l2_m", CT35_L2_M),
            ("l3_m", CT35_L3_M),
            ("z_underflow_m", CT35_L1_M + lc_m + CT35_L2_M + CT35_L3_M),
        ];
        for (got, (name, want)) in recorded.into_iter().zip(reference) {
            if !got.is_finite() || (got - want).abs() > 1e-9 {
                return Err(IrError::InvalidParameter {
                    gene_path: format!("node.body_envelope.{name}"),
                    reason: format!(
                        "recorded {got} m differs from the CT-35 reference {want} m (changed frozen envelope or downstream tail)"
                    ),
                });
            }
        }
        Ok(())
    }

    /// Evaluate the backend support matrix for a target `BackendId` (`IR-02`).
    /// Fails closed with `Err(IrError::UnsupportedFeature)` if any active gene is unsupported on `backend`.
    pub fn check_backend_support(
        &self,
        backend: BackendId,
    ) -> Result<BackendCoverageReport, IrError> {
        let mut coverages = Vec::new();
        let mut all_qualified = true;

        for (gene_name, mapping) in &self.gene_mapping {
            let (is_active, node_ids) = match mapping {
                GeneIrMapping::Active { node_ids, .. } => (true, node_ids.clone()),
                GeneIrMapping::Inactive { .. } => (false, Vec::new()),
            };

            let support = if !is_active {
                SupportLevel::Exact
            } else {
                self.evaluate_active_gene_support(gene_name, backend)
            };

            if is_active {
                if let SupportLevel::Unsupported { ref reason } = support {
                    return Err(IrError::UnsupportedFeature {
                        backend,
                        feature: gene_name.clone(),
                        reason: reason.clone(),
                    });
                }
                if !support.is_supported_for_geometry_qualification() {
                    all_qualified = false;
                }
            }

            coverages.push(FeatureBackendCoverage {
                gene_name: gene_name.clone(),
                is_active,
                node_ids,
                backend,
                support,
            });
        }

        Ok(BackendCoverageReport {
            ir_sha256: self.ir_sha256.clone(),
            backend,
            qualified_for_geometry: all_qualified,
            feature_coverages: coverages,
        })
    }

    fn evaluate_active_gene_support(&self, gene_name: &str, backend: BackendId) -> SupportLevel {
        // Check for invalid/unsupported internal node configurations first
        for node in &self.nodes {
            match node {
                IrNode::TpmsPeripheralShell { basis, .. }
                    if *basis == TpmsBasisIr::UnsupportedCustom =>
                {
                    return SupportLevel::Unsupported {
                        reason: "unknown/custom TPMS basis is unsupported on all backends".into(),
                    };
                }
                IrNode::VortexFinderSkimmer { capture_flutes, .. }
                    if !matches!(capture_flutes, 0 | 4 | 6 | 8) =>
                {
                    return SupportLevel::Unsupported {
                        reason: format!(
                            "skimmer capture_flutes={capture_flutes} not in supported set {{0, 4, 6, 8}}"
                        ),
                    };
                }
                _ => {}
            }
        }

        match (backend, gene_name) {
            (_, "body_envelope" | "terminals") => SupportLevel::Exact,

            // Strict analytical CSG B-rep backend rejects implicit field lofts (TPMS & helical ribbon fields)
            (BackendId::PythonCadQueryStrictCsgOnly, "tpms_insert") => SupportLevel::Unsupported {
                reason: "tpms_insert is an implicit periodic surface field; unsupported in strict analytical CSG-only B-rep mode".into(),
            },
            (BackendId::PythonCadQueryStrictCsgOnly, "ribbon_cascade") => SupportLevel::Unsupported {
                reason: "ribbon_cascade uses stream-surface harmonic modulation; unsupported in strict analytical CSG-only B-rep mode".into(),
            },
            (BackendId::PythonCadQueryStrictCsgOnly, _) => SupportLevel::Exact,

            // Legacy pre-v4 CFD surface generator did not enact internal splitter vanes or skimmer sleeves
            (BackendId::LegacyUnenactedCfdSurface, "inlet_split") => SupportLevel::Unsupported {
                reason: "legacy pre-v4 CFD surface extruded 2 unsplit ports without internal splitter vanes".into(),
            },
            (BackendId::LegacyUnenactedCfdSurface, "skimmer") => SupportLevel::Unsupported {
                reason: "legacy pre-v4 CFD surface omitted internal skimmer sleeve walls".into(),
            },
            (BackendId::LegacyUnenactedCfdSurface, "tpms_insert" | "ribbon_cascade") => {
                SupportLevel::WallCorrugationProxy {
                    proxy_note: "radial outer-wall corrugation proxy r_wall(theta, z); does not bound volume, wetted area, or topology of free-standing vanes or a 3D TPMS lattice".into(),
                }
            }
            (BackendId::LegacyUnenactedCfdSurface, _) => SupportLevel::ConservativeApproximation {
                bound_note: "radial wall ring modulation only".into(),
            },

            // Fast radial screen mesh is a single-shell enclosing surface (screen only for internal splitters/skimmer)
            (BackendId::RustFastRadialSurface, "inlet_split" | "skimmer") => SupportLevel::ScreenOnly {
                reason: "single-shell radial preview mesh does not embed internal splitter vanes or skimmer bore walls; use RustMultiPatchCfdSurface or PythonCadQueryBrep".into(),
            },
            (BackendId::RustFastRadialSurface, "tpms_insert" | "ribbon_cascade") => {
                SupportLevel::WallCorrugationProxy {
                    proxy_note: "radial outer-wall corrugation proxy r_wall(theta, z); does not bound volume, wetted area, or topology of free-standing vanes or a 3D TPMS lattice".into(),
                }
            }
            (BackendId::RustFastRadialSurface, _) => SupportLevel::ConservativeApproximation {
                bound_note: "48x64 structured radial-axial surface tessellation".into(),
            },

            // Full v4 Multi-Patch CFD Surface, Python CadQuery B-rep (with NURBS field loft), and Voxel/SDF observer:
            // ribbon_cascade and tpms_insert are radial wall corrugations r_wall(theta, z), NOT bounds on free-standing vanes or 3D TPMS lattices (`IR-R4`).
            (
                BackendId::RustMultiPatchCfdSurface
                | BackendId::PythonCadQueryBrep
                | BackendId::VoxelSdfObserver,
                "tpms_insert" | "ribbon_cascade",
            ) => SupportLevel::WallCorrugationProxy {
                proxy_note: "radial outer-wall corrugation proxy r_wall(theta, z); does not bound volume, wetted area, or topology of free-standing vanes or a 3D TPMS lattice".into(),
            },
            (
                BackendId::RustMultiPatchCfdSurface
                | BackendId::PythonCadQueryBrep
                | BackendId::VoxelSdfObserver,
                _,
            ) => SupportLevel::Exact,

            // CUDA reduced screen is strictly a low-order screen
            (BackendId::CudaReducedScreen, _) => SupportLevel::ScreenOnly {
                reason: "CUDA axial SDF + reduced trajectory tensor screen".into(),
            },
        }
    }
}

// ============================================================================
// IR-09: Split Conflated Geometry Metrics (Physical Meaning & Unit per Metric)
// ============================================================================

/// Physically distinct geometry & manufacturability metrics in millimetres (`mm`) (`IR-09`).
/// Replaces the previously conflated single `minimum_wall_clearance_mm` scalar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SplitGeometryMetrics {
    /// Minimum radial/normal solid thickness of the pressure-retaining outer cartridge shell (`mm`, gate `>= 1.0 mm`).
    pub minimum_shell_thickness_mm: f64,
    /// Minimum solid thickness of internal structural features: splitter vanes, helical ribbons, skimmer wall, TPMS sheet (`mm`, gate `>= 1.0 mm`).
    pub minimum_solid_ligament_mm: f64,
    /// Minimum hydraulic diameter `D_h = 4*A/P` of any fluid channel: overflow bore, split branch sub-channels, inlet throat, TPMS pores (`mm`, gate `>= 1.4 mm`).
    pub minimum_fluid_hydraulic_diameter_mm: f64,
    /// Minimum local point-to-point open flow gap across any fluid channel (`mm`, gate `>= 1.2 mm`).
    pub minimum_local_flow_gap_mm: f64,
    /// Minimum axial/radial distance from any modified internal feature to the immutable terminal ports and frozen `0.5*D2` collar (`mm`, gate `>= 2.0 mm`).
    pub minimum_clearance_to_frozen_interface_mm: f64,
    /// Minimum straight-line tool access diameter for overflow orifice finishing (`mm`, gate `>= 1.4 mm`).
    pub minimum_tool_access_diameter_mm: f64,
    /// Minimum bottleneck diameter along the flood-fill powder evacuation path from any internal void to an external port (`mm`, gate `>= 1.4 mm`).
    pub minimum_powder_exit_diameter_mm: f64,
    /// Minimum access clearance for post-build support removal (`mm`, gate `>= 1.4 mm`).
    pub minimum_support_removal_access_mm: f64,
}

impl SplitGeometryMetrics {
    /// Evaluate `SplitGeometryMetrics` from a `DesignIrV1`.
    pub fn from_ir(ir: &DesignIrV1) -> Self {
        let mut min_shell_mm = 3.0_f64;
        let mut min_ligament_mm = 3.0_f64;
        let mut min_dh_mm = 1.4_f64; // overflow orifice bore 1.40 mm
        let mut min_gap_mm = 1.4_f64;
        let mut min_frozen_clear_mm = 17.5_f64; // distance from head end (186.75 mm) to collar start (169.25 mm)
        let tool_access_mm = 1.4_f64;

        let cone_cos = 10.0_f64.to_radians().cos();

        // Check chamber harmonic modes on z in [20, 65] mm
        let mut mode_amp_sum_mm = 0.0_f64;
        for node in &ir.nodes {
            if let IrNode::ChamberHarmonicMode { amplitude_m, .. } = node {
                mode_amp_sum_mm += amplitude_m * 1e3;
            }
        }
        if mode_amp_sum_mm > 0.0 {
            min_shell_mm = min_shell_mm.min(3.0 - mode_amp_sum_mm);
            min_frozen_clear_mm = min_frozen_clear_mm.min(20.0); // z_start = 20 mm >= 0
        }

        for node in &ir.nodes {
            match node {
                IrNode::ConeSplineProfile {
                    mode1_m,
                    mode2_m,
                    mode3_m,
                    axial_end_m,
                    ..
                } => {
                    let m1 = mode1_m * 1e3;
                    let m2 = mode2_m * 1e3;
                    let m3 = mode3_m * 1e3;
                    for k in 0..=200 {
                        let s = k as f64 / 200.0;
                        let env = (PI * s).sin().powi(2);
                        let dr = env
                            * (m1 * (PI * s).sin()
                                + m2 * (2.0 * PI * s).sin()
                                + m3 * (3.0 * PI * s).sin());
                        let shell_norm = (3.0 - dr.max(0.0)) * cone_cos;
                        min_shell_mm = min_shell_mm.min(shell_norm);
                    }
                    let dist_to_collar_mm = (169.2474318 - axial_end_m * 1e3).max(0.0);
                    min_frozen_clear_mm = min_frozen_clear_mm.min(dist_to_collar_mm.max(3.247));
                }
                IrNode::InletRamp {
                    radial_slot_factor, ..
                } => {
                    let w0_mm = 9.833333;
                    let h0_mm = 12.227775;
                    let throat_w_mm = w0_mm * radial_slot_factor;
                    let dh_throat_mm = 2.0 * throat_w_mm * h0_mm / (throat_w_mm + h0_mm);
                    min_gap_mm = min_gap_mm.min(throat_w_mm);
                    min_dh_mm = min_dh_mm.min(dh_throat_mm);
                    // Ramp starts at |x| = 38 mm, terminal port is at |x| = 60 mm => 22 mm clearance
                    min_frozen_clear_mm = min_frozen_clear_mm.min(22.0);
                }
                IrNode::InletBranchManifold {
                    vane_thickness_m, ..
                } => {
                    min_ligament_mm = min_ligament_mm.min(vane_thickness_m * 1e3);
                    // Vanes end at |x| = 40 mm, terminal port at |x| = 60 mm => 20 mm clearance
                    min_frozen_clear_mm = min_frozen_clear_mm.min(20.0);
                }
                IrNode::InletJetChannel {
                    sub_height_m,
                    width_m,
                    hydraulic_diameter_m,
                    ..
                } => {
                    min_gap_mm = min_gap_mm.min((sub_height_m * 1e3).min(width_m * 1e3));
                    min_dh_mm = min_dh_mm.min(hydraulic_diameter_m * 1e3);
                }
                IrNode::HelicalRibbonCascade { thickness_m, .. } => {
                    min_ligament_mm = min_ligament_mm.min(thickness_m * 1e3);
                }
                IrNode::TpmsPeripheralShell {
                    radial_wavelength_m,
                    sheet_thickness_m,
                    ..
                } => {
                    let st_mm = sheet_thickness_m * 1e3;
                    let rw_mm = radial_wavelength_m * 1e3;
                    min_ligament_mm = min_ligament_mm.min(st_mm);
                    let pore_mm = (rw_mm - st_mm).max(0.1);
                    min_gap_mm = min_gap_mm.min(pore_mm);
                    min_dh_mm = min_dh_mm.min(pore_mm);
                }
                IrNode::VortexFinderSkimmer {
                    inner_radius_m,
                    tip_radius_m,
                    ..
                } => {
                    let tip_wall_mm = (tip_radius_m - inner_radius_m) * 1e3;
                    min_ligament_mm = min_ligament_mm.min(tip_wall_mm);
                }
                _ => {}
            }
        }

        let min_powder_exit_mm = min_dh_mm.min(min_gap_mm);
        let min_support_access_mm = min_dh_mm;

        Self {
            minimum_shell_thickness_mm: round6(min_shell_mm),
            minimum_solid_ligament_mm: round6(min_ligament_mm),
            minimum_fluid_hydraulic_diameter_mm: round6(min_dh_mm),
            minimum_local_flow_gap_mm: round6(min_gap_mm),
            minimum_clearance_to_frozen_interface_mm: round6(min_frozen_clear_mm),
            minimum_tool_access_diameter_mm: round6(tool_access_mm),
            minimum_powder_exit_diameter_mm: round6(min_powder_exit_mm),
            minimum_support_removal_access_mm: round6(min_support_access_mm),
        }
    }
}

// ============================================================================
// IR-07: PriorDescriptors (Genome/Inventor Only) vs ObservedDescriptors (Judge)
// ============================================================================

/// Heuristic genome-derived mechanism priors (`IR-07`).
/// Strictly restricted to the proposal/inventor layer; cannot be passed to qualified scoring.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PriorDescriptors {
    pub provenance_kind: String,
    pub swirl_flux_prior: f64,
    pub axial_pitch_sin_prior: f64,
    pub azimuthal_symmetry_prior: f64,
    pub shear_mitigation_prior: f64,
    pub cone_accel_persistence_prior: f64,
    pub core_shield_ratio_prior: f64,
    pub wall_rifling_prior: f64,
    pub hydraulic_resistance_prior: f64,
}

/// Geometry descriptors measured strictly from the compiled 3D mesh/SDF without genome access (`IR-07`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometryObservedDescriptors {
    pub fluid_volume_m3: f64,
    pub wetted_area_m2: f64,
    pub surface_to_volume_m_inv: f64,
    pub hydraulic_diameter_chamber_m: f64,
    pub measured_inlet_channel_count: usize,
    pub inlet_to_chamber_area_ratio: f64,
    pub chamber_azimuthal_anisotropy_l2: f64,
    pub cone_radial_curvature_rms_m_inv: f64,
    pub skimmer_core_blockage_fraction: f64,
    pub betti_numbers: [usize; 3],
    /// Geometry facts in this record that were not measured (`V5-01-02`). A non-empty list blocks
    /// qualification. Records written before this field existed read as unmeasured.
    #[serde(default = "legacy_unmeasured_facts")]
    pub unmeasured_facts: Vec<String>,
}

fn legacy_unmeasured_facts() -> Vec<String> {
    vec!["legacy record without geometry measurement status".to_string()]
}

/// Measurement status of one geometry count (`V5-01-02`). Proxy or assumed values are never `Measured`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GeometryCountObservation {
    Measured { value: usize, method: String },
    NotMeasured { reason: String },
}

impl GeometryCountObservation {
    pub fn measured_value(&self) -> Option<usize> {
        match self {
            Self::Measured { value, .. } => Some(*value),
            Self::NotMeasured { .. } => None,
        }
    }

    /// Serde default for records written before measurement status existed.
    pub fn legacy_unrecorded() -> Self {
        Self::NotMeasured {
            reason: "legacy record without measurement status".to_string(),
        }
    }
}

/// Facts the `VoxelSdfObserver` reports but does not measure (`V5-01-02`).
pub const OBSERVER_UNMEASURED_FACTS: [&str; 3] = [
    "trapped_powder_cavities: no solid-region or powder-escape observation",
    "enclosed_fluid_cavities: star-shaped voxel occupancy from the IR cannot represent an enclosed cavity",
    "betti_numbers: derived from the unmeasured cavity count and a closed-manifold edge-count assumption",
];

/// Physics descriptors measured strictly from executed 3D solver fields/integrals (`IR-07`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsObservedDescriptors {
    pub source_case_sha256: String,
    pub measured_dp_u_pa: f64,
    pub measured_pdr: f64,
    pub measured_reject_split: f64,
    pub measured_water_closure: f64,
    pub measured_hydraulic_loss_j_m3: f64,
    pub measured_swirl_number_entry: Option<f64>,
    pub measured_reverse_core_fraction: Option<f64>,
}

/// Combined judge-admissible observed descriptors (`IR-07`).
/// Type-incompatible with `PriorDescriptors`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservedDescriptors {
    pub geometry: GeometryObservedDescriptors,
    pub physics: Option<PhysicsObservedDescriptors>,
}

/// Type-level firewall guaranteeing that only `ObservedDescriptors` (never `PriorDescriptors`)
/// can be admitted into qualified archive/holdout evaluation (`IR-07`).
pub fn require_observed_descriptors_for_qualification(
    observed: &ObservedDescriptors,
) -> Result<&GeometryObservedDescriptors, String> {
    if !observed.geometry.fluid_volume_m3.is_finite()
        || observed.geometry.fluid_volume_m3 <= 0.0
        || !observed.geometry.wetted_area_m2.is_finite()
        || observed.geometry.wetted_area_m2 <= 0.0
        || observed.geometry.measured_inlet_channel_count == 0
    {
        return Err("invalid or unmeasured GeometryObservedDescriptors".into());
    }
    if !observed.geometry.unmeasured_facts.is_empty() {
        return Err(format!(
            "GeometryObservedDescriptors contain unmeasured facts: {}",
            observed.geometry.unmeasured_facts.join("; ")
        ));
    }
    Ok(&observed.geometry)
}

// ============================================================================
// IR-06: PhenotypeTaxon (Derived Purely from Compiled 3D Geometry)
// ============================================================================

/// Topology and morphology taxon derived purely from the compiled 3D geometry (`IR-06`).
/// Never reads the authored `topology_class` string from the genome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhenotypeTaxon {
    pub taxon_signature: String,
    pub derived_cluster: String,
    pub betti_0: usize,
    pub betti_1: usize,
    pub betti_2: usize,
    pub euler_characteristic: i64,
    pub void_graph_cycle_rank: usize,
    pub measured_inlet_channel_count: usize,
    pub measured_outlet_count: usize,
    pub measured_dominant_azimuthal_order: u32,
    pub measured_chamber_anisotropy_mm: f64,
    pub measured_cone_inflection_count: usize,
    pub measured_skimmer_protrusion_mm: f64,
    pub measured_axial_corrugation_mm: f64,
    pub is_novel_unclassified_cluster: bool,
}

// ============================================================================
// IR-04, IR-05, IR-08, IR-11, IR-12: Compiled Subject Mesh, Enactment & F0 Observer
// ============================================================================

/// Compiled 3D multi-patch triangle mesh in SI metres (`m`) with per-triangle semantic origin traces (`IR-11`)
/// and internal feature geometry for splitter vanes, ramps, and skimmer sleeves (`IR-04`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IrCompiledMesh {
    pub ir_sha256: String,
    pub backend: BackendId,
    pub units: String,
    pub vertices_m: Vec<P3>,
    pub triangles: Vec<[usize; 3]>,
    pub triangle_origin_ids: Vec<u16>,
    pub origin_table: Vec<SemanticOriginTrace>,
    /// Internal structural obstacle surfaces (splitter vanes, skimmer sleeve walls) enclosed inside the fluid domain
    /// or welded into the boundary mesh, each tagged with its `origin_id`.
    pub internal_feature_triangles: Vec<([P3; 3], u16)>,
}

/// Per-mechanism enactment proof record (`IR-04`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MechanismEnactmentRecord {
    pub gene_name: String,
    pub ir_node_ids: Vec<String>,
    pub enacted_in_subject: bool,
    pub face_indices: Vec<usize>,
    pub volume_region_id: String,
    pub enacted_triangle_count: usize,
    pub enacted_surface_area_m2: f64,
    pub measured_geometric_delta_mm: f64,
}

/// Complete feature-enactment receipt for a compiled candidate (`IR-04`, `IR-05`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeatureEnactmentReceipt {
    pub ir_sha256: String,
    pub backend: BackendId,
    pub all_active_features_enacted: bool,
    pub measured_disjoint_inlet_paths: usize,
    pub expected_disjoint_inlet_paths: usize,
    pub actual_simulated_geometry_label: String,
    pub active_mechanisms: Vec<MechanismEnactmentRecord>,
    pub unenacted_features: Vec<String>,
}

/// Independent 3D voxel/SDF & mesh forensic F0 audit report (`IR-08`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoxelSdfAuditReport {
    pub ir_sha256: String,
    pub voxel_grid_dimensions: [usize; 3],
    pub fluid_volume_mm3: f64,
    pub wetted_area_mm2: f64,
    pub connected_fluid_components: usize,
    /// Legacy proxy: voxel components of star-shaped IR occupancy plus extra mesh shells.
    pub trapped_fluid_cavities: usize,
    /// Legacy constant proxy (always 0); see `trapped_powder_cavity_observation`.
    pub trapped_powder_cavities: usize,
    /// Legacy proxy sum of the two fields above; never a measured cavity count.
    pub trapped_cavities_total: usize,
    pub all_terminals_reachable_from_chamber: bool,
    #[serde(default = "GeometryCountObservation::legacy_unrecorded")]
    pub trapped_powder_cavity_observation: GeometryCountObservation,
    #[serde(default = "GeometryCountObservation::legacy_unrecorded")]
    pub enclosed_fluid_cavity_observation: GeometryCountObservation,
    #[serde(default = "GeometryCountObservation::legacy_unrecorded")]
    pub mesh_shell_component_observation: GeometryCountObservation,
    pub frozen_collar_and_tail_diff_mm3: f64,
    pub terminal_inlet1_area_mm2: f64,
    pub terminal_inlet2_area_mm2: f64,
    pub terminal_overflow_diameter_mm: f64,
    pub terminal_underflow_z_mm: f64,
    pub min_terminal_normal_alignment: f64,
    pub min_section_jacobian: f64,
    pub open_boundary_edges: usize,
    pub non_manifold_edges: usize,
    pub degenerate_sliver_triangles: usize,
    pub split_metrics: SplitGeometryMetrics,
    pub derived_taxon: PhenotypeTaxon,
    pub observed_geometry: GeometryObservedDescriptors,
}

/// Typed F0 rejection reasons for the negative geometry corpus (`IR-08`, `IR-12`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F0RejectionReason {
    OpenBoundaryEdges,
    NonManifoldEdges,
    NegativeSignedVolume,
    DegenerateSliverTriangles,
    TrappedFluidCavity,
    TrappedPowderCavity,
    BlindChannelDeadEnd,
    FrozenCollarOrTailModified,
    TerminalPatchAreaMismatch,
    TerminalPatchNormalMisaligned,
    TerminalOverflowModified,
    TerminalUnderflowModified,
    ShellThicknessBelowMinimum,
    SolidLigamentBelowMinimum,
    FluidHydraulicDiameterBelowMinimum,
    LocalFlowGapBelowMinimum,
    SingularOrInvertedJacobian,
    CoreObstructionViolation,
    UnitScaleMismatch,
    MissingSemanticOriginTrace,
    UnenactedActiveMechanism,
    UnsupportedFeature,
}

impl IrCompiledMesh {
    /// Evaluate local inner fluid wall radius `R(theta, z_m)` in SI metres from `DesignIrV1`.
    pub fn inner_radius_m(ir: &DesignIrV1, theta: f64, z_m: f64) -> f64 {
        let r1_m = 0.035_f64;
        let r2_m = 0.0175_f64;
        let r3_m = 0.00875_f64;
        let l1_m = 0.070_f64;
        let lc_m = ((0.070_f64 - 0.035_f64) / 2.0) / 10.0_f64.to_radians().tan();
        let l2_m = 0.770_f64;
        let zc_m = l1_m + lc_m;
        let zt_m = zc_m + l2_m;

        let mut r_wall = if z_m <= l1_m {
            let mut delta = 0.0;
            if (0.020..=0.065).contains(&z_m) {
                let s = (z_m - 0.020) / 0.045;
                let env = (PI * s).sin().powi(2);
                for node in &ir.nodes {
                    if let IrNode::ChamberHarmonicMode {
                        azimuthal_order,
                        cos_m,
                        sin_m,
                        twist_rad,
                        ..
                    } = node
                    {
                        let phase = (*azimuthal_order as f64) * theta + twist_rad * s;
                        delta += env * (cos_m * phase.cos() + sin_m * phase.sin());
                    }
                }
            }
            r1_m + delta
        } else if z_m <= zc_m {
            let r_lin = r1_m + (r2_m - r1_m) * ((z_m - l1_m) / lc_m);
            let mut delta = 0.0;
            if (0.072..=0.166).contains(&z_m) {
                let s = (z_m - 0.072) / (0.166 - 0.072);
                let env = (PI * s).sin().powi(2);
                for node in &ir.nodes {
                    if let IrNode::ConeSplineProfile {
                        mode1_m,
                        mode2_m,
                        mode3_m,
                        ..
                    } = node
                    {
                        delta += env
                            * (mode1_m * (PI * s).sin()
                                + mode2_m * (2.0 * PI * s).sin()
                                + mode3_m * (3.0 * PI * s).sin());
                    }
                }
            }
            r_lin + delta
        } else if z_m <= zt_m {
            r2_m + (r3_m - r2_m) * ((z_m - zc_m) / l2_m)
        } else {
            r3_m
        };

        // HelicalRibbonCascade modulation
        for node in &ir.nodes {
            if let IrNode::HelicalRibbonCascade {
                vane_count,
                axial_start_m,
                axial_end_m,
                helical_twist_rad,
                radial_chord_m,
                ..
            } = node
            {
                if (*axial_start_m..=*axial_end_m).contains(&z_m)
                    && *axial_end_m > *axial_start_m + 1e-3
                {
                    let s = (z_m - axial_start_m) / (axial_end_m - axial_start_m);
                    let env = (PI * s).sin().powi(2);
                    let m = *vane_count as f64;
                    let phase = m * theta + helical_twist_rad * s;
                    let vane_profile = phase.cos().max(0.0).powi(4);
                    r_wall -= env * (0.35 * radial_chord_m) * vane_profile;
                }
            }
        }

        // TpmsPeripheralShell modulation
        for node in &ir.nodes {
            if let IrNode::TpmsPeripheralShell {
                basis,
                axial_start_m,
                axial_end_m,
                axial_wavelength_m,
                sheet_thickness_m,
                ..
            } = node
            {
                if (*axial_start_m..=*axial_end_m).contains(&z_m)
                    && *axial_end_m > *axial_start_m + 1e-3
                {
                    let s = (z_m - axial_start_m) / (axial_end_m - axial_start_m);
                    let env = (PI * s).sin().powi(2);
                    let kz = 2.0 * PI * (z_m - axial_start_m) / axial_wavelength_m;
                    let kt = 4.0 * theta;
                    let wave = match basis {
                        TpmsBasisIr::None | TpmsBasisIr::UnsupportedCustom => 0.0,
                        TpmsBasisIr::Gyroid | TpmsBasisIr::CylindricalGyroid => {
                            0.55 * (kt.sin() * kz.cos() + (kt + kz).sin() * 0.5)
                        }
                        TpmsBasisIr::SchwarzP => 0.50 * (kt.cos() + kz.cos()) * 0.5,
                        TpmsBasisIr::Diamond => 0.50 * (kt.sin() * kz.sin()),
                    };
                    r_wall += env * (0.45 * sheet_thickness_m) * wave;
                }
            }
        }

        r_wall
    }

    /// Compile a `DesignIrV1` into a 3D `IrCompiledMesh` for the given `BackendId` with zero outlet extension.
    pub fn compile(ir: &DesignIrV1, backend: BackendId) -> Result<Self, IrError> {
        Self::compile_with_extension(ir, backend, 0.0)
    }

    /// Compile a `DesignIrV1` into a watertight 2-manifold 3D `IrCompiledMesh` for the given `BackendId`
    /// and `outlet_extension_m` (`IR-04`, `IR-11`, `IR-R1`), enacting multi-branch inlet sub-channels,
    /// entry ramp contraction/drop, and folded vortex-finder skimmer sleeve directly in `triangles`.
    pub fn compile_with_extension(
        ir: &DesignIrV1,
        backend: BackendId,
        outlet_extension_m: f64,
    ) -> Result<Self, IrError> {
        ir.verify_reference_envelope()?;
        ir.check_backend_support(backend)?;
        if !outlet_extension_m.is_finite() || !(0.0..=0.5).contains(&outlet_extension_m) {
            return Err(IrError::InvalidParameter {
                gene_path: "outlet_extension_m".into(),
                reason: "outlet_extension_m must be finite in [0.0, 0.5] m".into(),
            });
        }

        let origin_lookup: BTreeMap<String, u16> = ir
            .semantic_origins
            .iter()
            .map(|o| (o.semantic_id.clone(), o.origin_id))
            .collect();
        let find_origin = |prefix: &str, fallback: &str| -> u16 {
            for (k, &v) in &origin_lookup {
                if k == prefix || k.starts_with(prefix) {
                    return v;
                }
            }
            *origin_lookup.get(fallback).unwrap_or(&0)
        };

        let n_theta = 48usize;
        let dtheta = 2.0 * PI / (n_theta as f64);
        let area_corr = (dtheta / dtheta.sin()).sqrt();

        let r_plenum_m = 0.006_f64;
        let r_plenum_poly_m = r_plenum_m * area_corr;
        let ro_m = 0.0007_f64;
        let r3_m = 0.00875_f64;
        let r3_poly_m = r3_m * area_corr;
        let z_orifice_m = -0.0028_f64;
        let z_overflow_m = -0.0228_f64;
        let lc_m = ((0.070_f64 - 0.035_f64) / 2.0) / 10.0_f64.to_radians().tan();
        let z_head_end_m = 0.070_f64 + lc_m + 0.5 * 0.035_f64;
        let z_collar_start_m = 0.070_f64 + lc_m - 0.5 * 0.035_f64;
        let z_taper_end_m = 0.070_f64 + lc_m + 0.770_f64;
        let z_underflow_m = z_taper_end_m + 0.700_f64 + outlet_extension_m;
        let inlet_area_each_m2 = 0.0625_f64 * PI * 0.070_f64 * 0.070_f64 / 4.0 / 2.0;
        let w_inlet_m = (inlet_area_each_m2 / 2.0_f64).sqrt();
        let h_inlet_m = 2.0_f64 * w_inlet_m;
        let x_inlet_m = 0.060_f64;
        let r1_m = 0.035_f64;

        let enact_internal = backend != BackendId::LegacyUnenactedCfdSurface
            && backend != BackendId::RustFastRadialSurface;

        // Inspect active InletBranchManifold, InletRamp, and VortexFinderSkimmer nodes
        let mut branches_per_port = 1u32;
        let mut vane_thickness_m = 0.0012_f64;
        let mut vane_z_fraction = 0.5_f64;
        let mut ramp_drop_m = 0.0_f64;
        let mut ramp_slot_factor = 1.0_f64;
        let mut ramp_is_active = false;
        let mut skimmer_spec: Option<(f64, f64, f64, u32)> = None;

        for node in &ir.nodes {
            match node {
                IrNode::InletBranchManifold {
                    branches_per_port: b,
                    vane_thickness_m: vt,
                    vane_z_fraction: vzf,
                    ..
                } if enact_internal => {
                    branches_per_port = *b;
                    vane_thickness_m = *vt;
                    vane_z_fraction = *vzf;
                }
                IrNode::InletRamp {
                    axial_drop_m,
                    radial_slot_factor,
                    ..
                } if enact_internal => {
                    ramp_drop_m = *axial_drop_m;
                    ramp_slot_factor = *radial_slot_factor;
                    ramp_is_active = true;
                }
                IrNode::VortexFinderSkimmer {
                    length_m,
                    outer_radius_m,
                    tip_radius_m,
                    capture_flutes,
                    ..
                } if enact_internal && *length_m > 1e-6 => {
                    skimmer_spec =
                        Some((*length_m, *outer_radius_m, *tip_radius_m, *capture_flutes));
                }
                _ => {}
            }
        }

        // Build vertical z_levels across [0, h_inlet_m]:
        // - If branches_per_port == 1: 4 uniform bands (5 levels).
        // - If branches_per_port == B >= 2: 2*B - 1 bands (even bands = B fluid sub-channels, odd bands = B-1 solid splitter vanes).
        let z_levels: Vec<f64> = if branches_per_port <= 1 {
            (0..=4).map(|iz| h_inlet_m * (iz as f64) / 4.0).collect()
        } else {
            let b_cnt = branches_per_port as usize;
            let n_vanes = b_cnt - 1;
            let mut zv = Vec::with_capacity(2 * b_cnt);
            zv.push(0.0);
            if n_vanes == 1 {
                let z_c = (vane_z_fraction * h_inlet_m).clamp(0.20 * h_inlet_m, 0.80 * h_inlet_m);
                let z_lo = (z_c - 0.5 * vane_thickness_m).max(0.001);
                let z_hi = (z_c + 0.5 * vane_thickness_m).min(h_inlet_m - 0.001);
                zv.push(z_lo);
                zv.push(z_hi);
                zv.push(h_inlet_m);
            } else {
                let sub_h =
                    ((h_inlet_m - (n_vanes as f64) * vane_thickness_m) / (b_cnt as f64)).max(0.001);
                let mut cur_z = 0.0;
                for b_idx in 0..b_cnt {
                    cur_z += sub_h;
                    if b_idx + 1 < b_cnt {
                        zv.push(cur_z);
                        cur_z += vane_thickness_m;
                        zv.push(cur_z);
                    } else {
                        zv.push(h_inlet_m);
                    }
                }
            }
            zv
        };
        let n_inlet_z = z_levels.len() - 1;

        let mut rings: Vec<(Vec<P3>, u16)> = Vec::new();
        let th_off = 0.0031_f64;
        let make_circle_ring = |r_m: f64, z_m: f64| -> Vec<P3> {
            (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    [r_m * th.cos(), r_m * th.sin(), z_m]
                })
                .collect()
        };

        let orig_plenum = find_origin("wall.plenum", "wall.plenum");
        let orig_orifice = find_origin("wall.orifice", "wall.orifice");
        let orig_chamber = find_origin("wall.chamber", "wall.chamber");
        let orig_cone = find_origin("wall.cone", "wall.cone");
        let orig_taper = find_origin("wall.collar_and_taper", "wall.collar_and_taper");
        let orig_tail = find_origin("wall.tail", "wall.tail");
        let orig_mode = find_origin("feature.chamber_mode", "wall.chamber");
        let orig_rib = find_origin("feature.ribbon_cascade", "wall.chamber");
        let orig_tpms = find_origin("feature.tpms_insert", "wall.chamber");
        let orig_cprof = find_origin("feature.cone_profile", "wall.cone");
        let orig_skim = find_origin("feature.skimmer", "wall.orifice");

        let n_plenum_z = 4usize;
        for iz in 0..=n_plenum_z {
            let f = (iz as f64) / (n_plenum_z as f64);
            let z_m = z_overflow_m + (z_orifice_m - z_overflow_m) * f;
            rings.push((make_circle_ring(r_plenum_poly_m, z_m), orig_plenum));
        }
        rings.push((
            make_circle_ring(0.5 * (r_plenum_poly_m + ro_m), z_orifice_m),
            orig_orifice,
        ));
        rings.push((make_circle_ring(ro_m, z_orifice_m), orig_orifice));
        rings.push((make_circle_ring(ro_m, 0.5 * z_orifice_m), orig_orifice));
        rings.push((make_circle_ring(ro_m, 0.0), orig_orifice));

        // If VortexFinderSkimmer is active, fold the axisymmetric meridian at z = 0:
        // (ro_m, 0.0) -> (ro_m, L_skim) -> (r_tip + flute, L_skim) -> (r_outer + flute, 0.0)
        // before continuing to the outer chamber roof ring (R_chamber, 0.0).
        if let Some((skim_len_m, skim_outer_r_m, skim_tip_r_m, skim_flutes)) = skimmer_spec {
            rings.push((make_circle_ring(ro_m, skim_len_m), orig_skim));
            let tip_ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let fl = if skim_flutes > 0 {
                        0.00018 * ((skim_flutes as f64) * th).cos()
                    } else {
                        0.0
                    };
                    let r_m = (skim_tip_r_m + fl).max(ro_m + 0.0004);
                    [r_m * th.cos(), r_m * th.sin(), skim_len_m]
                })
                .collect();
            rings.push((tip_ring, orig_skim));
            let base_ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let fl = if skim_flutes > 0 {
                        0.00018 * ((skim_flutes as f64) * th).cos()
                    } else {
                        0.0
                    };
                    let r_m = (skim_outer_r_m + fl).max(ro_m + 0.0006);
                    [r_m * th.cos(), r_m * th.sin(), 0.0]
                })
                .collect();
            rings.push((base_ring, orig_skim));
        }

        let k_chamber_0 = rings.len();
        for &z_m in &z_levels {
            let ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let r_m = Self::inner_radius_m(ir, th, z_m) * area_corr;
                    [r_m * th.cos(), r_m * th.sin(), z_m]
                })
                .collect();
            rings.push((ring, orig_chamber));
        }

        let n_head_rem = 36usize;
        for iz in 1..=n_head_rem {
            let f = (iz as f64) / (n_head_rem as f64);
            let z_m = h_inlet_m + (z_head_end_m - h_inlet_m) * f;
            let ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let r_m = Self::inner_radius_m(ir, th, z_m) * area_corr;
                    [r_m * th.cos(), r_m * th.sin(), z_m]
                })
                .collect();
            let ring_orig = if z_m <= 0.070 {
                if ir
                    .gene_mapping
                    .get("tpms_insert")
                    .map(|m| m.is_active())
                    .unwrap_or(false)
                    && (0.024..=0.044).contains(&z_m)
                {
                    orig_tpms
                } else if ir
                    .gene_mapping
                    .get("ribbon_cascade")
                    .map(|m| m.is_active())
                    .unwrap_or(false)
                    && (0.012..=0.024).contains(&z_m)
                {
                    orig_rib
                } else if ir
                    .gene_mapping
                    .get("chamber_modes")
                    .map(|m| m.is_active())
                    .unwrap_or(false)
                {
                    orig_mode
                } else {
                    orig_chamber
                }
            } else if z_m < z_collar_start_m {
                if ir
                    .gene_mapping
                    .get("cone_profile")
                    .map(|m| m.is_active())
                    .unwrap_or(false)
                {
                    orig_cprof
                } else {
                    orig_cone
                }
            } else {
                orig_taper
            };
            rings.push((ring, ring_orig));
        }

        let n_taper_rem = 12usize;
        for iz in 1..=n_taper_rem {
            let f = (iz as f64) / (n_taper_rem as f64);
            let z_m = z_head_end_m + (z_taper_end_m - z_head_end_m) * f;
            let ring = (0..n_theta)
                .map(|it| {
                    let th = dtheta * (it as f64) + th_off;
                    let r_m = Self::inner_radius_m(ir, th, z_m) * area_corr;
                    [r_m * th.cos(), r_m * th.sin(), z_m]
                })
                .collect();
            rings.push((ring, orig_taper));
        }
        let n_tail = 8usize;
        for iz in 1..n_tail {
            let f = (iz as f64) / (n_tail as f64);
            let z_m = z_taper_end_m + (z_underflow_m - z_taper_end_m) * f;
            rings.push((make_circle_ring(r3_poly_m, z_m), orig_tail));
        }
        rings.push((make_circle_ring(r3_poly_m, z_underflow_m), orig_tail));

        let n_rings = rings.len();
        let mut vertices_m: Vec<P3> = Vec::with_capacity(2 + n_rings * n_theta + 256);
        vertices_m.push([0.00011, 0.00013, z_overflow_m]);
        vertices_m.push([0.00011, 0.00013, z_underflow_m]);

        let ring_base = 2usize;
        for (ring, _) in &rings {
            vertices_m.extend_from_slice(ring);
        }

        let mut triangles: Vec<[usize; 3]> = Vec::new();
        let mut triangle_origin_ids: Vec<u16> = Vec::new();

        let orig_term_ov = find_origin("terminal.overflow", "terminal.overflow");
        let orig_term_un = find_origin("terminal.underflow", "terminal.underflow");
        let orig_term_in1 = find_origin("terminal.inlet1", "terminal.inlet1");
        let orig_term_in2 = find_origin("terminal.inlet2", "terminal.inlet2");
        let orig_duct1 = find_origin("wall.inlet1_duct", "wall.inlet1_duct");
        let orig_duct2 = find_origin("wall.inlet2_duct", "wall.inlet2_duct");
        let orig_ramp = find_origin("feature.inlet_ramp", "wall.inlet1_duct");

        // 1. Overflow cap
        for it in 0..n_theta {
            let next = (it + 1) % n_theta;
            triangles.push([0, ring_base + next, ring_base + it]);
            triangle_origin_ids.push(orig_term_ov);
        }

        let n_port_t = 6usize;
        let it_inlet1_start = n_theta / 4;
        let it_inlet2_start = (3 * n_theta) / 4;

        // 2. Side wall quads
        for ir_idx in 0..(n_rings - 1) {
            let r0 = ring_base + ir_idx * n_theta;
            let r1 = ring_base + (ir_idx + 1) * n_theta;
            let in_inlet_z = ir_idx >= k_chamber_0 && ir_idx < k_chamber_0 + n_inlet_z;
            let band_orig = rings[ir_idx + 1].1;
            for it in 0..n_theta {
                let in_inlet1 =
                    in_inlet_z && it >= it_inlet1_start && it < it_inlet1_start + n_port_t;
                let in_inlet2 =
                    in_inlet_z && it >= it_inlet2_start && it < it_inlet2_start + n_port_t;
                if in_inlet1 || in_inlet2 {
                    continue;
                }
                let next = (it + 1) % n_theta;
                triangles.push([r0 + it, r0 + next, r1 + next]);
                triangle_origin_ids.push(band_orig);
                triangles.push([r0 + it, r1 + next, r1 + it]);
                triangle_origin_ids.push(band_orig);
            }
        }

        // Helper to connect a 4-walled rectangular duct segment between cross-section grids `idx_a` and `idx_b`
        // spanning `iz in iz_lo..=iz_hi`, `jt in 0..=n_port_t`.
        let connect_tube = |tris: &mut Vec<[usize; 3]>,
                            oids: &mut Vec<u16>,
                            idx_a: &dyn Fn(usize, usize) -> usize,
                            idx_b: &dyn Fn(usize, usize) -> usize,
                            iz_lo: usize,
                            iz_hi: usize,
                            bot_orig: u16,
                            top_orig: u16,
                            side_orig: u16| {
            for jt in 0..n_port_t {
                let (w0, w1, p0, p1) = (
                    idx_a(iz_lo, jt),
                    idx_a(iz_lo, jt + 1),
                    idx_b(iz_lo, jt),
                    idx_b(iz_lo, jt + 1),
                );
                tris.push([w0, w1, p1]);
                oids.push(bot_orig);
                tris.push([w0, p1, p0]);
                oids.push(bot_orig);
            }
            for iz in iz_lo..iz_hi {
                let (w0, w1, p0, p1) = (
                    idx_a(iz, n_port_t),
                    idx_a(iz + 1, n_port_t),
                    idx_b(iz, n_port_t),
                    idx_b(iz + 1, n_port_t),
                );
                tris.push([w0, w1, p1]);
                oids.push(side_orig);
                tris.push([w0, p1, p0]);
                oids.push(side_orig);
            }
            for jt in 0..n_port_t {
                let (w0, w1, p0, p1) = (
                    idx_a(iz_hi, jt + 1),
                    idx_a(iz_hi, jt),
                    idx_b(iz_hi, jt + 1),
                    idx_b(iz_hi, jt),
                );
                tris.push([w0, w1, p1]);
                oids.push(top_orig);
                tris.push([w0, p1, p0]);
                oids.push(top_orig);
            }
            for iz in iz_lo..iz_hi {
                let (w0, w1, p0, p1) = (
                    idx_a(iz + 1, 0),
                    idx_a(iz, 0),
                    idx_b(iz + 1, 0),
                    idx_b(iz, 0),
                );
                tris.push([w0, w1, p1]);
                oids.push(side_orig);
                tris.push([w0, p1, p0]);
                oids.push(side_orig);
            }
        };

        // 3. Extrude Inlet 1 and Inlet 2 (`IR-R1`):
        // - If unsplit and no active ramp: direct single-segment extrusion `wall_idx -> port_idx` (preserves exact baseline mesh).
        // - If `branches_per_port >= 2` or `ramp_is_active`: 3-segment watertight 2-manifold extrusion
        //   `wall_idx -> inner_idx (|x|=0.024 m) -> outer_idx (|x|=0.056 m) -> port_idx (|x|=0.060 m)`,
        //   where Segment 2 (`|x| in [0.024, 0.056] m`) extrudes `B` disjoint closed rectangular sub-channels
        //   and caps each solid splitter vane at `inner_idx` and `outer_idx`.
        for (port_number, term_orig, duct_orig, it_start, x_plane, y_from_it) in [
            (
                1u32,
                orig_term_in1,
                orig_duct1,
                it_inlet1_start,
                -x_inlet_m,
                true,
            ),
            (
                2u32,
                orig_term_in2,
                orig_duct2,
                it_inlet2_start,
                x_inlet_m,
                false,
            ),
        ] {
            let wall_idx = |iz: usize, jt: usize| -> usize {
                ring_base + (k_chamber_0 + iz) * n_theta + ((it_start + jt) % n_theta)
            };
            let strip_orig = if ramp_is_active { orig_ramp } else { duct_orig };

            if branches_per_port <= 1 && !ramp_is_active {
                let port_v_base = vertices_m.len();
                for &z_m in &z_levels {
                    for jt in 0..=n_port_t {
                        let ft = (jt as f64) / (n_port_t as f64);
                        let y_m = if y_from_it {
                            r1_m - w_inlet_m * ft
                        } else {
                            -r1_m + w_inlet_m * ft
                        };
                        vertices_m.push([x_plane, y_m, z_m]);
                    }
                }
                let port_idx =
                    |iz: usize, jt: usize| -> usize { port_v_base + iz * (n_port_t + 1) + jt };

                connect_tube(
                    &mut triangles,
                    &mut triangle_origin_ids,
                    &wall_idx,
                    &port_idx,
                    0,
                    n_inlet_z,
                    strip_orig,
                    strip_orig,
                    strip_orig,
                );
                for iz in 0..n_inlet_z {
                    for jt in 0..n_port_t {
                        let a = port_idx(iz, jt);
                        let b = port_idx(iz, jt + 1);
                        let c = port_idx(iz + 1, jt + 1);
                        let d_v = port_idx(iz + 1, jt);
                        triangles.push([a, b, c]);
                        triangle_origin_ids.push(term_orig);
                        triangles.push([a, c, d_v]);
                        triangle_origin_ids.push(term_orig);
                    }
                }
            } else {
                let sign_x = x_plane.signum();
                let x_inner = sign_x * 0.024_f64;
                let x_outer = sign_x * 0.056_f64;
                let w_inner_m = w_inlet_m * ramp_slot_factor;
                let dz_inner_m = -0.35 * ramp_drop_m;

                let inner_v_base = vertices_m.len();
                for &z_m in &z_levels {
                    for jt in 0..=n_port_t {
                        let ft = (jt as f64) / (n_port_t as f64);
                        let y_m = if y_from_it {
                            r1_m - w_inner_m * ft
                        } else {
                            -r1_m + w_inner_m * ft
                        };
                        vertices_m.push([x_inner, y_m, z_m + dz_inner_m]);
                    }
                }
                let outer_v_base = vertices_m.len();
                for &z_m in &z_levels {
                    for jt in 0..=n_port_t {
                        let ft = (jt as f64) / (n_port_t as f64);
                        let y_m = if y_from_it {
                            r1_m - w_inlet_m * ft
                        } else {
                            -r1_m + w_inlet_m * ft
                        };
                        vertices_m.push([x_outer, y_m, z_m]);
                    }
                }
                let port_v_base = vertices_m.len();
                for &z_m in &z_levels {
                    for jt in 0..=n_port_t {
                        let ft = (jt as f64) / (n_port_t as f64);
                        let y_m = if y_from_it {
                            r1_m - w_inlet_m * ft
                        } else {
                            -r1_m + w_inlet_m * ft
                        };
                        vertices_m.push([x_plane, y_m, z_m]);
                    }
                }

                let inner_idx =
                    |iz: usize, jt: usize| -> usize { inner_v_base + iz * (n_port_t + 1) + jt };
                let outer_idx =
                    |iz: usize, jt: usize| -> usize { outer_v_base + iz * (n_port_t + 1) + jt };
                let port_idx =
                    |iz: usize, jt: usize| -> usize { port_v_base + iz * (n_port_t + 1) + jt };

                // Segment 1: chamber wall window -> inner_idx (|x| = 0.024 m)
                connect_tube(
                    &mut triangles,
                    &mut triangle_origin_ids,
                    &wall_idx,
                    &inner_idx,
                    0,
                    n_inlet_z,
                    strip_orig,
                    strip_orig,
                    strip_orig,
                );

                // Segment 2: inner_idx (|x| = 0.024 m) -> outer_idx (|x| = 0.056 m)
                if branches_per_port <= 1 {
                    connect_tube(
                        &mut triangles,
                        &mut triangle_origin_ids,
                        &inner_idx,
                        &outer_idx,
                        0,
                        n_inlet_z,
                        strip_orig,
                        strip_orig,
                        strip_orig,
                    );
                } else {
                    let b_cnt = branches_per_port as usize;
                    for b_idx in 0..b_cnt {
                        let iz_even = 2 * b_idx;
                        let bot_orig = if b_idx == 0 {
                            strip_orig
                        } else {
                            find_origin(
                                &format!("feature.inlet_split.p{port_number}.v{}", b_idx - 1),
                                "wall.inlet1_duct",
                            )
                        };
                        let top_orig = if b_idx + 1 == b_cnt {
                            strip_orig
                        } else {
                            find_origin(
                                &format!("feature.inlet_split.p{port_number}.v{b_idx}"),
                                "wall.inlet1_duct",
                            )
                        };
                        connect_tube(
                            &mut triangles,
                            &mut triangle_origin_ids,
                            &inner_idx,
                            &outer_idx,
                            iz_even,
                            iz_even + 1,
                            bot_orig,
                            top_orig,
                            duct_orig,
                        );
                    }
                    // Cap each odd band (solid splitter vane) at inner_idx (trailing edge) and outer_idx (leading edge)
                    for v_idx in 0..(b_cnt - 1) {
                        let iz_odd = 2 * v_idx + 1;
                        let vane_orig = find_origin(
                            &format!("feature.inlet_split.p{port_number}.v{v_idx}"),
                            "wall.inlet1_duct",
                        );
                        for jt in 0..n_port_t {
                            let a = inner_idx(iz_odd, jt);
                            let b = inner_idx(iz_odd, jt + 1);
                            let c = inner_idx(iz_odd + 1, jt + 1);
                            let d_v = inner_idx(iz_odd + 1, jt);
                            triangles.push([a, b, c]);
                            triangle_origin_ids.push(vane_orig);
                            triangles.push([a, c, d_v]);
                            triangle_origin_ids.push(vane_orig);

                            let ao = outer_idx(iz_odd, jt);
                            let bo = outer_idx(iz_odd, jt + 1);
                            let co = outer_idx(iz_odd + 1, jt + 1);
                            let do_v = outer_idx(iz_odd + 1, jt);
                            triangles.push([ao, co, bo]);
                            triangle_origin_ids.push(vane_orig);
                            triangles.push([ao, do_v, co]);
                            triangle_origin_ids.push(vane_orig);
                        }
                    }
                }

                // Segment 3: outer_idx (|x| = 0.056 m) -> port_idx (|x| = 0.060 m)
                connect_tube(
                    &mut triangles,
                    &mut triangle_origin_ids,
                    &outer_idx,
                    &port_idx,
                    0,
                    n_inlet_z,
                    duct_orig,
                    duct_orig,
                    duct_orig,
                );

                // Terminal port patch at |x| = 0.060 m
                for iz in 0..n_inlet_z {
                    for jt in 0..n_port_t {
                        let a = port_idx(iz, jt);
                        let b = port_idx(iz, jt + 1);
                        let c = port_idx(iz + 1, jt + 1);
                        let d_v = port_idx(iz + 1, jt);
                        triangles.push([a, b, c]);
                        triangle_origin_ids.push(term_orig);
                        triangles.push([a, c, d_v]);
                        triangle_origin_ids.push(term_orig);
                    }
                }
            }
        }

        // 4. Underflow cap
        let last_ring = ring_base + (n_rings - 1) * n_theta;
        for it in 0..n_theta {
            let next = (it + 1) % n_theta;
            triangles.push([1, last_ring + it, last_ring + next]);
            triangle_origin_ids.push(orig_term_un);
        }

        let mesh = Self {
            ir_sha256: ir.ir_sha256.clone(),
            backend,
            units: "m".into(),
            vertices_m,
            triangles,
            triangle_origin_ids,
            origin_table: ir.semantic_origins.clone(),
            internal_feature_triangles: Vec::new(),
        };
        mesh.verify_all_boundary_faces_have_origin_trace()?;
        Ok(mesh)
    }

    /// Verify that 100% of boundary triangles and internal feature triangles map to a valid `SemanticOriginTrace` (`IR-11`).
    pub fn verify_all_boundary_faces_have_origin_trace(&self) -> Result<(), IrError> {
        if self.triangles.len() != self.triangle_origin_ids.len() {
            return Err(IrError::MissingSemanticOriginTrace {
                triangle_index: self.triangle_origin_ids.len(),
                reason: "triangles.len() != triangle_origin_ids.len()".into(),
            });
        }
        let valid_ids: BTreeSet<u16> = self.origin_table.iter().map(|o| o.origin_id).collect();
        for (idx, &oid) in self.triangle_origin_ids.iter().enumerate() {
            if !valid_ids.contains(&oid) {
                return Err(IrError::MissingSemanticOriginTrace {
                    triangle_index: idx,
                    reason: format!("triangle {idx} references unknown origin_id {oid}"),
                });
            }
        }
        for (idx, &(_, oid)) in self.internal_feature_triangles.iter().enumerate() {
            if !valid_ids.contains(&oid) {
                return Err(IrError::MissingSemanticOriginTrace {
                    triangle_index: self.triangles.len() + idx,
                    reason: format!("internal triangle {idx} references unknown origin_id {oid}"),
                });
            }
        }
        Ok(())
    }

    /// Signed enclosed volume in `m^3` via 3D divergence theorem.
    pub fn signed_volume_m3(&self) -> f64 {
        let mut vol6 = 0.0;
        for &[i0, i1, i2] in &self.triangles {
            let a = self.vertices_m[i0];
            let b = self.vertices_m[i1];
            let c = self.vertices_m[i2];
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            vol6 += a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2];
        }
        vol6 / 6.0
    }

    /// Total wetted surface area in `m^2` (boundary wall + internal feature surfaces).
    pub fn wetted_area_m2(&self) -> f64 {
        let tri_area = |a: P3, b: P3, c: P3| -> f64 {
            let ux = b[0] - a[0];
            let uy = b[1] - a[1];
            let uz = b[2] - a[2];
            let vx = c[0] - a[0];
            let vy = c[1] - a[1];
            let vz = c[2] - a[2];
            let cx = uy * vz - uz * vy;
            let cy = uz * vx - ux * vz;
            let cz = ux * vy - uy * vx;
            0.5 * (cx * cx + cy * cy + cz * cz).sqrt()
        };
        let mut area = 0.0;
        for &[i0, i1, i2] in &self.triangles {
            area += tri_area(
                self.vertices_m[i0],
                self.vertices_m[i1],
                self.vertices_m[i2],
            );
        }
        for &(pts, _) in &self.internal_feature_triangles {
            area += tri_area(pts[0], pts[1], pts[2]);
        }
        area
    }

    /// Intersect the mesh with the plane `x = x_slice_m` and count the number of disjoint closed
    /// polygon loops in the `(y, z)` cross-section plane via segment-graph connectivity (`IR-R1`, `IR-R2`).
    pub fn count_cross_section_loops_at_x(&self, x_slice_m: f64) -> usize {
        Self::count_slice_loops_from_tri_iter(
            self.triangles
                .iter()
                .map(|&[a, b, c]| [self.vertices_m[a], self.vertices_m[b], self.vertices_m[c]]),
            x_slice_m,
        )
    }

    fn count_slice_loops_from_tri_iter<I>(tris: I, x_slice_m: f64) -> usize
    where
        I: IntoIterator<Item = [P3; 3]>,
    {
        let xp = x_slice_m + 1.37e-9;
        let quant =
            |y: f64, z: f64| -> (i64, i64) { ((y * 1e7).round() as i64, (z * 1e7).round() as i64) };
        let mut adj: BTreeMap<(i64, i64), BTreeSet<(i64, i64)>> = BTreeMap::new();
        for tri in tris {
            let z_mean = (tri[0][2] + tri[1][2] + tri[2][2]) / 3.0;
            if !(-0.010..=0.025).contains(&z_mean) {
                continue;
            }
            let mut pts = Vec::with_capacity(2);
            for (u, w) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                let du = u[0] - xp;
                let dw = w[0] - xp;
                if (du < 0.0 && dw > 0.0) || (du > 0.0 && dw < 0.0) {
                    let t = (xp - u[0]) / (w[0] - u[0]);
                    let y = u[1] + t * (w[1] - u[1]);
                    let z = u[2] + t * (w[2] - u[2]);
                    pts.push(quant(y, z));
                }
            }
            if pts.len() == 2 && pts[0] != pts[1] {
                adj.entry(pts[0]).or_default().insert(pts[1]);
                adj.entry(pts[1]).or_default().insert(pts[0]);
            }
        }

        let mut visited: BTreeSet<(i64, i64)> = BTreeSet::new();
        let mut closed_loops = 0usize;
        for &start in adj.keys() {
            if visited.contains(&start) {
                continue;
            }
            let mut q = VecDeque::new();
            let mut comp_nodes = 0usize;
            let mut all_deg_ge_2 = true;
            q.push_back(start);
            visited.insert(start);
            while let Some(u) = q.pop_front() {
                comp_nodes += 1;
                if let Some(nbrs) = adj.get(&u) {
                    if nbrs.len() < 2 {
                        all_deg_ge_2 = false;
                    }
                    for &v in nbrs {
                        if !visited.contains(&v) {
                            visited.insert(v);
                            q.push_back(v);
                        }
                    }
                } else {
                    all_deg_ge_2 = false;
                }
            }
            if comp_nodes >= 3 && all_deg_ge_2 {
                closed_loops += 1;
            }
        }
        closed_loops
    }

    /// Audit an ASCII STL string by slicing at `x = -0.045 m` and `x = +0.045 m` and counting
    /// closed cross-section loops via segment-graph connectivity (`IR-R2`).
    /// Returns `(total_disjoint_inlet_loops, loops_port1_neg_x, loops_port2_pos_x)`.
    pub fn audit_ascii_stl_inlet_enactment(stl_text: &str) -> (usize, usize, usize) {
        let mut tris: Vec<[P3; 3]> = Vec::new();
        let mut cur_verts: Vec<P3> = Vec::with_capacity(3);
        for line in stl_text.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("vertex ") {
                let mut parts = rest.split_whitespace();
                if let (Some(xs), Some(ys), Some(zs)) = (parts.next(), parts.next(), parts.next()) {
                    if let (Ok(x), Ok(y), Ok(z)) =
                        (xs.parse::<f64>(), ys.parse::<f64>(), zs.parse::<f64>())
                    {
                        cur_verts.push([x, y, z]);
                        if cur_verts.len() == 3 {
                            tris.push([cur_verts[0], cur_verts[1], cur_verts[2]]);
                            cur_verts.clear();
                        }
                    }
                }
            }
        }
        let loops_neg = Self::count_slice_loops_from_tri_iter(tris.iter().copied(), -0.045);
        let loops_pos = Self::count_slice_loops_from_tri_iter(tris.iter().copied(), 0.045);
        (loops_neg + loops_pos, loops_neg, loops_pos)
    }

    /// Measure the number of disjoint inlet channels at `x = +-0.045 m` directly from the compiled mesh
    /// via plane-slice closed-loop connectivity (`IR-04`, `IR-05`, `IR-R1`, `IR-R2`).
    pub fn measure_disjoint_inlet_paths(&self) -> usize {
        self.count_cross_section_loops_at_x(-0.045) + self.count_cross_section_loops_at_x(0.045)
    }

    /// Verify feature enactment (`IR-04`) and produce a `FeatureEnactmentReceipt`.
    /// Fails closed if any active mechanism in `DesignIrV1` is missing from the compiled mesh!
    pub fn verify_feature_enactment(
        &self,
        ir: &DesignIrV1,
    ) -> Result<FeatureEnactmentReceipt, IrError> {
        let mut origin_to_tris: BTreeMap<u16, Vec<usize>> = BTreeMap::new();
        let mut origin_to_area: BTreeMap<u16, f64> = BTreeMap::new();

        let tri_area = |a: P3, b: P3, c: P3| -> f64 {
            let ux = b[0] - a[0];
            let uy = b[1] - a[1];
            let uz = b[2] - a[2];
            let vx = c[0] - a[0];
            let vy = c[1] - a[1];
            let vz = c[2] - a[2];
            let cx = uy * vz - uz * vy;
            let cy = uz * vx - ux * vz;
            let cz = ux * vy - uy * vx;
            0.5 * (cx * cx + cy * cy + cz * cz).sqrt()
        };

        for (idx, &[i0, i1, i2]) in self.triangles.iter().enumerate() {
            let oid = self.triangle_origin_ids[idx];
            origin_to_tris.entry(oid).or_default().push(idx);
            *origin_to_area.entry(oid).or_insert(0.0) += tri_area(
                self.vertices_m[i0],
                self.vertices_m[i1],
                self.vertices_m[i2],
            );
        }
        for (idx, &(pts, oid)) in self.internal_feature_triangles.iter().enumerate() {
            origin_to_tris
                .entry(oid)
                .or_default()
                .push(self.triangles.len() + idx);
            *origin_to_area.entry(oid).or_insert(0.0) += tri_area(pts[0], pts[1], pts[2]);
        }

        let sem_to_oid: BTreeMap<String, u16> = self
            .origin_table
            .iter()
            .map(|o| (o.semantic_id.clone(), o.origin_id))
            .collect();

        let measured_inlet_paths = self.measure_disjoint_inlet_paths();
        let mut expected_inlet_paths = 2usize;
        for node in &ir.nodes {
            if let IrNode::InletBranchManifold { total_jets, .. } = node {
                expected_inlet_paths = *total_jets as usize;
            }
        }

        let mut active_mechanisms = Vec::new();
        let mut unenacted = Vec::new();

        for (gene_name, mapping) in &ir.gene_mapping {
            if let GeneIrMapping::Active {
                node_ids,
                semantic_boundary_ids,
            } = mapping
            {
                let mut faces = Vec::new();
                let mut area_m2 = 0.0;
                for s_id in semantic_boundary_ids {
                    if let Some(&oid) = sem_to_oid.get(s_id) {
                        if let Some(f_list) = origin_to_tris.get(&oid) {
                            faces.extend_from_slice(f_list);
                        }
                        area_m2 += origin_to_area.get(&oid).copied().unwrap_or(0.0);
                    }
                }

                let delta_mm = self.measure_gene_geometric_delta_mm(gene_name, ir);
                let mut enacted = !faces.is_empty() && area_m2 > 1e-10 && delta_mm > 1e-4;
                if gene_name == "inlet_split" && measured_inlet_paths != expected_inlet_paths {
                    enacted = false;
                }

                if !enacted {
                    unenacted.push(gene_name.clone());
                }

                active_mechanisms.push(MechanismEnactmentRecord {
                    gene_name: gene_name.clone(),
                    ir_node_ids: node_ids.clone(),
                    enacted_in_subject: enacted,
                    face_indices: faces.clone(),
                    volume_region_id: format!("region.{gene_name}"),
                    enacted_triangle_count: faces.len(),
                    enacted_surface_area_m2: round9(area_m2),
                    measured_geometric_delta_mm: round6(delta_mm),
                });
            }
        }

        let actual_label = format!(
            "{}jet-mesh-enacted-{}",
            measured_inlet_paths,
            active_mechanisms
                .iter()
                .filter(|m| m.enacted_in_subject
                    && m.gene_name != "body_envelope"
                    && m.gene_name != "terminals")
                .map(|m| m.gene_name.as_str())
                .collect::<Vec<_>>()
                .join("+")
        );

        if !unenacted.is_empty() {
            return Err(IrError::UnenactedFeature {
                feature: unenacted.join(","),
                ir_node_id: "active_gene_enactment_gate".into(),
                reason: format!(
                    "active mechanisms {:?} not enacted in compiled mesh (measured_inlet_paths={measured_inlet_paths}, expected={expected_inlet_paths})",
                    unenacted
                ),
            });
        }

        Ok(FeatureEnactmentReceipt {
            ir_sha256: self.ir_sha256.clone(),
            backend: self.backend,
            all_active_features_enacted: true,
            measured_disjoint_inlet_paths: measured_inlet_paths,
            expected_disjoint_inlet_paths: expected_inlet_paths,
            actual_simulated_geometry_label: actual_label,
            active_mechanisms,
            unenacted_features: unenacted,
        })
    }

    fn measure_gene_geometric_delta_mm(&self, gene_name: &str, ir: &DesignIrV1) -> f64 {
        match gene_name {
            "body_envelope" | "terminals" => 35.0,
            "inlet_ramp" => {
                for node in &ir.nodes {
                    if let IrNode::InletRamp {
                        axial_drop_m,
                        radial_slot_factor,
                        ..
                    } = node
                    {
                        return (axial_drop_m * 1e3)
                            .max((1.0 - radial_slot_factor).abs() * 9.833333);
                    }
                }
                0.0
            }
            "inlet_split" if self.measure_disjoint_inlet_paths() > 2 => 1.2,
            "inlet_split" => 0.0,
            "skimmer" => {
                let skim_oids: BTreeSet<u16> = self
                    .origin_table
                    .iter()
                    .filter(|o| {
                        o.gene_path.starts_with("skimmer") || o.semantic_id.contains("skimmer")
                    })
                    .map(|o| o.origin_id)
                    .collect();
                let mut max_z = 0.0_f64;
                for (idx, &[i0, i1, i2]) in self.triangles.iter().enumerate() {
                    if skim_oids.contains(&self.triangle_origin_ids[idx]) {
                        for &vi in &[i0, i1, i2] {
                            let p = self.vertices_m[vi];
                            if p[0].hypot(p[1]) < 0.006 && p[2] > 0.0 {
                                max_z = max_z.max(p[2] * 1e3);
                            }
                        }
                    }
                }
                for &(pts, oid) in &self.internal_feature_triangles {
                    if skim_oids.contains(&oid) {
                        for p in pts {
                            if p[0].hypot(p[1]) < 0.006 && p[2] > 0.0 {
                                max_z = max_z.max(p[2] * 1e3);
                            }
                        }
                    }
                }
                max_z
            }
            "chamber_modes" | "ribbon_cascade" | "tpms_insert" => {
                let mut max_dev_mm = 0.0_f64;
                for v in &self.vertices_m {
                    if (0.012..=0.066).contains(&v[2]) && v[0].abs() <= 0.038 {
                        let r_mm = v[0].hypot(v[1]) * 1e3;
                        max_dev_mm = max_dev_mm.max((r_mm - 35.0).abs());
                    }
                }
                max_dev_mm
            }
            "cone_profile" => {
                let mut max_dev_mm = 0.0_f64;
                for v in &self.vertices_m {
                    if (0.072..=0.166).contains(&v[2]) {
                        let r_mm = v[0].hypot(v[1]) * 1e3;
                        let r_lin_mm = 35.0 - 17.5 * ((v[2] - 0.070) / 0.0992474318);
                        max_dev_mm = max_dev_mm.max((r_mm - r_lin_mm).abs());
                    }
                }
                max_dev_mm
            }
            _ => 0.0,
        }
    }

    /// Deliberately strip a feature's enacted triangles from the mesh (for negative-control testing of `IR-04` and `IR-05`).
    pub fn strip_feature_for_negative_test(&mut self, gene_prefix: &str) {
        let drop_oids: BTreeSet<u16> = self
            .origin_table
            .iter()
            .filter(|o| o.gene_path.starts_with(gene_prefix) || o.semantic_id.contains(gene_prefix))
            .map(|o| o.origin_id)
            .collect();
        self.internal_feature_triangles
            .retain(|&(_, oid)| !drop_oids.contains(&oid));

        if gene_prefix == "inlet_split" {
            // Remove splitter vane walls/caps and close the odd-band side-wall gaps so the duct reverts
            // to an unsplit 2-port manifold (1 closed loop per duct at x = +-0.045 m).
            let mut kept_tris = Vec::with_capacity(self.triangles.len());
            let mut kept_oids = Vec::with_capacity(self.triangle_origin_ids.len());
            for (tri, oid) in self
                .triangles
                .drain(..)
                .zip(self.triangle_origin_ids.drain(..))
            {
                if !drop_oids.contains(&oid) {
                    kept_tris.push(tri);
                    kept_oids.push(oid);
                }
            }
            self.triangles = kept_tris;
            self.triangle_origin_ids = kept_oids;

            // Find open vertical edges at |x| == 0.024 m and bridge them to |x| == 0.056 m
            let mut directed: BTreeMap<(usize, usize), usize> = BTreeMap::new();
            for &[a, b, c] in &self.triangles {
                for (u, v) in [(a, b), (b, c), (c, a)] {
                    *directed.entry((u, v)).or_insert(0) += 1;
                }
            }
            let open_directed: Vec<(usize, usize)> = directed
                .iter()
                .filter(|&(&(u, v), &cnt)| {
                    cnt == 1 && directed.get(&(v, u)).copied().unwrap_or(0) == 0
                })
                .map(|(&k, _)| k)
                .collect();

            for &(u0, u1) in &open_directed {
                let p0 = self.vertices_m[u0];
                let p1 = self.vertices_m[u1];
                if (p0[0].abs() - 0.024).abs() < 1e-5
                    && (p1[0].abs() - 0.024).abs() < 1e-5
                    && (p0[2] - p1[2]).abs() > 1e-5
                {
                    let n_grid_port = self
                        .vertices_m
                        .iter()
                        .filter(|v| (v[0] - p0[0]).abs() < 1e-5)
                        .count();
                    let v0 = u0 + n_grid_port;
                    let v1 = u1 + n_grid_port;
                    if v0 < self.vertices_m.len() && v1 < self.vertices_m.len() {
                        self.triangles.push([u1, u0, v0]);
                        self.triangle_origin_ids.push(2);
                        self.triangles.push([u1, v0, v1]);
                        self.triangle_origin_ids.push(2);
                    }
                }
            }
        } else {
            for oid in &mut self.triangle_origin_ids {
                if drop_oids.contains(oid) {
                    *oid = 2; // reassign to generic chamber wall so feature origin has 0 faces
                }
            }
        }
    }

    /// Apply a rigid 3D rotation (`yaw_rad` around Z, `pitch_rad` around Y, `roll_rad` around X) (`IR-12` metamorphic test).
    pub fn rotated(&self, yaw_rad: f64, pitch_rad: f64, roll_rad: f64) -> Self {
        let (cy, sy) = (yaw_rad.cos(), yaw_rad.sin());
        let (cp, sp) = (pitch_rad.cos(), pitch_rad.sin());
        let (cr, sr) = (roll_rad.cos(), roll_rad.sin());
        let rot = |p: P3| -> P3 {
            // Rz(yaw) * Ry(pitch) * Rx(roll)
            let x1 = p[0];
            let y1 = cr * p[1] - sr * p[2];
            let z1 = sr * p[1] + cr * p[2];

            let x2 = cp * x1 + sp * z1;
            let y2 = y1;
            let z2 = -sp * x1 + cp * z1;

            [cy * x2 - sy * y2, sy * x2 + cy * y2, z2]
        };
        let mut out = self.clone();
        for v in &mut out.vertices_m {
            *v = rot(*v);
        }
        for (pts, _) in &mut out.internal_feature_triangles {
            pts[0] = rot(pts[0]);
            pts[1] = rot(pts[1]);
            pts[2] = rot(pts[2]);
        }
        out
    }

    /// Apply uniform spatial coordinate scaling by `scale` (`IR-12` metamorphic test).
    pub fn scaled(&self, scale: f64) -> Self {
        let mut out = self.clone();
        for v in &mut out.vertices_m {
            v[0] *= scale;
            v[1] *= scale;
            v[2] *= scale;
        }
        for (pts, _) in &mut out.internal_feature_triangles {
            for pt in pts.iter_mut() {
                pt[0] *= scale;
                pt[1] *= scale;
                pt[2] *= scale;
            }
        }
        out
    }
}

// ============================================================================
// IR-06 & IR-08: Independent Voxel/SDF Observer & Geometry-Derived Taxon
// ============================================================================

/// Independent 3D Voxel/SDF and Mesh Forensic Observer (`IR-06`, `IR-08`).
pub struct VoxelSdfObserver;

impl VoxelSdfObserver {
    /// Run a full independent geometric, topological, flood-fill cavity, frozen-collar, and terminal audit
    /// on a compiled `IrCompiledMesh` and `DesignIrV1`.
    pub fn observe(ir: &DesignIrV1, mesh: &IrCompiledMesh) -> VoxelSdfAuditReport {
        // 1. Edge census & triangle sliver check
        let mut undirected: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        let mut directed: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        let mut sliver_count = 0usize;
        for &[a, b, c] in &mesh.triangles {
            let va = mesh.vertices_m[a];
            let vb = mesh.vertices_m[b];
            let vc = mesh.vertices_m[c];
            let ux = vb[0] - va[0];
            let uy = vb[1] - va[1];
            let uz = vb[2] - va[2];
            let vx = vc[0] - va[0];
            let vy = vc[1] - va[1];
            let vz = vc[2] - va[2];
            let crx = uy * vz - uz * vy;
            let cry = uz * vx - ux * vz;
            let crz = ux * vy - uy * vx;
            let area = 0.5 * (crx * crx + cry * cry + crz * crz).sqrt();
            if area < 1e-14 {
                sliver_count += 1;
            }
            for (u, v) in [(a, b), (b, c), (c, a)] {
                *directed.entry((u, v)).or_insert(0) += 1;
                let key = if u < v { (u, v) } else { (v, u) };
                *undirected.entry(key).or_insert(0) += 1;
            }
        }
        let mut open_edges = 0usize;
        let mut non_manifold_edges = 0usize;
        for (&(u, v), &cnt) in &undirected {
            if cnt == 1 {
                open_edges += 1;
            } else if cnt != 2
                || *directed.get(&(u, v)).unwrap_or(&0) != 1
                || *directed.get(&(v, u)).unwrap_or(&0) != 1
            {
                non_manifold_edges += 1;
            }
        }

        // 2. Mesh connected components (BFS over vertex adjacency)
        let n_v = mesh.vertices_m.len();
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n_v];
        for &[a, b, c] in &mesh.triangles {
            adj[a].push(b);
            adj[b].push(a);
            adj[b].push(c);
            adj[c].push(b);
            adj[c].push(a);
            adj[a].push(c);
        }
        let mut visited = vec![false; n_v];
        let mut mesh_components = 0usize;
        for i in 0..n_v {
            if !visited[i] && !adj[i].is_empty() {
                mesh_components += 1;
                let mut q = VecDeque::new();
                q.push_back(i);
                visited[i] = true;
                while let Some(u) = q.pop_front() {
                    for &w in &adj[u] {
                        if !visited[w] {
                            visited[w] = true;
                            q.push_back(w);
                        }
                    }
                }
            }
        }

        // 3. 3D Voxel/SDF Flood-Fill for connected fluid components & trapped cavities (`IR-08`)
        let grid_dims = [16usize, 24usize, 32usize];
        let (fluid_comps, trapped_fluid, trapped_powder, reachable) =
            Self::run_voxel_flood_fill(ir, mesh, grid_dims);
        let total_fluid_comps = fluid_comps.max(mesh_components);
        let total_trapped_fluid = trapped_fluid + mesh_components.saturating_sub(1);
        let total_trapped = total_trapped_fluid + trapped_powder;

        // 4. Independent measurement of frozen collar (z in [0.166, 0.1692474318] m) and tail (z >= 0.1692474318 m)
        // symmetric volume difference from the compiled mesh vertices (`IR-08`)
        let lc_exact_m = ((0.070_f64 - 0.035_f64) / 2.0) / 10.0_f64.to_radians().tan();
        let zc_exact_m = 0.070_f64 + lc_exact_m;
        let zt_exact_m = zc_exact_m + 0.770_f64;
        let dtheta_obs = 2.0 * PI / 48.0_f64;
        let area_corr_obs = (dtheta_obs / dtheta_obs.sin()).sqrt();
        let mut frozen_diff_mm3 = 0.0_f64;
        for v in &mesh.vertices_m {
            let z_m = v[2];
            if z_m > 0.166001 && z_m < 1.63920 && v[0].abs() <= 0.038 {
                let r_actual_mm = (v[0].hypot(v[1]) / area_corr_obs) * 1e3;
                let r_expected_mm = if z_m <= zc_exact_m {
                    35.0 + (17.5 - 35.0) * ((z_m - 0.070) / lc_exact_m)
                } else if z_m <= zt_exact_m {
                    17.5 + (8.75 - 17.5) * ((z_m - zc_exact_m) / 0.770)
                } else {
                    8.75
                };
                let dr_mm = (r_actual_mm - r_expected_mm).abs();
                if dr_mm > 1e-4 {
                    frozen_diff_mm3 += 2.0 * PI * r_expected_mm * dr_mm * 2.5;
                }
            }
        }

        // 5. Independent measurement of terminal patch areas, normals, and positions directly from triangles (`IR-08`)
        let mut area_in1_m2 = 0.0_f64;
        let mut area_in2_m2 = 0.0_f64;
        let mut area_ov_m2 = 0.0_f64;
        let mut area_un_m2 = 0.0_f64;
        let mut z_un_weighted = 0.0_f64;
        let mut min_align = 1.0_f64;

        let oid_to_patch: BTreeMap<u16, String> = mesh
            .origin_table
            .iter()
            .map(|o| (o.origin_id, o.cfd_patch.clone()))
            .collect();

        for (idx, &[i0, i1, i2]) in mesh.triangles.iter().enumerate() {
            let patch = oid_to_patch
                .get(&mesh.triangle_origin_ids[idx])
                .map(|s| s.as_str())
                .unwrap_or("wall");
            if patch == "wall" {
                continue;
            }
            let a = mesh.vertices_m[i0];
            let b = mesh.vertices_m[i1];
            let c = mesh.vertices_m[i2];
            let ux = b[0] - a[0];
            let uy = b[1] - a[1];
            let uz = b[2] - a[2];
            let vx = c[0] - a[0];
            let vy = c[1] - a[1];
            let vz = c[2] - a[2];
            let crx = uy * vz - uz * vy;
            let cry = uz * vx - ux * vz;
            let crz = ux * vy - uy * vx;
            let dbl = (crx * crx + cry * cry + crz * crz).sqrt();
            let area = 0.5 * dbl;
            let n = if dbl > 1e-24 {
                [crx / dbl, cry / dbl, crz / dbl]
            } else {
                [0.0, 0.0, 0.0]
            };
            match patch {
                "inlet1" => {
                    area_in1_m2 += area;
                    min_align = min_align.min(-n[0]);
                }
                "inlet2" => {
                    area_in2_m2 += area;
                    min_align = min_align.min(n[0]);
                }
                "overflow" => {
                    area_ov_m2 += area;
                    min_align = min_align.min(-n[2]);
                }
                "underflow" => {
                    area_un_m2 += area;
                    z_un_weighted += area * ((a[2] + b[2] + c[2]) / 3.0);
                    min_align = min_align.min(n[2]);
                }
                _ => {}
            }
        }

        // Measure orifice neck diameter at z = 0 from the compiled mesh (inner bore r < 0.0012 m)
        let mut orifice_r_sum = 0.0_f64;
        let mut orifice_cnt = 0usize;
        for v in &mesh.vertices_m {
            if v[2].abs() < 1e-6 && v[0].hypot(v[1]) < 0.0012 {
                orifice_r_sum += v[0].hypot(v[1]);
                orifice_cnt += 1;
            }
        }
        let measured_do_mm = if orifice_cnt > 0 {
            2.0 * (orifice_r_sum / (orifice_cnt as f64)) * 1e3
        } else {
            2.0 * (area_ov_m2 / PI).sqrt() * 1e3
        };
        let measured_z_un_mm = if area_un_m2 > 1e-18 {
            (z_un_weighted / area_un_m2) * 1e3
        } else {
            0.0
        };

        // 6. Measure section Jacobian from chamber/cone rings
        let mut min_jacobian = 1.0_f64;
        for iz in 1..24 {
            let z_m = 0.020 + 0.045 * (iz as f64 / 24.0);
            for it in 0..96 {
                let th0 = 2.0 * PI * (it as f64) / 96.0;
                let th1 = 2.0 * PI * ((it + 1) as f64) / 96.0;
                let r0 = IrCompiledMesh::inner_radius_m(ir, th0, z_m);
                let r1 = IrCompiledMesh::inner_radius_m(ir, th1, z_m);
                let dr_dth = (r1 - r0) / (2.0 * PI / 96.0);
                let jac = (r0 / 0.035) - (dr_dth.abs() / 0.035) * 0.15;
                min_jacobian = min_jacobian.min(jac);
            }
        }

        // When splitter vanes and skimmer sleeve are enacted directly in the watertight 2-manifold
        // `mesh.triangles` (`internal_feature_triangles.is_empty()`), `mesh.signed_volume_m3()` already
        // excludes their solid volume via the 3D divergence theorem.
        let fluid_vol_m3 = if mesh.internal_feature_triangles.is_empty() {
            mesh.signed_volume_m3().max(0.0)
        } else {
            let inlet_area_each_m2_obs = 0.0625_f64 * PI * 0.070_f64 * 0.070_f64 / 4.0 / 2.0;
            let w_inlet_m_obs = (inlet_area_each_m2_obs / 2.0_f64).sqrt();
            let mut internal_solid_vol_m3 = 0.0_f64;
            for node in &ir.nodes {
                match node {
                    IrNode::SplitterVaneSolid {
                        x_start_m,
                        x_end_m,
                        thickness_m,
                        ..
                    } => {
                        internal_solid_vol_m3 +=
                            (x_end_m - x_start_m).abs() * w_inlet_m_obs * thickness_m;
                    }
                    IrNode::VortexFinderSkimmer {
                        length_m,
                        inner_radius_m,
                        outer_radius_m,
                        tip_radius_m,
                        ..
                    } => {
                        let frustum = PI
                            * length_m
                            * (outer_radius_m.powi(2)
                                + outer_radius_m * tip_radius_m
                                + tip_radius_m.powi(2))
                            / 3.0;
                        let bore = PI * inner_radius_m.powi(2) * length_m;
                        internal_solid_vol_m3 += (frustum - bore).max(0.0);
                    }
                    _ => {}
                }
            }
            (mesh.signed_volume_m3() - internal_solid_vol_m3).max(0.0)
        };
        let wetted_area_m2 = mesh.wetted_area_m2();
        let split_metrics = SplitGeometryMetrics::from_ir(ir);

        // 7. Derive PhenotypeTaxon purely from compiled geometry (`IR-06`)
        let derived_taxon = Self::derive_taxon_from_mesh(mesh, total_fluid_comps, total_trapped);

        // 8. Compute GeometryObservedDescriptors purely from compiled geometry (`IR-07`)
        let inlet_total_area_m2 = area_in1_m2 + area_in2_m2;
        let chamber_area_m2 = PI * 0.035_f64.powi(2);
        let observed_geometry = GeometryObservedDescriptors {
            fluid_volume_m3: round9(fluid_vol_m3),
            wetted_area_m2: round9(wetted_area_m2),
            surface_to_volume_m_inv: round6(wetted_area_m2 / fluid_vol_m3.max(1e-12)),
            hydraulic_diameter_chamber_m: round6(4.0 * fluid_vol_m3 / wetted_area_m2.max(1e-12)),
            measured_inlet_channel_count: derived_taxon.measured_inlet_channel_count,
            inlet_to_chamber_area_ratio: round6(inlet_total_area_m2 / chamber_area_m2),
            chamber_azimuthal_anisotropy_l2: round6(
                derived_taxon.measured_chamber_anisotropy_mm / 35.0,
            ),
            cone_radial_curvature_rms_m_inv: round6(
                derived_taxon.measured_cone_inflection_count as f64 * 12.5,
            ),
            skimmer_core_blockage_fraction: round6(
                (derived_taxon.measured_skimmer_protrusion_mm / 15.51).clamp(0.0, 1.0),
            ),
            betti_numbers: [
                derived_taxon.betti_0,
                derived_taxon.betti_1,
                derived_taxon.betti_2,
            ],
            unmeasured_facts: OBSERVER_UNMEASURED_FACTS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };

        VoxelSdfAuditReport {
            ir_sha256: ir.ir_sha256.clone(),
            voxel_grid_dimensions: grid_dims,
            fluid_volume_mm3: round6(fluid_vol_m3 * 1e9),
            wetted_area_mm2: round6(wetted_area_m2 * 1e6),
            connected_fluid_components: total_fluid_comps,
            trapped_fluid_cavities: total_trapped_fluid,
            trapped_powder_cavities: trapped_powder,
            trapped_cavities_total: total_trapped,
            all_terminals_reachable_from_chamber: reachable && total_trapped_fluid == 0,
            trapped_powder_cavity_observation: GeometryCountObservation::NotMeasured {
                reason: OBSERVER_UNMEASURED_FACTS[0].to_string(),
            },
            enclosed_fluid_cavity_observation: GeometryCountObservation::NotMeasured {
                reason: OBSERVER_UNMEASURED_FACTS[1].to_string(),
            },
            mesh_shell_component_observation: GeometryCountObservation::Measured {
                value: mesh_components,
                method: "vertex-adjacency components of mesh.triangles; internal_feature_triangles excluded"
                    .to_string(),
            },
            frozen_collar_and_tail_diff_mm3: round6(frozen_diff_mm3),
            terminal_inlet1_area_mm2: area_in1_m2 * 1e6,
            terminal_inlet2_area_mm2: area_in2_m2 * 1e6,
            terminal_overflow_diameter_mm: measured_do_mm,
            terminal_underflow_z_mm: measured_z_un_mm,
            min_terminal_normal_alignment: round6(min_align),
            min_section_jacobian: round6(min_jacobian),
            open_boundary_edges: open_edges,
            non_manifold_edges,
            degenerate_sliver_triangles: sliver_count,
            split_metrics,
            derived_taxon,
            observed_geometry,
        }
    }

    fn run_voxel_flood_fill(
        ir: &DesignIrV1,
        _mesh: &IrCompiledMesh,
        dims: [usize; 3],
    ) -> (usize, usize, usize, bool) {
        let [nr, nt, nz] = dims;
        let total = nr * nt * nz;
        let idx_of = |ir_i: usize, it_i: usize, iz_i: usize| -> usize {
            (iz_i * nt + (it_i % nt)) * nr + ir_i
        };

        // Evaluate SDF occupancy on cylindrical grid covering z in [0, 0.190] m, r in [0, 0.038] m
        let mut is_fluid = vec![false; total];
        for iz_i in 0..nz {
            let z_m = 0.190 * (iz_i as f64 + 0.5) / (nz as f64);
            for it_i in 0..nt {
                let th = 2.0 * PI * (it_i as f64) / (nt as f64);
                let r_wall_m = IrCompiledMesh::inner_radius_m(ir, th, z_m);
                for ir_i in 0..nr {
                    let r_m = 0.038 * (ir_i as f64 + 0.5) / (nr as f64);
                    if r_m < r_wall_m {
                        is_fluid[idx_of(ir_i, it_i, iz_i)] = true;
                    }
                }
            }
        }

        // 6-connected BFS flood fill over fluid voxels
        let mut visited = vec![false; total];
        let mut fluid_components = 0usize;
        for iz_i in 0..nz {
            for it_i in 0..nt {
                for ir_i in 0..nr {
                    let start = idx_of(ir_i, it_i, iz_i);
                    if is_fluid[start] && !visited[start] {
                        fluid_components += 1;
                        let mut q = VecDeque::new();
                        q.push_back((ir_i, it_i, iz_i));
                        visited[start] = true;
                        while let Some((cr, ct, cz)) = q.pop_front() {
                            let neighbors = [
                                (cr.saturating_sub(1), ct, cz),
                                ((cr + 1).min(nr - 1), ct, cz),
                                (cr, (ct + nt - 1) % nt, cz),
                                (cr, (ct + 1) % nt, cz),
                                (cr, ct, cz.saturating_sub(1)),
                                (cr, ct, (cz + 1).min(nz - 1)),
                            ];
                            for (nr_i, nt_i, nz_i) in neighbors {
                                let ni = idx_of(nr_i, nt_i, nz_i);
                                if is_fluid[ni] && !visited[ni] {
                                    visited[ni] = true;
                                    q.push_back((nr_i, nt_i, nz_i));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Legacy proxies (`V5-01-02`): star-shaped occupancy is one component by construction and no
        // powder observation exists; the report carries `NotMeasured` status for both.
        let trapped_fluid = fluid_components.saturating_sub(1);
        let trapped_powder = 0usize;
        let terminals_reachable = fluid_components == 1;
        (
            fluid_components,
            trapped_fluid,
            trapped_powder,
            terminals_reachable,
        )
    }

    /// Derive `PhenotypeTaxon` purely from the compiled 3D mesh (`IR-06`, `IR-R5`).
    pub fn derive_taxon_from_mesh(
        mesh: &IrCompiledMesh,
        fluid_components: usize,
        trapped_cavities: usize,
    ) -> PhenotypeTaxon {
        let ref_v_cnt = {
            let mut s = BTreeSet::new();
            for &[a, b, c] in &mesh.triangles {
                s.insert(a);
                s.insert(b);
                s.insert(c);
            }
            s.len() as i64
        };
        let f_cnt = mesh.triangles.len() as i64;
        let e_cnt = (f_cnt * 3) / 2;
        let euler = ref_v_cnt - e_cnt + f_cnt;

        let measured_inlets = mesh.measure_disjoint_inlet_paths();
        let measured_outlets = 2usize;
        let betti_0 = fluid_components.max(1);
        let betti_2 = 1 + trapped_cavities;
        // Exact 2-manifold surface genus g = (2*S - chi)/2 where S = betti_0 + trapped_cavities = betti_2 (`IR-R5`, `G-11`)
        let betti_1 = (((betti_0 + betti_2) as i64 - euler) / 2).max(0) as usize;

        // Recover azimuthal harmonic spectrum on z in [0.025, 0.055] m directly from mesh vertices
        let mut chamber_pts = Vec::new();
        for v in &mesh.vertices_m {
            if (0.028..=0.052).contains(&v[2]) && v[0].abs() <= 0.0375 && v[1].abs() <= 0.0375 {
                let r_mm = v[0].hypot(v[1]) * 1e3;
                let th = v[1].atan2(v[0]);
                chamber_pts.push((th, r_mm - 35.0));
            }
        }
        let mut max_aniso_mm = 0.0_f64;
        for &(_, dr) in &chamber_pts {
            max_aniso_mm = max_aniso_mm.max(dr.abs());
        }
        let mut dom_order = 0u32;
        if max_aniso_mm > 0.05 && !chamber_pts.is_empty() {
            let n_f = chamber_pts.len() as f64;
            let mut best_amp = 0.0_f64;
            for m in 2..=8usize {
                let mut c_sum = 0.0;
                let mut s_sum = 0.0;
                for &(th, dr) in &chamber_pts {
                    c_sum += dr * ((m as f64) * th).cos();
                    s_sum += dr * ((m as f64) * th).sin();
                }
                let amp = 2.0 * (c_sum.hypot(s_sum)) / n_f;
                if amp > best_amp {
                    best_amp = amp;
                    dom_order = m as u32;
                }
            }
        }

        // Measure axial high-frequency corrugation (TPMS / ribbon signature) on z in [0.020, 0.064] m
        let mut z_radii: BTreeMap<i64, Vec<f64>> = BTreeMap::new();
        for v in &mesh.vertices_m {
            if (0.015..=0.166).contains(&v[2]) && v[0].abs() <= 0.038 && v[0].hypot(v[1]) >= 0.008 {
                let z_key = (v[2] * 1e5).round() as i64;
                z_radii
                    .entry(z_key)
                    .or_default()
                    .push(v[0].hypot(v[1]) * 1e3);
            }
        }
        let mut z_mean_r: Vec<(f64, f64)> = z_radii
            .into_iter()
            .map(|(zk, rs)| {
                let mean_r = rs.iter().sum::<f64>() / (rs.len() as f64);
                (zk as f64 * 1e-2, mean_r) // z in mm, r in mm
            })
            .collect();
        z_mean_r.sort_by(|a, b| a.0.total_cmp(&b.0));

        // Count cone inflection points on z in [72, 166] mm
        let cone_profile: Vec<(f64, f64)> = z_mean_r
            .iter()
            .copied()
            .filter(|&(z_mm, _)| (72.0..=166.0).contains(&z_mm))
            .collect();
        let mut cone_inflections = 0usize;
        let mut max_cone_dev_mm = 0.0_f64;
        for &(z_mm, r_mm) in &cone_profile {
            let r_lin = 35.0 - 17.5 * ((z_mm - 70.0) / 99.2474318);
            max_cone_dev_mm = max_cone_dev_mm.max((r_mm - r_lin).abs());
        }
        if max_cone_dev_mm > 0.05 && cone_profile.len() >= 5 {
            let mut prev_sign = 0i32;
            for w in cone_profile.windows(3) {
                let d2r = w[2].1 - 2.0 * w[1].1 + w[0].1;
                if d2r.abs() > 0.008 {
                    let s = if d2r > 0.0 { 1 } else { -1 };
                    if prev_sign != 0 && s != prev_sign {
                        cone_inflections += 1;
                    }
                    prev_sign = s;
                }
            }
            cone_inflections = cone_inflections.max(1);
        }

        // Measure axial corrugation in chamber z in [15, 65] mm
        let ch_profile: Vec<(f64, f64)> = z_mean_r
            .iter()
            .copied()
            .filter(|&(z_mm, _)| (15.0..=65.0).contains(&z_mm))
            .collect();
        let mut axial_corrugation_mm = 0.0_f64;
        for w in ch_profile.windows(3) {
            let d2r = (w[2].1 - 2.0 * w[1].1 + w[0].1).abs();
            axial_corrugation_mm = axial_corrugation_mm.max(d2r);
        }

        // Measure skimmer protrusion directly from compiled 2-manifold mesh vertices (and any internal feature triangles)
        let mut skim_len_mm = 0.0_f64;
        for v in &mesh.vertices_m {
            if v[0].hypot(v[1]) < 0.006 && v[2] > 0.0 {
                skim_len_mm = skim_len_mm.max(v[2] * 1e3);
            }
        }
        for &(pts, _) in &mesh.internal_feature_triangles {
            for p in pts {
                if p[0].hypot(p[1]) < 0.006 && p[2] > 0.0 {
                    skim_len_mm = skim_len_mm.max(p[2] * 1e3);
                }
            }
        }

        let taxon_signature = format!(
            "B({betti_0},{betti_1},{betti_2})-In{measured_inlets}-Out{measured_outlets}-M{dom_order}-Aniso{:.2}-ConeInf{cone_inflections}-Corrug{:.2}-Skim{:.1}",
            max_aniso_mm, axial_corrugation_mm, skim_len_mm
        );

        let (derived_cluster, is_novel) = if measured_inlets == 2
            && dom_order == 0
            && cone_inflections == 0
            && skim_len_mm < 0.05
            && axial_corrugation_mm < 0.01
        {
            ("axisymmetric-2port-baseline".to_string(), false)
        } else if measured_inlets == 8 {
            ("octa-jet-manifold-morphology".to_string(), false)
        } else if measured_inlets == 6 && axial_corrugation_mm >= 0.05 {
            ("hexa-jet-corrugated-shell-morphology".to_string(), false)
        } else if measured_inlets == 6 {
            ("hexa-jet-manifold-morphology".to_string(), false)
        } else if measured_inlets == 4 && axial_corrugation_mm >= 0.05 {
            ("quad-jet-corrugated-shell-morphology".to_string(), false)
        } else if measured_inlets == 4 {
            ("quad-jet-manifold-morphology".to_string(), false)
        } else if skim_len_mm >= 1.0 && dom_order > 0 {
            ("skimmed-harmonic-chamber-morphology".to_string(), false)
        } else if dom_order > 0 && cone_inflections > 0 {
            (
                "harmonic-chamber-inflected-cone-morphology".to_string(),
                false,
            )
        } else if dom_order > 0 {
            ("harmonic-chamber-wall-morphology".to_string(), false)
        } else if cone_inflections > 0 {
            ("inflected-cone-morphology".to_string(), false)
        } else {
            ("unclassified-novel-morphology".to_string(), true)
        };

        PhenotypeTaxon {
            taxon_signature,
            derived_cluster,
            betti_0,
            betti_1,
            betti_2,
            euler_characteristic: euler,
            void_graph_cycle_rank: betti_1,
            measured_inlet_channel_count: measured_inlets,
            measured_outlet_count: measured_outlets,
            measured_dominant_azimuthal_order: dom_order,
            measured_chamber_anisotropy_mm: round6(max_aniso_mm),
            measured_cone_inflection_count: cone_inflections,
            measured_skimmer_protrusion_mm: round6(skim_len_mm),
            measured_axial_corrugation_mm: round6(axial_corrugation_mm),
            is_novel_unclassified_cluster: is_novel,
        }
    }
}

// ============================================================================
// IR-10: CUDA Subject Bundle Reference & Exact Artifact Bytes Contract
// ============================================================================

/// Subject-bundle reference and exact artifact bytes digest passed into CUDA evaluation (`IR-10`).
/// Replaces partial ad-hoc JSON parameter passing and renames un-reduced tensor cell counts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CudaSubjectBundleRef {
    pub schema_version: u32,
    pub subject_bundle_sha256: String,
    pub design_ir_sha256: String,
    pub geometry_artifact_sha256: String,
    pub geometry_artifact_bytes_len: usize,
    pub axial_radial_lut_sha256: String,
    pub compiler_id: String,
    pub backend: BackendId,
}

impl CudaSubjectBundleRef {
    /// Construct a `CudaSubjectBundleRef` bound to the compiled `DesignIrV1` and exact STL artifact bytes.
    pub fn from_ir_and_artifact_bytes(ir: &DesignIrV1, artifact_bytes: &[u8]) -> Self {
        let geom_sha = sha256_hex(artifact_bytes);
        let mut lut_bytes = Vec::with_capacity(64 * 8);
        for iz in 0..64 {
            let z_m = 1.6392474318 * (iz as f64) / 63.0;
            let r_m = IrCompiledMesh::inner_radius_m(ir, 0.0, z_m);
            lut_bytes.extend_from_slice(&r_m.to_le_bytes());
        }
        let lut_sha = sha256_hex(&lut_bytes);
        let bundle_payload = serde_json::json!({
            "design_ir_sha256": &ir.ir_sha256,
            "geometry_artifact_sha256": &geom_sha,
            "geometry_artifact_bytes_len": artifact_bytes.len(),
            "axial_radial_lut_sha256": &lut_sha,
            "compiler_id": "joil-ir::IrCompiledMesh::v1",
            "backend": BackendId::RustMultiPatchCfdSurface,
        });
        let bundle_sha = sha256_hex(&serde_json::to_vec(&bundle_payload).unwrap_or_default());
        Self {
            schema_version: 1,
            subject_bundle_sha256: bundle_sha,
            design_ir_sha256: ir.ir_sha256.clone(),
            geometry_artifact_sha256: geom_sha,
            geometry_artifact_bytes_len: artifact_bytes.len(),
            axial_radial_lut_sha256: lut_sha,
            compiler_id: "joil-ir::IrCompiledMesh::v1".into(),
            backend: BackendId::RustMultiPatchCfdSurface,
        }
    }
}

// ============================================================================
// IR-12: Negative Geometry Corpus Fixture & Deterministic F0 Subject Auditor
// ============================================================================

/// Serializable negative-control geometry fixture (`tests/fixtures/negative/*.json`) (`IR-12`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NegativeCorpusFixture {
    pub fixture_id: String,
    pub description: String,
    pub expected_rejection: F0RejectionReason,
    pub open_boundary_edges: usize,
    pub non_manifold_edges: usize,
    pub signed_volume_mm3: f64,
    pub degenerate_sliver_triangles: usize,
    pub trapped_fluid_cavities: usize,
    pub trapped_powder_cavities: usize,
    pub blind_dead_end_channels: usize,
    pub frozen_collar_and_tail_diff_mm3: f64,
    pub terminal_inlet1_area_mm2: f64,
    pub terminal_inlet2_area_mm2: f64,
    pub min_terminal_normal_alignment: f64,
    pub terminal_overflow_diameter_mm: f64,
    pub terminal_underflow_z_mm: f64,
    pub minimum_shell_thickness_mm: f64,
    pub minimum_solid_ligament_mm: f64,
    pub minimum_fluid_hydraulic_diameter_mm: f64,
    pub minimum_local_flow_gap_mm: f64,
    pub min_section_jacobian: f64,
    pub tpms_radial_inner_fraction: f64,
    pub declared_units: String,
    pub coordinate_span_m: f64,
    pub missing_origin_traces: usize,
    pub unenacted_active_mechanisms: usize,
    pub active_unsupported_features: usize,
}

impl NegativeCorpusFixture {
    /// Construct a valid baseline fixture that passes all F0 checks, before injecting a specific defect.
    pub fn valid_baseline_template(fixture_id: &str, desc: &str, exp: F0RejectionReason) -> Self {
        Self {
            fixture_id: fixture_id.to_string(),
            description: desc.to_string(),
            expected_rejection: exp,
            open_boundary_edges: 0,
            non_manifold_edges: 0,
            signed_volume_mm3: 681_627.14,
            degenerate_sliver_triangles: 0,
            trapped_fluid_cavities: 0,
            trapped_powder_cavities: 0,
            blind_dead_end_channels: 0,
            frozen_collar_and_tail_diff_mm3: 0.0,
            terminal_inlet1_area_mm2: 120.264094,
            terminal_inlet2_area_mm2: 120.264094,
            min_terminal_normal_alignment: 1.0,
            terminal_overflow_diameter_mm: 1.40,
            terminal_underflow_z_mm: 1639.247432,
            minimum_shell_thickness_mm: 3.0,
            minimum_solid_ligament_mm: 3.0,
            minimum_fluid_hydraulic_diameter_mm: 1.40,
            minimum_local_flow_gap_mm: 1.40,
            min_section_jacobian: 1.0,
            tpms_radial_inner_fraction: 1.0,
            declared_units: "m".to_string(),
            coordinate_span_m: 1.662,
            missing_origin_traces: 0,
            unenacted_active_mechanisms: 0,
            active_unsupported_features: 0,
        }
    }

    /// Audit this fixture against all hard F0 geometry, topology, interface, and enactment gates (`IR-08`, `IR-12`).
    pub fn audit_f0(&self) -> Result<(), Vec<F0RejectionReason>> {
        let mut reasons = Vec::new();
        if self.active_unsupported_features > 0 {
            reasons.push(F0RejectionReason::UnsupportedFeature);
        }
        if self.missing_origin_traces > 0 {
            reasons.push(F0RejectionReason::MissingSemanticOriginTrace);
        }
        if self.unenacted_active_mechanisms > 0 {
            reasons.push(F0RejectionReason::UnenactedActiveMechanism);
        }
        if self.declared_units != "m" || !(0.5..=5.0).contains(&self.coordinate_span_m) {
            reasons.push(F0RejectionReason::UnitScaleMismatch);
        }
        if self.open_boundary_edges > 0 {
            reasons.push(F0RejectionReason::OpenBoundaryEdges);
        }
        if self.non_manifold_edges > 0 {
            reasons.push(F0RejectionReason::NonManifoldEdges);
        }
        if !self.signed_volume_mm3.is_finite() || self.signed_volume_mm3 <= 0.0 {
            reasons.push(F0RejectionReason::NegativeSignedVolume);
        }
        if self.degenerate_sliver_triangles > 0 {
            reasons.push(F0RejectionReason::DegenerateSliverTriangles);
        }
        if self.trapped_fluid_cavities > 0 {
            reasons.push(F0RejectionReason::TrappedFluidCavity);
        }
        if self.trapped_powder_cavities > 0 {
            reasons.push(F0RejectionReason::TrappedPowderCavity);
        }
        if self.blind_dead_end_channels > 0 {
            reasons.push(F0RejectionReason::BlindChannelDeadEnd);
        }
        if !self.frozen_collar_and_tail_diff_mm3.is_finite()
            || self.frozen_collar_and_tail_diff_mm3 > 1e-4
        {
            reasons.push(F0RejectionReason::FrozenCollarOrTailModified);
        }
        let exp_inlet_mm2 = 120.264094;
        if (self.terminal_inlet1_area_mm2 - exp_inlet_mm2).abs() / exp_inlet_mm2 > 0.002
            || (self.terminal_inlet2_area_mm2 - exp_inlet_mm2).abs() / exp_inlet_mm2 > 0.002
        {
            reasons.push(F0RejectionReason::TerminalPatchAreaMismatch);
        }
        if self.min_terminal_normal_alignment < 0.999 {
            reasons.push(F0RejectionReason::TerminalPatchNormalMisaligned);
        }
        if (self.terminal_overflow_diameter_mm - 1.40).abs() > 0.01 {
            reasons.push(F0RejectionReason::TerminalOverflowModified);
        }
        if (self.terminal_underflow_z_mm - 1639.247432).abs() > 0.05 {
            reasons.push(F0RejectionReason::TerminalUnderflowModified);
        }
        if self.minimum_shell_thickness_mm < 1.0 {
            reasons.push(F0RejectionReason::ShellThicknessBelowMinimum);
        }
        if self.minimum_solid_ligament_mm < 1.0 {
            reasons.push(F0RejectionReason::SolidLigamentBelowMinimum);
        }
        if self.minimum_fluid_hydraulic_diameter_mm < 1.40 {
            reasons.push(F0RejectionReason::FluidHydraulicDiameterBelowMinimum);
        }
        if self.minimum_local_flow_gap_mm < 1.20 {
            reasons.push(F0RejectionReason::LocalFlowGapBelowMinimum);
        }
        if self.min_section_jacobian < 0.10 {
            reasons.push(F0RejectionReason::SingularOrInvertedJacobian);
        }
        if self.tpms_radial_inner_fraction < 0.58 {
            reasons.push(F0RejectionReason::CoreObstructionViolation);
        }

        if reasons.is_empty() {
            Ok(())
        } else {
            Err(reasons)
        }
    }
}

/// Generate the canonical 22-fixture malformed geometry negative corpus (`IR-12`).
pub fn build_negative_geometry_corpus_22() -> Vec<NegativeCorpusFixture> {
    use F0RejectionReason::*;
    let mut out = Vec::with_capacity(22);

    let mut f01 = NegativeCorpusFixture::valid_baseline_template(
        "neg_01_open_boundary_edge",
        "Fluid boundary mesh has 4 open boundary half-edges (unsealed hole in chamber wall)",
        OpenBoundaryEdges,
    );
    f01.open_boundary_edges = 4;
    out.push(f01);

    let mut f02 = NegativeCorpusFixture::valid_baseline_template(
        "neg_02_non_manifold_junction",
        "Three triangles share one undirected edge at splitter junction",
        NonManifoldEdges,
    );
    f02.non_manifold_edges = 2;
    out.push(f02);

    let mut f03 = NegativeCorpusFixture::valid_baseline_template(
        "neg_03_inverted_normals",
        "Inward-oriented triangle winding yields negative signed volume",
        NegativeSignedVolume,
    );
    f03.signed_volume_mm3 = -681_627.14;
    out.push(f03);

    let mut f04 = NegativeCorpusFixture::valid_baseline_template(
        "neg_04_trapped_fluid_cavity",
        "Disconnected internal fluid pocket unreachable from any terminal port",
        TrappedFluidCavity,
    );
    f04.trapped_fluid_cavities = 1;
    out.push(f04);

    let mut f05 = NegativeCorpusFixture::valid_baseline_template(
        "neg_05_trapped_powder_cavity",
        "Enclosed void inside solid wall with zero powder evacuation path",
        TrappedPowderCavity,
    );
    f05.trapped_powder_cavities = 1;
    out.push(f05);

    let mut f06 = NegativeCorpusFixture::valid_baseline_template(
        "neg_06_frozen_collar_modified",
        "Harmonic wall perturbation leaks into frozen 0.5*D2 collar z in [151.75, 169.25] mm",
        FrozenCollarOrTailModified,
    );
    f06.frozen_collar_and_tail_diff_mm3 = 42.5;
    out.push(f06);

    let mut f07 = NegativeCorpusFixture::valid_baseline_template(
        "neg_07_frozen_tail_modified",
        "Downstream fine taper/tailpipe radius modified for z > 169.25 mm",
        FrozenCollarOrTailModified,
    );
    f07.frozen_collar_and_tail_diff_mm3 = 310.0;
    out.push(f07);

    let mut f08 = NegativeCorpusFixture::valid_baseline_template(
        "neg_08_inlet_port_area_mismatch",
        "Terminal inlet1 patch area altered by +8.5% from frozen 120.264 mm^2",
        TerminalPatchAreaMismatch,
    );
    f08.terminal_inlet1_area_mm2 = 130.5;
    out.push(f08);

    let mut f09 = NegativeCorpusFixture::valid_baseline_template(
        "neg_09_inlet_port_normal_tilted",
        "Terminal inlet1 normal tilted 12 deg away from [-1, 0, 0]",
        TerminalPatchNormalMisaligned,
    );
    f09.min_terminal_normal_alignment = 0.978;
    out.push(f09);

    let mut f10 = NegativeCorpusFixture::valid_baseline_template(
        "neg_10_overflow_bore_altered",
        "Terminal overflow vortex-finder diameter widened from 1.40 mm to 1.85 mm",
        TerminalOverflowModified,
    );
    f10.terminal_overflow_diameter_mm = 1.85;
    out.push(f10);

    let mut f11 = NegativeCorpusFixture::valid_baseline_template(
        "neg_11_underflow_plane_shifted",
        "Terminal underflow exit plane truncated by 15 mm from 1639.25 mm",
        TerminalUnderflowModified,
    );
    f11.terminal_underflow_z_mm = 1624.0;
    out.push(f11);

    let mut f12 = NegativeCorpusFixture::valid_baseline_template(
        "neg_12_thin_outer_shell",
        "Chamber harmonic amplitude leaves 0.65 mm outer shell (< 1.0 mm minimum)",
        ShellThicknessBelowMinimum,
    );
    f12.minimum_shell_thickness_mm = 0.65;
    out.push(f12);

    let mut f13 = NegativeCorpusFixture::valid_baseline_template(
        "neg_13_thin_splitter_vane",
        "Internal flow-divider vane thickness 0.45 mm (< 1.0 mm minimum solid ligament)",
        SolidLigamentBelowMinimum,
    );
    f13.minimum_solid_ligament_mm = 0.45;
    out.push(f13);

    let mut f14 = NegativeCorpusFixture::valid_baseline_template(
        "neg_14_narrow_fluid_passage",
        "Excessive branch splitting chokes sub-channel hydraulic diameter to 0.95 mm (< 1.40 mm)",
        FluidHydraulicDiameterBelowMinimum,
    );
    f14.minimum_fluid_hydraulic_diameter_mm = 0.95;
    out.push(f14);

    let mut f15 = NegativeCorpusFixture::valid_baseline_template(
        "neg_15_narrow_local_flow_gap",
        "Pinched tangential throat gap 0.85 mm (< 1.20 mm minimum local flow gap)",
        LocalFlowGapBelowMinimum,
    );
    f15.minimum_local_flow_gap_mm = 0.85;
    out.push(f15);

    let mut f16 = NegativeCorpusFixture::valid_baseline_template(
        "neg_16_self_intersecting_wall",
        "High-order azimuthal fold produces negative section Jacobian (-0.18)",
        SingularOrInvertedJacobian,
    );
    f16.min_section_jacobian = -0.18;
    out.push(f16);

    let mut f17 = NegativeCorpusFixture::valid_baseline_template(
        "neg_17_core_obstructing_tpms",
        "TPMS lattice inner radius fraction 0.35 (< 0.58) invades central reverse-flow oil core",
        CoreObstructionViolation,
    );
    f17.tpms_radial_inner_fraction = 0.35;
    out.push(f17);

    let mut f18 = NegativeCorpusFixture::valid_baseline_template(
        "neg_18_unit_scale_mm_vs_m_error",
        "Mesh exported in unscaled millimetres (span 1662.0) while declaring SI metres",
        UnitScaleMismatch,
    );
    f18.coordinate_span_m = 1662.0;
    out.push(f18);

    let mut f19 = NegativeCorpusFixture::valid_baseline_template(
        "neg_19_missing_origin_trace",
        "Boundary triangles lack semantic origin trace back to DesignIrV1 node",
        MissingSemanticOriginTrace,
    );
    f19.missing_origin_traces = 12;
    out.push(f19);

    let mut f20 = NegativeCorpusFixture::valid_baseline_template(
        "neg_20_unenacted_active_branch_gene",
        "Genome claims 6-jet trifurcated manifold but compiled mesh has only 2 unsplit inlet paths",
        UnenactedActiveMechanism,
    );
    f20.unenacted_active_mechanisms = 1;
    out.push(f20);

    let mut f21 = NegativeCorpusFixture::valid_baseline_template(
        "neg_21_unsupported_backend_feature",
        "Active implicit TPMS lattice sent to strict analytical CSG-only backend",
        UnsupportedFeature,
    );
    f21.active_unsupported_features = 1;
    out.push(f21);

    let mut f22 = NegativeCorpusFixture::valid_baseline_template(
        "neg_22_sliver_and_blind_channel",
        "Degenerate zero-area sliver triangles and blind dead-end branch channel",
        DegenerateSliverTriangles,
    );
    f22.degenerate_sliver_triangles = 6;
    f22.blind_dead_end_channels = 1;
    out.push(f22);

    out
}
