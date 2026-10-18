//! Checked inference building blocks. Statistical applicability and physical
//! qualification require separate evidence; a distribution quantile grants neither.
pub mod analysis;
pub mod distribution;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatsError {
    InvalidInput(&'static str),
    Unsupported(&'static str),
    NumericalFailure(&'static str),
}

impl std::fmt::Display for StatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for StatsError {}
