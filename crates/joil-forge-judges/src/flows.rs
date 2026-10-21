//! Volumetric network flows with recycle: `Q_u = Σ_{links v→u} f_v Q_v + feed`.

use crate::math::dense_solve;
use crate::plant::{Dest, Plant};
use crate::{JudgeError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Flows {
    pub q_feed: f64,
    /// Inlet flow of each unit (all cells together), m³/s.
    pub unit_in: Vec<f64>,
    /// Flow on each unit outlet link, m³/s.
    pub link: Vec<Vec<f64>>,
    pub q_product: f64,
    pub q_reject: f64,
    /// Largest unit inlet flow divided by the feed (recycle amplification).
    pub amplification: f64,
    /// Units receiving flow.
    pub active: Vec<bool>,
}

pub fn solve(plant: &Plant, q_feed: f64) -> Result<Flows> {
    plant.validate()?;
    if !(q_feed > 0.0 && q_feed.is_finite()) {
        return Err(JudgeError::Input("feed flow must be positive".into()));
    }
    let n = plant.units.len();
    let mut a = vec![0.0; n * n];
    let mut b = vec![0.0; n];
    for i in 0..n {
        a[i * n + i] = 1.0;
    }
    if let Dest::Unit(j) = plant.feed.to {
        b[j] += q_feed;
    }
    let fractions: Vec<Vec<f64>> = plant
        .units
        .iter()
        .map(|u| u.spec.outlet_fractions())
        .collect();
    for (i, u) in plant.units.iter().enumerate() {
        for (k, l) in u.outlets.iter().enumerate() {
            if let Dest::Unit(j) = l.to {
                a[j * n + i] -= fractions[i][k];
            }
        }
    }
    let q = dense_solve(a, b)?;
    let tol = 1e-9 * q_feed;
    if q.iter().any(|v| *v < -tol) {
        return Err(JudgeError::Numerical(
            "negative unit flow in network solve".into(),
        ));
    }
    let unit_in: Vec<f64> = q.into_iter().map(|v| v.max(0.0)).collect();
    let mut link = Vec::with_capacity(n);
    let (mut q_product, mut q_reject) = (0.0, 0.0);
    match plant.feed.to {
        Dest::Product => q_product += q_feed,
        Dest::Reject => q_reject += q_feed,
        Dest::Unit(_) => {}
    }
    for (i, u) in plant.units.iter().enumerate() {
        let flows: Vec<f64> = fractions[i].iter().map(|f| f * unit_in[i]).collect();
        for (k, l) in u.outlets.iter().enumerate() {
            match l.to {
                Dest::Product => q_product += flows[k],
                Dest::Reject => q_reject += flows[k],
                Dest::Unit(_) => {}
            }
        }
        link.push(flows);
    }
    let balance = (q_product + q_reject - q_feed).abs() / q_feed;
    if balance > 1e-9 {
        return Err(JudgeError::Numerical(format!(
            "water balance error {balance:e}"
        )));
    }
    let amplification = unit_in.iter().fold(0.0_f64, |m, v| m.max(*v)) / q_feed;
    let active = unit_in.iter().map(|v| *v > tol).collect();
    Ok(Flows {
        q_feed,
        unit_in,
        link,
        q_product,
        q_reject,
        amplification,
        active,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plant::{Link, UnitNode, UnitSpec};

    fn link(to: Dest) -> Link {
        Link {
            to,
            pumped: false,
            channel_d_m: 0.008,
            length_m: 0.05,
        }
    }

    #[test]
    fn recycle_matches_geometric_series() {
        // splitter: 30 % back to itself, 70 % to product. Inlet = Q / (1 - 0.3).
        let plant = Plant {
            units: vec![UnitNode {
                spec: UnitSpec::Splitter { fraction: 0.3 },
                outlets: vec![link(Dest::Unit(0)), link(Dest::Product)],
            }],
            feed: link(Dest::Unit(0)),
        };
        let f = solve(&plant, 1.0).unwrap();
        assert!((f.unit_in[0] - 1.0 / 0.7).abs() < 1e-12);
        assert!((f.q_product - 1.0).abs() < 1e-12);
    }

    #[test]
    fn closed_loop_is_rejected() {
        let plant = Plant {
            units: vec![
                UnitNode {
                    spec: UnitSpec::Splitter { fraction: 0.5 },
                    outlets: vec![link(Dest::Unit(1)), link(Dest::Unit(1))],
                },
                UnitNode {
                    spec: UnitSpec::Splitter { fraction: 0.5 },
                    outlets: vec![link(Dest::Unit(0)), link(Dest::Unit(0))],
                },
            ],
            feed: link(Dest::Unit(0)),
        };
        assert!(matches!(solve(&plant, 1.0), Err(JudgeError::Hydraulic(_))));
    }
}
