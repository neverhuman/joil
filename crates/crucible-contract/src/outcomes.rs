//! Worker outcomes have no direct conversion into a physical verdict.
use crate::subject::BlobDigest;
use crate::ContractError;
use joil_evidence::claims::Registry;
use joil_evidence::identity::{check_id, strict_decode, ContentRef};
use joil_evidence::Verdict;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskOutcome {
    Succeeded { output_manifest: Option<BlobDigest> },
    NumericalFailure { diagnostic: Option<BlobDigest> },
    InfrastructureFailure { diagnostic: Option<BlobDigest> },
    Unsupported,
    NotRun,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationState {
    Measured { claim: ContentRef },
    Missing,
    Invalid,
    OutOfScope,
    NotApplicable { rule: ContentRef },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NonPassingReason {
    NotRun,
    NumericalFailure,
    InfrastructureFailure,
    Unsupported,
    AbsentOutput,
    MissingObservation,
    InvalidObservation,
    OutOfScope,
    UnregisteredNotApplicable,
    SourceMismatch,
    UnreviewedPolicy,
    IncompleteEvidence,
    InvalidEvidence,
    ReopenedEvidence,
    RevokedEvidence,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum GateState {
    Pass { claim: ContentRef },
    Fail { claim: ContentRef },
    Unknown { reason: NonPassingReason },
    NotApplicable { rule: ContentRef },
}
/// Decisions are constructed by current registry resolution, never by workers.
/// ```compile_fail
/// use crucible_contract::outcomes::{GateDecision,GateState,NonPassingReason};
/// use joil_evidence::identity::ContentRef;
/// let r=ContentRef::from_bytes("synthetic", "1", b"software only");
/// let forged=GateDecision {state:GateState::Unknown {reason:NonPassingReason::AbsentOutput},policy:r.clone(),requirements:r,gate_id:"synthetic".into()};
/// ```
/// ```compile_fail
/// let _=serde_json::from_str::<crucible_contract::outcomes::GateDecision>("{}");
/// ```
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GateDecision {
    state: GateState,
    policy: ContentRef,
    requirements: ContentRef,
    gate_id: String,
}
impl GateDecision {
    pub fn state(&self) -> &GateState {
        &self.state
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatePolicyRecord {
    pub id: String,
    pub claim: ContentRef,
    pub not_applicable_rule: Option<ContentRef>,
}
pub struct GatePolicy {
    record: GatePolicyRecord,
    reference: ContentRef,
    requirements: ContentRef,
}
impl GatePolicy {
    fn result(&self, state: GateState) -> GateDecision {
        GateDecision {
            state,
            policy: self.reference.clone(),
            requirements: self.requirements.clone(),
            gate_id: self.record.id.clone(),
        }
    }
    pub fn from_approved_registry(
        registry: &Registry,
        policy: &ContentRef,
        requirements: &ContentRef,
    ) -> Result<Self, ContractError> {
        let bytes = registry
            .checked_artifact(policy, std::slice::from_ref(requirements))
            .map_err(ContractError::Wire)?;
        let record: GatePolicyRecord = strict_decode(bytes).map_err(ContractError::Wire)?;
        check_id(&record.id).map_err(ContractError::Wire)?;
        record.claim.check().map_err(ContractError::Wire)?;
        if let Some(r) = &record.not_applicable_rule {
            r.check().map_err(ContractError::Wire)?;
        }
        Ok(Self {
            record,
            reference: policy.clone(),
            requirements: requirements.clone(),
        })
    }
    pub fn id(&self) -> &str {
        &self.record.id
    }
    pub fn decide(
        &self,
        task: &TaskOutcome,
        observation: &ObservationState,
        registry: &Registry,
    ) -> GateDecision {
        use NonPassingReason as N;
        let unknown = |reason| self.result(GateState::Unknown { reason });
        match task {
            TaskOutcome::NumericalFailure { .. } => return unknown(N::NumericalFailure),
            TaskOutcome::InfrastructureFailure { .. } => return unknown(N::InfrastructureFailure),
            TaskOutcome::Unsupported => return unknown(N::Unsupported),
            TaskOutcome::NotRun => return unknown(N::NotRun),
            TaskOutcome::Succeeded {
                output_manifest: None,
            } => return unknown(N::AbsentOutput),
            TaskOutcome::Succeeded {
                output_manifest: Some(_),
            } => (),
        }
        if registry
            .checked_artifact(&self.reference, std::slice::from_ref(&self.requirements))
            .is_err()
        {
            return unknown(N::UnreviewedPolicy);
        }
        let reference = match observation {
            ObservationState::Missing => return unknown(N::MissingObservation),
            ObservationState::Invalid => return unknown(N::InvalidObservation),
            ObservationState::OutOfScope => return unknown(N::OutOfScope),
            ObservationState::NotApplicable { rule } => {
                if self.record.not_applicable_rule.as_ref() != Some(rule) {
                    return unknown(N::UnregisteredNotApplicable);
                }
                return if registry.resolve_in_scope(rule, std::slice::from_ref(&self.requirements))
                    == Verdict::Pass
                {
                    self.result(GateState::NotApplicable { rule: rule.clone() })
                } else {
                    unknown(N::IncompleteEvidence)
                };
            }
            ObservationState::Measured { claim } => claim,
        };
        if reference != &self.record.claim {
            return unknown(N::SourceMismatch);
        }
        self.result(
            match registry.resolve_in_scope(reference, std::slice::from_ref(&self.requirements)) {
                Verdict::Pass => GateState::Pass {
                    claim: reference.clone(),
                },
                Verdict::Fail => GateState::Fail {
                    claim: reference.clone(),
                },
                Verdict::Incomplete => GateState::Unknown {
                    reason: N::IncompleteEvidence,
                },
                Verdict::Invalid => GateState::Unknown {
                    reason: N::InvalidEvidence,
                },
                Verdict::Reopened => GateState::Unknown {
                    reason: N::ReopenedEvidence,
                },
                Verdict::Revoked => GateState::Unknown {
                    reason: N::RevokedEvidence,
                },
            },
        )
    }
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EligibilityState {
    Blocked { gates: Vec<String> },
    ReadyForEvidenceReview { claims: Vec<ContentRef> },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateInventoryRecord {
    pub id: String,
    pub requirements: ContentRef,
    pub gates: Vec<GatePolicyRecord>,
}
pub struct ApprovedGateInventory {
    reference: ContentRef,
    record: GateInventoryRecord,
}
impl ApprovedGateInventory {
    pub fn from_approved_registry(
        registry: &Registry,
        reference: &ContentRef,
        requirements: &ContentRef,
    ) -> Result<Self, ContractError> {
        let bytes = registry
            .checked_artifact(reference, std::slice::from_ref(requirements))
            .map_err(ContractError::Wire)?;
        let record: GateInventoryRecord = strict_decode(bytes).map_err(ContractError::Wire)?;
        if record.requirements != *requirements || record.gates.is_empty() {
            return Err(ContractError::InvalidKey(
                "empty_or_mismatched_gate_inventory",
            ));
        }
        check_id(&record.id).map_err(ContractError::Wire)?;
        joil_evidence::identity::unique_ids(record.gates.iter().map(|g| g.id.as_str()))
            .map_err(ContractError::Wire)?;
        for g in &record.gates {
            g.claim.check().map_err(ContractError::Wire)?;
            if let Some(r) = &g.not_applicable_rule {
                r.check().map_err(ContractError::Wire)?;
            }
        }
        Ok(Self {
            reference: reference.clone(),
            record,
        })
    }
}
/// ```compile_fail
/// let _=serde_json::from_str::<crucible_contract::outcomes::ClaimEligibility>("{}");
/// ```
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ClaimEligibility {
    state: EligibilityState,
}
impl ClaimEligibility {
    pub fn state(&self) -> &EligibilityState {
        &self.state
    }
    pub fn from_gate_decisions(
        inventory: &ApprovedGateInventory,
        actual: &BTreeMap<String, GateDecision>,
        registry: &Registry,
    ) -> Self {
        let required: BTreeSet<_> = inventory
            .record
            .gates
            .iter()
            .map(|g| g.id.clone())
            .collect();
        let actual_ids: BTreeSet<_> = actual.keys().cloned().collect();
        if registry
            .checked_artifact(
                &inventory.reference,
                std::slice::from_ref(&inventory.record.requirements),
            )
            .is_err()
            || required.is_empty()
            || actual_ids != required
        {
            return Self {
                state: EligibilityState::Blocked {
                    gates: vec!["incomplete_or_unreviewed_gate_inventory".into()],
                },
            };
        }
        let mut blocked = Vec::new();
        let mut claims = Vec::new();
        for (id, decision) in actual {
            let required = inventory.record.gates.iter().find(|g| &g.id == id).unwrap();
            if decision.gate_id != *id
                || decision.requirements != inventory.record.requirements
                || registry
                    .checked_artifact(
                        &decision.policy,
                        std::slice::from_ref(&decision.requirements),
                    )
                    .is_err()
            {
                blocked.push(id.clone());
                continue;
            }
            match &decision.state {
                GateState::Pass { claim }
                    if claim == &required.claim
                        && registry.resolve_in_scope(
                            claim,
                            std::slice::from_ref(&inventory.record.requirements),
                        ) == Verdict::Pass =>
                {
                    claims.push(claim.clone())
                }
                GateState::NotApplicable { rule }
                    if required.not_applicable_rule.as_ref() == Some(rule)
                        && registry.resolve_in_scope(
                            rule,
                            std::slice::from_ref(&inventory.record.requirements),
                        ) == Verdict::Pass =>
                {
                    claims.push(rule.clone())
                }
                _ => blocked.push(id.clone()),
            }
        }
        Self {
            state: if blocked.is_empty() {
                EligibilityState::ReadyForEvidenceReview { claims }
            } else {
                EligibilityState::Blocked { gates: blocked }
            },
        }
    }
}
