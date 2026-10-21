//! Regression controls for unknown physics, configuration validation, and forked scores.
//! The historical genome is a software fixture, never physical validation.

use joil_forge::campaign::Campaign;
use joil_forge::genome::Genome;
use joil_forge::score::Evaluator;

fn fixture() -> Genome {
    let receipt: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/forge/results/forge-v2-demo-receipt.json"
    ))
    .unwrap();
    serde_json::from_value(receipt["best"]["genome"].clone()).unwrap()
}

#[test]
fn successful_duties_keep_measured_outlets_and_finite_fitness() {
    let campaign = Campaign::default();
    let ev = Evaluator::new(&campaign);
    let (score, ph, _) = ev.evaluate_full(&fixture(), campaign.resolution);
    assert!(score.feasible, "{:?}", score.reasons);
    assert!(!score.numerical_failure);
    assert!(score.fitness.is_finite());
    let plant = ph.unwrap().plant;
    let q = score.q_treated_m3_h / (1.0 - score.reject_fraction.unwrap());
    for (d, duty) in campaign.duties.iter().enumerate() {
        let worst = (0..ev.closures.len())
            .map(|k| {
                let measured = joil_forge_judges::network::evaluate(
                    &plant,
                    &duty.feed.with_flow(q),
                    &ev.duty_settings(d, k, campaign.resolution),
                )
                .unwrap();
                if d == 0 {
                    assert!(measured.cout_mg_l <= campaign.limits.quality_limit_mg_l);
                    assert!(
                        measured.equivalent_pressure_pa
                            <= campaign.limits.pressure_budget_pa * (1.0 + 1e-9)
                    );
                }
                measured.cout_mg_l
            })
            .fold(0.0_f64, f64::max);
        assert_eq!(score.duty_cout_mg_l[d], Some(worst));
    }
}

#[test]
fn failed_secondary_duty_cannot_be_scored_as_measured_zero_removal() {
    let mut campaign = Campaign::default();
    // Finite inputs whose dynamic-pressure arithmetic overflows only on duty 1.
    campaign.duties[1].fluids.rho_water_kg_m3 = f64::MAX;
    campaign.validate().unwrap();
    let ev = Evaluator::new(&campaign);
    let (score, ph, _) = ev.evaluate_full(&fixture(), campaign.resolution);
    let plant = ph.unwrap().plant;
    assert!(matches!(
        joil_forge_judges::network::evaluate(
            &plant,
            &campaign.duties[1].feed,
            &ev.duty_settings(1, 0, campaign.resolution),
        ),
        Err(joil_forge_judges::JudgeError::Numerical(_))
    ));
    assert!(
        !score.feasible,
        "a failed duty cannot qualify the objective"
    );
    assert!(score.numerical_failure);
    assert_eq!(score.fitness, 0.0);
    assert_eq!(score.q_treated_m3_h, 0.0);
    assert_eq!(score.duty_cout_mg_l.len(), campaign.duties.len());
    assert!(score.duty_cout_mg_l[0].is_some());
    assert!(score.duty_cout_mg_l[1].is_none());
    assert!(score
        .reasons
        .iter()
        .any(|r| r.contains("duty 1") && r.contains("closure 0") && r.contains("numerical")));
}

#[test]
fn invalid_campaign_arithmetic_is_rejected_before_search() {
    let cases: Vec<(&str, Campaign)> = vec![
        ("nonfinite duty weight", {
            let mut c = Campaign::default();
            c.duties[1].weight = f64::NAN;
            c
        }),
        ("infinite objective floor", {
            let mut c = Campaign::default();
            c.objective.floor_mg_l = f64::INFINITY;
            c
        }),
        ("nonfinite objective weight", {
            let mut c = Campaign::default();
            c.objective.capacity = f64::NAN;
            c
        }),
        (
            "invalid archive rate",
            Campaign {
                archive_parent_rate: 2.0,
                ..Campaign::default()
            },
        ),
        ("overflowing duty weights", {
            let mut c = Campaign::default();
            for d in &mut c.duties {
                d.weight = f64::MAX;
            }
            c
        }),
        ("reversed flow bounds", {
            let mut c = Campaign::default();
            c.limits.q_max_m3_h = 0.1;
            c
        }),
        ("invalid reject budget", {
            let mut c = Campaign::default();
            c.limits.reject_budget = 1.0;
            c
        }),
        ("zero probe count", {
            let mut c = Campaign::default();
            c.limits.quality_probes = 0;
            c
        }),
        ("nonfinite envelope", {
            let mut c = Campaign::default();
            c.envelope.radius_mm = f64::INFINITY;
            c
        }),
        ("nonfinite fluid", {
            let mut c = Campaign::default();
            c.duties[1].fluids.mu_water_pa_s = f64::INFINITY;
            c
        }),
        ("nonfinite drop size", {
            let mut c = Campaign::default();
            c.duties[1].feed.dsd.volume_median_um = f64::INFINITY;
            c
        }),
        ("zero pump efficiency", {
            let mut c = Campaign::default();
            c.terminals.pump_efficiency = 0.0;
            c
        }),
    ];
    for (name, campaign) in cases {
        assert!(campaign.validate().is_err(), "accepted {name}");
    }
}

#[test]
fn finite_weights_that_overflow_the_objective_cannot_qualify() {
    let mut campaign = Campaign::default();
    campaign.objective.separation = f64::MAX;
    campaign.validate().unwrap();
    let score = Evaluator::new(&campaign).evaluate(&fixture()).0;
    assert!(!score.feasible);
    assert!(score.numerical_failure);
    assert_eq!(score.fitness, 0.0);
    assert!(score
        .reasons
        .iter()
        .any(|r| r.contains("nonfinite objective")));
}

#[test]
fn a_campaign_fork_rescores_all_live_candidates_before_saving() {
    use joil_forge::checkpoint;
    use joil_forge::run::{run, Options};
    let dir = std::env::temp_dir().join(format!("joil-forge-score-fork-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut campaign = Campaign {
        population: 4,
        islands: 1,
        closure_samples: 0,
        ..Campaign::default()
    };
    let campaign_path = dir.join("campaign.json");
    std::fs::write(&campaign_path, serde_json::to_vec(&campaign).unwrap()).unwrap();
    let seed_path = dir.join("seed.json");
    std::fs::write(&seed_path, serde_json::to_vec(&fixture()).unwrap()).unwrap();
    let run_dir = dir.join("run");
    let parent = run(&Options {
        campaign: Some(campaign_path.clone()),
        run_dir: run_dir.clone(),
        seed_from: Some(seed_path),
        generations: 1,
        quiet: true,
        ..Options::default()
    })
    .unwrap();
    assert!(parent.best.as_ref().unwrap().score.feasible);
    let parent_checkpoint = std::fs::read(run_dir.join("checkpoint.json")).unwrap();
    let parent_history = std::fs::read(run_dir.join("history.jsonl")).unwrap();
    assert!(!parent.history.is_empty());
    campaign.objective.capacity += 2.0;
    std::fs::write(&campaign_path, serde_json::to_vec(&campaign).unwrap()).unwrap();
    let fork = run(&Options {
        campaign: Some(campaign_path),
        run_dir: run_dir.clone(),
        resume: true,
        fork: true,
        generations: 1,
        quiet: true,
        ..Options::default()
    })
    .unwrap();
    let ev = Evaluator::new(&campaign);
    for ind in fork
        .islands
        .iter()
        .flat_map(|i| &i.pop)
        .chain(fork.archive.cells.values())
        .chain(fork.best.iter())
    {
        assert_eq!(
            ind.score,
            ev.evaluate(&ind.genome).0,
            "stale score after fork"
        );
    }
    assert_ne!(
        parent.best.unwrap().score.fitness,
        fork.best.as_ref().unwrap().score.fitness
    );
    assert!(fork.evaluations > parent.evaluations);
    assert!(fork.history.is_empty());
    assert!(std::fs::read(run_dir.join("history.jsonl"))
        .unwrap()
        .is_empty());
    let cp = checkpoint::load(&run_dir).unwrap();
    let parent_id = cp.manifest.parent_manifest_sha256.unwrap();
    assert_eq!(
        std::fs::read(
            run_dir
                .join("lineage")
                .join(&parent_id)
                .join("checkpoint.json")
        )
        .unwrap(),
        parent_checkpoint
    );
    assert_eq!(
        std::fs::read(
            run_dir
                .join("lineage")
                .join(parent_id)
                .join("history.jsonl")
        )
        .unwrap(),
        parent_history
    );
}
