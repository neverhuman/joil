//! Measured CFD KPI evaluation, physics-observed descriptors, coupled OIL-03 rotation ODE oracle, and Lagrangian probe audit.

use super::*;

/// A failed execution has no numerical projection. Successful results from this
/// legacy importer remain carrier diagnostics, not qualified performance labels.
pub fn read_cfd_kpi_observation(
    root_dir: &Path,
    integrals_path: &Path,
    expected_time_step: u64,
) -> Result<serde_json::Value, joil_evidence::observation::Observation> {
    use joil_evidence::{
        identity::ContentRef,
        observation::{FailureClass, Observation},
    };
    let case_dir = integrals_path
        .parent()
        .ok_or_else(|| Observation::Unknown {
            reason: "missing_case_directory".into(),
        })?;
    let failures = failed_case_executions(case_dir).map_err(|_| Observation::Insufficient {
        reason: "case_execution_evidence_unreadable".into(),
        missing: vec!["readable_execution_receipts".into()],
    })?;
    if !failures.is_empty() {
        let diagnostics = failures
            .iter()
            .filter_map(|path| {
                fs::read(path).ok().map(|bytes| {
                    ContentRef::from_bytes(
                        path.file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("failure-artifact"),
                        "1",
                        &bytes,
                    )
                })
            })
            .collect();
        return Err(Observation::Failed {
            class: FailureClass::Numerical,
            reason: "case_contains_failed_or_stopped_execution".into(),
            diagnostics,
        });
    }
    evaluate_measured_cfd_kpis(root_dir, integrals_path, expected_time_step).map_err(|_| {
        Observation::Insufficient {
            reason: "carrier_diagnostic_not_admitted".into(),
            missing: vec!["verified_successful_execution_and_source_bound_finite_integrals".into()],
        }
    })
}

pub fn evaluate_measured_cfd_kpis(
    root_dir: &Path,
    integrals_path: &Path,
    expected_time_step: u64,
) -> Result<serde_json::Value, String> {
    let case_dir = integrals_path
        .parent()
        .ok_or_else(|| "invalid integrals_path parent".to_string())?;
    if !failed_case_executions(case_dir)?.is_empty() {
        return Err(
            "refusing failed or stopped case; no performance observations available".into(),
        );
    }
    let case_name = case_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    if case_name == "ct35-hydraulic-diagnostic-001" {
        return Err("refusing diverged case ct35-hydraulic-diagnostic-001".into());
    }

    let integrals_raw = fs::read_to_string(integrals_path).map_err(|e| e.to_string())?;
    let integrals: serde_json::Value =
        serde_json::from_str(&integrals_raw).map_err(|e| e.to_string())?;
    let time_step = integrals
        .get("time")
        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
        .ok_or_else(|| "missing numeric time in integrals JSON".to_string())?;
    if time_step != expected_time_step {
        return Err(format!(
            "integrals time {time_step} != expected_time_step {expected_time_step}"
        ));
    }

    let prov = verify_carrier_case_provenance(root_dir, case_dir, time_step)?;
    if prov.failed_marker_present {
        return Err(format!(
            "refusing case {case_name}: failed-run / numerical-failure marker present"
        ));
    }
    if !prov.input_manifest_verified || !prov.mismatched_inputs.is_empty() {
        return Err(format!(
            "refusing case {case_name}: input-manifest verification failed: {:?}",
            prov.mismatched_inputs
        ));
    }
    if !prov.metrics_binary_verified || !prov.metrics_receipt_verified {
        return Err("refusing unverified or unsuccessful metrics execution; no performance observations available".into());
    }

    let patches = integrals
        .get("patches")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "missing patches object".to_string())?;
    let get_patch_f64 = |patch: &str, key: &str| -> Result<f64, String> {
        let v = patches
            .get(patch)
            .and_then(|p| p.get(key))
            .and_then(|x| x.as_f64())
            .ok_or_else(|| format!("missing {patch}.{key}"))?;
        if !v.is_finite() {
            return Err(format!("non-finite {patch}.{key}"));
        }
        Ok(v)
    };

    let p_in1 = get_patch_f64("inlet1", "area_average_static_pressure_pa")?;
    let p_in2 = get_patch_f64("inlet2", "area_average_static_pressure_pa")?;
    let p_in = 0.5 * (p_in1 + p_in2);
    let p_u = get_patch_f64("underflow", "area_average_static_pressure_pa")?;
    let p_o = get_patch_f64("overflow", "area_average_static_pressure_pa")?;
    let dp_u = p_in - p_u;
    let dp_o = p_in - p_o;
    let pdr_val = pdr(p_in, p_o, p_u)?;

    let ov_net = get_patch_f64("overflow", "signed_flow_m3_s")?;
    let ov_rev = get_patch_f64("overflow", "reverse_flow_m3_s")?;
    let un_net = get_patch_f64("underflow", "signed_flow_m3_s")?;
    let un_rev = get_patch_f64("underflow", "reverse_flow_m3_s")?;
    if ov_net <= 0.0 || un_net <= 0.0 || ov_rev < 0.0 || un_rev < 0.0 {
        return Err("invalid outlet signed or reverse flow".into());
    }
    let ov_rev_ratio = ov_rev / ov_net;
    let un_rev_ratio = un_rev / un_net;

    let q_feed = integrals
        .get("feed_m3_h")
        .and_then(|v| v.as_f64())
        .ok_or("missing feed_m3_h")?;
    let q_treated = integrals
        .get("treated_m3_h")
        .and_then(|v| v.as_f64())
        .ok_or("missing treated_m3_h")?;
    let r_act = integrals
        .get("reject_fraction")
        .and_then(|v| v.as_f64())
        .ok_or("missing reject_fraction")?;
    let water_closure = integrals
        .get("water_closure_fraction")
        .and_then(|v| v.as_f64())
        .ok_or("missing water_closure_fraction")?;
    let loss_w = integrals
        .get("hydraulic_loss_w")
        .and_then(|v| v.as_f64())
        .ok_or("missing hydraulic_loss_w")?;
    let loss_j_m3 = integrals
        .get("hydraulic_loss_j_per_treated_m3")
        .and_then(|v| v.as_f64())
        .ok_or("missing hydraulic_loss_j_per_treated_m3")?;

    let orig_dev = integrals.get("original_device_overflow");
    let dev_loss_j_m3 = orig_dev
        .and_then(|o| o.get("device_hydraulic_loss_j_per_treated_m3"))
        .and_then(|v| v.as_f64());
    let dev_ov_rev = orig_dev
        .and_then(|o| o.get("reverse_flow_m3_s"))
        .and_then(|v| v.as_f64());

    let hydraulics_pass = prov.input_manifest_verified
        && prov.metrics_binary_verified
        && prov.metrics_receipt_verified
        && water_closure <= 0.001
        && (r_act - 0.02).abs() <= 0.0002
        && ov_rev_ratio <= 0.01
        && un_rev_ratio <= 0.01;

    Ok(serde_json::json!({
        "schema_version": 2,
        "engine": "joil-core::openfoam (native Rust)",
        "case_name": case_name,
        "time_step": time_step,
        "integrals_sha256": file_sha256(integrals_path)?,
        "provenance": prov,
        "hydraulics_and_terminal_backflow_pass_for_diagnostic_pilot": hydraulics_pass,
        "eligible_for_oil_grade": false,
        "eligible_for_campaign_fitness": false,
        "qualified_multiobjective_vector": null,
        "qualified_capacity_gain": null,
        "measured_hydraulics": {
            "feed_flow_m3_h": q_feed,
            "treated_flow_m3_h": q_treated,
            "reject_fraction": r_act,
            "water_closure_fraction": water_closure,
            "dp_inlet_to_underflow_pa": dp_u,
            "dp_inlet_to_overflow_pa": dp_o,
            "pdr": pdr_val,
            "hydraulic_loss_w": loss_w,
            "hydraulic_loss_j_per_treated_m3": loss_j_m3,
            "device_hydraulic_loss_j_per_treated_m3": dev_loss_j_m3,
            "internal_device_overflow_reverse_flow_m3_s": dev_ov_rev,
            "overflow_reverse_flow_m3_s": ov_rev,
            "overflow_reverse_to_net_ratio": ov_rev_ratio,
            "underflow_reverse_flow_m3_s": un_rev,
            "underflow_reverse_to_net_ratio": un_rev_ratio
        }
    }))
}

/// Extract source-bound `PhysicsObservedDescriptors` (`IR-07`) from an executed and provenance-verified
/// OpenFOAM case's integrals file. Rejects unverified or failed cases.
pub fn extract_physics_observed_descriptors(
    root_dir: &Path,
    integrals_path: &Path,
    expected_time_step: u64,
) -> Result<joil_ir::PhysicsObservedDescriptors, String> {
    let kpis = evaluate_measured_cfd_kpis(root_dir, integrals_path, expected_time_step)?;
    let mh = kpis
        .get("measured_hydraulics")
        .ok_or_else(|| "missing measured_hydraulics in KPI output".to_string())?;
    let integrals_sha = kpis
        .get("integrals_sha256")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(joil_ir::PhysicsObservedDescriptors {
        source_case_sha256: integrals_sha,
        measured_dp_u_pa: mh
            .get("dp_inlet_to_underflow_pa")
            .and_then(|v| v.as_f64())
            .ok_or("missing dp_inlet_to_underflow_pa")?,
        measured_pdr: mh
            .get("pdr")
            .and_then(|v| v.as_f64())
            .ok_or("missing pdr")?,
        measured_reject_split: mh
            .get("reject_fraction")
            .and_then(|v| v.as_f64())
            .ok_or("missing reject_fraction")?,
        measured_water_closure: mh
            .get("water_closure_fraction")
            .and_then(|v| v.as_f64())
            .ok_or("missing water_closure_fraction")?,
        measured_hydraulic_loss_j_m3: mh
            .get("hydraulic_loss_j_per_treated_m3")
            .and_then(|v| v.as_f64())
            .ok_or("missing hydraulic_loss_j_per_treated_m3")?,
        measured_swirl_number_entry: None,
        measured_reverse_core_fraction: None,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RotationOdeResult {
    pub diameter_um: f64,
    pub r0_m: f64,
    pub r_end_m: f64,
    pub radial_displacement_m: f64,
    pub u_r_end_m_s: f64,
    pub u_theta_end_m_s: f64,
    pub stokes_u_r_m_s: f64,
}

/// Integrate the exact coupled polar-coordinate Lagrangian equation of motion in a solid-body rotating vortex
/// (`SchillerNaumannDrag` + `pressureGradientForce` + `constantCoefficientVirtualMass(Cvm=0.5)`).
pub fn integrate_coupled_rotation_ode(
    d_um: f64,
    r0_m: f64,
    omega_rad_s: f64,
    dt_s: f64,
    end_time_s: f64,
) -> RotationOdeResult {
    let diam = d_um * 1e-6;
    let rho_w = 998.0_f64;
    let rho_o = 850.0_f64;
    let mu_w = 0.001_f64;
    let c_vm = 0.5_f64;
    let m_eff = rho_o + c_vm * rho_w;

    let mut r = r0_m;
    let mut ur = 0.0_f64;
    let mut ut = omega_rad_s * r0_m;

    let n_sub = ((end_time_s / (dt_s * 0.1)).round() as usize).max(1);
    let h = end_time_s / (n_sub as f64);

    for _ in 0..n_sub {
        let uf_r = 0.0_f64;
        let uf_t = omega_rad_s * r;
        let u_rel = (uf_r - ur).hypot(uf_t - ut);
        let re_p = rho_w * u_rel * diam / mu_w;
        let f_sn = if re_p > 0.0 {
            1.0 + 0.15 * re_p.powf(0.687)
        } else {
            1.0
        };
        let drag_coeff = (18.0 * mu_w / (diam * diam)) * f_sn;
        let f_rad = rho_o * (ut * ut / r)
            + c_vm * rho_w * (ut * ut / r - omega_rad_s * omega_rad_s * r)
            - rho_w * omega_rad_s * omega_rad_s * r;
        let f_tan = -rho_o * (ur * ut / r) - c_vm * rho_w * (ur * ut / r) + drag_coeff * uf_t;
        let denom = 1.0 + (h / m_eff) * drag_coeff;
        ur = (ur + (h / m_eff) * f_rad) / denom;
        ut = (ut + (h / m_eff) * f_tan) / denom;
        r += h * ur;
    }

    let stokes_u_r =
        -(rho_w - rho_o) * (omega_rad_s * omega_rad_s) * r * (diam * diam) / (18.0 * mu_w);

    RotationOdeResult {
        diameter_um: d_um,
        r0_m,
        r_end_m: r,
        radial_displacement_m: r - r0_m,
        u_r_end_m_s: ur,
        u_theta_end_m_s: ut,
        stokes_u_r_m_s: stokes_u_r,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LagrangianProbeAuditSummary {
    pub summary_path: String,
    pub summary_sha256: String,
    pub kind: String,
    pub injection_mode: String,
    pub passed: bool,
    pub inventory_closure_passed: bool,
    pub launch_time_parent_manifest_verified: bool,
    pub cohort_count: usize,
    pub max_flux_integer_residual: f64,
    pub total_discrepancy_count: i64,
    pub reference_cloud_discrepancy_count: i64,
    pub audit_passed: bool,
    pub failure_reasons: Vec<String>,
}

/// Audits a full-liner or flux-weighted-inlet Lagrangian capability probe JSON receipt,
/// verifying 10-bin + reference-cloud conservation, near-integer `cloudNumberFlux` residuals (`<= 1e-6`),
/// and launch-time `parent-carrier-manifest.json` SHA-256 binding.
pub fn audit_lagrangian_probe_summary(
    summary_path: &Path,
    root: &Path,
) -> Result<LagrangianProbeAuditSummary, String> {
    if !summary_path.is_file() {
        return Err(format!(
            "missing probe summary file {}",
            summary_path.display()
        ));
    }
    let summary_sha256 = file_sha256(summary_path)?;
    let raw = fs::read_to_string(summary_path).map_err(|e| e.to_string())?;
    let doc: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    // Support both single-case result JSON and paired pilot summary JSON (`pilot_coarse` / `pilot_fine`)
    let target_doc = if doc.get("cohorts").is_some() {
        &doc
    } else if let Some(pc) = doc.get("pilot_coarse") {
        pc
    } else {
        return Err("probe summary JSON contains neither `cohorts` nor `pilot_coarse`".to_string());
    };

    let kind = target_doc
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let injection_mode = target_doc
        .get("injection_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("deterministic_32_probe_seeds")
        .to_string();
    let passed = target_doc
        .get("passed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let inventory_closure_passed = target_doc
        .get("inventory_closure_passed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut failure_reasons = Vec::new();
    if !passed {
        failure_reasons.push("summary_passed_is_false".to_string());
    }
    if !inventory_closure_passed {
        failure_reasons.push("inventory_closure_passed_is_false".to_string());
    }

    // Verify parent-carrier-manifest.json if referenced
    let mut launch_time_parent_manifest_verified = false;
    if let Some(expected_hashes) = target_doc
        .get("parent_carrier_field_hashes")
        .and_then(|v| v.as_object())
    {
        let manifest_path = if let Some(rel_m) = target_doc
            .get("parent_carrier_manifest_file")
            .and_then(|v| v.as_str())
        {
            Some(root.join(rel_m))
        } else if let Some(rel_res) = target_doc.get("result_file").and_then(|v| v.as_str()) {
            root.join(rel_res)
                .parent()
                .map(|p| p.join("parent-carrier-manifest.json"))
        } else {
            summary_path
                .parent()
                .map(|p| p.join("parent-carrier-manifest.json"))
        };

        if let Some(mp) = manifest_path {
            if mp.is_file() {
                let m_raw = fs::read_to_string(&mp).map_err(|e| e.to_string())?;
                let m_json: serde_json::Value =
                    serde_json::from_str(&m_raw).map_err(|e| e.to_string())?;
                if let Some(m_obj) = m_json.as_object() {
                    let mut all_match = !expected_hashes.is_empty();
                    for (k, exp_v) in expected_hashes {
                        if m_obj.get(k) != Some(exp_v) {
                            all_match = false;
                            failure_reasons.push(format!("parent_manifest_mismatch_for_{k}"));
                        }
                    }
                    launch_time_parent_manifest_verified = all_match;
                }
            } else {
                failure_reasons.push(format!("missing_parent_manifest({})", mp.display()));
            }
        }
    }

    let cohorts = target_doc
        .get("cohorts")
        .and_then(|v| v.as_array())
        .ok_or("missing cohorts array in probe summary")?;
    if cohorts.len() != STANDARD_BINS_UM.len() {
        failure_reasons.push(format!(
            "cohort_count_mismatch({} != {})",
            cohorts.len(),
            STANDARD_BINS_UM.len()
        ));
    }

    let mut max_flux_integer_residual = 0.0_f64;
    let mut total_discrepancy_count = 0_i64;

    for (idx, c) in cohorts.iter().enumerate() {
        let d_um = c
            .get("diameter_um")
            .and_then(|v| v.as_f64())
            .unwrap_or(-1.0);
        if (d_um - STANDARD_BINS_UM.get(idx).copied().unwrap_or(-2.0)).abs() > 1e-6 {
            failure_reasons.push(format!("cohort_{idx}_diameter_mismatch({d_um})"));
        }
        let n_inj = c
            .get("injected_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let n_ov = c
            .get("overflow_terminal_escaped_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let n_un = c
            .get("underflow_terminal_escaped_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let n_wall = c
            .get("wall_stuck_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let n_rem = c
            .get("remaining_in_domain_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let n_other = c
            .get("other_patch_active_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let disc = c
            .get("discrepancy_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(n_inj - (n_ov + n_un + n_wall + n_rem + n_other));
        total_discrepancy_count += disc.abs();
        if n_inj <= 0 || disc != 0 || n_inj != n_ov + n_un + n_wall + n_rem + n_other {
            failure_reasons.push(format!("cohort_{d_um}um_discrepancy({disc})"));
        }
        let ov_res = c
            .get("overflow_flux_integer_residual")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let un_res = c
            .get("underflow_flux_integer_residual")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        max_flux_integer_residual = max_flux_integer_residual.max(ov_res).max(un_res);
        if ov_res > 1e-6 || un_res > 1e-6 {
            failure_reasons.push(format!(
                "cohort_{d_um}um_flux_residual_exceeded({ov_res:.3e}, {un_res:.3e})"
            ));
        }
    }

    let ref_cloud = target_doc
        .get("neutral_density_1um_reference_cloud")
        .ok_or("missing neutral_density_1um_reference_cloud")?;
    let ref_disc = ref_cloud
        .get("discrepancy_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    if ref_disc != 0 {
        failure_reasons.push(format!("reference_cloud_discrepancy({ref_disc})"));
    }

    let audit_passed = failure_reasons.is_empty();
    Ok(LagrangianProbeAuditSummary {
        summary_path: summary_path.display().to_string(),
        summary_sha256,
        kind,
        injection_mode,
        passed,
        inventory_closure_passed,
        launch_time_parent_manifest_verified,
        cohort_count: cohorts.len(),
        max_flux_integer_residual,
        total_discrepancy_count,
        reference_cloud_discrepancy_count: ref_disc,
        audit_passed,
        failure_reasons,
    })
}
