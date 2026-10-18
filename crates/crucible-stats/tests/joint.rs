//! V5-04-03: joint decision bounds on the SYNTHETIC four-margin S02 family.
mod common;
use common::fixture;
use common::joint::{f, unit_of, JointFixture};
use crucible_stats::analysis::joint::*;
use joil_evidence::claims::ReviewAction;
use joil_evidence::decision::*;

fn scale(fx: &serde_json::Value, m: &serde_json::Value) -> f64 {
    if m["ratio_margin"].as_bool().unwrap() {
        fx["litre_per_second_to_cubic_metre_per_hour"]
            .as_f64()
            .unwrap()
    } else {
        1.0
    }
}

fn summaries(fx: &serde_json::Value, x: &JointFixture) -> Vec<MarginSummary> {
    let n = fx["n_paired_blocks"].as_u64().unwrap() as usize;
    fx["margins"]
        .as_array()
        .unwrap()
        .iter()
        .zip(&x.profile.gate_scenario_family)
        .map(|(m, member)| {
            let s = scale(fx, m);
            MarginSummary {
                member: member.clone(),
                unit: unit_of(m["unit"].as_str().unwrap()),
                n,
                mean: m["mean"].as_f64().unwrap() * s,
                sd: m["sd"].as_f64().unwrap() * s,
                ratio_baseline_mean: m["baseline_mean_l_s"].as_f64().map(|b| b * s),
            }
        })
        .collect()
}

fn decide(x: &JointFixture, data: Vec<MarginSummary>) -> Result<JointDecision, JointError> {
    joint_lower_bounds(
        &x.profile,
        &x.requirements,
        &x.registry,
        &MarginData::Summaries(data),
    )
}

#[test]
fn reviewed_ratio_margin_cannot_omit_its_baseline() {
    let fx = fixture("joint.json");
    let x = JointFixture::new(&fx, true);
    let mut data = summaries(&fx, &x);
    let ratio = data
        .iter_mut()
        .find(|r| r.ratio_baseline_mean.is_some())
        .unwrap();
    ratio.ratio_baseline_mean = None;
    assert_eq!(
        decide(&x, data),
        Err(JointError::Input(
            "declared ratio margin requires baseline data"
        ))
    );
}

#[test]
fn reviewed_ratio_domain_cannot_be_relabelled_to_an_absolute_margin() {
    let fx = fixture("joint.json");
    let mut x = JointFixture::new(&fx, true);
    let data = summaries(&fx, &x);
    let ratio = x
        .profile
        .estimands
        .iter_mut()
        .find(|e| e.input_domain == EstimandInputDomain::StrictlyPositiveBaseline)
        .unwrap();
    ratio.input_domain = EstimandInputDomain::RealValued;
    assert!(
        matches!(decide(&x, data), Err(JointError::Profile(m)) if m == "estimand_definition_or_population_drift")
    );
}

#[test]
fn unspecified_legacy_domain_and_missing_paired_ratio_baseline_never_pass() {
    let fx = fixture("joint.json");
    let mut x = JointFixture::new(&fx, true);
    let data = summaries(&fx, &x);
    let e = x.profile.estimands[0].clone();
    let formula = x.artifact(
        "synthetic-legacy-unspecified-domain",
        &EstimandDefinition {
            kind: e.kind,
            input_domain: EstimandInputDomain::Unspecified,
            unit: e.unit,
            direction: e.direction,
            population: e.population.clone(),
            expression: e.population,
        },
    );
    x.profile.estimands[0].input_domain = EstimandInputDomain::Unspecified;
    x.profile.estimands[0].formula = formula;
    assert_eq!(
        decide(&x, data),
        Err(JointError::Unsupported(
            "estimand input domain is unspecified"
        ))
    );

    let x = JointFixture::new(&fx, true);
    let margins = summaries(&fx, &x)
        .into_iter()
        .map(|s| PairedMargin {
            member: s.member,
            unit: s.unit,
            values: (0..20)
                .map(|i| s.mean + if i % 2 == 0 { s.sd } else { -s.sd })
                .collect(),
            ratio_baseline: None,
        })
        .collect();
    let result = joint_lower_bounds(
        &x.profile,
        &x.requirements,
        &x.registry,
        &MarginData::PairedBlocks {
            block_ids: (0..20)
                .map(|i| format!("synthetic-domain-block-{i}"))
                .collect(),
            margins,
        },
    );
    assert_eq!(
        result,
        Err(JointError::Input(
            "declared ratio margin requires baseline data"
        ))
    );
}

#[test]
fn frozen_four_margin_family_reproduces_the_simultaneous_lower_bounds() {
    let fx = fixture("joint.json");
    let x = JointFixture::new(&fx, true);
    x.profile
        .qualified_admission(&x.requirements, &x.registry)
        .unwrap();
    let decision = decide(&x, summaries(&fx, &x)).unwrap();
    assert_eq!(decision.scope, Some(JointScope::SimultaneousLowerBounds));
    assert_eq!(decision.outcome, JointOutcome::Pass);
    let t = fx["t_09875_df19"].as_f64().unwrap();
    for (bound, m) in decision
        .bounds
        .iter()
        .zip(fx["margins"].as_array().unwrap())
    {
        assert!((bound.alpha - 0.0125).abs() < 1e-15);
        assert!((bound.t_critical - t).abs() < 1e-9);
        let s = scale(&fx, m);
        // Reported back in the table's original unit (L/s for the capacity margin).
        let lower = bound.lower_bound / s;
        let expected = m["expected_lower"].as_f64().unwrap();
        assert!(
            (lower - expected).abs() <= m["tolerance"].as_f64().unwrap(),
            "{}: {lower}",
            m["estimand"]
        );
        assert!(
            (bound.adverse_systematic_bound / s - m["adverse_bound"].as_f64().unwrap()).abs()
                < 1e-12
        );
    }
    assert!(
        decision.interpretation.contains("simultaneously")
            && decision.interpretation.contains("not reliability")
    );
}

#[test]
fn one_failed_margin_blocks_the_whole_family() {
    let fx = fixture("joint.json");
    let x = JointFixture::new(&fx, true);
    let mut data = summaries(&fx, &x);
    let variant = &fx["failed_margin_variant"];
    let row = data
        .iter_mut()
        .find(|r| r.member.estimand == variant["estimand"].as_str().unwrap())
        .unwrap();
    row.mean = variant["mean"].as_f64().unwrap();
    let decision = decide(&x, data).unwrap();
    match decision.outcome {
        JointOutcome::Fail { failed } => {
            assert_eq!(failed.len(), 1);
            assert_eq!(failed[0].estimand, "margin_cout_condition_b");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn missing_or_unreviewed_bias_and_unresolved_requirements_never_pass() {
    let fx = fixture("joint.json");
    // Missing (unresolved) adverse bound abstains.
    let mut x = JointFixture::new(&fx, true);
    x.profile.model_discrepancy[2].bound = RequirementValue::Unresolved {
        reason: "synthetic-unresolved-discrepancy".into(),
        owner: "synthetic-owner".into(),
        blocks: vec![x.profile.claim_scope.clone()],
    };
    let d = decide(&x, summaries(&fx, &x)).unwrap();
    assert!(matches!(d.outcome, JointOutcome::Abstain { .. }), "{d:?}");
    assert!(d.bounds.is_empty());
    // An unreviewed (revoked) bias source blocks as an invalid profile.
    let mut x = JointFixture::new(&fx, true);
    let source = match &x.profile.numerical_bounds[0].bound {
        RequirementValue::Admitted { source, .. } => source.clone(),
        _ => unreachable!(),
    };
    x.review(source, ReviewAction::Revoked);
    assert!(
        matches!(decide(&x, summaries(&fx, &x)), Err(JointError::Profile(m)) if m.starts_with("source:"))
    );
    // An unresolved requirement limit abstains with its blocker named.
    let x = JointFixture::new(&fx, false);
    let d = decide(&x, summaries(&fx, &x)).unwrap();
    match d.outcome {
        JointOutcome::Abstain { reasons } => assert_eq!(
            reasons,
            vec!["limit:synthetic_quality_limit_mg_l".to_string()]
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn wrong_family_scenario_unit_and_nonpositive_baseline_block() {
    let fx = fixture("joint.json");
    let x = JointFixture::new(&fx, true);
    let good = summaries(&fx, &x);
    let mut missing = good.clone();
    missing.pop();
    assert_eq!(
        decide(&x, missing),
        Err(JointError::Input(
            "margin data do not match the declared gate-scenario family"
        ))
    );
    let mut wrong_scenario = good.clone();
    wrong_scenario[0].member.scenario = "S03_other_duty".into();
    assert_eq!(
        decide(&x, wrong_scenario),
        Err(JointError::Input(
            "margin data do not match the declared gate-scenario family"
        ))
    );
    let mut duplicate = good.clone();
    duplicate[1] = duplicate[0].clone();
    assert_eq!(
        decide(&x, duplicate),
        Err(JointError::Input(
            "margin data do not match the declared gate-scenario family"
        ))
    );
    let mut wrong_unit = good.clone();
    wrong_unit[2].unit = DecisionUnit::MgPerLitreDispersedOil;
    assert_eq!(
        decide(&x, wrong_unit),
        Err(JointError::Input(
            "margin unit differs from its estimand unit"
        ))
    );
    for baseline in [0.0, -1.0, f64::NAN] {
        let mut bad = good.clone();
        bad[3].ratio_baseline_mean = Some(baseline);
        assert_eq!(
            decide(&x, bad),
            Err(JointError::Input(
                "ratio margin needs a strictly positive baseline mean"
            ))
        );
    }
    let mut zero_sd = good.clone();
    zero_sd[0].sd = 0.0;
    assert_eq!(
        decide(&x, zero_sd),
        Err(JointError::Input(
            "each margin needs n >= 2, a finite mean and a positive finite sd"
        ))
    );
    // A profile whose declared family omits a mandatory gate is invalid before any bound.
    let mut x = JointFixture::new(&fx, true);
    x.profile.gate_scenario_family.pop();
    x.profile.statistical_method.allocations.pop();
    assert!(
        matches!(decide(&x, good), Err(JointError::Profile(m)) if m == "incomplete_or_duplicate_gate_scenario_family")
    );
}

#[test]
fn fixed_conjunction_and_adaptive_winner_are_different_claims_from_simultaneous_display() {
    let fx = fixture("joint.json");
    let mut x = JointFixture::new(&fx, true);
    x.policy.criterion = ErrorCriterion::FixedCandidateConjunction;
    x.policy.procedure = Procedure::IntersectionUnion;
    x.profile.statistical_method.criterion = ErrorCriterion::FixedCandidateConjunction;
    x.profile.statistical_method.procedure = Procedure::IntersectionUnion;
    for a in &mut x.profile.statistical_method.allocations {
        a.alpha = f(0.05);
    }
    x.install_policy("synthetic-s02-family-policy-iu");
    let d = decide(&x, summaries(&fx, &x)).unwrap();
    assert_eq!(d.scope, Some(JointScope::FixedCandidateConjunction));
    assert!(d.interpretation.contains("NOT simultaneous"));
    // Each component is tested at alpha = 0.05, so its bound is tighter than the simultaneous one.
    let simultaneous = decide(
        &JointFixture::new(&fx, true),
        summaries(&fx, &JointFixture::new(&fx, true)),
    )
    .unwrap();
    for (a, b) in d.bounds.iter().zip(&simultaneous.bounds) {
        assert!(a.lower_bound > b.lower_bound);
    }
    let mut x = JointFixture::new(&fx, true);
    x.policy.criterion = ErrorCriterion::AdaptiveWinnerQualification;
    x.policy.selection = Selection::AdaptiveSearch;
    x.profile.statistical_method.criterion = ErrorCriterion::AdaptiveWinnerQualification;
    x.install_policy("synthetic-s02-family-policy-adaptive");
    x.profile
        .qualified_admission(&x.requirements, &x.registry)
        .unwrap();
    assert_eq!(
        decide(&x, summaries(&fx, &x)),
        Err(JointError::Unsupported(
            "adaptive winner qualification requires experiment-family ledger charges"
        ))
    );
}

#[test]
fn paired_blocks_keep_correlations_and_match_their_summaries() {
    let fx = fixture("joint.json");
    let x = JointFixture::new(&fx, true);
    let block_ids: Vec<String> = (0..20).map(|i| format!("synthetic-block-{i}")).collect();
    // Deterministic SYNTHETIC blocks: every margin shares one block effect, so they are correlated.
    let base: Vec<f64> = (0..20)
        .map(|i| ((i * 7) % 20) as f64 / 19.0 - 0.5)
        .collect();
    let margins: Vec<PairedMargin> = summaries(&fx, &x)
        .into_iter()
        .enumerate()
        .map(|(k, s)| PairedMargin {
            member: s.member,
            unit: s.unit,
            values: base
                .iter()
                .enumerate()
                .map(|(i, b)| s.mean + s.sd * (2.0 * b + 0.01 * ((i * (k + 3)) % 5) as f64))
                .collect(),
            ratio_baseline: s.ratio_baseline_mean.map(|m| vec![m; 20]),
        })
        .collect();
    let data = MarginData::PairedBlocks {
        block_ids: block_ids.clone(),
        margins: margins.clone(),
    };
    let paired = joint_lower_bounds(&x.profile, &x.requirements, &x.registry, &data).unwrap();
    let correlations = paired.correlations.clone().unwrap();
    assert!((correlations[0][0] - 1.0).abs() < 1e-12);
    assert!(
        correlations[0][1] > 0.9,
        "shared block effect keeps margins correlated"
    );
    let as_summaries: Vec<MarginSummary> = margins
        .iter()
        .map(|m| {
            let (mean, var) = crucible_stats::analysis::sample_moments(&m.values).unwrap();
            MarginSummary {
                member: m.member.clone(),
                unit: m.unit,
                n: m.values.len(),
                mean,
                sd: var.sqrt(),
                ratio_baseline_mean: m.ratio_baseline.as_ref().map(|b| b[0]),
            }
        })
        .collect();
    let summary = decide(&x, as_summaries).unwrap();
    for (a, b) in paired.bounds.iter().zip(&summary.bounds) {
        assert!((a.lower_bound - b.lower_bound).abs() < 1e-12);
    }
    let mut short = margins.clone();
    short[1].values.pop();
    assert_eq!(
        joint_lower_bounds(
            &x.profile,
            &x.requirements,
            &x.registry,
            &MarginData::PairedBlocks {
                block_ids: block_ids.clone(),
                margins: short
            }
        ),
        Err(JointError::Input(
            "every margin must be observed on every paired block"
        ))
    );
    let mut dup = block_ids.clone();
    dup[3] = dup[0].clone();
    assert_eq!(
        joint_lower_bounds(
            &x.profile,
            &x.requirements,
            &x.registry,
            &MarginData::PairedBlocks {
                block_ids: dup,
                margins
            }
        ),
        Err(JointError::Input("invalid or duplicate block identity"))
    );
}

#[test]
fn every_paired_ratio_baseline_block_must_be_strictly_positive() {
    let fx = fixture("joint.json");
    let x = JointFixture::new(&fx, true);
    let block_ids: Vec<String> = (0..20).map(|i| format!("synthetic-block-{i}")).collect();
    // Positive per-block baseline mean with one non-positive block: the mean check alone would pass.
    for bad_block in [-2.0, 0.0] {
        let mut blocks = vec![2.0; 20];
        blocks[5] = bad_block;
        let margins: Vec<PairedMargin> = summaries(&fx, &x)
            .into_iter()
            .enumerate()
            .map(|(k, s)| PairedMargin {
                member: s.member,
                unit: s.unit,
                values: (0..20)
                    .map(|i| s.mean + s.sd * ((i + k) % 5) as f64)
                    .collect(),
                ratio_baseline: s.ratio_baseline_mean.map(|_| blocks.clone()),
            })
            .collect();
        assert_eq!(
            joint_lower_bounds(
                &x.profile,
                &x.requirements,
                &x.registry,
                &MarginData::PairedBlocks {
                    block_ids: block_ids.clone(),
                    margins
                }
            ),
            Err(JointError::Input(
                "every ratio baseline block must be finite and strictly positive"
            )),
            "bad block {bad_block}"
        );
    }
}

#[test]
fn pearson_is_stable_under_huge_scale_and_offset_in_paired_blocks() {
    let fx = fixture("joint.json");
    let x = JointFixture::new(&fx, true);
    let block_ids: Vec<String> = (0..20).map(|i| format!("synthetic-block-{i}")).collect();
    // Products of variances near 1e300 overflow; the correlation must still match the small-scale value.
    let big = 1e150;
    let pattern = |i: usize, k: usize| ((i * (2 * k + 3)) % 7) as f64 + (i * (k + 1)) as f64;
    let margins: Vec<PairedMargin> = summaries(&fx, &x)
        .into_iter()
        .enumerate()
        .map(|(k, s)| PairedMargin {
            member: s.member,
            unit: s.unit,
            values: (0..20).map(|i| big * pattern(i, k)).collect(),
            ratio_baseline: s.ratio_baseline_mean.map(|_| vec![big; 20]),
        })
        .collect();
    let paired = joint_lower_bounds(
        &x.profile,
        &x.requirements,
        &x.registry,
        &MarginData::PairedBlocks {
            block_ids: block_ids.clone(),
            margins: margins.clone(),
        },
    )
    .unwrap();
    // Independent small-scale reference (no overflow at these magnitudes).
    let reference = |u: &[f64], v: &[f64]| -> f64 {
        let n = u.len() as f64;
        let (mu, mv) = (u.iter().sum::<f64>() / n, v.iter().sum::<f64>() / n);
        let cov: f64 = u.iter().zip(v).map(|(p, q)| (p - mu) * (q - mv)).sum();
        let vu: f64 = u.iter().map(|p| (p - mu).powi(2)).sum();
        let vv: f64 = v.iter().map(|q| (q - mv).powi(2)).sum();
        cov / (vu.sqrt() * vv.sqrt())
    };
    let small: Vec<Vec<f64>> = (0..margins.len())
        .map(|k| (0..20).map(|i| pattern(i, k)).collect())
        .collect();
    let correlations = paired.correlations.unwrap();
    for (k, row) in correlations.iter().enumerate() {
        for (l, c) in row.iter().enumerate() {
            assert!(
                (c - reference(&small[k], &small[l])).abs() < 1e-9,
                "({k},{l}) = {c}"
            );
        }
    }
}

#[test]
fn constant_paired_margin_never_yields_a_nonfinite_pass() {
    let fx = fixture("joint.json");
    let x = JointFixture::new(&fx, true);
    let block_ids: Vec<String> = (0..20).map(|i| format!("synthetic-block-{i}")).collect();
    let mut margins: Vec<PairedMargin> = summaries(&fx, &x)
        .into_iter()
        .map(|s| PairedMargin {
            member: s.member,
            unit: s.unit,
            values: (0..20).map(|i| s.mean + s.sd * i as f64).collect(),
            ratio_baseline: s.ratio_baseline_mean.map(|_| vec![2.0; 20]),
        })
        .collect();
    margins[1].values = vec![0.5; 20];
    let result = joint_lower_bounds(
        &x.profile,
        &x.requirements,
        &x.registry,
        &MarginData::PairedBlocks { block_ids, margins },
    );
    assert!(
        !matches!(result, Ok(ref d) if d.outcome == JointOutcome::Pass),
        "{result:?}"
    );
}
