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
}

pub mod cache_retention;
pub use cache_retention::*;
