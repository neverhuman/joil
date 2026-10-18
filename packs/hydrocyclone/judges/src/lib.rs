//! V5-13 material/process and mass judges for the hydrocyclone pack.
//!
//! Judges here never invent a threshold, density or allowable: every number comes from a
//! status-bearing [`Field`] that is admitted with a source digest, or the judgement is
//! `Unresolved` and names its blockers. Geometry arrives as caller-supplied measurements of
//! one accepted subject (V5-09/V5-10 observers are not accepted yet and are not depended on).
//! Nothing in this crate is a physical qualification.
pub mod as_built;
pub mod dynamics;
pub mod fatigue;
pub mod manufacturing;
pub mod mass;
pub mod material;
pub mod metrology;
pub mod structure;
pub mod surface;
pub mod validation;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq)]
pub enum JudgeError {
    Invalid(String),
}

impl std::fmt::Display for JudgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for JudgeError {}

pub(crate) fn invalid(message: impl Into<String>) -> JudgeError {
    JudgeError::Invalid(message.into())
}

/// Lowercase SHA-256 hex, the same shape as a store `BlobDigest`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Hex(String);

impl Sha256Hex {
    pub fn new(value: &str) -> Result<Self, JudgeError> {
        if value.len() == 64
            && value
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        {
            Ok(Self(value.to_string()))
        } else {
            Err(invalid("expected a lowercase sha256 hex digest"))
        }
    }
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        )
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for Sha256Hex {
    type Error = JudgeError;
    fn try_from(value: String) -> Result<Self, JudgeError> {
        Self::new(&value)
    }
}
impl From<Sha256Hex> for String {
    fn from(value: Sha256Hex) -> String {
        value.0
    }
}

/// Canonical identity of a serializable value (object keys sorted by serde_json).
pub fn canonical_hash<T: Serialize>(value: &T) -> Result<Sha256Hex, JudgeError> {
    let json = serde_json::to_value(value).map_err(|e| invalid(e.to_string()))?;
    let bytes = serde_json::to_vec(&json).map_err(|e| invalid(e.to_string()))?;
    Ok(Sha256Hex::of_bytes(&bytes))
}

/// An input with an explicit status. Only `Admitted` values can be used by a judge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Field<T> {
    Admitted {
        value: T,
        source: Sha256Hex,
    },
    /// A proposal or legacy constant: recorded, never used for a decision.
    Proposed {
        value: T,
        reason: String,
    },
    Unresolved {
        reason: String,
        owner: String,
        blocks: Vec<String>,
    },
}

impl<T> Field<T> {
    pub fn admitted(&self) -> Option<&T> {
        match self {
            Self::Admitted { value, .. } => Some(value),
            _ => None,
        }
    }
    pub fn source(&self) -> Option<&Sha256Hex> {
        match self {
            Self::Admitted { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Three-valued judgement. A failed measurement against an admitted threshold is `Fail`;
/// missing evidence or thresholds are `Unresolved`, never `Pass` and never a physical `Fail`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "verdict", content = "reasons", rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail(Vec<String>),
    Unresolved(Vec<String>),
}

impl Verdict {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }
    /// Fail dominates, then Unresolved; reasons are sorted and deduplicated.
    pub fn combine(parts: impl IntoIterator<Item = Verdict>) -> Verdict {
        let (mut fail, mut open) = (Vec::new(), Vec::new());
        for part in parts {
            match part {
                Verdict::Pass => {}
                Verdict::Fail(r) => fail.extend(r),
                Verdict::Unresolved(r) => open.extend(r),
            }
        }
        for list in [&mut fail, &mut open] {
            list.sort();
            list.dedup();
        }
        if !fail.is_empty() {
            Verdict::Fail(fail)
        } else if !open.is_empty() {
            Verdict::Unresolved(open)
        } else {
            Verdict::Pass
        }
    }
}

/// Finite-positive check shared by the judges.
pub(crate) fn positive(name: &str, value: f64) -> Result<f64, JudgeError> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(invalid(format!("{name} must be finite and positive")))
    }
}
