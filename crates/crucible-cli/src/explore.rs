//! Read-only inspection of a versioned evidence manifest.
use crucible_runtime::{Result, RuntimeError};
use serde_json::Value;
use std::path::Path;

pub fn inspect(path: &Path, requested_revision: &str, subject_id: &str) -> Result<Value> {
    let bytes = std::fs::read(path)?;
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
    let subjects = manifest
        .get("subjects")
        .and_then(Value::as_array)
        .ok_or(RuntimeError::Invalid("explorer_manifest_subjects_missing"))?;
    let subject = subjects
        .iter()
        .find(|s| s.get("id").and_then(Value::as_str) == Some(subject_id))
        .ok_or(RuntimeError::Invalid("explorer_subject_not_found"))?;
    let values = subject
        .get("values")
        .and_then(Value::as_array)
        .ok_or(RuntimeError::Invalid("explorer_subject_values_missing"))?;
    Ok(serde_json::json!({
        "read_only": true,
        "schema_version": 1,
        "revision": revision,
        "current": current,
        "subject": subject_id,
        "geometry_sha256": subject.get("geometry_sha256"),
        "values": values,
        "scientific_eligibility": if current { "requires_registered_claim_resolution" } else { "stale_ineligible" }
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
}
