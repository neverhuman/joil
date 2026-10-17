//! Observation cache identity is separate from later policy/weight decisions.
use crate::subject::{BlobDigest, SemanticId, SubjectBundle};
use crate::ContractError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Calibration {
    Bound { record: BlobDigest },
    Uncalibrated,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationKey {
    pub schema_version: u32,
    pub subject: SubjectBundle,
    pub manufacturing_state: BlobDigest,
    pub scenario: BlobDigest,
    pub solver_build: BlobDigest,
    pub container: BlobDigest,
    pub numerics: BlobDigest,
    pub mesh_policy: BlobDigest,
    pub sampling_design: BlobDigest,
    pub randomness: BlobDigest,
    pub extractor: BlobDigest,
    pub calibration: Calibration,
}
impl EvaluationKey {
    pub fn identity(&self) -> Result<SemanticId, ContractError> {
        if self.schema_version != 1 {
            return Err(ContractError::InvalidKey("evaluation_schema"));
        }
        self.subject.check_structure()?;
        SemanticId::of(&("crucible.evaluation/1", self))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionKey {
    pub observation: SemanticId,
    pub requirement: BlobDigest,
    pub decision_profile: BlobDigest,
    pub preference_policy: BlobDigest,
}
impl DecisionKey {
    pub fn identity(&self) -> Result<SemanticId, ContractError> {
        SemanticId::of(&("crucible.decision/1", self))
    }
}
