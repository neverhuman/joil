//! Restricted physical interchange; no genotype or origin registry dependency.
pub mod architecture;
pub mod keys;
pub mod outcomes;
pub mod subject;
pub use joil_evidence::identity::{scientific_hash, strict_decode};

#[derive(Clone, Debug, PartialEq)]
pub enum ContractError {
    Wire(String),
    InvalidDigest,
    Unit(crucible_units::UnitError),
    InvalidSubject(&'static str),
    ArtifactMismatch,
    InterfaceMismatch,
    InvalidKey(&'static str),
    ForbiddenDependency(Vec<String>),
}
impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ContractError {}
impl From<crucible_units::UnitError> for ContractError {
    fn from(value: crucible_units::UnitError) -> Self {
        Self::Unit(value)
    }
}
