//! Generate software-progress metadata; no scientific observations.
#[path = "../plan_validation.rs"]
mod plan_validation;
use plan_validation::{digest, encoded, read, validate, Result, STATES};
use serde_json::{json, Map};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn generated(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    validate(root)?;
    let mut progress = Map::new();
    for entry in fs::read_dir(root.join("docs/v5-plan/progress")).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|e| e == "json") {
            progress.insert(
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("invalid progress filename")?
                    .into(),
                read(&path)?,
            );
        }
    }
    let status = read(&root.join("docs/STATUS.json"))?;
    let source = json!({"schema_version":1,"kind":"canonical_public_software_progress_not_physical_evidence",
        "task_registry_sha256":digest(&fs::read(root.join("docs/v5-plan/TASKS.json")).map_err(|e|e.to_string())?),
        "progress":progress,"scientific_status":status});
    let raw = encoded(&source)?;
    let identity = digest(&raw);
    let mut counts = [0; 5];
    let mut total = 0;
    for p in progress.values() {
        for row in p["tasks"]
            .as_object()
            .ok_or("invalid progress tasks")?
            .values()
        {
            total += 1;
            for (i, state) in STATES.iter().enumerate() {
                if row["status"] == *state {
                    counts[i] += 1;
                }
            }
        }
    }
    let mut blockers = match status.get("campaign_blockers") {
        None => vec![],
        Some(v) => v.as_array().ok_or("invalid campaign blockers")?.clone(),
    };
    blockers.extend([
        json!(format!(
            "{} software tasks blocked; inspect the source snapshot for exact reasons",
            counts[2]
        )),
        json!("V5-00-03 remains AUDIT-HELD on D-07, D-18 and D-21"),
        json!("TR-16 / EV-09 / EV-12 remain deferred"),
    ]);
    let manifest = json!({"schema_version":1,"revision":identity,"generated_at":"Wall-clock time not recorded; revision identifies exact source bytes","current":true,
        "source_snapshot":{"sha256":identity,"source":format!("./v5/source-{identity}.json")},
        "campaign":{"tasks_total":total,"tasks_completed":counts[4],"attempts":null,"spend":null,"reservations":null,"checkpoint":null,
        "control_arms":[],"admitted_capabilities":[],"blockers":blockers,
        "software_status":STATES.iter().enumerate().map(|(i,s)|format!("{} {s}",counts[i])).collect::<Vec<_>>().join(", "),
        "scientific_release":format!("Scientific phase: {}; campaign admission: {}",status["phase"].as_str().ok_or("'phase'")?,status["campaign_admission"].as_str().ok_or("'campaign_admission'")?)},
        "subjects":[],"values":[],"archive":[],"historical_corrections":[]});
    Ok(vec![
        (
            "apps/explorer/public/evidence-manifest.json".into(),
            encoded(&manifest)?,
        ),
        (
            format!("apps/explorer/public/v5/source-{identity}.json").into(),
            raw,
        ),
    ])
}
fn run(root: &Path, check: bool) -> Result<()> {
    for (name, data) in generated(root)? {
        let path = root.join(&name);
        if check {
            if path.is_symlink()
                || !path.is_file()
                || fs::read(&path).map_err(|e| e.to_string())? != data
            {
                return Err(format!(
                    "stale explorer manifest/source: {}",
                    name.display()
                ));
            }
        } else {
            if path.is_symlink() {
                return Err("refusing explorer output symlink".into());
            }
            if name
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|n| n == "v5")
                && path.exists()
                && fs::read(&path).map_err(|e| e.to_string())? != data
            {
                return Err("immutable source snapshot collision".into());
            }
            fs::create_dir_all(path.parent().ok_or("missing output parent")?)
                .map_err(|e| e.to_string())?;
            fs::write(path, data).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (command, root) = match args.as_slice() {
        [c] => (c, PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")),
        [c, flag, root] if flag == "--root" => (c, PathBuf::from(root)),
        _ => {
            eprintln!("usage: v5-explorer-manifest {{build,check}} [--root ROOT]");
            std::process::exit(2);
        }
    };
    if !["build", "check"].contains(&command.as_str()) {
        eprintln!("invalid command: {command}");
        std::process::exit(2);
    }
    let result = fs::canonicalize(root)
        .map_err(|e| e.to_string())
        .and_then(|root| run(&root, command == "check"));
    match result {
        Ok(()) => println!(
            "PASS: canonical software-progress explorer {command}; physical observations absent"
        ),
        Err(e) => {
            eprintln!("FAIL: {e}");
            std::process::exit(1);
        }
    }
}
