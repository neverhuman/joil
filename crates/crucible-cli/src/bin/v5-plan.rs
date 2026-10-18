//! Native planning views. Completion is not scientific qualification.
#[allow(dead_code)] // The shared validator also exports explorer-only pretty encoding.
#[path = "../plan_validation.rs"]
mod plan_validation;
use plan_validation::{digest, read, validate, Result};
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

// Keep input insertion order for Python-compatible embedded dashboard bytes.
#[derive(Clone)]
enum Ordered {
    Object(Vec<(String, Ordered)>),
    Array(Vec<Ordered>),
    Scalar(Value),
}
impl<'de> Deserialize<'de> for Ordered {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Ordered;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON")
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Ordered, A::Error> {
                let mut v = vec![];
                while let Some(p) = a.next_entry()? {
                    v.push(p)
                }
                Ok(Ordered::Object(v))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Ordered, A::Error> {
                let mut v = vec![];
                while let Some(p) = a.next_element()? {
                    v.push(p)
                }
                Ok(Ordered::Array(v))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(v.into()))
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(Value::Null))
            }
        }
        d.deserialize_any(V)
    }
}
impl Ordered {
    fn parse(path: &Path) -> Result<Self> {
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }
    fn field(&mut self, k: &str) -> Result<&mut Self> {
        if let Self::Object(m) = self {
            m.iter_mut()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v)
                .ok_or(format!("'{k}'"))
        } else {
            Err("expected object".into())
        }
    }
    fn array(&mut self) -> Result<&mut Vec<Self>> {
        if let Self::Array(a) = self {
            Ok(a)
        } else {
            Err("expected array".into())
        }
    }
    fn put(&mut self, k: String, v: Self) -> Result<()> {
        if let Self::Object(m) = self {
            if let Some((_, old)) = m.iter_mut().find(|(n, _)| n == &k) {
                *old = v
            } else {
                m.push((k, v))
            }
            Ok(())
        } else {
            Err("expected object".into())
        }
    }
    fn pretty(&self, depth: usize) -> String {
        let indent = "  ".repeat(depth + 1);
        let (open, close, rows) = match self {
            Self::Scalar(v) => return v.to_string(),
            Self::Array(a) => (
                "[",
                "]",
                a.iter().map(|v| v.pretty(depth + 1)).collect::<Vec<_>>(),
            ),
            Self::Object(m) => (
                "{",
                "}",
                m.iter()
                    .map(|(k, v)| format!("{}: {}", Value::from(k.clone()), v.pretty(depth + 1)))
                    .collect(),
            ),
        };
        if rows.is_empty() {
            format!("{open}{close}")
        } else {
            format!(
                "{open}\n{indent}{}\n{}{close}",
                rows.join(&format!(",\n{indent}")),
                "  ".repeat(depth)
            )
        }
    }
    fn encode(&self, sorted: bool) -> String {
        let sep = if sorted { ", " } else { "," };
        match self {
            Self::Scalar(v) => v.to_string(),
            Self::Array(a) => format!(
                "[{}]",
                a.iter()
                    .map(|v| v.encode(sorted))
                    .collect::<Vec<_>>()
                    .join(sep)
            ),
            Self::Object(m) => {
                let mut entries = m.iter().collect::<Vec<_>>();
                if sorted {
                    entries.sort_by(|a, b| a.0.cmp(&b.0));
                }
                format!(
                    "{{{}}}",
                    entries
                        .iter()
                        .map(|(k, v)| format!(
                            "{}{}{}",
                            Value::from(k.clone()),
                            if sorted { ": " } else { ":" },
                            v.encode(sorted)
                        ))
                        .collect::<Vec<_>>()
                        .join(sep)
                )
            }
        }
    }
}
fn s(v: &Value) -> Result<&str> {
    v.as_str().ok_or("expected text".into())
}
fn a(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("expected array".into())
}
fn join(v: &Value) -> Result<String> {
    Ok(a(v)?.iter().map(s).collect::<Result<Vec<_>>>()?.join(", "))
}
fn fallback<'a>(v: &'a Value, default: &'a str) -> Result<&'a str> {
    if v.is_null() || v == "" {
        Ok(default)
    } else {
        s(v)
    }
}
fn enriched(root: &Path) -> Result<(Value, String)> {
    validate(root)?;
    let base = root.join("docs/v5-plan");
    let mut ordered = Ordered::parse(&base.join("TASKS.json"))?;
    let defects = read(&base.join("DEFECT_MAP.json"))?;
    for p in ordered.field("packets")?.array()? {
        let v: Value = serde_json::from_str(&p.encode(false)).map_err(|e| e.to_string())?;
        let mut progress = Ordered::parse(&base.join(format!("progress/{}.json", s(&v["id"])?)))?;
        for t in p.field("tasks")?.array()? {
            let tv: Value = serde_json::from_str(&t.encode(false)).map_err(|e| e.to_string())?;
            let id = s(&tv["id"])?;
            if let Ordered::Object(fields) = progress.field("tasks")?.field(id)? {
                for (k, val) in fields {
                    t.put(k.clone(), val.clone())?;
                }
            }
            let ids = a(&defects["defects"])?
                .iter()
                .filter(|d| d["primary_task"] == id)
                .map(|d| Ordered::Scalar(d["id"].clone()))
                .collect();
            t.put("defect_ids".into(), Ordered::Array(ids))?;
        }
    }
    let hash = digest(ordered.encode(true).as_bytes());
    ordered.put("snapshot_sha256".into(), Ordered::Scalar(hash.into()))?;
    let encoded = ordered.encode(false);
    Ok((
        serde_json::from_str(&encoded).map_err(|e| e.to_string())?,
        encoded,
    ))
}

fn run(root: &Path, check: bool) -> Result<usize> {
    let (data, embedded) = enriched(root)?;
    let mut files = vec![];
    let done_count = a(&data["packets"])?
        .iter()
        .flat_map(|p| p["tasks"].as_array().into_iter().flatten())
        .filter(|t| t["status"] == "done")
        .count();
    let mut lines = Vec::<String>::new();
    lines.push("# Joil v5 implementation ToDos".to_string());
    lines.push("".to_string());
    lines.push("Generated from `TASKS.json` and packet-owned `progress/*.json`. Run `rtk proxy python3 tools/v5_plan.py build` after a progress commit.".to_string());
    lines.push("".to_string());
    lines.push(format!("**Planning is prepared; engineering implementation progress is {}/160. Scientific qualification remains governed by `docs/STATUS.json`.**", done_count));
    lines.push("".to_string());
    lines.push("| Packet | Milestone | Owner role | Complete | Dependencies |".to_string());
    lines.push("| --- | --- | --- | ---: | --- |".to_string());
    for p in a(&data["packets"])? {
        let complete = a(&p["tasks"])?
            .iter()
            .filter(|t| t["status"] == "done")
            .count();
        lines.push(format!(
            "| [{} — {}](todos/{}.md) | {} | {} | {}/{} | {} |",
            s(&p["id"])?,
            s(&p["title"])?,
            s(&p["id"])?,
            s(&p["milestone"])?,
            s(&p["owner_role"])?,
            complete,
            a(&p["tasks"])?.len(),
            {
                let value = join(&p["dependencies"])?;
                if value.is_empty() {
                    "none".to_string()
                } else {
                    value
                }
            }
        ));
        let mut body = Vec::<String>::new();
        body.push(format!("# {} — {}", s(&p["id"])?, s(&p["title"])?));
        body.push("".to_string());
        body.push(format!(
            "Milestone: {}. Parent packets: {}. Owner role: {}.",
            s(&p["milestone"])?,
            join(&p["legacy_packets"])?,
            s(&p["owner_role"])?
        ));
        body.push("".to_string());
        body.push(format!("Dependencies: {}. Completion means accepted packet evidence; scientific admission is separate.", { let value=join(&p["dependencies"])?; if value.is_empty() {"none".to_string()} else {value} }));
        body.push("".to_string());
        body.push("## Objective and boundary".to_string());
        body.push("".to_string());
        body.push(s(&p["title"])?.to_string() + ". Execute the five ordered tasks below. Read other packets as dependencies; write only the listed scope. Root manifests, schemas, requirements and generated clients require the integrator.");
        body.push("".to_string());
        body.push("## Ordered read-first paths".to_string());
        body.push("".to_string());
        for (i, name) in a(&p["read_first"])?.iter().enumerate() {
            body.push(format!("{}. `{}`", (i + 1), s(name)?));
        }
        body.push("".to_string());
        body.push("Read the global ordered reads in `../ENGINEERING_WORKPLAN.md` first. New paths are planned contracts: wait for their dependency commit before implementation.".to_string());
        body.push("".to_string());
        body.push("## Exclusive write scope".to_string());
        body.push("".to_string());
        for name in a(&p["write_scope"])? {
            body.push(format!("- `{}`", s(name)?));
        }
        body.push("".to_string());
        body.push("Forbidden: another project, unowned shared files, frozen raw inputs/receipts and private artifacts in Git. Generated views are rebuilt by the integrator. Concurrent writes to parent/child scopes require an explicit path split.".to_string());
        body.push("".to_string());
        body.push("## Ordered tasks".to_string());
        body.push("".to_string());
        for t in a(&p["tasks"])? {
            body.push(format!(
                "### {} {} — {}",
                if t["status"] == "done" { "[x]" } else { "[ ]" },
                s(&t["id"])?,
                s(&t["title"])?
            ));
            body.push("".to_string());
            body.push(format!(
                "Status: **{}**. Assigned owner: {}.",
                s(&t["status"])?,
                fallback(&t["owner"], "unassigned")?
            ));
            body.push("".to_string());
            body.push(s(&t["instructions"])?.to_string());
            body.push("".to_string());
            body.push(format!("Acceptance: {}", s(&t["acceptance"])?));
            body.push("".to_string());
            body.push(format!("Negative control: {}", s(&t["negative_control"])?));
            body.push("".to_string());
            body.push(format!(
                "Progress note: {}",
                fallback(&t["note"], "No note recorded.")?
            ));
            body.push("".to_string());
            if !a(&t["evidence"])?.is_empty() {
                for r in a(&t["evidence"])? {
                    body.push(format!(
                        "Evidence: `{}` SHA-256 `{}`.",
                        s(&r["path"])?,
                        s(&r["sha256"])?
                    ));
                }
                body.push("".to_string());
            }
        }
        body.push("## Hard parts and edge cases".to_string());
        body.push("".to_string());
        for v in a(&p["edge_cases"])? {
            body.push(format!("- {}.", s(v)?.trim()));
        }
        body.push("".to_string());
        body.push("## Validation commands and expected artifacts".to_string());
        body.push("".to_string());
        body.push("Commands below are future packet acceptance commands. The target crate, test or CLI subcommand must be implemented and registered before use. Run from `/home/ubuntu/joil` in the assigned branch/checkout, with a current explicit resource lease for native/large tests.".to_string());
        body.push("".to_string());
        body.push("```sh".to_string());
        body.extend(
            a(&p["validation_commands"])?
                .iter()
                .map(|v| s(v).map(str::to_string))
                .collect::<Result<Vec<_>>>()?,
        );
        body.push("```".to_string());
        body.push("".to_string());
        body.push("Expected artifacts: tested source commit; source-bound independent positive and negative fixture results; exact command arrays/exit codes; raw-log and result SHA-256; packet completion receipt; explicit scientific eligibility and remaining limits. Raw outputs stay private. A command failure blocks this packet; missing physical inputs block their dependent claims, not unrelated software work.".to_string());
        body.push("".to_string());
        body.push("## Stop and handoff".to_string());
        body.push("".to_string());
        body.push("Stop on ownership conflict, incompatible identity/units, missing mandatory evidence, failed numerical/model budget, stale lease or resource pressure. Preserve diagnostics; do not relax a threshold or replace a failed subject. Append material progress to root COORD.md and this packet’s progress record.".to_string());
        body.push("".to_string());
        body.push(format!("Update `../progress/{}.json` through `tools/v5_plan.py update`. A completed task requires a hashed receipt with tested commit, changed paths, exact commands, log/artifact digests, positive/negative checks, resources, limitations and next dependency. `done` is task completion and never scientific PASS. Integrator regenerates ToDos/dashboard and accepts the packet before downstream work.", s(&p["id"])?));
        body.push("".to_string());
        files.push((
            format!("docs/v5-plan/todos/{}.md", s(&p["id"])?),
            body.join("\n"),
        ));
    }
    lines.push("".to_string());
    lines.push("## Progress commands".to_string());
    lines.push("".to_string());
    lines.push("```sh".to_string());
    lines.push("rtk proxy python3 tools/v5_plan.py update V5-00-01 --status in_progress --owner NAME --note \"Baseline review started\"".to_string());
    lines.push("rtk proxy python3 tools/v5_plan.py update V5-00-01 --status done --owner NAME --receipt docs/v5-plan/receipts/UNIQUE.json".to_string());
    lines.push("rtk proxy python3 tools/v5_plan.py build".to_string());
    lines.push("rtk proxy python3 tools/v5_plan.py check".to_string());
    lines.push("```".to_string());
    lines.push("".to_string());
    lines.push("Only use `done` after the receipt is real and reviewed. Browser draft marks and notes stay local and never update this progress source.".to_string());
    lines.push("".to_string());
    files.push(("docs/v5-plan/TODO.md".into(), lines.join("\n")));
    let template =
        fs::read_to_string(root.join("tools/v5_dashboard.html")).map_err(|e| e.to_string())?;
    if template.matches("__PLAN_DATA__").count() != 1 {
        return Err("dashboard template needs exactly one data marker".into());
    }
    let embedded = embedded
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    files.push((
        "docs/v5-plan/dashboard.html".into(),
        template.replace("__PLAN_DATA__", &embedded),
    ));
    let mut stale = vec![];
    for (name, content) in files {
        let path = root.join(&name);
        if check {
            if !path.is_file()
                || fs::read_to_string(&path)
                    .map_err(|e| e.to_string())?
                    .replace("\r\n", "\n")
                    != content
            {
                stale.push(name);
            }
        } else {
            fs::create_dir_all(path.parent().ok_or("missing parent")?)
                .map_err(|e| e.to_string())?;
            fs::write(path, content).map_err(|e| e.to_string())?;
        }
    }
    if !stale.is_empty() {
        return Err(format!("stale generated views: {}", stale.join(", ")));
    }
    Ok(done_count)
}
fn update(root: &Path, args: &[String]) -> Result<()> {
    let id = args.first().ok_or("missing task")?;
    let mut options = std::collections::BTreeMap::new();
    for pair in args[1..].chunks(2) {
        if pair.len() != 2
            || ![
                "--status",
                "--owner",
                "--note",
                "--receipt",
                "--supporting-receipt",
            ]
            .contains(&pair[0].as_str())
        {
            return Err("invalid update arguments".into());
        }
        options.insert(pair[0].as_str(), pair[1].as_str());
    }
    let status = *options.get("--status").ok_or("missing --status")?;
    if !plan_validation::STATES.contains(&status)
        || options.contains_key("--receipt") && options.contains_key("--supporting-receipt")
    {
        return Err("invalid update arguments".into());
    }
    validate(root)?;
    let data = read(&root.join("docs/v5-plan/TASKS.json"))?;
    let packet = a(&data["packets"])?
        .iter()
        .find(|p| {
            p["tasks"]
                .as_array()
                .is_some_and(|ts| ts.iter().any(|t| t["id"] == *id))
        })
        .ok_or(format!("unknown task: {id}"))?;
    let path = root.join(format!("docs/v5-plan/progress/{}.json", s(&packet["id"])?));
    let old = fs::read(&path).map_err(|e| e.to_string())?;
    let mut state = Ordered::parse(&path)?;
    let row = state.field("tasks")?.field(id)?;
    row.put("status".into(), Ordered::Scalar(status.into()))?;
    for (flag, key) in [("--owner", "owner"), ("--note", "note")] {
        if let Some(v) = options.get(flag) {
            row.put(key.into(), Ordered::Scalar((*v).into()))?;
        }
    }
    if let Some(name) = options
        .get("--receipt")
        .or_else(|| options.get("--supporting-receipt"))
    {
        if Path::new(name).is_absolute() {
            return Err(format!("expected relative path: {name}"));
        }
        let receipt = fs::canonicalize(root.join(name)).map_err(|e| e.to_string())?;
        if !receipt.starts_with(root) {
            return Err(format!("path escapes root: {name}"));
        }
        let mut fields = vec![
            (
                "path".into(),
                Ordered::Scalar(
                    receipt
                        .strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .to_string()
                        .into(),
                ),
            ),
            (
                "sha256".into(),
                Ordered::Scalar(digest(&fs::read(receipt).map_err(|e| e.to_string())?).into()),
            ),
        ];
        if options.contains_key("--supporting-receipt") {
            fields.push(("purpose".into(), Ordered::Scalar("integration_only".into())));
        }
        row.put(
            "evidence".into(),
            Ordered::Array(vec![Ordered::Object(fields)]),
        )?;
    }
    fs::write(&path, format!("{}\n", state.pretty(0))).map_err(|e| e.to_string())?;
    if let Err(e) = validate(root) {
        fs::write(path, old).map_err(|restore| format!("{e}; rollback failed: {restore}"))?;
        return Err(e);
    }
    Ok(())
}
fn main() {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if args.len() >= 2 && args[args.len() - 2] == "--root" {
        root = PathBuf::from(args.pop().unwrap());
        args.pop();
    }
    let Some(command) = args.first() else {
        eprintln!("usage: v5-plan {{build,check,update}} [--root ROOT]");
        std::process::exit(2)
    };
    if !["build", "check", "update"].contains(&command.as_str())
        || (command != "update" && args.len() != 1)
    {
        eprintln!("invalid command arguments");
        std::process::exit(2)
    }
    let result=fs::canonicalize(root).map_err(|e|e.to_string()).and_then(|r|{
 if command=="update"{update(&r,&args[1..])?;println!("Progress source updated; integrator must run build and review the generated diff.");}else{let done=run(&r,command=="check")?;println!("PASS: 32 packets, 160 tasks, {done} completed; 10 sources, 31 defects, catalogue coverage; views {command} OK");}Ok(())});
    if let Err(e) = result {
        eprintln!("FAIL: {e}");
        std::process::exit(1)
    }
}
