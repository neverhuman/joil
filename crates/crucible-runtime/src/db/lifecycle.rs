//! Verification, resource charging, atomic commit/failure, diagnostics, and SQLite backup for `RuntimeDb`.

use super::*;

impl RuntimeDb {
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
