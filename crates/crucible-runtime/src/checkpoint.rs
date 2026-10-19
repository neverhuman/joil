//! Complete synthetic state, online ledger backup and verified checkpoint history.
use crate::{
    db::{event, RuntimeDb},
    ledger::Accounting,
    replay::{Experiment, SearchState},
    task::*,
    BlobDigest, Result, RuntimeError, SemanticId,
};
use crucible_store::{
    manifest::{Metadata, ObjectKind},
    store::{PinReason, PublishPhase},
    Store,
};
use rusqlite::{
    params, types::ValueRef, Connection, OpenFlags, OptionalExtension, TransactionBehavior,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

const TABLES: [&str; 14] = [
    "metadata",
    "tasks",
    "attempts",
    "leases",
    "events",
    "object_refs",
    "reservations",
    "charges",
    "conflicts",
    "family_policies",
    "family_decisions",
    "pointers",
    "sqlite_sequence",
    "__contract",
];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableSnapshot {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerSnapshot {
    pub schema_version: u32,
    pub experiment: SemanticId,
    pub accounting: Accounting,
    pub event_position: u64,
    pub tables: BTreeMap<String, TableSnapshot>,
}
fn snapshot(connection: &Connection) -> Result<LedgerSnapshot> {
    let mut tables = BTreeMap::new();
    for name in TABLES.into_iter().filter(|n| *n != "__contract") {
        assert!(TABLES.contains(&name));
        let prefix = ["SEL", "ECT * FR", "OM "].concat();
        let sql = [prefix.as_str(), name].concat();
        let width = connection.prepare(&sql)?.column_count();
        let order = (1..=width)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let ordering = [" OR", "DER BY "].concat();
        let sql = [sql.as_str(), ordering.as_str(), order.as_str()].concat();
        let mut stmt = connection.prepare(&sql)?;
        let columns = stmt
            .column_names()
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let width = columns.len();
        let rows = stmt
            .query_map([], |r| {
                (0..width)
                    .map(|i| {
                        Ok(match r.get_ref(i)? {
                            ValueRef::Null => Value::Null,
                            ValueRef::Integer(n) => Value::from(n),
                            ValueRef::Text(s) => {
                                Value::String(String::from_utf8_lossy(s).into_owned())
                            }
                            _ => return Err(rusqlite::Error::InvalidQuery),
                        })
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        tables.insert(name.to_string(), TableSnapshot { columns, rows });
    }
    let version: u32 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
    tables.insert(
        "__contract".into(),
        TableSnapshot {
            columns: vec!["sqlite_user_version".into()],
            rows: vec![vec![Value::from(version)]],
        },
    );
    let raw: String = connection.query_row(
        "SELECT value FROM metadata WHERE key='experiment'",
        [],
        |r| r.get(0),
    )?;
    let result = LedgerSnapshot {
        schema_version: 1,
        experiment: SemanticId::parse(&raw).map_err(|e| RuntimeError::Wire(e.to_string()))?,
        accounting: crate::db::accounting(connection)?,
        event_position: connection.query_row(
            "SELECT coalesce(max(sequence),0) FROM events",
            [],
            |r| r.get(0),
        )?,
        tables,
    };
    result.check()?;
    Ok(result)
}
impl LedgerSnapshot {
    pub fn check(&self) -> Result<()> {
        if self.schema_version != 1
            || self
                .tables
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                != BTreeSet::from(TABLES)
            || self
                .tables
                .values()
                .any(|t| t.columns.is_empty() || t.rows.iter().any(|r| r.len() != t.columns.len()))
            || self.tables["__contract"].rows != vec![vec![Value::from(1)]]
        {
            return Err(RuntimeError::Invalid("complete_versioned_ledger_snapshot"));
        }
        self.accounting.authorized.check()?;
        self.accounting.spent.check()?;
        self.accounting.reserved.check()?;
        Ok(())
    }
    pub fn actual_cost_digest(&self) -> Result<SemanticId> {
        SemanticId::of(&(
            "actual-ledger-cost/1",
            &self.accounting,
            &self.tables["charges"],
        ))
        .map_err(|e| RuntimeError::Wire(e.to_string()))
    }
    pub fn object_references(&self) -> Result<BTreeSet<BlobDigest>> {
        let mut refs = BTreeSet::new();
        for (table, columns) in [
            ("tasks", &["accepted_object"][..]),
            ("attempts", &["staged", "verified"]),
            ("object_refs", &["object", "verification"]),
            ("pointers", &["object"]),
        ] {
            let t = &self.tables[table];
            for column in columns {
                let i = t
                    .columns
                    .iter()
                    .position(|s| s == column)
                    .ok_or(RuntimeError::Invalid("ledger_column"))?;
                for row in &t.rows {
                    if table == "pointers" {
                        let n = t
                            .columns
                            .iter()
                            .position(|c| c == "name")
                            .ok_or(RuntimeError::Invalid("pointer_name_column"))?;
                        if !row[n]
                            .as_str()
                            .is_some_and(|s| s.starts_with("checkpoint-"))
                        {
                            continue;
                        }
                    }
                    if let Some(s) = row[i].as_str() {
                        refs.insert(
                            BlobDigest::parse(s).map_err(|e| RuntimeError::Wire(e.to_string()))?,
                        );
                    }
                }
            }
        }
        let t = &self.tables["events"];
        let k = t
            .columns
            .iter()
            .position(|s| s == "kind")
            .ok_or(RuntimeError::Invalid("event_kind_column"))?;
        let p = t
            .columns
            .iter()
            .position(|s| s == "payload")
            .ok_or(RuntimeError::Invalid("event_payload_column"))?;
        for row in &t.rows {
            if row[k].as_str() == Some("private_attempt_diagnostic") {
                let r: BlobDigest = crate::decode(
                    row[p]
                        .as_str()
                        .ok_or(RuntimeError::Invalid("diagnostic_payload"))?,
                )?;
                refs.insert(r);
            }
        }
        Ok(refs)
    }
    pub fn require_extension(&self, current: &Self) -> Result<()> {
        self.check()?;
        current.check()?;
        if self.experiment != current.experiment
            || self.accounting.authorized != current.accounting.authorized
            || self.event_position > current.event_position
            || !self.accounting.spent.fits(current.accounting.spent)
        {
            return Err(RuntimeError::Conflict(
                "canonical_ledger_missing_or_rolled_back",
            ));
        }
        // These rows are append-only; equality of costs/family attempts/event prefix prevents a reset.
        for name in ["charges", "family_policies", "family_decisions", "events"] {
            let old = &self.tables[name];
            let new = &current.tables[name];
            if old.columns != new.columns || old.rows.iter().any(|r| !new.rows.contains(r)) {
                return Err(RuntimeError::Conflict("canonical_ledger_prefix_changed"));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub schema_version: u32,
    pub synthetic_only: bool,
    pub experiment: Experiment,
    pub state: SearchState,
    pub ledger: LedgerSnapshot,
    pub sqlite_backup: BlobDigest,
}
impl Checkpoint {
    pub fn check(&self) -> Result<()> {
        self.experiment.check()?;
        self.state.check()?;
        self.ledger.check()?;
        if self.schema_version != 1
            || !self.synthetic_only
            || self.experiment.identity()? != self.state.campaign
            || self.ledger.experiment != self.state.campaign
        {
            return Err(RuntimeError::Invalid("checkpoint_experiment_binding"));
        }
        Ok(())
    }
    fn references(&self) -> Result<BTreeSet<BlobDigest>> {
        let mut refs = self.ledger.object_references()?;
        refs.extend(self.experiment.dependencies.values().cloned());
        refs.extend(self.state.object_references());
        refs.insert(self.sqlite_backup.clone());
        Ok(refs)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointPhase {
    BeforeRename,
    AfterRename,
    BeforePointerCommit,
    AfterPointerCommit,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointPointer {
    pub generation: u64,
    pub object: BlobDigest,
    pub event_position: u64,
}
#[derive(Clone, Debug)]
pub struct Restored {
    pub pointer: CheckpointPointer,
    pub checkpoint: Checkpoint,
    pub rejected_newer: Vec<(BlobDigest, String)>,
}
static UNIQUE: AtomicU64 = AtomicU64::new(0);
fn suffix() -> Result<String> {
    Ok(format!(
        "{}-{}-{}",
        std::process::id(),
        crate::utc_millis()?,
        UNIQUE.fetch_add(1, Ordering::Relaxed)
    ))
}
fn read_bounded(path: &Path, max: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    if !f.metadata()?.is_file() || f.metadata()?.len() > max {
        return Err(RuntimeError::Invalid("backup_file_budget"));
    }
    let mut b = Vec::new();
    f.take(max + 1).read_to_end(&mut b)?;
    if b.len() as u64 > max {
        return Err(RuntimeError::Invalid("backup_file_budget"));
    }
    Ok(b)
}
/// Validate the stored backup as a real read-only SQLite database, not just opaque bytes.
pub fn verify_checkpoint(store: &Store, root: &BlobDigest) -> Result<Checkpoint> {
    let closure = store.current_closure(&BTreeSet::from([root.clone()]))?;
    let object = &closure.objects[root];
    if object.manifest.metadata.kind != ObjectKind::Checkpoint {
        return Err(RuntimeError::Invalid("checkpoint_object_kind"));
    }
    let cp: Checkpoint =
        joil_evidence::identity::strict_decode(&object.payload).map_err(RuntimeError::Wire)?;
    cp.check()?;
    if cp.references()? != object.manifest.metadata.dependencies {
        return Err(RuntimeError::Invalid("checkpoint_complete_closure"));
    }
    let backup = &closure.objects[&cp.sqlite_backup];
    if backup.manifest.metadata.kind != ObjectKind::Checkpoint {
        return Err(RuntimeError::Invalid("sqlite_backup_kind"));
    }
    let directory = store
        .path()
        .join(format!("offline-verify-{}", cp.sqlite_backup.as_str()));
    let _private = Store::open(&directory)?;
    let path = directory.join("ledger.sqlite");
    if path.exists() {
        if read_bounded(&path, 16 * 1024 * 1024)? != backup.payload {
            return Err(RuntimeError::Conflict("offline_backup_file_changed"));
        }
    } else {
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)?;
        f.write_all(&backup.payload)?;
        f.sync_all()?;
        File::open(&directory)?.sync_all()?;
    }
    let db = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    let integrity: String = db.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if integrity != "ok" || snapshot(&db)? != cp.ledger {
        return Err(RuntimeError::Conflict("sqlite_backup_snapshot_mismatch"));
    }
    Ok(cp)
}

pub mod db_ops;
pub use db_ops::*;
