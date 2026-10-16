use joil_core::metrics::pareto_hypervolume;

#[test]
fn truth_pareto_hypervolume_matches_pymoo() {
    // Independently computed with pymoo==0.6.1.3, pymoo.indicators.hv.HV.
    // target/truth-hypervolume-reference.json SHA-256:
    // 4dc9c6286b44bcd1709fd54fb1d8bf04e1c7f1bd8253f2f44523e9b785db3e30
    let fixtures = [
        (
            vec![vec![1.0, 4.0], vec![2.0, 2.0], vec![4.0, 1.0]],
            vec![5.0; 2],
            11.0,
        ),
        (
            vec![
                vec![0.1, 0.8, 0.6],
                vec![0.5, 0.2, 0.4],
                vec![0.7, 0.5, 0.1],
                vec![0.8, 0.8, 0.8],
                vec![0.5, 0.2, 0.4],
            ],
            vec![1.0; 3],
            0.317,
        ),
        (
            vec![
                vec![0.1, 0.7, 0.4, 0.6, 0.2],
                vec![0.4, 0.1, 0.7, 0.3, 0.5],
                vec![0.5, 0.4, 0.1, 0.8, 0.3],
                vec![0.7, 0.2, 0.6, 0.1, 0.7],
                vec![0.3, 0.6, 0.5, 0.4, 0.4],
                vec![0.8; 5],
            ],
            vec![1.0; 5],
            0.13962000000000002,
        ),
    ];
    for (points, reference, expected) in fixtures {
        let actual = pareto_hypervolume(&points, &reference).unwrap();
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
        let mut duplicate = points.clone();
        duplicate.extend(points.clone());
        assert_eq!(actual, pareto_hypervolume(&duplicate, &reference).unwrap());
        let largest_box = points
            .iter()
            .map(|p| {
                p.iter()
                    .zip(&reference)
                    .map(|(p, r)| r - p)
                    .product::<f64>()
            })
            .fold(0.0, f64::max);
        assert!(
            actual > largest_box,
            "fixture must distinguish a union from the former max-box proxy"
        );
    }
    assert_eq!(pareto_hypervolume(&[], &[1.0; 5]).unwrap(), 0.0);
    assert_eq!(
        pareto_hypervolume(&[vec![2.0, 0.0]], &[1.0; 2]).unwrap(),
        0.0
    );
    assert!(pareto_hypervolume(&[vec![f64::NAN]], &[1.0]).is_err());
    assert!(pareto_hypervolume(&[vec![0.0; 2]], &[1.0]).is_err());
}
