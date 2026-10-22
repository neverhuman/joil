//! Software controls for the fleet summary: replicate pairing at equal budget, refusal of
//! mixed campaigns and objectives, the sign test and pooled seeds; and for the fleet figures:
//! exact step lines, every run drawn, the winner marked, and the winner card's numbers.

use joil_forge::campaign::Campaign;
use joil_forge::control::random_search;
use joil_forge::fleet::{
    pool, sign_test_p, summarize, ArmSummary, FleetSummary, Replicate, SignTest, Verdict,
};
use joil_forge::render::fleet_figures::{
    change_points, evolution_figure, seed_panels, value_at, winner_card, Mark, Reference, Winner,
};
use joil_forge::render::fleet_report::{closure_summary, write_figures, FigureOptions};
use joil_forge::run::{run, Options};
use joil_forge::score::Score;
use std::path::{Path, PathBuf};

fn tiny(seed: u64) -> Campaign {
    Campaign {
        seed,
        closure_seed: Some(7),
        population: 6,
        islands: 2,
        migrate_every: 2,
        closure_samples: 1,
        duties: vec![joil_forge::campaign::Duty::produced_water()],
        ..Campaign::default()
    }
}

fn root(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("joil-forge-fleet-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("runs")).unwrap();
    std::fs::create_dir_all(d.join("controls")).unwrap();
    d
}

fn evolve(root: &Path, name: &str, c: &Campaign, seed_from: Option<PathBuf>) -> u64 {
    let cfile = root.join(format!("{name}.campaign.json"));
    std::fs::write(&cfile, serde_json::to_vec(c).unwrap()).unwrap();
    let state = run(&Options {
        campaign: Some(cfile),
        run_dir: root.join("runs").join(name),
        generations: 3,
        seed_from,
        quiet: true,
        ..Options::default()
    })
    .unwrap();
    state.evaluations
}

fn control(root: &Path, seed: u64, c: &Campaign, evaluations: u64) {
    let r = random_search(c, evaluations, c.population * c.islands);
    std::fs::write(
        root.join("controls").join(format!("R-{seed}.json")),
        serde_json::to_vec(&r).unwrap(),
    )
    .unwrap();
}

#[test]
fn sign_test_tail_probabilities_are_exact() {
    assert_eq!(sign_test_p(8, 8), 1.0 / 256.0);
    assert_eq!(sign_test_p(7, 8), 9.0 / 256.0);
    assert_eq!(sign_test_p(0, 8), 1.0);
    assert_eq!(sign_test_p(1, 1), 0.5);
}

#[test]
fn fleet_pairs_replicates_at_equal_budget_and_refuses_mixed_campaigns() {
    let dir = root("pairs");
    // A lineage to seed from, outside the replicate naming.
    evolve(&dir, "C0", &tiny(5), None);
    for seed in [11, 12] {
        let c = tiny(seed);
        evolve(&dir, &format!("S-{seed}"), &c, Some(dir.join("runs/C0")));
        let n = evolve(&dir, &format!("D-{seed}"), &c, None);
        control(&dir, seed, &c, n);
    }
    let s = summarize(&dir).unwrap();
    assert_eq!(s.replicates.len(), 2);
    for r in &s.replicates {
        assert!(r.seeded.is_some() && r.de_novo.is_some() && r.random.is_some());
        assert_eq!(
            r.de_novo.as_ref().unwrap().evaluations,
            r.random.as_ref().unwrap().evaluations
        );
        assert_ne!(r.search_vs_random, Verdict::NotComparable);
        assert_ne!(r.seeded_vs_de_novo, Verdict::NotComparable);
    }
    let t = &s.search_vs_random;
    assert_eq!(
        t.first_better + t.second_better + t.ties_or_not_comparable,
        2
    );
    assert_eq!(t.p_one_sided, sign_test_p(t.first_better, 2));
    assert_eq!(s.other_runs.len(), 1);
    assert_eq!(s.other_runs[0].source, "C0");
    assert!(s.overall_best.is_some());
    assert!(joil_forge::fleet::chart(&s).starts_with("<svg"));

    // Pooled seeds: deduplicated, from S and D runs only, and usable by a new run.
    let genomes = pool(&dir).unwrap();
    let hashes: std::collections::HashSet<String> =
        genomes.iter().map(|g| g.canonical_sha256()).collect();
    assert_eq!(hashes.len(), genomes.len());
    assert!(!genomes.is_empty());
    let pfile = dir.join("pool.json");
    std::fs::write(&pfile, serde_json::to_vec(&genomes).unwrap()).unwrap();
    let pooled = root("pooled");
    evolve(&pooled, "P", &tiny(13), Some(pfile));

    // A control made under another replicate's campaign is refused.
    std::fs::copy(
        dir.join("controls/R-11.json"),
        dir.join("controls/R-12.json"),
    )
    .unwrap();
    let err = summarize(&dir).unwrap_err();
    assert!(err.contains("different campaigns"), "{err}");
    control(&dir, 12, &tiny(12), 1); // restore R-12 (budget now differs from D-12)
    let s = summarize(&dir).unwrap();
    let r12 = s.replicates.iter().find(|r| r.seed == 12).unwrap();
    assert_eq!(r12.search_vs_random, Verdict::NotComparable);

    // A replicate scored under another objective is refused.
    let other = Campaign {
        closure_seed: Some(8),
        ..tiny(14)
    };
    evolve(&dir, "D-14", &other, None);
    let err = summarize(&dir).unwrap_err();
    assert!(err.contains("different objective"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&pooled);
}

#[test]
fn fleet_chart_panels_share_one_fitness_scale() {
    use joil_forge::render::charts::{comparison_chart, comparison_chart_shared_y};
    let panels = vec![
        ("high panel", vec![("s", vec![(0.0, 0.5), (10.0, 3.0)])]),
        ("low panel", vec![("s", vec![(0.0, 0.5), (10.0, 1.5)])]),
    ];
    // y tick labels are the only right-anchored text; split them at the second panel's title.
    let ticks = |svg: String| {
        let (a, b) = svg.split_once(">low panel<").unwrap();
        let t = |s: &str| -> Vec<String> {
            s.split("text-anchor='end'>")
                .skip(1)
                .map(|t| t.split('<').next().unwrap().to_string())
                .collect()
        };
        (t(a), t(b))
    };
    let (high, low) = ticks(comparison_chart_shared_y("t", "x", &panels));
    assert_eq!(high, low);
    let (high, low) = ticks(comparison_chart("t", "x", &panels));
    assert_ne!(high, low);
}

fn score(fitness: f64) -> Score {
    serde_json::from_value(serde_json::json!({
        "feasible": true, "fitness": fitness, "violation": 0.0, "q_treated_m3_h": 1.0,
        "q_treated_nominal_m3_h": 1.0, "q_feed_m3_h": 1.0, "cout_mg_l": 5.0,
        "feed_pressure_kpa": 400.0, "specific_energy_kj_m3": 400.0, "shell_mass_kg": 0.5,
        "reject_fraction": 0.01, "separation": 3.0, "duty_cout_mg_l": [5.0], "units": 2,
        "per_closure_q_m3_h": [1.0], "reasons": [], "numerical_failure": false
    }))
    .unwrap()
}

fn arm(source: &str, trajectory: Vec<(f64, f64)>) -> ArmSummary {
    ArmSummary {
        source: source.into(),
        campaign_sha256: "campaign".into(),
        generation: Some(4),
        evaluations: trajectory.last().map_or(0, |p| p.0 as u64),
        best_genome_sha256: Some(format!("genome-{source}")),
        best: trajectory.last().map(|p| score(p.1)),
        niches: 1,
        feasible_niches: 1,
        first_feasible_evaluations: trajectory.first().map(|p| p.0 as u64),
        trajectory,
    }
}

/// Two replicates and one pooled run whose final score is the best of the fleet.
fn synthetic_fleet() -> FleetSummary {
    let t = |v: [f64; 3]| {
        vec![
            (400.0, v[0]),
            (4000.0, v[1]),
            (40000.0, v[1]),
            (400000.0, v[2]),
        ]
    };
    let rep = |seed: u64, d: [f64; 3], s: [f64; 3], r: [f64; 3]| Replicate {
        seed,
        seeded: Some(arm(&format!("S-{seed}"), t(s))),
        de_novo: Some(arm(&format!("D-{seed}"), t(d))),
        random: Some(arm(&format!("R-{seed}"), t(r))),
        search_vs_random: Verdict::FirstBetter,
        seeded_vs_de_novo: Verdict::SecondBetter,
    };
    let test = SignTest {
        replicates: 2,
        first_better: 2,
        second_better: 0,
        ties_or_not_comparable: 0,
        p_one_sided: 0.25,
    };
    let pooled = arm("F-9", vec![(400.0, 3.1), (400000.0, 3.2)]);
    FleetSummary {
        format: "test".into(),
        evidence_class: "unvalidated_reduced_order_screen".into(),
        forge_version: "v".into(),
        model_version: "m".into(),
        replicates: vec![
            rep(1, [1.9, 2.4, 2.9], [2.5, 2.5, 2.6], [1.6, 1.8, 2.0]),
            rep(2, [1.8, 2.2, 3.0], [2.5, 2.5, 2.5], [1.7, 1.7, 2.1]),
        ],
        search_vs_random: test.clone(),
        seeded_vs_de_novo: test,
        other_runs: vec![pooled.clone()],
        overall_best: Some(pooled),
        note: String::new(),
    }
}

#[test]
fn change_points_redraw_a_trajectory_exactly() {
    let t = vec![
        (384.0, 1.5),
        (768.0, 1.5),
        (1152.0, 1.7),
        (1536.0, 1.7),
        (1920.0, 2.0),
    ];
    let c = change_points(&t);
    assert_eq!(c, vec![(384.0, 1.5), (1152.0, 1.7), (1920.0, 2.0)]);
    for (x, y) in &t {
        assert_eq!(value_at(&c, *x), Some(*y), "at {x}");
    }
    assert_eq!(value_at(&c, 100.0), None);
    // A flat tail keeps its last point so the line reaches the end of the budget.
    let flat = vec![(10.0, 1.0), (20.0, 2.0), (30.0, 2.0)];
    assert_eq!(change_points(&flat), flat);
}

#[test]
fn evolution_figure_draws_every_run_and_stars_the_winner() {
    let s = synthetic_fleet();
    let mark = Mark {
        evaluations: 40000.0,
        label: "end of the first run".into(),
    };
    let previous = Reference {
        value: 2.5,
        label: "previous best".into(),
    };
    let svg = evolution_figure(&s, &[mark], &[previous]);
    for run in ["D-1", "D-2", "S-1", "S-2", "R-1", "R-2"] {
        assert_eq!(
            svg.matches(&format!("<title>{run}: ")).count(),
            2,
            "{run}: line and dot"
        );
    }
    // The winner is labelled at its star and at its line across the trajectory panel, where the
    // label is drawn twice (background halo, then text).
    assert_eq!(svg.matches(">winner (F-9) 3.200<").count(), 3);
    assert!(svg.contains("previous best 2.500") && svg.contains("end of the first run"));
    assert!(svg.contains(">1k<") && svg.contains(">100k<"));
    assert!(svg.contains("De novo evolution (D) · median 2.950"));
    assert!(svg.contains("Pooled final runs (P, F)"));
    assert!(svg.contains("prefers-color-scheme: dark"));
    let seeds = seed_panels(&s, &[]);
    assert!(seeds.contains(">seed 1<") && seeds.contains(">seed 2<"));
    assert!(seeds.contains(">D 2.900<") && seeds.contains(">R 2.100<"));
}

#[test]
fn winner_card_reports_the_receipt_numbers() {
    let receipt = serde_json::json!({
        "evidence_class": "unvalidated_reduced_order_screen",
        "campaign": {"duties": [{"name": "produced water", "feed": {"cin_mg_l": 500.0}}]},
        "best": {
            "behavior": {"series_depth": 2, "cyclone_cells": 14},
            "search_score": {"fitness": 3.2},
            "report_score": {"fitness": 3.19, "duty_cout_mg_l": [5.0], "q_treated_m3_h": 0.77,
                             "feed_pressure_kpa": 400.0, "specific_energy_kj_m3": 402.0}
        },
        "cad_best": {"length_mm": 400.0, "envelope_radius_mm": 50.0, "printed_mass_kg": 0.62,
                     "print": {"support_posts": 219, "unsupported_ceiling_area_mm2": 18.0,
                               "ceiling_area_mm2": 1000.0}}
    });
    let png = joil_forge::render::png::encode_rgb(1, 1, &[0, 0, 0]);
    let card = winner_card(&Winner {
        source: "F-9",
        receipt: &receipt,
        closures: Some((7, 8, 2.33)),
        hero_png: &png,
        pfd_svg: "<svg xmlns='http://www.w3.org/2000/svg'/>",
        history: &[],
    });
    assert!(card.contains("2 stages in series, 14 cyclone cells"));
    assert!(card.contains("3.200 / 3.190"));
    assert!(card.contains("500 → 5.0 mg/L (99.0% removed)"));
    assert!(card.contains("0.77 m³/h at 400 kPa"));
    assert!(card.contains("400 mm long, 100 mm across, 0.62 kg"));
    assert!(card.contains("219 posts; 1.8% of ceilings unsupported"));
    assert!(card.contains("feasible in 7 of 8; lowest score 2.330"));
    assert!(card.contains("data:image/png;base64,"));
}

#[test]
fn closure_summary_counts_evaluations_and_skips_other_files() {
    let dir = root("closures");
    let eval = |f: bool, x: f64| serde_json::json!({"score": {"feasible": f, "fitness": x}});
    for (name, v) in [
        ("eval-closure-1.json", eval(true, 2.9)),
        ("eval-closure-2.json", eval(true, 2.4)),
        ("eval-closure-3.json", eval(false, 0.0)),
        ("campaign-closure-1.json", serde_json::json!({"seed": 1})),
    ] {
        std::fs::write(dir.join(name), serde_json::to_vec(&v).unwrap()).unwrap();
    }
    assert_eq!(closure_summary(&dir).unwrap(), (2, 3, 2.4));
}

#[test]
fn fleet_figures_refuse_a_report_of_another_design() {
    let dir = root("figures");
    for seed in [31, 32] {
        let c = tiny(seed);
        let n = evolve(&dir, &format!("D-{seed}"), &c, None);
        control(&dir, seed, &c, n);
    }
    let out = dir.join("figures");
    let report = dir.join("report");
    std::fs::create_dir_all(&report).unwrap();
    std::fs::write(
        report.join("receipt.json"),
        br#"{"best": {"genome_sha256": "not-the-winner"}}"#,
    )
    .unwrap();
    let opts = FigureOptions {
        marks: vec![],
        references: vec![],
        winner_report: Some(&report),
        closures: None,
    };
    let err = write_figures(&dir, &out, &opts).unwrap_err();
    assert!(err.contains("not of the fleet's best design"), "{err}");
    let plain = FigureOptions {
        winner_report: None,
        ..opts
    };
    let files = write_figures(&dir, &out, &plain).unwrap();
    assert_eq!(files.len(), 2);
}
