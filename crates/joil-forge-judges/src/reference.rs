//! CT-35 reconstruction expressed as forge hardware, the same-model comparator.
//!
//! Dimensions follow `baselines/ct35/genome.json` (Colman–Thew ratios on `D2 = 35 mm`):
//! swirl chamber `D1 = 70`, `L1 = 70`; 10° reducing cone to `D2`; fine taper `22 D2` to
//! `D3 = 17.5`; tail `40 D3`; two `h/w = 2` inlets with total area `0.0625 π D1²/4`;
//! overflow orifice `0.04 D2`. Channels outside the liner are synthetic forge plumbing.

use crate::cyclone::CycloneSpec;
use crate::plant::{Dest, Link, Plant, UnitNode, UnitSpec};

/// Wall roughness assumed for every printed forge surface (m).
pub const PRINTED_ROUGHNESS_M: f64 = 20.0e-6;

pub fn ct35_cyclone(split: f64) -> CycloneSpec {
    let d2 = 0.035;
    let d1 = 2.0 * d2;
    let l1 = d1;
    let d3 = 0.5 * d2;
    let lc = 0.5 * (d1 - d2) / 10.0_f64.to_radians().tan();
    let l2 = 22.0 * d2;
    let l3 = 40.0 * d3;
    let z1 = l1;
    let z2 = z1 + lc;
    let z3 = z2 + l2;
    let z4 = z3 + l3;
    CycloneSpec {
        cells: 1,
        stations: vec![
            [0.0, 0.5 * d1],
            [z1, 0.5 * d1],
            [z2, 0.5 * d2],
            [z3, 0.5 * d3],
            [z4, 0.5 * d3],
        ],
        inlet_jets: 2,
        inlet_area_m2: 0.0625 * std::f64::consts::PI * d1 * d1 / 4.0,
        inlet_aspect: 2.0,
        involute: false,
        overflow_d_m: 0.04 * d2,
        split,
        roughness_m: PRINTED_ROUGHNESS_M,
    }
}

/// Single CT-35 liner with short synthetic feed, overflow and underflow channels.
pub fn ct35_plant(split: f64) -> Plant {
    let link = |to, d| Link {
        to,
        pumped: false,
        channel_d_m: d,
        length_m: 0.05,
    };
    Plant {
        units: vec![UnitNode {
            spec: UnitSpec::Cyclone(ct35_cyclone(split)),
            outlets: vec![link(Dest::Reject, 0.006), link(Dest::Product, 0.0175)],
        }],
        feed: link(Dest::Unit(0), 0.025),
    }
}

/// Carrier-CFD anchor recorded for CT-35 at 4 m³/h (`crates/joil-core/src/evolve/types.rs`,
/// traced in `docs/WHITEPAPER.md`): underflow static tap drop and overflow drop in Pa, split.
pub const CT35_CFD_ANCHOR: (f64, f64, f64) = (29_468.22, 208_358.5, 0.019_859_632);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ct35_geometry_matches_baseline_ratios() {
        let c = ct35_cyclone(0.02);
        c.validate().unwrap();
        assert!((c.length() * 1000.0 - 1_639.247_432).abs() < 1e-3);
        assert!((c.inlet_area_m2 * 1e6 - 240.528_188).abs() < 1e-3);
        let (w, h) = c.jet_size();
        assert!((w * 1000.0 - 7.754_486).abs() < 1e-3 && (h * 1000.0 - 15.508_971).abs() < 1e-3);
    }
}
