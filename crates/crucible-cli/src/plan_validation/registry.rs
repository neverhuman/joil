//! Plan registry (`TASKS.json`, `progress/*.json`, `SOURCE_MANIFEST.json`, `CATALOGUE_MAP.json`, `DEFECT_MAP.json`) validator.

use super::*;

pub fn validate(root: &Path) -> Result<()> {
    let plan = root.join("docs/v5-plan");
    let data = read(&plan.join("TASKS.json"))?;
    ensure(
        data["schema_version"] == 1 && data["packets"].as_array().is_some_and(|a| a.len() == 32),
        "expected v1 registry with 32 packets",
    )?;
    let packets = array(&data["packets"])?;
    let ids: BTreeSet<_> = packets
        .iter()
        .map(|p| text(&p["id"]))
        .collect::<Result<_>>()?;
    ensure(ids.len() == packets.len(), "duplicate packet ID")?;
    let mut tasks = vec![];
    let mut progress = Map::new();
    for p in packets {
        let id = text(&p["id"])?;
        ensure(
            id.len() == 5
                && id.starts_with("V5-")
                && id[3..].bytes().all(|b| b.is_ascii_digit())
                && truth(&p["write_scope"])
                && truth(&p["read_first"]),
            format!("incomplete packet: {id}"),
        )?;
        for key in [
            "owner_role",
            "edge_cases",
            "validation_commands",
            "legacy_packets",
        ] {
            ensure(truth(&p[key]), format!("{id}: missing {key}"))?;
        }
        let deps = array(&p["dependencies"])?;
        let dep_ids: BTreeSet<_> = deps.iter().map(text).collect::<Result<_>>()?;
        ensure(
            dep_ids.len() == deps.len(),
            format!("{id}: duplicate dependency"),
        )?;
        ensure(
            dep_ids.iter().all(|d| ids.contains(d) && *d != id),
            format!("{id}: invalid dependency"),
        )?;
        for path in array(&p["read_first"])?
            .iter()
            .chain(array(&p["write_scope"])?)
        {
            inside(root, path)?;
        }
        ensure(
            !array(&p["write_scope"])?.iter().any(|v| {
                v.as_str().is_some_and(|s| {
                    [".private/", "runs/", "artifacts/"]
                        .iter()
                        .any(|prefix| s.starts_with(prefix))
                })
            }),
            format!("{id}: public write scope includes private output"),
        )?;
        let state = read(&plan.join(format!("progress/{id}.json")))?;
        ensure(
            keys(&state, &["schema_version", "packet", "tasks"])
                && state["schema_version"] == 1
                && state["packet"] == id,
            format!("{id}: invalid progress schema"),
        )?;
        let task_rows = array(&p["tasks"])?;
        let task_ids: BTreeSet<_> = task_rows
            .iter()
            .map(|t| text(&t["id"]))
            .collect::<Result<_>>()?;
        ensure(
            task_ids
                == object(&state["tasks"])?
                    .keys()
                    .map(String::as_str)
                    .collect(),
            format!("{id}: progress task mismatch"),
        )?;
        for t in task_rows {
            let tid = text(&t["id"])?;
            ensure(
                tid.len() == 8
                    && tid.starts_with(&format!("{id}-"))
                    && tid[6..].bytes().all(|b| b.is_ascii_digit())
                    && ["title", "instructions", "acceptance", "negative_control"]
                        .iter()
                        .all(|k| truth(&t[k])),
                "incomplete task",
            )?;
            let s = &state["tasks"][tid];
            ensure(
                keys(s, &["status", "owner", "note", "evidence"])
                    && s["status"].as_str().is_some_and(|s| STATES.contains(&s)),
                format!("{tid}: invalid state"),
            )?;
            ensure(
                s["owner"].is_null() || s["owner"].is_string(),
                "owner must be text or null",
            )?;
            ensure(
                s["note"].is_string() && s["evidence"].is_array(),
                "invalid note/evidence",
            )?;
            let status = text(&s["status"])?;
            ensure(
                !["in_progress", "review", "done"].contains(&status) || truth(&s["owner"]),
                format!("{tid}: active/completed task needs an owner"),
            )?;
            ensure(
                status != "blocked" || !text(&s["note"])?.trim().is_empty(),
                format!("{tid}: blocker needs a reason"),
            )?;
            ensure(
                status != "done" || truth(&s["evidence"]),
                format!("{tid}: done needs a completion receipt"),
            )?;
            for r in array(&s["evidence"])? {
                receipt(root, r, tid, status)?;
            }
            tasks.push(tid);
        }
        progress.insert(id.into(), state);
    }
    ensure(
        tasks.len() == 160 && tasks.iter().collect::<BTreeSet<_>>().len() == 160,
        "expected 160 unique tasks",
    )?;
    for p in packets {
        walk(
            text(&p["id"])?,
            packets,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
        )?;
    }
    for p in packets {
        let mut prior = true;
        let mut deps_done = true;
        for dep in array(&p["dependencies"])? {
            deps_done &= object(&progress[text(dep)?]["tasks"])?.values().all(done);
        }
        for t in array(&p["tasks"])? {
            let tid = text(&t["id"])?;
            let row = &progress[text(&p["id"])?]["tasks"][tid];
            ensure(
                !["in_progress", "review", "done"].contains(&text(&row["status"])?)
                    || (prior && deps_done),
                format!("{tid}: dependencies or earlier ordered task incomplete"),
            )?;
            prior &= done(row);
        }
    }
    let manifest = read(&plan.join("SOURCE_MANIFEST.json"))?;
    ensure(
        manifest["source_count"] == 10 && array(&manifest["sources"])?.len() == 10,
        "expected ten source files",
    )?;
    ensure(
        array(&manifest["sources"])?
            .iter()
            .all(|s| sha(&s["sha256"])),
        "invalid source digest",
    )?;
    let catalogue = read(&root.join("docs/TEST_CATALOG.json"))?;
    let mapping = read(&plan.join("CATALOGUE_MAP.json"))?;
    let cat = array(&catalogue["families"])?;
    let map = array(&mapping["families"])?;
    ensure(
        map.len() == cat.len()
            && map
                .iter()
                .map(|r| r["id"].to_string())
                .collect::<BTreeSet<_>>()
                == cat.iter().map(|r| r["id"].to_string()).collect(),
        "catalogue coverage mismatch",
    )?;
    for f in map {
        ensure(
            truth(&f["recipe"]) && ids.contains(text(&f["packet"])?),
            "invalid catalogue mapping",
        )?;
    }
    let defects = read(&plan.join("DEFECT_MAP.json"))?;
    let defects = array(&defects["defects"])?;
    ensure(
        defects
            .iter()
            .map(|d| text(&d["id"]).map(str::to_owned))
            .collect::<Result<BTreeSet<_>>>()?
            == (1..32).map(|i| format!("D-{i:02}")).collect(),
        "defect coverage mismatch",
    )?;
    ensure(
        defects.iter().all(|d| {
            d["primary_task"]
                .as_str()
                .is_some_and(|s| tasks.contains(&s))
        }),
        "unknown defect task",
    )
}
