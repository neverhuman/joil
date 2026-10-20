//! Lowering of inlet ramp and inlet branch manifold layers into `DesignIrV1` nodes.

use super::*;

pub(super) fn lower_inlet_ramp_and_split_layers<F>(
    obj: &serde_json::Map<String, serde_json::Value>,
    inlet_w_m: f64,
    inlet_h_m: f64,
    nodes: &mut Vec<IrNode>,
    gene_mapping: &mut BTreeMap<String, GeneIrMapping>,
    mut push_origin: F,
) -> Result<(), IrError>
where
    F: FnMut(&str, &str, &str, &str, &str, bool) -> String,
{
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
                reason: "inlet_split.enabled == false (canonicalized to 1 branch per port)".into(),
                canonical_default_verified: true,
            },
        );
    }
    Ok(())
}
