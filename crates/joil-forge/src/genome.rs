//! The forge genome: a variable-length process network.
//!
//! A genome is a list of unit genes (hydrocyclone stages, lattice coalescer beds, flow
//! splitters) with stable innovation identities, plus one route per outlet port naming
//! the destination by identity. Series stages, parallel branches, numbered-up cells,
//! reject scavengers and pumped recycle loops are all ordinary values of this one type;
//! nothing about a "family" or archetype is stored. Development ([`crate::develop`])
//! turns a genome into hardware and a cartridge layout; the judges never see the genome.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA: &str = "joil-forge-genome/1";

/// Bounds of every continuous and integer gene. Normalisation clamps into these.
pub mod bounds {
    pub const MAX_UNITS: usize = 10;
    pub const MAX_TOTAL_CELLS: u32 = 48;
    pub const DIAMETER_MM: (f64, f64) = (8.0, 72.0);
    pub const CHAMBER_LEN_RATIO: (f64, f64) = (0.3, 3.0);
    pub const SEGMENTS: (usize, usize) = (1, 6);
    pub const SEG_LEN_RATIO: (f64, f64) = (0.1, 30.0);
    pub const SEG_END_RATIO: (f64, f64) = (0.1, 1.3);
    pub const JETS: (u32, u32) = (1, 8);
    pub const INLET_AREA_RATIO: (f64, f64) = (0.015, 0.35);
    pub const INLET_ASPECT: (f64, f64) = (0.4, 4.0);
    pub const ORIFICE_RATIO: (f64, f64) = (0.02, 0.35);
    pub const SPLIT: (f64, f64) = (0.003, 0.5);
    pub const CELLS: (u32, u32) = (1, 19);
    pub const BED_DIAMETER_MM: (f64, f64) = (15.0, 72.0);
    pub const BED_LENGTH_MM: (f64, f64) = (20.0, 600.0);
    pub const BED_POROSITY: (f64, f64) = (0.5, 0.95);
    pub const BED_STRUT_MM: (f64, f64) = (1.0, 4.0);
    pub const BED_CELLS: (u32, u32) = (1, 7);
    pub const SPLIT_FRACTION: (f64, f64) = (0.05, 0.95);
    pub const CHANNEL_MM: (f64, f64) = (3.0, 24.0);
    /// Smallest wall radius anywhere in a cell (mm): printable 3 mm passage.
    pub const MIN_RADIUS_MM: f64 = 1.5;
    /// Machined overflow orifice minimum (mm).
    pub const MIN_ORIFICE_MM: f64 = 1.0;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Unit(u32),
    Product,
    Reject,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub to: Target,
    pub pumped: bool,
    pub channel_mm: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    /// Segment length / roof diameter.
    pub len_ratio: f64,
    /// Wall radius at the segment end / roof radius.
    pub end_ratio: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CycloneGene {
    pub diameter_mm: f64,
    pub chamber_len_ratio: f64,
    pub segments: Vec<Segment>,
    pub jets: u32,
    pub inlet_area_ratio: f64,
    pub inlet_aspect: f64,
    pub involute: bool,
    pub orifice_ratio: f64,
    pub split: f64,
    pub cells: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoalescerGene {
    pub diameter_mm: f64,
    pub length_mm: f64,
    pub porosity: f64,
    pub strut_mm: f64,
    pub cells: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Body {
    Cyclone(CycloneGene),
    Coalescer(CoalescerGene),
    Splitter { fraction: f64 },
}

impl Body {
    pub fn outlets(&self) -> usize {
        match self {
            Body::Coalescer(_) => 1,
            _ => 2,
        }
    }

    pub fn cells(&self) -> u32 {
        match self {
            Body::Cyclone(c) => c.cells,
            Body::Coalescer(c) => c.cells,
            Body::Splitter { .. } => 0,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Body::Cyclone(_) => "cyclone",
            Body::Coalescer(_) => "coalescer",
            Body::Splitter { .. } => "splitter",
        }
    }

    /// Default destination of outlet `k` when its route is missing or broken:
    /// cyclone overflow to reject, everything else to product.
    pub fn default_target(&self, k: usize) -> Target {
        match (self, k) {
            (Body::Cyclone(_), 0) => Target::Reject,
            _ => Target::Product,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitGene {
    pub id: u32,
    pub body: Body,
    pub routes: Vec<Route>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Genome {
    pub schema: String,
    pub feed: Route,
    pub units: Vec<UnitGene>,
}

fn clampf(x: f64, b: (f64, f64)) -> f64 {
    if x.is_finite() {
        x.clamp(b.0, b.1)
    } else {
        b.0
    }
}

fn clampu(x: u32, b: (u32, u32)) -> u32 {
    x.clamp(b.0, b.1)
}

/// Declared quantisation: continuous genes are rounded to 1e-4 in their own units
/// (1e-6 for the small dimensionless inlet, orifice and split ratios).
pub fn round4(x: f64) -> f64 {
    (x * 1e4).round() / 1e4
}

pub fn round6(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
}

fn ceil4(x: f64) -> f64 {
    (x * 1e4 - 1e-9).ceil() / 1e4
}

impl Route {
    pub fn to(to: Target) -> Self {
        Self {
            to,
            pumped: false,
            channel_mm: 8.0,
        }
    }
}

impl CycloneGene {
    fn normalize(&mut self) {
        use bounds::*;
        self.diameter_mm = round4(clampf(self.diameter_mm, DIAMETER_MM));
        self.chamber_len_ratio = clampf(self.chamber_len_ratio, CHAMBER_LEN_RATIO);
        if self.segments.is_empty() {
            self.segments.push(Segment {
                len_ratio: 5.0,
                end_ratio: 0.5,
            });
        }
        self.segments.truncate(SEGMENTS.1);
        let min_ratio = (2.0 * (MIN_RADIUS_MM + 0.05) / self.diameter_mm).max(SEG_END_RATIO.0);
        // Keep every wall inside the judges' applicability envelope (repair, not a gate):
        // a segment is lengthened until its half-angle is within the declared limit.
        let (tan_c, tan_e) = (24.5_f64.to_radians().tan(), 7.8_f64.to_radians().tan());
        let mut prev = 1.0;
        for s in &mut self.segments {
            s.end_ratio = round4(clampf(s.end_ratio, (min_ratio, SEG_END_RATIO.1)));
            let dr = s.end_ratio - prev;
            let need = dr.abs() / (2.0 * if dr < 0.0 { tan_c } else { tan_e });
            s.len_ratio = round4(clampf(s.len_ratio.max(need + 1e-4), SEG_LEN_RATIO));
            prev = s.end_ratio;
        }
        self.jets = clampu(self.jets, JETS);
        self.inlet_aspect = round4(clampf(self.inlet_aspect, INLET_ASPECT));
        self.inlet_area_ratio = clampf(self.inlet_area_ratio, INLET_AREA_RATIO);
        // Jet width must stay below 0.55 R and above 1 mm; jet height inside the chamber.
        let d = self.diameter_mm;
        let area = |ratio: f64| ratio * std::f64::consts::PI * d * d / 4.0;
        let width =
            |ratio: f64, jets: u32, aspect: f64| (area(ratio) / jets as f64 / aspect).sqrt();
        let w_max = 0.55 * 0.5 * d;
        // Thresholds sit outside the targets so that rounding keeps normalisation idempotent.
        let w = width(self.inlet_area_ratio, self.jets, self.inlet_aspect);
        if w > w_max * 1.002 {
            self.inlet_area_ratio *= (w_max / w).powi(2);
        }
        while width(self.inlet_area_ratio, self.jets, self.inlet_aspect) < 1.0 && self.jets > 1 {
            self.jets -= 1;
        }
        let w = width(self.inlet_area_ratio, self.jets, self.inlet_aspect);
        if w < 1.0 {
            self.inlet_area_ratio *= (1.05 / w).powi(2);
        }
        self.inlet_area_ratio = round6(self.inlet_area_ratio);
        let h =
            width(self.inlet_area_ratio, self.jets, self.inlet_aspect).max(1.0) * self.inlet_aspect;
        let need = 1.05 * h / d;
        self.chamber_len_ratio = ceil4(
            self.chamber_len_ratio
                .max(need)
                .min(CHAMBER_LEN_RATIO.1.max(need)),
        );
        let min_orifice = (1.01 * MIN_ORIFICE_MM / d).max(ORIFICE_RATIO.0);
        self.orifice_ratio = round6(clampf(self.orifice_ratio, (min_orifice, ORIFICE_RATIO.1)));
        self.split = round6(clampf(self.split, SPLIT));
        self.cells = clampu(self.cells, CELLS);
    }
}

impl Genome {
    pub fn ids(&self) -> BTreeSet<u32> {
        self.units.iter().map(|u| u.id).collect()
    }

    pub fn max_id(&self) -> u32 {
        self.units.iter().map(|u| u.id).max().unwrap_or(0)
    }

    pub fn index_of(&self, id: u32) -> Option<usize> {
        self.units.iter().position(|u| u.id == id)
    }

    pub fn total_cells(&self) -> u32 {
        self.units.iter().map(|u| u.body.cells()).sum()
    }

    /// Units reachable from the feed through routes.
    pub fn reachable(&self) -> BTreeSet<u32> {
        let index: BTreeMap<u32, &UnitGene> = self.units.iter().map(|u| (u.id, u)).collect();
        let mut seen = BTreeSet::new();
        let mut stack = Vec::new();
        if let Target::Unit(id) = self.feed.to {
            stack.push(id);
        }
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(u) = index.get(&id) {
                for r in &u.routes {
                    if let Target::Unit(n) = r.to {
                        stack.push(n);
                    }
                }
            }
        }
        seen.retain(|id| index.contains_key(id));
        seen
    }

    /// Canonical form: bounded genes, repaired references, reachable units only.
    /// Idempotent. Inactive copies of equal hardware share one canonical hash.
    pub fn normalize(&mut self) {
        self.schema = SCHEMA.to_string();
        // unique ids (later duplicates get fresh ids)
        let mut seen = BTreeSet::new();
        let mut next = self.max_id() + 1;
        for u in &mut self.units {
            if !seen.insert(u.id) {
                u.id = next;
                seen.insert(next);
                next += 1;
            }
        }
        if self.units.is_empty() {
            return;
        }
        let ids = self.ids();
        if !matches!(self.feed.to, Target::Unit(id) if ids.contains(&id)) {
            self.feed.to = Target::Unit(self.units[0].id);
        }
        normalize_route(&mut self.feed, &ids, Target::Unit(self.units[0].id));
        self.feed.pumped = false;
        for u in &mut self.units {
            match &mut u.body {
                Body::Cyclone(c) => c.normalize(),
                Body::Coalescer(c) => {
                    use bounds::*;
                    c.diameter_mm = round4(clampf(c.diameter_mm, BED_DIAMETER_MM));
                    c.length_mm = round4(clampf(c.length_mm, BED_LENGTH_MM));
                    c.porosity = round4(clampf(c.porosity, BED_POROSITY));
                    c.strut_mm = round4(clampf(c.strut_mm, BED_STRUT_MM)).min(c.diameter_mm / 4.0);
                    c.cells = clampu(c.cells, BED_CELLS);
                }
                Body::Splitter { fraction } => {
                    *fraction = round4(clampf(*fraction, bounds::SPLIT_FRACTION))
                }
            }
            let n = u.body.outlets();
            u.routes.truncate(n);
            while u.routes.len() < n {
                let k = u.routes.len();
                u.routes.push(Route::to(u.body.default_target(k)));
            }
            for (k, r) in u.routes.iter_mut().enumerate() {
                let fallback = u.body.default_target(k);
                normalize_route(r, &ids, fallback);
            }
        }
        let live = self.reachable();
        self.units.retain(|u| live.contains(&u.id));
        self.units.truncate(bounds::MAX_UNITS);
        // References to units removed by truncation fall back to terminals.
        let ids = self.ids();
        for u in &mut self.units {
            let defaults: Vec<Target> = (0..u.routes.len())
                .map(|k| u.body.default_target(k))
                .collect();
            for (k, r) in u.routes.iter_mut().enumerate() {
                normalize_route(r, &ids, defaults[k]);
            }
        }
        // Cap the total number of cells by shrinking the largest units.
        while self.total_cells() > bounds::MAX_TOTAL_CELLS {
            if let Some(u) = self.units.iter_mut().max_by_key(|u| u.body.cells()) {
                match &mut u.body {
                    Body::Cyclone(c) => c.cells -= 1,
                    Body::Coalescer(c) => c.cells -= 1,
                    Body::Splitter { .. } => break,
                }
            }
        }
    }

    pub fn normalized(mut self) -> Self {
        self.normalize();
        self
    }

    /// SHA-256 of the canonical JSON (normalise first).
    pub fn canonical_sha256(&self) -> String {
        let canon = self.clone().normalized();
        let bytes = serde_json::to_vec(&canon).expect("genome serialises");
        format!("{:x}", Sha256::digest(&bytes))
    }
}

fn normalize_route(r: &mut Route, ids: &BTreeSet<u32>, fallback: Target) {
    if let Target::Unit(id) = r.to {
        if !ids.contains(&id) {
            r.to = fallback;
        }
    }
    if !matches!(r.to, Target::Unit(_)) {
        r.pumped = false;
    }
    r.channel_mm = round4(clampf(r.channel_mm, bounds::CHANNEL_MM));
}
