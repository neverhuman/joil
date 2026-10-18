//! Shared V5-04 test support: fixture loading and oracles computed by a different
//! method from the code under test. Synthetic values only.
#![allow(dead_code)]
use statrs::distribution::{ChiSquared, Continuous, ContinuousCDF, Normal};
use std::path::PathBuf;

pub fn fixture(name: &str) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/v5-statistics")
        .join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_slice(&bytes).unwrap()
}

pub fn f64s(value: &serde_json::Value) -> Vec<f64> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

pub fn strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

/// Exact power of the one-sided one-sample t test, P(T'(n-1, sqrt(n)*standardized) > c),
/// by integrating the conditional normal tail over the chi-square scale variable
/// (composite Simpson). Independent of the central-t planning iteration.
pub fn noncentral_t_power(n: usize, critical: f64, standardized_effect: f64) -> f64 {
    let df = (n - 1) as f64;
    let lambda = (n as f64).sqrt() * standardized_effect;
    let chi = ChiSquared::new(df).unwrap();
    let normal = Normal::new(0.0, 1.0).unwrap();
    let upper = df + 60.0 * (2.0 * df).sqrt() + 60.0;
    let steps = 40_000usize;
    let h = upper / steps as f64;
    let integrand = |v: f64| {
        if v <= 0.0 {
            return 0.0;
        }
        (1.0 - normal.cdf(critical * (v / df).sqrt() - lambda)) * chi.pdf(v)
    };
    let mut sum = integrand(0.0) + integrand(upper);
    for i in 1..steps {
        let w = if i % 2 == 1 { 4.0 } else { 2.0 };
        sum += w * integrand(i as f64 * h);
    }
    sum * h / 3.0
}

/// Preregistered synthetic generator: SplitMix64 with 53-bit uniforms and Box-Muller normals.
pub struct SplitMix64(pub u64);
impl SplitMix64 {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    pub fn normal(&mut self) -> f64 {
        let u1 = 1.0 - self.uniform(); // (0, 1]
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

pub fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Binomial Monte Carlo standard error of an observed rate.
pub fn monte_carlo_se(rate: f64, trials: usize) -> f64 {
    (rate * (1.0 - rate) / trials as f64).sqrt()
}

/// SYNTHETIC reviewed decision inputs for the four-margin S02 family. Every source is
/// an approved registry artifact; the profile is validated by joil-evidence itself.
pub mod joint {
    use joil_evidence::claims::{Registry, ReviewAction, ReviewEvent};
    use joil_evidence::decision::*;
    use joil_evidence::identity::ContentRef;
    use joil_evidence::observation::Finite;
    use serde::Serialize;

    pub fn f(x: f64) -> Finite {
        Finite::new(x).unwrap()
    }

    fn reference<T: Serialize>(id: &str, value: &T) -> (ContentRef, Vec<u8>) {
        let bytes = serde_json::to_vec(value).unwrap();
        (ContentRef::from_bytes(id, "1", &bytes), bytes)
    }

    fn inline(r: &ContentRef) -> String {
        format!(
            "{{ id = \"{}\", revision = \"{}\", sha256 = \"{}\" }}",
            r.id, r.revision, r.sha256
        )
    }

    pub fn unit_of(name: &str) -> DecisionUnit {
        serde_json::from_value(serde_json::Value::String(name.into())).unwrap()
    }

    pub struct JointFixture {
        pub registry: Registry,
        pub requirements: RequirementSet,
        pub reference: ContentRef,
        pub profile: DecisionProfile,
        pub policy: FamilyPolicy,
        sequence: u64,
    }

    impl JointFixture {
        pub fn review(&mut self, subject: ContentRef, action: ReviewAction) {
            self.sequence += 1;
            let event = ReviewEvent {
                sequence: self.sequence,
                subject,
                action,
                reviewer: "synthetic-statistics-reviewer".into(),
                requirements: vec![self.reference.clone()],
            };
            let bytes = serde_json::to_vec(&event).unwrap();
            self.registry
                .append_approved_review(
                    "synthetic-statistics-reviewer",
                    std::slice::from_ref(&self.reference),
                    ContentRef::from_bytes(
                        &format!("synthetic-stats-review-{}", self.sequence),
                        "1",
                        &bytes,
                    ),
                    bytes,
                )
                .unwrap();
        }

        fn approve(&mut self, r: ContentRef, bytes: Vec<u8>) {
            self.registry.insert_artifact(r.clone(), bytes).unwrap();
            self.review(r, ReviewAction::Approved);
        }

        pub fn artifact<T: Serialize>(&mut self, id: &str, value: &T) -> ContentRef {
            let (r, bytes) = reference(id, value);
            self.approve(r.clone(), bytes);
            r
        }

        pub fn install_policy(&mut self, id: &str) {
            self.profile.statistical_method.family_policy = self.artifact(id, &self.policy.clone());
        }

        pub fn new(fx: &serde_json::Value, limit_admitted: bool) -> Self {
            let req = &fx["requirements"];
            let scenario = &req["scenario"];
            let scope = req["claim_scope"].as_str().unwrap().to_string();
            let (baseline, baseline_bytes) = reference(
                "synthetic-s02-baseline",
                &serde_json::json!({"origin": "synthetic", "role": "baseline subject for the S02 arithmetic fixture"}),
            );
            let (prereg, prereg_bytes) = reference(
                "synthetic-s02-preregistration",
                &serde_json::json!({"origin": "synthetic", "duties": [{
                    "duty_id": scenario["id"], "q_in_m3_h": scenario["q_in_m3_h"], "cin_mg_l": scenario["cin_mg_l"],
                    "d50_feed_um": scenario["d50_feed_um"], "viscosity_factor": scenario["viscosity_factor"]}]}),
            );
            let (applicability, applicability_bytes) = reference(
                "synthetic-s02-applicability",
                &serde_json::json!({"origin": "synthetic", "reference": {"dsd_basis": "oil_volume"}}),
            );
            let limit = &req["limit"];
            let limit_value = BoundedValue {
                lower: f(limit["value_mg_l"].as_f64().unwrap()),
                upper: f(limit["value_mg_l"].as_f64().unwrap()),
                unit: DecisionUnit::MgPerLitreDispersedOil,
            };
            let (limit_source, limit_bytes) = reference(
                "synthetic-s02-limit-source",
                &LimitSource {
                    limit_id: limit["id"].as_str().unwrap().into(),
                    value: limit_value.clone(),
                    comparison: Comparison::AtMost,
                    applicability: applicability.clone(),
                },
            );
            let mut toml = format!(
                "schema_version = 2\nid = \"{}\"\nrevision = \"{}\"\nadmission = \"admitted_for_evidence_review\"\nconcentration_basis = \"dispersed_oil_mg_per_litre\"\nclaim_scopes = [\"{scope}\"]\nconflicts = []\nexternal_inputs = []\nbaseline = {}\npreregistration = {}\napplicability = {}\n\n[[scenarios]]\nid = \"{}\"\nq_in_m3_h = {:?}\ncin_mg_l = {:?}\nd50_feed_um = {:?}\nviscosity_factor = {:?}\ndsd_basis = \"oil_volume\"\nconcentration_basis = \"dispersed_oil_mg_per_litre\"\nsource = {}\nsource_pointer = \"/duties/0\"\nbasis_source = {}\n",
                req["id"].as_str().unwrap(),
                req["revision"].as_str().unwrap(),
                inline(&baseline),
                inline(&prereg),
                inline(&applicability),
                scenario["id"].as_str().unwrap(),
                scenario["q_in_m3_h"].as_f64().unwrap(),
                scenario["cin_mg_l"].as_f64().unwrap(),
                scenario["d50_feed_um"].as_f64().unwrap(),
                scenario["viscosity_factor"].as_f64().unwrap(),
                inline(&prereg),
                inline(&applicability),
            );
            for m in fx["margins"].as_array().unwrap() {
                toml += &format!(
                    "\n[[mandatory_gates]]\nid = \"{}\"\nestimand = \"{}\"\nunit = \"{}\"\n",
                    m["gate"].as_str().unwrap(),
                    m["estimand"].as_str().unwrap(),
                    m["unit"].as_str().unwrap()
                );
            }
            toml += &format!(
                "\n[[limits]]\nid = \"{}\"\ncomparison = \"at_most\"\nunit = \"mg_per_litre_dispersed_oil\"\nmandatory_for = [\"{scope}\"]\n[limits.value]\n",
                limit["id"].as_str().unwrap()
            );
            if limit_admitted {
                toml += &format!(
                    "status = \"admitted\"\nvalue = {{ lower = {v:?}, upper = {v:?}, unit = \"mg_per_litre_dispersed_oil\" }}\nsource = {}\napplicability = {}\n",
                    inline(&limit_source),
                    inline(&applicability),
                    v = limit_value.lower.get()
                );
            } else {
                toml += &format!(
                    "status = \"unresolved\"\nreason = \"synthetic unresolved limit\"\nowner = \"synthetic-owner\"\nblocks = [\"{scope}\"]\n"
                );
            }
            let bytes = toml.into_bytes();
            let requirements = RequirementSet::from_toml(&bytes).unwrap();
            let reference_ =
                ContentRef::from_bytes(&requirements.id, &requirements.revision, &bytes);
            let empty = ContentRef::from_bytes("synthetic-placeholder", "1", b"{}");
            let mut x = Self {
                registry: Registry::default(),
                requirements,
                reference: reference_.clone(),
                profile: DecisionProfile {
                    schema_version: 1,
                    experiment_family_id: "synthetic-s02-family".into(),
                    attempt: 1,
                    requirements: reference_.clone(),
                    subject: empty.clone(),
                    baseline: baseline.clone(),
                    applicability: applicability.clone(),
                    claim_scope: scope.clone(),
                    estimands: vec![],
                    gate_scenario_family: vec![],
                    sampling_design: empty.clone(),
                    joint_model: empty.clone(),
                    numerical_bounds: vec![],
                    model_discrepancy: vec![],
                    statistical_method: StatisticalMethod {
                        criterion: ErrorCriterion::SimultaneousCoverage,
                        procedure: Procedure::Bonferroni,
                        sidedness: Sidedness::Lower,
                        allocations: vec![],
                        family_policy: empty.clone(),
                    },
                    selection_policy: empty.clone(),
                    stopping_policy: empty.clone(),
                    holdout_policy: empty.clone(),
                    operating_population: empty.clone(),
                    unknown_policy: UnknownPolicy::Abstain,
                    planning_precision: vec![],
                    executable_acceptance_fixtures: vec![],
                },
                policy: FamilyPolicy {
                    experiment_family_id: "synthetic-s02-family".into(),
                    requirements: reference_.clone(),
                    family_alpha: f(fx["family_alpha"].as_f64().unwrap()),
                    max_attempts: 1,
                    criterion: ErrorCriterion::SimultaneousCoverage,
                    procedure: Procedure::Bonferroni,
                    sidedness: Sidedness::Lower,
                    selection: Selection::FixedCandidate,
                    fixed_subject: None,
                    claim_scope: scope,
                    sampling_design: empty.clone(),
                    joint_model: empty.clone(),
                    selection_policy: empty.clone(),
                    stopping_policy: empty.clone(),
                    holdout_policy: empty.clone(),
                    operating_population: empty,
                },
                sequence: 0,
            };
            x.approve(reference_, bytes);
            for (r, b) in [
                (baseline, baseline_bytes),
                (prereg, prereg_bytes),
                (applicability.clone(), applicability_bytes),
                (limit_source, limit_bytes),
            ] {
                x.approve(r, b);
            }
            let synthetic = x.artifact(
                "synthetic-s02-statement",
                &serde_json::json!({"origin": "synthetic", "purpose": "arithmetic and decision behaviour only"}),
            );
            let assumption = x.artifact(
                "synthetic-s02-normal-margins",
                &serde_json::json!({"assumption": "four marginal block-margin populations normal with unknown variance; within-block correlation allowed; 20 independent paired blocks"}),
            );
            let subject = x.artifact(
                "synthetic-s02-candidate",
                &serde_json::json!({"origin": "synthetic", "role": "fixed candidate"}),
            );
            let sampling = x.artifact(
                "synthetic-s02-sampling",
                &SamplingDesign {
                    kind: SamplingKind::IndependentBlocks,
                    independent_unit: "independent paired block".into(),
                    pairing: "baseline and candidate measured in each block".into(),
                    correlation_treatment: synthetic.clone(),
                },
            );
            let joint = x.artifact(
                "synthetic-s02-joint-model",
                &JointModel {
                    sampling_design: sampling.clone(),
                    assumptions: vec![assumption],
                },
            );
            x.profile.subject = subject.clone();
            x.policy.fixed_subject = Some(subject);
            for r in [
                &mut x.profile.sampling_design,
                &mut x.policy.sampling_design,
            ] {
                *r = sampling.clone();
            }
            for r in [&mut x.profile.joint_model, &mut x.policy.joint_model] {
                *r = joint.clone();
            }
            for r in [
                &mut x.profile.selection_policy,
                &mut x.profile.stopping_policy,
                &mut x.profile.holdout_policy,
                &mut x.profile.operating_population,
                &mut x.policy.selection_policy,
                &mut x.policy.stopping_policy,
                &mut x.policy.holdout_policy,
                &mut x.policy.operating_population,
            ] {
                *r = synthetic.clone();
            }
            x.profile.executable_acceptance_fixtures = vec![synthetic.clone()];
            let scenario_id = scenario["id"].as_str().unwrap().to_string();
            let margins = fx["margins"].as_array().unwrap().clone();
            let alpha = fx["family_alpha"].as_f64().unwrap() / margins.len() as f64;
            for (i, m) in margins.iter().enumerate() {
                let unit = unit_of(m["unit"].as_str().unwrap());
                let estimand = m["estimand"].as_str().unwrap().to_string();
                let scale = if m["ratio_margin"].as_bool().unwrap() {
                    fx["litre_per_second_to_cubic_metre_per_hour"]
                        .as_f64()
                        .unwrap()
                } else {
                    1.0
                };
                let half = 0.5 * m["adverse_bound"].as_f64().unwrap() * scale;
                let definition = x.artifact(
                    &format!("synthetic-s02-estimand-{i}"),
                    &EstimandDefinition {
                        kind: EstimandKind::Mean,
                        unit,
                        direction: Direction::Maximize,
                        population: synthetic.clone(),
                        expression: synthetic.clone(),
                    },
                );
                x.profile.estimands.push(Estimand {
                    id: estimand.clone(),
                    unit,
                    direction: Direction::Maximize,
                    kind: EstimandKind::Mean,
                    population: synthetic.clone(),
                    formula: definition,
                });
                for (inventory, label) in [("numerical", "numerical"), ("model", "model")] {
                    let range = BoundedValue {
                        lower: f(0.0),
                        upper: f(half),
                        unit,
                    };
                    let source = x.artifact(
                        &format!("synthetic-s02-{label}-bound-{i}"),
                        &SourceValue {
                            value: range.clone(),
                            comparison: Comparison::AtMost,
                            applicability: applicability.clone(),
                        },
                    );
                    let bound = UncertaintyBound {
                        estimand: estimand.clone(),
                        bound: RequirementValue::Admitted {
                            value: range,
                            source,
                            applicability: applicability.clone(),
                        },
                    };
                    if inventory == "numerical" {
                        x.profile.numerical_bounds.push(bound);
                    } else {
                        x.profile.model_discrepancy.push(bound);
                    }
                }
                x.profile.planning_precision.push(PlanningPrecision {
                    estimand: estimand.clone(),
                    useful_effect: BoundedValue {
                        lower: f(1.0),
                        upper: f(1.0),
                        unit,
                    },
                    maximum_uncertainty_fraction: f(1.0 / 3.0),
                });
                let member = FamilyMember {
                    gate: m["gate"].as_str().unwrap().into(),
                    scenario: scenario_id.clone(),
                    estimand,
                };
                x.profile
                    .statistical_method
                    .allocations
                    .push(AlphaAllocation {
                        member: member.clone(),
                        alpha: f(alpha),
                    });
                x.profile.gate_scenario_family.push(member);
            }
            x.install_policy("synthetic-s02-family-policy");
            x
        }
    }
}

/// Closed-form Cauchy (t with one degree of freedom) quantile.
pub fn cauchy_quantile(p: f64) -> f64 {
    (std::f64::consts::PI * (p - 0.5)).tan()
}
