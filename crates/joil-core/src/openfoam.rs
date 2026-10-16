//! Native Rust OpenFOAM 14 Provenance Verifier, Field/Cloud Parsers, KPI Evaluator,
//! and Coupled `OIL-03` Rotation ODE Oracle (`openfoam.rs`).
//!
//! Enforces strict path-traversal rejection, ELF binary SHA-256 verification, failure-marker
//! detection (`FAILED`, `failed.json`, `numerical-failure.json`), exact token-count checks on
//! `labelField` and `vectorField` files, and strictly monotonic timestamps on `surfaceFieldValue.dat`.

use crate::metrics::pdr;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path};

pub const STANDARD_BINS_UM: [f64; 10] = [3.0, 5.0, 8.0, 10.0, 15.0, 20.0, 25.0, 40.0, 60.0, 100.0];

pub fn file_sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    Ok(Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn is_safe_relative_subpath(rel: &str) -> bool {
    let p = Path::new(rel);
    if p.is_absolute() || rel.is_empty() {
        return false;
    }
    for comp in p.components() {
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CarrierProvenance {
    pub input_manifest_sha256: Option<String>,
    pub input_manifest_verified: bool,
    pub mismatched_inputs: Vec<String>,
    pub metrics_binary_metadata_sha256: Option<String>,
    pub metrics_binary_sha256: Option<String>,
    pub metrics_binary_verified: bool,
    pub metrics_receipt_sha256: Option<String>,
    pub metrics_receipt_verified: bool,
    pub failed_marker_present: bool,
}

/// Strictly verify `input-manifest.json`, the compiled `joilHydraulicMetrics` binary, its execution receipt,
/// and failure markers (`FAILED`, `failed.json`, `numerical-failure.json`).
pub fn verify_carrier_case_provenance(
    root_dir: &Path,
    case_dir: &Path,
    time_step: u64,
) -> Result<CarrierProvenance, String> {
    let root_canon = root_dir
        .canonicalize()
        .map_err(|e| format!("failed to canonicalize root_dir: {e}"))?;
    let case_canon = case_dir
        .canonicalize()
        .map_err(|e| format!("failed to canonicalize case_dir: {e}"))?;

    let manifest_path = case_canon.join("input-manifest.json");
    let mut input_manifest_sha256 = None;
    let mut input_manifest_verified = false;
    let mut mismatched_inputs = Vec::new();

    if manifest_path.is_file() {
        input_manifest_sha256 = Some(file_sha256(&manifest_path)?);
        let raw = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
        let map: BTreeMap<String, String> =
            serde_json::from_str(&raw).map_err(|e| format!("invalid input-manifest.json: {e}"))?;
        if map.is_empty() {
            mismatched_inputs.push("<empty_input_manifest>".into());
        } else {
            for (rel_file, expected_hash) in &map {
                if !is_safe_relative_subpath(rel_file) {
                    mismatched_inputs.push(format!("escaped_path:{rel_file}"));
                    continue;
                }
                let candidate = case_canon.join(rel_file);
                match candidate.canonicalize() {
                    Ok(target_canon) => {
                        if !target_canon.starts_with(&case_canon) {
                            mismatched_inputs.push(format!("escaped_symlink:{rel_file}"));
                            continue;
                        }
                        match file_sha256(&target_canon) {
                            Ok(actual_hash) if actual_hash == *expected_hash => {}
                            _ => mismatched_inputs.push(rel_file.clone()),
                        }
                    }
                    Err(_) => mismatched_inputs.push(rel_file.clone()),
                }
            }
            input_manifest_verified = mismatched_inputs.is_empty();
        }
    }

    let bin_meta_path = case_canon.join(format!("metrics-binary-t{time_step}.json"));
    let mut metrics_binary_metadata_sha256 = None;
    let mut metrics_binary_sha256 = None;
    let mut metrics_binary_verified = false;
    if bin_meta_path.is_file() {
        metrics_binary_metadata_sha256 = Some(file_sha256(&bin_meta_path)?);
        if let Ok(raw) = fs::read_to_string(&bin_meta_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                let bin_rel = val
                    .get("path")
                    .or_else(|| val.get("binary"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let expected_sha = val.get("sha256").and_then(|v| v.as_str()).unwrap_or("");
                if is_safe_relative_subpath(bin_rel) && expected_sha.len() == 64 {
                    if let Ok(bin_target) = root_canon.join(bin_rel).canonicalize() {
                        if bin_target.starts_with(&root_canon) && bin_target.is_file() {
                            if let Ok(actual_bin_sha) = file_sha256(&bin_target) {
                                metrics_binary_verified = actual_bin_sha == expected_sha;
                                metrics_binary_sha256 = Some(actual_bin_sha);
                            }
                        }
                    }
                }
            }
        }
    }

    let receipt_path = case_canon.join(format!(
        "metrics-t{time_step}-0-joilHydraulicMetrics.log.receipt.json"
    ));
    let log_path = case_canon.join(format!("metrics-t{time_step}-0-joilHydraulicMetrics.log"));
    let mut metrics_receipt_sha256 = None;
    let mut metrics_receipt_verified = false;
    if receipt_path.is_file() && log_path.is_file() {
        metrics_receipt_sha256 = Some(file_sha256(&receipt_path)?);
        let log_sha = file_sha256(&log_path)?;
        if let Ok(raw) = fs::read_to_string(&receipt_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                let exit_ok = val.get("exit_code").and_then(|v| v.as_i64()) == Some(0);
                let stop_ok = val.get("stop_reason").map(|v| v.is_null()).unwrap_or(false);
                let sha_ok =
                    val.get("log_sha256").and_then(|v| v.as_str()) == Some(log_sha.as_str());
                metrics_receipt_verified = exit_ok && stop_ok && sha_ok;
            }
        }
    }

    let failed_marker_present = ["FAILED", "failed.json", "numerical-failure.json"]
        .iter()
        .any(|m| case_canon.join(m).exists());

    Ok(CarrierProvenance {
        input_manifest_sha256,
        input_manifest_verified,
        mismatched_inputs,
        metrics_binary_metadata_sha256,
        metrics_binary_sha256,
        metrics_binary_verified,
        metrics_receipt_sha256,
        metrics_receipt_verified,
        failed_marker_present,
    })
}

/// Strip the leading `FoamFile { ... }` block from an OpenFOAM ASCII dictionary/field.
fn strip_foam_header(raw: &str) -> &str {
    if let Some(pos) = raw.find("FoamFile") {
        let after = &raw[pos..];
        if let Some(open) = after.find('{') {
            let mut depth = 0usize;
            for (idx, ch) in after[open..].char_indices() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return &after[open + idx + 1..];
                    }
                }
            }
        }
    }
    raw
}

/// Parse an OpenFOAM `labelField` (`N ( i0 i1 ... )` or `N { val }`), rejecting count mismatches.
pub fn parse_labels(raw: &str) -> Result<Vec<i64>, String> {
    let body = strip_foam_header(raw).trim();
    // Handle `N { val }` uniform syntax
    if let (Some(open_brace), Some(close_brace)) = (body.find('{'), body.find('}')) {
        if open_brace < close_brace && !body[..open_brace].contains('(') {
            let count_str = body[..open_brace].split_whitespace().last().unwrap_or("");
            let val_str = body[open_brace + 1..close_brace].trim();
            if let (Ok(count), Ok(val)) = (count_str.parse::<usize>(), val_str.parse::<i64>()) {
                return Ok(vec![val; count]);
            }
        }
    }

    let open_paren = body
        .find('(')
        .ok_or_else(|| "missing '(' in labelField".to_string())?;
    let close_paren = body
        .rfind(')')
        .ok_or_else(|| "missing ')' in labelField".to_string())?;
    if close_paren < open_paren {
        return Err("malformed parentheses in labelField".into());
    }
    let tail_lbl = body[close_paren + 1..].trim();
    if !tail_lbl.is_empty() && tail_lbl != ";" {
        return Err(format!("trailing garbage after labelField: '{tail_lbl}'"));
    }
    let prefix = body[..open_paren].trim();
    let count_tok = prefix
        .split_whitespace()
        .last()
        .ok_or_else(|| "missing element count before '(' in labelField".to_string())?;
    let expected_count: usize = count_tok
        .parse()
        .map_err(|e| format!("invalid labelField count '{count_tok}': {e}"))?;

    let inner = body[open_paren + 1..close_paren].trim();
    if expected_count == 0 {
        if !inner.is_empty() {
            return Err("non-empty body in zero-length labelField".into());
        }
        return Ok(Vec::new());
    }
    let mut out = Vec::with_capacity(expected_count);
    for tok in inner.split_whitespace() {
        let v: i64 = tok
            .parse()
            .map_err(|e| format!("invalid integer token '{tok}' in labelField: {e}"))?;
        out.push(v);
    }
    if out.len() != expected_count {
        return Err(format!(
            "labelField count mismatch: header declared {expected_count}, parsed {}",
            out.len()
        ));
    }
    Ok(out)
}

/// Parse an OpenFOAM `vectorField` (`N ( (x y z) ... )`), rejecting count mismatches and non-finite entries.
pub fn parse_vectors(raw: &str) -> Result<Vec<[f64; 3]>, String> {
    let body = strip_foam_header(raw).trim();
    let open_paren = body
        .find('(')
        .ok_or_else(|| "missing outer '(' in vectorField".to_string())?;
    let close_paren = body
        .rfind(')')
        .ok_or_else(|| "missing outer ')' in vectorField".to_string())?;
    if close_paren < open_paren {
        return Err("malformed parentheses in vectorField".into());
    }
    let tail_vec = body[close_paren + 1..].trim();
    if !tail_vec.is_empty() && tail_vec != ";" {
        return Err(format!("trailing garbage after vectorField: '{tail_vec}'"));
    }
    let prefix = body[..open_paren].trim();
    if prefix.ends_with("uniform") {
        let inner = body[open_paren + 1..close_paren].trim();
        let toks: Vec<&str> = inner.split_whitespace().collect();
        if toks.len() != 3 {
            return Err("uniform vectorField must have exactly 3 components".into());
        }
        let v = [
            toks[0].parse::<f64>().map_err(|e| e.to_string())?,
            toks[1].parse::<f64>().map_err(|e| e.to_string())?,
            toks[2].parse::<f64>().map_err(|e| e.to_string())?,
        ];
        if !v.iter().all(|x| x.is_finite()) {
            return Err("non-finite component in uniform vectorField".into());
        }
        return Ok(vec![v]);
    }

    let count_tok = prefix
        .split_whitespace()
        .last()
        .ok_or_else(|| "missing element count before '(' in vectorField".to_string())?;
    let expected_count: usize = count_tok
        .parse()
        .map_err(|e| format!("invalid vectorField count '{count_tok}': {e}"))?;

    let inner = body[open_paren + 1..close_paren].trim();
    if expected_count == 0 {
        if !inner.is_empty() {
            return Err("non-empty body in zero-length vectorField".into());
        }
        return Ok(Vec::new());
    }

    let mut out = Vec::with_capacity(expected_count);
    let mut rest = inner;
    while let Some(s_idx) = rest.find('(') {
        let after_s = &rest[s_idx + 1..];
        let e_idx = after_s
            .find(')')
            .ok_or_else(|| "unclosed tuple '(' in vectorField".to_string())?;
        let tuple_str = &after_s[..e_idx];
        let toks: Vec<&str> = tuple_str.split_whitespace().collect();
        if toks.len() != 3 {
            return Err(format!(
                "vector tuple must have 3 elements, got '{}'",
                tuple_str
            ));
        }
        let v = [
            toks[0].parse::<f64>().map_err(|e| e.to_string())?,
            toks[1].parse::<f64>().map_err(|e| e.to_string())?,
            toks[2].parse::<f64>().map_err(|e| e.to_string())?,
        ];
        if !v.iter().all(|x| x.is_finite()) {
            return Err("non-finite component in vectorField".into());
        }
        out.push(v);
        rest = &after_s[e_idx + 1..];
    }
    if !rest.trim().is_empty() {
        return Err(format!(
            "trailing garbage in vectorField: '{}'",
            rest.trim()
        ));
    }
    if out.len() != expected_count {
        return Err(format!(
            "vectorField count mismatch: expected {expected_count}, parsed {}",
            out.len()
        ));
    }
    Ok(out)
}

/// Integrate `cloudNumberFlux` (`number/s`) over strictly monotonic timestamps in `surfaceFieldValue.dat`.
pub fn parse_surface_field_value_sums(
    raw: &str,
    cloud_names: &[String],
) -> Result<BTreeMap<String, f64>, String> {
    if cloud_names.is_empty() {
        return Err("cloud_names must not be empty".into());
    }
    let mut totals: BTreeMap<String, f64> = cloud_names.iter().map(|c| (c.clone(), 0.0)).collect();
    let mut t_prev = 0.0_f64;
    let mut n_rows = 0usize;

    for raw_line in raw.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 1 + cloud_names.len() {
            return Err(format!(
                "surfaceFieldValue column count mismatch: expected {}, got {}",
                1 + cloud_names.len(),
                parts.len()
            ));
        }
        let t_cur: f64 = parts[0]
            .parse()
            .map_err(|e| format!("invalid timestamp '{}': {e}", parts[0]))?;
        if !t_cur.is_finite() || (n_rows == 0 && t_cur < 0.0) || (n_rows > 0 && t_cur <= t_prev) {
            return Err(format!(
                "non-monotonic or non-finite timestamp in surfaceFieldValue: t_cur={t_cur}, t_prev={t_prev}"
            ));
        }
        let step_dt = t_cur - t_prev;
        t_prev = t_cur;
        n_rows += 1;

        for (idx, cname) in cloud_names.iter().enumerate() {
            let flux_rate: f64 = parts[1 + idx]
                .parse()
                .map_err(|e| format!("invalid flux rate '{}': {e}", parts[1 + idx]))?;
            if !flux_rate.is_finite() {
                return Err(format!("non-finite flux rate for {cname} at t={t_cur}"));
            }
            if let Some(acc) = totals.get_mut(cname) {
                *acc += flux_rate * step_dt;
            }
        }
    }
    if n_rows == 0 {
        return Err("empty surfaceFieldValue table".into());
    }
    Ok(totals)
}

/// Verify that a time-integrated `cloudNumberFlux` sum is within `tol` of an integer before rounding.
pub fn checked_near_integer_flux(flux_integral: f64, tol: f64) -> Result<(i64, f64), String> {
    if !flux_integral.is_finite() || !tol.is_finite() || tol <= 0.0 {
        return Err("non-finite flux_integral or tolerance".into());
    }
    let nearest = flux_integral.round();
    let residual = (flux_integral - nearest).abs();
    if residual > tol {
        return Err(format!(
            "cloudNumberFlux integral {flux_integral:.12} exceeds integer tolerance {tol:e} (residual={residual:e})"
        ));
    }
    Ok((nearest as i64, residual))
}

/// Evaluate source-bound hydraulic KPIs and eligibility from an executed OpenFOAM case.
fn failed_case_executions(case_dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut failures = Vec::new();
    for marker in ["FAILED", "failed.json", "numerical-failure.json"] {
        let path = case_dir.join(marker);
        if path.exists() {
            failures.push(path);
        }
    }
    for entry in fs::read_dir(case_dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|name| name.ends_with(".receipt.json"))
        {
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            let receipt: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if receipt
                .get("exit_code")
                .and_then(|v| v.as_i64())
                .is_some_and(|code| code != 0)
                || receipt.get("stop_reason").is_some_and(|v| !v.is_null())
            {
                failures.push(path);
            }
        }
    }
    Ok(failures)
}

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
