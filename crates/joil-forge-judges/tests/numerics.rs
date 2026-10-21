//! Discretisation controls: search resolution against a much finer march.

use joil_forge_judges::cyclone::{hydraulics, Resolution};
use joil_forge_judges::fluid::FeedState;
use joil_forge_judges::network::Settings;
use joil_forge_judges::reference::ct35_cyclone;
use joil_forge_judges::separation::separate;

fn total_capture(scale: f64, q: f64, res: Resolution) -> f64 {
    let s = Settings::reference();
    let mut c = ct35_cyclone(0.02);
    c.stations.iter_mut().for_each(|st| {
        st[0] *= scale;
        st[1] *= scale;
    });
    c.inlet_area_m2 *= scale * scale;
    c.overflow_d_m = (c.overflow_d_m * scale).max(0.001);
    let h = hydraulics(&c, q, &s.fluids, &s.closure, res).unwrap();
    let r = separate(&c, &h, &s.grid, &s.fluids, &s.closure, res.ns).unwrap();
    let w = s.grid.lognormal_fractions(&FeedState::reference().dsd);
    r.capture.iter().zip(&w).map(|(e, w)| e * w).sum()
}

#[test]
fn search_resolution_is_within_half_percent_of_fine_march() {
    for (scale, q) in [(1.0, 4.0 / 3600.0), (0.3, 0.25 / 3600.0)] {
        let coarse = total_capture(scale, q, Resolution::default());
        let fine = total_capture(scale, q, Resolution { nz: 480, ns: 72 });
        let rel = (coarse / fine - 1.0).abs();
        assert!(rel < 5e-3, "scale {scale}: coarse {coarse} fine {fine}");
    }
}

#[test]
fn steep_cone_into_a_throat_is_resolved() {
    // A 24.5° cone into a 5.8 mm throat, the kind of feature an optimiser finds first.
    let s = Settings::reference();
    let mut c = ct35_cyclone(0.004);
    c.stations = vec![
        [0.0, 0.0289],
        [0.0529, 0.0289],
        [0.1100, 0.00289],
        [0.1379, 0.00671],
        [0.1802, 0.00436],
        [1.3202, 0.00836],
    ];
    c.inlet_area_m2 = 0.041 * std::f64::consts::PI * 0.0578f64.powi(2) / 4.0;
    c.inlet_jets = 4;
    c.inlet_aspect = 0.4;
    c.overflow_d_m = 0.0058;
    let w = s.grid.lognormal_fractions(&FeedState::reference().dsd);
    let cap = |res: Resolution| -> f64 {
        let h = hydraulics(&c, 2.7 / 3600.0, &s.fluids, &s.closure, res).unwrap();
        let r = separate(&c, &h, &s.grid, &s.fluids, &s.closure, res.ns).unwrap();
        r.capture.iter().zip(&w).map(|(e, w)| e * w).sum()
    };
    let (coarse, fine) = (
        cap(Resolution::default()),
        cap(Resolution { nz: 480, ns: 72 }),
    );
    assert!(
        (coarse / fine - 1.0).abs() < 5e-3,
        "coarse {coarse} fine {fine}"
    );
}
