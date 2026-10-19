use super::*;

#[derive(Clone, Debug, Serialize)]
pub struct RunResult {
    pub root: PathBuf,
    pub synthetic_only: bool,
    pub physical_eligibility: String,
    pub epochs: u64,
    pub checkpoint: BlobDigest,
    pub logical_digest: SemanticId,
    pub actual_cost_digest: SemanticId,
    pub lineage: SemanticId,
    pub accounting: crucible_runtime::ledger::Accounting,
    pub stopped: bool,
    pub fallback_reasons: Vec<(BlobDigest, String)>,
}
pub fn run(
    campaign: &Campaign,
    root: &Path,
    resume: bool,
    until: u64,
    continue_stop: bool,
) -> Result<RunResult> {
    campaign.check()?;
    if until == 0 || until > campaign.epochs {
        return Err(RuntimeError::Invalid("until_within_campaign"));
    }
    let selected = cpus(campaign.workers)?;
    let root = private_root(root)?;
    if resume {
        metadata(&root)?;
        if !root.join("runtime.sqlite").is_file() {
            return Err(RuntimeError::Conflict(
                "missing_canonical_ledger_refuse_budget_reset",
            ));
        }
    } else if root.exists() {
        return Err(RuntimeError::Conflict("fresh_run_root_already_exists"));
    }
    let store = open_store(&root)?;
    let _writer = WriterLock::acquire(&root)?;
    let exe_path = std::env::current_exe()?;
    let exe = read_private_or_source(&exe_path, 64 * 1024 * 1024)?;
    let expected = BlobDigest::of_bytes(&exe);
    let experiment = freeze(&store, campaign, &exe)?;
    let meta = if resume {
        let m = metadata(&root)?;
        m.experiment.require_resume(&experiment)?;
        if m.executable != expected {
            return Err(RuntimeError::Conflict("frozen_executable_drift"));
        }
        m
    } else {
        let m = RunMetadata {
            schema_version: 1,
            synthetic_only: true,
            experiment,
            authorized: total_budget(),
            executable: expected,
        };
        write_new(&root.join("run.json"), &wire(&m)?)?;
        m
    };
    let mut db = RuntimeDb::open(&root, meta.authorized, &meta.experiment.identity()?)?;
    let (mut state, rejected) = if resume {
        let r = db.restore_latest(&store, &meta.experiment)?;
        db.quarantine_checkpoint_pointers(&r.rejected_newer)?;
        quarantine_staging(&store)?;
        (r.checkpoint.state, r.rejected_newer)
    } else {
        (
            SearchState::new(
                meta.experiment.identity()?,
                campaign.seeds.clone(),
                campaign.weights,
            )?,
            Vec::new(),
        )
    };
    if continue_stop {
        if root.join("stop.request").exists() {
            std::fs::rename(
                root.join("stop.request"),
                root.join(format!("stop-consumed-{}", unique()?)),
            )?;
        }
        if matches!(
            state.stop,
            StopState::Requested { .. } | StopState::Completed
        ) {
            state.stop = StopState::Running;
        }
    }
    if resume && state.stop == StopState::Completed && state.completed_epochs < until {
        state.stop = StopState::Running;
    }
    if !resume {
        cp(&mut db, &store, &meta.experiment, &state)?;
    }
    let pointer;
    loop {
        db.reconcile(&store, &mut state)?;
        if STOP.load(Ordering::Relaxed) || root.join("stop.request").exists() {
            state.stop = StopState::Requested {
                reason: "explicit signal or stop command; actual charged batch retained".into(),
            };
        }
        if matches!(state.stop, StopState::Requested { .. }) {
            pointer = cp(&mut db, &store, &meta.experiment, &state)?;
            break;
        }
        if state.in_flight.is_empty() && !state.pending_results.is_empty() {
            state.apply_epoch()?;
        }
        if state.completed_epochs >= until {
            state.stop = if until == campaign.epochs {
                StopState::Completed
            } else {
                StopState::Running
            };
            pointer = cp(&mut db, &store, &meta.experiment, &state)?;
            break;
        }
        if state.in_flight.is_empty() {
            state.prepare_epoch(&meta.executable, job_budget(), campaign.mode.clone())?;
            cp(&mut db, &store, &meta.experiment, &state)?;
            db.reconcile(&store, &mut state)?;
        }
        let tasks = state.in_flight.values().cloned().collect::<Vec<_>>();
        let mut arrivals = Vec::new();
        for batch in tasks.chunks(campaign.workers) {
            if STOP.load(Ordering::Relaxed) || root.join("stop.request").exists() {
                break;
            }
            let (send, receive) = std::sync::mpsc::channel();
            std::thread::scope(|scope| {
                for (i, task) in batch.iter().enumerate() {
                    let send = send.clone();
                    let root = &root;
                    let meta = &meta;
                    let exe_path = &exe_path;
                    let cpu = selected[i];
                    scope.spawn(move || {
                        let r = execute_one(root, meta, task, cpu, exe_path, campaign.timeout_ms);
                        let _ = send.send((task.id.clone(), r));
                    });
                }
            });
            drop(send);
            for (name, result) in receive {
                result?;
                arrivals.push(name);
            }
            // Threads are joined before any publication; native costs are known at these checkpoints.
        }
        for name in arrivals {
            let row = db.task(&name)?;
            let output = row
                .accepted_object
                .ok_or(RuntimeError::Invalid("accepted_fixture_output_absent"))?;
            let observed: FixtureObservation =
                joil_evidence::identity::strict_decode(&store.read(&output)?.payload)
                    .map_err(RuntimeError::Wire)?;
            state.collect(&name, output, observed)?;
        }
        // The joined batch already has durable, charged canonical results. STOP
        // needs its complete pending snapshot now. Normal complete epochs can use
        // their next boundary; recovery still verifies each original acceptance
        // when reconciling canonical results after interrupted publication.
        if STOP.load(Ordering::Relaxed) || root.join("stop.request").exists() {
            state.stop = StopState::Requested {
                reason: "explicit signal or stop command; actual charged batch retained".into(),
            };
            pointer = cp(&mut db, &store, &meta.experiment, &state)?;
            break;
        }
        // A complete normal epoch is folded on the next iteration, which then
        // publishes either the next prepared epoch or the terminal state. The
        // canonical ledger already durably owns every accepted result/charge;
        // reconciliation rehydrates them if we crash before that publication.
        // Keep partial and STOP snapshots, including their nonempty pending state.
        if !state.in_flight.is_empty() {
            cp(&mut db, &store, &meta.experiment, &state)?;
        }
    }
    let ledger = db.ledger_snapshot()?;
    Ok(RunResult {
        root,
        synthetic_only: true,
        physical_eligibility: "Unsupported: software fixture only".into(),
        epochs: state.completed_epochs,
        checkpoint: pointer.object,
        logical_digest: state.logical_digest()?,
        actual_cost_digest: ledger.actual_cost_digest()?,
        lineage: state.lineage.id,
        accounting: ledger.accounting,
        stopped: matches!(state.stop, StopState::Requested { .. }),
        fallback_reasons: rejected,
    })
}
pub fn status(root: &Path) -> Result<serde_json::Value> {
    let root = private_root(root)?;
    let meta = metadata(&root)?;
    let mut db = RuntimeDb::inspect_existing(&root)?;
    let history = db.checkpoint_history()?;
    Ok(
        serde_json::json!({"synthetic_only":true,"physical_eligibility":"Unsupported: software fixture only","experiment":meta.experiment.identity()?,"history":history,"accounting":db.accounting()?,"event_position":db.event_position()?}),
    )
}
pub fn fork(root: &Path, weights: [i64; 2]) -> Result<serde_json::Value> {
    let root = private_root(root)?;
    let meta = metadata(&root)?;
    let _writer = WriterLock::acquire(&root)?;
    let store = open_store(&root)?;
    let mut db = RuntimeDb::open(&root, meta.authorized, &meta.experiment.identity()?)?;
    let r = db.restore_latest(&store, &meta.experiment)?;
    let mut state = r.checkpoint.state;
    db.reconcile(&store, &mut state)?;
    if state.in_flight.is_empty() && !state.pending_results.is_empty() {
        state.apply_epoch()?;
    }
    let next = state.fork(weights)?;
    let before = db.accounting()?;
    let pointer = cp(&mut db, &store, &meta.experiment, &next)?;
    let result = serde_json::json!({"synthetic_only":true,"lineage":next.lineage,"checkpoint":pointer,"shared_canonical_ledger":true,"observations_reused":next.archives.observations.len(),"accounting":before});
    write_new(
        &root.join(format!("lineage-{}.json", next.lineage.id.as_str())),
        &wire(&result)?,
    )?;
    Ok(result)
}
pub fn verify(root: &Path) -> Result<serde_json::Value> {
    let root = private_root(root)?;
    let meta = metadata(&root)?;
    let store = open_store(&root)?;
    let mut db = RuntimeDb::inspect_existing(&root)?;
    let restored = db.restore_latest(&store, &meta.experiment)?;
    Ok(
        serde_json::json!({"synthetic_only":true,"offline":true,"checkpoint":restored.pointer,"logical_digest":restored.checkpoint.state.logical_digest()?,"rejected_newer":restored.rejected_newer,"physical_eligibility":"Unsupported"}),
    )
}
pub fn stop(root: &Path) -> Result<()> {
    let root = private_root(root)?;
    metadata(&root)?;
    let path = root.join("stop.request");
    if path.exists() {
        return Ok(());
    }
    write_new(
        &path,
        b"explicit operator stop; retain prior checkpoints and charges\n",
    )
}
