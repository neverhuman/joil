//! Analytic oracles and conservation controls for the forge judges.
//! These are software and numerical checks, not physical validation.

use joil_forge_judges::closure::Closure;
use joil_forge_judges::cyclone::{hydraulics, Resolution};
use joil_forge_judges::fluid::{underflow_concentration, FeedState};
use joil_forge_judges::network::{evaluate, feed_pressure, Settings};
use joil_forge_judges::plant::{Dest, Link, Plant, UnitNode, UnitSpec};
use joil_forge_judges::reference::{ct35_cyclone, ct35_plant, CT35_CFD_ANCHOR};
use joil_forge_judges::separation::separate;
use joil_forge_judges::JudgeError;

fn link(to: Dest, pumped: bool) -> Link {
    Link {
        to,
        pumped,
        channel_d_m: 0.012,
        length_m: 0.08,
    }
}

/// Settings with breakup switched off (huge Davies constant).
fn no_breakup() -> Settings {
    let mut s = Settings::reference();
    s.closure.davies_c = 1.0e6;
    s.closure.satellite_fraction = 0.0;
    s
}

#[test]
fn passive_tracer_reports_exactly_the_split() {
    let s = Settings::reference();
    for split in [0.005, 0.02, 0.1, 0.4] {
        for scale in [0.4, 1.0] {
            let mut c = ct35_cyclone(split);
            c.stations.iter_mut().for_each(|st| {
                st[0] *= scale;
                st[1] *= scale;
            });
            c.inlet_area_m2 *= scale * scale;
            c.overflow_d_m = (c.overflow_d_m * scale).max(0.001);
            let h = hydraulics(
                &c,
                2.0e-4 * scale,
                &s.fluids,
                &s.closure,
                Resolution::default(),
            )
            .unwrap();
            let r = separate(&c, &h, &s.grid, &s.fluids, &s.closure, 12).unwrap();
            assert!(
                (r.tracer_capture - split).abs() < 1e-12,
                "split {split}: {}",
                r.tracer_capture
            );
            assert!(r.mass_error < 1e-12);
        }
    }
}

#[test]
fn grade_efficiency_is_monotone_and_bounded() {
    let s = Settings::reference();
    let c = ct35_cyclone(0.02);
    let h = hydraulics(
        &c,
        4.0 / 3600.0,
        &s.fluids,
        &s.closure,
        Resolution::default(),
    )
    .unwrap();
    let r = separate(&c, &h, &s.grid, &s.fluids, &s.closure, 12).unwrap();
    for w in r.capture.windows(2) {
        assert!(w[1] >= w[0] - 1e-12);
    }
    assert!(r.capture[0] >= 0.02 - 1e-9 && r.capture[0] < 0.05);
    assert!(*r.capture.last().unwrap() > 0.999);
}

#[test]
fn single_stage_matches_science_concentration_formula() {
    let s = no_breakup();
    let plant = ct35_plant(0.02);
    let feed = FeedState::reference();
    let ev = evaluate(&plant, &feed, &s).unwrap();
    let cu = underflow_concentration(feed.cin_mg_l, ev.oil_capture, ev.reject_fraction).unwrap();
    assert!((cu - ev.cout_mg_l).abs() < 1e-9 * cu);
    assert!(ev.oil_balance_rel_error < 1e-12);
}

#[test]
fn series_composition_uses_conditional_populations() {
    let s = no_breakup();
    let (a, b) = (ct35_cyclone(0.01), ct35_cyclone(0.01));
    let plant = Plant {
        units: vec![
            UnitNode {
                spec: UnitSpec::Cyclone(a.clone()),
                outlets: vec![link(Dest::Reject, false), link(Dest::Unit(1), false)],
            },
            UnitNode {
                spec: UnitSpec::Cyclone(b.clone()),
                outlets: vec![link(Dest::Reject, false), link(Dest::Product, false)],
            },
        ],
        feed: link(Dest::Unit(0), false),
    };
    let feed = FeedState::reference();
    let ev = evaluate(&plant, &feed, &s).unwrap();
    let q = feed.q_m3_s();
    let res = s.resolution;
    let ha = hydraulics(&a, q, &s.fluids, &s.closure, res).unwrap();
    let hb = hydraulics(&b, q * 0.99, &s.fluids, &s.closure, res).unwrap();
    let ea = separate(&a, &ha, &s.grid, &s.fluids, &s.closure, res.ns)
        .unwrap()
        .capture;
    let eb = separate(&b, &hb, &s.grid, &s.fluids, &s.closure, res.ns)
        .unwrap()
        .capture;
    let w = s.grid.lognormal_fractions(&feed.dsd);
    for i in 0..s.grid.n() {
        let expected = w[i] * feed.oil_kg_s() * (1.0 - ea[i]) * (1.0 - eb[i]);
        let got = ev.product_dsd[i];
        assert!(
            (got - expected).abs() <= 1e-12 + 1e-9 * expected,
            "class {i}: {got} vs {expected}"
        );
    }
    // Multiplying class-averaged remainders would be wrong: the second stage sees the
    // surviving distribution.
    let avg_a: f64 = w.iter().zip(&ea).map(|(w, e)| w * (1.0 - e)).sum();
    let avg_b: f64 = w.iter().zip(&eb).map(|(w, e)| w * (1.0 - e)).sum();
    let product_oil: f64 = ev.product_dsd.iter().sum::<f64>() / feed.oil_kg_s();
    assert!(product_oil > avg_a * avg_b + 1e-4);
}

#[test]
fn splitter_recycle_loop_conserves_feed_concentration() {
    // Splitters do not separate; with a pumped recycle the product must equal the feed.
    let s = Settings::reference();
    let plant = Plant {
        units: vec![
            UnitNode {
                spec: UnitSpec::Splitter { fraction: 0.6 },
                outlets: vec![link(Dest::Unit(1), false), link(Dest::Product, false)],
            },
            UnitNode {
                spec: UnitSpec::Splitter { fraction: 0.5 },
                outlets: vec![link(Dest::Unit(0), true), link(Dest::Product, false)],
            },
        ],
        feed: link(Dest::Unit(0), false),
    };
    let feed = FeedState::reference();
    let ev = evaluate(&plant, &feed, &s).unwrap();
    assert!((ev.cout_mg_l - feed.cin_mg_l).abs() < 1e-9 * feed.cin_mg_l);
    // Inlet of unit 0 = Q / (1 - 0.6 * 0.5).
    assert!((ev.amplification - 1.0 / 0.7).abs() < 1e-12);
    assert!(ev.oil_iterations > 2 && ev.pump_power_w > 0.0);
}

#[test]
fn unpumped_recycle_is_a_passive_loop() {
    let s = Settings::reference();
    let mut scav = ct35_cyclone(0.2);
    scav.overflow_d_m = 0.006;
    let plant = Plant {
        units: vec![
            UnitNode {
                spec: UnitSpec::Cyclone(ct35_cyclone(0.05)),
                outlets: vec![link(Dest::Unit(1), false), link(Dest::Product, false)],
            },
            UnitNode {
                spec: UnitSpec::Cyclone(scav),
                outlets: vec![link(Dest::Reject, false), link(Dest::Unit(0), false)],
            },
        ],
        feed: link(Dest::Unit(0), false),
    };
    let err = feed_pressure(&plant, 4.0 / 3600.0, &s).unwrap_err();
    assert!(matches!(err, JudgeError::PassiveLoop(_)), "{err}");
    let mut pumped = plant.clone();
    pumped.units[1].outlets[1].pumped = true;
    let ev = evaluate(&pumped, &FeedState::reference(), &s).unwrap();
    assert!(ev.reject_fraction < 0.05 && ev.pump_power_w > 0.0);
    assert!(ev.oil_balance_rel_error < 1e-9);
}

#[test]
fn feed_pressure_scales_close_to_quadratically() {
    let s = Settings::reference();
    let plant = ct35_plant(0.02);
    let p1 = feed_pressure(&plant, 2.0 / 3600.0, &s).unwrap();
    let p2 = feed_pressure(&plant, 4.0 / 3600.0, &s).unwrap();
    let ratio = p2 / p1;
    assert!(ratio > 3.3 && ratio < 4.4, "ratio {ratio}");
}

#[test]
fn ct35_overflow_drop_reproduces_carrier_cfd_anchor() {
    let s = Settings::reference();
    let (dp_u_cfd, dp_o_cfd, split) = CT35_CFD_ANCHOR;
    let h = hydraulics(
        &ct35_cyclone(split),
        4.0 / 3600.0,
        &s.fluids,
        &s.closure,
        Resolution::report(),
    )
    .unwrap();
    assert!(
        (h.dp_overflow_pa / dp_o_cfd - 1.0).abs() < 0.05,
        "dp_o {}",
        h.dp_overflow_pa
    );
    // Declared model-form discrepancy: the energy budget charges discharged swirl and axial
    // kinetic energy and wall friction; the carrier CFD static tap drop is lower.
    let ratio = h.dp_underflow_pa / dp_u_cfd;
    assert!(ratio > 1.0 && ratio < 3.0, "dp_u ratio {ratio}");
}

#[test]
fn closure_box_corners_evaluate() {
    let plant = ct35_plant(0.02);
    for u in [[0.0; 14], [1.0; 14], [0.5; 14]] {
        let mut s = Settings::reference();
        s.closure = Closure::from_unit(&u);
        let ev = evaluate(&plant, &FeedState::reference(), &s).unwrap();
        assert!(ev.cout_mg_l.is_finite() && ev.cout_mg_l > 0.0);
    }
}
