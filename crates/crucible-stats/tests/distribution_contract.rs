use crucible_stats::{analysis::sample_moments, distribution::*, StatsError};

#[test]
fn independent_critical_values_reject_legacy_t_approximation() {
    // Frozen v5 S02/S03 fixtures, independent of the distribution implementation.
    let small_n = student_t_quantile(0.975, 4).unwrap();
    assert!((small_n - 2.776_445_105_2).abs() < 1e-9);
    let old_approximation: f64 = 1.95996398 + 2.37 / 4.0 + 2.8 / 16.0;
    assert!((old_approximation - 2.776_445_105_2).abs() > 0.04);
    assert!((student_t_quantile(0.9875, 19).unwrap() - 2.433_440_211_4).abs() < 1e-9);
    assert!((normal_quantile(0.975).unwrap() - 1.959_963_984_540_054).abs() < 1e-12);
    // df=1 is a Cauchy: this closed-form oracle is not a t table lookup.
    assert!((student_t_quantile(0.75, 1).unwrap() - 1.0).abs() < 1e-12);
    assert!((student_t_quantile(0.25, 1).unwrap() + 1.0).abs() < 1e-12);
}

#[test]
fn invalid_distribution_inputs_are_errors_before_library_evaluation() {
    for p in [f64::NAN, f64::INFINITY, -0.1, 0.0, 1.0, 1.1] {
        assert!(matches!(
            student_t_quantile(p, 4),
            Err(StatsError::InvalidInput(_))
        ));
        assert!(matches!(
            normal_quantile(p),
            Err(StatsError::InvalidInput(_))
        ));
    }
    assert!(student_t_quantile(0.975, 0).is_err());
    #[cfg(target_pointer_width = "64")]
    assert!(student_t_quantile(0.975, (1_usize << 53) + 1).is_err());
}

#[test]
fn finite_moments_preserve_zero_variance_and_reject_overflow() {
    assert_eq!(sample_moments(&[1.0, 2.0, 3.0]).unwrap(), (2.0, 1.0));
    assert_eq!(sample_moments(&[7.0; 5]).unwrap(), (7.0, 0.0));
    for values in [
        vec![],
        vec![1.0],
        vec![f64::NAN, 2.0],
        vec![f64::MAX, -f64::MAX],
    ] {
        assert!(sample_moments(&values).is_err());
    }
}
