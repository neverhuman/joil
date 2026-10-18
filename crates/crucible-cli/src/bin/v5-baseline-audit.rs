//! Frozen inventory audit only; never physical qualification.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path, process::Command};
const FROZEN: &str = "e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0";
type Result<T> = std::result::Result<T, String>;
fn require(ok: bool, why: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(why.into())
    }
}
fn rows(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("expected array".into())
}
fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or("expected string".into())
}
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn read(root: &Path, name: &str) -> Result<Value> {
    serde_json::from_slice(&fs::read(root.join(name)).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn bound(root: &Path, path: &str, hash: &Value, source: Option<&Value>) -> Result<()> {
    let out = Command::new("git")
        .args(["show", &format!("{FROZEN}:{path}")])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    require(out.status.success(), "frozen source unavailable")?;
    require(
        hash == &json!(sha(&out.stdout)),
        "source changed; inventory reuse refused",
    )?;
    if let Some(s) = source {
        let raw = String::from_utf8(out.stdout).map_err(|e| e.to_string())?;
        let lines: Vec<_> = raw.lines().collect();
        require(
            !text(&s["symbol"])?.is_empty() && !text(&s["anchor"])?.is_empty(),
            "missing source anchor",
        )?;
        let ns = rows(&s["current_lines"])?;
        let anchor = text(&s["anchor"])?;
        require(
            !ns.is_empty()
                && ns.iter().all(|n| {
                    n.as_u64().is_some_and(|n| {
                        n > 0
                            && (n as usize) <= lines.len()
                            && lines[n as usize - 1].contains(anchor)
                    })
                }),
            "source anchor/line drift",
        )?;
    }
    Ok(())
}
fn audit(root: &Path, source: bool) -> Result<Value> {
    let raw =
        fs::read(root.join("docs/v5-plan/execution/DEFECTS.json")).map_err(|e| e.to_string())?;
    let inventory: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    let mapping = read(root, "docs/v5-plan/DEFECT_MAP.json")?;
    let gates = read(root, "docs/v5-plan/execution/DEFECT_ACCEPTANCE_GATES.json")?;
    let deferrals = read(root, "docs/v5-plan/execution/V4_DEFERRALS.json")?;
    let packets = read(root, "docs/v5-plan/TASKS.json")?;
    require(
        [
            &inventory["frozen_source"],
            &gates["frozen_source_commit"],
            &deferrals["frozen_source_commit"],
        ]
        .iter()
        .all(|v| **v == FROZEN),
        "frozen source drift",
    )?;
    require(
        gates["inventory_path"] == "docs/v5-plan/execution/DEFECTS.json"
            && gates["inventory_sha256"] == sha(&raw),
        "inventory identity drift",
    )?;
    let originals = rows(&inventory["defects"])?;
    let repairs = rows(&mapping["defects"])?;
    let records = rows(&gates["records"])?;
    let expected: BTreeSet<_> = (1..32).map(|n| format!("D-{n:02}")).collect();
    for list in [originals, repairs, records] {
        let ids: BTreeSet<_> = list
            .iter()
            .map(|r| r["id"].as_str().unwrap_or("").to_owned())
            .collect();
        require(
            ids == expected && (list.len() == 31 || !std::ptr::eq(list, records)),
            "defect coverage/duplicates",
        )?;
    }
    let mut behaviors = 0;
    for row in records {
        let old = originals
            .iter()
            .rev()
            .find(|r| r["id"] == row["id"])
            .ok_or("missing defect")?;
        let repair = repairs
            .iter()
            .rev()
            .find(|r| r["id"] == row["id"])
            .ok_or("missing repair")?;
        let probe = &old["minimal_reproducer"]["runtime_probe"];
        let has = match probe {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            Value::Number(n) => n.as_f64() != Some(0.0),
        };
        require(
            row["primary_task"] == old["primary_task"]
                && row["primary_task"] == repair["primary_task"],
            "primary repair drift",
        )?;
        require(
            row["baseline_disposition"] == old["evidence_state"],
            "disposition drift",
        )?;
        let sources = rows(&old["sources"])?;
        require(
            row["source_finding_sha256"]
                == json!(sources
                    .iter()
                    .map(|s| &s["source_sha256"])
                    .collect::<Vec<_>>()),
            "source identity drift",
        )?;
        require(
            row["runtime_probe"] == *probe
                && row["behavior_probe_recorded_at_baseline"] == json!(has)
                && row["inventory_evidence_kind"]
                    == if has {
                        "behavior_probe_recorded"
                    } else {
                        "source_only_inventory"
                    },
            "unexecuted regression promoted",
        )?;
        require(
            row["positive_fixture"] == old["independent_regression"]["positive_control"]
                && row["negative_fixture"] == old["independent_regression"]["negative_fixture"]
                && !text(&row["positive_fixture"])?.is_empty()
                && !text(&row["negative_fixture"])?.is_empty(),
            "independent challenge missing/drift",
        )?;
        let closure = text(&row["primary_repair_closure"])?;
        require(
            closure.contains("executed") && closure.contains("inventory alone cannot close"),
            "primary repair closure gate missing",
        )?;
        require(
            row["physical_qualification"] == false && old["physical_qualification"] == false,
            "physical promotion refused",
        )?;
        behaviors += usize::from(has);
        if source {
            for s in sources {
                bound(root, text(&s["path"])?, &s["source_sha256"], Some(s))?;
            }
        }
    }
    require(behaviors == 15, "baseline behavior count drift")?;
    let ds = rows(&deferrals["deferrals"])?;
    let expected: BTreeSet<_> = [
        "TR-12",
        "G-01",
        "G-04",
        "G-05",
        "INT-06",
        "F2-Oil",
        "F4",
        "TR-16",
        "TR-16-threshold-audit",
        "EV-09",
        "EV-12",
        "V4-follow-on-program",
    ]
    .into_iter()
    .collect();
    require(
        ds.len() == expected.len()
            && ds
                .iter()
                .map(|r| r["id"].as_str().unwrap_or(""))
                .collect::<BTreeSet<_>>()
                == expected,
        "v4 deferral coverage/duplicates",
    )?;
    require(
        deferrals["source_path"] == "docs/WORKPLAN-2026-10-15.md",
        "v4 source drift",
    )?;
    for row in ds {
        require(
            [
                "source_location",
                "title",
                "historical_scoped_disposition",
                "owner",
                "next_dependency",
            ]
            .iter()
            .all(|k| row[k].as_str().is_some_and(|s| !s.trim().is_empty())),
            "deferral owner/dependency/source missing",
        )?;
        let targets = rows(&row["target_packets"])?;
        let ps = rows(&packets["packets"])?;
        require(
            !targets.is_empty() && targets.iter().all(|t| ps.iter().any(|p| p["id"] == *t)),
            "deferral target missing",
        )?;
        require(
            row["physical_pass"] == false
                && row["current_state"] == "blocked_scoped_remainder_retained",
            "deferral promotion refused",
        )?;
    }
    if source {
        bound(
            root,
            text(&deferrals["source_path"])?,
            &deferrals["source_sha256"],
            None,
        )?;
    }
    Ok(
        json!({"allegations":31,"recorded_behavior_probes":behaviors,"source_only_inventory":31-behaviors,"retained_deferrals":ds.len(),"frozen_source_bytes_checked":source,"physical_qualification":false}),
    )
}
fn main() {
    let mut root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut source = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--source-check" => source = true,
            "--root" => root = args.next().unwrap_or_default().into(),
            _ => {
                eprintln!("usage: v5-baseline-audit [--source-check] [--root ROOT]");
                std::process::exit(2)
            }
        }
    }
    match audit(&root, source) {
        Ok(v) => {
            let mut entries = v.as_object().unwrap().iter().collect::<Vec<_>>();
            entries.sort_by_key(|(k, _)| *k);
            println!(
                "{{{}}}",
                entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", serde_json::to_string(k).unwrap(), v))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        Err(e) => {
            eprintln!("FAIL: {e}");
            std::process::exit(1)
        }
    }
}
