//! Fenced local software task execution. Physical/native capabilities are explicit.
pub mod bench;
pub mod checkpoint;
pub mod db;
pub mod lease;
pub mod ledger;
pub mod replay;
pub mod task;
pub mod worker;
pub use crucible_contract::subject::{BlobDigest, SemanticId};

#[derive(Debug)]
pub enum RuntimeError {
    Sql(rusqlite::Error),
    Io(std::io::Error),
    Store(crucible_store::StoreError),
    Wire(String),
    Invalid(&'static str),
    Conflict(&'static str),
    Budget(&'static str),
    StaleFence,
    ExpiredLease,
    Unsupported(&'static str),
}
impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RuntimeError {}
impl From<rusqlite::Error> for RuntimeError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<std::io::Error> for RuntimeError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<crucible_store::StoreError> for RuntimeError {
    fn from(e: crucible_store::StoreError) -> Self {
        Self::Store(e)
    }
}
pub type Result<T> = std::result::Result<T, RuntimeError>;
pub(crate) fn encode<T: serde::Serialize>(v: &T) -> Result<String> {
    serde_json::to_string(v).map_err(|e| RuntimeError::Wire(e.to_string()))
}
pub(crate) fn decode<T: serde::de::DeserializeOwned>(v: &str) -> Result<T> {
    joil_evidence::identity::strict_decode(v.as_bytes()).map_err(RuntimeError::Wire)
}
pub fn utc_millis() -> Result<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| RuntimeError::Invalid("pre_epoch_clock"))?
        .as_millis()
        .try_into()
        .map_err(|_| RuntimeError::Invalid("clock_overflow"))
}
