use joil_ir::*;
use serde_json::json;
use std::collections::BTreeSet;
use std::path::PathBuf;

const BASE_SHA: &str = "dfee47126bf99807be79d865a153a4e1ffdf87ef6601a49ef5d80346d04d6c4d";

fn baseline_program_json() -> serde_json::Value {
    json!({
        "schema_version": 2,
        "family": "ct35-exotic-head-v2",
        "topology_class": "axisymmetric-baseline",
        "baseline_file_sha256": BASE_SHA,
        "inlet_ramp": {
            "axial_drop_mm": 0.0,
            "radial_slot_factor": 1.0,
            "superellipse_exponent": 2.0
        },
        "inlet_split": {
            "enabled": false,
            "vane_thickness_mm": 1.2,
            "vane_z_fraction": 0.5,
            "branches_per_port": 1,
            "azimuthal_phase_deg": 0.0
        },
        "chamber_modes": [],
        "cone_profile": {
            "mode1_mm": 0.0,
            "mode2_mm": 0.0,
            "mode3_mm": 0.0
        },
        "skimmer": {
            "length_mm": 0.0,
            "outer_radius_mm": 2.4,
            "capture_flutes": 0
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn exotic_program_json(
    schema_version: u32,
    family: &str,
    ramp_drop_mm: f64,
    slot_factor: f64,
    se_exp: f64,
    branches_per_port: u32,
    rib_vanes: u32,
    tpms_basis: &str,
    modes: &[(u32, f64, f64, f64)],
    cone: (f64, f64, f64),
    skim_len_mm: f64,
    skim_flutes: u32,
) -> serde_json::Value {
    let modes_json: Vec<serde_json::Value> = modes
        .iter()
        .map(|&(ord, c, s, tw)| {
            json!({
                "azimuthal_order": ord,
                "cos_mm": c,
                "sin_mm": s,
                "twist_rad": tw
            })
        })
        .collect();
    json!({
        "schema_version": schema_version,
        "family": family,
        "topology_class": "authored-label-ignored-by-ir",
        "baseline_file_sha256": BASE_SHA,
        "inlet_ramp": {
            "axial_drop_mm": ramp_drop_mm,
            "radial_slot_factor": slot_factor,
            "superellipse_exponent": se_exp
        },
        "inlet_split": {
            "enabled": branches_per_port > 1,
            "vane_thickness_mm": 1.2,
            "vane_z_fraction": 0.5,
            "branches_per_port": branches_per_port,
            "azimuthal_phase_deg": if branches_per_port > 1 { 12.0 } else { 0.0 }
        },
        "ribbon_cascade": {
            "vane_count": rib_vanes,
            "axial_span_mm": if rib_vanes > 0 { 28.0 } else { 0.0 },
            "helical_twist_rad": 0.85,
            "thickness_mm": 1.25,
            "radial_chord_mm": 2.1
        },
        "tpms_insert": {
            "basis": tpms_basis,
            "radial_inner_fraction": 0.68,
            "axial_start_mm": 24.0,
            "axial_end_mm": 62.0,
            "radial_wavelength_mm": 8.5,
            "axial_wavelength_mm": 9.0,
            "sheet_thickness_mm": 1.35
        },
        "chamber_modes": modes_json,
        "cone_profile": {
            "mode1_mm": cone.0,
            "mode2_mm": cone.1,
            "mode3_mm": cone.2
        },
        "skimmer": {
            "length_mm": skim_len_mm,
            "outer_radius_mm": 2.5,
            "capture_flutes": skim_flutes
        }
    })
}

#[test]
fn ir_01_every_gene_maps_to_node_or_inactive() {
    let base = DesignIrV1::from_program_json(&baseline_program_json(), BASE_SHA)
        .expect("baseline program must lower to DesignIrV1");
    assert_eq!(base.units, "SI");
    for expected_gene in [
        "body_envelope",
        "terminals",
        "inlet_ramp",
        "inlet_split",
        "ribbon_cascade",
        "tpms_insert",
        "chamber_modes",
        "cone_profile",
        "skimmer",
    ] {
        assert!(
            base.gene_mapping.contains_key(expected_gene),
            "DesignIrV1 must map gene group {expected_gene}"
        );
    }
    assert!(base.gene_mapping["body_envelope"].is_active());
    assert!(base.gene_mapping["terminals"].is_active());
    assert!(!base.gene_mapping["inlet_ramp"].is_active());
    assert!(!base.gene_mapping["inlet_split"].is_active());
    assert!(!base.gene_mapping["ribbon_cascade"].is_active());
    assert!(!base.gene_mapping["tpms_insert"].is_active());
    assert!(!base.gene_mapping["chamber_modes"].is_active());
    assert!(!base.gene_mapping["cone_profile"].is_active());
    assert!(!base.gene_mapping["skimmer"].is_active());

    let full_prog = exotic_program_json(
        3,
        "ct35-exotic-head-v3",
        2.2,
        0.88,
        2.8,
        3,
        4,
        "Gyroid",
        &[(2, 0.45, -0.20, 0.4), (3, -0.25, 0.30, -0.3)],
        (0.35, -0.22, 0.14),
        6.5,
        4,
    );
    let full = DesignIrV1::from_program_json(&full_prog, BASE_SHA)
        .expect("full Wave-2 exotic program must lower to DesignIrV1");
    let node_ids: BTreeSet<&str> = full.nodes.iter().map(|n| n.node_id()).collect();
    for (gene_name, mapping) in &full.gene_mapping {
        match mapping {
            GeneIrMapping::Active {
                node_ids: mapped_ids,
                semantic_boundary_ids,
            } => {
                assert!(
                    !mapped_ids.is_empty(),
                    "active gene {gene_name} must map to >=1 IrNode"
                );
                assert!(
                    !semantic_boundary_ids.is_empty(),
                    "active gene {gene_name} must map to >=1 SemanticOriginTrace"
                );
                for id in mapped_ids {
                    assert!(
                        node_ids.contains(id.as_str()),
                        "mapped node_id {id} must exist in DesignIrV1.nodes"
                    );
                }
            }
            GeneIrMapping::Inactive { .. } => {
                panic!("gene {gene_name} should be active in full Wave-2 program");
            }
        }
    }
}

#[test]
fn inactive_genes_canonicalize_active_unsupported_fail() {
    // 1. Inactive genes with noisy default fields canonicalize to the exact baseline IR SHA-256
    let noisy_inactive = json!({
        "schema_version": 3,
        "family": "ct35-exotic-head-v3",
        "topology_class": "noisy-inactive-label",
        "baseline_file_sha256": BASE_SHA,
        "inlet_ramp": {
            "axial_drop_mm": 0.0,
            "radial_slot_factor": 1.0,
            "superellipse_exponent": 2.0
        },
        "inlet_split": {
            "enabled": false,
            "vane_thickness_mm": 1.95,
            "vane_z_fraction": 0.33,
            "branches_per_port": 3,
            "azimuthal_phase_deg": 27.0
        },
        "ribbon_cascade": {
            "vane_count": 0,
            "axial_span_mm": 35.0,
            "helical_twist_rad": 1.9,
            "thickness_mm": 1.8,
            "radial_chord_mm": 3.2
        },
        "tpms_insert": {
            "basis": "None",
            "radial_inner_fraction": 0.55,
            "axial_start_mm": 18.0,
            "axial_end_mm": 64.0,
            "radial_wavelength_mm": 6.5,
            "axial_wavelength_mm": 7.0,
            "sheet_thickness_mm": 1.9
        },
        "chamber_modes": [
            {"azimuthal_order": 4, "cos_mm": 0.0, "sin_mm": 0.0, "twist_rad": 1.4}
        ],
        "cone_profile": {
            "mode1_mm": 0.0,
            "mode2_mm": 0.0,
            "mode3_mm": 0.0
        },
        "skimmer": {
            "length_mm": 0.0,
            "outer_radius_mm": 3.8,
            "capture_flutes": 6
        }
    });
    let ir_noisy = DesignIrV1::from_program_json(&noisy_inactive, BASE_SHA).unwrap();
    let ir_base = DesignIrV1::from_program_json(&baseline_program_json(), BASE_SHA).unwrap();
    assert_eq!(
        ir_noisy.ir_sha256, ir_base.ir_sha256,
        "inactive genes must canonicalize to identical ir_sha256"
    );

    // Baseline succeeds on all backends
    for backend in [
        BackendId::RustFastRadialSurface,
        BackendId::RustMultiPatchCfdSurface,
        BackendId::PythonCadQueryBrep,
        BackendId::PythonCadQueryStrictCsgOnly,
        BackendId::VoxelSdfObserver,
        BackendId::LegacyUnenactedCfdSurface,
        BackendId::CudaReducedScreen,
    ] {
        ir_noisy
            .check_backend_support(backend)
            .expect("canonicalized inactive features must succeed on all backends");
    }

    // 2. Active unsupported features fail closed with IrError::UnsupportedFeature
    let tpms_prog = exotic_program_json(
        3,
        "ct35-exotic-head-v3",
        0.0,
        1.0,
        2.0,
        1,
        0,
        "Gyroid",
        &[],
        (0.0, 0.0, 0.0),
        0.0,
        0,
    );
    let tpms_ir = DesignIrV1::from_program_json(&tpms_prog, BASE_SHA).unwrap();
    let err_csg = tpms_ir
        .check_backend_support(BackendId::PythonCadQueryStrictCsgOnly)
        .expect_err("active tpms_insert on strict analytical CSG-only backend must fail");
    assert!(matches!(
        err_csg,
        IrError::UnsupportedFeature {
            backend: BackendId::PythonCadQueryStrictCsgOnly,
            ref feature,
            ..
        } if feature == "tpms_insert"
    ));

    let split_prog = exotic_program_json(
        3,
        "ct35-exotic-head-v3",
        0.0,
        1.0,
        2.0,
        3,
        0,
        "None",
        &[],
        (0.0, 0.0, 0.0),
        0.0,
        0,
    );
    let split_ir = DesignIrV1::from_program_json(&split_prog, BASE_SHA).unwrap();
    let err_legacy = split_ir
        .check_backend_support(BackendId::LegacyUnenactedCfdSurface)
        .expect_err("active inlet_split on LegacyUnenactedCfdSurface must fail");
    assert!(matches!(
        err_legacy,
        IrError::UnsupportedFeature {
            backend: BackendId::LegacyUnenactedCfdSurface,
            ref feature,
            ..
        } if feature == "inlet_split"
    ));
}

#[test]
fn ir_04_feature_enactment_receipt_passes_and_dropped_feature_fails() {
    let prog = exotic_program_json(
        3,
        "ct35-exotic-head-v3",
        2.0,
        0.88,
        2.6,
        3,
        3,
        "Gyroid",
        &[(2, 0.40, -0.15, 0.35)],
        (0.30, -0.18, 0.10),
        6.0,
        4,
    );
    let ir = DesignIrV1::from_program_json(&prog, BASE_SHA).unwrap();
    let mesh = IrCompiledMesh::compile(&ir, BackendId::RustMultiPatchCfdSurface)
        .expect("mesh compilation must succeed");
    let receipt = mesh
        .verify_feature_enactment(&ir)
        .expect("complete compiled mesh must pass feature enactment verification");
    assert!(receipt.all_active_features_enacted);
    assert_eq!(receipt.measured_disjoint_inlet_paths, 6);
    assert_eq!(receipt.expected_disjoint_inlet_paths, 6);

    // Deliberately drop splitter vanes -> must FAIL feature enactment check
    let mut dropped_split = mesh.clone();
    dropped_split.strip_feature_for_negative_test("inlet_split");
    let err = dropped_split
        .verify_feature_enactment(&ir)
        .expect_err("deliberately dropped splitter vanes must fail enactment check");
    assert!(matches!(
        err,
        IrError::UnenactedFeature { ref feature, .. } if feature.contains("inlet_split")
    ));

    // Deliberately drop skimmer sleeve -> must FAIL feature enactment check
    let mut dropped_skimmer = mesh.clone();
    dropped_skimmer.strip_feature_for_negative_test("skimmer");
    let err_skim = dropped_skimmer
        .verify_feature_enactment(&ir)
        .expect_err("deliberately dropped skimmer must fail enactment check");
    assert!(matches!(
        err_skim,
        IrError::UnenactedFeature { ref feature, .. } if feature.contains("skimmer")
    ));
}

#[test]
fn ir_05_legacy_6jet_disjoint_inlet_paths_forensic_relabel() {
    let prog = exotic_program_json(
        2,
        "ct35-exotic-head-v2",
        2.4,
        0.85,
        2.5,
        3,
        0,
        "None",
        &[(3, 0.45, -0.20, 0.5)],
        (0.30, -0.15, 0.08),
        5.5,
        4,
    );
    let ir = DesignIrV1::from_program_json(&prog, BASE_SHA).unwrap();
    let mesh = IrCompiledMesh::compile(&ir, BackendId::RustMultiPatchCfdSurface).unwrap();
    assert_eq!(mesh.measure_disjoint_inlet_paths(), 6);

    // Strip the splitter vanes to reproduce the legacy pre-v4 unsplit CFD surface
    let mut legacy_unsplit = mesh.clone();
    legacy_unsplit.strip_feature_for_negative_test("inlet_split");
    assert_eq!(
        legacy_unsplit.measure_disjoint_inlet_paths(),
        2,
        "Legacy unsplit surface has only 2 disjoint inlet paths, not 6"
    );
}

#[test]
fn ir_06_taxon_derived_from_final_geometry_and_novel_clusters() {
    // Test across 11 representative family configurations
    let configs: Vec<(&str, serde_json::Value)> = vec![
        ("axisymmetric", baseline_program_json()),
        (
            "helical_slot",
            exotic_program_json(
                2,
                "ct35-exotic-head-v2",
                2.5,
                0.85,
                2.8,
                1,
                0,
                "None",
                &[],
                (0.0, 0.0, 0.0),
                0.0,
                0,
            ),
        ),
        (
            "split4_manifold",
            exotic_program_json(
                2,
                "ct35-exotic-head-v2",
                0.0,
                1.0,
                2.0,
                2,
                0,
                "None",
                &[],
                (0.0, 0.0, 0.0),
                0.0,
                0,
            ),
        ),
        (
            "split6_manifold",
            exotic_program_json(
                2,
                "ct35-exotic-head-v2",
                0.0,
                1.0,
                2.0,
                3,
                0,
                "None",
                &[],
                (0.0, 0.0, 0.0),
                0.0,
                0,
            ),
        ),
        (
            "split8_manifold",
            exotic_program_json(
                3,
                "ct35-exotic-head-v3",
                0.0,
                1.0,
                2.0,
                4,
                0,
                "None",
                &[],
                (0.0, 0.0, 0.0),
                0.0,
                0,
            ),
        ),
        (
            "lobed_swirl",
            exotic_program_json(
                2,
                "ct35-exotic-head-v2",
                0.0,
                1.0,
                2.0,
                1,
                0,
                "None",
                &[(3, 0.55, -0.25, 0.8)],
                (0.0, 0.0, 0.0),
                0.0,
                0,
            ),
        ),
        (
            "spline_cone",
            exotic_program_json(
                2,
                "ct35-exotic-head-v2",
                0.0,
                1.0,
                2.0,
                1,
                0,
                "None",
                &[],
                (0.50, -0.35, 0.20),
                0.0,
                0,
            ),
        ),
        (
            "contoured_skimmer",
            exotic_program_json(
                2,
                "ct35-exotic-head-v2",
                0.0,
                1.0,
                2.0,
                1,
                0,
                "None",
                &[(2, 0.35, 0.10, 0.3)],
                (0.0, 0.0, 0.0),
                6.0,
                4,
            ),
        ),
        (
            "helical_ribbon",
            exotic_program_json(
                3,
                "ct35-exotic-head-v3",
                0.0,
                1.0,
                2.0,
                1,
                3,
                "None",
                &[],
                (0.0, 0.0, 0.0),
                0.0,
                0,
            ),
        ),
        (
            "tpms_gyroid",
            exotic_program_json(
                3,
                "ct35-exotic-head-v3",
                0.0,
                1.0,
                2.0,
                2,
                0,
                "Gyroid",
                &[],
                (0.0, 0.0, 0.0),
                0.0,
                0,
            ),
        ),
        (
            "chimera_all",
            exotic_program_json(
                3,
                "ct35-exotic-head-v3",
                2.0,
                0.88,
                2.6,
                3,
                4,
                "Diamond",
                &[(2, 0.4, 0.2, 0.5), (3, -0.3, 0.2, -0.4)],
                (0.4, -0.25, 0.15),
                6.5,
                4,
            ),
        ),
    ];

    for (label, prog_val) in &configs {
        let ir = DesignIrV1::from_program_json(prog_val, BASE_SHA).unwrap();
        let mesh = IrCompiledMesh::compile(&ir, BackendId::RustMultiPatchCfdSurface).unwrap();
        let report = VoxelSdfObserver::observe(&ir, &mesh);
        assert!(
            !report.derived_taxon.taxon_signature.is_empty(),
            "derived taxon signature must be non-empty for {label}"
        );
        let taxon_json = serde_json::to_string(&report.derived_taxon).unwrap();
        assert!(
            !taxon_json.contains("topology_class"),
            "PhenotypeTaxon must never serialize authored topology_class"
        );
    }
}

#[test]
fn ir_07_prior_vs_observed_descriptors_qualification_gate() {
    let prog = exotic_program_json(
        3,
        "ct35-exotic-head-v3",
        1.5,
        0.90,
        2.4,
        2,
        2,
        "SchwarzP",
        &[(2, 0.35, 0.15, 0.4)],
        (0.25, -0.15, 0.05),
        4.5,
        4,
    );
    let ir = DesignIrV1::from_program_json(&prog, BASE_SHA).unwrap();
    let mesh = IrCompiledMesh::compile(&ir, BackendId::RustMultiPatchCfdSurface).unwrap();
    let report = VoxelSdfObserver::observe(&ir, &mesh);

    let obs = ObservedDescriptors {
        geometry: report.observed_geometry.clone(),
        physics: None,
    };
    // V5-01-02: the observer does not measure cavities or topology, so its descriptors cannot qualify.
    let refused = require_observed_descriptors_for_qualification(&obs).unwrap_err();
    assert!(refused.contains("trapped_powder_cavities"), "{refused}");
    let mut synthetic_measured = obs.clone();
    synthetic_measured.geometry.unmeasured_facts.clear();
    let verified = require_observed_descriptors_for_qualification(&synthetic_measured)
        .expect("SYNTHETIC fully measured GeometryObservedDescriptors pass the qualification gate");
    assert_eq!(verified.measured_inlet_channel_count, 4);

    // Unmeasured / zeroed GeometryObservedDescriptors must fail qualification gate
    let mut invalid_obs = synthetic_measured.clone();
    invalid_obs.geometry.fluid_volume_m3 = 0.0;
    assert!(require_observed_descriptors_for_qualification(&invalid_obs).is_err());
}

#[test]
fn ir_08_independent_voxel_sdf_observer_detects_cavities_and_frozen_diff() {
    let ir = DesignIrV1::from_program_json(&baseline_program_json(), BASE_SHA).unwrap();
    let mesh = IrCompiledMesh::compile(&ir, BackendId::RustMultiPatchCfdSurface).unwrap();
    let report = VoxelSdfObserver::observe(&ir, &mesh);
    // Legacy proxy count; V5-01-02 records that powder and enclosed cavities are not measured.
    assert_eq!(report.trapped_cavities_total, 0);
    assert_eq!(
        report.trapped_powder_cavity_observation.measured_value(),
        None
    );
    assert_eq!(
        report.enclosed_fluid_cavity_observation.measured_value(),
        None
    );
    assert_eq!(report.frozen_collar_and_tail_diff_mm3, 0.0);
    assert!(report.all_terminals_reachable_from_chamber);
    assert!((report.terminal_overflow_diameter_mm - 1.40).abs() < 0.02);
    assert!((report.terminal_underflow_z_mm - 1639.247432).abs() < 0.05);

    // Perturb a vertex in the frozen tail (z > 0.16925 m) -> independent observer MUST detect non-zero diff
    let mut perturbed_tail = mesh.clone();
    for v in &mut perturbed_tail.vertices_m {
        if v[2] > 0.25 && v[2] < 1.20 {
            v[0] *= 1.08;
            v[1] *= 1.08;
        }
    }
    let bad_report = VoxelSdfObserver::observe(&ir, &perturbed_tail);
    assert!(
        bad_report.frozen_collar_and_tail_diff_mm3 > 10.0,
        "independent VoxelSdfObserver must detect frozen tail modification"
    );
}

#[test]
fn ir_09_split_geometry_metrics_distinct_physical_definitions() {
    let prog = exotic_program_json(
        3,
        "ct35-exotic-head-v3",
        1.8,
        0.88,
        2.5,
        3,
        3,
        "Gyroid",
        &[(2, 0.4, 0.1, 0.3)],
        (0.2, -0.1, 0.0),
        5.0,
        4,
    );
    let ir = DesignIrV1::from_program_json(&prog, BASE_SHA).unwrap();
    let metrics = SplitGeometryMetrics::from_ir(&ir);
    assert!(metrics.minimum_shell_thickness_mm >= 1.0);
    assert!(metrics.minimum_solid_ligament_mm >= 1.0);
    assert!(metrics.minimum_fluid_hydraulic_diameter_mm >= 1.4);
    assert!(metrics.minimum_local_flow_gap_mm >= 1.2);
    assert!(metrics.minimum_clearance_to_frozen_interface_mm >= 2.0);
    assert!(metrics.minimum_tool_access_diameter_mm >= 1.4);
    assert!(metrics.minimum_powder_exit_diameter_mm >= 1.4);
    assert!(metrics.minimum_support_removal_access_mm >= 1.4);
    // Ensure shell thickness, ligament thickness, frozen clearance, and hydraulic diameter are distinct
    assert!(
        (metrics.minimum_shell_thickness_mm - metrics.minimum_solid_ligament_mm).abs() > 0.1
            && (metrics.minimum_clearance_to_frozen_interface_mm
                - metrics.minimum_fluid_hydraulic_diameter_mm)
                .abs()
                > 1.0
    );
}

#[test]
fn ir_10_cuda_subject_bundle_ref_binds_artifact_bytes() {
    let ir = DesignIrV1::from_program_json(&baseline_program_json(), BASE_SHA).unwrap();
    let artifact_a = b"SOLID ct35_fluid_domain_v1 ... ENDSOLID";
    let artifact_b = b"SOLID ct35_fluid_domain_TAMPERED ... ENDSOLID";
    let ref_a = CudaSubjectBundleRef::from_ir_and_artifact_bytes(&ir, artifact_a);
    let ref_b = CudaSubjectBundleRef::from_ir_and_artifact_bytes(&ir, artifact_b);
    assert_eq!(ref_a.design_ir_sha256, ir.ir_sha256);
    assert_ne!(
        ref_a.geometry_artifact_sha256, ref_b.geometry_artifact_sha256,
        "Changing artifact bytes must change geometry_artifact_sha256"
    );
    assert_ne!(
        ref_a.subject_bundle_sha256, ref_b.subject_bundle_sha256,
        "Changing artifact bytes must change subject_bundle_sha256"
    );
}

#[test]
fn ir_11_every_boundary_face_has_semantic_origin_trace() {
    let prog = exotic_program_json(
        3,
        "ct35-exotic-head-v3",
        1.5,
        0.90,
        2.4,
        3,
        3,
        "CylindricalGyroid",
        &[(2, 0.35, -0.2, 0.5)],
        (0.3, -0.2, 0.1),
        5.5,
        4,
    );
    let ir = DesignIrV1::from_program_json(&prog, BASE_SHA).unwrap();
    let mesh = IrCompiledMesh::compile(&ir, BackendId::RustMultiPatchCfdSurface).unwrap();
    mesh.verify_all_boundary_faces_have_origin_trace()
        .expect("100% of boundary faces must carry valid SemanticOriginTrace");

    // Mutate one face to an unmapped origin_id -> must fail closed
    let mut bad_mesh = mesh.clone();
    bad_mesh.triangle_origin_ids[0] = 9999;
    let err = bad_mesh
        .verify_all_boundary_faces_have_origin_trace()
        .expect_err("unknown origin_id on triangle 0 must fail");
    assert!(matches!(
        err,
        IrError::MissingSemanticOriginTrace {
            triangle_index: 0,
            ..
        }
    ));
}

#[test]
fn ir_12_negative_geometry_corpus_22_rejected_and_metamorphic_invariants() {
    // 1. Verify valid baseline template passes F0
    let valid_template = NegativeCorpusFixture::valid_baseline_template(
        "control_valid_baseline",
        "Valid baseline control",
        F0RejectionReason::OpenBoundaryEdges,
    );
    valid_template
        .audit_f0()
        .expect("valid baseline template must pass audit_f0");

    // 2. Verify all 22 malformed negative fixtures fail F0 with their expected rejection reason
    let corpus = build_negative_geometry_corpus_22();
    assert!(
        corpus.len() >= 20,
        "IR-12 requires >= 20 negative fixtures, got {}",
        corpus.len()
    );

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = manifest_dir.join("../../tests/fixtures/negative");
    std::fs::create_dir_all(&fixtures_dir).expect("create tests/fixtures/negative");

    for fixture in &corpus {
        let path = fixtures_dir.join(format!("{}.json", fixture.fixture_id));
        let serialized = serde_json::to_string_pretty(fixture).unwrap();
        std::fs::write(&path, &serialized).expect("write negative fixture JSON");

        // Reload from disk to test JSON roundtrip + F0 rejection
        let loaded: NegativeCorpusFixture =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let reasons = loaded.audit_f0().expect_err(&format!(
            "Negative fixture {} MUST fail F0",
            loaded.fixture_id
        ));
        assert!(
            reasons.contains(&loaded.expected_rejection),
            "Fixture {} expected {:?}, got {:?}",
            loaded.fixture_id,
            loaded.expected_rejection,
            reasons
        );
    }

    // 3. Metamorphic Test A: Rigid 3D rotation invariance of enclosed volume and wetted area
    let prog = exotic_program_json(
        3,
        "ct35-exotic-head-v3",
        1.5,
        0.90,
        2.4,
        2,
        2,
        "Gyroid",
        &[(2, 0.35, -0.20, 0.45)],
        (0.25, -0.15, 0.10),
        5.0,
        4,
    );
    let ir = DesignIrV1::from_program_json(&prog, BASE_SHA).unwrap();
    let mesh0 = IrCompiledMesh::compile(&ir, BackendId::RustMultiPatchCfdSurface).unwrap();
    let mesh_rot = mesh0.rotated(0.73, -0.41, 0.29);
    assert!(
        (mesh0.signed_volume_m3() - mesh_rot.signed_volume_m3()).abs() < 1e-11,
        "Rigid 3D rotation must preserve enclosed volume"
    );
    assert!(
        (mesh0.wetted_area_m2() - mesh_rot.wetted_area_m2()).abs() < 1e-11,
        "Rigid 3D rotation must preserve wetted area"
    );

    // 4. Metamorphic Test B: Isotropic scale factor s scales volume by s^3 and area by s^2
    let s: f64 = 1.25;
    let mesh_scaled = mesh0.scaled(s);
    assert!(
        (mesh_scaled.signed_volume_m3() / mesh0.signed_volume_m3() - s.powi(3)).abs() < 1e-9,
        "Isotropic scale must scale volume by s^3"
    );
    assert!(
        (mesh_scaled.wetted_area_m2() / mesh0.wetted_area_m2() - s.powi(2)).abs() < 1e-9,
        "Isotropic scale must scale area by s^2"
    );

    // 5. Metamorphic Test C: Chamber-mode order permutation invariance
    let prog_perm_a = exotic_program_json(
        2,
        "ct35-exotic-head-v2",
        0.0,
        1.0,
        2.0,
        2,
        0,
        "None",
        &[(3, -0.25, 0.30, -0.3), (2, 0.45, -0.20, 0.4)],
        (0.2, -0.1, 0.0),
        0.0,
        0,
    );
    let prog_perm_b = exotic_program_json(
        2,
        "ct35-exotic-head-v2",
        0.0,
        1.0,
        2.0,
        2,
        0,
        "None",
        &[(2, 0.45, -0.20, 0.4), (3, -0.25, 0.30, -0.3)],
        (0.2, -0.1, 0.0),
        0.0,
        0,
    );
    let ir_a = DesignIrV1::from_program_json(&prog_perm_a, BASE_SHA).unwrap();
    let ir_b = DesignIrV1::from_program_json(&prog_perm_b, BASE_SHA).unwrap();
    assert_eq!(
        ir_a.ir_sha256, ir_b.ir_sha256,
        "Permuting chamber_modes order must canonicalize to identical ir_sha256"
    );
}

#[test]
fn ir_r1_r2_r4_r5_watertight_split_loops_corrugation_proxy_and_betti_parity() {
    // 1. IR-R1 & IR-R2: 3-branch (6-jet) compiled mesh is a single watertight 2-manifold (0 floating internal cards)
    //    and slices to 3 closed cross-section loops per inlet duct at x = +-0.045 m and x = +-0.050 m.
    let prog_6jet = exotic_program_json(
        3,
        "ct35-exotic-head-v3",
        2.0,
        0.88,
        2.5,
        3,
        0,
        "None",
        &[(3, 0.40, -0.20, 0.5)],
        (0.25, -0.15, 0.08),
        6.0,
        4,
    );
    let ir_6jet = DesignIrV1::from_program_json(&prog_6jet, BASE_SHA).unwrap();
    let mesh_6jet = IrCompiledMesh::compile(&ir_6jet, BackendId::RustMultiPatchCfdSurface).unwrap();
    assert!(
        mesh_6jet.internal_feature_triangles.is_empty(),
        "IR-R1: splitter vanes and skimmer must be woven into the watertight 2-manifold boundary, not floating internal cards"
    );
    assert_eq!(mesh_6jet.count_cross_section_loops_at_x(-0.045), 3);
    assert_eq!(mesh_6jet.count_cross_section_loops_at_x(0.045), 3);
    assert_eq!(mesh_6jet.count_cross_section_loops_at_x(-0.050), 3);
    assert_eq!(mesh_6jet.count_cross_section_loops_at_x(0.050), 3);
    assert_eq!(mesh_6jet.measure_disjoint_inlet_paths(), 6);

    // 2. IR-R4: Active ribbon_cascade and tpms_insert report WallCorrugationProxy (not ConservativeApproximation)
    //    and return is_supported_for_geometry_qualification() == false.
    let prog_corrug = exotic_program_json(
        3,
        "ct35-exotic-head-v3",
        0.0,
        1.0,
        2.0,
        1,
        4,
        "Gyroid",
        &[],
        (0.0, 0.0, 0.0),
        0.0,
        0,
    );
    let ir_corrug = DesignIrV1::from_program_json(&prog_corrug, BASE_SHA).unwrap();
    let support = ir_corrug
        .check_backend_support(BackendId::RustMultiPatchCfdSurface)
        .unwrap();
    let rib_cov = support
        .feature_coverages
        .iter()
        .find(|fc| fc.gene_name == "ribbon_cascade")
        .unwrap();
    let tpms_cov = support
        .feature_coverages
        .iter()
        .find(|fc| fc.gene_name == "tpms_insert")
        .unwrap();
    assert!(matches!(
        rib_cov.support,
        SupportLevel::WallCorrugationProxy { .. }
    ));
    assert!(matches!(
        tpms_cov.support,
        SupportLevel::WallCorrugationProxy { .. }
    ));
    assert!(!rib_cov.support.is_supported_for_geometry_qualification());
    assert!(!tpms_cov.support.is_supported_for_geometry_qualification());
    assert!(!support.qualified_for_geometry);

    // 3. IR-R5: Actual Betti numbers (beta_0, beta_1, beta_2) computed from 2-manifold Euler characteristic:
    //    for B branches per port, each splitter vane creates a topological handle -> beta_1 = 2*(B - 1).
    for (branches, expected_beta1) in [(1u32, 0usize), (2, 2), (3, 4), (4, 6)] {
        let p = exotic_program_json(
            3,
            "ct35-exotic-head-v3",
            0.0,
            1.0,
            2.0,
            branches,
            0,
            "None",
            &[],
            (0.0, 0.0, 0.0),
            0.0,
            0,
        );
        let ir = DesignIrV1::from_program_json(&p, BASE_SHA).unwrap();
        let mesh = IrCompiledMesh::compile(&ir, BackendId::RustMultiPatchCfdSurface).unwrap();
        let obs = VoxelSdfObserver::observe(&ir, &mesh);
        assert_eq!(obs.derived_taxon.betti_0, 1);
        assert_eq!(obs.derived_taxon.betti_2, 1);
        assert_eq!(
            obs.derived_taxon.betti_1, expected_beta1,
            "B={branches} branches per port must have beta_1 = {expected_beta1}"
        );
    }
}
