//! Software controls for the forge: genome invariants, operator closure, determinism,
//! exact resume, fork/refusal semantics, seeding, and mesh watertightness.

use joil_forge::campaign::Campaign;
use joil_forge::crossover::crossover;
use joil_forge::develop::develop;
use joil_forge::evolve::RunState;
use joil_forge::genome::{bounds, Genome};
use joil_forge::mutate::mutate;
use joil_forge::random::{archetypes, random_genome};
use joil_forge::rng::Rng;
use joil_forge::run::{run, Options};
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("joil-forge-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn tiny() -> Campaign {
    Campaign {
        population: 6,
        islands: 2,
        migrate_every: 2,
        closure_samples: 1,
        duties: vec![joil_forge::campaign::Duty::produced_water()],
        ..Campaign::default()
    }
}

fn write_campaign(dir: &std::path::Path, c: &Campaign) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join("campaign-input.json");
    std::fs::write(&p, serde_json::to_vec(c).unwrap()).unwrap();
    p
}

/// State without wall-clock timings (the only nondeterministic field).
fn canonical(state: &RunState) -> serde_json::Value {
    let mut v = serde_json::to_value(state).unwrap();
    for h in v["history"].as_array_mut().unwrap() {
        h["wall_s"] = serde_json::json!(0.0);
    }
    v
}

#[test]
fn genome_normalisation_is_idempotent_and_round_trips() {
    let mut r = Rng::new(3);
    for i in 0..150 {
        let g = random_genome(&mut r, 1 + 20 * i);
        let again = g.clone().normalized();
        assert_eq!(g, again, "normalise not idempotent");
        let back: Genome = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(back, g);
        assert_eq!(back.canonical_sha256(), g.canonical_sha256());
    }
}

#[test]
fn operators_stay_inside_the_genome_language() {
    let mut r = Rng::new(9);
    let mut next = 10_000;
    let mut pool: Vec<Genome> = archetypes().into_iter().map(|(_, g)| g).collect();
    for i in 0..40 {
        pool.push(random_genome(&mut r, 100 + 20 * i));
    }
    for k in 0..400 {
        let a = &pool[r.below(pool.len())];
        let b = &pool[r.below(pool.len())];
        let child = if k % 3 == 0 {
            crossover(a, b, &mut r)
        } else {
            mutate(a, &mut r, &mut next, 0.6).0
        };
        assert!(!child.units.is_empty());
        assert_eq!(child.clone().normalized(), child);
        assert!(child.units.len() <= bounds::MAX_UNITS);
        assert!(child.total_cells() <= bounds::MAX_TOTAL_CELLS);
        for u in &child.units {
            assert_eq!(u.routes.len(), u.body.outlets());
        }
        develop(&child, &Campaign::default().envelope).expect("every normalised genome develops");
        pool.push(child);
    }
}

#[test]
fn evolution_is_independent_of_thread_count() {
    let c = tiny();
    let mut states = Vec::new();
    for threads in [1usize, 2] {
        let dir = tmp(&format!("threads{threads}"));
        let campaign = write_campaign(&dir, &c);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        let st = pool
            .install(|| {
                run(&Options {
                    campaign: Some(campaign),
                    run_dir: dir.join("run"),
                    generations: 3,
                    quiet: true,
                    ..Default::default()
                })
            })
            .unwrap();
        states.push(canonical(&st));
    }
    assert_eq!(states[0], states[1]);
}

#[test]
fn resume_continues_bit_for_bit_and_refuses_a_changed_campaign() {
    let c = tiny();
    let a = tmp("straight");
    let ca = write_campaign(&a, &c);
    let straight = run(&Options {
        campaign: Some(ca.clone()),
        run_dir: a.join("run"),
        generations: 4,
        quiet: true,
        ..Default::default()
    })
    .unwrap();
    let b = tmp("resumed");
    let cb = write_campaign(&b, &c);
    run(&Options {
        campaign: Some(cb.clone()),
        run_dir: b.join("run"),
        generations: 2,
        quiet: true,
        ..Default::default()
    })
    .unwrap();
    let resumed = run(&Options {
        run_dir: b.join("run"),
        generations: 4,
        resume: true,
        quiet: true,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(canonical(&straight), canonical(&resumed));
    // A fresh start into an existing run directory is refused.
    assert!(run(&Options {
        campaign: Some(cb),
        run_dir: b.join("run"),
        generations: 5,
        quiet: true,
        ..Default::default()
    })
    .is_err());
    // A changed campaign needs an explicit fork.
    let mut changed = c.clone();
    changed.limits.quality_limit_mg_l = 20.0;
    let cc = write_campaign(&tmp("changed"), &changed);
    let refused = run(&Options {
        campaign: Some(cc.clone()),
        run_dir: b.join("run"),
        generations: 5,
        resume: true,
        quiet: true,
        ..Default::default()
    });
    assert!(refused.unwrap_err().contains("--fork"));
    let forked = run(&Options {
        campaign: Some(cc),
        run_dir: b.join("run"),
        generations: 5,
        resume: true,
        fork: true,
        quiet: true,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(forked.generation, 5);
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(b.join("run/manifest.json")).unwrap()).unwrap();
    assert!(manifest["parent_manifest_sha256"].is_string());
}

#[test]
fn a_new_run_can_start_from_the_last_best() {
    let c = tiny();
    let a = tmp("parent");
    let ca = write_campaign(&a, &c);
    let parent = run(&Options {
        campaign: Some(ca.clone()),
        run_dir: a.join("run"),
        generations: 2,
        quiet: true,
        ..Default::default()
    })
    .unwrap();
    let best = parent.best.unwrap();
    let child = run(&Options {
        campaign: Some(ca),
        run_dir: a.join("child"),
        generations: 1,
        seed_from: Some(a.join("run")),
        quiet: true,
        ..Default::default()
    })
    .unwrap();
    let all: Vec<String> = child
        .islands
        .iter()
        .flat_map(|i| i.pop.iter().map(|p| p.hash.clone()))
        .collect();
    let child_best = child.best.unwrap();
    assert!(all.contains(&best.hash) || child_best.score.compare(&best.score).is_le());
}

#[test]
fn cad_meshes_are_closed_oriented_single_parts() {
    let g = archetypes()
        .into_iter()
        .find(|(n, _)| *n == "mini-cells")
        .unwrap()
        .1;
    // 0.8 mm: coarse enough for a debug test, fine enough that the 1 mm jets stay open.
    let m = joil_forge::cad::build(&g, &Campaign::default(), 0.8).unwrap();
    for (name, a) in [("fluid", &m.report.fluid), ("part", &m.report.part)] {
        assert!(a.closed, "{name} open");
        assert!(a.consistently_oriented, "{name} misoriented");
        assert_eq!(a.components, 1, "{name} components");
        assert!(a.volume_mm3 > 0.0);
    }
    assert!(
        m.report.fluid_volume_rel_diff.abs() < 0.15,
        "{}",
        m.report.fluid_volume_rel_diff
    );
    // Seven cells sharing three chambers make loops: the fluid domain has positive genus.
    assert!(m.report.fluid.genus[0] >= 6);
    // Printed standing on its head: plenum posts and teardrop runs leave little ceiling
    // beyond the bridge limit.
    let pa = &m.report.print;
    assert!(pa.support_posts > 0);
    assert!(
        pa.unsupported_ceiling_area_mm2 < 0.1 * pa.ceiling_area_mm2,
        "{} of {} mm2 unsupported",
        pa.unsupported_ceiling_area_mm2,
        pa.ceiling_area_mm2
    );
}

#[test]
fn folding_puts_series_stages_side_by_side_when_they_fit() {
    // Two small one-cell stages in series: stacked they need two bays; folded, one.
    let mut g = archetypes()
        .into_iter()
        .find(|(n, _)| *n == "series-pair")
        .unwrap()
        .1;
    for u in &mut g.units {
        if let joil_forge::genome::Body::Cyclone(c) = &mut u.body {
            c.diameter_mm = 10.0;
            c.cells = 1;
            c.chamber_len_ratio = 0.5;
            c.segments = vec![
                joil_forge::genome::Segment {
                    len_ratio: 1.0,
                    end_ratio: 0.5,
                },
                joil_forge::genome::Segment {
                    len_ratio: 12.0,
                    end_ratio: 0.4,
                },
            ];
            c.orifice_ratio = 0.15;
        }
        for r in &mut u.routes {
            r.channel_mm = 4.0;
        }
    }
    g.feed.channel_mm = 4.0;
    let c = Campaign::default();
    let ph = develop(&g, &c.envelope).unwrap();
    assert!(ph.violations.is_empty(), "{:?}", ph.violations);
    assert_eq!(
        ph.layout.bays.len(),
        1,
        "series stages should share one bay"
    );
    assert_eq!(ph.behavior.series_depth, 2);
}

#[test]
fn committed_campaign_matches_the_documented_defaults() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../campaigns/forge-v2.json");
    let text = std::fs::read_to_string(path).unwrap();
    let committed: Campaign = serde_json::from_str(&text).unwrap();
    assert_eq!(
        committed,
        Campaign::default(),
        "regenerate campaigns/forge-v2.json"
    );
}

#[test]
fn random_search_control_spends_exactly_its_budget() {
    let c = tiny();
    let r = joil_forge::control::random_search(&c, 15, 6);
    assert_eq!(r.evaluations, 15);
    assert_eq!(r.trajectory.len(), 3);
    assert!(r.feasible <= 15);
    let again = joil_forge::control::random_search(&c, 15, 6);
    assert_eq!(
        serde_json::to_value(&r).unwrap(),
        serde_json::to_value(&again).unwrap()
    );
}

#[test]
fn closure_seed_lets_replicates_share_one_objective() {
    use joil_forge::score::Evaluator;
    // Absent, the field changes nothing: forge-v2 keeps its bytes and its digest.
    let base = Campaign::default();
    let text = serde_json::to_string(&base).unwrap();
    assert!(!text.contains("closure_seed"));
    assert_eq!(
        base.sha256(),
        "29875952c998a3c40f466730b1e957dad14d1aa375485efbfab47a54eb0b9c28"
    );
    assert_eq!(base.closure_seed(), base.seed);
    let closures = |c: &Campaign| Evaluator::new(c).closures;
    // Another search seed alone draws other closures; pinning the closure seed restores them.
    let other = Campaign {
        seed: base.seed + 80,
        ..base.clone()
    };
    assert_ne!(closures(&other), closures(&base));
    let replicate = Campaign {
        closure_seed: Some(base.seed),
        ..other
    };
    assert_eq!(closures(&replicate), closures(&base));
    // Every committed fleet campaign is forge-v2 with only its search settings changed.
    for n in 1..=9u64 {
        let path = format!(
            "{}/../../campaigns/forge-v2-fleet/seed-{}.json",
            env!("CARGO_MANIFEST_DIR"),
            20_261_100 + n
        );
        let c: Campaign = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        c.validate().unwrap();
        assert_eq!(c.seed, 20_261_100 + n, "{path}");
        assert_eq!(closures(&c), closures(&base), "{path}");
        let scoring = Campaign {
            name: base.name.clone(),
            seed: base.seed,
            closure_seed: None,
            population: base.population,
            islands: base.islands,
            ..c
        };
        assert_eq!(
            scoring, base,
            "{path}: only name, seeds, population and islands may differ"
        );
    }
}
