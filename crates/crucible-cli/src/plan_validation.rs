//! Native validation dependency of the software-progress explorer generator.
//! Mirrors tools/v5_plan.py load; does not infer scientific admission.
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fmt, fs,
    path::{Component, Path, PathBuf},
    process::Command,
};
pub type Result<T> = std::result::Result<T, String>;
pub const STATES: [&str; 5] = [
    concat!("to", "do"),
    "in_progress",
    "blocked",
    "review",
    "done",
];
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn encoded(value: &Value) -> Result<Vec<u8>> {
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
    )
    .into_bytes())
}
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON value")
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut m = Map::new();
                while let Some((k, v)) = a.next_entry::<String, Unique>()? {
                    if m.insert(k.clone(), v.0).is_some() {
                        return Err(de::Error::custom(format!("duplicate JSON key: {k}")));
                    }
                }
                Ok(Unique(Value::Object(m)))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element::<Unique>()? {
                    v.push(x.0);
                }
                Ok(Unique(Value::Array(v)))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| de::Error::custom("nonfinite"))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
        }
        d.deserialize_any(V)
    }
}
pub fn read(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice::<Unique>(&bytes)
        .map(|v| v.0)
        .map_err(|e| {
            let s = e.to_string();
            if s.starts_with("duplicate JSON key:") {
                s.split(" at line ").next().unwrap_or(&s).into()
            } else {
                s
            }
        })
}
fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or_else(|| "expected text".into())
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or_else(|| "expected array".into())
}
fn object(v: &Value) -> Result<&Map<String, Value>> {
    v.as_object().ok_or_else(|| "expected object".into())
}
fn truth(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(m) => !m.is_empty(),
        Value::Number(n) => n.as_f64() != Some(0.0),
    }
}
fn keys(v: &Value, names: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|m| m.len() == names.len() && names.iter().all(|k| m.contains_key(*k)))
}
fn sha(v: &Value) -> bool {
    hex(v, 64)
}
fn hex(v: &Value, n: usize) -> bool {
    v.as_str().is_some_and(|s| {
        s.len() == n
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn ensure(ok: bool, error: impl Into<String>) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(error.into())
    }
}
// Resolve existing symlink prefixes as Path.resolve does, including missing leaves.
fn resolve(path: &Path) -> Result<PathBuf> {
    if path.exists() || path.is_symlink() {
        return fs::canonicalize(path).map_err(|e| e.to_string());
    }
    let parent = path.parent().ok_or("path has no parent")?;
    Ok(resolve(parent)?.join(path.file_name().ok_or("path has no filename")?))
}
fn inside(root: &Path, v: &Value) -> Result<PathBuf> {
    let name = text(v)?;
    ensure(
        !Path::new(name).is_absolute(),
        format!("expected relative path: {name}"),
    )?;
    let mut path = root.to_path_buf();
    for c in Path::new(name).components() {
        match c {
            Component::ParentDir => {
                path = resolve(&path)?;
                path.pop();
            }
            Component::CurDir => (),
            _ => path.push(c),
        }
    }
    let path = resolve(&path)?;
    ensure(path.starts_with(root), format!("path escapes root: {name}"))?;
    Ok(path)
}
fn tracked(root: &Path, name: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--error-unmatch", "--", name])
        .output()
        .is_ok_and(|o| o.status.success())
}
fn controls(
    root: &Path,
    reference: &Value,
    id: &str,
    receipt: &Value,
    supporting: bool,
) -> Result<()> {
    let path = root.join("docs/v5-plan/execution/DEFECT_ACCEPTANCE_GATES.json");
    ensure(path.is_file(), "missing defect acceptance gates")?;
    let gates = read(&path)?;
    let rows = array(&gates["records"]).map_err(|_| "invalid defect acceptance gates")?;
    if supporting {
        return Ok(());
    }
    let empty = vec![];
    let prior = gates["accepted_prior_primary_receipts"]
        .as_array()
        .unwrap_or(&empty);
    let required: Vec<_> = rows
        .iter()
        .filter(|r| {
            r["primary_task"] == id
                && r["inventory_evidence_kind"] == "source_only_inventory"
                && !prior.iter().any(|p| {
                    p["defect_id"] == r["id"] && p["task_id"] == id && p["receipt"] == *reference
                })
        })
        .collect();
    if required.is_empty() {
        return Ok(());
    }
    let checks = receipt["defect_controls"].as_array().unwrap_or(&empty);
    let ids: BTreeSet<_> = checks.iter().map(|c| c["defect_id"].to_string()).collect();
    ensure(
        ids.len() == checks.len() && required.iter().all(|r| ids.contains(&r["id"].to_string())),
        format!("{id}: missing/duplicate executed defect controls"),
    )?;
    let inventory = read(&root.join(text(&gates["inventory_path"])?))?;
    for row in required {
        let c = checks
            .iter()
            .find(|c| c["defect_id"] == row["id"])
            .ok_or("missing control")?;
        let fixture = &c["fixture"];
        ensure(
            keys(fixture, &["path", "sha256"])
                && array(&receipt["artifacts"])?.contains(fixture)
                && sha(&fixture["sha256"]),
            format!("{id}: defect fixture must bind a receipt artifact"),
        )?;
        let path = inside(root, &fixture["path"])?;
        ensure(
            path.is_file()
                && digest(&fs::read(&path).map_err(|e| e.to_string())?) == fixture["sha256"]
                && tracked(root, text(&fixture["path"])?),
            format!("{id}: stale defect fixture"),
        )?;
        let original = array(&inventory["defects"])?
            .iter()
            .find(|r| r["id"] == row["id"])
            .ok_or("missing original defect")?;
        let source = &c["source"];
        ensure(
            source["commit"] == gates["frozen_source_commit"]
                && array(&original["sources"])?.iter().any(|s| {
                    source["path"] == s["path"]
                        && source["symbol"] == s["symbol"]
                        && source["sha256"] == s["source_sha256"]
                }),
            format!("{id}: defect source binding drift"),
        )?;
        for kind in ["positive", "red"] {
            let command = &c[kind];
            let code = command["exit_code"].as_i64();
            ensure(
                command["argv"].as_array().is_some_and(|a| !a.is_empty())
                    && sha(&command["log_sha256"])
                    && command["expected_result"]
                        .as_str()
                        .is_some_and(|s| !s.trim().is_empty())
                    && code.is_some_and(|c| if kind == "positive" { c == 0 } else { c != 0 }),
                format!("{id}: invalid executed {kind} defect control"),
            )?;
            if kind == "positive" {
                ensure(
                    array(&receipt["commands"])?.iter().any(|c| {
                        ["argv", "exit_code", "log_sha256"]
                            .iter()
                            .all(|k| c[k] == command[k])
                    }),
                    format!("{id}: positive defect command not executed in receipt"),
                )?;
            }
        }
    }
    Ok(())
}
fn receipt(root: &Path, reference: &Value, id: &str, status: &str) -> Result<()> {
    let supporting = reference["purpose"] == "integration_only";
    ensure(
        keys(
            reference,
            if supporting {
                &["path", "sha256", "purpose"]
            } else {
                &["path", "sha256"]
            },
        ) && sha(&reference["sha256"]),
        format!("{id}: invalid receipt reference"),
    )?;
    let path = inside(root, &reference["path"])?;
    ensure(
        path.is_file()
            && digest(&fs::read(&path).map_err(|e| e.to_string())?) == reference["sha256"],
        format!("{id}: missing or stale receipt {}", path.display()),
    )?;
    let r = read(&path)?;
    if supporting {
        ensure(
            status == "blocked",
            format!("{id}: integration-only evidence is allowed only blocked"),
        )?;
        ensure(
            r["packet_acceptance"] == false && r["scientific_admission"] == false,
            format!("{id}: supporting evidence must disclaim packet/scientific admission"),
        )?;
    }
    ensure(
        [
            "task_ids",
            "tested_commit",
            "changed_paths",
            "commands",
            "positive_checks",
            "negative_checks",
            "artifacts",
            "resource_allocation",
            "limitations",
            "next_dependency",
        ]
        .iter()
        .all(|k| r.get(k).is_some())
            && array(&r["task_ids"])?.contains(&Value::from(id)),
        format!("{id}: incomplete receipt"),
    )?;
    ensure(
        hex(&r["tested_commit"], 40),
        format!("{id}: invalid tested commit"),
    )?;
    ensure(
        [
            "changed_paths",
            "commands",
            "positive_checks",
            "negative_checks",
        ]
        .iter()
        .all(|k| truth(&r[k])),
        format!("{id}: receipt lacks checks or paths"),
    )?;
    for name in array(&r["changed_paths"])? {
        inside(root, name)?;
    }
    for c in array(&r["commands"])? {
        ensure(
            c["argv"].as_array().is_some_and(|a| !a.is_empty()) && c["exit_code"] == 0,
            format!("{id}: incomplete or failed command in completion receipt"),
        )?;
        ensure(sha(&c["log_sha256"]), format!("{id}: missing log hash"))?;
    }
    ensure(
        r["artifacts"]
            .as_array()
            .is_some_and(|a| !a.is_empty() && a.iter().all(|a| sha(&a["sha256"]))),
        format!("{id}: missing artifact hashes"),
    )?;
    controls(root, reference, id, &r, supporting)
}
fn done(v: &Value) -> bool {
    v["status"] == "done"
        || (v["status"] == "in_progress"
            && v["note"].as_str().is_some_and(|s| s.contains("SC-DEC-071")))
}
fn walk<'a>(
    id: &'a str,
    packets: &'a [Value],
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
) -> Result<()> {
    ensure(!visiting.contains(id), format!("dependency cycle: {id}"))?;
    if visited.contains(id) {
        return Ok(());
    }
    visiting.insert(id);
    let packet = packets
        .iter()
        .find(|p| p["id"] == id)
        .ok_or("unknown packet")?;
    for dep in array(&packet["dependencies"])? {
        walk(text(dep)?, packets, visiting, visited)?;
    }
    visiting.remove(id);
    visited.insert(id);
    Ok(())
}

#[path = "plan_validation/registry.rs"]
mod registry;
pub use registry::validate;
