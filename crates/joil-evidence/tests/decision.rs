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

// ---------------------------------------------------------------------------
// V5-02 audit follow-up: the executable 1.4 mm wrong-context negative and the nine-input by
// claim-scope matrix, through `DecisionProfile::qualified_admission`. Every source below is
// SYNTHETIC and approved only inside the test registry; none is a physical limit or measurement.

const CLAIM_SCOPES: [&str; 4] = [
    "computational_comparator",
    "physical_candidate",
    "commercial_claim",
    "protected_export",
];
const COMPUTATIONAL: &[&str] = &[
    "computational_comparator",
    "physical_candidate",
    "commercial_claim",
];
/// Independently stated expectation: external input, accountable owner, claims it must block.
const EXTERNAL_EXPECTATIONS: [(&str, &str, &[&str]); 9] = [
    (
        "installation_interfaces",
        "installation_engineer",
        COMPUTATIONAL,
    ),
    ("system_pressure_and_loads", "process_owner", COMPUTATIONAL),
    ("fluid_and_dsd", "fluid_measurement_lead", COMPUTATIONAL),
    (
        "manufacturing_passport",
        "manufacturing_lead",
        COMPUTATIONAL,
    ),
    ("material_allowables", "materials_engineer", COMPUTATIONAL),
    ("independent_validation", "validation_lead", COMPUTATIONAL),
    (
        "laboratory_access",
        "experimental_lead",
        &["physical_candidate", "commercial_claim"],
    ),
    (
        "contemporary_comparator",
        "project_and_experimental_owners",
        &["commercial_claim"],
    ),
    (
        "license_and_disclosure",
        "project_owner",
        &["protected_export"],
    ),
];

/// SYNTHETIC values that only complete the policy so one change can be isolated.
fn synthetic_limit_value(id: &str) -> (f64, f64) {
    match id {
        "pressure_budget_pa" => (1.0e5, 1.0e5),
        "energy_budget_j_per_treated_m3" => (500.0, 500.0),
        "absolute_pressure_min_pa" => (2.0e5, 2.0e5),
        "continuous_duty_band_m3_h" => (2.6, 5.4),
        "external_demand_pa" => (0.0, 2.5e6),
        "quality_limit_mg_l" => (30.0, 30.0),
        "split_fraction" => (0.0198, 0.0202),
        "printed_min_passage_mm" => (3.0, 3.0),
        "machined_overflow_nominal_mm" => (1.4, 1.4),
        other => panic!("unexpected draft limit {other}; extend the synthetic table"),
    }
}

type Pending = Vec<(ContentRef, Vec<u8>)>;

fn pending_artifact<T: Serialize>(pending: &mut Pending, id: &str, value: &T) -> ContentRef {
    let bytes = serde_json::to_vec(value).unwrap();
    let reference = ContentRef::from_bytes(id, "1", &bytes);
    pending.push((reference.clone(), bytes));
    reference
}

fn toml_of<T: Serialize>(value: &T) -> toml::Value {
    toml::Value::try_from(value).unwrap()
}

fn entry_mut<'a>(raw: &'a mut toml::Value, list: &str, id: &str) -> &'a mut toml::Value {
    raw[list]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e["id"].as_str() == Some(id))
        .unwrap()
}

/// The draft with admission for evidence review and every limit, conflict and external input
/// bound to an approved SYNTHETIC source; `edit` makes the single change under test.
fn complete_bound(edit: impl FnOnce(&mut toml::Value, &mut Pending, &ContentRef)) -> Fixture {
    let mut raw: toml::Value = toml::from_str(std::str::from_utf8(DRAFT).unwrap()).unwrap();
    let scope: ContentRef = raw["applicability"].clone().try_into().unwrap();
    let mut pending = Pending::new();
    raw["admission"] = toml::Value::String("admitted_for_evidence_review".into());
    for limit in raw["limits"].as_array_mut().unwrap() {
        let id = limit["id"].as_str().unwrap().to_string();
        let unit: DecisionUnit = limit["unit"].clone().try_into().unwrap();
        let comparison: Comparison = limit["comparison"].clone().try_into().unwrap();
        let (lower, upper) = synthetic_limit_value(&id);
        let value = bounded(unit, lower, upper);
        let source = pending_artifact(
            &mut pending,
            &format!("synthetic-limit-source-{id}"),
            &LimitSource {
                limit_id: id.clone(),
                value: value.clone(),
                comparison,
                applicability: scope.clone(),
            },
        );
        limit["value"] = toml_of(&RequirementValue::Admitted {
            value,
            source,
            applicability: scope.clone(),
        });
    }
    let statement = pending_artifact(
        &mut pending,
        "synthetic-conflict-statement",
        &serde_json::json!({"origin": "synthetic", "statement": "fixture resolution, not owner authority"}),
    );
    for conflict in raw["conflicts"].as_array_mut().unwrap() {
        let id = conflict["id"].as_str().unwrap().to_string();
        let alternatives: Vec<Alternative> = conflict["alternatives"].clone().try_into().unwrap();
        let resolution = pending_artifact(
            &mut pending,
            &format!("synthetic-conflict-resolution-{id}"),
            &ConflictResolution {
                conflict_id: id,
                alternatives,
                applicability: scope.clone(),
                resolution_statement: statement.clone(),
            },
        );
        conflict
            .as_table_mut()
            .unwrap()
            .insert("resolution".into(), toml_of(&resolution));
    }
    for input in raw["external_inputs"].as_array_mut().unwrap() {
        let id = input["id"].as_str().unwrap().to_string();
        let claims: Vec<String> = input["blocks"].clone().try_into().unwrap();
        let artifact = pending_artifact(
            &mut pending,
            &format!("synthetic-external-evidence-{id}"),
            &serde_json::json!({"origin": "synthetic", "input": id}),
        );
        let source = pending_artifact(
            &mut pending,
            &format!("synthetic-external-source-{id}"),
            &ExternalSource {
                input_id: id,
                artifact: artifact.clone(),
                applicability: scope.clone(),
                claims,
            },
        );
        input["evidence"] = toml_of(&RequirementValue::Admitted {
            value: artifact,
            source,
            applicability: scope.clone(),
        });
    }
    edit(&mut raw, &mut pending, &scope);
    let bytes = toml::to_string(&raw).unwrap();
    let mut x = Fixture::new(bytes.as_bytes());
    for (reference, bytes) in pending {
        x.approve(reference, bytes);
    }
    x
}

/// Public admission path for one claim scope (profile and reviewed family policy both bound).
fn admit(x: &mut Fixture, claim: &str) -> Result<(), DecisionError> {
    x.profile.claim_scope = claim.into();
    x.policy.claim_scope = claim.into();
    x.install_policy(&format!("synthetic-family-policy-{claim}"));
    x.profile.qualified_admission(&x.requirements, &x.registry)
}

fn unresolved(reason: &str, owner: &str, blocks: &toml::Value) -> toml::Value {
    let blocks: Vec<String> = blocks.clone().try_into().unwrap();
    toml_of(&RequirementValue::<ContentRef>::Unresolved {
        reason: reason.into(),
        owner: owner.into(),
        blocks,
    })
}

#[test]
fn complete_bound_synthetic_policy_admits_every_claim_scope_for_evidence_review() {
    let mut x = complete_bound(|_, _, _| {});
    x.valid().unwrap();
    for claim in CLAIM_SCOPES {
        assert_eq!(admit(&mut x, claim), Ok(()), "{claim}");
    }
}

#[test]
fn borrowed_machined_overflow_source_cannot_admit_a_printed_passage_limit() {
    // Exact borrow: the machined-overflow source record that admits 1.4 mm "within" is reused
    // as the printed-passage source; the refusal names the printed limit.
    let mut x = complete_bound(|raw, _, scope| {
        let machined: ContentRef = entry_mut(raw, "limits", "machined_overflow_nominal_mm")
            ["value"]["source"]
            .clone()
            .try_into()
            .unwrap();
        entry_mut(raw, "limits", "printed_min_passage_mm")["value"] =
            toml_of(&RequirementValue::Admitted {
                value: bounded(DecisionUnit::Millimetre, 1.4, 1.4),
                source: machined,
                applicability: scope.clone(),
            });
    });
    x.valid().unwrap();
    assert_eq!(
        admit(&mut x, "computational_comparator"),
        Err(DecisionError::Invalid(
            "limit_source_drift:printed_min_passage_mm".into()
        ))
    );
    // The draft's machined source (the baseline genome, do_over_d2 x D2 = 1.4 mm) is not a
    // printed-passage limit record either.
    let mut x = complete_bound(|raw, _, scope| {
        let genome: ContentRef = raw["baseline"].clone().try_into().unwrap();
        entry_mut(raw, "limits", "printed_min_passage_mm")["value"] =
            toml_of(&RequirementValue::Admitted {
                value: bounded(DecisionUnit::Millimetre, 1.4, 1.4),
                source: genome,
                applicability: scope.clone(),
            });
    });
    let err = admit(&mut x, "computational_comparator").unwrap_err();
    assert!(
        matches!(&err, DecisionError::Invalid(m) if m.contains("unknown field")),
        "{err}"
    );
}

#[test]
fn proper_printed_context_admits_without_a_numeric_ban_and_proposed_3mm_stays_blocked() {
    // 1.4 mm from a printed-process source bound to the printed limit is not banned by value.
    let mut x = complete_bound(|raw, pending, scope| {
        let value = bounded(DecisionUnit::Millimetre, 1.4, 1.4);
        let source = pending_artifact(
            pending,
            "synthetic-printed-process-passage-source",
            &LimitSource {
                limit_id: "printed_min_passage_mm".into(),
                value: value.clone(),
                comparison: Comparison::AtLeast,
                applicability: scope.clone(),
            },
        );
        entry_mut(raw, "limits", "printed_min_passage_mm")["value"] =
            toml_of(&RequirementValue::Admitted {
                value,
                source,
                applicability: scope.clone(),
            });
    });
    assert_eq!(admit(&mut x, "computational_comparator"), Ok(()));
    // The draft's proposed 3 mm is never silently admitted.
    let mut x = complete_bound(|raw, _, _| {
        let mut draft: toml::Value = toml::from_str(std::str::from_utf8(DRAFT).unwrap()).unwrap();
        let proposed = entry_mut(&mut draft, "limits", "printed_min_passage_mm")["value"].clone();
        assert_eq!(proposed["status"].as_str(), Some("proposed"));
        entry_mut(raw, "limits", "printed_min_passage_mm")["value"] = proposed;
    });
    for claim in COMPUTATIONAL {
        assert_eq!(
            admit(&mut x, claim),
            Err(DecisionError::Unresolved(vec![
                "limit:printed_min_passage_mm".into()
            ])),
            "{claim}"
        );
    }
    assert_eq!(admit(&mut x, "protected_export"), Ok(()));
}

/// Formerly the G-004 ignored contract-blocker control (red log 2ef2a8f2…12c6 at 794d044): a
/// record restating the machined 1.4 mm with the printed comparison was admitted. With
/// `LimitSource.limit_id` the machined identity is refused even though value, comparison and
/// applicability all match the printed limit, in both directions.
#[test]
fn restated_machined_limit_identity_is_refused_for_the_printed_limit() {
    let mut x = complete_bound(|raw, pending, scope| {
        let value = bounded(DecisionUnit::Millimetre, 1.4, 1.4);
        let source = pending_artifact(
            pending,
            "synthetic-machined-overflow-1.4mm-restated-at-least",
            &LimitSource {
                limit_id: "machined_overflow_nominal_mm".into(),
                value: value.clone(),
                comparison: Comparison::AtLeast,
                applicability: scope.clone(),
            },
        );
        entry_mut(raw, "limits", "printed_min_passage_mm")["value"] =
            toml_of(&RequirementValue::Admitted {
                value,
                source,
                applicability: scope.clone(),
            });
    });
    x.valid().unwrap();
    assert_eq!(
        admit(&mut x, "computational_comparator"),
        Err(DecisionError::Invalid(
            "limit_source_drift:printed_min_passage_mm".into()
        ))
    );
    // Reverse borrow: a printed-passage record restated as the machined "within" value.
    let mut x = complete_bound(|raw, pending, scope| {
        let value = bounded(DecisionUnit::Millimetre, 1.4, 1.4);
        let source = pending_artifact(
            pending,
            "synthetic-printed-passage-1.4mm-restated-within",
            &LimitSource {
                limit_id: "printed_min_passage_mm".into(),
                value: value.clone(),
                comparison: Comparison::Within,
                applicability: scope.clone(),
            },
        );
        entry_mut(raw, "limits", "machined_overflow_nominal_mm")["value"] =
            toml_of(&RequirementValue::Admitted {
                value,
                source,
                applicability: scope.clone(),
            });
    });
    assert_eq!(
        admit(&mut x, "computational_comparator"),
        Err(DecisionError::Invalid(
            "limit_source_drift:machined_overflow_nominal_mm".into()
        ))
    );
}

/// Old generic limit records (a `SourceValue` without `limit_id`) are not migrated automatically.
#[test]
fn generic_limit_record_without_limit_id_is_refused_not_migrated() {
    for (limit, unit, lower, upper, comparison) in [
        (
            "printed_min_passage_mm",
            DecisionUnit::Millimetre,
            1.4,
            1.4,
            Comparison::AtLeast,
        ),
        (
            "pressure_budget_pa",
            DecisionUnit::Pascal,
            1.0e5,
            1.0e5,
            Comparison::AtMost,
        ),
    ] {
        let mut x = complete_bound(|raw, pending, scope| {
            let value = bounded(unit, lower, upper);
            let source = pending_artifact(
                pending,
                &format!("synthetic-generic-limit-record-{limit}"),
                &SourceValue {
                    value: value.clone(),
                    comparison,
                    applicability: scope.clone(),
                },
            );
            entry_mut(raw, "limits", limit)["value"] = toml_of(&RequirementValue::Admitted {
                value,
                source,
                applicability: scope.clone(),
            });
        });
        x.valid().unwrap();
        let err = admit(&mut x, "computational_comparator").unwrap_err();
        assert!(
            matches!(&err, DecisionError::Invalid(m) if m.contains("missing field `limit_id`")),
            "{limit}: {err}"
        );
    }
}

/// Integrator counterexample (Codex 01:05Z, red log 40b33dff…5056 at 63c93dd): a reviewed
/// `LimitSource` whose lower bound is -0.0 equalled a valid +0.0 requirement under float equality.
/// The source bound is now checked before comparison; a genuine +0.0 source still admits.
#[test]
fn zero_limit_source_admits_and_negative_zero_source_is_refused_before_equality() {
    let with_source_lower = |lower: f64| {
        complete_bound(move |raw, pending, scope| {
            let source = pending_artifact(
                pending,
                &format!("synthetic-external-demand-source-lower-{lower:?}"),
                &LimitSource {
                    limit_id: "external_demand_pa".into(),
                    value: bounded(DecisionUnit::Pascal, lower, 2.5e6),
                    comparison: Comparison::Within,
                    applicability: scope.clone(),
                },
            );
            entry_mut(raw, "limits", "external_demand_pa")["value"] =
                toml_of(&RequirementValue::Admitted {
                    value: bounded(DecisionUnit::Pascal, 0.0, 2.5e6),
                    source,
                    applicability: scope.clone(),
                });
        })
    };
    let mut x = with_source_lower(0.0);
    x.valid().unwrap();
    for claim in CLAIM_SCOPES {
        assert_eq!(admit(&mut x, claim), Ok(()), "{claim}");
    }
    let mut x = with_source_lower(-0.0);
    x.valid().unwrap();
    for claim in COMPUTATIONAL {
        assert_eq!(
            admit(&mut x, claim),
            Err(DecisionError::Invalid(
                "reversed_bound_or_negative_zero".into()
            )),
            "{claim}"
        );
    }
}

#[test]
fn each_missing_external_input_blocks_exactly_its_claims_with_its_owner() {
    let probe = complete_bound(|_, _, _| {});
    let ids: std::collections::BTreeSet<_> = probe
        .requirements
        .external_inputs
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    let expected: std::collections::BTreeSet<_> =
        EXTERNAL_EXPECTATIONS.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(ids, expected);
    for (input, owner, blocked) in EXTERNAL_EXPECTATIONS {
        let mut x = complete_bound(|raw, _, _| {
            let entry = entry_mut(raw, "external_inputs", input);
            let missing = unresolved(
                "SYNTHETIC missing input: owner data not supplied",
                owner,
                &entry["blocks"],
            );
            entry["evidence"] = missing;
        });
        x.valid().unwrap();
        for claim in CLAIM_SCOPES {
            let expected = if blocked.contains(&claim) {
                Err(DecisionError::Unresolved(vec![format!(
                    "external:{input}:{owner}"
                )]))
            } else {
                Ok(())
            };
            assert_eq!(admit(&mut x, claim), expected, "{input} x {claim}");
        }
    }
    // All nine missing together: each claim lists exactly the inputs the matrix assigns to it.
    let mut x = complete_bound(|raw, _, _| {
        for (input, owner, _) in EXTERNAL_EXPECTATIONS {
            let entry = entry_mut(raw, "external_inputs", input);
            let missing = unresolved("SYNTHETIC missing input", owner, &entry["blocks"]);
            entry["evidence"] = missing;
        }
    });
    for claim in CLAIM_SCOPES {
        let mut expected: Vec<String> = EXTERNAL_EXPECTATIONS
            .iter()
            .filter(|(_, _, blocked)| blocked.contains(&claim))
            .map(|(input, owner, _)| format!("external:{input}:{owner}"))
            .collect();
        expected.sort();
        assert_eq!(
            admit(&mut x, claim),
            Err(DecisionError::Unresolved(expected)),
            "{claim}"
        );
    }
}

#[test]
fn waiting_data_timeout_stays_unresolved_with_reason_not_infeasible_or_zero_uncertainty() {
    let reason =
        "SYNTHETIC waiting-data timeout: laboratory access data not received before review";
    let mut x = complete_bound(|raw, _, _| {
        let entry = entry_mut(raw, "external_inputs", "laboratory_access");
        let timeout = unresolved(reason, "experimental_lead", &entry["blocks"]);
        entry["evidence"] = timeout;
    });
    let bounds_before = (
        x.profile.numerical_bounds.clone(),
        x.profile.model_discrepancy.clone(),
    );
    // Semantically valid policy: the timeout is not an infeasible or invalid design.
    x.valid().unwrap();
    for claim in ["physical_candidate", "commercial_claim"] {
        assert_eq!(
            admit(&mut x, claim),
            Err(DecisionError::Unresolved(vec![
                "external:laboratory_access:experimental_lead".into()
            ])),
            "{claim}"
        );
    }
    for claim in ["computational_comparator", "protected_export"] {
        assert_eq!(admit(&mut x, claim), Ok(()), "{claim}");
    }
    // The reviewed requirement record keeps the timeout and its reason; nothing became admitted
    // and no uncertainty bound was created or narrowed.
    let lab = x
        .requirements
        .external_inputs
        .iter()
        .find(|e| e.id == "laboratory_access")
        .unwrap();
    assert!(matches!(
        &lab.evidence,
        RequirementValue::Unresolved { reason: r, owner, blocks }
            if r == reason && owner == "experimental_lead"
                && blocks == &vec!["physical_candidate".to_string(), "commercial_claim".to_string()]
    ));
    assert!(!lab.evidence.is_admitted());
    assert_eq!(
        (
            x.profile.numerical_bounds.clone(),
            x.profile.model_discrepancy.clone()
        ),
        bounds_before
    );
    // A timeout cannot be recorded as affecting no claims.
    let mut raw: toml::Value = toml::from_str(std::str::from_utf8(DRAFT).unwrap()).unwrap();
    entry_mut(&mut raw, "external_inputs", "laboratory_access")["evidence"]["blocks"] =
        toml::Value::Array(vec![]);
    assert_eq!(
        RequirementSet::from_toml(toml::to_string(&raw).unwrap().as_bytes()),
        Err(DecisionError::Invalid(
            "unresolved_input_needs_affected_claims".into()
        ))
    );
}
