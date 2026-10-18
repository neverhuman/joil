//! Frozen criterion inventories and transitive, revocable evidence closure.
use crate::identity::{check_id, scientific_hash, strict_decode, ContentRef, ScientificIdentity};
use crate::observation::{EvidenceTier, Finite, Observation, Unit};
use crate::Verdict;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

fn verify_bytes(reference: &ContentRef, bytes: &[u8]) -> Result<(), String> {
    reference.check()?;
    if ContentRef::from_bytes(&reference.id, &reference.revision, bytes) != *reference {
        return Err("artifact_bytes_mismatch".into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationNode {
    pub id: String,
    pub observation: Observation,
    pub dependencies: Vec<ContentRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub id: String,
    pub observation: ContentRef,
    pub quantity: String,
    pub unit: Unit,
    pub minimum_tier: EvidenceTier,
    pub at_most: Finite,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimInventory {
    pub id: String,
    pub identity: ScientificIdentity,
    pub criteria: Vec<Criterion>,
    pub dependencies: Vec<ContentRef>,
}

/// Explicit application trust boundary, never reconstructed from incoming claim policy.
/// This crate verifies bytes/scope; the caller must authenticate the approving authority.
pub struct ApprovedInventory {
    reference: ContentRef,
    inventory: ClaimInventory,
}
impl ApprovedInventory {
    pub fn from_approved_bytes(
        reference: ContentRef,
        approved_requirements: &[ContentRef],
        bytes: &[u8],
    ) -> Result<Self, String> {
        verify_bytes(&reference, bytes)?;
        let inventory: ClaimInventory = strict_decode(bytes)?;
        inventory.identity.check()?;
        if reference.id != inventory.id
            || inventory.criteria.is_empty()
            || inventory.identity.requirements != approved_requirements
        {
            return Err("inventory_scope_or_criteria_mismatch".into());
        }
        crate::identity::unique_ids(inventory.criteria.iter().map(|c| c.id.as_str()))?;
        for criterion in &inventory.criteria {
            criterion.observation.check()?;
            check_id(&criterion.quantity)?;
        }
        for dependency in &inventory.dependencies {
            dependency.check()?;
        }
        Ok(Self {
            reference,
            inventory,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewAction {
    Approved,
    Reopened,
    Revoked,
    Superseded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewEvent {
    pub sequence: u64,
    pub subject: ContentRef,
    pub action: ReviewAction,
    pub reviewer: String,
    pub requirements: Vec<ContentRef>,
}

#[derive(Default)]
pub struct Registry {
    artifacts: BTreeMap<String, (ContentRef, Vec<u8>)>,
    observations: BTreeMap<String, (ContentRef, ObservationNode)>,
    claims: BTreeMap<String, ApprovedInventory>,
    reviews: Vec<ReviewEvent>,
}

impl Registry {
    /// Read only exact, reviewed artifact bytes under the caller's admitted requirement scope.
    pub fn checked_artifact(
        &self,
        reference: &ContentRef,
        requirements: &[ContentRef],
    ) -> Result<&[u8], String> {
        if !self.has_artifact(reference) || self.reviewed(reference, requirements) != Verdict::Pass
        {
            return Err("artifact_not_reviewed_or_identity_mismatch".into());
        }
        Ok(&self.artifacts[&reference.id].1)
    }

    pub fn checked_measurement(
        &self,
        reference: &ContentRef,
        requirements: &[ContentRef],
    ) -> Result<&crate::observation::Measurement, String> {
        if self.resolve_observation(reference, requirements, &mut BTreeSet::new()) != Verdict::Pass
        {
            return Err("observation_evidence_closure_incomplete_or_revoked".into());
        }
        match &self.observations[&reference.id].1.observation {
            Observation::Measured { measurement } => Ok(measurement),
            _ => Err("training_requires_measured_observation".into()),
        }
    }

    pub fn insert_artifact(&mut self, reference: ContentRef, bytes: Vec<u8>) -> Result<(), String> {
        verify_bytes(&reference, &bytes)?;
        if let Some((old, _)) = self.artifacts.get(&reference.id) {
            if old != &reference {
                return Err("artifact_identity_conflict".into());
            }
        }
        self.artifacts
            .insert(reference.id.clone(), (reference, bytes));
        Ok(())
    }

    pub fn insert_observation(
        &mut self,
        reference: ContentRef,
        bytes: Vec<u8>,
    ) -> Result<(), String> {
        verify_bytes(&reference, &bytes)?;
        let node: ObservationNode = strict_decode(&bytes)?;
        check_id(&node.id)?;
        node.observation.check()?;
        if node.id != reference.id {
            return Err("observation_id_mismatch".into());
        }
        crate::identity::unique_ids(node.dependencies.iter().map(|d| d.id.as_str()))?;
        for dependency in &node.dependencies {
            dependency.check()?;
        }
        self.insert_artifact(reference.clone(), bytes)?;
        self.observations.insert(node.id.clone(), (reference, node));
        Ok(())
    }

    pub fn install_inventory(&mut self, approved: ApprovedInventory) -> Result<(), String> {
        if self.claims.contains_key(&approved.reference.id)
            || self.artifacts.contains_key(&approved.reference.id)
        {
            return Err("inventory_already_installed".into());
        }
        self.claims.insert(approved.reference.id.clone(), approved);
        Ok(())
    }

    /// Approval parameters come from authenticated application configuration, not event data.
    pub fn append_approved_review(
        &mut self,
        approved_reviewer: &str,
        approved_requirements: &[ContentRef],
        receipt: ContentRef,
        bytes: Vec<u8>,
    ) -> Result<(), String> {
        verify_bytes(&receipt, &bytes)?;
        let event: ReviewEvent = strict_decode(&bytes)?;
        event.subject.check()?;
        check_id(&event.reviewer)?;
        if event.sequence != self.reviews.len() as u64 + 1
            || event.reviewer != approved_reviewer
            || event.requirements.is_empty()
            || event.requirements != approved_requirements
        {
            return Err("review_authority_scope_or_sequence_mismatch".into());
        }
        self.insert_artifact(receipt, bytes)?;
        self.reviews.push(event);
        Ok(())
    }

    fn reviewed(&self, reference: &ContentRef, requirements: &[ContentRef]) -> Verdict {
        match self
            .reviews
            .iter()
            .rev()
            .find(|e| &e.subject == reference && e.requirements == requirements)
            .map(|e| e.action)
        {
            Some(ReviewAction::Approved) => Verdict::Pass,
            Some(ReviewAction::Revoked | ReviewAction::Superseded) => Verdict::Revoked,
            Some(ReviewAction::Reopened) => Verdict::Reopened,
            None => Verdict::Incomplete,
        }
    }

    fn has_artifact(&self, reference: &ContentRef) -> bool {
        self.artifacts
            .get(&reference.id)
            .is_some_and(|(r, bytes)| r == reference && verify_bytes(r, bytes).is_ok())
    }

    pub fn resolve(&self, reference: &ContentRef) -> Verdict {
        self.resolve_inner(reference, &mut BTreeSet::new())
    }

    /// Resolve a claim only under the caller's admitted requirements. The
    /// legacy resolver intentionally uses the inventory's own requirement scope.
    pub fn resolve_in_scope(&self, reference: &ContentRef, requirements: &[ContentRef]) -> Verdict {
        if self
            .claims
            .get(&reference.id)
            .is_some_and(|approved| approved.inventory.identity.requirements != requirements)
        {
            return Verdict::Invalid;
        }
        self.resolve(reference)
    }

    fn resolve_inner(&self, reference: &ContentRef, visiting: &mut BTreeSet<String>) -> Verdict {
        if reference.check().is_err() || !visiting.insert(reference.id.clone()) {
            return Verdict::Invalid;
        }
        let result = if let Some(approved) = self.claims.get(&reference.id) {
            if &approved.reference != reference {
                Verdict::Invalid
            } else {
                self.resolve_inventory(approved, visiting)
            }
        } else {
            Verdict::Incomplete
        };
        visiting.remove(&reference.id);
        result
    }

    fn resolve_inventory(
        &self,
        approved: &ApprovedInventory,
        visiting: &mut BTreeSet<String>,
    ) -> Verdict {
        let inventory = &approved.inventory;
        let requirements = &inventory.identity.requirements;
        let mut results = vec![self.reviewed(&approved.reference, requirements)];
        for dependency in &inventory.dependencies {
            results.push(self.resolve_inner(dependency, visiting));
        }
        for criterion in &inventory.criteria {
            results.push(self.resolve_observation(&criterion.observation, requirements, visiting));
            let Some((stored, node)) = self.observations.get(&criterion.observation.id) else {
                continue;
            };
            if stored != &criterion.observation {
                results.push(Verdict::Invalid);
                continue;
            }
            if let Observation::Measured { measurement: m } = &node.observation {
                if m.identity != inventory.identity || m.quantity != criterion.quantity {
                    results.push(Verdict::Invalid);
                }
            }
            results.push(node.observation.compare_at_most(
                criterion.at_most,
                criterion.unit,
                criterion.minimum_tier,
            ));
        }
        combine(&results)
    }

    fn resolve_observation(
        &self,
        reference: &ContentRef,
        requirements: &[ContentRef],
        visiting: &mut BTreeSet<String>,
    ) -> Verdict {
        if !visiting.insert(reference.id.clone()) {
            return Verdict::Invalid;
        }
        let result = (|| {
            let Some((stored, node)) = self.observations.get(&reference.id) else {
                return Verdict::Incomplete;
            };
            if stored != reference
                || !self.has_artifact(reference)
                || node.observation.check().is_err()
            {
                return Verdict::Invalid;
            }
            let mut results = vec![self.reviewed(reference, requirements)];
            for dependency in &node.dependencies {
                results.push(self.resolve_observation(dependency, requirements, visiting));
            }
            if let Observation::Measured { measurement: m } = &node.observation {
                let mut refs = m.identity.refs();
                refs.extend(m.evidence.iter());
                refs.extend([&m.recovery, &m.applicability]);
                if refs.iter().any(|r| !self.has_artifact(r)) {
                    results.push(Verdict::Incomplete);
                }
            } else {
                results.push(Verdict::Incomplete);
            }
            combine(&results)
        })();
        visiting.remove(&reference.id);
        result
    }

    pub fn inventory_digest(&self, reference: &ContentRef) -> Result<String, String> {
        let inventory = self.claims.get(&reference.id).ok_or("missing_inventory")?;
        if &inventory.reference != reference {
            return Err("inventory_identity_mismatch".into());
        }
        scientific_hash(&inventory.inventory)
    }
}

fn combine(results: &[Verdict]) -> Verdict {
    for state in [
        Verdict::Revoked,
        Verdict::Invalid,
        Verdict::Reopened,
        Verdict::Fail,
        Verdict::Incomplete,
    ] {
        if results.contains(&state) {
            return state;
        }
    }
    Verdict::Pass
}
