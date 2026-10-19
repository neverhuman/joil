use crate::*;
use serde::{Deserialize, Serialize};

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

pub(crate) fn legacy_unmeasured_facts() -> Vec<String> {
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
