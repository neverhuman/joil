#[path = "../../crucible-contract/tests/common/mod.rs"]
mod physical_fixture;
use crucible_contract::keys::*;
use crucible_store::{
    graph::*, manifest::*, remote::*, store::*, BlobDigest, SemanticId, Store, StoreError,
};
use joil_evidence::{claims::*, identity::ContentRef};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};
static SEQ: AtomicU64 = AtomicU64::new(0);
fn root(label: &str) -> PathBuf {
    let base = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute() && p.components().any(|p| p.as_os_str() == ".private"))
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join(".private/test-evidence")
        });
    base.join(format!(
        "v5-store-{label}-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ))
}
fn store(label: &str) -> Store {
    Store::open(&root(label)).unwrap()
}
fn roots(id: &BlobDigest) -> BTreeSet<BlobDigest> {
    BTreeSet::from([id.clone()])
}
fn metadata(kind: ObjectKind, deps: BTreeSet<BlobDigest>) -> Metadata {
    Metadata {
        dependencies: deps,
        ..Metadata::private(kind)
    }
}
fn expected(bytes: &[u8], metadata: Metadata) -> BlobDigest {
    BlobDigest::of_bytes(
        &serde_json::to_vec(&Manifest {
            schema_version: 1,
            payload: BlobDigest::of_bytes(bytes),
            payload_bytes: bytes.len() as u64,
            metadata,
        })
        .unwrap(),
    )
}

#[test]
fn actual_sigkill_at_every_publication_boundary_never_accepts_partial_bytes() {
    use std::os::unix::process::ExitStatusExt;
    for phase in [
        PublishPhase::StagingCreated,
        PublishPhase::PayloadWritten,
        PublishPhase::PayloadSynced,
        PublishPhase::ManifestWritten,
        PublishPhase::ManifestSynced,
        PublishPhase::Verified,
        PublishPhase::Renamed,
        PublishPhase::ParentSynced,
    ] {
        let path = root(&format!("kill-{phase:?}"));
        let store = Store::open(&path).unwrap();
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_store-fixture"))
            .arg(&path)
            .arg(format!("{phase:?}"))
            .spawn()
            .unwrap();
        let start = std::time::Instant::now();
        while !path.join("child-ready").exists()
            && start.elapsed() < std::time::Duration::from_secs(5)
        {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let ready = path.join("child-ready").exists();
        child.kill().unwrap();
        let status = child.wait().unwrap();
        assert!(ready, "child never reached {phase:?}");
        assert_eq!(status.signal(), Some(9));
        let id = expected(
            b"synthetic crash payload",
            Metadata::private(ObjectKind::Input),
        );
        let after = store.read(&id);
        if matches!(phase, PublishPhase::Renamed | PublishPhase::ParentSynced) {
            assert_eq!(after.unwrap().payload, b"synthetic crash payload");
        } else {
            assert!(matches!(after, Err(StoreError::Missing(_))));
        }
        // New process-equivalent open recovers and publishes exactly once; stale
        // staging is retained privately and is never a lookup source.
        let reopened = Store::open(&path).unwrap();
        assert_eq!(
            reopened
                .publish(
                    b"synthetic crash payload",
                    Metadata::private(ObjectKind::Input)
                )
                .unwrap(),
            id
        );
        reopened.current_closure(&roots(&id)).unwrap();
    }
}
#[test]
fn digest_mismatch_corruption_truncation_traversal_and_symlinks_refuse() {
    use std::os::unix::fs::symlink;
    let s = store("paths");
    let bad = BlobDigest::of_bytes(b"wrong expected digest");
    assert!(s
        .publish_checked(
            b"abc",
            Metadata::private(ObjectKind::Input),
            Some(&bad),
            |_| Ok(())
        )
        .is_err());
    assert_eq!(
        BlobDigest::of_bytes(b"abc").as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let id = s
        .publish(b"abc", Metadata::private(ObjectKind::Input))
        .unwrap();
    let payload = s.path().join("objects").join(id.as_str()).join("payload");
    std::fs::write(&payload, b"a").unwrap();
    assert!(matches!(s.read(&id), Err(StoreError::Corrupt(_))));
    assert!(Store::open(&s.path().join("../escape")).is_err());
    assert!(BlobDigest::parse("../../escape").is_err());
    let real = store("symlink-target");
    let alias = s.path().join("alias");
    symlink(real.path(), &alias).unwrap();
    assert!(Store::open(&alias).is_err());
    let symlink_id = BlobDigest::of_bytes(b"symlink-object");
    symlink(
        real.path(),
        s.path().join("objects").join(symlink_id.as_str()),
    )
    .unwrap();
    assert!(s.read(&symlink_id).is_err());
    let t = store("leaf-symlink");
    let id = t
        .publish(b"leaf", Metadata::private(ObjectKind::Input))
        .unwrap();
    let object = t.path().join("objects").join(id.as_str());
    std::fs::rename(object.join("payload"), object.join("original-payload")).unwrap();
    symlink(object.join("original-payload"), object.join("payload")).unwrap();
    assert!(t.read(&id).is_err()); // unexpected entry and no-follow both fail closed
}
#[test]
fn concurrent_identical_publication_is_idempotent_and_array_layout_has_independent_oracle() {
    let path = root("concurrent");
    Store::open(&path).unwrap();
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let path = path.clone();
            std::thread::spawn(move || {
                Store::open(&path)
                    .unwrap()
                    .publish(
                        b"same immutable bytes",
                        Metadata::private(ObjectKind::Input),
                    )
                    .unwrap()
            })
        })
        .collect();
    let ids: BTreeSet<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(ids.len(), 1);
    let s = Store::open(&path).unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/v5-store/array-oracle.json"
    ))
    .unwrap();
    let payload: Vec<_> = [1_f64, 2., 3., 4.]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(payload.len() as u64, oracle["bytes"].as_u64().unwrap());
    assert_eq!(
        BlobDigest::of_bytes(&payload).as_str(),
        oracle["sha256"].as_str().unwrap()
    );
    let mut meta = Metadata::private(ObjectKind::Field);
    meta.array = Some(ArrayLayout {
        shape: vec![2, 2],
        scalar: ScalarType::F64,
        byte_order: ByteOrder::LittleEndian,
        layout: Layout::ContiguousCRowMajor,
        unit: crucible_units::Unit::Pascal,
    });
    let id = s.publish(&payload, meta.clone()).unwrap();
    assert_eq!(
        s.read(&id).unwrap().manifest.metadata.array.unwrap().shape,
        vec![2, 2]
    );
    assert!(s.publish(&payload[..31], meta.clone()).is_err());
    meta.array.as_mut().unwrap().shape = vec![u64::MAX, 2];
    assert!(s.publish(&payload, meta).is_err());
}
fn cache_fixture(s: &Store) -> (EvaluationKey, BlobDigest, BlobDigest) {
    let (review, bundle, mut objects) = physical_fixture::bundle();
    for (blob, namespace) in [(bundle.interface_contract.clone(), "v5.interface")]
        .into_iter()
        .chain(
            bundle
                .artifacts
                .iter()
                .map(|a| (a.acceptance.clone(), "v5.artifact-acceptance")),
        )
    {
        objects.insert(
            blob.clone(),
            review
                .registry
                .checked_artifact(
                    &blob.reference(namespace),
                    std::slice::from_ref(&review.requirements),
                )
                .unwrap()
                .to_vec(),
        );
    }
    let mut deps = BTreeSet::new();
    for bytes in objects.values() {
        deps.insert(
            s.publish(bytes, Metadata::private(ObjectKind::Input))
                .unwrap(),
        );
    }
    let mut input = |name: &str| {
        let bytes = format!("synthetic independent input {name}").into_bytes();
        let blob = BlobDigest::of_bytes(&bytes);
        let object = s
            .publish(
                &bytes,
                Metadata::private(if name == "calibration" {
                    ObjectKind::Calibration
                } else {
                    ObjectKind::Input
                }),
            )
            .unwrap();
        deps.insert(object.clone());
        (blob, object)
    };
    let manufacturing_state = input("manufacturing").0;
    let scenario = input("scenario").0;
    let solver_build = input("solver").0;
    let container = input("container").0;
    let numerics = input("numerics").0;
    let mesh_policy = input("mesh").0;
    let sampling_design = input("sampling").0;
    let randomness = input("randomness").0;
    let extractor = input("extractor").0;
    let (calibration, calibration_object) = input("calibration");
    let key = EvaluationKey {
        schema_version: 1,
        subject: bundle,
        manufacturing_state,
        scenario,
        solver_build,
        container,
        numerics,
        mesh_policy,
        sampling_design,
        randomness,
        extractor,
        calibration: Calibration::Bound {
            record: calibration,
        },
    };
    let mut meta = metadata(ObjectKind::Observation, deps);
    meta.semantic = Some(key.identity().unwrap());
    let result = s
        .publish(
            b"synthetic artifact-only scalar record; no physical qualification",
            meta,
        )
        .unwrap();
    (key, result, calibration_object)
}
#[test]
fn exact_cache_requires_all_bytes_and_identities_and_near_duplicates_never_authorize_reuse() {
    let s = store("cache");
    let (key, result, _) = cache_fixture(&s);
    s.bind_exact_cache(&key, &result).unwrap();
    assert_eq!(s.lookup_exact_cache(&key).unwrap().unwrap().0, result);
    let fingerprint = SemanticId::of(&"synthetic same shape").unwrap();
    s.index_fingerprint(&fingerprint, &result).unwrap();
    assert_eq!(s.neighbors(&fingerprint).unwrap(), roots(&result));
    for field in 0..6 {
        let mut changed = key.clone();
        let d = BlobDigest::of_bytes(b"different mesh/scenario/solver/extractor");
        match field {
            0 => changed.mesh_policy = d,
            1 => changed.scenario = d,
            2 => changed.solver_build = d,
            3 => changed.extractor = d,
            4 => changed.subject.frozen_regions[0].boundary = d,
            5 => changed.sampling_design = d,
            _ => unreachable!(),
        };
        assert!(s.lookup_exact_cache(&changed).unwrap().is_none());
        assert!(s.bind_exact_cache(&changed, &result).is_err());
    }
    let original = s.read(&result).unwrap();
    let mut incomplete = original.manifest.metadata.clone();
    incomplete.dependencies.clear();
    let missing = s
        .publish(b"missing all governing inputs", incomplete)
        .unwrap();
    assert!(s.bind_exact_cache(&key, &missing).is_err());
    let other = s
        .publish(
            b"conflicting result at exact same evaluation identity",
            original.manifest.metadata,
        )
        .unwrap();
    assert!(matches!(
        s.bind_exact_cache(&key, &other),
        Err(StoreError::Conflict)
    ));
    assert!(s.path().join("cache-conflicts.jsonl").is_file());
    let first_input = s
        .closure(&roots(&result))
        .unwrap()
        .objects
        .into_iter()
        .find(|(id, _)| id != &result)
        .unwrap()
        .0;
    std::fs::write(
        s.path()
            .join("objects")
            .join(first_input.as_str())
            .join("payload"),
        b"corrupted",
    )
    .unwrap();
    assert!(s.lookup_exact_cache(&key).is_err());
}
#[test]
fn revocation_propagates_to_decisions_archives_scenes_and_paper_claims_without_erasing_history() {
    let s = store("revocation");
    let (key, result, calibration) = cache_fixture(&s);
    s.bind_exact_cache(&key, &result).unwrap();
    let mut parent = result.clone();
    let mut descendants = Vec::new();
    for kind in [
        ObjectKind::Decision,
        ObjectKind::QualifiedArchive,
        ObjectKind::Scene,
        ObjectKind::PaperClaim,
    ] {
        parent = s
            .publish(
                format!("synthetic {kind:?}").as_bytes(),
                metadata(kind, roots(&parent)),
            )
            .unwrap();
        descendants.push(parent.clone());
    }
    for id in &descendants {
        s.current_closure(&roots(id)).unwrap();
    }
    let event = s
        .invalidate(
            calibration,
            InvalidationKind::Revoked,
            "synthetic calibration withdrawal".into(),
        )
        .unwrap();
    assert_eq!(event.sequence, 1);
    assert!(matches!(
        s.lookup_exact_cache(&key),
        Err(StoreError::Stale(_))
    ));
    for id in &descendants {
        assert!(
            matches!(s.current_closure(&roots(id)),Err(StoreError::Stale(ref v)) if v==&vec![1])
        );
        assert!(!s.read(id).unwrap().payload.is_empty());
        assert!(matches!(
            s.closure(&roots(id)).unwrap().freshness,
            Freshness::Stale { .. }
        ));
    }
    // A crashed journal append is an explicit unreadable current state, not an
    // empty event stream that could resurrect an already rendered PASS badge.
    use std::io::Write;
    std::fs::OpenOptions::new()
        .append(true)
        .open(s.path().join("invalidations.jsonl"))
        .unwrap()
        .write_all(b"{partial")
        .unwrap();
    assert!(s.current_closure(&roots(&parent)).is_err());
}
#[test]
fn closure_cycle_missing_edges_persistent_retention_and_private_image_inheritance() {
    let s = store("retention");
    let mesh = s
        .publish(
            b"synthetic governing mesh",
            Metadata::private(ObjectKind::Mesh),
        )
        .unwrap();
    let field = s
        .publish(
            b"synthetic governing field",
            metadata(ObjectKind::Field, roots(&mesh)),
        )
        .unwrap();
    let summary = s
        .publish(
            b"synthetic scalar summary",
            metadata(ObjectKind::PaperClaim, roots(&field)),
        )
        .unwrap();
    s.pin_root(summary.clone(), PinReason::ClaimEvidence)
        .unwrap();
    let garbage = s
        .publish(
            b"unused synthetic scratch",
            Metadata::private(ObjectKind::Input),
        )
        .unwrap();
    let reopened = Store::open(s.path()).unwrap();
    let plan = reopened.retention(&BTreeSet::new()).unwrap();
    assert!(roots(&mesh).is_subset(&plan.retained));
    assert!(plan.retained.contains(&field));
    assert!(plan.candidates.contains(&garbage));
    assert!(reopened.quarantine_unreferenced(&mesh).is_err());
    assert!(reopened.read(&mesh).is_ok());
    reopened.quarantine_unreferenced(&garbage).unwrap();
    assert!(reopened.read(&garbage).is_err());
    assert!(reopened
        .path()
        .join("gc-quarantine")
        .join(garbage.as_str())
        .exists());
    let mut image = metadata(ObjectKind::Scene, roots(&mesh));
    image.disclosure = Disclosure::PublicCandidate;
    let image = s.publish(b"synthetic rendered pixels", image).unwrap();
    assert_eq!(
        s.read(&image).unwrap().manifest.metadata.disclosure,
        Disclosure::Private
    );
    let a = BlobDigest::of_bytes(b"synthetic graph a");
    let b = BlobDigest::of_bytes(b"synthetic graph b");
    let manifest = |deps| Manifest {
        schema_version: 1,
        payload: a.clone(),
        payload_bytes: 0,
        metadata: metadata(ObjectKind::Input, deps),
    };
    let graph = BTreeMap::from([
        (a.clone(), manifest(roots(&b))),
        (b.clone(), manifest(roots(&a))),
    ]);
    assert!(matches!(
        transitive_closure(&roots(&a), &graph),
        Err(StoreError::Cycle(_))
    ));
    assert!(matches!(
        transitive_closure(
            &roots(&a),
            &BTreeMap::from([(a.clone(), manifest(roots(&b)))])
        ),
        Err(StoreError::Missing(_))
    ));
}
#[derive(Default)]
struct MemoryBackend {
    values: Mutex<BTreeMap<String, Vec<u8>>>,
    pointer: Mutex<Option<Vec<u8>>>,
    reject_cas: std::sync::atomic::AtomicBool,
    truncate_upload: std::sync::atomic::AtomicBool,
    deny_reads: std::sync::atomic::AtomicBool,
}
impl ObjectBackend for MemoryBackend {
    fn get(&self, key: &str, max: u64) -> crucible_store::Result<Option<Vec<u8>>> {
        if self.deny_reads.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "synthetic backend read denial",
            )
            .into());
        }
        let result = if key == "latest" {
            self.pointer.lock().unwrap().clone()
        } else {
            self.values.lock().unwrap().get(key).cloned()
        };
        if result.as_ref().is_some_and(|v| v.len() as u64 > max) {
            return Err(StoreError::Invalid("test_backend_read_budget"));
        }
        Ok(result)
    }
    fn put_immutable(&self, key: &str, bytes: &[u8]) -> crucible_store::Result<()> {
        let mut values = self.values.lock().unwrap();
        if values.get(key).is_some_and(|old| old != bytes) {
            return Err(StoreError::Conflict);
        }
        values.entry(key.into()).or_insert_with(|| {
            if self.truncate_upload.load(Ordering::Relaxed) {
                bytes[..bytes.len().saturating_sub(1)].to_vec()
            } else {
                bytes.to_vec()
            }
        });
        Ok(())
    }
    fn compare_exchange_pointer(
        &self,
        expected: Option<&[u8]>,
        replacement: &[u8],
    ) -> crucible_store::Result<bool> {
        let mut current = self.pointer.lock().unwrap();
        if self.reject_cas.swap(false, Ordering::Relaxed) {
            return Ok(false);
        }
        if current.as_deref() != expected {
            return Ok(false);
        }
        *current = Some(replacement.to_vec());
        Ok(true)
    }
}
#[test]
fn conditional_remote_pointer_backup_restore_and_corrupt_latest_fallback_are_real() {
    let source = store("remote-source");
    let backend = MemoryBackend::default();
    let first = source
        .publish(
            b"synthetic first checkpoint",
            Metadata::private(ObjectKind::Checkpoint),
        )
        .unwrap();
    source
        .pin_root(first.clone(), PinReason::Checkpoint)
        .unwrap();
    let one = upload_snapshot(&source, &roots(&first), &backend, 0).unwrap();
    assert_eq!(one.generation, 1);
    assert!(matches!(
        upload_snapshot(&source, &roots(&first), &backend, 0),
        Err(StoreError::Conflict)
    ));
    let second = source
        .publish(
            b"synthetic second checkpoint",
            metadata(ObjectKind::Checkpoint, roots(&first)),
        )
        .unwrap();
    let two = upload_snapshot(&source, &roots(&second), &backend, 1).unwrap();
    assert_eq!(two.snapshots.len(), 2);
    let restored = store("remote-restored");
    let report = restore_snapshot(&restored, &backend).unwrap();
    assert_eq!(report.roots, roots(&second));
    assert!(restored.pinned_roots().unwrap().contains(&first));
    assert_eq!(
        restored.read(&first).unwrap().payload,
        b"synthetic first checkpoint"
    );
    let key = format!("snapshots/{}", two.snapshots[0].as_str());
    backend
        .values
        .lock()
        .unwrap()
        .insert(key, b"corrupt latest manifest".to_vec());
    let fallback = store("remote-fallback");
    let report = restore_snapshot(&fallback, &backend).unwrap();
    assert_eq!(report.roots, roots(&first));
    assert_eq!(report.rejected_snapshots.len(), 1);
    let payload = source.read(&first).unwrap().manifest.payload;
    backend
        .values
        .lock()
        .unwrap()
        .remove(&format!("payloads/{}", payload.as_str()));
    assert!(restore_snapshot(&store("missing-backup"), &backend).is_err());
    let local = store("local-backup");
    source.backup_into(&local, &roots(&second)).unwrap();
    local.current_closure(&roots(&second)).unwrap();
}
fn permit(
    review: &mut physical_fixture::ReviewFixture,
    s: &Store,
    root: &BlobDigest,
) -> (ExportPermit, ContentRef) {
    let record = ExportAllowlist {
        requirements: review.requirements.clone(),
        roots: roots(root),
        objects: s
            .closure(&roots(root))
            .unwrap()
            .objects
            .into_keys()
            .collect(),
    };
    let bytes = serde_json::to_vec(&record).unwrap();
    let reference = ContentRef::from_bytes(
        &format!("synthetic-export-review-{}", review.sequence),
        "1",
        &bytes,
    );
    review.approve(reference.clone(), bytes);
    (
        ExportPermit::from_approved_registry(
            &review.registry,
            reference.clone(),
            &review.requirements,
        )
        .unwrap(),
        reference,
    )
}
#[test]
fn export_requires_current_exact_review_and_never_declassifies_private_geometry_images() {
    let s = store("export");
    let mut review = physical_fixture::ReviewFixture::new();
    let private = s
        .publish(
            b"synthetic private geometry",
            Metadata::private(ObjectKind::Geometry),
        )
        .unwrap();
    let image = s
        .publish(
            b"synthetic pixels",
            Metadata {
                disclosure: Disclosure::PublicCandidate,
                ..metadata(ObjectKind::Scene, roots(&private))
            },
        )
        .unwrap();
    let (p, _) = permit(&mut review, &s, &image);
    assert!(matches!(
        s.export_reviewed(&review.registry, &p, &store("refused-export")),
        Err(StoreError::Private)
    ));
    let public = s
        .publish(
            b"synthetic expressly public fixture",
            Metadata {
                disclosure: Disclosure::PublicCandidate,
                ..Metadata::private(ObjectKind::Input)
            },
        )
        .unwrap();
    let (p, r) = permit(&mut review, &s, &public);
    let exported = store("allowed-export");
    s.export_reviewed(&review.registry, &p, &exported).unwrap();
    assert!(exported.read(&public).is_ok());
    review.review(r, ReviewAction::Revoked);
    assert!(s
        .export_reviewed(&review.registry, &p, &store("stale-export-review"))
        .is_err());
}

#[test]
fn explicit_object_and_closure_limits_refuse_before_publication() {
    let path = root("limits");
    let s = Store::with_limits(
        &path,
        Limits {
            object_bytes: 8,
            closure_bytes: 4096,
            closure_objects: 2,
        },
    )
    .unwrap();
    assert!(s
        .publish(
            b"over object byte budget",
            Metadata::private(ObjectKind::Input)
        )
        .is_err());
    let first = s
        .publish(b"first", Metadata::private(ObjectKind::Input))
        .unwrap();
    let second = s
        .publish(b"second", metadata(ObjectKind::Input, roots(&first)))
        .unwrap();
    let meta = metadata(ObjectKind::Input, roots(&second));
    let id = expected(b"third", meta.clone());
    assert!(s.publish(b"third", meta).is_err());
    assert!(s.read(&id).is_err());
    s.closure(&roots(&second)).unwrap();
}

#[test]
fn incomplete_reviewed_export_allowlist_cannot_drop_a_governing_public_parent() {
    let s = store("incomplete-export");
    let public = Metadata {
        disclosure: Disclosure::PublicCandidate,
        ..Metadata::private(ObjectKind::Geometry)
    };
    let geometry = s
        .publish(b"synthetic public governing geometry", public)
        .unwrap();
    let scene = s
        .publish(
            b"synthetic pixels",
            Metadata {
                disclosure: Disclosure::PublicCandidate,
                ..metadata(ObjectKind::Scene, roots(&geometry))
            },
        )
        .unwrap();
    let mut review = physical_fixture::ReviewFixture::new();
    let record = ExportAllowlist {
        requirements: review.requirements.clone(),
        roots: roots(&scene),
        objects: roots(&scene),
    };
    let bytes = serde_json::to_vec(&record).unwrap();
    let reference = ContentRef::from_bytes("synthetic-incomplete-export-review", "1", &bytes);
    review.approve(reference.clone(), bytes);
    let p = ExportPermit::from_approved_registry(&review.registry, reference, &review.requirements)
        .unwrap();
    assert!(s
        .export_reviewed(&review.registry, &p, &store("missing-export-parent"))
        .is_err());
}

#[test]
fn remote_upload_corruption_lost_cas_and_permission_denial_remain_explicit() {
    let s = store("remote-faults");
    let id = s
        .publish(
            b"synthetic checkpoint bytes",
            Metadata::private(ObjectKind::Checkpoint),
        )
        .unwrap();
    let truncated = MemoryBackend::default();
    truncated.truncate_upload.store(true, Ordering::Relaxed);
    assert!(upload_snapshot(&s, &roots(&id), &truncated, 0).is_err());
    assert!(truncated.pointer.lock().unwrap().is_none());
    let raced = MemoryBackend::default();
    raced.reject_cas.store(true, Ordering::Relaxed);
    assert!(matches!(
        upload_snapshot(&s, &roots(&id), &raced, 0),
        Err(StoreError::Conflict)
    ));
    assert!(raced.pointer.lock().unwrap().is_none());
    assert!(!raced.values.lock().unwrap().is_empty());
    upload_snapshot(&s, &roots(&id), &raced, 0).unwrap();
    raced.deny_reads.store(true, Ordering::Relaxed);
    assert!(
        matches!(restore_snapshot(&store("denied-restore"),&raced),Err(StoreError::Io(e)) if e.kind()==std::io::ErrorKind::PermissionDenied)
    );
}

#[test]
fn reviewed_export_cannot_copy_an_unselected_private_invalidation_journal() {
    let source = store("export-canary");
    let private = source
        .publish(
            b"synthetic private unselected source",
            Metadata::private(ObjectKind::Calibration),
        )
        .unwrap();
    source
        .invalidate(
            private,
            InvalidationKind::Revoked,
            "SYNTHETIC_PRIVATE_UNSELECTED_REASON".into(),
        )
        .unwrap();
    let public = source
        .publish(
            b"synthetic reviewed public selection",
            Metadata {
                disclosure: Disclosure::PublicCandidate,
                ..Metadata::private(ObjectKind::Input)
            },
        )
        .unwrap();
    let mut review = physical_fixture::ReviewFixture::new();
    let (permit, _) = permit(&mut review, &source, &public);
    let target = store("export-no-private-history");
    source
        .export_reviewed(&review.registry, &permit, &target)
        .unwrap();
    assert_eq!(
        target.read(&public).unwrap().payload,
        b"synthetic reviewed public selection"
    );
    assert!(
        target.invalidations().unwrap().is_empty(),
        "unselected private journal must stay at source"
    );
    assert!(!target.path().join("invalidations.jsonl").exists());
}

#[test]
fn reviewed_export_refuses_an_existing_destination_with_unselected_private_files() {
    let source = store("fresh-export-source");
    let public = source
        .publish(
            b"synthetic reviewed public selection",
            Metadata {
                disclosure: Disclosure::PublicCandidate,
                ..Metadata::private(ObjectKind::Input)
            },
        )
        .unwrap();
    let mut review = physical_fixture::ReviewFixture::new();
    let (permit, _) = permit(&mut review, &source, &public);
    let target = store("existing-export-target");
    std::fs::write(
        target.path().join("unselected-private-note"),
        b"SYNTHETIC_PRIVATE_DESTINATION_CANARY",
    )
    .unwrap();
    assert!(source
        .export_reviewed(&review.registry, &permit, &target)
        .is_err());
    assert!(target.read(&public).is_err());
}
