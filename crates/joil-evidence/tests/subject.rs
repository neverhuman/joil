use joil_evidence::{identity::*, subject::*};
mod common;

fn reference(id: &str) -> ContentRef {
    ContentRef::from_bytes(id, "1", id.as_bytes())
}
fn bundle() -> SubjectBundle {
    SubjectBundle {
        genome: reference("genome"),
        ir: reference("ir"),
        compiler: reference("compiler"),
        geometry_kernel: reference("kernel"),
        compiler_config: reference("config"),
        geometry: reference("geometry"),
        surface_mesh: reference("surface"),
        volume_mesh: Some(reference("volume")),
        solver_case: Some(reference("case")),
    }
}

#[test]
fn two_compilers_distinct_subject_ids() {
    let a = bundle();
    let mut b = a.clone();
    b.compiler = reference("compiler2");
    assert_eq!(a.genome, b.genome);
    assert_eq!(a.geometry, b.geometry);
    assert_ne!(a.subject_id().unwrap(), b.subject_id().unwrap());
    b = a.clone();
    b.compiler_config = reference("other-tolerance");
    assert_ne!(a.subject_id().unwrap(), b.subject_id().unwrap());
    b = a.clone();
    b.volume_mesh = Some(reference("refined"));
    assert_ne!(a.subject_id().unwrap(), b.subject_id().unwrap());
}

#[test]
fn stages_are_distinct_even_for_identical_bytes() {
    let hashes = [
        ArtifactStage::Genome,
        ArtifactStage::Ir,
        ArtifactStage::Geometry,
        ArtifactStage::SurfaceMesh,
        ArtifactStage::VolumeMesh,
        ArtifactStage::SolverCase,
    ]
    .map(|stage| stage_identity(stage, b"same").unwrap());
    assert_eq!(
        hashes
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        6
    );
}

#[test]
fn lineage_rejects_missing_compiler_and_invalid_span() {
    let mut lineage = Lineage {
        parents: vec!["parent".into()],
        operator: reference("mutate"),
        operator_version: 1,
        seed: 7,
        rng_span: [0, 3],
        repairs: vec![],
        compiler: reference("compiler"),
    };
    lineage.check().unwrap();
    lineage.rng_span = [3, 0];
    assert!(lineage.check().is_err());
    lineage.rng_span = [0, 3];
    lineage.compiler.sha256.clear();
    assert!(lineage.check().is_err());
}

#[test]
fn case_manifest_requires_full_chain_and_exact_artifact_bytes() {
    for kind in [CaseKind::Openfoam, CaseKind::Fea] {
        let mut manifest = CaseManifest {
            schema: 1,
            kind,
            identity: common::identity(),
        };
        manifest
            .verify_artifacts(b"geometry", b"surface", b"volume", b"case")
            .unwrap();
        assert!(manifest
            .verify_artifacts(b"geometry", b"surface", b"volume", b"other-case")
            .is_err());
        let encoded = serde_json::to_vec(&manifest).unwrap();
        let decoded: CaseManifest = strict_decode(&encoded).unwrap();
        assert_eq!(decoded, manifest);
        let SubjectRef::Compiled { bundle } = &mut manifest.identity.subject else {
            unreachable!()
        };
        bundle.solver_case = None;
        assert!(manifest.check().is_err());
    }
}
