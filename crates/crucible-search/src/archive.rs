use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Search evidence class is explicit. Unknown or failed evidence is never feasible.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Exploration,
    Screen,
    Qualified,
    Unknown,
    NumericalFailure,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub proposal_id: u64,
    pub class: EvidenceClass,
    /// Measured, evaluator-produced descriptor values. Never derived from a family label.
    pub descriptors: Vec<f64>,
    /// All objectives are minimization objectives and must be finite.
    pub objectives: Vec<f64>,
    pub feasible: bool,
}

impl Candidate {
    fn usable(&self, class: EvidenceClass) -> bool {
        self.class == class
            && self.feasible
            && !self.objectives.is_empty()
            && self.objectives.iter().all(|v| v.is_finite())
            && self.descriptors.iter().all(|v| v.is_finite())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArchiveConfig {
    pub capacity_per_pool: usize,
    pub nondominated_capacity_per_descriptor_bin: usize,
    pub descriptor_bin_widths: Vec<f64>,
    pub protection_lifetime_epochs: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SearchArchive {
    pub exploration: Vec<Candidate>,
    pub screen: Vec<Candidate>,
    pub qualified: Vec<Candidate>,
    /// Append-only evidence history. Pruning working sets never prunes this journal.
    pub evidence_front: Vec<Candidate>,
    pub protected_until: BTreeMap<u64, u64>,
}

impl SearchArchive {
    /// Persist the unpruned evidence history as a private immutable archive object.
    pub fn persist_evidence_front(
        &self,
        store: &crucible_store::Store,
    ) -> crucible_store::Result<crucible_store::BlobDigest> {
        let bytes = serde_json::to_vec(&self.evidence_front)
            .map_err(|e| crucible_store::StoreError::Wire(e.to_string()))?;
        store.publish(
            &bytes,
            crucible_store::manifest::Metadata::private(
                crucible_store::manifest::ObjectKind::Checkpoint,
            ),
        )
    }

    pub fn load_evidence_front(
        store: &crucible_store::Store,
        id: &crucible_store::BlobDigest,
    ) -> crucible_store::Result<Vec<Candidate>> {
        let object = store.read(id)?;
        serde_json::from_slice(&object.payload)
            .map_err(|e| crucible_store::StoreError::Wire(e.to_string()))
    }

    pub fn insert(&mut self, candidate: Candidate, epoch: u64, cfg: &ArchiveConfig) {
        let class = candidate.class;
        self.evidence_front.push(candidate.clone());
        let pool = match class {
            EvidenceClass::Exploration => &mut self.exploration,
            EvidenceClass::Screen => &mut self.screen,
            EvidenceClass::Qualified => &mut self.qualified,
            EvidenceClass::Unknown
            | EvidenceClass::NumericalFailure
            | EvidenceClass::Unsupported => return,
        };
        if !candidate.usable(class) || cfg.capacity_per_pool == 0 {
            return;
        }
        if class == EvidenceClass::Qualified
            && candidate.descriptors.len() != cfg.descriptor_bin_widths.len()
        {
            return;
        }
        pool.push(candidate);
        let mut bins: BTreeMap<Vec<i64>, Vec<Candidate>> = BTreeMap::new();
        for c in pool.drain(..) {
            let key = if class == EvidenceClass::Qualified {
                c.descriptors
                    .iter()
                    .zip(&cfg.descriptor_bin_widths)
                    .map(|(v, w)| (v / w).floor() as i64)
                    .collect()
            } else {
                Vec::new()
            };
            let bin = bins.entry(key).or_default();
            if !bin.iter().any(|old| dominates(old, &c)) {
                bin.retain(|old| !dominates(&c, old));
                bin.push(c);
            }
        }
        for bin in bins.values_mut() {
            bin.sort_by_key(|c| c.proposal_id);
            if class == EvidenceClass::Qualified
                && bin.len() > cfg.nondominated_capacity_per_descriptor_bin
            {
                bin.sort_by(|a, b| {
                    lex_objective(a)
                        .total_cmp(&lex_objective(b))
                        .then(a.proposal_id.cmp(&b.proposal_id))
                });
                bin.truncate(cfg.nondominated_capacity_per_descriptor_bin);
            }
        }
        *pool = bins.into_values().flatten().collect();
        pool.sort_by_key(|c| c.proposal_id);
        if pool.len() > cfg.capacity_per_pool {
            pool.truncate(cfg.capacity_per_pool);
        }
        self.protected_until.insert(
            self.evidence_front.last().unwrap().proposal_id,
            epoch.saturating_add(cfg.protection_lifetime_epochs),
        );
    }

    pub fn expire_protection(&mut self, epoch: u64) {
        self.protected_until.retain(|_, until| *until > epoch);
    }
}

fn lex_objective(c: &Candidate) -> f64 {
    c.objectives[0]
}
fn dominates(a: &Candidate, b: &Candidate) -> bool {
    a.objectives.len() == b.objectives.len()
        && a.objectives.iter().zip(&b.objectives).all(|(x, y)| x <= y)
        && a.objectives.iter().zip(&b.objectives).any(|(x, y)| x < y)
}
