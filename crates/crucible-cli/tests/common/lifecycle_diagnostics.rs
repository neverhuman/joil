use super::scratch::TestScratch;
use super::{base, call, command, record, wait};
use crucible_cli::{read_private_or_source, Campaign};
use crucible_runtime::db::RuntimeDb;
use serde_json::Value;
use std::{path::Path, process::Output, time::Duration};

pub(super) fn failure_context(base: &Path, args: &[&str], out: &Output, expected: i32) -> Value {
    let bounded = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .chars()
            .take(256)
            .collect::<String>()
    };
    let ledger = args
        .windows(2)
        .find(|pair| matches!(pair[0], "--root" | "--resume"))
        .map(|pair| {
            let root = Path::new(pair[1]);
            RuntimeDb::inspect_existing(root)
                .and_then(|mut db| db.ledger_snapshot())
                .map(|snapshot| {
                    let attempts = &snapshot.tables["attempts"];
                    let fields = ["id", "task", "fence", "phase", "ended_ms"];
                    let phase = attempts.columns.iter().position(|c| c == "phase");
                    let ended = attempts.columns.iter().position(|c| c == "ended_ms");
                    let mut ordered = attempts.rows.iter().collect::<Vec<_>>();
                    ordered.sort_by_key(|row| (
                        phase.is_some_and(|column| row[column] == "committed"),
                        std::cmp::Reverse(ended.and_then(|column| row[column].as_u64()).unwrap_or(0)),
                    ));
                    let rows = ordered.into_iter().take(4).map(|row| {
                        fields.iter().filter_map(|name| {
                            attempts.columns.iter().position(|c| c == name)
                                .map(|column| (name.to_string(), row[column].clone()))
                        }).collect::<serde_json::Map<_, _>>()
                    }).collect::<Vec<_>>();
                    let events = &snapshot.tables["events"];
                    let column = |name| events.columns.iter().position(|c| c == name);
                    let diagnostics = match (column("kind"), column("subject"), column("payload")) {
                        (Some(kind), Some(subject), Some(payload)) => events.rows.iter().rev()
                            .filter(|row| row[kind] == "private_attempt_diagnostic").take(4)
                            .map(|row| {
                                let diagnostic = row[payload].as_str().and_then(|wire| {
                                    serde_json::from_str::<crucible_runtime::BlobDigest>(wire).ok()
                                });
                                let detail = diagnostic.as_ref().map(|id| diagnostic_text(root, id));
                                serde_json::json!({"attempt":row[subject],"object":diagnostic,"detail":detail})
                            }).collect::<Vec<_>>(),
                        _ => Vec::new(),
                    };
                    serde_json::json!({"attempts":rows,"diagnostics":diagnostics})
                })
                .unwrap_or_else(|error| serde_json::json!({"unavailable":error.to_string()}))
        });
    serde_json::json!({"base":base,"argv":args,"expected_exit":expected,
        "actual_exit":out.status.code(),"stderr":bounded(&out.stderr),"stdout":bounded(&out.stdout),"ledger":ledger})
}

fn diagnostic_text(root: &Path, id: &crucible_runtime::BlobDigest) -> Value {
    let read = || -> Result<String, String> {
        let objects = root.join("objects");
        let directory = objects.join(id.as_str());
        for path in [&objects, &directory] {
            if !std::fs::symlink_metadata(path)
                .map_err(|e| e.to_string())?
                .is_dir()
            {
                return Err("diagnostic directory is not a real directory".into());
            }
        }
        let manifest = read_private_or_source(&directory.join("manifest.json"), 8192)
            .map_err(|e| e.to_string())?;
        if crucible_runtime::BlobDigest::of_bytes(&manifest) != *id {
            return Err("diagnostic manifest hash mismatch".into());
        }
        let manifest: crucible_store::manifest::Manifest =
            joil_evidence::identity::strict_decode(&manifest).map_err(|e| e.to_string())?;
        manifest.check().map_err(|e| e.to_string())?;
        let bytes =
            read_private_or_source(&directory.join("payload"), 8192).map_err(|e| e.to_string())?;
        if bytes.len() as u64 != manifest.payload_bytes
            || crucible_runtime::BlobDigest::of_bytes(&bytes) != manifest.payload
        {
            return Err("diagnostic payload hash mismatch".into());
        }
        Ok(String::from_utf8_lossy(&bytes).chars().take(256).collect())
    };
    match read() {
        Ok(text) => serde_json::json!({"verified_text":text}),
        Err(error) => serde_json::json!({"unavailable":error}),
    }
}

#[test]
fn unexpected_exit_retains_real_retry_diagnostics_without_repairing_evidence() {
    let _scratch = TestScratch::new();
    let base = base();
    let root = base.join("diagnostic-run");
    let cfg = Campaign {
        schema_version: 1,
        domain: "synthetic_integer_fixture".into(),
        epochs: 1,
        workers: 1,
        seeds: vec![[3, 4]],
        weights: [1, 0],
        mode: crucible_runtime::task::FixtureMode::NumericalFailure,
        timeout_ms: 1000,
    };
    let config = base.join("diagnostic.toml");
    std::fs::write(&config, toml::to_string(&cfg).unwrap()).unwrap();
    let args = [
        "run",
        config.to_str().unwrap(),
        "--root",
        root.to_str().unwrap(),
    ];
    call(&base, &args, 0);
    let before = RuntimeDb::inspect_existing(&root)
        .unwrap()
        .ledger_snapshot()
        .unwrap();
    // The real CLI refuses to reuse a fresh-run root. Its original exit/error
    // must remain visible alongside the earlier real failed/committed attempts.
    let out = wait(command(&args).spawn().unwrap(), Duration::from_secs(60));
    record(&base, &args, &out);
    assert_eq!(out.status.code(), Some(78));
    let context = failure_context(&base, &args, &out, 0);
    assert_eq!(context["argv"], serde_json::json!(args));
    assert_eq!(context["base"], serde_json::json!(base));
    assert_eq!(context["actual_exit"], 78);
    assert_eq!(context["expected_exit"], 0);
    assert!(context["stderr"]
        .as_str()
        .unwrap()
        .contains("fresh_run_root_already_exists"));
    let attempts = context["ledger"]["attempts"].as_array().unwrap();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0]["phase"], "numerical_failure");
    assert_eq!(attempts[0]["fence"], 1);
    assert_eq!(attempts[1]["phase"], "committed");
    assert_eq!(attempts[1]["fence"], 2);
    assert!(attempts.iter().all(|row| row.get("lease").is_none()));
    let diagnostic = &context["ledger"]["diagnostics"][0];
    assert!(diagnostic["detail"]["verified_text"]
        .as_str()
        .unwrap()
        .contains("injected numerical failure"));
    assert_eq!(
        before,
        RuntimeDb::inspect_existing(&root)
            .unwrap()
            .ledger_snapshot()
            .unwrap()
    );

    let id = crucible_runtime::BlobDigest::parse(diagnostic["object"].as_str().unwrap()).unwrap();
    let payload = root.join("objects").join(id.as_str()).join("payload");
    std::fs::write(&payload, b"forged diagnostic").unwrap();
    assert!(diagnostic_text(&root, &id)["unavailable"]
        .as_str()
        .unwrap()
        .contains("payload hash mismatch"));
    let missing = base.join("missing-diagnostic-root");
    assert!(diagnostic_text(&missing, &id).get("unavailable").is_some());
    assert!(
        !missing.exists(),
        "failure reporting must not recreate missing evidence"
    );
    std::fs::rename(&payload, payload.with_file_name("retained-corrupt-payload")).unwrap();
    let outside = base.join("outside-diagnostic");
    std::fs::write(&outside, b"must-not-disclose-this-symlink-target").unwrap();
    std::os::unix::fs::symlink(&outside, &payload).unwrap();
    let rejected = diagnostic_text(&root, &id);
    assert!(rejected.get("unavailable").is_some());
    assert!(!rejected.to_string().contains("must-not-disclose"));
}
