use super::*;

impl DecisionProfile {
    pub(super) fn check_bound(
        value: &RequirementValue<BoundedValue>,
        unit: DecisionUnit,
        registry: &Registry,
        requirements: &ContentRef,
        applicability: &ContentRef,
    ) -> Result<(), DecisionError> {
        value.check()?;
        let RequirementValue::Admitted {
            value,
            source,
            applicability: scope,
        } = value
        else {
            return Err(DecisionError::Unresolved(vec![
                "uncertainty_bound_not_admitted".into(),
            ]));
        };
        value.check()?;
        let stored: SourceValue = decode_reviewed(registry, source, requirements)?;
        if value.unit != unit
            || value.lower.get() < 0.0
            || scope != applicability
            || stored.value != *value
            || stored.applicability != *scope
            || stored.comparison != Comparison::AtMost
        {
            return Err(invalid("uncertainty_source_unit_value_or_scope"));
        }
        reviewed(registry, scope, requirements)?;
        Ok(())
    }
    pub(super) fn check_inference(&self, registry: &Registry) -> Result<(), DecisionError> {
        let method = &self.statistical_method;
        let policy: FamilyPolicy =
            decode_reviewed(registry, &method.family_policy, &self.requirements)?;
        if policy.experiment_family_id != self.experiment_family_id
            || policy.requirements != self.requirements
            || policy.max_attempts == 0
            || self.attempt > policy.max_attempts
            || !(0.0 < policy.family_alpha.get() && policy.family_alpha.get() < 1.0)
            || policy.criterion != method.criterion
            || policy.procedure != method.procedure
            || policy.sidedness != method.sidedness
            || policy.claim_scope != self.claim_scope
            || policy.sampling_design != self.sampling_design
            || policy.joint_model != self.joint_model
            || policy.selection_policy != self.selection_policy
            || policy.stopping_policy != self.stopping_policy
            || policy.holdout_policy != self.holdout_policy
            || policy.operating_population != self.operating_population
        {
            return Err(invalid("inference_family_policy_or_reference_mismatch"));
        }
        if policy.selection == Selection::FixedCandidate
            && policy.fixed_subject.as_ref() != Some(&self.subject)
        {
            return Err(invalid("fixed_candidate_subject_changed"));
        }
        let sampling: SamplingDesign =
            decode_reviewed(registry, &self.sampling_design, &self.requirements)?;
        id(&sampling.independent_unit)?;
        id(&sampling.pairing)?;
        reviewed(
            registry,
            &sampling.correlation_treatment,
            &self.requirements,
        )?;
        let joint: JointModel = decode_reviewed(registry, &self.joint_model, &self.requirements)?;
        if joint.sampling_design != self.sampling_design || joint.assumptions.is_empty() {
            return Err(invalid("unjustified_joint_sampling_model"));
        }
        for r in &joint.assumptions {
            reviewed(registry, r, &self.requirements)?;
        }
        if sampling.kind == SamplingKind::FiniteEnvelope
            && self
                .estimands
                .iter()
                .any(|e| matches!(e.kind, EstimandKind::Mean | EstimandKind::Quantile))
        {
            return Err(invalid("finite_envelope_is_not_iid_tail_distribution"));
        }
        let expected: BTreeSet<_> = self.gate_scenario_family.iter().cloned().collect();
        let allocated: BTreeSet<_> = method
            .allocations
            .iter()
            .map(|a| a.member.clone())
            .collect();
        if expected != allocated
            || allocated.len() != method.allocations.len()
            || method
                .allocations
                .iter()
                .any(|a| !(0.0 < a.alpha.get() && a.alpha.get() < 1.0))
        {
            return Err(invalid("incomplete_alpha_allocation"));
        }
        let attempt_budget = policy.family_alpha.get() / f64::from(policy.max_attempts);
        match (method.criterion, method.procedure, policy.selection) {
            (
                ErrorCriterion::FixedCandidateConjunction,
                Procedure::IntersectionUnion,
                Selection::FixedCandidate,
            ) if method.sidedness != Sidedness::TwoSided => {
                if method
                    .allocations
                    .iter()
                    .any(|a| a.alpha.get() > attempt_budget)
                {
                    return Err(invalid("conjunction_attempt_alpha_exceeded"));
                }
            }
            (
                ErrorCriterion::SimultaneousCoverage | ErrorCriterion::AdaptiveWinnerQualification,
                Procedure::Bonferroni,
                _,
            ) => {
                if method
                    .allocations
                    .iter()
                    .map(|a| a.alpha.get())
                    .sum::<f64>()
                    > attempt_budget
                {
                    return Err(invalid("simultaneous_or_adaptive_alpha_exceeded"));
                }
            }
            _ => {
                return Err(invalid(
                    "unsupported_error_criterion_procedure_or_selection",
                ))
            }
        }
        Ok(())
    }
    pub fn qualified_admission(
        &self,
        requirements: &RequirementSet,
        registry: &Registry,
    ) -> Result<(), DecisionError> {
        self.validate(requirements, registry)?;
        let mut blockers = requirements.qualified_blockers(&self.claim_scope);
        for l in &requirements.limits {
            if !l.mandatory_for.contains(&self.claim_scope) {
                continue;
            }
            if let RequirementValue::Admitted {
                value,
                source,
                applicability,
            } = &l.value
            {
                let stored: LimitSource = decode_reviewed(registry, source, &self.requirements)?;
                stored.value.check()?;
                if stored.limit_id != l.id
                    || stored.value != *value
                    || stored.comparison != l.comparison
                    || stored.applicability != *applicability
                    || applicability != &self.applicability
                {
                    return Err(invalid(format!("limit_source_drift:{}", l.id)));
                }
            }
        }
        for e in &requirements.external_inputs {
            if e.blocks.contains(&self.claim_scope) {
                if let RequirementValue::Admitted {
                    value,
                    source,
                    applicability,
                } = &e.evidence
                {
                    let stored: ExternalSource =
                        decode_reviewed(registry, source, &self.requirements)?;
                    if &stored.artifact != value
                        || stored.input_id != e.id
                        || stored.claims != e.blocks
                        || stored.applicability != *applicability
                        || applicability != &self.applicability
                    {
                        return Err(invalid("external_source_scope_or_value"));
                    }
                    reviewed(registry, value, &self.requirements)?;
                }
            }
        }
        for c in &requirements.conflicts {
            if c.blocks.contains(&self.claim_scope) {
                if let Some(r) = &c.resolution {
                    let stored: ConflictResolution =
                        decode_reviewed(registry, r, &self.requirements)?;
                    if stored.conflict_id != c.id
                        || stored.alternatives != c.alternatives
                        || stored.applicability != self.applicability
                    {
                        return Err(invalid("conflict_resolution_source_or_scope"));
                    }
                    reviewed(registry, &stored.resolution_statement, &self.requirements)?;
                }
            }
        }
        // Evidence review readiness is not a physical verdict. Claim admission
        // still requires the trusted registry's complete observable gate closure.
        if blockers.is_empty() {
            Ok(())
        } else {
            blockers.sort();
            Err(DecisionError::Unresolved(blockers))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionCampaignMode {
    Sandbox,
    Qualified,
}
pub type CampaignMode = DecisionCampaignMode;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignRequest {
    pub schema_version: u32,
    pub mode: CampaignMode,
    pub synthetic_software_only: bool,
    pub requirements: ContentRef,
    pub decision_profile: Option<ContentRef>,
}
impl CampaignRequest {
    pub fn from_toml(bytes: &[u8]) -> Result<Self, DecisionError> {
        toml::from_str(std::str::from_utf8(bytes).map_err(|e| invalid(e.to_string()))?)
            .map_err(|e| invalid(e.to_string()))
    }
    pub fn preflight(&self, requirement_bytes: &[u8]) -> Result<RequirementSet, DecisionError> {
        let requirements = RequirementSet::from_toml(requirement_bytes)?;
        let actual =
            ContentRef::from_bytes(&requirements.id, &requirements.revision, requirement_bytes);
        if self.schema_version != 1 || self.requirements != actual {
            return Err(invalid("campaign_requirement_identity_mismatch"));
        }
        if self.mode == CampaignMode::Sandbox && self.synthetic_software_only {
            return Ok(requirements);
        }
        // This template is deliberately not an execution/admission shortcut.
        let mut blockers = requirements.qualified_blockers("computational_comparator");
        if self.decision_profile.is_none() {
            blockers.push("decision_profile_missing".into());
        }
        blockers.push("trusted_observable_gate_closure_required_before_execution".into());
        Err(DecisionError::Unresolved(blockers))
    }
}
