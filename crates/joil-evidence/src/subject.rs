//! Compiler-bound subject stages, case contracts and reproducible lineage.
use crate::identity::{check_id, scientific_hash, ContentRef, ScientificIdentity};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectBundle {
    pub genome: ContentRef,
    pub ir: ContentRef,
    pub compiler: ContentRef,
    pub geometry_kernel: ContentRef,
    pub compiler_config: ContentRef,
    pub geometry: ContentRef,
    pub surface_mesh: ContentRef,
    pub volume_mesh: Option<ContentRef>,
    pub solver_case: Option<ContentRef>,
}

impl SubjectBundle {
    pub fn refs(&self) -> Vec<&ContentRef> {
        let mut refs = vec![
            &self.genome,
            &self.ir,
            &self.compiler,
            &self.geometry_kernel,
            &self.compiler_config,
            &self.geometry,
            &self.surface_mesh,
        ];
        refs.extend(self.volume_mesh.iter());
        refs.extend(self.solver_case.iter());
        refs
    }

    pub fn check(&self) -> Result<(), String> {
        for reference in self.refs() {
            reference.check()?;
        }
        Ok(())
    }

    pub fn subject_id(&self) -> Result<String, String> {
        self.check()?;
        scientific_hash(&serde_json::json!({"contract": "joil.subject.v1", "bundle": self}))
    }
}

/// Hash stage semantics and exact bytes; identical bytes in different stages remain distinct.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStage {
    Genome,
    Ir,
    Geometry,
    SurfaceMesh,
    VolumeMesh,
    SolverCase,
}

pub fn stage_identity(stage: ArtifactStage, bytes: &[u8]) -> Result<String, String> {
    scientific_hash(&(
        "joil.stage.v1",
        stage,
        crate::hex(&sha2::Sha256::digest(bytes)),
    ))
}

use sha2::Digest;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairRecord {
    pub rule: String,
    pub before_hash: String,
    pub after_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lineage {
    pub parents: Vec<String>,
    pub operator: ContentRef,
    pub operator_version: u32,
    pub seed: u64,
    pub rng_span: [u64; 2],
    pub repairs: Vec<RepairRecord>,
    pub compiler: ContentRef,
}

impl Lineage {
    pub fn check(&self) -> Result<(), String> {
        self.operator.check()?;
        self.compiler.check()?;
        if self.operator_version == 0 || self.rng_span[0] > self.rng_span[1] {
            return Err("invalid_operator_or_rng_span".into());
        }
        crate::identity::unique_ids(self.parents.iter().map(String::as_str))?;
        for repair in &self.repairs {
            check_id(&repair.rule)?;
            if !crate::identity::is_hash(&repair.before_hash)
                || !crate::identity::is_hash(&repair.after_hash)
            {
                return Err("invalid_repair_identity".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseKind {
    Openfoam,
    Fea,
}

/// Writers supply exact geometry/mesh/case bytes; an absent stage cannot qualify a case.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseManifest {
    pub schema: u32,
    pub kind: CaseKind,
    pub identity: ScientificIdentity,
}

impl CaseManifest {
    pub fn check(&self) -> Result<(), String> {
        self.identity.check()?;
        let crate::identity::SubjectRef::Compiled { bundle } = &self.identity.subject else {
            return Err("case_requires_compiled_subject".into());
        };
        if self.schema != 1 || bundle.volume_mesh.is_none() || bundle.solver_case.is_none() {
            return Err("case_identity_incomplete".into());
        }
        Ok(())
    }

    pub fn verify_artifacts(
        &self,
        geometry: &[u8],
        surface: &[u8],
        volume: &[u8],
        case: &[u8],
    ) -> Result<(), String> {
        self.check()?;
        let crate::identity::SubjectRef::Compiled { bundle } = &self.identity.subject else {
            unreachable!()
        };
        for (reference, bytes) in [
            (&bundle.geometry, geometry),
            (&bundle.surface_mesh, surface),
            (bundle.volume_mesh.as_ref().unwrap(), volume),
            (bundle.solver_case.as_ref().unwrap(), case),
        ] {
            if ContentRef::from_bytes(&reference.id, &reference.revision, bytes) != *reference {
                return Err("case_artifact_bytes_mismatch".into());
            }
        }
        Ok(())
    }
}
