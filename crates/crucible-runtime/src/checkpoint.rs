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
        let mut stmt = connection.prepare(&format!(
            "SELECT * FROM {name} ORDER BY {}",
            (1..=connection
                .prepare(&format!("SELECT * FROM {name}"))?
                .column_count())
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ))?;
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
impl RuntimeDb {
    pub fn ledger_snapshot(&mut self) -> Result<LedgerSnapshot> {
        let tx = self.connection().transaction()?;
        let s = snapshot(&tx)?;
        tx.commit()?;
        Ok(s)
    }
    pub fn checkpoint_history(&mut self) -> Result<Vec<CheckpointPointer>> {
        let mut stmt=self.connection().prepare("SELECT generation,object,event_position FROM pointers WHERE name LIKE 'checkpoint-%' ORDER BY generation DESC")?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, u64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, u64>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(generation, object, event_position)| {
                Ok(CheckpointPointer {
                    generation,
                    object: BlobDigest::parse(&object)
                        .map_err(|e| RuntimeError::Wire(e.to_string()))?,
                    event_position,
                })
            })
            .collect()
    }
    pub fn publish_checkpoint(
        &mut self,
        store: &Store,
        experiment: &Experiment,
        state: &SearchState,
    ) -> Result<CheckpointPointer> {
        self.publish_checkpoint_checked(store, experiment, state, |_| {})
    }
    /// Caller owns the campaign writer lock. SQLite backup/snapshot disagreement still fails closed.
    pub fn publish_checkpoint_checked(
        &mut self,
        store: &Store,
        experiment: &Experiment,
        state: &SearchState,
        mut hook: impl FnMut(CheckpointPhase),
    ) -> Result<CheckpointPointer> {
        state.check()?;
        if experiment.identity()? != state.campaign {
            return Err(RuntimeError::Conflict("checkpoint_campaign"));
        }
        let start = self.event_position()?;
        let backup_dir = store.path().join(format!("ledger-backup-{}", suffix()?));
        self.backup(&backup_dir)?;
        let ledger = self.ledger_snapshot()?;
        if start != ledger.event_position {
            return Err(RuntimeError::Conflict("concurrent_ledger_writer"));
        }
        let sqlite_backup = store.publish(
            &read_bounded(&backup_dir.join("runtime.sqlite"), 16 * 1024 * 1024)?,
            Metadata::private(ObjectKind::Checkpoint),
        )?;
        let cp = Checkpoint {
            schema_version: 1,
            synthetic_only: true,
            experiment: experiment.clone(),
            state: state.clone(),
            ledger,
            sqlite_backup,
        };
        cp.check()?;
        let bytes = crate::encode(&cp)?;
        let root = store.publish_checked(
            bytes.as_bytes(),
            Metadata {
                dependencies: cp.references()?,
                ..Metadata::private(ObjectKind::Checkpoint)
            },
            None,
            |phase| {
                match phase {
                    PublishPhase::Verified => hook(CheckpointPhase::BeforeRename),
                    PublishPhase::Renamed => hook(CheckpointPhase::AfterRename),
                    _ => (),
                };
                Ok(())
            },
        )?;
        verify_checkpoint(store, &root)?;
        store.pin_root(root.clone(), PinReason::Checkpoint)?;
        let tx = self
            .connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let position: u64 =
            tx.query_row("SELECT coalesce(max(sequence),0) FROM events", [], |r| {
                r.get(0)
            })?;
        if position != start {
            return Err(RuntimeError::Conflict("concurrent_ledger_writer"));
        }
        let generation: u64 = tx.query_row(
            "SELECT coalesce(max(generation),0)+1 FROM pointers",
            [],
            |r| r.get(0),
        )?;
        let pointer = CheckpointPointer {
            generation,
            object: root,
            event_position: start,
        };
        tx.execute(
            "INSERT INTO pointers(name,generation,object,event_position) VALUES(?1,?2,?3,?4)",
            params![
                format!("checkpoint-{generation:016}"),
                generation,
                pointer.object.as_str(),
                start
            ],
        )?;
        event(
            &tx,
            "checkpoint_pointer_published",
            pointer.object.as_str(),
            &pointer,
        )?;
        hook(CheckpointPhase::BeforePointerCommit);
        tx.commit()?;
        self.sync_directory()?;
        hook(CheckpointPhase::AfterPointerCommit);
        Ok(pointer)
    }
    pub fn restore_latest(&mut self, store: &Store, requested: &Experiment) -> Result<Restored> {
        let mut rejected = Vec::new();
        for pointer in self.checkpoint_history()? {
            match verify_checkpoint(store, &pointer.object) {
                Err(e) => rejected.push((pointer.object, e.to_string())),
                Ok(checkpoint) => {
                    checkpoint.experiment.require_resume(requested)?;
                    checkpoint
                        .ledger
                        .require_extension(&self.ledger_snapshot()?)?;
                    return Ok(Restored {
                        pointer,
                        checkpoint,
                        rejected_newer: rejected,
                    });
                }
            }
        }
        Err(RuntimeError::Wire(format!(
            "no_verified_checkpoint: {rejected:?}"
        )))
    }
    pub fn quarantine_checkpoint_pointers(
        &mut self,
        rejected: &[(BlobDigest, String)],
    ) -> Result<()> {
        if rejected.is_empty() {
            return Ok(());
        }
        let tx = self
            .connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        for (root, reason) in rejected {
            tx.execute("UPDATE pointers SET name='quarantined-'||name WHERE object=?1 AND name LIKE 'checkpoint-%'",[root.as_str()])?;
            event(
                &tx,
                "checkpoint_corruption_quarantined",
                root.as_str(),
                reason,
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn has_task(&mut self, name: &str) -> Result<bool> {
        Ok(self.connection().query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)",
            [name],
            |r| r.get(0),
        )?)
    }
    /// Rehydrate accepted post-checkpoint results, retaining every canonical charge.
    /// An unaccounted crashed attempt retains its reservation, even after owned cancellation.
    pub fn reconcile(&mut self, store: &Store, state: &mut SearchState) -> Result<()> {
        for task in state.in_flight.values().cloned().collect::<Vec<_>>() {
            if !self.has_task(&task.id)? {
                self.enqueue(&task.id, &task.spec)?;
                continue;
            }
            let row = self.task(&task.id)?;
            if row.spec != task.spec {
                return Err(RuntimeError::Conflict("canonical_task_differs"));
            }
            if row.phase == Phase::Committed {
                let accepted = self.attempt_for(&task.id, row.fence)?;
                if row.accepted_attempt.as_deref() != Some(accepted.id.as_str()) {
                    return Err(RuntimeError::Conflict("accepted_attempt_identity"));
                }
                // The original software acceptance record must still be current, not just its output.
                let object = self.commit(&accepted, store, crate::utc_millis()?)?;
                let closure = store.current_closure(&BTreeSet::from([object.clone()]))?;
                let observation: FixtureObservation =
                    joil_evidence::identity::strict_decode(&closure.objects[&object].payload)
                        .map_err(RuntimeError::Wire)?;
                state.collect(&task.id, object, observation)?;
            } else if matches!(
                row.phase,
                Phase::Leased
                    | Phase::Running
                    | Phase::Staged
                    | Phase::Verified
                    | Phase::Quarantined
            ) {
                let attempt:Option<(String,bool)>=self.connection().query_row("SELECT a.id,EXISTS(SELECT 1 FROM charges c WHERE c.attempt=a.id) FROM attempts a WHERE task=?1 AND fence=?2",params![task.id,row.fence],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
                let Some((name, charged)) = attempt else {
                    return Err(RuntimeError::Invalid("active_attempt_absent"));
                };
                if !charged {
                    if let Some(process) = self.process_identity(&name)? {
                        if !process.cancel_recorded_fixture(Duration::from_millis(20))? {
                            return Err(RuntimeError::Conflict("owned_crash_child_not_reconciled"));
                        }
                    }
                    let tx = self
                        .connection()
                        .transaction_with_behavior(TransactionBehavior::Immediate)?;
                    event(&tx,"crash_accounting_unresolved_reservation_held",&name,&"wait4 and coordinator cost unavailable; do not invent zero, require operator reconciliation")?;
                    tx.commit()?;
                    return Err(RuntimeError::Conflict("crash_accounting_unresolved"));
                }
                // Already charged/verified can be committed with its original live fence.
                let attempt = self.attempt_for(&task.id, row.fence)?;
                if row.phase == Phase::Verified && crate::utc_millis()? < attempt.expires_ms {
                    self.commit(&attempt, store, crate::utc_millis()?)?;
                } else {
                    let tx = self
                        .connection()
                        .transaction_with_behavior(TransactionBehavior::Immediate)?;
                    tx.execute(
                        "UPDATE attempts SET phase='quarantined' WHERE id=?1",
                        [&attempt.id],
                    )?;
                    tx.execute(
                        "UPDATE tasks SET phase='infrastructure_failure' WHERE id=?1 AND fence=?2",
                        params![task.id, row.fence],
                    )?;
                    event(
                        &tx,
                        "charged_crash_attempt_reconciled",
                        &attempt.id,
                        &"known charge retained; new fence required",
                    )?;
                    tx.commit()?;
                    self.retry_ready(&task.id)?;
                }
                return self.reconcile(store, state);
            } else if matches!(
                row.phase,
                Phase::NumericalFailure
                    | Phase::InfrastructureFailure
                    | Phase::TimedOut
                    | Phase::Cancelled
            ) {
                self.retry_ready(&task.id)?;
            } else if row.phase != Phase::Ready {
                return Err(RuntimeError::Unsupported(
                    "fixture_task_terminal_without_observation",
                ));
            }
        }
        state.check()
    }
    pub fn attempt_for(&mut self, task: &str, fence: u64) -> Result<Attempt> {
        let (id,lease,expires,evaluation):(String,String,u64,String)=self.connection().query_row("SELECT a.id,a.lease,l.expires_ms,a.evaluation FROM attempts a JOIN leases l ON l.attempt=a.id WHERE a.task=?1 AND a.fence=?2",params![task,fence],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
        Ok(Attempt {
            id,
            task_id: task.into(),
            fence,
            lease_id: lease,
            expires_ms: expires,
            evaluation: SemanticId::parse(&evaluation)
                .map_err(|e| RuntimeError::Wire(e.to_string()))?,
        })
    }
}
/// Preserve partial publication evidence but remove it from the active staging namespace.
pub fn quarantine_staging(store: &Store) -> Result<usize> {
    let quarantine = store.path().join("quarantined-staging");
    let _private = Store::open(&quarantine)?;
    let mut count = 0;
    for entry in std::fs::read_dir(store.path().join("staging"))? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            return Err(RuntimeError::Invalid("unexpected_staging_entry"));
        }
        std::fs::rename(entry.path(), quarantine.join(entry.file_name()))?;
        count += 1;
    }
    File::open(&quarantine)?.sync_all()?;
    File::open(store.path().join("staging"))?.sync_all()?;
    Ok(count)
}
