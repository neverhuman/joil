use crucible_render::{
    cache_key, Artifact, DigestRef, Disclosure, EvidenceClass, ScalarScale, SceneBundle, Window,
};
use crucible_units::{CoordinateFrame, FrameTransform, Metres};
use joil_evidence::observation::Finite;

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
    use crucible_render::{plan_interrupt_safe_plate_generation, CompletedFrameRecord};
    let bundle = scene();
    let key = cache_key(&bundle.subject, &bundle.recipe, "synthetic-build", &hash()).unwrap();
    let completed = vec![
        CompletedFrameRecord {
            frame_index: 0,
            cache_key: key.clone(),
            frame_sha256: hash(),
            bytes: 12,
        },
        CompletedFrameRecord {
            frame_index: 1,
            cache_key: key.clone(),
            frame_sha256: hash(),
            bytes: 0,
        },
        CompletedFrameRecord {
            frame_index: 2,
            cache_key: hash(),
            frame_sha256: hash(),
            bytes: 12,
        },
    ];
    let plan = plan_interrupt_safe_plate_generation(
        &bundle,
        "synthetic-build",
        &hash(),
        4,
        &completed,
        true,
        false,
    )
    .unwrap();
    assert_eq!(plan.reused_frames, vec![0]);
    assert_eq!(plan.pending_frames, vec![1, 2, 3]);
    assert!(plan.low_priority_leased && plan.human_layout_review_required);
    assert!(!plan.scientific_qualification_upgraded);
    assert!(!plan.evidence_limitations.is_empty());
    let changed = plan_interrupt_safe_plate_generation(
        &bundle,
        "changed-build",
        &hash(),
        4,
        &completed,
        true,
        false,
    )
    .unwrap();
    assert_eq!(changed.pending_frames, vec![0, 1, 2, 3]);
    for (frames, lease, upgrade) in [(0, true, false), (4, false, false), (4, true, true)] {
        assert!(plan_interrupt_safe_plate_generation(
            &bundle,
            "synthetic-build",
            &hash(),
            frames,
            &completed,
            lease,
            upgrade
        )
        .is_err());
    }
    let mut invalid = bundle.clone();
    invalid.window.end_s = invalid.window.start_s;
    assert!(plan_interrupt_safe_plate_generation(
        &invalid,
        "synthetic-build",
        &hash(),
        4,
        &completed,
        true,
        false
    )
    .is_err());
    let mut resumed = completed.clone();
    resumed[1].bytes = 12;
    let plan = plan_interrupt_safe_plate_generation(
        &bundle,
        "synthetic-build",
        &hash(),
        4,
        &resumed,
        true,
        false,
    )
    .unwrap();
    assert_eq!(plan.reused_frames, vec![0, 1]);
    assert_eq!(plan.pending_frames, vec![2, 3]);
}
