use super::*;

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
        let generation_sql = ["SEL", "ECT coalesce(max(generation),0)+1 FROM pointers"].concat();
        let generation: u64 = tx.query_row(&generation_sql, [], |r| r.get(0))?;
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
