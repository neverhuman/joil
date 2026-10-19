use super::*;

impl Store {
    pub fn bind_exact_cache(&self, key: &EvaluationKey, result: &BlobDigest) -> Result<()> {
        self.check_cache_closure(key, result)?;
        let identity = key
            .identity()
            .map_err(|e| StoreError::Wire(e.to_string()))?;
        let _lock = fs::lock(&self.root, "coordinator.lock")?;
        let name = format!("cache-{}", identity.as_str());
        match fs::read(&self.indexes, &name, 256) {
            Ok(bytes) => {
                let existing: BlobDigest = decode(&bytes)?;
                if existing != *result {
                    let mut bytes = encode(&(identity, existing, result))?;
                    bytes.push(b'\n');
                    fs::append(&self.root, "cache-conflicts.jsonl", &bytes)?;
                    return Err(StoreError::Conflict);
                }
                Ok(())
            }
            Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                fs::new_file(&self.indexes, &name, &encode(result)?)?;
                self.indexes.sync_all()?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
    fn check_cache_closure(&self, key: &EvaluationKey, result: &BlobDigest) -> Result<Closure> {
        let identity = key
            .identity()
            .map_err(|e| StoreError::Wire(e.to_string()))?;
        let closure = self.current_closure(&BTreeSet::from([result.clone()]))?;
        let manifest = &closure.objects[result].manifest;
        if manifest.metadata.kind != ObjectKind::Observation
            || manifest.metadata.semantic.as_ref() != Some(&identity)
        {
            return Err(StoreError::Invalid("cache_identity_or_kind"));
        }
        let mut required = key.subject.dependencies();
        required.extend([
            key.manufacturing_state.clone(),
            key.scenario.clone(),
            key.solver_build.clone(),
            key.container.clone(),
            key.numerics.clone(),
            key.mesh_policy.clone(),
            key.sampling_design.clone(),
            key.randomness.clone(),
            key.extractor.clone(),
        ]);
        if let Calibration::Bound { record } = &key.calibration {
            required.insert(record.clone());
        }
        let actual: BTreeSet<_> = closure
            .objects
            .values()
            .map(|o| o.manifest.payload.clone())
            .collect();
        if !required.is_subset(&actual) {
            return Err(StoreError::Invalid("cache_missing_bound_inputs"));
        }
        Ok(closure)
    }
    pub fn lookup_exact_cache(&self, key: &EvaluationKey) -> Result<Option<(BlobDigest, Closure)>> {
        let identity = key
            .identity()
            .map_err(|e| StoreError::Wire(e.to_string()))?;
        match fs::read(&self.indexes, &format!("cache-{}", identity.as_str()), 256) {
            Ok(bytes) => {
                let id = decode(&bytes)?;
                let closure = self.check_cache_closure(key, &id)?;
                Ok(Some((id, closure)))
            }
            Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub fn index_fingerprint(&self, fingerprint: &SemanticId, object: &BlobDigest) -> Result<()> {
        self.read(object)?;
        let _lock = fs::lock(&self.root, "coordinator.lock")?;
        let mut bytes = encode(object)?;
        bytes.push(b'\n');
        fs::append(
            &self.indexes,
            &format!("neighbor-{}", fingerprint.as_str()),
            &bytes,
        )
    }
    pub fn neighbors(&self, fingerprint: &SemanticId) -> Result<BTreeSet<BlobDigest>> {
        match fs::read(
            &self.indexes,
            &format!("neighbor-{}", fingerprint.as_str()),
            1_048_576,
        ) {
            Ok(bytes) => bytes
                .split(|b| *b == b'\n')
                .filter(|l| !l.is_empty())
                .map(decode)
                .collect(),
            Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(BTreeSet::new())
            }
            Err(e) => Err(e),
        }
    }
    /// Mark all governing dependencies. Scalar summaries never replace roots.
    pub fn retention(
        &self,
        accepted_and_checkpoint_roots: &BTreeSet<BlobDigest>,
    ) -> Result<RetentionPlan> {
        let mut roots = self.pinned_roots()?;
        roots.extend(accepted_and_checkpoint_roots.iter().cloned());
        let retained = if roots.is_empty() {
            BTreeSet::new()
        } else {
            self.closure(&roots)?.objects.into_keys().collect()
        };
        let objects: BTreeSet<_> = fs::entries(&self.objects)?
            .into_iter()
            .map(|n| {
                BlobDigest::parse(&n).map_err(|_| StoreError::Invalid("unexpected_object_name"))
            })
            .collect::<Result<_>>()?;
        Ok(RetentionPlan {
            candidates: objects.difference(&retained).cloned().collect(),
            retained,
        })
    }
    /// Retention only: pinning never confers evidence admission or PASS.
    pub fn pin_root(&self, root: BlobDigest, reason: PinReason) -> Result<()> {
        let _lock = fs::lock(&self.root, "coordinator.lock")?;
        let closure = self.closure(&BTreeSet::from([root.clone()]))?;
        let kind = closure.objects[&root].manifest.metadata.kind;
        if (reason == PinReason::Checkpoint && kind != ObjectKind::Checkpoint)
            || (reason == PinReason::ClaimEvidence
                && !matches!(
                    kind,
                    ObjectKind::Decision | ObjectKind::QualifiedArchive | ObjectKind::PaperClaim
                ))
        {
            return Err(StoreError::Invalid("retention_root_kind"));
        }
        let mut bytes = encode(&(root, reason))?;
        bytes.push(b'\n');
        fs::append(&self.root, "retention-roots.jsonl", &bytes)
    }
    pub fn pinned_roots(&self) -> Result<BTreeSet<BlobDigest>> {
        let bytes = match fs::read(&self.root, "retention-roots.jsonl", 1_048_576) {
            Ok(b) => b,
            Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BTreeSet::new())
            }
            Err(e) => return Err(e),
        };
        if !bytes.is_empty() && !bytes.ends_with(b"\n") {
            return Err(StoreError::Invalid("partial_retention_event"));
        }
        bytes
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .map(|line| decode::<(BlobDigest, PinReason)>(line).map(|v| v.0))
            .collect()
    }
    /// Reversible GC quarantine. Pins are reread under the publication lock;
    /// accepted/checkpoint closures cannot be moved by a stale caller plan.
    pub fn quarantine_unreferenced(&self, id: &BlobDigest) -> Result<()> {
        let _lock = fs::lock(&self.root, "coordinator.lock")?;
        if !self.retention(&BTreeSet::new())?.candidates.contains(id) {
            return Err(StoreError::Invalid("retained_evidence"));
        }
        self.read(id)?;
        let quarantine = fs::directory(&self.root, "gc-quarantine", true)?;
        fs::rename(&self.objects, id.as_str(), &quarantine, id.as_str())
    }
    pub fn backup_into(&self, destination: &Store, roots: &BTreeSet<BlobDigest>) -> Result<()> {
        let copied = self.copy_objects_into(destination, roots)?;
        // Backups remain private and preserve the coordinator's full history.
        destination.restore_event_prefix(&self.invalidations()?)?;
        for id in self.pinned_roots()? {
            if copied.contains(&id) {
                destination.pin_restored_root(id)?;
            }
        }
        destination.closure(roots)?;
        Ok(())
    }
    fn copy_objects_into(
        &self,
        destination: &Store,
        roots: &BTreeSet<BlobDigest>,
    ) -> Result<BTreeSet<BlobDigest>> {
        let closure = self.closure(roots)?;
        let mut remaining = closure.objects;
        let mut copied = BTreeSet::new();
        while !remaining.is_empty() {
            let ready = remaining
                .iter()
                .find(|(_, o)| o.manifest.metadata.dependencies.is_subset(&copied))
                .map(|(id, _)| id.clone())
                .ok_or(StoreError::Invalid("backup_cycle"))?;
            let o = remaining.remove(&ready).unwrap();
            destination
                .publish_checked(&o.payload, o.manifest.metadata, Some(&ready), |_| Ok(()))?;
            copied.insert(ready);
        }
        destination.closure(roots)?;
        Ok(copied)
    }
    pub(crate) fn pin_restored_root(&self, id: BlobDigest) -> Result<()> {
        let kind = self.read(&id)?.manifest.metadata.kind;
        self.pin_root(
            id,
            if kind == ObjectKind::Checkpoint {
                PinReason::Checkpoint
            } else {
                PinReason::ClaimEvidence
            },
        )
    }
    pub(crate) fn restore_event_prefix(&self, source_events: &[Invalidation]) -> Result<()> {
        for (i, event) in source_events.iter().enumerate() {
            if event.sequence != i as u64 + 1 || event.reason.trim().is_empty() {
                return Err(StoreError::Invalid("restored_event_sequence"));
            }
        }
        let _lock = fs::lock(&self.root, "coordinator.lock")?;
        let old = self.invalidations()?;
        if !source_events.starts_with(&old) {
            return Err(StoreError::Conflict);
        }
        for event in &source_events[old.len()..] {
            let mut bytes = encode(event)?;
            bytes.push(b'\n');
            fs::append(&self.root, "invalidations.jsonl", &bytes)?;
        }
        Ok(())
    }
    pub fn export_reviewed(
        &self,
        registry: &Registry,
        permit: &ExportPermit,
        destination: &Store,
    ) -> Result<()> {
        let bytes = registry
            .checked_artifact(
                &permit.reference,
                std::slice::from_ref(&permit.record.requirements),
            )
            .map_err(StoreError::Wire)?;
        if decode::<ExportAllowlist>(bytes)? != permit.record {
            return Err(StoreError::Conflict);
        }
        // An export is a fresh exact selection. Existing private journals,
        // indexes, partial staging or unrelated objects cannot hitchhike.
        if fs::entries(&destination.root)? != ["indexes", "objects", "staging"]
            || !fs::entries(&destination.objects)?.is_empty()
            || !fs::entries(&destination.indexes)?.is_empty()
            || !fs::entries(&destination.staging)?.is_empty()
        {
            return Err(StoreError::Invalid("export_destination_must_be_fresh"));
        }
        let _source_lock = fs::lock(&self.root, "coordinator.lock")?;
        let closure = self.current_closure(&permit.record.roots)?;
        if closure
            .objects
            .values()
            .any(|o| o.manifest.metadata.disclosure != Disclosure::PublicCandidate)
        {
            return Err(StoreError::Private);
        }
        if closure.objects.keys().cloned().collect::<BTreeSet<_>>() != permit.record.objects {
            return Err(StoreError::Invalid("export_allowlist_not_exact_closure"));
        }
        self.copy_objects_into(destination, &permit.record.roots)?;
        Ok(())
    }
}
pub struct RetentionPlan {
    pub retained: BTreeSet<BlobDigest>,
    pub candidates: BTreeSet<BlobDigest>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinReason {
    ClaimEvidence,
    Checkpoint,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportAllowlist {
    pub requirements: ContentRef,
    pub roots: BTreeSet<Digest>,
    pub objects: BTreeSet<Digest>,
}
/// ```compile_fail
/// let _=serde_json::from_str::<crucible_store::store::ExportPermit>("{}");
/// ```
pub struct ExportPermit {
    reference: ContentRef,
    record: ExportAllowlist,
}
impl ExportPermit {
    pub fn from_approved_registry(
        registry: &Registry,
        reference: ContentRef,
        requirements: &ContentRef,
    ) -> Result<Self> {
        let record: ExportAllowlist = decode(
            registry
                .checked_artifact(&reference, std::slice::from_ref(requirements))
                .map_err(StoreError::Wire)?,
        )?;
        if &record.requirements != requirements
            || record.roots.is_empty()
            || !record.roots.is_subset(&record.objects)
        {
            return Err(StoreError::Invalid("export_allowlist_scope"));
        }
        Ok(Self { reference, record })
    }
}
