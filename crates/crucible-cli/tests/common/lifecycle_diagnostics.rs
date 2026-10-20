use super::scratch::TestScratch;
use super::{base, call, command, record, wait};
use crucible_cli::Campaign;
use crucible_runtime::{checkpoint::TableSnapshot, db::RuntimeDb};
use serde_json::Value;
use std::{
    fs::OpenOptions,
    io::Read,
    os::unix::{ffi::OsStrExt, fs::OpenOptionsExt},
    path::Path,
    process::{Output, Stdio},
    time::Duration,
};

fn bounded_text(bytes: &[u8]) -> Value {
    let text = String::from_utf8_lossy(bytes);
    let mut end = text.len().min(256);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    serde_json::json!({"text":&text[..end],"truncated":end < text.len(),
        "omitted_utf8_bytes":text.len()-end,"lossy_utf8":std::str::from_utf8(bytes).is_err()})
}

fn bounded_value(value: &Value) -> Value {
    match value.as_str() {
        Some(text) if text.len() > 256 => bounded_text(text.as_bytes()),
        _ => value.clone(),
    }
}

pub(super) fn failure_context(base: &Path, args: &[&str], out: &Output, expected: i32) -> Value {
    let ledger = args.windows(2)
        .find(|pair| matches!(pair[0], "--root" | "--resume"))
        .map(|pair| {
            let root = Path::new(pair[1]);
            RuntimeDb::inspect_existing(root)
                .and_then(|mut db| db.ledger_snapshot())
                .map(|snapshot| ledger_context(root, &snapshot.tables["attempts"], &snapshot.tables["events"]))
                .unwrap_or_else(|error| serde_json::json!({"unavailable":bounded_text(error.to_string().as_bytes())}))
        });
    serde_json::json!({"base":bounded_value(&serde_json::json!(base)),
        "argv":args.iter().map(|arg| bounded_value(&serde_json::json!(arg))).collect::<Vec<_>>(),
        "expected_exit":expected,"actual_exit":out.status.code(),
        "stderr":bounded_text(&out.stderr),"stdout":bounded_text(&out.stdout),"ledger":ledger})
}

fn ledger_context(root: &Path, attempts: &TableSnapshot, events: &TableSnapshot) -> Value {
    let fields = ["id", "task", "fence", "phase", "ended_ms"];
    let phase = attempts.columns.iter().position(|c| c == "phase");
    let ended = attempts.columns.iter().position(|c| c == "ended_ms");
    let mut ordered = attempts.rows.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|row| {
        (
            phase.is_some_and(|column| row[column] == "committed"),
            std::cmp::Reverse(ended.and_then(|column| row[column].as_u64()).unwrap_or(0)),
        )
    });
    let rows = ordered
        .into_iter()
        .take(4)
        .map(|row| {
            fields
                .iter()
                .filter_map(|name| {
                    attempts
                        .columns
                        .iter()
                        .position(|c| c == name)
                        .map(|column| (name.to_string(), bounded_value(&row[column])))
                })
                .collect::<serde_json::Map<_, _>>()
        })
        .collect::<Vec<_>>();
    let column = |name| events.columns.iter().position(|c| c == name);
    let mut diagnostics_total = 0;
    let diagnostics = match (column("kind"), column("subject"), column("payload")) {
        (Some(kind), Some(subject), Some(payload)) => {
            let matching = events
                .rows
                .iter()
                .rev()
                .filter(|row| row[kind] == "private_attempt_diagnostic")
                .collect::<Vec<_>>();
            diagnostics_total = matching.len();
            matching.into_iter().take(4).map(|row| {
                let diagnostic = row[payload].as_str().and_then(|wire| {
                    serde_json::from_str::<crucible_runtime::BlobDigest>(wire).ok()
                });
                let detail = diagnostic.as_ref().map(|id| diagnostic_text(root, id));
                serde_json::json!({"attempt":bounded_value(&row[subject]),"object":diagnostic,"detail":detail})
            }).collect::<Vec<_>>()
        }
        _ => Vec::new(),
    };
    serde_json::json!({"attempts_total":attempts.rows.len(),"attempts_omitted":attempts.rows.len()-rows.len(),
        "attempts":rows,"diagnostics_total":diagnostics_total,
        "diagnostics_omitted":diagnostics_total-diagnostics.len(),"diagnostics":diagnostics})
}

fn read_regular(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    // Check the opened descriptor, so replacement by a FIFO cannot block open
    // or race a pre-open file-type check. This reader is test-only.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|error| error.to_string())?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("diagnostic is not a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("diagnostic grew past its byte limit".into());
    }
    Ok(bytes)
}

fn diagnostic_text(root: &Path, id: &crucible_runtime::BlobDigest) -> Value {
    let read = || -> Result<Vec<u8>, String> {
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
        let manifest = read_regular(&directory.join("manifest.json"), 8192)?;
        if crucible_runtime::BlobDigest::of_bytes(&manifest) != *id {
            return Err("diagnostic manifest hash mismatch".into());
        }
        let manifest: crucible_store::manifest::Manifest =
            joil_evidence::identity::strict_decode(&manifest).map_err(|e| e.to_string())?;
        manifest.check().map_err(|e| e.to_string())?;
        let bytes = read_regular(&directory.join("payload"), 8192)?;
        if bytes.len() as u64 != manifest.payload_bytes
            || crucible_runtime::BlobDigest::of_bytes(&bytes) != manifest.payload
        {
            return Err("diagnostic payload hash mismatch".into());
        }
        Ok(bytes)
    };
    match read() {
        Ok(bytes) => serde_json::json!({"verified_text":bounded_text(&bytes)}),
        Err(error) => serde_json::json!({"unavailable":bounded_text(error.as_bytes())}),
    }
}

#[test]
fn oversized_context_reports_omissions_and_preserves_utf8_boundaries() {
    let oversized = "a".repeat(255) + "😀";
    let clipped = bounded_text(oversized.as_bytes());
    assert_eq!(clipped["text"], "a".repeat(255));
    assert_eq!(clipped["truncated"], true);
    assert_eq!(clipped["omitted_utf8_bytes"], 4);
    assert_eq!(bounded_text(b"complete")["truncated"], false);
    let id = crucible_runtime::BlobDigest::of_bytes(b"synthetic missing diagnostic");
    let attempts = TableSnapshot {
        columns: ["id", "task", "fence", "phase", "ended_ms"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        rows: (1..=8)
            .map(|n| {
                vec![
                    serde_json::json!(format!("a{n}")),
                    serde_json::json!("t".repeat(1024)),
                    serde_json::json!(n),
                    serde_json::json!("infrastructure_failure"),
                    serde_json::json!(n),
                ]
            })
            .collect(),
    };
    let events = TableSnapshot {
        columns: ["kind", "subject", "payload"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        rows: (1..=8)
            .map(|n| {
                vec![
                    serde_json::json!("private_attempt_diagnostic"),
                    serde_json::json!(format!("a{n}")),
                    serde_json::json!(serde_json::to_string(&id).unwrap()),
                ]
            })
            .collect(),
    };
    let _scratch = TestScratch::new();
    let missing = TestScratch::path("absent-diagnostic");
    let context = ledger_context(&missing, &attempts, &events);
    assert_eq!(context["attempts"].as_array().unwrap().len(), 4);
    assert_eq!(context["attempts_total"], 8);
    assert_eq!(context["attempts_omitted"], 4);
    assert_eq!(context["diagnostics_total"], 8);
    assert_eq!(context["diagnostics_omitted"], 4);
    assert_eq!(context["attempts"][0]["id"], "a8");
    assert_eq!(
        context["attempts"][0]["task"]["text"]
            .as_str()
            .unwrap()
            .len(),
        256
    );
    assert_eq!(context["attempts"][0]["task"]["truncated"], true);
    assert!(!missing.exists());
}

#[test]
fn unexpected_exit_retains_real_retry_diagnostics_without_repairing_evidence() {
    const FIFO_ROOT: &str = "JOIL_TEST_DIAGNOSTIC_FIFO_ROOT";
    if let Some(root) = std::env::var_os(FIFO_ROOT) {
        let root = std::path::PathBuf::from(root);
        let config = root.parent().unwrap().join("diagnostic.toml");
        let args = [
            "run",
            config.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
        ];
        let out = wait(command(&args).spawn().unwrap(), Duration::from_secs(60));
        assert_eq!(out.status.code(), Some(78));
        let context = failure_context(root.parent().unwrap(), &args, &out, 0);
        assert_eq!(context["actual_exit"], 78);
        assert_eq!(context["expected_exit"], 0);
        assert!(context["stderr"]["text"]
            .as_str()
            .unwrap()
            .contains("fresh_run_root_already_exists"));
        assert!(context["ledger"]["diagnostics"][0]["detail"]
            .get("unavailable")
            .is_some());
        println!("{context}");
        return;
    }
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
    assert!(context["stderr"]["text"]
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
    assert!(diagnostic["detail"]["verified_text"]["text"]
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
    let original_payload = std::fs::read(&payload).unwrap();
    std::fs::write(&payload, b"forged diagnostic").unwrap();
    assert!(diagnostic_text(&root, &id)["unavailable"]["text"]
        .as_str()
        .unwrap()
        .contains("payload hash mismatch"));
    let missing = base.join("missing-diagnostic-root");
    assert!(diagnostic_text(&missing, &id).get("unavailable").is_some());
    assert!(
        !missing.exists(),
        "failure reporting must not recreate missing evidence"
    );
    std::fs::rename(&payload, base.join("retained-corrupt-payload")).unwrap();
    let outside = base.join("outside-diagnostic");
    std::fs::write(&outside, b"must-not-disclose-this-symlink-target").unwrap();
    std::os::unix::fs::symlink(&outside, &payload).unwrap();
    let rejected = diagnostic_text(&root, &id);
    assert!(rejected.get("unavailable").is_some());
    assert!(!rejected.to_string().contains("must-not-disclose"));
    std::fs::remove_file(&payload).unwrap(); // Only this test's symlink.
    std::fs::write(&payload, &original_payload).unwrap();
    for name in ["manifest.json", "payload"] {
        let path = root.join("objects").join(id.as_str()).join(name);
        let retained = base.join(format!("retained-{name}-fifo"));
        std::fs::rename(&path, &retained).unwrap();
        let fifo = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: fifo is a live NUL-terminated owned scratch path; mode0600
        // creates only the controlled test FIFO, never an external endpoint.
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "lifecycle_diagnostics::unexpected_exit_retains_real_retry_diagnostics_without_repairing_evidence", "--nocapture"])
            .env(FIFO_ROOT, &root).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().unwrap();
        let output = wait(child, Duration::from_secs(10));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("fresh_run_root_already_exists"));
        std::fs::remove_file(&path).unwrap(); // Only this test's FIFO.
        std::fs::rename(&retained, &path).unwrap();
    }
}
