//! MAP-Elites archive over *measured* hardware descriptors: total cyclone cells, series
//! depth, recycle count, auxiliary units, and whether later cyclone stages are smaller,
//! alike or larger than the first. Each niche keeps its best design, so the run retains
//! unfamiliar topologies, including chains of unlike stages, long enough to be tested.
//! The archive shapes diversity only; it never adds to fitness.

use crate::develop::Behavior;
use crate::evolve::Individual;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Archive {
    pub cells: BTreeMap<String, Individual>,
    pub insertions: u64,
}

pub fn niche(b: &Behavior) -> String {
    let cells = match b.cyclone_cells {
        0 | 1 => 0,
        2..=3 => 1,
        4..=7 => 2,
        8..=15 => 3,
        _ => 4,
    };
    let depth = b.series_depth.clamp(1, 4);
    let rec = b.recycles.min(2);
    let aux = usize::from(b.coalescer) * 2 + usize::from(b.splitters > 0);
    // Stage taper: 0 alike (within 15 %), 1 later stages smaller, 2 later stages larger.
    let taper = if (b.stage_ratio.ln()).abs() < 1.15_f64.ln() {
        0
    } else if b.stage_ratio < 1.0 {
        1
    } else {
        2
    };
    format!("c{cells}-d{depth}-r{rec}-x{aux}-s{taper}")
}

impl Archive {
    /// Insert if the niche is empty or the candidate ranks better. Returns true on change.
    pub fn offer(&mut self, ind: &Individual) -> bool {
        let Some(b) = &ind.behavior else { return false };
        let key = niche(b);
        let better = match self.cells.get(&key) {
            None => true,
            Some(cur) => ind.score.compare(&cur.score).is_lt(),
        };
        if better {
            self.cells.insert(key, ind.clone());
            self.insertions += 1;
        }
        better
    }

    pub fn feasible(&self) -> usize {
        self.cells.values().filter(|i| i.score.feasible).count()
    }

    pub fn elites(&self) -> Vec<&Individual> {
        self.cells.values().collect()
    }
}
