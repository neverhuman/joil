use crate::{BlobDigest, Result, SemanticId, StoreError};
use crucible_units::Unit;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestDisclosure {
    PublicCandidate,
    Private,
}
pub type Disclosure = ManifestDisclosure;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Input,
    Geometry,
    Mesh,
    Field,
    RawReduction,
    Observation,
    Calibration,
    Decision,
    QualifiedArchive,
    Scene,
    PaperClaim,
    Checkpoint,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalarType {
    U8,
    I32,
    F64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    ContiguousCRowMajor,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ByteOrder {
    LittleEndian,
    BigEndian,
    NotApplicable,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArrayLayout {
    pub shape: Vec<u64>,
    pub scalar: ScalarType,
    pub byte_order: ByteOrder,
    pub layout: Layout,
    pub unit: Unit,
}
impl ArrayLayout {
    pub fn bytes(&self) -> Result<u64> {
        if self.shape.is_empty() || self.shape.len() > 16 || self.shape.contains(&0) {
            return Err(StoreError::Invalid("array_shape"));
        }
        if (self.scalar == ScalarType::U8) != (self.byte_order == ByteOrder::NotApplicable) {
            return Err(StoreError::Invalid("array_byte_order"));
        }
        let width = match self.scalar {
            ScalarType::U8 => 1_u64,
            ScalarType::I32 => 4,
            ScalarType::F64 => 8,
        };
        self.shape.iter().try_fold(width, |n, &d| {
            n.checked_mul(d)
                .ok_or(StoreError::Invalid("array_size_overflow"))
        })
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub kind: ObjectKind,
    pub media_type: String,
    pub semantic: Option<SemanticId>,
    pub dependencies: BTreeSet<BlobDigest>,
    pub array: Option<ArrayLayout>,
    pub disclosure: Disclosure,
}
impl Metadata {
    pub fn private(kind: ObjectKind) -> Self {
        Self {
            kind,
            media_type: "application/octet-stream".into(),
            semantic: None,
            dependencies: BTreeSet::new(),
            array: None,
            disclosure: Disclosure::Private,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub payload: BlobDigest,
    pub payload_bytes: u64,
    pub metadata: Metadata,
}
impl Manifest {
    pub fn check(&self) -> Result<()> {
        if self.schema_version != 1
            || self.payload_bytes > 2_u64.pow(53)
            || self.metadata.media_type.is_empty()
            || self.metadata.media_type.len() > 128
            || self.metadata.dependencies.len() > 4096
        {
            return Err(StoreError::Invalid("manifest_contract"));
        }
        if let Some(array) = &self.metadata.array {
            if array.bytes()? != self.payload_bytes {
                return Err(StoreError::Invalid("array_byte_length"));
            }
        }
        Ok(())
    }
}
