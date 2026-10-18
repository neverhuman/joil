//! V5-18 acceptance on SYNTHETIC fixtures: concentration oracles, common-flow maps, treated-volume
//! energy, bounded objectives and decision profiles. Synthetic admitted values only exercise the
//! decision logic; nothing here is a physical capacity, energy or qualification result.
use pack_hydrocyclone_judges::capacity::*;
use pack_hydrocyclone_judges::metrics::*;
use pack_hydrocyclone_judges::objectives::*;
use pack_hydrocyclone_judges::{Field, Sha256Hex, Verdict};
use std::collections::BTreeMap;

fn sha(label: &str) -> Sha256Hex {
    Sha256Hex::of_bytes(format!("synthetic-source:{label}").as_bytes())
}
fn adm<T>(value: T, label: &str) -> Field<T> {
    Field::Admitted {
        value,
        source: sha(label),
    }
}
fn unresolved<T>() -> Field<T> {
    Field::Unresolved {
        reason: "synthetic unresolved".into(),
        owner: "synthetic-owner".into(),
        blocks: vec!["physical_candidate".into()],
    }
}
fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

// ---------------------------------------------------------------- V5-18-01 concentration oracles

#[test]
fn oracle_500_mg_l_at_94_percent_fails_30_mg_l() {
    let cu = outlet_concentration_mg_l(500.0, 0.94, 0.02).unwrap();
    assert!(close(cu, 30.6122449, 1e-7), "got {cu}");
    assert!(cu > 30.0, "Cu must fail the 30 mg/L research limit");
}

#[test]
fn oracle_95_percent_and_required_capture_for_limits() {
    let cu = outlet_concentration_mg_l(500.0, 0.95, 0.02).unwrap();
    assert!(close(cu, 25.5102041, 1e-7), "got {cu}");
    assert!(close(
        required_capture_for_limit(500.0, 0.02, 30.0).unwrap(),
        0.9412,
        1e-12
    ));
    assert!(close(
        required_capture_for_limit(500.0, 0.02, 20.0).unwrap(),
        0.9608,
        1e-12
    ));
}

#[test]
fn reduced_efficiency_path_equals_raw_capture_path() {
    let raw = outlet_concentration_mg_l(500.0, 0.94, 0.02).unwrap();
    let reduced = reduced_efficiency(0.94, 0.02).unwrap();
    let via_reduced = outlet_concentration_from_reduced_mg_l(500.0, reduced).unwrap();
    assert!(close(raw, via_reduced, 1e-9));
}

#[test]
fn negative_control_missing_split_denominator_is_caught_by_oracle() {
    // Without (1 - r) the same inputs give exactly 30.0, which would pass a <= 30 limit.
    let without_denominator = 500.0 * (1.0 - 0.94);
    let cu = outlet_concentration_mg_l(500.0, 0.94, 0.02).unwrap();
    assert!(close(without_denominator, 30.0, 1e-9));
    assert!(cu > without_denominator, "split denominator must raise Cu");
}

#[test]
fn negative_control_negative_reduced_efficiency_is_not_clipped() {
    // E = 0.01 < r = 0.02: oil leaves in the treated stream, so Cu must exceed Ci.
    let reduced = reduced_efficiency(0.01, 0.02).unwrap();
    assert!(
        reduced < 0.0,
        "reduced efficiency must stay negative, got {reduced}"
    );
    assert!(close(reduced, -0.010204081632653, 1e-12));
    let cu = outlet_concentration_mg_l(500.0, 0.01, 0.02).unwrap();
    assert!(
        cu > 500.0,
        "clipping to zero would give exactly Ci; got {cu}"
    );
}

#[test]
fn inventory_bounds_assign_unresolved_mass_adversely() {
    let closed = OilInventory {
        injected_kg: 1.0,
        reject_kg: 0.94,
        treated_kg: 0.02,
        in_domain_kg: 0.04,
    };
    let b = closed.bounds(500.0, 0.02).unwrap();
    assert!(close(b.capture_lower, 0.94, 1e-12) && close(b.capture_upper, 0.98, 1e-12));
    assert!(b.cu_upper_mg_l > b.cu_lower_mg_l);
    assert!(close(b.cu_upper_mg_l, 30.6122449, 1e-7));

    // Unclosed inventory: 0.06 kg unexplained. The lower capture discounts it.
    let open = OilInventory {
        injected_kg: 1.0,
        reject_kg: 0.94,
        treated_kg: 0.0,
        in_domain_kg: 0.0,
    };
    let o = open.bounds(500.0, 0.02).unwrap();
    assert!(
        close(o.capture_lower, 0.88, 1e-12),
        "got {}",
        o.capture_lower
    );
    assert!(close(o.capture_upper, 1.0, 1e-12));
    assert!(close(o.closure_fraction, 0.06, 1e-12));
    assert!(
        o.cu_upper_mg_l > 30.0,
        "adverse bound must fail the research limit"
    );
}

#[test]
fn transient_concentration_uses_one_synchronised_window() {
    let w = SynchronizedWindow {
        start_s: 10.0,
        end_s: 20.0,
        treated_oil_kg: 0.0003,
        treated_water_m3: 0.5,
    };
    assert!(close(
        transient_outlet_concentration_mg_l(&w).unwrap(),
        0.6,
        1e-12
    ));
    let inverted = SynchronizedWindow {
        end_s: 10.0,
        ..w.clone()
    };
    assert!(transient_outlet_concentration_mg_l(&inverted).is_err());
}

#[test]
fn cohort_concentration_needs_oil_basis_and_normalised_bins() {
    let bins = [
        DsdBin {
            lower_um: 3.0,
            upper_um: 20.0,
            weight: 0.5,
            capture: 0.9,
        },
        DsdBin {
            lower_um: 20.0,
            upper_um: 100.0,
            weight: 0.5,
            capture: 0.98,
        },
    ];
    let cu = cohort_outlet_concentration_mg_l(500.0, 0.02, DsdBasis::OilMass, &bins).unwrap();
    assert!(close(cu, 30.6122449, 1e-6), "got {cu}");
    assert!(cohort_outlet_concentration_mg_l(500.0, 0.02, DsdBasis::Number, &bins).is_err());
    let unnormalised = [
        DsdBin {
            weight: 0.4,
            ..bins[0]
        },
        bins[1],
    ];
    assert!(
        cohort_outlet_concentration_mg_l(500.0, 0.02, DsdBasis::OilVolume, &unnormalised).is_err()
    );
}

#[test]
fn negative_control_daughter_double_count_is_refused() {
    // Parent 1.0 kg; a daughter counted both as its own outlet and as extra injected mass
    // makes the outlet states sum to more than the parent.
    let double_count = TransferMatrix {
        parent_kg: vec![1.0],
        rows: vec![TransferRow {
            reject_kg: 0.6,
            treated_kg: 0.5,
            in_domain_kg: 0.0,
        }],
    };
    assert!(double_count.inventory().is_err());

    let conserving = TransferMatrix {
        parent_kg: vec![0.7, 0.3],
        rows: vec![
            TransferRow {
                reject_kg: 0.6,
                treated_kg: 0.05,
                in_domain_kg: 0.05,
            },
            TransferRow {
                reject_kg: 0.2,
                treated_kg: 0.1,
                in_domain_kg: 0.0,
            },
        ],
    };
    let inv = conserving.inventory().unwrap();
    assert!(close(inv.injected_kg, 1.0, 1e-12));
    assert!(close(inv.reject_kg, 0.8, 1e-12));
}

// ---------------------------------------------------------------- V5-18-03 treated-volume energy

/// Hand-computed control volume (gauge pressures, rho = 1000, g = 9.81, alpha = 1):
/// inlet flux 0.01 * (200000 + 50000 + 9810) = 2598.10 W;
/// overflow flux 0.0002 * (50000 + 2000 + 4905) = 11.381 W;
/// underflow flux 0.0098 * (20000 + 48020) = 666.596 W;
/// loss = 1920.123 W, divided by the treated flow 0.0098 m³/s.
fn fixture() -> (ControlVolume, FrozenDuty) {
    let port = |q, area, p, z| Port {
        q_m3_s: q,
        area_m2: area,
        static_gauge_pa: p,
        z_m: z,
        alpha: 1.0,
    };
    let cv = ControlVolume {
        density_kg_m3: 1000.0,
        gravity_m_s2: 9.81,
        inlet: port(0.01, 0.001, 200000.0, 1.0),
        overflow: port(0.0002, 0.0001, 50000.0, 0.5),
        underflow: port(0.0098, 0.001, 20000.0, 0.0),
    };
    let duty = FrozenDuty {
        id: "synthetic-common-duty".into(),
        feed_m3_h: 36.0,
        split: 0.02,
    };
    (cv, duty)
}

#[test]
fn energy_per_treated_m3_from_integrated_port_fluxes() {
    let (cv, duty) = fixture();
    let claim = energy_flux_loss_per_treated_m3(&cv, &duty).unwrap();
    assert_eq!(claim.basis, EnergyBasis::TreatedVolumeFluxLoss);
    assert_eq!(claim.duty_id, "synthetic-common-duty");
    let per_treated = admit_treated_energy(&claim).unwrap();
    assert!(
        close(per_treated, 1920.123 / 0.0098, 1e-6),
        "got {per_treated}"
    );
    // The per-feed quantity is a different number and must not be the reported one.
    assert!(!close(per_treated, 1920.123 / 0.01, 1.0));
}

#[test]
fn negative_control_per_feed_energy_mislabelled_as_treated_is_rejected() {
    let (cv, duty) = fixture();
    let per_feed = 1920.123 / cv.inlet.q_m3_s;
    let mislabelled = EnergyClaim {
        basis: EnergyBasis::TreatedVolumeFluxLoss,
        duty_id: duty.id.clone(),
        value_j_per_m3: per_feed,
    };
    // Labelling is the only route to admission; a historical per-feed label is refused.
    let historical = historical_static_pressure_work_per_feed_m3(per_feed, &duty.id);
    assert_eq!(
        historical.basis,
        EnergyBasis::StaticPressureWorkPerFeedM3Historical
    );
    assert!(admit_treated_energy(&historical).is_err());
    assert!(admit_treated_energy(&mislabelled).is_ok());
    assert!(energy_flux_loss_per_treated_m3(
        &cv,
        &FrozenDuty {
            split: 0.05,
            ..duty
        }
    )
    .is_err());
}

#[test]
fn energy_rejects_unbalanced_ports_and_unphysical_negative_loss() {
    let (mut cv, duty) = fixture();
    cv.underflow.q_m3_s = 0.0099;
    assert!(energy_flux_loss_per_treated_m3(&cv, &duty).is_err());

    let (mut cv, duty) = fixture();
    cv.underflow.static_gauge_pa = 300000.0;
    assert!(energy_flux_loss_per_treated_m3(&cv, &duty).is_err());
}

// ---------------------------------------------------------------- V5-18-02 common-flow maps

const SCENARIOS: [&str; 3] = ["D1", "D2", "D3"];

fn pass(r: f64) -> ScenarioOutcome {
    ScenarioOutcome::Pass {
        reject_fraction_upper: r,
    }
}
fn fail() -> ScenarioOutcome {
    ScenarioOutcome::Fail {
        reasons: vec!["synthetic_fail".into()],
    }
}
fn point(feed: f64, outcomes: &[(&str, ScenarioOutcome)]) -> FeedPoint {
    FeedPoint {
        feed_m3_h: feed,
        outcomes: outcomes
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect::<BTreeMap<_, _>>(),
    }
}
fn all_pass(feed: f64) -> FeedPoint {
    point(
        feed,
        &[("D1", pass(0.02)), ("D2", pass(0.03)), ("D3", pass(0.02))],
    )
}
fn map(points: Vec<FeedPoint>) -> OperatingMap {
    OperatingMap {
        required: SCENARIOS.iter().map(|s| s.to_string()).collect(),
        points,
    }
}
fn regular(max_spacing: f64) -> Field<Regularity> {
    adm(
        Regularity {
            max_spacing_m3_h: max_spacing,
        },
        "regularity",
    )
}

#[test]
fn common_capacity_is_min_over_scenarios_at_the_same_feed() {
    let m = map(vec![all_pass(2.6), all_pass(3.0)]);
    let r = evaluate_common_map(&m, &unresolved()).unwrap();
    let s = r.sampled.unwrap();
    assert!(close(s.feed_m3_h, 3.0, 1e-12));
    assert!(close(s.treated_lower_m3_h, 3.0 * 0.97, 1e-12));
    assert_eq!(
        r.band,
        BandClaim::SampledOnly {
            reason: "interpolation regularity not admitted".into()
        }
    );
}

#[test]
fn admitted_regular_band_is_claimed_only_without_gaps() {
    let m = map(vec![all_pass(2.6), all_pass(3.0), all_pass(3.4)]);
    let r = evaluate_common_map(&m, &regular(0.5)).unwrap();
    assert_eq!(
        r.band,
        BandClaim::Band {
            lower_feed_m3_h: 2.6,
            upper_feed_m3_h: 3.4
        }
    );
    let too_sparse = evaluate_common_map(&m, &regular(0.3)).unwrap();
    assert!(matches!(too_sparse.band, BandClaim::SampledOnly { .. }));
}

#[test]
fn disjoint_feasible_scenario_maps_produce_no_common_duty() {
    // D1 feasible only at low feed, D2 only at high feed. Each scenario alone has a maximum,
    // but no common point exists.
    let m = map(vec![
        point(
            2.6,
            &[("D1", pass(0.02)), ("D2", fail()), ("D3", pass(0.02))],
        ),
        point(
            3.0,
            &[("D1", pass(0.02)), ("D2", fail()), ("D3", pass(0.02))],
        ),
        point(
            5.4,
            &[("D1", fail()), ("D2", pass(0.03)), ("D3", pass(0.02))],
        ),
    ]);
    let r = evaluate_common_map(&m, &regular(10.0)).unwrap();
    assert!(
        r.sampled.is_none(),
        "no false common duty from disjoint maps"
    );
    assert!(matches!(r.band, BandClaim::SampledOnly { .. }));
    assert_eq!(r.failures_m3_h, vec![2.6, 3.0, 5.4]);
}

#[test]
fn negative_control_crossing_a_failure_island_is_not_a_band() {
    let m = map(vec![
        all_pass(2.6),
        point(
            3.0,
            &[("D1", pass(0.02)), ("D2", fail()), ("D3", pass(0.02))],
        ),
        all_pass(3.4),
        all_pass(4.0),
    ]);
    let r = evaluate_common_map(&m, &regular(0.8)).unwrap();
    assert_eq!(
        r.runs.len(),
        2,
        "failure splits the feasible set into two runs"
    );
    assert!(matches!(r.band, BandClaim::SampledOnly { .. }));
    assert_eq!(r.sampled.unwrap().feed_m3_h, 4.0);
    assert_eq!(r.brackets.len(), 2, "both transitions are bracketed");
    assert_eq!(r.brackets[0].below_feed_m3_h, 2.6);
    assert_eq!(r.brackets[0].above_feed_m3_h, 3.0);
}

#[test]
fn missing_scenario_is_a_gap_never_a_pass() {
    let m = map(vec![
        all_pass(2.6),
        point(3.0, &[("D1", pass(0.02)), ("D2", pass(0.03))]),
    ]);
    let r = evaluate_common_map(&m, &regular(10.0)).unwrap();
    assert_eq!(r.gaps_m3_h, vec![3.0]);
    assert_eq!(r.sampled.unwrap().feed_m3_h, 2.6);
    assert!(evaluate_common_map(
        &map(vec![point(3.0, &[("D9", pass(0.02))])]),
        &regular(10.0)
    )
    .is_err());
}

// ---------------------------------------------------------------- V5-18-04 bounded objectives

fn baseline() -> Baseline {
    Baseline {
        id: "synthetic-baseline".into(),
        reserve_lower: adm(1.5, "b-reserve"),
        treated_capacity_lower_m3_h: adm(3.0, "b-capacity"),
        energy_upper_j_per_m3: adm(150000.0, "b-energy"),
    }
}

fn all_gates(v: Verdict) -> BTreeMap<String, Verdict> {
    REQUIRED_GATES
        .iter()
        .map(|g| (g.to_string(), v.clone()))
        .collect()
}

fn record(id: &str, mass: f64, reserve: f64, capacity: f64, energy: f64) -> CandidateRecord {
    CandidateRecord {
        id: id.into(),
        objectives: Objectives {
            printed_mass_kg_upper: Some(mass),
            reserve_lower: Some(reserve),
            treated_capacity_lower_m3_h: Some(capacity),
            energy_upper_j_per_m3: Some(energy),
        },
        gates: all_gates(Verdict::Pass),
    }
}

fn scale(reservation: f64, aspiration: f64) -> Scale {
    Scale {
        reservation,
        aspiration,
    }
}

fn profile(weights: [f64; 4]) -> DecisionProfile {
    DecisionProfile {
        id: "synthetic-sensitivity".into(),
        status: ProfileStatus::Proposed,
        weights: Weights {
            printed_mass: weights[0],
            reserve: weights[1],
            capacity: weights[2],
            energy: weights[3],
        },
        scales: Scales {
            printed_mass: scale(20.0, 5.0),
            reserve: scale(1.5, 2.5),
            capacity: scale(3.0, 5.0),
            energy: scale(200000.0, 100000.0),
        },
        multiplicity: 4,
        alpha: 0.05,
    }
}

const PROPOSED: [f64; 4] = [0.35, 0.20, 0.30, 0.15];

#[test]
fn feasible_candidate_has_expected_utility_and_wins() {
    let a = record("A", 10.0, 1.8, 3.4, 120000.0);
    let d = decide(&profile(PROPOSED), &baseline(), &[a]).unwrap();
    match &d.states["A"] {
        CandidateState::Feasible { utility } => {
            // 0.35*(10/15) + 0.20*(0.3) + 0.30*(0.2) + 0.15*(0.8)
            assert!(close(*utility, 0.4733333333, 1e-9), "got {utility}");
        }
        other => panic!("expected feasible, got {other:?}"),
    }
    assert_eq!(d.winner.as_deref(), Some("A"));
    assert_eq!(d.profile_status, ProfileStatus::Proposed);
}

#[test]
fn negative_control_missing_buckling_gate_cannot_be_compensated() {
    // B is much lighter and has a higher utility, but its buckling verdict is absent.
    let a = record("A", 10.0, 1.8, 3.4, 120000.0);
    let mut b = record("B", 5.0, 2.4, 4.9, 100000.0);
    b.gates.remove("buckling");
    let d = decide(&profile(PROPOSED), &baseline(), &[a, b]).unwrap();
    assert!(matches!(d.states["B"], CandidateState::Unresolved { .. }));
    assert_eq!(d.winner.as_deref(), Some("A"));

    let mut c = record("C", 5.0, 2.4, 4.9, 100000.0);
    c.gates
        .insert("material".into(), Verdict::Fail(vec!["synthetic".into()]));
    let d = decide(&profile(PROPOSED), &baseline(), &[c]).unwrap();
    assert!(matches!(d.states["C"], CandidateState::Infeasible { .. }));
    assert_eq!(d.winner, None);
}

#[test]
fn baseline_floors_precede_preference() {
    // D is lighter, but its capacity regresses below the baseline floor.
    let d = record("D", 4.0, 2.4, 2.9, 100000.0);
    let out = decide(&profile(PROPOSED), &baseline(), &[d]).unwrap();
    match &out.states["D"] {
        CandidateState::Infeasible { reasons } => {
            assert!(reasons.iter().any(|r| r.contains("capacity")));
        }
        other => panic!("expected floor failure, got {other:?}"),
    }
}

#[test]
fn unresolved_objective_is_not_a_pass() {
    let mut e = record("E", 4.0, 2.4, 4.9, 100000.0);
    e.objectives.energy_upper_j_per_m3 = None;
    let out = decide(&profile(PROPOSED), &baseline(), &[e]).unwrap();
    assert!(matches!(out.states["E"], CandidateState::Unresolved { .. }));
    assert_eq!(out.winner, None);
}

// ---------------------------------------------------------------- V5-18-05 ratios and policy

#[test]
fn changed_weights_create_a_new_decision_over_the_same_observations() {
    let cands = [
        record("A", 10.0, 1.8, 3.4, 120000.0),
        record("F", 8.0, 2.0, 4.0, 130000.0),
    ];
    let p1 = decide(&profile(PROPOSED), &baseline(), &cands).unwrap();
    let p2 = decide(&profile([0.25, 0.25, 0.25, 0.25]), &baseline(), &cands).unwrap();
    assert_eq!(p1.observations_sha256, p2.observations_sha256);
    assert_ne!(p1.profile_sha256, p2.profile_sha256);
    assert_ne!(p1.decision_sha256, p2.decision_sha256);
}

#[test]
fn undefined_baseline_capacity_cannot_win() {
    let mut base = baseline();
    base.treated_capacity_lower_m3_h = unresolved();
    let a = record("A", 10.0, 1.8, 3.4, 120000.0);
    let d = decide(&profile(PROPOSED), &base, &[a]).unwrap();
    assert!(matches!(d.states["A"], CandidateState::Unresolved { .. }));
    assert_eq!(d.winner, None);
    assert!(gain_over_baseline(3.4, &base.treated_capacity_lower_m3_h).is_err());
}

#[test]
fn negative_control_equal_aspiration_and_reservation_has_no_finite_utility() {
    let mut p = profile(PROPOSED);
    p.scales.capacity = scale(3.0, 3.0);
    let a = record("A", 10.0, 1.8, 3.4, 120000.0);
    assert!(decide(&p, &baseline(), &[a]).is_err());
}

#[test]
fn gain_requires_a_positive_admitted_baseline_and_no_epsilon() {
    assert!(close(
        gain_over_baseline(3.3, &adm(3.0, "cap")).unwrap(),
        0.1,
        1e-12
    ));
    assert!(gain_over_baseline(3.3, &adm(0.0, "zero")).is_err());
    assert!(gain_over_baseline(3.3, &unresolved()).is_err());
}

#[test]
fn multiplicity_splits_significance_and_weights_must_normalise() {
    let p = profile(PROPOSED);
    assert!(close(p.per_comparison_alpha().unwrap(), 0.0125, 1e-12));
    assert!(profile([0.5, 0.5, 0.5, 0.5]).check().is_err());
}

#[test]
fn g009_guard_nonfinite_frozen_duty_cannot_bind_energy() {
    let (cv, mut duty) = fixture();
    assert!(energy_flux_loss_per_treated_m3(&cv, &duty).is_ok());
    duty.feed_m3_h = f64::NAN;
    assert!(energy_flux_loss_per_treated_m3(&cv, &duty).is_err());
}
#[test]
fn g009_guard_tiny_transfer_cannot_lose_all_parent_mass() {
    let matrix = TransferMatrix {
        parent_kg: vec![1e-15],
        rows: vec![TransferRow {
            reject_kg: 0.0,
            treated_kg: 0.0,
            in_domain_kg: 0.0,
        }],
    };
    assert!(matrix.check().is_err());
    let closed = TransferMatrix {
        rows: vec![TransferRow {
            reject_kg: 1e-15,
            treated_kg: 0.0,
            in_domain_kg: 0.0,
        }],
        ..matrix
    };
    assert!(closed.check().is_ok());
}
#[test]
fn g009_guard_derived_concentration_and_gain_must_be_finite() {
    assert!(outlet_concentration_from_reduced_mg_l(f64::MAX, -1.0).is_err());
    let w = SynchronizedWindow {
        start_s: 0.0,
        end_s: 1.0,
        treated_oil_kg: f64::MAX,
        treated_water_m3: 1e-10,
    };
    assert!(transient_outlet_concentration_mg_l(&w).is_err());
    assert!(gain_over_baseline(f64::MAX, &adm(1e-10, "tiny baseline")).is_err());
}
#[test]
fn g009_guard_nonfinite_or_negative_objective_is_unresolved() {
    for bad in [f64::NAN, f64::INFINITY, -1.0] {
        let candidate = record("invalid", bad, 2.0, 100.0, 1.0);
        let decision = decide(
            &profile([0.35, 0.20, 0.30, 0.15]),
            &baseline(),
            &[candidate],
        )
        .unwrap();
        assert!(!matches!(
            decision.states["invalid"],
            CandidateState::Feasible { .. }
        ));
        assert!(decision.winner.is_none());
    }
}

#[test]
fn v5_18_02_to_05_sc_dec_031_common_flow_maps_treated_volume_energy_bounded_objectives_and_ratios()
{
    // Common-flow maps retain scenario intersections, failure islands and missing evaluations.
    common_capacity_is_min_over_scenarios_at_the_same_feed();
    admitted_regular_band_is_claimed_only_without_gaps();
    disjoint_feasible_scenario_maps_produce_no_common_duty();
    negative_control_crossing_a_failure_island_is_not_a_band();
    missing_scenario_is_a_gap_never_a_pass();

    // Energy is integrated over all ports and normalized by treated volume; per-feed energy
    // cannot be relabelled. Objective gates/floors precede bounded preference utility.
    energy_per_treated_m3_from_integrated_port_fluxes();
    negative_control_per_feed_energy_mislabelled_as_treated_is_rejected();
    energy_rejects_unbalanced_ports_and_unphysical_negative_loss();
    feasible_candidate_has_expected_utility_and_wins();
    negative_control_missing_buckling_gate_cannot_be_compensated();
    baseline_floors_precede_preference();
    unresolved_objective_is_not_a_pass();

    // Ratios have no epsilon denominator; unresolved baselines and incompatible decision
    // policies cannot become a finite gain or inherit an earlier decision identity.
    changed_weights_create_a_new_decision_over_the_same_observations();
    undefined_baseline_capacity_cannot_win();
    negative_control_equal_aspiration_and_reservation_has_no_finite_utility();
    gain_requires_a_positive_admitted_baseline_and_no_epsilon();
    multiplicity_splits_significance_and_weights_must_normalise();
}
