//! Lowering of internal and outlet `DesignProgram` layers (`ribbon_cascade`, `tpms_insert`,
//! `chamber_modes`, `cone_profile`, `skimmer`) into `DesignIrV1` nodes.

use super::*;

pub(super) fn lower_internal_and_outlet_layers<F>(
    obj: &serde_json::Map<String, serde_json::Value>,
    do_m: f64,
    nodes: &mut Vec<IrNode>,
    gene_mapping: &mut BTreeMap<String, GeneIrMapping>,
    mut push_origin: F,
) -> Result<(), IrError>
where
    F: FnMut(&str, &str, &str, &str, &str, bool) -> String,
{
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
        for (idx, (ord, cos_mm, sin_mm, amp_mm, twist_rad)) in parsed_modes.into_iter().enumerate()
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

    Ok(())
}
