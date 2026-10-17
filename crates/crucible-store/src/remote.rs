//! Backend protocol only. No cloud credentials, SDK or worker dispatch exists.
use crate::{
    decode, encode,
    graph::{transitive_closure, Invalidation},
    manifest::Manifest,
    BlobDigest, Result, Store, StoreError,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Implementations must bound reads, make uploads immutable, and implement
/// linearizable pointer CAS. A provider lacking these capabilities is unsupported.
pub trait ObjectBackend {
    fn get(&self, key: &str, max_bytes: u64) -> Result<Option<Vec<u8>>>;
    fn put_immutable(&self, key: &str, bytes: &[u8]) -> Result<()>;
    fn compare_exchange_pointer(&self, expected: Option<&[u8]>, replacement: &[u8])
        -> Result<bool>;
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pointer {
    pub schema_version: u32,
    pub generation: u64,
    pub snapshots: Vec<BlobDigest>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    schema_version: u32,
    roots: BTreeSet<BlobDigest>,
    objects: BTreeSet<BlobDigest>,
    pins: BTreeSet<BlobDigest>,
    events: Vec<Invalidation>,
}
fn bounded(backend: &impl ObjectBackend, key: &str, limit: u64) -> Result<Option<Vec<u8>>> {
    let bytes = backend.get(key, limit)?;
    if bytes.as_ref().is_some_and(|b| b.len() as u64 > limit) {
        return Err(StoreError::Invalid("backend_violated_read_limit"));
    }
    Ok(bytes)
}
fn pointer(backend: &impl ObjectBackend) -> Result<(Option<Vec<u8>>, Pointer)> {
    let bytes = bounded(backend, "latest", 4096)?;
    let pointer = match &bytes {
        Some(bytes) => decode(bytes)?,
        None => Pointer {
            schema_version: 1,
            generation: 0,
            snapshots: vec![],
        },
    };
    if pointer.schema_version != 1
        || pointer.snapshots.len() > 2
        || (pointer.generation == 0) != pointer.snapshots.is_empty()
    {
        return Err(StoreError::Invalid("remote_pointer"));
    }
    Ok((bytes, pointer))
}
fn immutable(backend: &impl ObjectBackend, key: &str, bytes: &[u8]) -> Result<()> {
    backend.put_immutable(key, bytes)?;
    if bounded(backend, key, bytes.len() as u64)?.as_deref() != Some(bytes) {
        return Err(StoreError::Invalid("remote_upload_not_verified"));
    }
    Ok(())
}
pub fn upload_snapshot(
    store: &Store,
    roots: &BTreeSet<BlobDigest>,
    backend: &impl ObjectBackend,
    expected_generation: u64,
) -> Result<Pointer> {
    let (old_bytes, old) = pointer(backend)?;
    if old.generation != expected_generation {
        return Err(StoreError::Conflict);
    }
    let closure = store.closure(roots)?;
    for (id, object) in &closure.objects {
        immutable(
            backend,
            &format!("payloads/{}", object.manifest.payload.as_str()),
            &object.payload,
        )?;
        immutable(
            backend,
            &format!("manifests/{}", id.as_str()),
            &encode(&object.manifest)?,
        )?;
    }
    let objects: BTreeSet<_> = closure.objects.into_keys().collect();
    let pins = store
        .pinned_roots()?
        .intersection(&objects)
        .cloned()
        .collect();
    let snapshot = Snapshot {
        schema_version: 1,
        roots: roots.clone(),
        objects,
        pins,
        events: store.invalidations()?,
    };
    let bytes = encode(&snapshot)?;
    if bytes.len() > 1_048_576 {
        return Err(StoreError::Invalid("snapshot_manifest_budget"));
    }
    let id = BlobDigest::of_bytes(&bytes);
    immutable(backend, &format!("snapshots/{}", id.as_str()), &bytes)?;
    let mut snapshots = vec![id];
    snapshots.extend(old.snapshots.into_iter().take(1));
    let new = Pointer {
        schema_version: 1,
        generation: old
            .generation
            .checked_add(1)
            .ok_or(StoreError::Invalid("pointer_generation_overflow"))?,
        snapshots,
    };
    if !backend.compare_exchange_pointer(old_bytes.as_deref(), &encode(&new)?)? {
        return Err(StoreError::Conflict);
    }
    Ok(new)
}
pub struct RestoreReport {
    pub generation: u64,
    pub snapshot: BlobDigest,
    pub roots: BTreeSet<BlobDigest>,
    pub rejected_snapshots: Vec<(BlobDigest, String)>,
}
pub fn restore_snapshot(
    destination: &Store,
    backend: &impl ObjectBackend,
) -> Result<RestoreReport> {
    let (_, pointer) = pointer(backend)?;
    let mut rejected = Vec::new();
    for id in &pointer.snapshots {
        match restore_one(destination, backend, id) {
            Ok(roots) => {
                return Ok(RestoreReport {
                    generation: pointer.generation,
                    snapshot: id.clone(),
                    roots,
                    rejected_snapshots: rejected,
                })
            }
            Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                return Err(StoreError::Io(e))
            }
            Err(error) => rejected.push((id.clone(), error.to_string())),
        }
    }
    Err(StoreError::RestoreFailures(rejected))
}
fn restore_one(
    destination: &Store,
    backend: &impl ObjectBackend,
    id: &BlobDigest,
) -> Result<BTreeSet<BlobDigest>> {
    let bytes = bounded(backend, &format!("snapshots/{}", id.as_str()), 1_048_576)?
        .ok_or_else(|| StoreError::Missing(id.clone()))?;
    if BlobDigest::of_bytes(&bytes) != *id {
        return Err(StoreError::Corrupt(id.clone()));
    }
    let snapshot: Snapshot = decode(&bytes)?;
    let limits = destination.limits();
    if snapshot.schema_version != 1
        || snapshot.roots.is_empty()
        || !snapshot.roots.is_subset(&snapshot.objects)
        || !snapshot.pins.is_subset(&snapshot.objects)
        || snapshot.objects.len() > limits.closure_objects
    {
        return Err(StoreError::Invalid("snapshot_contract"));
    }
    let mut objects = BTreeMap::new();
    let mut manifests = BTreeMap::new();
    let mut total = 0_u64;
    for object_id in &snapshot.objects {
        let bytes = bounded(
            backend,
            &format!("manifests/{}", object_id.as_str()),
            1_048_576,
        )?
        .ok_or_else(|| StoreError::Missing(object_id.clone()))?;
        if BlobDigest::of_bytes(&bytes) != *object_id {
            return Err(StoreError::Corrupt(object_id.clone()));
        }
        let manifest: Manifest = decode(&bytes)?;
        manifest.check()?;
        if manifest.payload_bytes > limits.object_bytes {
            return Err(StoreError::Invalid("restore_object_budget"));
        }
        total = total
            .checked_add(bytes.len() as u64 + manifest.payload_bytes)
            .ok_or(StoreError::Invalid("restore_size_overflow"))?;
        if total > limits.closure_bytes {
            return Err(StoreError::Invalid("restore_closure_budget"));
        }
        let payload = bounded(
            backend,
            &format!("payloads/{}", manifest.payload.as_str()),
            limits.object_bytes,
        )?
        .ok_or_else(|| StoreError::Missing(manifest.payload.clone()))?;
        if payload.len() as u64 != manifest.payload_bytes
            || BlobDigest::of_bytes(&payload) != manifest.payload
        {
            return Err(StoreError::Corrupt(object_id.clone()));
        }
        manifests.insert(object_id.clone(), manifest.clone());
        objects.insert(object_id.clone(), (manifest, payload));
    }
    if transitive_closure(&snapshot.roots, &manifests)? != snapshot.objects {
        return Err(StoreError::Invalid("snapshot_not_exact_closure"));
    }
    let mut copied = BTreeSet::new();
    while !objects.is_empty() {
        let ready = objects
            .iter()
            .find(|(_, o)| o.0.metadata.dependencies.is_subset(&copied))
            .map(|(id, _)| id.clone())
            .ok_or(StoreError::Invalid("restore_cycle"))?;
        let (manifest, payload) = objects.remove(&ready).unwrap();
        destination.publish_checked(&payload, manifest.metadata, Some(&ready), |_| Ok(()))?;
        copied.insert(ready);
    }
    destination.restore_event_prefix(&snapshot.events)?;
    for id in snapshot.pins {
        destination.pin_restored_root(id)?;
    }
    destination.closure(&snapshot.roots)?;
    Ok(snapshot.roots)
}
