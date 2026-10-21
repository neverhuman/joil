//! Physical description of a separation plant: units wired into a directed network.
//!
//! The plant is what the judges see. It names hardware and wiring only; it carries no
//! genome, no design history and no labels that could influence scoring.

use crate::coalescer::CoalescerSpec;
use crate::cyclone::{CycloneSpec, MIN_FEATURE_M};
use crate::{JudgeError, Result};
use serde::{Deserialize, Serialize};

/// Where a stream goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dest {
    Unit(usize),
    Product,
    Reject,
}

/// A printed channel carrying one outlet stream. `pumped` marks an external boost pump
/// at the channel inlet (the only way a stream may return to higher pressure).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub to: Dest,
    pub pumped: bool,
    pub channel_d_m: f64,
    pub length_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum UnitSpec {
    /// Outlets: `[overflow (reject-side), underflow (treated-side)]`.
    Cyclone(CycloneSpec),
    /// Outlets: `[bed exit]`.
    Coalescer(CoalescerSpec),
    /// Outlets: `[branch a (fraction), branch b (1 - fraction)]`.
    Splitter { fraction: f64 },
}

impl UnitSpec {
    pub fn outlet_count(&self) -> usize {
        match self {
            UnitSpec::Cyclone(_) => 2,
            UnitSpec::Coalescer(_) => 1,
            UnitSpec::Splitter { .. } => 2,
        }
    }

    /// Volumetric fraction of the unit feed leaving through each outlet.
    pub fn outlet_fractions(&self) -> Vec<f64> {
        match self {
            UnitSpec::Cyclone(c) => vec![c.split, 1.0 - c.split],
            UnitSpec::Coalescer(_) => vec![1.0],
            UnitSpec::Splitter { fraction } => vec![*fraction, 1.0 - fraction],
        }
    }

    pub fn cells(&self) -> u32 {
        match self {
            UnitSpec::Cyclone(c) => c.cells,
            UnitSpec::Coalescer(c) => c.cells,
            UnitSpec::Splitter { .. } => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitNode {
    pub spec: UnitSpec,
    pub outlets: Vec<Link>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plant {
    pub units: Vec<UnitNode>,
    pub feed: Link,
}

impl Plant {
    /// Structural validation: outlet counts, references, channel sizes, unit bounds.
    pub fn validate(&self) -> Result<()> {
        if self.units.is_empty() {
            return Err(JudgeError::Hydraulic("plant has no units".into()));
        }
        if self.units.len() > 48 {
            return Err(JudgeError::Input("plant exceeds 48 units".into()));
        }
        let check_link = |l: &Link| -> Result<()> {
            if let Dest::Unit(i) = l.to {
                if i >= self.units.len() {
                    return Err(JudgeError::Input(format!("link to missing unit {i}")));
                }
            }
            if !(l.channel_d_m >= 3.0 * MIN_FEATURE_M && l.channel_d_m.is_finite()) {
                return Err(JudgeError::Geometry(
                    "channel below 3 mm printed passage".into(),
                ));
            }
            if !(l.length_m >= 0.0 && l.length_m.is_finite()) {
                return Err(JudgeError::Input("channel length must be finite".into()));
            }
            Ok(())
        };
        check_link(&self.feed)?;
        if self.feed.pumped {
            return Err(JudgeError::Input(
                "feed link cannot carry a recycle pump".into(),
            ));
        }
        for (i, u) in self.units.iter().enumerate() {
            if u.outlets.len() != u.spec.outlet_count() {
                return Err(JudgeError::Input(format!("unit {i} outlet count mismatch")));
            }
            for l in &u.outlets {
                check_link(l)?;
            }
            match &u.spec {
                UnitSpec::Cyclone(c) => c.validate()?,
                UnitSpec::Coalescer(c) => c.validate()?,
                UnitSpec::Splitter { fraction } => {
                    if !(*fraction > 0.0 && *fraction < 1.0) {
                        return Err(JudgeError::Input(
                            "splitter fraction must lie in (0,1)".into(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
