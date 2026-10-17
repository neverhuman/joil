use crate::{manifest::Manifest, BlobDigest, Result, StoreError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidationKind {
    Revoked,
    Reopened,
    Superseded,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invalidation {
    pub sequence: u64,
    pub source: BlobDigest,
    pub kind: InvalidationKind,
    pub reason: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Freshness {
    Current,
    Stale { events: Vec<u64> },
}

/// A complete closure is required before either freshness or retention is used.
pub fn transitive_closure(
    roots: &BTreeSet<BlobDigest>,
    manifests: &BTreeMap<BlobDigest, Manifest>,
) -> Result<BTreeSet<BlobDigest>> {
    fn visit(
        id: &BlobDigest,
        graph: &BTreeMap<BlobDigest, Manifest>,
        active: &mut BTreeSet<BlobDigest>,
        done: &mut BTreeSet<BlobDigest>,
    ) -> Result<()> {
        if done.contains(id) {
            return Ok(());
        }
        if active.len() >= 128 {
            return Err(StoreError::Invalid("closure_depth_budget"));
        }
        if !active.insert(id.clone()) {
            return Err(StoreError::Cycle(id.clone()));
        }
        let manifest = graph
            .get(id)
            .ok_or_else(|| StoreError::Missing(id.clone()))?;
        for parent in &manifest.metadata.dependencies {
            visit(parent, graph, active, done)?;
        }
        active.remove(id);
        done.insert(id.clone());
        Ok(())
    }
    let mut done = BTreeSet::new();
    for root in roots {
        visit(root, manifests, &mut BTreeSet::new(), &mut done)?;
    }
    Ok(done)
}
pub fn freshness(closure: &BTreeSet<BlobDigest>, events: &[Invalidation]) -> Freshness {
    let events: Vec<_> = events
        .iter()
        .filter(|e| closure.contains(&e.source))
        .map(|e| e.sequence)
        .collect();
    if events.is_empty() {
        Freshness::Current
    } else {
        Freshness::Stale { events }
    }
}
