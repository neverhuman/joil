//! Conservative composition of fixed linear stage responses on a flow network.
//!
//! This is cohort accounting, not a hydraulic solver or a coalescence model. Route
//! fractions must come from a separate hydraulic calculation or measurement. All
//! results remain unqualified software diagnostics, including closed balances.
mod schema;
mod transport;
mod validate;

pub use schema::*;
pub use transport::evaluate;
