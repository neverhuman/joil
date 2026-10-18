//! V5-04-04: multiplicity, ranks, tails and censoring on SYNTHETIC fixtures.
mod common;
use common::{f64s, fixture};
use crucible_stats::analysis::order::*;
use crucible_stats::StatsError;

fn fraction(v: &serde_json::Value) -> f64 {
    v[0].as_f64().unwrap() / v[1].as_f64().unwrap()
}

fn iid(values: &[f64]) -> Vec<Draw> {
    values
        .iter()
        .enumerate()
        .map(|(i, v)| Draw {
            draw_id: format!("draw-{i}"),
            stream: format!("stream-{i}"),
            observation: Observation::Measured { value: *v },
        })
        .collect()
}

#[test]
fn holm_matches_hand_adjustment_with_ties_and_is_order_invariant() {
    let fx = fixture("order.json");
    for case in fx["holm"].as_array().unwrap() {
        let p = f64s(&case["p"]);
        let adjusted = holm_adjusted(&p).unwrap();
        for (a, e) in adjusted.iter().zip(f64s(&case["adjusted"])) {
            assert!((a - e).abs() < 1e-15, "{case}");
        }
        let mut reversed = p.clone();
        reversed.reverse();
        let mut back = holm_adjusted(&reversed).unwrap();
        back.reverse();
        assert_eq!(back, adjusted);
    }
    let ties = &fx["holm"][1];
    let p = f64s(&ties["p"]);
    let expect = |key: &str| -> Vec<bool> {
        ties[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_bool().unwrap())
            .collect()
    };
    assert_eq!(holm_rejections(&p, 0.05).unwrap(), expect("reject_at_0_05"));
    assert_eq!(holm_rejections(&p, 0.10).unwrap(), expect("reject_at_0_10"));
    for bad in [vec![], vec![f64::NAN], vec![1.5], vec![-0.1]] {
        assert!(holm_adjusted(&bad).is_err());
    }
    assert!(holm_rejections(&p, 0.0).is_err());
}

#[test]
fn cliff_counts_ties_as_zero_and_is_permutation_and_swap_consistent() {
    let fx = fixture("order.json");
    for case in fx["cliff"].as_array().unwrap() {
        let (a, b) = (f64s(&case["a"]), f64s(&case["b"]));
        let d = cliffs_delta(&a, &b).unwrap();
        assert!(
            (d - fraction(&case["delta_fraction"])).abs() < 1e-15,
            "{case}"
        );
        let (mut ra, mut rb) = (a.clone(), b.clone());
        ra.reverse();
        rb.rotate_left(1);
        assert_eq!(cliffs_delta(&ra, &rb).unwrap(), d);
        assert_eq!(cliffs_delta(&b, &a).unwrap(), -d);
    }
    let p = &fx["cliff_permutation"];
    let test = cliffs_delta_permutation(&f64s(&p["a"]), &f64s(&p["b"])).unwrap();
    assert!((test.delta - fraction(&p["delta_fraction"])).abs() < 1e-15);
    assert_eq!(test.relabellings, 20);
    assert!((test.p_greater - fraction(&p["p_greater_fraction"])).abs() < 1e-15);
    assert!(test.interpretation.contains("exchangeability"));
    assert!(cliffs_delta(&[], &[1.0]).is_err());
    assert!(cliffs_delta(&[f64::NAN], &[1.0]).is_err());
    assert!(cliffs_delta_permutation(&[1.0; 11], &[2.0; 10]).is_err());
}

#[test]
fn exact_order_statistic_ranks_and_tail_thresholds_match_independent_derivation() {
    let fx = fixture("order.json");
    let r = &fx["order_statistic_ranks"];
    for case in r["cases"].as_array().unwrap() {
        let n = case["n"].as_u64().unwrap() as usize;
        let q = case["quantile"].as_f64().unwrap();
        let c = case["confidence"].as_f64().unwrap();
        let ranks = order_statistic_ranks(n, q, c).unwrap();
        match case["upper_rank"].as_u64() {
            Some(upper) => {
                let (l, u, coverage) = ranks.unwrap();
                assert_eq!(
                    (l, u),
                    (
                        case["lower_rank"].as_u64().unwrap() as usize,
                        upper as usize
                    ),
                    "{case}"
                );
                assert!((coverage - case["exact_coverage"].as_f64().unwrap()).abs() < 1e-12);
            }
            None => assert!(ranks.is_none(), "{case}"),
        }
    }
    assert_eq!(
        minimum_iid_draws(0.95, 0.95).unwrap() as u64,
        r["minimal_n"]["p95_conf95"].as_u64().unwrap()
    );
    assert_eq!(
        minimum_iid_draws(0.99, 0.95).unwrap() as u64,
        r["minimal_n"]["p99_conf95"].as_u64().unwrap()
    );
    // n = 72 IID draws give an interval; 71 do not.
    let values: Vec<f64> = (0..72).map(|i| i as f64).collect();
    assert!(matches!(
        quantile_interval(&iid(&values), Sampling::Iid, 0.95, 0.95).unwrap(),
        TailOutcome::Interval(_)
    ));
    assert!(matches!(
        quantile_interval(&iid(&values[..71]), Sampling::Iid, 0.95, 0.95).unwrap(),
        TailOutcome::Insufficient {
            n: 71,
            minimum_n: 72,
            ..
        }
    ));
}

#[test]
fn sample_count_alone_never_buys_a_tail_interval() {
    // 72 values satisfy the count threshold, but only declared IID draws on distinct streams qualify.
    let values: Vec<f64> = (0..72).map(|i| i as f64).collect();
    for (sampling, message) in [
        (
            Sampling::QuasiRandom,
            "quasi-random points are not IID order-statistic samples",
        ),
        (
            Sampling::Correlated,
            "correlated draws cannot support an IID tail interval",
        ),
        (
            Sampling::IndependentBlocks,
            "order-statistic tail intervals need IID draws, not block summaries",
        ),
        (
            Sampling::FiniteEnvelope,
            "a finite scenario envelope is not a probability distribution",
        ),
    ] {
        assert_eq!(
            quantile_interval(&iid(&values), sampling, 0.95, 0.95),
            Err(StatsError::Unsupported(message))
        );
    }
    let mut shared = iid(&values);
    shared[10].stream = shared[9].stream.clone();
    assert_eq!(
        quantile_interval(&shared, Sampling::Iid, 0.95, 0.95),
        Err(StatsError::Unsupported(
            "draws sharing a random stream are not independent"
        ))
    );
}

#[test]
fn censored_values_bound_the_tail_only_as_at_most_loq_never_as_zero() {
    let fx = fixture("order.json");
    let c = &fx["censoring"];
    let loq = c["loq"].as_f64().unwrap();
    let draws = |censored: usize| -> Vec<Draw> {
        (0..72)
            .map(|i| Draw {
                draw_id: format!("d{i}"),
                stream: format!("s{i}"),
                observation: if i < censored {
                    Observation::BelowLoq { loq }
                } else {
                    Observation::Measured {
                        value: loq + (i - censored) as f64,
                    }
                },
            })
            .collect()
    };
    let interval =
        |censored| match quantile_interval(&draws(censored), Sampling::Iid, 0.95, 0.95).unwrap() {
            TailOutcome::Interval(i) => i,
            other => panic!("{other:?}"),
        };
    // 62 censored: both p95 ranks (64, 72) are measured.
    let a = interval(62);
    assert!(
        matches!(a.lower, Endpoint::Measured { .. })
            && matches!(a.upper, Endpoint::Measured { .. })
    );
    // 66 censored: the lower rank falls in the censored range and is reported only as <= LOQ.
    let b = interval(66);
    assert_eq!(b.lower, Endpoint::AtMostLoq { loq });
    assert!(matches!(b.upper, Endpoint::Measured { .. }));
    // All censored: the p95 upper bound is "at most LOQ"; imputing zeros would manufacture 0.0.
    let all = interval(72);
    assert_eq!(all.upper, Endpoint::AtMostLoq { loq });
    assert_eq!(all.censored_observations, 72);
    let zeros = match quantile_interval(&iid(&[0.0; 72]), Sampling::Iid, 0.95, 0.95).unwrap() {
        TailOutcome::Interval(i) => i,
        other => panic!("{other:?}"),
    };
    assert_eq!(zeros.upper, Endpoint::Measured { value: 0.0 });
    assert_ne!(all.upper, zeros.upper);
    assert!(all.interpretation.contains("never as zero") && all.interpretation.contains("IID"));
    // A measured value below the LOQ interleaves the censored range: ranks are not identified.
    let mut mixed = draws(30);
    mixed[40].observation = Observation::Measured { value: 0.5 };
    assert_eq!(
        quantile_interval(&mixed, Sampling::Iid, 0.95, 0.95),
        Err(StatsError::Unsupported(
            "censored observations interleave measured values; order statistics are not identified"
        ))
    );
    let mut bad = draws(1);
    bad[0].observation = Observation::BelowLoq { loq: 0.0 };
    assert!(quantile_interval(&bad, Sampling::Iid, 0.95, 0.95).is_err());
}

#[test]
fn finite_envelopes_stay_finite_envelopes() {
    let env =
        finite_envelope(&[("D1".into(), 3.0), ("D2".into(), 7.5), ("D3".into(), 5.0)]).unwrap();
    assert_eq!((env.scenarios, env.minimum, env.maximum), (3, 3.0, 7.5));
    assert!(env
        .interpretation
        .contains("not a probability, quantile or reliability"));
    assert!(finite_envelope(&[("D1".into(), 1.0), ("D1".into(), 2.0)]).is_err());
    assert!(finite_envelope(&[("D1".into(), f64::INFINITY)]).is_err());
    assert!(finite_envelope(&[]).is_err());
}
