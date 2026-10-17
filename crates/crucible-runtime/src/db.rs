//! Local WAL transactions. Worker payloads cannot directly mint a verified row.
use crate::{
    decode, encode,
    lease::{AdmittedLease, PhysicalCore, ResourceLease},
    ledger::*,
    task::*,
    BlobDigest, Result, RuntimeError, SemanticId,
};
use crucible_store::{
    manifest::{Metadata, ObjectKind},
    store::PinReason,
    Store,
};
use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

const SCHEMA_VERSION: i64 = 1;
const SCHEMA: &str = r#"
CREATE TABLE metadata (key TEXT PRIMARY KEY,value TEXT NOT NULL) STRICT;
CREATE TABLE tasks (id TEXT PRIMARY KEY,spec TEXT NOT NULL CHECK(json_valid(spec)),evaluation TEXT NOT NULL,phase TEXT NOT NULL,fence INTEGER NOT NULL DEFAULT 0,accepted_attempt TEXT,accepted_object TEXT) STRICT;
CREATE TABLE attempts (id TEXT PRIMARY KEY,task TEXT NOT NULL REFERENCES tasks(id),fence INTEGER NOT NULL,lease TEXT NOT NULL,phase TEXT NOT NULL,evaluation TEXT NOT NULL,staged TEXT,verified TEXT,started_ms INTEGER NOT NULL,ended_ms INTEGER,UNIQUE(task,fence)) STRICT;
CREATE TABLE leases (id TEXT PRIMARY KEY,attempt TEXT NOT NULL UNIQUE REFERENCES attempts(id),record TEXT NOT NULL CHECK(json_valid(record)),cores TEXT NOT NULL CHECK(json_valid(cores)),process TEXT,expires_ms INTEGER NOT NULL,closed INTEGER NOT NULL DEFAULT 0 CHECK(closed IN(0,1))) STRICT;
CREATE TABLE events (sequence INTEGER PRIMARY KEY AUTOINCREMENT,kind TEXT NOT NULL,subject TEXT NOT NULL,payload TEXT NOT NULL CHECK(json_valid(payload))) STRICT;
CREATE TABLE object_refs (attempt TEXT PRIMARY KEY REFERENCES attempts(id),object TEXT NOT NULL,verification TEXT NOT NULL,evaluation TEXT NOT NULL) STRICT;
CREATE TABLE reservations (attempt TEXT PRIMARY KEY REFERENCES attempts(id),vector TEXT NOT NULL CHECK(json_valid(vector)),released INTEGER NOT NULL DEFAULT 0 CHECK(released IN(0,1))) STRICT;
CREATE TABLE charges (attempt TEXT PRIMARY KEY REFERENCES attempts(id),charge TEXT NOT NULL CHECK(json_valid(charge))) STRICT;
CREATE TABLE conflicts (sequence INTEGER PRIMARY KEY AUTOINCREMENT,subject TEXT NOT NULL,reason TEXT NOT NULL,payload TEXT NOT NULL CHECK(json_valid(payload))) STRICT;
CREATE TABLE family_policies (id TEXT PRIMARY KEY,source TEXT NOT NULL CHECK(json_valid(source)),requirements TEXT NOT NULL CHECK(json_valid(requirements)),policy TEXT NOT NULL CHECK(json_valid(policy)),alpha_units INTEGER NOT NULL,max_attempts INTEGER NOT NULL) STRICT;
CREATE TABLE family_decisions (family TEXT NOT NULL REFERENCES family_policies(id),ordinal INTEGER NOT NULL,alpha_units INTEGER NOT NULL,decision TEXT NOT NULL,PRIMARY KEY(family,ordinal)) STRICT;
CREATE TABLE pointers (name TEXT PRIMARY KEY,generation INTEGER NOT NULL,object TEXT NOT NULL,event_position INTEGER NOT NULL) STRICT;
"#;
pub struct RuntimeDb {
    connection: Connection,
    directory: File,
    path: PathBuf,
}
fn id(value: &str) -> Result<()> {
    joil_evidence::identity::check_id(value).map_err(RuntimeError::Wire)
}
pub(crate) fn event(
    tx: &Transaction<'_>,
    kind: &str,
    subject: &str,
    value: &impl Serialize,
) -> Result<u64> {
    tx.execute(
        "INSERT INTO events(kind,subject,payload) VALUES(?1,?2,?3)",
        params![kind, subject, encode(value)?],
    )?;
    Ok(tx.last_insert_rowid() as u64)
}
pub(crate) fn conflict(
    tx: &Transaction<'_>,
    subject: &str,
    reason: &str,
    value: &impl Serialize,
) -> Result<()> {
    tx.execute(
        "INSERT INTO conflicts(subject,reason,payload) VALUES(?1,?2,?3)",
        params![subject, reason, encode(value)?],
    )?;
    Ok(())
}
fn phase(value: &str) -> Result<Phase> {
    decode(&format!("\"{value}\""))
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifiedRecord {
    schema_version: u32,
    attempt: String,
    fence: u64,
    evaluation: SemanticId,
    output: BlobDigest,
    software_only: bool,
}
#[derive(Clone, Debug)]
pub struct TaskRow {
    pub id: String,
    pub spec: TaskSpec,
    pub phase: Phase,
    pub fence: u64,
    pub accepted_attempt: Option<String>,
    pub accepted_object: Option<BlobDigest>,
}
fn task(connection: &Connection, name: &str) -> Result<TaskRow> {
    let (spec, state, fence, attempt, object): (
        String,
        String,
        u64,
        Option<String>,
        Option<String>,
    ) = connection.query_row(
        "SELECT spec,phase,fence,accepted_attempt,accepted_object FROM tasks WHERE id=?1",
        [name],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    Ok(TaskRow {
        id: name.into(),
        spec: decode(&spec)?,
        phase: phase(&state)?,
        fence,
        accepted_attempt: attempt,
        accepted_object: object
            .map(|s| BlobDigest::parse(&s).map_err(|e| RuntimeError::Wire(e.to_string())))
            .transpose()?,
    })
}
fn accounting(connection: &Connection) -> Result<Accounting> {
    let authorized: Resources = decode(&connection.query_row::<String, _, _>(
        "SELECT value FROM metadata WHERE key='authorized'",
        [],
        |r| r.get(0),
    )?)?;
    let mut reserved = Resources::default();
    let mut spent = Resources::default();
    let mut count = 0;
    let mut exceeded = false;
    let mut statement = connection.prepare("SELECT vector FROM reservations WHERE released=0")?;
    for row in statement.query_map([], |r| r.get::<_, String>(0))? {
        reserved = reserved.checked_add(decode(&row?)?)?;
    }
    let mut statement = connection.prepare(
        "SELECT c.charge,r.vector FROM charges c JOIN reservations r ON r.attempt=c.attempt",
    )?;
    for row in statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (charge, reservation) = row?;
        let charge: Charge = decode(&charge)?;
        let reservation: Resources = decode(&reservation)?;
        charge.check()?;
        spent = spent.checked_add(charge.measured.cumulative())?;
        spent.ram_bytes = spent.ram_bytes.max(charge.measured.ram_bytes);
        spent.vram_bytes = spent.vram_bytes.max(charge.measured.vram_bytes);
        spent.scratch_bytes = spent.scratch_bytes.max(charge.measured.scratch_bytes);
        exceeded |= !charge.measured.fits(reservation);
        count += 1;
    }
    exceeded |= !spent.fits(authorized);
    Ok(Accounting {
        authorized,
        spent,
        reserved,
        charges: count,
        exceeded,
    })
}
fn live(tx: &Transaction<'_>, attempt: &Attempt, now: u64) -> Result<(TaskRow, Phase)> {
    let row = task(tx, &attempt.task_id)?;
    let actual:(String,u64,String,String,String,u64)=tx.query_row("SELECT a.task,a.fence,a.lease,a.evaluation,a.phase,l.expires_ms FROM attempts a JOIN leases l ON l.attempt=a.id WHERE a.id=?1",[&attempt.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))?;
    if actual.0 != attempt.task_id
        || actual.1 != attempt.fence
        || row.fence != attempt.fence
        || actual.2 != attempt.lease_id
        || actual.3 != attempt.evaluation.as_str()
        || row.spec.request.identity()? != attempt.evaluation
    {
        return Err(RuntimeError::StaleFence);
    }
    if now >= actual.5 {
        return Err(RuntimeError::ExpiredLease);
    }
    Ok((row, phase(&actual.4)?))
}
impl RuntimeDb {
    pub fn open(path: &Path, authorized: Resources, experiment: &SemanticId) -> Result<Self> {
        authorized.check()?;
        let _private = Store::open(path)?;
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(path)?;
        let anchored = std::fs::read_link(format!("/proc/self/fd/{}", directory.as_raw_fd()))?;
        let mut connection = Connection::open_with_flags(
            anchored.join("runtime.sqlite"),
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        connection.busy_timeout(Duration::from_millis(250))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version != 0 && version != SCHEMA_VERSION {
            return Err(RuntimeError::Unsupported("runtime_schema_version"));
        }
        if version == 0 {
            let tables: i64 = connection.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
                [],
                |r| r.get(0),
            )?;
            if tables != 0 {
                return Err(RuntimeError::Invalid("unversioned_nonempty_database"));
            }
            // Existing empty SQLite files need no data migration. A new schema
            // never rewrites an unknown/nonempty version-zero database.
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(SCHEMA)?;
            for (key, value) in [
                (
                    "schema",
                    BlobDigest::of_bytes(SCHEMA.as_bytes()).as_str().to_string(),
                ),
                ("authorized", encode(&authorized)?),
                ("experiment", experiment.as_str().into()),
            ] {
                tx.execute(
                    "INSERT INTO metadata(key,value) VALUES(?1,?2)",
                    params![key, value],
                )?;
            }
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            tx.commit()?;
        }
        let stored_schema: String =
            connection.query_row("SELECT value FROM metadata WHERE key='schema'", [], |r| {
                r.get(0)
            })?;
        let stored_authorized: String = connection.query_row(
            "SELECT value FROM metadata WHERE key='authorized'",
            [],
            |r| r.get(0),
        )?;
        let stored_experiment: String = connection.query_row(
            "SELECT value FROM metadata WHERE key='experiment'",
            [],
            |r| r.get(0),
        )?;
        if stored_schema != BlobDigest::of_bytes(SCHEMA.as_bytes()).as_str()
            || decode::<Resources>(&stored_authorized)? != authorized
            || stored_experiment != experiment.as_str()
        {
            return Err(RuntimeError::Conflict("database_immutable_contract"));
        }
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let journal: String =
            connection.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get(0))?;
        if journal != "wal" {
            return Err(RuntimeError::Unsupported("sqlite_wal_unavailable"));
        }
        connection.pragma_update(None, "synchronous", "FULL")?;
        Ok(Self {
            connection,
            directory,
            path: path.to_path_buf(),
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn accounting(&self) -> Result<Accounting> {
        accounting(&self.connection)
    }
    pub fn task(&self, name: &str) -> Result<TaskRow> {
        task(&self.connection, name)
    }
    pub fn event_position(&self) -> Result<u64> {
        Ok(self
            .connection
            .query_row("SELECT coalesce(max(sequence),0) FROM events", [], |r| {
                r.get(0)
            })?)
    }
    pub fn enqueue(&mut self, name: &str, spec: &TaskSpec) -> Result<()> {
        id(name)?;
        spec.check()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<String> = tx
            .query_row("SELECT spec FROM tasks WHERE id=?1", [name], |r| r.get(0))
            .optional()?;
        if let Some(existing) = existing {
            if decode::<TaskSpec>(&existing)? != *spec {
                conflict(&tx, name, "task_identity", spec)?;
                tx.commit()?;
                return Err(RuntimeError::Conflict("task_identity"));
            }
            return Ok(());
        }
        tx.execute(
            "INSERT INTO tasks(id,spec,evaluation,phase) VALUES(?1,?2,?3,'ready')",
            params![name, encode(spec)?, spec.request.identity()?.as_str()],
        )?;
        event(&tx, "task_ready", name, spec)?;
        tx.commit()?;
        Ok(())
    }
    pub fn lease(
        &mut self,
        name: &str,
        attempt_id: &str,
        lease: &AdmittedLease,
        now: u64,
    ) -> Result<Attempt> {
        id(attempt_id)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = task(&tx, name)?;
        let record = lease.record();
        lease.check_task(row.spec.reserve, row.spec.ranks, row.spec.threads, now)?;
        if row.phase == Phase::Committed {
            return Err(RuntimeError::Conflict("task_already_committed"));
        }
        if row.phase != Phase::Ready {
            if !matches!(
                row.phase,
                Phase::Leased | Phase::Running | Phase::Staged | Phase::Verified
            ) {
                return Err(RuntimeError::Conflict(
                    "terminal_task_requires_explicit_retry_or_new_capability",
                ));
            }
            let expiry:Option<u64>=tx.query_row("SELECT l.expires_ms FROM attempts a JOIN leases l ON l.attempt=a.id WHERE a.task=?1 AND a.fence=?2",params![name,row.fence],|r|r.get(0)).optional()?;
            if expiry.is_none_or(|expiry| expiry > now) {
                return Err(RuntimeError::Conflict("task_not_ready_or_expired"));
            }
        }
        accounting(&tx)?.may_reserve(row.spec.reserve)?;
        let mut statement = tx.prepare("SELECT record,cores FROM leases WHERE closed=0")?;
        for active in
            statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        {
            let (other, cores) = active?;
            let other: ResourceLease = decode(&other)?;
            let cores: BTreeSet<PhysicalCore> = decode(&cores)?;
            if other.host == record.host && !cores.is_disjoint(lease.cores()) {
                return Err(RuntimeError::Conflict(
                    "physical_core_still_owned_until_reconciled",
                ));
            }
        }
        drop(statement);
        let fence = row
            .fence
            .checked_add(1)
            .ok_or(RuntimeError::Invalid("fence_overflow"))?;
        if fence > 2_u64.pow(53) {
            return Err(RuntimeError::Invalid("fence_integer_range"));
        }
        let attempt = Attempt {
            id: attempt_id.into(),
            task_id: name.into(),
            fence,
            lease_id: record.id.clone(),
            expires_ms: record.expires_ms,
            evaluation: row.spec.request.identity()?,
        };
        tx.execute("INSERT INTO attempts(id,task,fence,lease,phase,evaluation,started_ms) VALUES(?1,?2,?3,?4,'leased',?5,?6)",params![attempt.id,name,fence,record.id,attempt.evaluation.as_str(),now])?;
        tx.execute(
            "INSERT INTO leases(id,attempt,record,cores,expires_ms) VALUES(?1,?2,?3,?4,?5)",
            params![
                record.id,
                attempt.id,
                encode(record)?,
                encode(lease.cores())?,
                record.expires_ms
            ],
        )?;
        tx.execute(
            "INSERT INTO reservations(attempt,vector) VALUES(?1,?2)",
            params![attempt.id, encode(&row.spec.reserve)?],
        )?;
        tx.execute(
            "UPDATE tasks SET fence=?2,phase='leased' WHERE id=?1",
            params![name, fence],
        )?;
        event(&tx, "attempt_leased", &attempt.id, &attempt)?;
        tx.commit()?;
        Ok(attempt)
    }
    pub fn start(&mut self, attempt: &Attempt, now: u64) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (_, state) = live(&tx, attempt, now)?;
        if state != Phase::Leased {
            return Err(RuntimeError::Invalid("start_requires_leased"));
        }
        tx.execute(
            "UPDATE attempts SET phase='running' WHERE id=?1",
            [&attempt.id],
        )?;
        tx.execute(
            "UPDATE tasks SET phase='running' WHERE id=?1",
            [&attempt.task_id],
        )?;
        event(&tx, "attempt_running", &attempt.id, attempt)?;
        tx.commit()?;
        Ok(())
    }
    pub fn stage(&mut self, attempt: &Attempt, object: &BlobDigest, now: u64) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (_, state) = live(&tx, attempt, now)?;
        if state != Phase::Running {
            return Err(RuntimeError::Invalid("stage_requires_running"));
        }
        tx.execute(
            "UPDATE attempts SET phase='staged',staged=?2 WHERE id=?1",
            params![attempt.id, object.as_str()],
        )?;
        tx.execute(
            "UPDATE tasks SET phase='staged' WHERE id=?1",
            [&attempt.task_id],
        )?;
        event(&tx, "attempt_staged", &attempt.id, object)?;
        tx.commit()?;
        Ok(())
    }
    pub fn verify(&mut self, attempt: &Attempt, store: &Store, now: u64) -> Result<BlobDigest> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (row, state) = live(&tx, attempt, now)?;
        if state != Phase::Staged {
            return Err(RuntimeError::Invalid("verify_requires_staged"));
        }
        let raw: String = tx.query_row(
            "SELECT staged FROM attempts WHERE id=?1",
            [&attempt.id],
            |r| r.get(0),
        )?;
        let object = BlobDigest::parse(&raw).map_err(|e| RuntimeError::Wire(e.to_string()))?;
        let closure = store.current_closure(&BTreeSet::from([object.clone()]))?;
        let output = &closure.objects[&object];
        let Request::SoftwareFixture { request } = &row.spec.request else {
            return Err(RuntimeError::Unsupported("physical_result_verification"));
        };
        let observed: FixtureObservation =
            joil_evidence::identity::strict_decode(&output.payload).map_err(RuntimeError::Wire)?;
        if output.manifest.metadata.kind != ObjectKind::Observation
            || output.manifest.metadata.semantic.as_ref() != Some(&attempt.evaluation)
            || observed.schema_version != 1
            || !observed.synthetic_only
            || observed.request != attempt.evaluation
            || observed.address != request.address
            || observed.score != request.synthetic_score()?
        {
            return Err(RuntimeError::Invalid(
                "fixture_observation_identity_or_value",
            ));
        }
        let record = VerifiedRecord {
            schema_version: 1,
            attempt: attempt.id.clone(),
            fence: attempt.fence,
            evaluation: attempt.evaluation.clone(),
            output: object.clone(),
            software_only: true,
        };
        let verification = store.publish(
            encode(&record)?.as_bytes(),
            Metadata {
                dependencies: BTreeSet::from([object.clone()]),
                ..Metadata::private(ObjectKind::Checkpoint)
            },
        )?;
        store.pin_root(verification.clone(), PinReason::Checkpoint)?;
        tx.execute(
            "INSERT INTO object_refs(attempt,object,verification,evaluation) VALUES(?1,?2,?3,?4)",
            params![
                attempt.id,
                object.as_str(),
                verification.as_str(),
                attempt.evaluation.as_str()
            ],
        )?;
        tx.execute(
            "UPDATE attempts SET phase='verified',verified=?2 WHERE id=?1",
            params![attempt.id, verification.as_str()],
        )?;
        tx.execute(
            "UPDATE tasks SET phase='verified' WHERE id=?1",
            [&attempt.task_id],
        )?;
        event(&tx, "attempt_verified", &attempt.id, &verification)?;
        tx.commit()?;
        Ok(verification)
    }
    pub fn charge(&mut self, attempt: &Attempt, charge: &Charge, now: u64) -> Result<()> {
        charge.check()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT charge FROM charges WHERE attempt=?1",
                [&attempt.id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(old) = existing {
            if decode::<Charge>(&old)? != *charge {
                conflict(&tx, &attempt.id, "charge_conflict", charge)?;
                tx.commit()?;
                return Err(RuntimeError::Conflict("charge_conflict"));
            }
            return Ok(());
        }
        let identity: (String, u64, String) = tx.query_row(
            "SELECT task,fence,evaluation FROM attempts WHERE id=?1",
            [&attempt.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if identity.0 != attempt.task_id
            || identity.1 != attempt.fence
            || identity.2 != attempt.evaluation.as_str()
        {
            return Err(RuntimeError::StaleFence);
        }
        tx.execute(
            "INSERT INTO charges(attempt,charge) VALUES(?1,?2)",
            params![attempt.id, encode(charge)?],
        )?;
        tx.execute(
            "UPDATE reservations SET released=1 WHERE attempt=?1",
            [&attempt.id],
        )?;
        tx.execute("UPDATE leases SET closed=1 WHERE attempt=?1", [&attempt.id])?;
        tx.execute(
            "UPDATE attempts SET ended_ms=?2 WHERE id=?1",
            params![attempt.id, now],
        )?;
        event(&tx, "attempt_charged", &attempt.id, charge)?;
        tx.commit()?;
        Ok(())
    }
    pub fn commit(&mut self, attempt: &Attempt, store: &Store, now: u64) -> Result<BlobDigest> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = task(&tx, &attempt.task_id)?;
        if row.phase == Phase::Committed {
            if row.accepted_attempt.as_deref() == Some(&attempt.id) {
                let stored_lease: String = tx.query_row(
                    "SELECT lease FROM attempts WHERE id=?1",
                    [&attempt.id],
                    |r| r.get(0),
                )?;
                if row.fence != attempt.fence
                    || row.spec.request.identity()? != attempt.evaluation
                    || stored_lease != attempt.lease_id
                {
                    conflict(&tx, &attempt.id, "forged_duplicate_identity", attempt)?;
                    tx.commit()?;
                    return Err(RuntimeError::StaleFence);
                }
                let verified: String = tx.query_row(
                    "SELECT verification FROM object_refs WHERE attempt=?1",
                    [&attempt.id],
                    |r| r.get(0),
                )?;
                let verified =
                    BlobDigest::parse(&verified).map_err(|e| RuntimeError::Wire(e.to_string()))?;
                store.current_closure(&BTreeSet::from([verified]))?;
                return row
                    .accepted_object
                    .ok_or(RuntimeError::Invalid("committed_object_absent"));
            }
            conflict(&tx, &attempt.id, "duplicate_delivery_after_commit", attempt)?;
            tx.commit()?;
            return Err(RuntimeError::StaleFence);
        }
        let state = match live(&tx, attempt, now) {
            Ok((_, s)) => s,
            Err(e @ (RuntimeError::StaleFence | RuntimeError::ExpiredLease)) => {
                tx.execute(
                    "UPDATE attempts SET phase='quarantined' WHERE id=?1",
                    [&attempt.id],
                )?;
                conflict(&tx, &attempt.id, "stale_commit", attempt)?;
                tx.commit()?;
                return Err(e);
            }
            Err(e) => return Err(e),
        };
        if state != Phase::Verified {
            return Err(RuntimeError::Invalid("commit_requires_verified_closure"));
        }
        let (object, verification, key): (String, String, String) = tx.query_row(
            "SELECT object,verification,evaluation FROM object_refs WHERE attempt=?1",
            [&attempt.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let object = BlobDigest::parse(&object).map_err(|e| RuntimeError::Wire(e.to_string()))?;
        let verification =
            BlobDigest::parse(&verification).map_err(|e| RuntimeError::Wire(e.to_string()))?;
        let closure = store.current_closure(&BTreeSet::from([verification.clone()]))?;
        let record: VerifiedRecord =
            joil_evidence::identity::strict_decode(&closure.objects[&verification].payload)
                .map_err(RuntimeError::Wire)?;
        if key != attempt.evaluation.as_str()
            || record.attempt != attempt.id
            || record.fence != attempt.fence
            || record.evaluation != attempt.evaluation
            || record.output != object
            || !record.software_only
        {
            return Err(RuntimeError::Invalid("verification_receipt_mismatch"));
        }
        let charged: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM charges WHERE attempt=?1)",
            [&attempt.id],
            |r| r.get(0),
        )?;
        if !charged {
            return Err(RuntimeError::Invalid("commit_requires_actual_charge"));
        }
        tx.execute(
            "UPDATE attempts SET phase='committed' WHERE id=?1",
            [&attempt.id],
        )?;
        tx.execute("UPDATE tasks SET phase='committed',accepted_attempt=?2,accepted_object=?3 WHERE id=?1 AND fence=?4",params![attempt.task_id,attempt.id,object.as_str(),attempt.fence])?;
        event(&tx, "attempt_committed", &attempt.id, &object)?;
        tx.commit()?;
        Ok(object)
    }
    pub fn fail(&mut self, attempt: &Attempt, state: Phase, now: u64) -> Result<()> {
        if !matches!(
            state,
            Phase::NumericalFailure
                | Phase::InfrastructureFailure
                | Phase::Unsupported
                | Phase::TimedOut
                | Phase::Cancelled
        ) {
            return Err(RuntimeError::Invalid("failure_phase"));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = task(&tx, &attempt.task_id)?;
        if row.phase == Phase::Committed && row.accepted_attempt.as_deref() == Some(&attempt.id) {
            conflict(
                &tx,
                &attempt.id,
                "committed_attempt_cannot_be_failed",
                &state,
            )?;
            tx.commit()?;
            return Err(RuntimeError::Conflict("committed_attempt_is_terminal"));
        }
        let charged: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM charges WHERE attempt=?1)",
            [&attempt.id],
            |r| r.get(0),
        )?;
        if !charged {
            return Err(RuntimeError::Invalid("failure_requires_actual_charge"));
        }
        match live(&tx, attempt, now) {
            Ok(_) => {
                tx.execute(
                    "UPDATE attempts SET phase=?2 WHERE id=?1",
                    params![attempt.id, state.as_str()],
                )?;
                tx.execute(
                    "UPDATE tasks SET phase=?2 WHERE id=?1",
                    params![attempt.task_id, state.as_str()],
                )?;
            }
            Err(RuntimeError::StaleFence | RuntimeError::ExpiredLease) => {
                tx.execute(
                    "UPDATE attempts SET phase='quarantined' WHERE id=?1",
                    [&attempt.id],
                )?;
                conflict(&tx, &attempt.id, "late_failure_quarantined", &state)?;
            }
            Err(e) => return Err(e),
        }
        event(
            &tx,
            "attempt_failed_not_physical_infeasibility",
            &attempt.id,
            &state,
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn conflict_count(&self) -> Result<u64> {
        Ok(self
            .connection
            .query_row("SELECT count(*) FROM conflicts", [], |r| r.get(0))?)
    }
    pub fn note_diagnostic(&mut self, attempt: &Attempt, diagnostic: &BlobDigest) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let known: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM attempts WHERE id=?1 AND task=?2 AND fence=?3)",
            params![attempt.id, attempt.task_id, attempt.fence],
            |r| r.get(0),
        )?;
        if !known {
            return Err(RuntimeError::StaleFence);
        }
        event(&tx, "private_attempt_diagnostic", &attempt.id, diagnostic)?;
        tx.commit()?;
        Ok(())
    }
    pub fn record_process(
        &mut self,
        attempt: &Attempt,
        identity: &crate::lease::ProcessIdentity,
        now: u64,
    ) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (_, state) = live(&tx, attempt, now)?;
        if state != Phase::Running || !identity.still_matches()? {
            return Err(RuntimeError::Invalid(
                "process_registration_requires_live_owned_running_child",
            ));
        }
        tx.execute(
            "UPDATE leases SET process=?2 WHERE attempt=?1",
            params![attempt.id, encode(identity)?],
        )?;
        event(&tx, "owned_process_recorded", &attempt.id, identity)?;
        tx.commit()?;
        Ok(())
    }
    pub fn process_identity(
        &self,
        attempt_id: &str,
    ) -> Result<Option<crate::lease::ProcessIdentity>> {
        let value: Option<String> = self.connection.query_row(
            "SELECT process FROM leases WHERE attempt=?1",
            [attempt_id],
            |r| r.get(0),
        )?;
        value.map(|v| decode(&v)).transpose()
    }
    pub fn retry_ready(&mut self, name: &str) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = task(&tx, name)?;
        if !matches!(
            row.phase,
            Phase::NumericalFailure
                | Phase::InfrastructureFailure
                | Phase::TimedOut
                | Phase::Cancelled
        ) {
            return Err(RuntimeError::Invalid(
                "retry_requires_recorded_retryable_failure",
            ));
        }
        let pending:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM reservations r JOIN attempts a ON a.id=r.attempt WHERE a.task=?1 AND r.released=0)",[name],|r|r.get(0))?;
        if pending {
            return Err(RuntimeError::Conflict(
                "retry_has_unreconciled_resource_reservation",
            ));
        }
        tx.execute("UPDATE tasks SET phase='ready' WHERE id=?1", [name])?;
        event(&tx, "same_request_retry_ready", name, &row.fence)?;
        tx.commit()?;
        Ok(())
    }
    pub fn backup(&self, destination: &Path) -> Result<()> {
        let _private = Store::open(destination)?;
        let dir = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(destination)?;
        let path = std::fs::read_link(format!("/proc/self/fd/{}", dir.as_raw_fd()))?
            .join("runtime.sqlite");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)?;
        let mut target = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        rusqlite::backup::Backup::new(&self.connection, &mut target)?.run_to_completion(
            64,
            Duration::from_millis(5),
            None,
        )?;
        target.close().map_err(|(_, e)| RuntimeError::Sql(e))?;
        File::open(path)?.sync_all()?;
        dir.sync_all()?;
        Ok(())
    }
    pub(crate) fn connection(&mut self) -> &mut Connection {
        &mut self.connection
    }
    pub fn sync_directory(&self) -> Result<()> {
        self.directory.sync_all()?;
        Ok(())
    }
}
