//! V5-04-02: weighted clustered capture uncertainty on SYNTHETIC fixtures.
mod common;
use common::{fixture, logistic, monte_carlo_se, SplitMix64};
use crucible_stats::analysis::capture::*;
use crucible_stats::StatsError;
use std::collections::BTreeMap;

fn parcels(value: &serde_json::Value) -> Vec<Parcel> {
    serde_json::from_value(value.clone()).unwrap()
}

fn fraction(value: &serde_json::Value) -> f64 {
    value[0].as_f64().unwrap() / value[1].as_f64().unwrap()
}

fn design(
    fractions: &serde_json::Value,
    clusters: &serde_json::Value,
    unit: IndependentUnit,
) -> CaptureDesign {
    CaptureDesign {
        independent_unit: unit,
        stratum_mass_fractions: serde_json::from_value(fractions.clone()).unwrap(),
        clusters: common::strings(clusters)
            .into_iter()
            .map(|cluster_id| Cluster { cluster_id, randomization_id: None })
            .collect(),
        confidence: 0.95,
        applicability: "SYNTHETIC independent carrier realizations; fates within a realization may be correlated".into(),
    }
}

#[test]
fn unequal_weight_mass_ratio_matches_hand_derivation_not_a_count_share() {
    let fx = fixture("capture.json");
    let s = &fx["one_stratum"];
    let d = design(
        &s["stratum_mass_fractions"],
        &s["clusters"],
        IndependentUnit::CarrierCluster,
    );
    let e = estimate_capture(&d, &parcels(&s["parcels"])).unwrap();
    let hand = &s["hand_derived"];
    assert!((e.estimate - fraction(&hand["estimate_fraction"])).abs() < 1e-15);
    assert!((e.standard_error.powi(2) - fraction(&hand["variance_fraction"])).abs() < 1e-15);
    assert_eq!(e.degrees_of_freedom, 2);
    assert!((e.t_critical - fx["t_0975_df2"].as_f64().unwrap()).abs() < 1e-9);
    // A Bernoulli count share (4 of 7 parcels captured) is a different, wrong estimand.
    assert!((fraction(&hand["count_shortcut_fraction"]) - e.estimate).abs() > 0.02);
    assert!(e.interpretation.contains("represented mass") && e.applicability.contains("SYNTHETIC"));
}

#[test]
fn shared_cluster_strata_keep_their_covariance() {
    let fx = fixture("capture.json");
    let s = &fx["two_strata"];
    let mut all = parcels(&fx["one_stratum"]["parcels"]);
    all.extend(parcels(&s["extra_fine_parcels"]));
    let d = design(
        &s["stratum_mass_fractions"],
        &s["clusters"],
        IndependentUnit::CarrierCluster,
    );
    let e = estimate_capture(&d, &all).unwrap();
    let hand = &s["hand_derived"];
    assert!((e.estimate - fraction(&hand["estimate_fraction"])).abs() < 1e-15);
    assert!((e.standard_error.powi(2) - fraction(&hand["variance_fraction"])).abs() < 1e-15);
    // Summing per-stratum variances (independence across strata) gives a different answer.
    let independent: f64 = e
        .strata
        .iter()
        .map(|s| s.mass_fraction.powi(2) * s.variance.unwrap())
        .sum();
    assert!((independent - e.standard_error.powi(2)).abs() > 1e-3);
}

#[test]
fn splitting_a_parcel_into_daughters_with_the_same_fate_changes_nothing() {
    let fx = fixture("capture.json");
    let s = &fx["one_stratum"];
    let d = design(
        &s["stratum_mass_fractions"],
        &s["clusters"],
        IndependentUnit::CarrierCluster,
    );
    let original = parcels(&s["parcels"]);
    let mut split = original.clone();
    let a1 = split.remove(0);
    for (suffix, mass) in [("a", 0.75), ("b", 1.25)] {
        split.push(Parcel {
            parcel_id: format!("{}{suffix}", a1.parcel_id),
            represented_mass: mass,
            ..a1.clone()
        });
    }
    let before = estimate_capture(&d, &original).unwrap();
    let after = estimate_capture(&d, &split).unwrap();
    assert!((before.estimate - after.estimate).abs() < 1e-15);
    assert!((before.standard_error - after.standard_error).abs() < 1e-15);
}

#[test]
fn dependent_units_and_broken_ancestry_are_refused() {
    let fx = fixture("capture.json");
    let s = &fx["one_stratum"];
    let good = parcels(&s["parcels"]);
    for (unit, message) in [
        (
            IndependentUnit::Parcel,
            "parcels within a carrier realization are not independent units",
        ),
        (
            IndependentUnit::Daughter,
            "daughters sharing ancestry and carrier are not independent units",
        ),
        (
            IndependentUnit::QuadraturePoint,
            "points of one quasi-random set are not independent replicates",
        ),
    ] {
        let d = design(&s["stratum_mass_fractions"], &s["clusters"], unit);
        assert_eq!(
            estimate_capture(&d, &good),
            Err(StatsError::Unsupported(message))
        );
    }
    let d = design(
        &s["stratum_mass_fractions"],
        &s["clusters"],
        IndependentUnit::CarrierCluster,
    );
    let mut moved = good.clone();
    moved[3].cluster = "C".into(); // daughter B2 of root B-r1 placed in another realization
    assert_eq!(
        estimate_capture(&d, &moved),
        Err(StatsError::InvalidInput(
            "daughters must keep their root's stratum and independent cluster"
        ))
    );
    let mut duplicate = good.clone();
    duplicate[1].parcel_id = "A1".into();
    assert_eq!(
        estimate_capture(&d, &duplicate),
        Err(StatsError::InvalidInput("duplicate parcel identity"))
    );
    for mass in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut bad = good.clone();
        bad[0].represented_mass = mass;
        assert_eq!(
            estimate_capture(&d, &bad),
            Err(StatsError::InvalidInput(
                "represented mass must be finite and positive"
            ))
        );
    }
    let only_a: Vec<Parcel> = good.iter().filter(|p| p.cluster == "A").cloned().collect();
    let one_cluster = CaptureDesign {
        clusters: vec![Cluster {
            cluster_id: "A".into(),
            randomization_id: None,
        }],
        ..d.clone()
    };
    assert_eq!(
        estimate_capture(&one_cluster, &only_a),
        Err(StatsError::Unsupported(
            "each stratum needs represented mass in at least two independent units"
        ))
    );
    let mut empty_cluster = d.clone();
    empty_cluster.clusters.push(Cluster {
        cluster_id: "D".into(),
        randomization_id: None,
    });
    assert_eq!(
        estimate_capture(&empty_cluster, &good),
        Err(StatsError::InvalidInput(
            "declared cluster without represented mass"
        ))
    );
    let mut bad_weights = d.clone();
    bad_weights
        .stratum_mass_fractions
        .insert("coarse".into(), 0.9);
    assert_eq!(
        estimate_capture(&bad_weights, &good),
        Err(StatsError::InvalidInput(
            "stratum mass fractions must sum to one"
        ))
    );
    let mut randomized = d.clone();
    randomized.clusters[0].randomization_id = Some("shift-1".into());
    assert_eq!(
        estimate_capture(&randomized, &good),
        Err(StatsError::InvalidInput(
            "carrier cluster declares a quadrature randomization"
        ))
    );
    let mut pseudo = d.clone();
    pseudo.independent_unit = IndependentUnit::RandomizedQuadratureReplicate;
    for c in &mut pseudo.clusters {
        c.randomization_id = Some("one-shared-shift".into());
    }
    assert_eq!(
        estimate_capture(&pseudo, &good),
        Err(StatsError::Unsupported(
            "replicates sharing one randomization are dependent pseudo-replicates"
        ))
    );
}

/// Wilson score interval treating every daughter as an independent Bernoulli trial (the refused shortcut).
fn naive_wilson(p: f64, n: usize) -> (f64, f64) {
    let z = 1.959_963_984_540_054_f64;
    let n = n as f64;
    let centre = (p + z * z / (2.0 * n)) / (1.0 + z * z / n);
    let half = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt() / (1.0 + z * z / n);
    (centre - half, centre + half)
}

struct Cohort {
    design: CaptureDesign,
    parcels: Vec<Parcel>,
}

fn correlated_cohort(rng: &mut SplitMix64, spec: &serde_json::Value, trial: usize) -> Cohort {
    let clusters = spec["carrier_clusters"].as_u64().unwrap() as usize;
    let strata: BTreeMap<String, serde_json::Value> =
        serde_json::from_value(spec["strata"].clone()).unwrap();
    let sd = spec["cluster_logit_effect_sd"].as_f64().unwrap();
    let roots = spec["roots_per_stratum_per_cluster"].as_u64().unwrap() as usize;
    let mut out = Vec::new();
    for k in 0..clusters {
        let effect = sd * rng.normal();
        for (stratum, s) in &strata {
            let p = logistic(s["logit_mean"].as_f64().unwrap() + effect);
            for r in 0..roots {
                let mass = (0.75 * rng.normal()).exp();
                let daughters = 1 + rng.below(3) as usize;
                let shares: Vec<f64> = (0..daughters).map(|_| 0.1 + 0.9 * rng.uniform()).collect();
                let total: f64 = shares.iter().sum();
                for (j, share) in shares.iter().enumerate() {
                    out.push(Parcel {
                        parcel_id: format!("t{trial}-k{k}-{stratum}-r{r}-d{j}"),
                        root_id: format!("t{trial}-k{k}-{stratum}-r{r}"),
                        stratum: stratum.clone(),
                        cluster: format!("k{k}"),
                        represented_mass: mass * share / total,
                        captured: rng.uniform() < p,
                    });
                }
            }
        }
    }
    Cohort {
        design: CaptureDesign {
            independent_unit: IndependentUnit::CarrierCluster,
            stratum_mass_fractions: strata
                .iter()
                .map(|(k, s)| (k.clone(), s["mass_fraction"].as_f64().unwrap()))
                .collect(),
            clusters: (0..clusters)
                .map(|k| Cluster { cluster_id: format!("k{k}"), randomization_id: None })
                .collect(),
            confidence: 0.95,
            applicability: "SYNTHETIC: independent carrier clusters, cluster-shared logit effect, mass independent of fate".into(),
        },
        parcels: out,
    }
}

fn cohort_truth(spec: &serde_json::Value) -> f64 {
    let strata: BTreeMap<String, serde_json::Value> =
        serde_json::from_value(spec["strata"].clone()).unwrap();
    let sd = spec["cluster_logit_effect_sd"].as_f64().unwrap();
    let points = 4001;
    strata
        .values()
        .map(|s| {
            let mean = s["logit_mean"].as_f64().unwrap();
            let (lo, hi) = (-8.0 * sd, 8.0 * sd);
            let h = (hi - lo) / (points - 1) as f64;
            let mut integral = 0.0;
            for i in 0..points {
                let b = lo + i as f64 * h;
                let density =
                    (-(b * b) / (2.0 * sd * sd)).exp() / (sd * (2.0 * std::f64::consts::PI).sqrt());
                let weight = if i == 0 || i == points - 1 { 0.5 } else { 1.0 };
                integral += weight * logistic(mean + b) * density * h;
            }
            s["mass_fraction"].as_f64().unwrap() * integral
        })
        .sum()
}

#[test]
fn naive_wilson_over_daughters_is_narrower_than_the_cluster_interval_and_is_refused() {
    let fx = fixture("capture.json");
    let spec = &fx["preregistered_coverage"]["correlated_cohort"];
    let mut rng = SplitMix64(1);
    let cohort = correlated_cohort(&mut rng, spec, 0);
    let valid = estimate_capture(&cohort.design, &cohort.parcels).unwrap();
    let (lo, hi) = naive_wilson(valid.estimate, cohort.parcels.len());
    assert!(
        hi - lo < valid.upper - valid.lower,
        "naive interval should be narrower here"
    );
    let mut naive = cohort.design.clone();
    naive.independent_unit = IndependentUnit::Daughter;
    assert!(matches!(
        estimate_capture(&naive, &cohort.parcels),
        Err(StatsError::Unsupported(_))
    ));
}

#[test]
fn preregistered_coverage_correlated_unequal_weight_cohort() {
    let fx = fixture("capture.json");
    let reg = &fx["preregistered_coverage"];
    let spec = &reg["correlated_cohort"];
    let truth = cohort_truth(spec);
    let trials = spec["trials"].as_u64().unwrap() as usize;
    let mut rng = SplitMix64(spec["seed"].as_u64().unwrap());
    let (mut covered, mut naive_covered) = (0usize, 0usize);
    for t in 0..trials {
        let cohort = correlated_cohort(&mut rng, spec, t);
        let e = estimate_capture(&cohort.design, &cohort.parcels).unwrap();
        covered += usize::from(e.lower <= truth && truth <= e.upper);
        let (lo, hi) = naive_wilson(e.estimate, cohort.parcels.len());
        naive_covered += usize::from(lo <= truth && truth <= hi);
    }
    let coverage = covered as f64 / trials as f64;
    let naive = naive_covered as f64 / trials as f64;
    println!(
        "C007-COVERAGE correlated_cohort truth={truth:.6} trials={trials} coverage={coverage:.4} mc_se={:.4} naive_daughter_wilson_coverage={naive:.4} mc_se={:.4}",
        monte_carlo_se(coverage, trials),
        monte_carlo_se(naive, trials)
    );
    let a = &spec["acceptance"];
    assert!(
        coverage >= a["coverage_min"].as_f64().unwrap()
            && coverage <= a["coverage_max"].as_f64().unwrap()
    );
    assert!(naive < 0.85);
}

#[test]
fn preregistered_coverage_randomized_lattice_replicates() {
    let fx = fixture("capture.json");
    let spec = &fx["preregistered_coverage"]["randomized_lattice"];
    let truth = 1.0 - (0.5 * 0.37 + 0.37 * 0.37 / 2.0);
    let trials = spec["trials"].as_u64().unwrap() as usize;
    let replicates = spec["replicates"].as_u64().unwrap() as usize;
    let points = spec["points_per_replicate"].as_u64().unwrap() as usize;
    let mut rng = SplitMix64(spec["seed"].as_u64().unwrap());
    let mut covered = 0usize;
    for t in 0..trials {
        let mut parcels = Vec::new();
        let mut clusters = Vec::new();
        for r in 0..replicates {
            let shift = rng.uniform();
            clusters.push(Cluster {
                cluster_id: format!("t{t}-r{r}"),
                randomization_id: Some(format!("t{t}-shift{r}")),
            });
            for j in 0..points {
                let u = (j as f64 / points as f64 + shift).fract();
                parcels.push(Parcel {
                    parcel_id: format!("t{t}-r{r}-p{j}"),
                    root_id: format!("t{t}-r{r}-p{j}"),
                    stratum: "all".into(),
                    cluster: format!("t{t}-r{r}"),
                    represented_mass: 0.5 + u,
                    captured: u > 0.37,
                });
            }
        }
        let design = CaptureDesign {
            independent_unit: IndependentUnit::RandomizedQuadratureReplicate,
            stratum_mass_fractions: BTreeMap::from([("all".to_string(), 1.0)]),
            clusters,
            confidence: 0.95,
            applicability: "SYNTHETIC: independently shifted rank-1 lattice replicates".into(),
        };
        let e = estimate_capture(&design, &parcels).unwrap();
        covered += usize::from(e.lower <= truth && truth <= e.upper);
    }
    let coverage = covered as f64 / trials as f64;
    println!(
        "C007-COVERAGE randomized_lattice truth={truth:.6} trials={trials} coverage={coverage:.4} mc_se={:.4}",
        monte_carlo_se(coverage, trials)
    );
    let a = &spec["acceptance"];
    assert!(
        coverage >= a["coverage_min"].as_f64().unwrap()
            && coverage <= a["coverage_max"].as_f64().unwrap()
    );
}
