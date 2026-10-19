//! Synthetic policy fixtures, never physical source admission or inference validation.
use joil_evidence::claims::{Registry, ReviewAction, ReviewEvent};
use joil_evidence::decision::*;
use joil_evidence::identity::ContentRef;
use joil_evidence::observation::Finite;
use serde::Serialize;

const DRAFT: &[u8] = include_bytes!("../../../requirements/joil-hydrocyclone-v2.toml");
fn f(x: f64) -> Finite {
    Finite::new(x).unwrap()
}
fn bounded(unit: DecisionUnit, lower: f64, upper: f64) -> BoundedValue {
    BoundedValue {
        unit,
        lower: f(lower),
        upper: f(upper),
    }
}
struct Fixture {
    registry: Registry,
    sequence: u64,
    requirements: RequirementSet,
    reference: ContentRef,
    profile: DecisionProfile,
    policy: FamilyPolicy,
}
impl Fixture {
    fn approve(&mut self, reference: ContentRef, bytes: Vec<u8>) {
        self.registry
            .insert_artifact(reference.clone(), bytes)
            .unwrap();
        self.review(reference, ReviewAction::Approved);
    }
    fn review(&mut self, reference: ContentRef, action: ReviewAction) {
        self.sequence += 1;
        let event = ReviewEvent {
            sequence: self.sequence,
            subject: reference,
            action,
            reviewer: "synthetic-policy-reviewer".into(),
            requirements: vec![self.reference.clone()],
        };
        let bytes = serde_json::to_vec(&event).unwrap();
        self.registry
            .append_approved_review(
                "synthetic-policy-reviewer",
                std::slice::from_ref(&self.reference),
                ContentRef::from_bytes(&format!("synthetic-review-{}", self.sequence), "1", &bytes),
                bytes,
            )
            .unwrap();
    }
    fn artifact<T: Serialize>(&mut self, id: &str, value: &T) -> ContentRef {
        let bytes = serde_json::to_vec(value).unwrap();
        let r = ContentRef::from_bytes(id, "1", &bytes);
        self.approve(r.clone(), bytes);
        r
    }
    fn new(bytes: &[u8]) -> Self {
        let requirements = RequirementSet::from_toml(bytes).unwrap();
        let reference = ContentRef::from_bytes(&requirements.id, &requirements.revision, bytes);
        // Deserialize a complete profile below, rather than constructing an empty
        // admitted object. This placeholder never reaches a validator.
        let empty = ContentRef::from_bytes("synthetic-placeholder", "1", b"{}");
        let policy = FamilyPolicy {
            experiment_family_id: "synthetic-family".into(),
            requirements: reference.clone(),
            family_alpha: f(0.05),
            max_attempts: 1,
            criterion: ErrorCriterion::SimultaneousCoverage,
            procedure: Procedure::Bonferroni,
            sidedness: Sidedness::Upper,
            selection: Selection::FixedCandidate,
            fixed_subject: None,
            claim_scope: "computational_comparator".into(),
            sampling_design: empty.clone(),
            joint_model: empty.clone(),
            selection_policy: empty.clone(),
            stopping_policy: empty.clone(),
            holdout_policy: empty.clone(),
            operating_population: empty.clone(),
        };
        let profile = DecisionProfile {
            schema_version: 1,
            experiment_family_id: "synthetic-family".into(),
            attempt: 1,
            requirements: reference.clone(),
            subject: empty.clone(),
            baseline: requirements.baseline.clone(),
            applicability: requirements.applicability.clone(),
            claim_scope: "computational_comparator".into(),
            estimands: vec![],
            gate_scenario_family: vec![],
            sampling_design: empty.clone(),
            joint_model: empty.clone(),
            numerical_bounds: vec![],
            model_discrepancy: vec![],
            statistical_method: StatisticalMethod {
                criterion: ErrorCriterion::SimultaneousCoverage,
                procedure: Procedure::Bonferroni,
                sidedness: Sidedness::Upper,
                allocations: vec![],
                family_policy: empty.clone(),
            },
            selection_policy: empty.clone(),
            stopping_policy: empty.clone(),
            holdout_policy: empty.clone(),
            operating_population: empty,
            unknown_policy: UnknownPolicy::Abstain,
            planning_precision: vec![],
            executable_acceptance_fixtures: vec![],
        };
        let mut x = Self {
            registry: Registry::default(),
            sequence: 0,
            requirements,
            reference,
            profile,
            policy,
        };
        x.approve(x.reference.clone(), bytes.to_vec());
        for (r, raw) in [
            (
                x.requirements.baseline.clone(),
                include_bytes!("../../../baselines/ct35/genome.json").as_slice(),
            ),
            (
                x.requirements.preregistration.clone(),
                include_bytes!("../../../experiments/wave2-preregistration-v1.json").as_slice(),
            ),
            (
                x.requirements.applicability.clone(),
                include_bytes!("../../../requirements/RS-joil-research-v1.json").as_slice(),
            ),
        ] {
            x.approve(r, raw.to_vec());
        }
        let synthetic = x.artifact(
            "synthetic-only-input",
            &serde_json::json!({"origin":"synthetic","purpose":"policy_fixture_not_measurement"}),
        );
        let sampling = x.artifact(
            "synthetic-sampling",
            &SamplingDesign {
                kind: SamplingKind::IndependentBlocks,
                independent_unit: "independent synthetic block".into(),
                pairing: "paired baseline/candidate block".into(),
                correlation_treatment: synthetic.clone(),
            },
        );
        let joint = x.artifact(
            "synthetic-joint-model",
            &JointModel {
                sampling_design: sampling.clone(),
                assumptions: vec![synthetic.clone()],
            },
        );
        x.profile.subject = synthetic.clone();
        x.policy.fixed_subject = Some(synthetic.clone());
        x.profile.sampling_design = sampling.clone();
        x.profile.joint_model = joint.clone();
        x.profile.selection_policy = synthetic.clone();
        x.profile.stopping_policy = synthetic.clone();
        x.profile.holdout_policy = synthetic.clone();
        x.profile.operating_population = synthetic.clone();
        x.profile.executable_acceptance_fixtures = vec![synthetic.clone()];
        x.policy.sampling_design = sampling;
        x.policy.joint_model = joint;
        x.policy.selection_policy = synthetic.clone();
        x.policy.stopping_policy = synthetic.clone();
        x.policy.holdout_policy = synthetic.clone();
        x.policy.operating_population = synthetic.clone();
        let gates = x.requirements.mandatory_gates.clone();
        for (i, g) in gates.iter().enumerate() {
            let definition = x.artifact(
                &format!("synthetic-estimand-{i}"),
                &EstimandDefinition {
                    kind: EstimandKind::Point,
                    input_domain: EstimandInputDomain::Unspecified,
                    unit: g.unit,
                    direction: Direction::Minimize,
                    population: synthetic.clone(),
                    expression: synthetic.clone(),
                },
            );
            x.profile.estimands.push(Estimand {
                id: g.estimand.clone(),
                unit: g.unit,
                direction: Direction::Minimize,
                kind: EstimandKind::Point,
                input_domain: EstimandInputDomain::Unspecified,
                population: synthetic.clone(),
                formula: definition,
            });
            let range = bounded(g.unit, 0.0, 0.01);
            let source = x.artifact(
                &format!("synthetic-bound-{i}"),
                &SourceValue {
                    value: range.clone(),
                    comparison: Comparison::AtMost,
                    applicability: x.requirements.applicability.clone(),
                },
            );
            let bound = UncertaintyBound {
                estimand: g.estimand.clone(),
                bound: RequirementValue::Admitted {
                    value: range,
                    source,
                    applicability: x.requirements.applicability.clone(),
                },
            };
            x.profile.numerical_bounds.push(bound.clone());
            x.profile.model_discrepancy.push(bound);
            x.profile.planning_precision.push(PlanningPrecision {
                estimand: g.estimand.clone(),
                useful_effect: bounded(g.unit, 1.0, 1.0),
                maximum_uncertainty_fraction: f(1.0 / 3.0),
            });
        }
        x.profile.gate_scenario_family = x
            .requirements
            .scenarios
            .iter()
            .flat_map(|s| {
                gates.iter().map(move |g| FamilyMember {
                    gate: g.id.clone(),
                    scenario: s.id.clone(),
                    estimand: g.estimand.clone(),
                })
            })
            .collect();
        let n = x.profile.gate_scenario_family.len() as f64;
        x.profile.statistical_method.allocations = x
            .profile
            .gate_scenario_family
            .iter()
            .map(|m| AlphaAllocation {
                member: m.clone(),
                alpha: f(0.04 / n),
            })
            .collect();
        x.install_policy("synthetic-family-policy");
        x
    }
    fn install_policy(&mut self, id: &str) {
        self.profile.statistical_method.family_policy = self.artifact(id, &self.policy.clone());
    }
    fn valid(&self) -> Result<(), DecisionError> {
        self.profile.validate(&self.requirements, &self.registry)
    }
}

#[test]
fn real_draft_and_sandbox_bind_exact_source_but_qualified_template_refuses() {
    let req = RequirementSet::from_toml(DRAFT).unwrap();
    let sandbox =
        CampaignRequest::from_toml(include_bytes!("../../../campaigns/ct35-v5-sandbox.toml"))
            .unwrap();
    assert_eq!(sandbox.preflight(DRAFT).unwrap(), req);
    let qualified = CampaignRequest::from_toml(include_bytes!(
        "../../../campaigns/ct35-v5-qualified.template.toml"
    ))
    .unwrap();
    let DecisionError::Unresolved(blockers) = qualified.preflight(DRAFT).unwrap_err() else {
        panic!()
    };
    for missing in [
        "pressure_budget_pa",
        "energy_budget_j_per_treated_m3",
        "absolute_pressure_min_pa",
        "continuous_duty_band_m3_h",
    ] {
        assert!(blockers.iter().any(|b| b.contains(missing)));
    }
    assert!(blockers.contains(&"decision_profile_missing".to_string()));
    let mut changed = sandbox;
    changed.requirements.sha256 = "0".repeat(64);
    assert!(matches!(
        changed.preflight(DRAFT),
        Err(DecisionError::Invalid(_))
    ));
    assert!(!req.limits.iter().any(|l| l.id == "sandbox_pdr_min"));
    let injected = [b"sandbox_pdr_min = 4.5\n".as_slice(), DRAFT].concat();
    assert!(RequirementSet::from_toml(&injected).is_err());
}
#[test]
fn complete_source_bound_synthetic_profile_is_semantically_valid_without_physical_admission() {
    let x = Fixture::new(DRAFT);
    x.valid().unwrap();
    assert!(matches!(
        x.profile.qualified_admission(&x.requirements, &x.registry),
        Err(DecisionError::Unresolved(_))
    ));
    assert_eq!(
        x.requirements
            .scenarios
            .iter()
            .map(|s| s.d50_feed_um.get())
            .collect::<Vec<_>>(),
        vec![22.0, 22.0, 14.5, 20.0]
    );
    assert_eq!(x.requirements.conflicts.len(), 7);
}
#[test]
fn proposed_load_passage_and_borrowed_pdr_never_admit() {
    let mut x = Fixture::new(DRAFT);
    for (name, unit, value) in [
        ("synthetic_2_5mpa", DecisionUnit::Pascal, 2.5e6),
        ("synthetic_printed3mm", DecisionUnit::Millimetre, 3.0),
        ("borrowed_pdr", DecisionUnit::Dimensionless, 2.6),
    ] {
        x.requirements.limits.push(LimitSpec {
            id: name.into(),
            comparison: Comparison::AtMost,
            unit,
            mandatory_for: vec!["computational_comparator".into()],
            value: RequirementValue::Proposed {
                value: bounded(unit, value, value),
                source: x.reference.clone(),
                applicability: x.requirements.applicability.clone(),
                reason: "synthetic proposal is not owner admission".into(),
            },
        });
        assert!(x
            .requirements
            .qualified_blockers("computational_comparator")
            .contains(&format!("limit:{name}")));
    }
}
#[test]
fn missing_cold_scenario_and_empty_uncertainty_refuse_specific_boundaries() {
    let mut x = Fixture::new(DRAFT);
    x.profile
        .gate_scenario_family
        .retain(|m| m.scenario != "D4_cold_viscous_slug");
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("gate_scenario_family"));
    let mut x = Fixture::new(DRAFT);
    x.profile.model_discrepancy.clear();
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("uncertainty_inventory"));
}
#[test]
fn stale_references_unreviewed_and_revoked_sources_refuse() {
    let mut x = Fixture::new(DRAFT);
    x.profile.subject.sha256 = "0".repeat(64);
    assert!(x.valid().unwrap_err().to_string().contains("source:"));
    let mut x = Fixture::new(DRAFT);
    let r = x.profile.joint_model.clone();
    x.review(r, ReviewAction::Revoked);
    assert!(x.valid().unwrap_err().to_string().contains("source:"));
    let mut x = Fixture::new(DRAFT);
    x.registry = Registry::default();
    assert!(x.valid().unwrap_err().to_string().contains("source:"));
}
#[test]
fn changing_declared_scenario_without_changing_frozen_source_fails() {
    let text =
        std::str::from_utf8(DRAFT)
            .unwrap()
            .replacen("d50_feed_um = 22.0", "d50_feed_um = 15.0", 1);
    let x = Fixture::new(text.as_bytes());
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("scenario_source_value"));
    let text = std::str::from_utf8(DRAFT).unwrap().replacen(
        "dsd_basis = \"oil_volume\"",
        "dsd_basis = \"number\"",
        1,
    );
    assert!(RequirementSet::from_toml(text.as_bytes()).is_err());
}
#[test]
fn incorrect_units_unknown_keys_duplicates_nonfinite_and_negative_zero_reject() {
    let mut x = Fixture::new(DRAFT);
    let RequirementValue::Admitted { value, .. } = &mut x.profile.numerical_bounds[0].bound else {
        panic!()
    };
    value.unit = DecisionUnit::Pascal;
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("uncertainty_source_unit"));
    let mut value = serde_json::to_value(&x.profile).unwrap();
    value["mechanism_bonus"] = serde_json::json!(1.0);
    assert!(DecisionProfile::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(DecisionProfile::from_json(b"{\"schema_version\":1,\"schema_version\":1}").is_err());
    assert!(DecisionProfile::from_json(b"{\"attempt\":NaN}").is_err());
    assert!(bounded(DecisionUnit::Pascal, -0.0, 0.0).check().is_err());
}
#[test]
fn marginal_bounds_are_not_simultaneous_but_fixed_candidate_iut_is_distinct() {
    let mut x = Fixture::new(DRAFT);
    for a in &mut x.profile.statistical_method.allocations {
        a.alpha = f(0.05);
    }
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("simultaneous_or_adaptive_alpha_exceeded"));
    x.profile.statistical_method.criterion = ErrorCriterion::FixedCandidateConjunction;
    x.profile.statistical_method.procedure = Procedure::IntersectionUnion;
    x.policy.criterion = ErrorCriterion::FixedCandidateConjunction;
    x.policy.procedure = Procedure::IntersectionUnion;
    x.install_policy("synthetic-iut-policy");
    x.valid().unwrap();
    x.policy.selection = Selection::AdaptiveSearch;
    x.install_policy("synthetic-adaptive-iut-invalid");
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("unsupported_error_criterion"));
}
#[test]
fn planning_one_third_is_not_an_inference_or_alpha_waiver() {
    let mut x = Fixture::new(DRAFT);
    x.profile.planning_precision[0].maximum_uncertainty_fraction = f(0.5);
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("planning_precision_not_inference"));
    let mut x = Fixture::new(DRAFT);
    x.policy.max_attempts = 10;
    x.install_policy("synthetic-many-attempts");
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("alpha_exceeded"));
    for a in &mut x.profile.statistical_method.allocations {
        a.alpha = f(0.0001);
    }
    x.valid().unwrap();
    let previous = x.profile.clone();
    x.profile.attempt = 2;
    x.profile.check_continuation(&previous).unwrap();
    x.profile.attempt = 1;
    assert!(x.profile.check_continuation(&previous).is_err());
    x.profile.attempt = 2;
    x.profile.experiment_family_id = "reset-family".into();
    assert!(x.profile.check_continuation(&previous).is_err());
}
#[test]
fn unresolved_discrepancy_and_timeouts_remain_unknown_not_infeasible() {
    let mut x = Fixture::new(DRAFT);
    x.profile.model_discrepancy[0].bound = RequirementValue::Unresolved {
        reason: "no admitted model discrepancy".into(),
        owner: "validation_lead".into(),
        blocks: vec!["computational_comparator".into()],
    };
    assert!(matches!(x.valid(), Err(DecisionError::Unresolved(_))));
    let blockers = x.requirements.qualified_blockers("protected_export");
    assert!(blockers
        .iter()
        .any(|b| b.contains("license_and_disclosure")));
    assert!(!blockers.iter().any(|b| b.contains("pressure_budget")));
    let blockers = x
        .requirements
        .qualified_blockers("computational_comparator");
    assert!(!blockers
        .iter()
        .any(|b| b.contains("license_and_disclosure")));
}

#[test]
fn reviewed_formula_direction_fixed_subject_and_sidedness_cannot_drift() {
    let mut x = Fixture::new(DRAFT);
    x.profile.estimands[0].direction = Direction::Maximize;
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("estimand_definition_or_population_drift"));
    let mut x = Fixture::new(DRAFT);
    x.profile.statistical_method.sidedness = Sidedness::Lower;
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("inference_family_policy_or_reference_mismatch"));
    let mut x = Fixture::new(DRAFT);
    let other = x.artifact(
        "synthetic-other-candidate",
        &serde_json::json!({"candidate":2}),
    );
    x.profile.subject = other;
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("fixed_candidate_subject_changed"));
}

#[test]
fn finite_envelope_does_not_supply_iid_quantile_inference() {
    let mut x = Fixture::new(DRAFT);
    let assumption = x.profile.subject.clone();
    let sampling = x.artifact(
        "synthetic-finite-sampling",
        &SamplingDesign {
            kind: SamplingKind::FiniteEnvelope,
            independent_unit: "finite scenario, not random draw".into(),
            pairing: "paired finite scenarios".into(),
            correlation_treatment: assumption.clone(),
        },
    );
    let joint = x.artifact(
        "synthetic-finite-joint",
        &JointModel {
            sampling_design: sampling.clone(),
            assumptions: vec![assumption.clone()],
        },
    );
    x.profile.sampling_design = sampling.clone();
    x.profile.joint_model = joint.clone();
    x.policy.sampling_design = sampling;
    x.policy.joint_model = joint;
    x.profile.estimands[0].kind = EstimandKind::Quantile;
    let e = x.profile.estimands[0].clone();
    let definition = x.artifact(
        "synthetic-tail-definition",
        &EstimandDefinition {
            kind: e.kind,
            input_domain: e.input_domain,
            unit: e.unit,
            direction: e.direction,
            population: e.population,
            expression: assumption,
        },
    );
    x.profile.estimands[0].formula = definition;
    x.install_policy("synthetic-finite-family");
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("finite_envelope_is_not_iid_tail_distribution"));
}

#[test]
fn schema_and_strict_wire_agree_on_every_profile_field() {
    let x = Fixture::new(DRAFT);
    let schema: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../schemas/v5/decision-profile.schema.json"
    ))
    .unwrap();
    let wire = serde_json::to_value(&x.profile).unwrap();
    let keys: std::collections::BTreeSet<_> = wire.as_object().unwrap().keys().cloned().collect();
    let required: std::collections::BTreeSet<_> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(keys, required);
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        DecisionProfile::from_json(&serde_json::to_vec(&x.profile).unwrap()).unwrap(),
        x.profile
    );
    assert!(
        DecisionProfile::from_json(b"{\"model_discrepancy\":[],\"unknown_policy\":\"pass\"}")
            .is_err()
    );
}

#[test]
fn input_domain_is_bound_to_the_reviewed_definition_and_legacy_identity() {
    let mut x = Fixture::new(DRAFT);
    let legacy = serde_json::to_value(&x.profile.estimands[0]).unwrap();
    assert!(legacy.get("input_domain").is_none());
    let decoded: Estimand = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(decoded.input_domain, EstimandInputDomain::Unspecified);
    assert_eq!(serde_json::to_value(&decoded).unwrap(), legacy);
    let old_hash = joil_evidence::identity::scientific_hash(&legacy).unwrap();
    assert_eq!(
        joil_evidence::identity::scientific_hash(&decoded).unwrap(),
        old_hash
    );
    x.valid().unwrap();

    // The data/profile author cannot borrow an approved formula while changing
    // its input obligations. This is checked before any inference algorithm.
    x.profile.estimands[0].input_domain = EstimandInputDomain::StrictlyPositiveBaseline;
    assert!(x
        .valid()
        .unwrap_err()
        .to_string()
        .contains("estimand_definition_or_population_drift"));
    let e = x.profile.estimands[0].clone();
    let expression = x.profile.estimands[1].population.clone();
    let formula = x.artifact(
        "synthetic-positive-baseline-definition",
        &EstimandDefinition {
            kind: e.kind,
            input_domain: e.input_domain,
            unit: e.unit,
            direction: e.direction,
            population: e.population,
            expression,
        },
    );
    x.profile.estimands[0].formula = formula;
    x.valid().unwrap();

    let schema: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../schemas/v5/decision-profile.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["estimand"]["properties"]["input_domain"]["default"],
        "unspecified"
    );
    let mut invalid = legacy;
    invalid["input_domain"] = serde_json::json!("assume_positive");
    assert!(serde_json::from_value::<Estimand>(invalid).is_err());
}

#[test]
fn changing_proposed_tag_cannot_turn_provisional_raw_source_into_admission() {
    let mut raw: toml::Value = toml::from_str(std::str::from_utf8(DRAFT).unwrap()).unwrap();
    for limit in raw["limits"].as_array_mut().unwrap() {
        let value = limit["value"].as_table_mut().unwrap();
        if value.get("status").and_then(|v| v.as_str()) == Some("proposed") {
            value.insert("status".into(), toml::Value::String("admitted".into()));
            value.remove("reason");
        }
    }
    let bytes = toml::to_string(&raw).unwrap();
    let x = Fixture::new(bytes.as_bytes());
    x.valid().unwrap();
    let error = x
        .profile
        .qualified_admission(&x.requirements, &x.registry)
        .unwrap_err();
    assert!(matches!(error, DecisionError::Invalid(_)));
    assert!(error.to_string().contains("unknown field"));
}

#[path = "decision/policy_external.rs"]
mod policy_external;
