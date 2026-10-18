//! Software-only public projection; never authorizes installation.
use serde_json::Value;
use std::{collections::BTreeSet, fs};
const FIELDS: &[&str] = &[
    "subject_sha256",
    "duty_summary",
    "method_summary",
    "software_version",
    "uncertainty_summary",
    "result_summary",
    "limitations",
];
fn project(private: &Value, review: &Value) -> Result<Value, &'static str> {
    if private["classification"] != "private" {
        return Err("dossier_classification_must_be_private");
    }
    if private["industrial_installation_authorized"] != false {
        return Err("software_export_gate_cannot_authorize_installation");
    }
    if private["private_fields"]
        .as_object()
        .is_none_or(|v| v.is_empty())
    {
        return Err("private_dossier_content_required");
    }
    let reviewer = match review.get("reviewer") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
    };
    if review["approved"] != true || reviewer.trim().is_empty() {
        return Err("approved_human_review_required");
    }
    let fields = review["reviewed_fields"]
        .as_object()
        .ok_or("public_export_fields_must_match_exact_allowlist")?;
    if fields.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != FIELDS.iter().copied().collect()
    {
        return Err("public_export_fields_must_match_exact_allowlist");
    }
    for (key, value) in fields {
        let text = value
            .as_str()
            .filter(|v| !v.trim().is_empty())
            .ok_or("public_export_values_must_be_nonempty")?;
        let normalized: String = format!("{key}{text}")
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();
        if [
            "customer",
            "facility",
            "siteid",
            "plantid",
            "accountid",
            "address",
            "contact",
            "serialnumber",
        ]
        .iter()
        .any(|token| normalized.contains(token))
        {
            return Err("public_export_contains_sensitive_identity_token");
        }
    }
    Ok(Value::Object(fields.clone()))
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: v5-private-dossier PRIVATE.json REVIEW.json".into());
    }
    let private = serde_json::from_slice(&fs::read(&args[0])?)?;
    let review = serde_json::from_slice(&fs::read(&args[1])?)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&project(&private, &review)?)?
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("FAIL: {e}");
        std::process::exit(1);
    }
}
