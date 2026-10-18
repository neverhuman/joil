//! Content-bound research evidence. Software execution never grants physical validation.
pub mod campaign;
pub mod claims;
pub mod decision;
pub mod identity;
pub mod observation;
pub mod subject;
pub mod training;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|x| format!("{x:02x}")).collect()
}

/// Requirement-scoped resolution; absence is a separate state from failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    Incomplete,
    Invalid,
    Reopened,
    Revoked,
}

/// Hash-chained, append-only receipt. Storage callers must persist before publishing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub sequence: u64,
    pub previous_sha256: Option<String>,
    pub payload: identity::ContentRef,
}

impl Event {
    pub fn sha256(&self) -> Result<String, String> {
        self.payload.check()?;
        match (self.sequence, &self.previous_sha256) {
            (1, None) => (),
            (2.., Some(hash)) if identity::is_hash(hash) => (),
            _ => return Err("invalid_event_chain".into()),
        }
        identity::scientific_hash(self)
    }
}

pub fn verify_event_chain(events: &[Event]) -> Result<(), String> {
    let mut previous = None;
    for (i, event) in events.iter().enumerate() {
        if event.sequence != i as u64 + 1 || event.previous_sha256 != previous {
            return Err("event_chain_gap_or_mutation".into());
        }
        previous = Some(event.sha256()?);
    }
    Ok(())
}

/// Schedule-independent sample identity (worker/thread count is absent).
pub fn sample_identity(campaign: &str, seed: u64, index: u64) -> String {
    let mut hash = Sha256::new();
    hash.update(b"joil.sample.v1\0");
    hash.update((campaign.len() as u64).to_be_bytes());
    hash.update(campaign.as_bytes());
    hash.update(seed.to_be_bytes());
    hash.update(index.to_be_bytes());
    hex(&hash.finalize())
}
