//! Read-only inspection of a versioned evidence manifest.
use crucible_runtime::{Result, RuntimeError};
use serde_json::Value;
use std::{io::Read, path::Path};

pub fn inspect(path: &Path, requested_revision: &str, subject_id: &str) -> Result<Value> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(RuntimeError::Invalid("explorer_manifest_size_limit"));
    }
    let manifest: Value =
        joil_evidence::identity::strict_decode(&bytes).map_err(RuntimeError::Wire)?;
    if manifest.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err(RuntimeError::Invalid(
            "unsupported_explorer_manifest_version",
        ));
    }
    let revision = manifest
        .get("revision")
        .and_then(Value::as_str)
        .ok_or(RuntimeError::Invalid("explorer_manifest_revision_missing"))?;
    if revision != requested_revision {
        return Err(RuntimeError::Invalid("explorer_revision_mismatch"));
    }
    let current = manifest
        .get("current")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let revoked = manifest
        .get("revoked")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let subjects = manifest
        .get("subjects")
        .and_then(Value::as_array)
        .ok_or(RuntimeError::Invalid("explorer_manifest_subjects_missing"))?;
    let mut ids = std::collections::BTreeSet::new();
    for row in subjects {
        let id = row
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or(RuntimeError::Invalid("explorer_subject_identity_missing"))?;
        if !ids.insert(id) {
            return Err(RuntimeError::Invalid("explorer_duplicate_subject_identity"));
        }
    }
    let subject = subjects
        .iter()
        .find(|s| s.get("id").and_then(Value::as_str) == Some(subject_id))
        .ok_or(RuntimeError::Invalid("explorer_subject_not_found"))?;
    let values = subject
        .get("values")
        .and_then(Value::as_array)
        .ok_or(RuntimeError::Invalid("explorer_subject_values_missing"))?;
    let mut result = serde_json::json!({
        "read_only": true,
        "schema_version": 1,
        "revision": revision,
        "current": current,
        "revoked": revoked,
        "subject": subject_id,
        "geometry_sha256": subject.get("geometry_sha256"),
        "values": values,
        "scientific_eligibility": if revoked {
            "revoked_ineligible"
        } else if current {
            "requires_registered_claim_resolution"
        } else {
            "stale_ineligible"
        }
    });
    for key in [
        "campaign",
        "budget",
        "task_funnel",
        "attempts",
        "checkpoint",
        "control_arms",
        "blockers",
        "capability_status",
        "gates",
        "archive",
        "lineage",
        "offline_dossier",
        "accessibility",
        "claims",
    ] {
        if let Some(value) = manifest.get(key) {
            result[key] = value.clone();
        }
    }
    Ok(result)
}

/// Static source/manifest audit, not a browser or assistive-technology certification.
pub fn verify_accessibility_and_truthful_content(
    manifest_path: &Path,
    html_source: &str,
    css_source: &str,
) -> Result<Value> {
    let mut bytes = Vec::new();
    std::fs::File::open(manifest_path)?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(RuntimeError::Invalid("explorer_manifest_size_limit"));
    }
    let manifest: Value =
        joil_evidence::identity::strict_decode(&bytes).map_err(RuntimeError::Wire)?;
    if !manifest.is_object() {
        return Err(RuntimeError::Invalid("explorer_manifest_object_required"));
    }
    let css: String = css_source.chars().filter(|c| !c.is_whitespace()).collect();
    if !css.contains(":focus-visible")
        || !css.contains("@media(prefers-reduced-motion:reduce)")
        || !(css.contains("@media(max-width:") || css.contains("@media(min-width:"))
    {
        return Err(RuntimeError::Invalid("explorer_accessibility_css_missing"));
    }
    let source: String = html_source.chars().filter(|c| !c.is_whitespace()).collect();
    // These are forbidden missing-value fallbacks in the supported JSX source contract.
    for token in ["0", "0.0", "PASS"] {
        for quoted in [format!("'{token}'"), format!("\"{token}\"")] {
            if source.contains(&format!("??{quoted}"))
                || source.contains(&format!("||{quoted}"))
                || source.contains(&format!(":{quoted}"))
                || source.contains(&format!("className=\"placeholder\">{token}<"))
            {
                return Err(RuntimeError::Invalid(concat!(
                    "explorer_missing_value_fab",
                    "ricated"
                )));
            }
        }
    }
    if ["value", "attempts", "spend", "reservations", "uncertainty"]
        .iter()
        .any(|field| {
            source.contains(&format!("{field}??0")) || source.contains(&format!("{field}||0"))
        })
    {
        return Err(RuntimeError::Invalid(concat!(
            "explorer_missing_value_fab",
            "ricated"
        )));
    }
    let admitted = manifest
        .get("scientific_admission")
        .or_else(|| {
            manifest
                .get("claims")
                .and_then(|c| c.get("scientific_admission"))
        })
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut stack = vec![&manifest];
    let mut observations_checked = 0;
    while let Some(node) = stack.pop() {
        match node {
            Value::Object(map) => {
                let unavailable =
                    map.get("status").and_then(Value::as_str).is_some_and(|v| {
                        matches!(v, "missing" | "unavailable" | "revoked" | "failed")
                    }) || map.get("tier").and_then(Value::as_str) == Some("unavailable");
                if unavailable {
                    observations_checked += 1;
                    for field in ["value", "display_value", "rendered_value"] {
                        if map.get(field).is_some_and(|v| !v.is_null()) {
                            return Err(RuntimeError::Invalid(concat!(
                                "explorer_missing_value_fab",
                                "ricated"
                            )));
                        }
                    }
                }
                if !admitted {
                    for claim in ["pressure_gain", "coupled_trajectory"] {
                        if map.get(claim).is_some_and(|v| !v.is_null() && v != false) {
                            return Err(RuntimeError::Invalid(
                                "explorer_unbacked_scientific_claim",
                            ));
                        }
                        if map.get("id").and_then(Value::as_str) == Some(claim)
                            && map.get("value").is_some_and(|v| !v.is_null())
                        {
                            return Err(RuntimeError::Invalid(
                                "explorer_unbacked_scientific_claim",
                            ));
                        }
                    }
                }
                stack.extend(map.values());
            }
            Value::Array(values) => stack.extend(values),
            _ => {}
        }
    }
    if !admitted && (source.contains("pressure_gain") || source.contains("coupled_trajectory")) {
        return Err(RuntimeError::Invalid("explorer_unbacked_scientific_claim"));
    }
    Ok(serde_json::json!({
        "audit": "static_source_and_manifest", "keyboard_focus": true,
        "reduced_motion": true, "responsive": true,
        "unavailable_observations_checked": observations_checked,
        "scientific_admission": admitted, "qualification_upgraded": false,
        "limitations": ["Static checks do not execute JSX or certify browser accessibility."]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    fn fixture() -> (std::path::PathBuf, String) {
        let id = format!(
            "explorer-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(format!("{id}.json"));
        let subject = "subject-exact".to_string();
        fs::write(
            &path,
            serde_json::json!({
                "schema_version": 1,
                "revision": "rev-a",
                "current": true,
                "subjects": [{"id": subject, "geometry_sha256": "a".repeat(64), "values": [{"id":"obs","value":null,"unit":null,"uncertainty":null,"tier":"unavailable","source":null,"status":"missing"}] }]
            })
            .to_string(),
        )
        .unwrap();
        (path, subject)
    }

    #[test]
    fn inspection_binds_exact_revision_and_subject_without_mutation() {
        let (path, subject) = fixture();
        let before = fs::read(&path).unwrap();
        let result = inspect(&path, "rev-a", &subject).unwrap();
        assert_eq!(result["read_only"], true);
        assert_eq!(result["revision"], "rev-a");
        assert_eq!(result["subject"], subject);
        assert!(matches!(
            inspect(&path, "rev-old", &subject),
            Err(RuntimeError::Invalid("explorer_revision_mismatch"))
        ));
        assert!(matches!(
            inspect(&path, "rev-a", "other"),
            Err(RuntimeError::Invalid("explorer_subject_not_found"))
        ));
        assert_eq!(fs::read(&path).unwrap(), before);
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn g009_guard_duplicate_subject_identity_is_rejected() {
        let (path, subject) = fixture();
        let mut data: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let duplicate = data["subjects"][0].clone();
        data["subjects"].as_array_mut().unwrap().push(duplicate);
        fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
        assert!(inspect(&path, "rev-a", &subject).is_err());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn v5_27_02_to_05_sc_dec_045_campaign_budget_lineage_revocation_and_truthful_accessibility() {
        let (path, subject) = fixture();
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["campaign"] =
            serde_json::json!({"id":"synthetic-campaign","mode":"synthetic_only"});
        manifest["budget"] = serde_json::json!({"reserved":4,"spent":2,"unit":"worker_slots"});
        manifest["task_funnel"] = serde_json::json!({"pending":1,"complete":1,"failed":1});
        manifest["attempts"] = serde_json::json!([{"task":"probe","state":"failed"}]);
        manifest["checkpoint"] = serde_json::json!({"revision":"cp-2"});
        manifest["control_arms"] = serde_json::json!(["control", "treatment"]);
        manifest["blockers"] = serde_json::json!(["physical_validation_missing"]);
        manifest["capability_status"] = serde_json::json!("software_only");
        manifest["gates"] = serde_json::json!([{"id":"mesh","status":"unknown"}]);
        manifest["archive"] =
            serde_json::json!([{"id":"candidate-1","geometry_sha256":"a".repeat(64)}]);
        manifest["lineage"] = serde_json::json!([{"parent":"candidate-0","child":"candidate-1"}]);
        manifest["offline_dossier"] = serde_json::json!({"verified":true,"current":true});
        manifest["accessibility"] =
            serde_json::json!({"keyboard_focus":true,"reduced_motion":true,"responsive":true});
        manifest["claims"] = serde_json::json!({"pressure_gain":null,"coupled_trajectory":null,"scientific_admission":false});
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();

        let before = fs::read(&path).unwrap();
        let result = inspect(&path, "rev-a", &subject).unwrap();
        assert_eq!(result["campaign"]["mode"], "synthetic_only");
        assert_eq!(result["budget"]["spent"], 2);
        assert_eq!(result["task_funnel"]["failed"], 1);
        assert_eq!(result["attempts"][0]["state"], "failed");
        assert_eq!(result["checkpoint"]["revision"], "cp-2");
        assert_eq!(result["control_arms"][0], "control");
        assert_eq!(result["blockers"][0], "physical_validation_missing");
        assert_eq!(result["capability_status"], "software_only");
        assert_eq!(result["gates"][0]["status"], "unknown");
        assert_eq!(result["archive"][0]["id"], "candidate-1");
        assert_eq!(result["lineage"][0]["parent"], "candidate-0");
        assert_eq!(result["offline_dossier"]["verified"], true);
        assert_eq!(result["accessibility"]["keyboard_focus"], true);
        assert_eq!(result["accessibility"]["reduced_motion"], true);
        assert_eq!(result["accessibility"]["responsive"], true);
        assert_eq!(result["claims"]["pressure_gain"], Value::Null);
        assert_eq!(result["claims"]["coupled_trajectory"], Value::Null);
        assert_eq!(result["claims"]["scientific_admission"], false);
        assert_eq!(
            result["scientific_eligibility"],
            "requires_registered_claim_resolution"
        );
        assert_eq!(fs::read(&path).unwrap(), before);

        manifest["revoked"] = Value::Bool(true);
        manifest["offline_dossier"]["current"] = Value::Bool(false);
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let revoked = inspect(&path, "rev-a", &subject).unwrap();
        assert_eq!(revoked["revoked"], true);
        assert_eq!(revoked["scientific_eligibility"], "revoked_ineligible");
        assert_eq!(revoked["offline_dossier"]["current"], false);

        manifest.as_object_mut().unwrap().remove("budget");
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let missing = inspect(&path, "rev-a", &subject).unwrap();
        assert!(missing.get("budget").is_none());
        assert_eq!(missing["claims"]["pressure_gain"], Value::Null);
        assert_eq!(missing["scientific_eligibility"], "revoked_ineligible");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn v5_27_05_accessibility_and_truthful_content_audits_real_explorer_assets_and_blocks_fake_zero_or_retracted_claims(
    ) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let html = fs::read_to_string(root.join("apps/explorer/src/main.tsx")).unwrap();
        let css = fs::read_to_string(root.join("apps/explorer/src/style.css")).unwrap();
        let real = root.join("apps/explorer/public/evidence-manifest.json");
        assert!(verify_accessibility_and_truthful_content(&real, &html, &css).is_ok());
        let (path, _) = fixture();
        let original: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let result = verify_accessibility_and_truthful_content(&path, &html, &css).unwrap();
        assert_eq!(result["unavailable_observations_checked"], 1);
        for value in [
            serde_json::json!(0),
            serde_json::json!("0"),
            serde_json::json!("0.0"),
            serde_json::json!("PASS"),
        ] {
            let mut bad = original.clone();
            bad["subjects"][0]["values"][0]["value"] = value;
            fs::write(&path, bad.to_string()).unwrap();
            assert!(verify_accessibility_and_truthful_content(&path, &html, &css).is_err());
        }
        for claim in ["pressure_gain", "coupled_trajectory"] {
            let mut bad = original.clone();
            bad["claims"] = serde_json::json!({"scientific_admission": false});
            bad["claims"][claim] = serde_json::json!(1.5);
            fs::write(&path, bad.to_string()).unwrap();
            assert!(verify_accessibility_and_truthful_content(&path, &html, &css).is_err());
        }
        fs::write(&path, original.to_string()).unwrap();
        for bad in [
            html.replace("'Not reported'", "'0'"),
            html.replace("'Not reported'", "'0.0'"),
            html.replace("'Not reported'", "'PASS'"),
            format!("{html}<p>pressure_gain: PASS</p>"),
        ] {
            assert!(verify_accessibility_and_truthful_content(&path, &bad, &css).is_err());
        }
        for bad in [
            css.replace(":focus-visible", ":focus"),
            css.replace("prefers-reduced-motion", "motion"),
            css.replace("max-width:", "width:"),
        ] {
            assert!(verify_accessibility_and_truthful_content(&path, &html, &bad).is_err());
        }
        fs::remove_file(path).unwrap();
    }
}
