use crate::{
    decode, encode, graph::*, manifest::*, secure_fs as fs, BlobDigest, Result, SemanticId,
    StoreError,
};
use crucible_contract::{
    keys::{Calibration, EvaluationKey},
    subject::BlobDigest as Digest,
};
use joil_evidence::{claims::Registry, identity::ContentRef};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static COUNTER: AtomicU64 = AtomicU64::new(0);
fn unique() -> String {
    format!(
        "stage-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishPhase {
    StagingCreated,
    PayloadWritten,
    PayloadSynced,
    ManifestWritten,
    ManifestSynced,
    Verified,
    Renamed,
    ParentSynced,
}
#[derive(Clone, Debug)]
pub struct StoredObject {
    pub manifest: Manifest,
    pub payload: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Closure {
    pub objects: BTreeMap<BlobDigest, StoredObject>,
    pub freshness: Freshness,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub object_bytes: u64,
    pub closure_bytes: u64,
    pub closure_objects: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            object_bytes: 16 * 1024 * 1024,
            closure_bytes: 128 * 1024 * 1024,
            closure_objects: 4096,
        }
    }
}
pub struct Store {
    root: File,
    objects: File,
    staging: File,
    indexes: File,
    limits: Limits,
    path: PathBuf,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        Self::with_limits(path, Limits::default())
    }
    pub fn with_limits(path: &Path, limits: Limits) -> Result<Self> {
        if limits.object_bytes == 0
            || limits.closure_bytes < limits.object_bytes
            || limits.closure_objects == 0
            || limits.object_bytes > 256 * 1024 * 1024
            || limits.closure_bytes > 512 * 1024 * 1024
            || limits.closure_objects > 4096
        {
            return Err(StoreError::Invalid("store_limits"));
        }
        let root = fs::private_root(path)?;
        let objects = fs::directory(&root, "objects", true)?;
        let staging = fs::directory(&root, "staging", true)?;
        let indexes = fs::directory(&root, "indexes", true)?;
        Ok(Self {
            root,
            objects,
            staging,
            indexes,
            limits,
            path: path.to_path_buf(),
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn limits(&self) -> Limits {
        self.limits
    }
    pub fn publish(&self, bytes: &[u8], metadata: Metadata) -> Result<BlobDigest> {
        self.publish_checked(bytes, metadata, None, |_| Ok(()))
    }
    /// Fault hook is for deterministic durability tests. A post-rename crash may
    /// leave a complete object, but can never authorize partial bytes.
    pub fn publish_checked(
        &self,
        bytes: &[u8],
        mut metadata: Metadata,
        expected: Option<&BlobDigest>,
        mut hook: impl FnMut(PublishPhase) -> Result<()>,
    ) -> Result<BlobDigest> {
        let _lock = fs::lock(&self.root, "coordinator.lock")?;
        if bytes.len() as u64 > self.limits.object_bytes {
            return Err(StoreError::Invalid("object_budget"));
        }
        let mut parent_manifests = BTreeMap::new();
        let mut parent_bytes = 0_u64;
        if !metadata.dependencies.is_empty() {
            let closure = self.closure(&metadata.dependencies)?;
            if closure
                .objects
                .values()
                .any(|o| o.manifest.metadata.disclosure == Disclosure::Private)
            {
                metadata.disclosure = Disclosure::Private;
            }
            for (id, object) in closure.objects {
                parent_bytes = parent_bytes
                    .checked_add(
                        object.payload.len() as u64 + encode(&object.manifest)?.len() as u64,
                    )
                    .ok_or(StoreError::Invalid("publication_size_overflow"))?;
                parent_manifests.insert(id, object.manifest);
            }
        }
        let manifest = Manifest {
            schema_version: 1,
            payload: BlobDigest::of_bytes(bytes),
            payload_bytes: bytes.len() as u64,
            metadata,
        };
        manifest.check()?;
        let manifest_bytes = encode(&manifest)?;
        if manifest_bytes.len() > 1_048_576 {
            return Err(StoreError::Invalid("manifest_budget"));
        }
        let id = BlobDigest::of_bytes(&manifest_bytes);
        if parent_manifests.len() + 1 > self.limits.closure_objects
            || parent_bytes + manifest_bytes.len() as u64 + bytes.len() as u64
                > self.limits.closure_bytes
        {
            return Err(StoreError::Invalid("publication_closure_budget"));
        }
        parent_manifests.insert(id.clone(), manifest.clone());
        transitive_closure(&BTreeSet::from([id.clone()]), &parent_manifests)?;
        if expected.is_some_and(|e| e != &id) {
            return Err(StoreError::Corrupt(id));
        }
        if self.read(&id).is_ok() {
            return Ok(id);
        }
        let stage_name = unique();
        let stage = fs::directory(&self.staging, &stage_name, true)?;
        hook(PublishPhase::StagingCreated)?;
        let write = |name: &str,
                     data: &[u8],
                     written,
                     synced,
                     hook: &mut dyn FnMut(PublishPhase) -> Result<()>|
         -> Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(fs::child(&stage, name)?)?;
            file.write_all(data)?;
            hook(written)?;
            file.sync_all()?;
            hook(synced)
        };
        write(
            "payload",
            bytes,
            PublishPhase::PayloadWritten,
            PublishPhase::PayloadSynced,
            &mut hook,
        )?;
        write(
            "manifest.json",
            &manifest_bytes,
            PublishPhase::ManifestWritten,
            PublishPhase::ManifestSynced,
            &mut hook,
        )?;
        if fs::read(&stage, "payload", self.limits.object_bytes)? != bytes
            || fs::read(&stage, "manifest.json", 1_048_576)? != manifest_bytes
        {
            return Err(StoreError::Corrupt(id));
        }
        stage.sync_all()?;
        hook(PublishPhase::Verified)?;
        match std::fs::rename(
            fs::child(&self.staging, &stage_name)?,
            fs::child(&self.objects, id.as_str())?,
        ) {
            Ok(()) => (),
            Err(e) if matches!(e.raw_os_error(), Some(libc::EEXIST | libc::ENOTEMPTY)) => {
                self.read(&id)?;
                return Ok(id);
            }
            Err(e) => return Err(e.into()),
        }
        hook(PublishPhase::Renamed)?;
        self.objects.sync_all()?;
        self.staging.sync_all()?;
        hook(PublishPhase::ParentSynced)?;
        self.read(&id)?;
        Ok(id)
    }
    pub fn read(&self, id: &BlobDigest) -> Result<StoredObject> {
        let dir = fs::directory(&self.objects, id.as_str(), false).map_err(|e| match e {
            StoreError::Io(ref io) if io.kind() == std::io::ErrorKind::NotFound => {
                StoreError::Missing(id.clone())
            }
            _ => e,
        })?;
        if fs::entries(&dir)? != ["manifest.json", "payload"] {
            return Err(StoreError::Corrupt(id.clone()));
        }
        let manifest_bytes = fs::read(&dir, "manifest.json", 1_048_576)?;
        if BlobDigest::of_bytes(&manifest_bytes) != *id {
            return Err(StoreError::Corrupt(id.clone()));
        }
        let manifest: Manifest = decode(&manifest_bytes)?;
        manifest.check()?;
        if manifest.payload_bytes > self.limits.object_bytes {
            return Err(StoreError::Invalid("object_budget"));
        }
        let payload = fs::read(&dir, "payload", self.limits.object_bytes)?;
        if payload.len() as u64 != manifest.payload_bytes
            || BlobDigest::of_bytes(&payload) != manifest.payload
        {
            return Err(StoreError::Corrupt(id.clone()));
        }
        Ok(StoredObject { manifest, payload })
    }
    pub fn closure(&self, roots: &BTreeSet<BlobDigest>) -> Result<Closure> {
        if roots.is_empty() {
            return Err(StoreError::Invalid("empty_closure_roots"));
        }
        let mut pending: Vec<_> = roots.iter().cloned().collect();
        let mut objects = BTreeMap::new();
        let mut bytes = 0_u64;
        while let Some(id) = pending.pop() {
            if objects.contains_key(&id) {
                continue;
            }
            if objects.len() >= self.limits.closure_objects {
                return Err(StoreError::Invalid("closure_object_budget"));
            }
            let object = self.read(&id)?;
            bytes = bytes
                .checked_add(object.payload.len() as u64 + encode(&object.manifest)?.len() as u64)
                .ok_or(StoreError::Invalid("closure_size_overflow"))?;
            if bytes > self.limits.closure_bytes {
                return Err(StoreError::Invalid("closure_byte_budget"));
            }
            pending.extend(object.manifest.metadata.dependencies.iter().cloned());
            objects.insert(id, object);
        }
        let manifests = objects
            .iter()
            .map(|(id, o)| (id.clone(), o.manifest.clone()))
            .collect();
        let ids = transitive_closure(roots, &manifests)?;
        Ok(Closure {
            objects,
            freshness: freshness(&ids, &self.invalidations()?),
        })
    }
    pub fn current_closure(&self, roots: &BTreeSet<BlobDigest>) -> Result<Closure> {
        let closure = self.closure(roots)?;
        match &closure.freshness {
            Freshness::Current => Ok(closure),
            Freshness::Stale { events } => Err(StoreError::Stale(events.clone())),
        }
    }
    pub fn invalidations(&self) -> Result<Vec<Invalidation>> {
        let bytes = match fs::read(&self.root, "invalidations.jsonl", 16 * 1024 * 1024) {
            Ok(b) => b,
            Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(vec![])
            }
            Err(e) => return Err(e),
        };
        if !bytes.is_empty() && !bytes.ends_with(b"\n") {
            return Err(StoreError::Invalid("partial_invalidation_event"));
        }
        let mut events = Vec::new();
        for (i, line) in bytes
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .enumerate()
        {
            let event: Invalidation = decode(line)?;
            if event.sequence != i as u64 + 1 || event.reason.trim().is_empty() {
                return Err(StoreError::Invalid("invalidation_sequence_or_reason"));
            }
            events.push(event);
        }
        Ok(events)
    }
    pub fn invalidate(
        &self,
        source: BlobDigest,
        kind: InvalidationKind,
        reason: String,
    ) -> Result<Invalidation> {
        self.read(&source)?;
        if reason.trim().is_empty() || reason.len() > 4096 {
            return Err(StoreError::Invalid("invalidation_reason"));
        }
        let _lock = fs::lock(&self.root, "coordinator.lock")?;
        let event = Invalidation {
            sequence: self.invalidations()?.len() as u64 + 1,
            source,
            kind,
            reason,
        };
        let mut bytes = encode(&event)?;
        bytes.push(b'\n');
        fs::append(&self.root, "invalidations.jsonl", &bytes)?;
        Ok(event)
    }
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
