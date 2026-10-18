//! V5-04-05: preregistered SYNTHETIC calibration and experiment-family ledger charges.
mod common;
#[path = "../../crucible-contract/tests/common/scratch.rs"]
mod scratch;
use common::joint::{f, JointFixture};
use common::{fixture, SplitMix64};
use crucible_runtime::{db::RuntimeDb, ledger::*, RuntimeError, SemanticId};
use crucible_stats::analysis::calibration::*;
use crucible_stats::analysis::joint::*;
use crucible_stats::analysis::pilot::{
    confirm, plan_replication, ConfirmationBlock, ProspectiveDesign,
};
use crucible_stats::analysis::sample_moments;
use crucible_stats::StatsError;
use joil_evidence::decision::*;
use scratch::TestScratch;

fn registration() -> Preregistration {
    serde_json::from_value(fixture("calibration-preregistration.json")).unwrap()
}

fn case<'a>(r: &'a Preregistration, id: &str) -> &'a CalibrationCase {
    r.cases.iter().find(|c| c.id == id).unwrap()
}

#[test]
fn preregistration_is_bounded_identified_and_reports_distinct_metrics() {
    let r = registration();
    let identity = r.check().unwrap();
    assert_eq!(identity.len(), 64);
    let metrics: std::collections::BTreeSet<String> = r
        .cases
        .iter()
        .map(|c| serde_json::to_string(&c.metric).unwrap())
        .collect();
    for m in [
        "\"coverage\"",
        "\"false_qualification\"",
        "\"power\"",
        "\"assumption_violation_demonstration\"",
    ] {
        assert!(metrics.contains(m), "{m}");
    }
    let total: u64 = r.prior_packet_trials + r.cases.iter().map(|c| c.trials).sum::<u64>();
    assert!(total <= MAX_TOTAL_TRIALS);
    let mut over = r.clone();
    over.cases[0].trials = MAX_TOTAL_TRIALS;
    assert_eq!(
        over.check(),
        Err(StatsError::InvalidInput(
            "registered trials exceed the packet bound"
        ))
    );
    let mut wide = r.clone();
    wide.cases[0].independent_units_per_trial = MAX_UNITS_PER_TRIAL + 1;
    assert_eq!(
        wide.check(),
        Err(StatsError::InvalidInput(
            "case outside the unit or cluster bound"
        ))
    );
    let mut clustered = r.clone();
    clustered.cases[2].clusters_per_trial = MAX_CLUSTERS_PER_TRIAL + 1;
    assert_eq!(
        clustered.check(),
        Err(StatsError::InvalidInput(
            "case outside the unit or cluster bound"
        ))
    );
    let mut duplicate = r.clone();
    duplicate.cases[1].id = duplicate.cases[0].id.clone();
    assert_eq!(
        duplicate.check(),
        Err(StatsError::InvalidInput(
            "invalid or duplicate case identity"
        ))
    );
    // A tuned rerun changes the registration identity; a short run is not the registered case.
    let mut tuned = r.clone();
    tuned.cases[0].seed += 1;
    assert_ne!(tuned.check().unwrap(), identity);
    assert_eq!(
        report(&r, "joint-simultaneous-coverage-normal", 900, 999),
        Err(StatsError::InvalidInput(
            "executed trials differ from the registration"
        ))
    );
    assert_eq!(
        report(&r, "unregistered", 1, 1000),
        Err(StatsError::InvalidInput("unregistered calibration case"))
    );
    let at_least = report(&r, "joint-simultaneous-coverage-normal", 951, 1000).unwrap();
    assert!(at_least.passed && at_least.metric == Metric::Coverage);
    assert!((at_least.monte_carlo_se - (0.951f64 * 0.049 / 1000.0).sqrt()).abs() < 1e-15);
    let at_most = report(&r, "joint-iu-false-qualification-boundary", 80, 1000).unwrap();
    assert!(!at_most.passed && at_most.metric == Metric::FalseQualification);
    // The pilot cases must fit their declared unit bound before any trial runs.
    for id in [
        "pilot-plan-power-alternative",
        "pilot-plan-false-qualification-null",
    ] {
        let plan = plan_replication(&pilot_design(&r)).unwrap();
        assert!(plan.required_blocks as u64 <= case(&r, id).independent_units_per_trial);
    }
}

fn authorized() -> Resources {
    Resources {
        cpu_ms: 20_000,
        gpu_ms: 0,
        ram_bytes: 512 * 1024 * 1024,
        vram_bytes: 0,
        scratch_bytes: 32 * 1024 * 1024,
        io_bytes: 128 * 1024 * 1024,
    }
}

/// Two-attempt family: each attempt reserves alpha 0.05 / 2 before any qualification work.
fn two_attempt_family(fx: &serde_json::Value) -> JointFixture {
    let mut x = JointFixture::new(fx, true);
    x.policy.max_attempts = 2;
    for a in &mut x.profile.statistical_method.allocations {
        a.alpha = f(0.00625);
    }
    x.install_policy("synthetic-s02-family-policy-two-attempts");
    x
}

#[test]
fn family_ledger_charges_every_adaptive_attempt_and_refuses_resets() {
    let _scratch = TestScratch::new();
    let fx = fixture("joint.json");
    let mut x = two_attempt_family(&fx);
    let mut db = RuntimeDb::open(
        &TestScratch::path("v5-stats-family-ledger"),
        authorized(),
        &SemanticId::of(&"synthetic statistics family ledger").unwrap(),
    )
    .unwrap();
    let first = db
        .reserve_profile_attempt(&x.profile, &x.requirements, &x.registry)
        .unwrap();
    assert_eq!(
        (
            first.attempts,
            first.spent_alpha_units,
            first.maximum_alpha_units
        ),
        (1, 25_000_000_000, 50_000_000_000)
    );
    // Re-reserving the identical attempt is idempotent; it never refunds or double-charges.
    assert_eq!(
        db.reserve_profile_attempt(&x.profile, &x.requirements, &x.registry)
            .unwrap(),
        first
    );
    // Selecting a new winner (a different decision) for attempt 1 cannot reuse its ordinal.
    let mut new_winner = x.profile.clone();
    new_winner.planning_precision[0].maximum_uncertainty_fraction = f(0.25);
    assert!(matches!(
        db.reserve_profile_attempt(&new_winner, &x.requirements, &x.registry),
        Err(RuntimeError::Conflict("family_ordinal_reused"))
    ));
    // A forked profile that restarts the attempt counter fails continuation.
    assert!(new_winner.check_continuation(&x.profile).is_err());
    // A genuine second attempt is charged in the same family.
    let mut second = x.profile.clone();
    second.attempt = 2;
    second.check_continuation(&x.profile).unwrap();
    let spend = db
        .reserve_profile_attempt(&second, &x.requirements, &x.registry)
        .unwrap();
    assert_eq!(
        (spend.attempts, spend.spent_alpha_units),
        (2, 50_000_000_000)
    );
    // Forking the family policy (here a larger alpha) cannot reset the spent ledger.
    let original_policy = x.profile.statistical_method.family_policy.clone();
    x.policy.family_alpha = f(0.1);
    x.install_policy("synthetic-s02-family-policy-fork");
    assert!(matches!(
        db.reserve_profile_attempt(&x.profile, &x.requirements, &x.registry),
        Err(RuntimeError::Conflict("family_reset_refused"))
    ));
    // The registered family is exhausted after two attempts.
    let family = ApprovedFamily::from_registry(&x.registry, original_policy, &x.reference).unwrap();
    assert!(matches!(
        db.reserve_family_attempt(
            &family,
            &x.registry,
            3,
            &SemanticId::of(&"third attempt").unwrap()
        ),
        Err(RuntimeError::Budget("family_attempt_limit_or_sequence"))
    ));
    // Joint inference still refuses adaptive winners itself; charging lives in the ledger.
    let mut adaptive = two_attempt_family(&fx);
    adaptive.policy.criterion = ErrorCriterion::AdaptiveWinnerQualification;
    adaptive.policy.selection = Selection::AdaptiveSearch;
    adaptive.profile.statistical_method.criterion = ErrorCriterion::AdaptiveWinnerQualification;
    adaptive.install_policy("synthetic-s02-family-policy-adaptive");
    assert!(matches!(
        joint_lower_bounds(
            &adaptive.profile,
            &adaptive.requirements,
            &adaptive.registry,
            &MarginData::Summaries(vec![])
        ),
        Err(JointError::Unsupported(
            "adaptive winner qualification requires experiment-family ledger charges"
        ))
    ));
}

// ---- Registered trial generators (consumed in this order; see the registration RNG statement). ----

fn zero_bias_fixture(conjunction: bool) -> JointFixture {
    let mut fx = fixture("joint.json");
    for m in fx["margins"].as_array_mut().unwrap() {
        m["adverse_bound"] = serde_json::json!(0.0);
    }
    let mut x = JointFixture::new(&fx, true);
    if conjunction {
        x.policy.criterion = ErrorCriterion::FixedCandidateConjunction;
        x.policy.procedure = Procedure::IntersectionUnion;
        x.profile.statistical_method.criterion = ErrorCriterion::FixedCandidateConjunction;
        x.profile.statistical_method.procedure = Procedure::IntersectionUnion;
        for a in &mut x.profile.statistical_method.allocations {
            a.alpha = f(0.05);
        }
        x.install_policy("synthetic-s02-family-policy-iu-zero-bias");
    }
    x
}

fn generator_f64s(g: &serde_json::Value, key: &str) -> Vec<f64> {
    common::f64s(&g[key])
}

fn summaries(x: &JointFixture, columns: &[Vec<f64>]) -> MarginData {
    MarginData::Summaries(
        x.profile
            .gate_scenario_family
            .iter()
            .zip(columns)
            .zip(&x.profile.estimands)
            .map(|((member, values), estimand)| {
                let (mean, variance) = sample_moments(values).unwrap();
                MarginSummary {
                    member: member.clone(),
                    unit: estimand.unit,
                    n: values.len(),
                    mean,
                    sd: variance.sqrt(),
                    ratio_baseline_mean: (estimand.unit == DecisionUnit::CubicMetrePerHour)
                        .then_some(3.96),
                }
            })
            .collect(),
    )
}

fn equicorrelated_blocks(rng: &mut SplitMix64, g: &serde_json::Value) -> Vec<Vec<f64>> {
    let (means, sds) = (generator_f64s(g, "true_means"), generator_f64s(g, "sds"));
    let rho = g["correlation"].as_f64().unwrap();
    let blocks = g["blocks"].as_u64().unwrap() as usize;
    let mut columns = vec![Vec::with_capacity(blocks); means.len()];
    for _ in 0..blocks {
        let shared = rng.normal();
        for (j, column) in columns.iter_mut().enumerate() {
            let z = rho.sqrt() * shared + (1.0 - rho).sqrt() * rng.normal();
            column.push(means[j] + sds[j] * z);
        }
    }
    columns
}

fn covers(decision: &JointDecision, truth: &[f64]) -> bool {
    decision
        .bounds
        .iter()
        .zip(truth)
        .all(|(b, mu)| b.lower_bound <= *mu)
}

fn print_report(report: &RateReport) {
    println!(
        "C007-CALIBRATION {}",
        serde_json::to_string(report).unwrap()
    );
}

#[test]
fn preregistered_calibration_joint_normal_coverage_and_boundary_false_qualification() {
    let r = registration();
    for (id, conjunction) in [
        ("joint-simultaneous-coverage-normal", false),
        ("joint-iu-false-qualification-boundary", true),
    ] {
        let c = case(&r, id);
        let x = zero_bias_fixture(conjunction);
        let truth = generator_f64s(&c.generator, "true_means");
        let mut rng = SplitMix64(c.seed);
        let mut successes = 0u64;
        for _ in 0..c.trials {
            let columns = equicorrelated_blocks(&mut rng, &c.generator);
            let d = joint_lower_bounds(
                &x.profile,
                &x.requirements,
                &x.registry,
                &summaries(&x, &columns),
            )
            .unwrap();
            let event = match c.metric {
                Metric::Coverage => covers(&d, &truth),
                Metric::FalseQualification => d.outcome == JointOutcome::Pass,
                _ => unreachable!(),
            };
            successes += u64::from(event);
        }
        let rep = report(&r, id, successes, c.trials).unwrap();
        print_report(&rep);
        assert!(rep.passed, "{id}");
    }
}

fn standardized_lognormal(rng: &mut SplitMix64, sigma: f64) -> f64 {
    let mean = (sigma * sigma / 2.0).exp();
    let sd = (((sigma * sigma).exp() - 1.0) * (sigma * sigma).exp()).sqrt();
    ((sigma * rng.normal()).exp() - mean) / sd
}

#[test]
fn preregistered_calibration_clustered_skewed_cluster_level_and_naive_demonstration() {
    let r = registration();
    let valid = case(&r, "joint-clustered-skewed-cluster-level-coverage");
    let naive_case = case(&r, "joint-clustered-skewed-naive-block-level");
    let g = &valid.generator;
    let (means, sds) = (generator_f64s(g, "true_means"), generator_f64s(g, "sds"));
    let clusters = g["clusters"].as_u64().unwrap() as usize;
    let per_cluster = g["blocks_per_cluster"].as_u64().unwrap() as usize;
    let x = zero_bias_fixture(false);
    let mut rng = SplitMix64(valid.seed);
    let (mut cluster_covered, mut naive_covered) = (0u64, 0u64);
    for _ in 0..valid.trials {
        let mut blocks = vec![Vec::new(); means.len()];
        let mut cluster_means = vec![Vec::new(); means.len()];
        for _ in 0..clusters {
            let u = standardized_lognormal(&mut rng, 0.75);
            let mut sums = vec![0.0; means.len()];
            for _ in 0..per_cluster {
                for j in 0..means.len() {
                    let v = means[j] + sds[j] * (u + 0.2 * rng.normal());
                    blocks[j].push(v);
                    sums[j] += v;
                }
            }
            for j in 0..means.len() {
                cluster_means[j].push(sums[j] / per_cluster as f64);
            }
        }
        let at_cluster = joint_lower_bounds(
            &x.profile,
            &x.requirements,
            &x.registry,
            &summaries(&x, &cluster_means),
        )
        .unwrap();
        cluster_covered += u64::from(covers(&at_cluster, &means));
        let naive = joint_lower_bounds(
            &x.profile,
            &x.requirements,
            &x.registry,
            &summaries(&x, &blocks),
        )
        .unwrap();
        naive_covered += u64::from(covers(&naive, &means));
    }
    let valid_report = report(&r, &valid.id, cluster_covered, valid.trials).unwrap();
    let naive_report = report(&r, &naive_case.id, naive_covered, naive_case.trials).unwrap();
    print_report(&valid_report);
    print_report(&naive_report);
    assert!(valid_report.passed, "cluster-level coverage");
    assert!(
        naive_report.passed,
        "naive block-level analysis must undercover"
    );
}

fn pilot_design(r: &Preregistration) -> ProspectiveDesign {
    let d = &case(r, "pilot-plan-power-alternative").generator["design"];
    ProspectiveDesign {
        design_id: "synthetic-calibration-design".into(),
        pilot_block_ids: vec!["pilot-1".into(), "pilot-2".into(), "pilot-3".into()],
        pilot_paired_effects: common::f64s(&d["pilot_paired_effects"]),
        claim_threshold: d["claim_threshold"].as_f64().unwrap(),
        useful_effect: d["useful_effect"].as_f64().unwrap(),
        alpha_one_sided: d["alpha_one_sided"].as_f64().unwrap(),
        power: d["power"].as_f64().unwrap(),
        minimum_blocks: d["minimum_blocks"].as_u64().unwrap() as usize,
    }
}

#[test]
fn preregistered_calibration_prospective_plan_power_and_null_false_qualification() {
    let r = registration();
    let plan = plan_replication(&pilot_design(&r)).unwrap();
    for id in [
        "pilot-plan-power-alternative",
        "pilot-plan-false-qualification-null",
    ] {
        let c = case(&r, id);
        let (mu, sd) = (
            c.generator["true_mean"].as_f64().unwrap(),
            c.generator["true_sd"].as_f64().unwrap(),
        );
        let mut rng = SplitMix64(c.seed);
        let mut qualified = 0u64;
        for t in 0..c.trials {
            let blocks: Vec<ConfirmationBlock> = (0..plan.required_blocks)
                .map(|i| ConfirmationBlock {
                    block_id: format!("t{t}-b{i}"),
                    independent_preparation: true,
                    paired_effect: mu + sd * rng.normal(),
                })
                .collect();
            qualified += u64::from(confirm(&plan, &blocks).unwrap().qualified);
        }
        let rep = report(&r, id, qualified, c.trials).unwrap();
        println!(
            "C007-CALIBRATION planned_blocks={} {}",
            plan.required_blocks,
            serde_json::to_string(&rep).unwrap()
        );
        assert!(rep.passed, "{id}");
    }
}
