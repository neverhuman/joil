//! Synthetic numerical controls only; none of these fixtures qualifies native CFD.
use crucible_particles::{
    dispersion::{periodic_density_deviation, LesPartition, Matrix3, PeriodicFixture},
    population::{
        refine_mass_classes, refine_velocity_groups, smoluchowski_constant_kernel, BreakupModel,
        DataStatus, DaughterClass, VelocityGroup,
    },
    transfer::{
        cluster_weighted_interval, Applicability, DsdTransferMatrix, ErrorComponents,
        GradeObservation, LinearConditions, WeightedGradeSample,
    },
};

#[test]
fn anisotropic_covariance_and_well_mixed_drift_have_independent_controls() {
    let covariance = Matrix3([[2.0, 0.3, 0.0], [0.3, 1.0, 0.0], [0.0, 0.0, 0.25]]);
    let mut state = 19_u64;
    let mut sums = [0.0; 3];
    let mut cross = [[0.0; 3]; 3];
    let n = 20_000;
    for _ in 0..n {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let z = [normal(&mut state), normal(&mut state), normal(&mut state)];
        let dx = covariance.sample_increment(z, 0.5).unwrap();
        for i in 0..3 {
            sums[i] += dx[i];
            for j in 0..3 {
                cross[i][j] += dx[i] * dx[j];
            }
        }
    }
    let observed = std::array::from_fn::<_, 3, _>(|i| {
        std::array::from_fn::<_, 3, _>(|j| {
            cross[i][j] / n as f64 - (sums[i] / n as f64) * (sums[j] / n as f64)
        })
    });
    assert!((observed[0][0] - 2.0).abs() < 0.07);
    assert!((observed[1][1] - 1.0).abs() < 0.05);
    assert!((observed[0][1] - 0.3).abs() < 0.04);
    assert!((observed[2][2] - 0.25).abs() < 0.02);
    assert!(
        (observed[0][0] - observed[1][1]).abs() > 0.7,
        "isotropic-noise negative control"
    );

    let fixture = PeriodicFixture {
        tracer_count: 24_000,
        steps: 1800,
        dt_s: 0.0025,
        length_m: 1.0,
        base_diffusivity_m2_s: 0.02,
        modulation: 0.72,
        seed: 881,
    };
    let correct = fixture.evolve(true, 0.0).unwrap();
    let wrong = fixture.evolve(false, 0.0).unwrap();
    let correct_tv = periodic_density_deviation(&correct, 1.0, 20).unwrap();
    let wrong_tv = periodic_density_deviation(&wrong, 1.0, 20).unwrap();
    assert!(
        correct_tv < 0.045,
        "correct Ito drift invariant-density TV={correct_tv}"
    );
    assert!(
        wrong_tv > 0.08,
        "wrong-drift fixture must fail; TV={wrong_tv}"
    );
    assert!(
        correct.iter().all(|x| (0.0..1.0).contains(x)),
        "periodic boundary must wrap every tracer into domain"
    );
    let refined_fixture = PeriodicFixture {
        tracer_count: 12_000,
        steps: 3600,
        dt_s: 0.00125,
        length_m: 1.0,
        base_diffusivity_m2_s: 0.02,
        modulation: 0.72,
        seed: 882,
    };
    let refined = refined_fixture.evolve(true, 0.0).unwrap();
    let inlet = PeriodicFixture {
        seed: 883,
        ..refined_fixture
    }
    .evolve(true, 0.7)
    .unwrap();
    let refined_tv = periodic_density_deviation(&refined, 1.0, 20).unwrap();
    let inlet_tv = periodic_density_deviation(&inlet, 1.0, 20).unwrap();
    assert!(
        refined_tv < 0.06 && inlet_tv < 0.06,
        "refined/inlet sensitivity density deviations {refined_tv}, {inlet_tv}"
    );
    // The former telescoping arithmetic has no path to this ensemble acceptance.
}

fn normal(state: &mut u64) -> f64 {
    fn uni(s: &mut u64) -> f64 {
        *s = s.wrapping_mul(2862933555777941757).wrapping_add(3037000493);
        ((*s >> 11) as f64 + 0.5) / ((1_u64 << 53) as f64)
    }
    let u1 = uni(state).max(f64::MIN_POSITIVE);
    let u2 = uni(state);
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

#[test]
fn realizability_and_les_unresolved_energy_controls() {
    let not_psd = Matrix3([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -0.01]]);
    let (fixed, repair) = not_psd.repair_realizability().unwrap();
    assert!(repair > 0.0);
    fixed.sample_increment([1.0, 0.0, 0.0], 0.1).unwrap();
    let partition = LesPartition {
        total_tke_m2_s2: 3.0,
        resolved_tke_m2_s2: 2.5,
        unresolved_tke_m2_s2: 0.5,
    };
    assert!(partition.validate(0.49).is_ok());
    assert!(partition.validate(0.51).is_err()); // full-energy noise would double count resolved fluctuations
    assert!(LesPartition {
        total_tke_m2_s2: 3.0,
        resolved_tke_m2_s2: 2.0,
        unresolved_tke_m2_s2: 0.5
    }
    .validate(0.2)
    .is_err());
}

#[test]
fn breakup_is_exposure_dependent_mass_closed_and_never_promoted_by_hinze_clip() {
    let model = BreakupModel {
        data_status: DataStatus::Uncalibrated,
        rate_per_s: 0.8,
        relative_rate_uncertainty: 0.4,
        model_id: "synthetic-exposure-model".into(),
        daughters: vec![
            DaughterClass {
                diameter_m: 20e-6,
                mass_fraction: 0.4,
            },
            DaughterClass {
                diameter_m: 35e-6,
                mass_fraction: 0.6,
            },
        ],
    };
    let low = model.evaluate(0.1, 1e-9, 0.01, 998.0, 0.03).unwrap();
    let high = model.evaluate(2.0, 1e-9, 0.01, 998.0, 0.03).unwrap();
    assert!(high.probability_nominal > low.probability_nominal);
    assert!(!high.calibrated);
    assert_eq!(high.daughter_mass_fraction, 1.0);
    let mut bad = model.clone();
    bad.daughters[0].mass_fraction = 0.3;
    assert!(bad.evaluate(1.0, 1e-9, 0.01, 998.0, 0.03).is_err());
    assert!(BreakupModel::from_hinze_clipping(30e-6).is_err());
    let (n, exact) = smoluchowski_constant_kernel(1000.0, 0.002, 0.25, 10_000).unwrap();
    assert!((n - exact).abs() / exact < 1e-7);
    let refined = refine_mass_classes(&[0.2, 0.3, 0.5], 4).unwrap();
    assert!((refined.iter().sum::<f64>() - 1.0).abs() < 1e-14);
    let groups = refine_velocity_groups(
        &[
            VelocityGroup {
                velocity_m_s: [1.0, 0.0, 0.0],
                mass_fraction: 0.4,
            },
            VelocityGroup {
                velocity_m_s: [-1.0, 0.0, 0.0],
                mass_fraction: 0.6,
            },
        ],
        3,
    )
    .unwrap();
    assert_eq!(groups.len(), 6);
    assert!(
        groups.iter().any(|g| g.velocity_m_s[0] > 0.0)
            && groups.iter().any(|g| g.velocity_m_s[0] < 0.0)
    );
    assert!((groups.iter().map(|g| g.mass_fraction).sum::<f64>() - 1.0).abs() < 1e-14);
}

#[test]
fn transfer_reuse_closes_mass_matches_heldout_and_rejects_coupling() {
    let m = DsdTransferMatrix {
        condition_id: "linear-case-1".into(),
        outcome_labels: vec!["treated".into(), "reject".into()],
        entries: vec![vec![0.8, 0.2], vec![0.5, 0.5]],
    };
    let c = LinearConditions {
        noninteracting: true,
        coalescence_enabled: false,
        breakup_concentration_coupled: false,
        two_way_coupling: false,
        concentration_fraction: 0.005,
    };
    let result = m.reweight(&[2.0, 1.0], &c).unwrap();
    assert_eq!(result, vec![2.1, 0.9]);
    assert!(m.compare_held_out(&[2.0, 1.0], &result, 1e-12, &c).is_ok());
    let mut coupled = c.clone();
    coupled.coalescence_enabled = true;
    assert!(m.reweight(&[2.0, 1.0], &coupled).is_err());
    assert!(m
        .compare_held_out(&[2.0, 1.0], &[2.0, 1.0], 0.01, &c)
        .is_err());
    assert!(DsdTransferMatrix {
        entries: vec![vec![0.7, 0.2]],
        ..m.clone()
    }
    .validate(&c)
    .is_err());
}

#[test]
fn correlated_daughters_do_not_shrink_cluster_interval_and_grade_basis_is_bound() {
    let mk = |cluster: &str, ancestry: &str, value: f64| WeightedGradeSample {
        cluster_id: cluster.into(),
        ancestry_id: ancestry.into(),
        time_block_id: "block-a".into(),
        value,
        sampling_weight: 1.0,
        units: "mg/L".into(),
        basis: "treated_water_volume".into(),
    };
    let base = vec![mk("c1", "p1", 1.0), mk("c2", "p2", 3.0)];
    let expanded = vec![
        mk("c1", "p1", 1.0),
        mk("c1", "daughter-p1", 1.0),
        mk("c1", "daughter-p1b", 1.0),
        mk("c2", "p2", 3.0),
    ];
    let a = cluster_weighted_interval(&base, 12.706, "crucible-stats::student_t_quantile(0.975,1)")
        .unwrap();
    let b = cluster_weighted_interval(
        &expanded,
        12.706,
        "crucible-stats::student_t_quantile(0.975,1)",
    )
    .unwrap();
    assert_eq!(a.independent_cluster_count, 2);
    assert_eq!(b.independent_cluster_count, 2);
    assert_eq!(a.standard_error, b.standard_error);
    assert_eq!(a.interval_upper, b.interval_upper);
    let mut mixed = base.clone();
    mixed[1].units = "kg/m3".into();
    assert!(cluster_weighted_interval(&mixed, 2.0, "stats").is_err());
    let applicability = Applicability {
        films: false,
        enrichment: false,
        free_gas: false,
        solids: false,
        two_way_coupling: false,
    };
    let observation = GradeObservation::bind(
        a,
        &applicability,
        ErrorComponents {
            class_refinement_error: 0.01,
            substep_error: 0.02,
            model_error: 0.1,
        },
    )
    .unwrap();
    assert_eq!(observation.applicability_exclusions.len(), 5);
    assert_eq!(observation.estimate.units, "mg/L");
    assert!(GradeObservation::bind(
        b,
        &applicability,
        ErrorComponents {
            class_refinement_error: -1.0,
            substep_error: 0.02,
            model_error: 0.1
        }
    )
    .is_err());
}
