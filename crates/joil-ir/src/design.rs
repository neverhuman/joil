use crate::*;
use std::collections::BTreeMap;

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
}
