use crucible_render::{
    cache_key, Artifact, DigestRef, Disclosure, EvidenceClass, ScalarScale, SceneBundle, Window,
};
use crucible_units::{CoordinateFrame, FrameTransform, Metres};
use joil_evidence::observation::Finite;
use serde::{Deserialize, Serialize};

fn hash() -> DigestRef {
    DigestRef::parse(&"a".repeat(64)).unwrap()
}

fn scene() -> SceneBundle {
    SceneBundle {
        schema_version: 1,
        subject: hash(),
        source_subject: hash(),
        artifacts: vec![Artifact {
            role: "surface".into(),
            digest: hash(),
            bytes: 12,
            disclosure: Disclosure::PublicResearch,
        }],
        observable: "pressure".into(),
        unit: "Pa".into(),
        frame: CoordinateFrame::RightHandedSiZDownstream,
        transform: FrameTransform {
            from: CoordinateFrame::RightHandedSiZDownstream,
            to: CoordinateFrame::RightHandedSiZDownstream,
            translation_m: [Metres::new(0.).unwrap(); 3],
            rotation: [
                [
                    Finite::new(1.0).unwrap(),
                    Finite::new(0.0).unwrap(),
                    Finite::new(0.0).unwrap(),
                ],
                [
                    Finite::new(0.0).unwrap(),
                    Finite::new(1.0).unwrap(),
                    Finite::new(0.0).unwrap(),
                ],
                [
                    Finite::new(0.0).unwrap(),
                    Finite::new(0.0).unwrap(),
                    Finite::new(1.0).unwrap(),
                ],
            ],
        },
        window: Window {
            start_s: 1.,
            end_s: 2.,
        },
        mapping: hash(),
        mask: hash(),
        scale: ScalarScale {
            minimum: 0.,
            maximum: 1.,
            unit: "Pa".into(),
            colormap: "linear_blue_red".into(),
        },
        exaggeration: 1.,
        recipe: hash(),
        evidence_class: EvidenceClass::Synthetic,
        private_export_approved: false,
        motion_is_simulated: false,
    }
}

#[test]
fn v5_26_05_sc_dec_045_cached_interrupted_safe_plate_generation_within_zero_external_budget() {
    #[derive(Serialize, Deserialize)]
    struct CachedPlate {
        payload: Vec<u8>,
        digest: DigestRef,
        evidence_class: EvidenceClass,
        limitations: Vec<String>,
    }

    fn cached_or_render(
        path: &std::path::Path,
        identity: &str,
        plate: &str,
        render_count: &mut usize,
    ) -> Vec<u8> {
        let entry_path = path.join(format!("{plate}.json"));
        if let Ok(bytes) = std::fs::read(&entry_path) {
            if let Ok(entry) = serde_json::from_slice::<CachedPlate>(&bytes) {
                if entry.digest == DigestRef::of(&entry.payload)
                    && entry.evidence_class == EvidenceClass::Synthetic
                    && !entry.limitations.is_empty()
                {
                    return entry.payload;
                }
            }
        }

        *render_count += 1;
        let payload = format!("synthetic-plate-v1:{identity}:{plate}").into_bytes();
        let entry = CachedPlate {
            digest: DigestRef::of(&payload),
            payload: payload.clone(),
            evidence_class: EvidenceClass::Synthetic,
            limitations: vec!["synthetic fixture; no rendering or scientific validation".into()],
        };
        let encoded = serde_json::to_vec(&entry).unwrap();
        let temporary = entry_path.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&temporary, encoded).unwrap();
        std::fs::rename(&temporary, &entry_path).unwrap();
        payload
    }

    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"));
    let root = target.join(format!("v5-26-05-cache-test-{}", std::process::id()));
    let cache = root.join("cache");
    std::fs::create_dir_all(&cache).unwrap();

    let bundle = scene();
    let bundle_manifest = serde_json::to_vec(&bundle).unwrap();
    let manifest_path = root.join("bundle-manifest.json");
    std::fs::write(&manifest_path, &bundle_manifest).unwrap();
    let identity = cache_key(
        &bundle.identity().unwrap(),
        &bundle.recipe,
        "synthetic-build-1",
        &bundle.mapping,
    )
    .unwrap()
    .sha256;

    // First invocation is interrupted after one complete plate has been cached.
    let mut render_count = 0;
    let intact = cached_or_render(&cache, &identity, "overview", &mut render_count);
    assert_eq!(render_count, 1);

    // Simulate a torn/corrupt second entry left by an interrupted writer.
    let corrupt_path = cache.join("pressure.json");
    std::fs::write(
        &corrupt_path,
        serde_json::to_vec(&CachedPlate {
            payload: b"partial-corrupt-payload".to_vec(),
            digest: DigestRef::of(b"expected-but-missing-payload"),
            evidence_class: EvidenceClass::Synthetic,
            limitations: vec!["synthetic fixture".into()],
        })
        .unwrap(),
    )
    .unwrap();

    // Resume reuses the intact plate byte-for-byte and regenerates the bad entry.
    let resumed_intact = cached_or_render(&cache, &identity, "overview", &mut render_count);
    let regenerated = cached_or_render(&cache, &identity, "pressure", &mut render_count);
    assert_eq!(resumed_intact, intact);
    assert_eq!(
        render_count, 2,
        "only the missing/corrupt plate is recomputed"
    );

    // A second resume is deterministic and hits both now-valid entries.
    let mut second_resume_count = 0;
    assert_eq!(
        cached_or_render(&cache, &identity, "overview", &mut second_resume_count),
        intact
    );
    assert_eq!(
        cached_or_render(&cache, &identity, "pressure", &mut second_resume_count),
        regenerated
    );
    assert_eq!(second_resume_count, 0);
    assert_eq!(std::fs::read(manifest_path).unwrap(), bundle_manifest);
    assert_eq!(bundle.evidence_class, EvidenceClass::Synthetic);
    std::fs::remove_dir_all(root).unwrap();
}
