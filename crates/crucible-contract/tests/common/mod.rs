#![allow(dead_code)]
//! Private test constructors use synthetic authority only; no genotype imports.
use crucible_contract::subject::*;
use crucible_units::{CoordinateFrame, FrameTransform, Metres};
use joil_evidence::claims::{Registry, ReviewAction, ReviewEvent};
use joil_evidence::identity::ContentRef;
use joil_evidence::observation::Finite;
use serde::Serialize;
use std::collections::BTreeMap;
pub fn f(v: f64) -> Finite {
    Finite::new(v).unwrap()
}
pub fn m(v: f64) -> Metres {
    Metres::new(v).unwrap()
}
pub fn reference(id: &str) -> ContentRef {
    ContentRef::from_bytes(id, "1", id.as_bytes())
}
pub struct ReviewFixture {
    pub registry: Registry,
    pub requirements: ContentRef,
    pub sequence: u64,
}
impl ReviewFixture {
    pub fn new() -> Self {
        Self {
            registry: Registry::default(),
            requirements: reference("synthetic-requirements"),
            sequence: 0,
        }
    }
    pub fn review(&mut self, r: ContentRef, action: ReviewAction) {
        self.sequence += 1;
        let event = ReviewEvent {
            sequence: self.sequence,
            subject: r,
            action,
            reviewer: "synthetic-reviewer".into(),
            requirements: vec![self.requirements.clone()],
        };
        let bytes = serde_json::to_vec(&event).unwrap();
        self.registry
            .append_approved_review(
                "synthetic-reviewer",
                std::slice::from_ref(&self.requirements),
                ContentRef::from_bytes(&format!("synthetic-review-{}", self.sequence), "1", &bytes),
                bytes,
            )
            .unwrap();
    }
    pub fn approve(&mut self, r: ContentRef, bytes: Vec<u8>) {
        self.registry.insert_artifact(r.clone(), bytes).unwrap();
        self.review(r, ReviewAction::Approved);
    }
    pub fn blob<T: Serialize>(&mut self, namespace: &str, value: &T) -> BlobDigest {
        let bytes = serde_json::to_vec(value).unwrap();
        let blob = BlobDigest::of_bytes(&bytes);
        self.approve(blob.reference(namespace), bytes);
        blob
    }
}
pub fn surface() -> TriangleSurface {
    TriangleSurface {
        vertices_m: vec![
            [m(0.0), m(0.0), m(0.0)],
            [m(1.0), m(0.0), m(0.0)],
            [m(0.0), m(1.0), m(0.0)],
            [m(0.0), m(0.0), m(1.0)],
        ],
        triangles: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
    }
}
pub fn bundle() -> (ReviewFixture, SubjectBundle, BTreeMap<BlobDigest, Vec<u8>>) {
    let mut review = ReviewFixture::new();
    let bytes = serde_json::to_vec(&surface()).unwrap();
    let blob = BlobDigest::of_bytes(&bytes);
    let ports = vec![Port {
        tag: 1,
        purpose: PortPurpose::Inlet,
        frame: CoordinateFrame::RightHandedSiZDownstream,
        centre_m: [m(0.0), m(0.0), m(0.0)],
        area_m2: f(1.0),
        outward_normal: [f(0.0), f(0.0), f(1.0)],
    }];
    let regions = vec![FrozenRegion {
        tag: 2,
        boundary: BlobDigest::of_bytes(b"synthetic-fixed-boundary"),
    }];
    let interface = InterfaceContract {
        ports: ports.clone(),
        frozen_regions: regions.clone(),
        spatial_tolerance_m: m(1e-6),
    };
    let interface_id = review.blob("v5.interface", &interface);
    let acceptance = review.blob(
        "v5.artifact-acceptance",
        &ArtifactAcceptance {
            artifact: blob.clone(),
            format: ArtifactFormat::TriangleSurfaceJson,
            length_unit: LengthUnit::Metre,
            approximation_bound_m: m(1e-7),
            interface_contract: interface_id.clone(),
        },
    );
    let artifact = |role| ArtifactRef {
        role,
        format: ArtifactFormat::TriangleSurfaceJson,
        blob: blob.clone(),
        bytes: bytes.len() as u64,
        length_unit: LengthUnit::Metre,
        approximation_bound_m: m(1e-7),
        acceptance: acceptance.clone(),
    };
    let bundle = SubjectBundle {
        schema_version: 1,
        artifacts: vec![
            artifact(ArtifactRole::Geometry),
            artifact(ArtifactRole::SurfaceMesh),
        ],
        transform: FrameTransform {
            from: CoordinateFrame::RightHandedSiZDownstream,
            to: CoordinateFrame::RightHandedSiZDownstream,
            translation_m: [m(0.0), m(0.0), m(0.0)],
            rotation: [
                [f(1.0), f(0.0), f(0.0)],
                [f(0.0), f(1.0), f(0.0)],
                [f(0.0), f(0.0), f(1.0)],
            ],
        },
        material: MaterialState::Unknown,
        roughness: RoughnessState::Unknown,
        interface_contract: interface_id,
        ports,
        frozen_regions: regions,
    };
    let boundary = bundle.frozen_regions[0].boundary.clone();
    (
        review,
        bundle,
        BTreeMap::from([
            (blob, bytes),
            (boundary, b"synthetic-fixed-boundary".to_vec()),
        ]),
    )
}
