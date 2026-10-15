use joil_core::{geometry::baseline, metrics::*};
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn limits() -> Limits {
    Limits {
        cout_mg_l: 30.,
        dp_pa: 200_000.,
        energy_j_per_treated_m3: 250_000.,
        reject_min: 0.019,
        reject_max: 0.021,
    }
}
fn obs(name: &str, c: f64) -> Sample {
    Sample::Observed {
        scenario: name.into(),
        cout_upper_mg_l: c,
        dp_upper_pa: 100_000.,
        energy_upper_j_per_treated_m3: 110_000.,
        reject_fraction: 0.02,
    }
}
fn point(q: f64, a: f64, b: f64) -> OperatingPoint {
    OperatingPoint {
        feed_m3_h: q,
        samples: vec![obs("a", a), obs("b", b)],
    }
}
fn required() -> Vec<String> {
    vec!["a".into(), "b".into()]
}

#[test]
fn tracer_does_not_treat_water() {
    for r in [0., 0.02, 0.1, 0.8] {
        close(concentration_mg_l(500., r, r).unwrap(), 500.);
    }
}
#[test]
fn split_denominator_changes_threshold_verdict() {
    let c = concentration_mg_l(500., 0.94, 0.02).unwrap();
    close(c, 30.612244897959183);
    assert!(c > 30.);
}
#[test]
fn oil_capture_extremes() {
    close(concentration_mg_l(500., 1., 0.02).unwrap(), 0.);
    close(
        concentration_mg_l(500., 0., 0.02).unwrap(),
        510.2040816326531,
    );
}
#[test]
fn reduced_form_matches_mass_form() {
    let e = 0.95;
    let g = reduced_efficiency(e, 0.02).unwrap();
    close(500. * (1. - g), concentration_mg_l(500., e, 0.02).unwrap());
}
#[test]
fn negative_reduced_efficiency_is_preserved() {
    assert!(reduced_efficiency(0.01, 0.02).unwrap() < 0.);
}
#[test]
fn concentration_rejects_invalid_inputs() {
    for (c, e, r) in [
        (-1., 0.9, 0.02),
        (500., 1.1, 0.02),
        (500., -0.1, 0.02),
        (500., 0.9, 1.),
        (500., 0.9, -0.1),
        (f64::NAN, 0.9, 0.02),
        (500., 0.9, f64::INFINITY),
    ] {
        assert!(concentration_mg_l(c, e, r).is_err());
    }
}
#[test]
fn mass_weighted_bins() {
    close(weighted_capture(&[0.25, 0.75], &[0.2, 1.]).unwrap(), 0.8);
    assert!(weighted_capture(&[1., 1.], &[0.5, 0.5]).is_err());
    assert!(weighted_capture(&[], &[]).is_err());
}
#[test]
fn pdr_uses_correct_outlets() {
    close(pdr(500_000., 140_000., 300_000.).unwrap(), 1.8);
    assert!(pdr(1., 0., 1.).is_err());
}
#[test]
fn inventory_retained_oil_is_not_capture() {
    let b = OilBalance {
        injected_kg: 1.,
        reject_kg: 0.8,
        treated_kg: 0.1,
        remaining_kg: 0.06,
        wall_kg: 0.04,
    };
    close(b.residual_fraction().unwrap(), 0.);
    let (lo, hi) = b.capture_bounds(1e-9).unwrap();
    close(lo, 0.8);
    close(hi, 0.9);
}
#[test]
fn missing_oil_does_not_pass() {
    let b = OilBalance {
        injected_kg: 1.,
        reject_kg: 0.8,
        treated_kg: 0.1,
        remaining_kg: 0.,
        wall_kg: 0.,
    };
    assert!(b.capture_bounds(0.01).is_err());
}
#[test]
fn invalid_inventory_rejected() {
    let b = OilBalance {
        injected_kg: 0.,
        reject_kg: 0.,
        treated_kg: 0.,
        remaining_kg: 0.,
        wall_kg: 0.,
    };
    assert!(b.residual_fraction().is_err());
}
#[test]
fn disjoint_scenario_maxima_have_no_common_capacity() {
    let ps = [point(2., 20., 40.), point(4., 40., 20.)];
    assert_eq!(
        common_tested_capacity(&ps, &required(), &limits()).unwrap(),
        None
    );
}
#[test]
fn nonmonotonic_operating_map() {
    let ps = [
        point(5., 40., 40.),
        point(2., 20., 20.),
        point(3., 40., 40.),
        point(4., 25., 25.),
    ];
    let c = common_tested_capacity(&ps, &required(), &limits())
        .unwrap()
        .unwrap();
    close(c.feed_m3_h, 4.);
    close(c.treated_m3_h, 3.92);
}
#[test]
fn missing_or_unknown_scenario_never_passes() {
    for samples in [
        vec![obs("a", 20.)],
        vec![
            obs("a", 20.),
            Sample::Unknown {
                scenario: "b".into(),
                reason: "timeout".into(),
            },
        ],
    ] {
        let p = OperatingPoint {
            feed_m3_h: 4.,
            samples,
        };
        assert_eq!(
            common_tested_capacity(&[p], &required(), &limits()).unwrap(),
            None
        );
    }
}
#[test]
fn reject_cheating_fails() {
    let s = Sample::Observed {
        scenario: "a".into(),
        cout_upper_mg_l: 10.,
        dp_upper_pa: 1.,
        energy_upper_j_per_treated_m3: 1.,
        reject_fraction: 0.2,
    };
    let p = OperatingPoint {
        feed_m3_h: 4.,
        samples: vec![s, obs("b", 20.)],
    };
    assert_eq!(
        common_tested_capacity(&[p], &required(), &limits()).unwrap(),
        None
    );
}
#[test]
fn duplicate_scenarios_are_errors() {
    let p = OperatingPoint {
        feed_m3_h: 4.,
        samples: vec![obs("a", 20.), obs("a", 20.)],
    };
    assert!(common_tested_capacity(&[p], &required(), &limits()).is_err());
}
#[test]
fn duplicate_flows_are_errors() {
    assert!(common_tested_capacity(
        &[point(4., 20., 20.), point(4., 20., 20.)],
        &required(),
        &limits()
    )
    .is_err());
}
#[test]
fn unknown_is_not_zero_concentration() {
    let raw = r#"{"status":"observed","scenario":"a","dp_upper_pa":1,"energy_upper_j_per_treated_m3":1,"reject_fraction":0.02}"#;
    assert!(serde_json::from_str::<Sample>(raw).is_err());
}
#[test]
fn no_baseline_means_no_gain() {
    for x in [0., -1., f64::NAN] {
        assert!(gain(4., x).is_err());
    }
    close(gain(4.4, 4.).unwrap(), 0.1);
}
#[test]
fn baseline_dimensions_match_independent_fixture() {
    let d = baseline().derive().unwrap();
    close(d.d1_mm, 70.);
    close(d.d2_mm, 35.);
    close(d.overflow_diameter_mm, 1.4);
    close(d.cone_length_mm, 99.24743184330991);
    close(d.z_underflow_mm, 1639.2474318433099);
    close(d.z_head_end_mm, 186.74743184330993);
    assert!((d.fluid_volume_mm3 - 1105802.5227442954).abs() < 1e-6);
}
#[test]
fn geometry_scale_preserves_dimensionless_shape() {
    let g = baseline();
    let a = g.derive().unwrap();
    let mut h = g.clone();
    let p = &mut h.parameters;
    p.d2_mm *= 2.;
    p.inlet_z_offset_mm *= 2.;
    p.inlet_length_cfd_mm *= 2.;
    p.inlet_length_part_mm *= 2.;
    p.plenum_radius_mm *= 2.;
    p.plenum_length_mm *= 2.;
    p.wall_thickness_mm *= 2.;
    let b = h.derive().unwrap();
    close(b.z_underflow_mm / a.z_underflow_mm, 2.);
    close(b.fluid_volume_mm3 / a.fluid_volume_mm3, 8.);
}
#[test]
fn invalid_gene_does_not_clamp_to_baseline() {
    let mut g = baseline();
    g.parameters.d2_mm = f64::NAN;
    assert!(g.derive().is_err());
    g = baseline();
    g.family = "freeform-not-implemented".into();
    assert!(g.derive().is_err());
}
#[test]
fn identity_changes_with_gene() {
    let g = baseline();
    let mut h = g.clone();
    h.parameters.do_over_d2 = 0.05;
    assert_ne!(g.sha256().unwrap(), h.sha256().unwrap());
}
#[test]
fn json_roundtrip_and_unknown_field() {
    let g = baseline();
    let bytes = serde_json::to_vec(&g).unwrap();
    let h: joil_core::geometry::Genome = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(g.sha256().unwrap(), h.sha256().unwrap());
    let mut v = serde_json::to_value(&g).unwrap();
    v["secret_default"] = 1.into();
    assert!(serde_json::from_value::<joil_core::geometry::Genome>(v).is_err());
}
