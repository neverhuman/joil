//! V5-04-01: prospective replication planning on SYNTHETIC fixtures.
mod common;
use common::{cauchy_quantile, f64s, fixture, noncentral_t_power, strings};
use crucible_stats::analysis::pilot::*;
use crucible_stats::StatsError;

fn design(value: &serde_json::Value) -> ProspectiveDesign {
    serde_json::from_value(value.clone()).unwrap()
}

fn blocks(ids: &serde_json::Value, effects: &[f64]) -> Vec<ConfirmationBlock> {
    strings(ids)
        .into_iter()
        .zip(effects)
        .map(|(block_id, &paired_effect)| ConfirmationBlock {
            block_id,
            independent_preparation: true,
            paired_effect,
        })
        .collect()
}

#[test]
fn small_n_bound_uses_the_independent_critical_value_and_rejects_the_legacy_one() {
    let fx = fixture("pilot.json");
    let plan = plan_replication(&design(&fx["df4_design"])).unwrap();
    assert_eq!(
        plan.required_blocks,
        fx["df4_expected_plan_blocks"].as_u64().unwrap() as usize
    );
    let c = &fx["df4_confirmation"];
    let result = confirm(&plan, &blocks(&c["block_ids"], &f64s(&c["paired_effects"]))).unwrap();
    let tolerance = fx["regression_tolerance"].as_f64().unwrap();
    let independent = fx["independent_critical_values"]["t_0975_df4"]
        .as_f64()
        .unwrap();
    assert!((result.t_critical - independent).abs() < tolerance);
    // The value the legacy source actually used fails the same regression tolerance.
    let legacy = fx["legacy_observation"]["implied_t_df4"].as_f64().unwrap();
    assert!((legacy - independent).abs() > 1e3 * tolerance);
    let hand = &c["hand_derived"];
    assert!((result.mean_paired_effect - hand["mean"].as_f64().unwrap()).abs() < 1e-12);
    assert!((result.sd_paired_effect - hand["sd"].as_f64().unwrap()).abs() < 1e-9);
    assert!((result.lower_confidence_bound - hand["lower_bound"].as_f64().unwrap()).abs() < 1e-6);
    assert!(result.qualified);
    assert_eq!(result.planned_blocks, 5);
}

#[test]
fn df1_bound_matches_the_closed_form_cauchy_oracle() {
    let fx = fixture("pilot.json");
    let plan = plan_replication(&design(&fx["cauchy_design"])).unwrap();
    assert_eq!(plan.required_blocks, 2);
    let c = &fx["cauchy_confirmation"];
    let result = confirm(&plan, &blocks(&c["block_ids"], &f64s(&c["paired_effects"]))).unwrap();
    assert!((result.t_critical - cauchy_quantile(0.75)).abs() < 1e-12);
    assert!((result.t_critical - c["hand_derived"]["t_critical"].as_f64().unwrap()).abs() < 1e-12);
    assert!(
        (result.lower_confidence_bound - c["hand_derived"]["lower_bound"].as_f64().unwrap()).abs()
            < 1e-12
    );
}

#[test]
fn planned_count_is_adequate_and_efficient_under_exact_noncentral_power() {
    let fx = fixture("pilot.json");
    for case in fx["adequacy_cases"].as_array().unwrap() {
        let sd = case["pilot_sd"].as_f64().unwrap();
        let effect = case["useful_effect"].as_f64().unwrap();
        let alpha = case["alpha_one_sided"].as_f64().unwrap();
        let power = case["power"].as_f64().unwrap();
        let d = ProspectiveDesign {
            design_id: "synthetic-adequacy".into(),
            pilot_block_ids: vec!["p1".into(), "p2".into(), "p3".into()],
            pilot_paired_effects: vec![1.0 - sd, 1.0, 1.0 + sd],
            claim_threshold: 0.0,
            useful_effect: effect,
            alpha_one_sided: alpha,
            power,
            minimum_blocks: 2,
        };
        let plan = plan_replication(&d).unwrap();
        assert!((plan.pilot_sd - sd).abs() < 1e-12 * sd.max(1.0));
        let exact = |n: usize| {
            let critical =
                crucible_stats::distribution::student_t_quantile(1.0 - alpha, n - 1).unwrap();
            noncentral_t_power(n, critical, effect / sd)
        };
        let exact_minimum = (3..10_000).find(|&n| exact(n) >= power).unwrap();
        assert!(
            exact(plan.required_blocks) >= power - 1e-4,
            "{case}: {}",
            plan.required_blocks
        );
        assert!(
            plan.required_blocks == exact_minimum || plan.required_blocks == exact_minimum + 1,
            "{case}: planned {} exact minimum {exact_minimum}",
            plan.required_blocks
        );
    }
}

#[test]
fn confirmation_outcomes_cannot_change_the_frozen_plan() {
    let fx = fixture("pilot.json");
    let plan = plan_replication(&design(&fx["df4_design"])).unwrap();
    let frozen = plan.clone();
    let c = &fx["df4_confirmation"];
    let original = f64s(&c["paired_effects"]);
    let shift = fx["mean_shift"].as_f64().unwrap();
    let shifted: Vec<f64> = original.iter().map(|x| x + shift).collect();
    let a = confirm(&plan, &blocks(&c["block_ids"], &original)).unwrap();
    let b = confirm(&plan, &blocks(&c["block_ids"], &shifted)).unwrap();
    assert_eq!(plan, frozen);
    assert_eq!(a.planned_blocks, b.planned_blocks);
    assert_eq!(a.design_identity, b.design_identity);
    assert!((b.mean_paired_effect - a.mean_paired_effect - shift).abs() < 1e-12);
    // The legacy code re-planned from the observed effect (61 -> 11 blocks for the same spread).
    let legacy = &fx["legacy_observation"]["observed_effect_planning"];
    assert_ne!(
        legacy["required_blocks_original"],
        legacy["required_blocks_after_mean_shift"]
    );
    // A plan edited after the fact no longer matches its declared design.
    let mut tampered = plan.clone();
    tampered.required_blocks = 3;
    assert_eq!(
        confirm(&tampered, &blocks(&c["block_ids"], &original[..3])),
        Err(StatsError::InvalidInput(
            "sample plan does not match its declared design"
        ))
    );
}

#[test]
fn invalid_designs_are_refused_before_planning() {
    let fx = fixture("pilot.json");
    let base = design(&fx["df4_design"]);
    let mut cases: Vec<(ProspectiveDesign, &str)> = Vec::new();
    for bad in [f64::NAN, f64::INFINITY, 0.0, -0.05] {
        let mut d = base.clone();
        d.useful_effect = bad;
        cases.push((d, "useful effect must be finite and positive"));
    }
    let mut d = base.clone();
    d.pilot_paired_effects[1] = f64::NAN;
    cases.push((d, "need at least two finite values"));
    let mut d = base.clone();
    d.pilot_block_ids = vec!["pilot-a".into()];
    d.pilot_paired_effects = vec![0.1];
    cases.push((d, "insufficient independent pilot preparations"));
    let mut d = base.clone();
    d.pilot_block_ids[2] = "pilot-a".into();
    cases.push((d, "invalid or duplicate preparation identity"));
    for (alpha, power, message) in [
        (0.0, 0.9, "one-sided alpha must be in (0, 0.5)"),
        (0.5, 0.9, "one-sided alpha must be in (0, 0.5)"),
        (f64::NAN, 0.9, "one-sided alpha must be in (0, 0.5)"),
        (0.025, 0.5, "power must be in (0.5, 1)"),
        (0.025, 1.0, "power must be in (0.5, 1)"),
    ] {
        let mut d = base.clone();
        d.alpha_one_sided = alpha;
        d.power = power;
        cases.push((d, message));
    }
    let mut d = base.clone();
    d.claim_threshold = f64::INFINITY;
    cases.push((d, "nonfinite claim threshold"));
    for (d, message) in cases {
        let error = plan_replication(&d).unwrap_err();
        assert!(
            error == StatsError::InvalidInput(message),
            "{message}: {error:?}"
        );
    }
    assert_eq!(
        required_blocks(0.0, 0.05, 0.025, 0.9, 2),
        Err(StatsError::InvalidInput(
            "planning sd must be finite and positive"
        ))
    );
    assert_eq!(
        required_blocks(f64::NAN, 0.05, 0.025, 0.9, 2),
        Err(StatsError::InvalidInput(
            "planning sd must be finite and positive"
        ))
    );
    assert_eq!(
        required_blocks(1.0, 0.05, 0.025, 0.9, 1),
        Err(StatsError::InvalidInput(
            "minimum blocks outside [2, MAX_PLANNED_BLOCKS]"
        ))
    );
    assert_eq!(
        required_blocks(1.0, 1e-6, 0.025, 0.9, 2),
        Err(StatsError::Unsupported(
            "required blocks exceed MAX_PLANNED_BLOCKS"
        ))
    );
    let mut d = base.clone();
    d.pilot_paired_effects = vec![0.125; 3];
    assert_eq!(
        plan_replication(&d),
        Err(StatsError::Unsupported(
            "zero pilot variance cannot plan replication; no artificial variance floor"
        ))
    );
}

#[test]
fn confirmation_refuses_reuse_dependence_wrong_count_and_zero_variance() {
    let fx = fixture("pilot.json");
    let plan = plan_replication(&design(&fx["df4_design"])).unwrap();
    let c = &fx["df4_confirmation"];
    let good = blocks(&c["block_ids"], &f64s(&c["paired_effects"]));
    let mut reused = good.clone();
    reused[0].block_id = "pilot-b".into();
    assert_eq!(
        confirm(&plan, &reused),
        Err(StatsError::InvalidInput(
            "pilot preparation reused as a confirmation block"
        ))
    );
    let mut duplicate = good.clone();
    duplicate[4].block_id = duplicate[0].block_id.clone();
    assert_eq!(
        confirm(&plan, &duplicate),
        Err(StatsError::InvalidInput(
            "invalid or duplicate preparation identity"
        ))
    );
    let mut frames = good.clone();
    frames[2].independent_preparation = false;
    assert_eq!(
        confirm(&plan, &frames),
        Err(StatsError::InvalidInput(
            "frames or sub-samples are not independent preparations"
        ))
    );
    let mut extended = good.clone();
    extended.push(ConfirmationBlock {
        block_id: "confirm-6".into(),
        independent_preparation: true,
        paired_effect: 0.15,
    });
    for wrong_count in [&good[..4], &extended[..]] {
        assert_eq!(
            confirm(&plan, wrong_count),
            Err(StatsError::InvalidInput(
                "confirmation must use exactly the frozen planned block count"
            ))
        );
    }
    let mut nonfinite = good.clone();
    nonfinite[1].paired_effect = f64::INFINITY;
    assert_eq!(
        confirm(&plan, &nonfinite),
        Err(StatsError::InvalidInput("need at least two finite values"))
    );
    // The legacy code floored a zero SD to 1e-6 and reported a passing gate.
    let constant: Vec<ConfirmationBlock> = good
        .iter()
        .map(|b| ConfirmationBlock {
            paired_effect: 0.2,
            ..b.clone()
        })
        .collect();
    assert_eq!(
        confirm(&plan, &constant),
        Err(StatsError::Unsupported(
            "zero confirmation variance leaves the t bound undefined; no artificial floor"
        ))
    );
}
