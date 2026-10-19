use crate::*;
use serde::{Deserialize, Serialize};

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
