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

mod lagrangian_ode;
pub use lagrangian_ode::*;
