//! Software controls for the fleet summary: replicate pairing at equal budget, refusal of
//! mixed campaigns and objectives, the sign test and pooled seeds.

use joil_forge::campaign::Campaign;
use joil_forge::control::random_search;
use joil_forge::fleet::{pool, sign_test_p, summarize, Verdict};
use joil_forge::run::{run, Options};
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
