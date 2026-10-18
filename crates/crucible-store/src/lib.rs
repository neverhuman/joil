//! Private immutable artifacts, exact closures and current evidence freshness.
#![cfg(target_os = "linux")]
pub mod graph;
pub mod manifest;
pub mod remote;
mod secure_fs;
pub mod store;
pub use crucible_contract::subject::{BlobDigest, SemanticId};
pub use store::Store;

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    Wire(String),
    Invalid(&'static str),
    Corrupt(BlobDigest),
    Missing(BlobDigest),
    Cycle(BlobDigest),
    Stale(Vec<u64>),
    Conflict,
    Private,
    InjectedCrash(&'static str),
    RestoreFailures(Vec<(BlobDigest, String)>),
}
impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for StoreError {}
impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
pub type Result<T> = std::result::Result<T, StoreError>;

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    joil_evidence::identity::strict_decode(bytes).map_err(StoreError::Wire)
}
fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| StoreError::Wire(e.to_string()))
}
