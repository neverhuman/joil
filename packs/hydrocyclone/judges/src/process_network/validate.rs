use super::*;
use crate::{invalid, JudgeError};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const CONSERVATION_EPS: f64 = 1e-12;

pub(super) fn finite_nonnegative(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
}

impl NetworkRequest {
    /// Bound data/work and check conservative, compatible fixed response kernels.
    pub fn validate(&self) -> Result<(), JudgeError> {
        let n = self.diameter_edges_m.len();
        if self.schema_version != SCHEMA_VERSION || !(2..=65).contains(&n) {
            return Err(invalid("network:schema_or_diameter_bin_count"));
        }
        if self
            .diameter_edges_m
            .iter()
            .any(|x| !x.is_finite() || *x <= 0.0)
            || self.diameter_edges_m.windows(2).any(|w| w[0] >= w[1])
        {
            return Err(invalid("network:diameter_edges_m"));
        }
        let bins = n - 1;
        if self.stages.is_empty()
            || self.stages.len() > 64
            || self.feeds.is_empty()
            || self.feeds.len() > 64
        {
            return Err(invalid("network:stage_or_feed_count"));
        }
        let b = &self.budget;
        if !(1..=10_000).contains(&b.max_steps)
            || !(1..=1_000_000).contains(&b.max_stage_visits)
            || !(1..=100_000_000).contains(&b.max_matrix_terms)
            || !b.relative_tail_tolerance.is_finite()
            || !(0.0..=1e-2).contains(&b.relative_tail_tolerance)
        {
            return Err(invalid("network:transport_budget"));
        }
        let ids: BTreeSet<_> = self.stages.iter().map(|s| s.id.as_str()).collect();
        if ids.len() != self.stages.len() || ids.iter().any(|id| !valid_id(id)) {
            return Err(invalid("network:stage_ids"));
        }
        let mut terms = 0usize;
        for s in &self.stages {
            if s.routes.is_empty() || s.routes.len() > 32 {
                return Err(invalid("network:route_count"));
            }
            terms += s.routes.len() * bins * bins;
            if terms > 1_000_000 {
                return Err(invalid("network:kernel_size_budget"));
            }
            let mut route_ids = BTreeSet::new();
            let mut liquid = 0.0;
            let mut oil = vec![0.0; bins];
            for r in &s.routes {
                if !valid_id(&r.id) || !route_ids.insert(&r.id) {
                    return Err(invalid("network:route_ids"));
                }
                if let Destination::Stage { stage_id } = &r.destination {
                    if !ids.contains(stage_id.as_str()) {
                        return Err(invalid("network:missing_destination"));
                    }
                }
                if !finite_nonnegative(r.liquid_fraction) || r.liquid_fraction > 1.0 {
                    return Err(invalid("network:liquid_fraction"));
                }
                liquid += r.liquid_fraction;
                if r.oil_mass_transfer.len() != bins {
                    return Err(invalid("network:inlet_bin_count"));
                }
                for (i, row) in r.oil_mass_transfer.iter().enumerate() {
                    if row.len() != bins || row.iter().any(|x| !finite_nonnegative(*x) || *x > 1.0)
                    {
                        return Err(invalid("network:oil_mass_transfer"));
                    }
                    if r.liquid_fraction == 0.0
                        && row.iter().any(|x| *x > 0.0)
                        && !matches!(r.destination, Destination::Retained | Destination::Wall)
                    {
                        return Err(invalid("network:transported_oil_without_liquid"));
                    }
                    oil[i] += row.iter().sum::<f64>();
                }
            }
            if (liquid - 1.0).abs() > CONSERVATION_EPS
                || oil.iter().any(|x| (*x - 1.0).abs() > CONSERVATION_EPS)
            {
                return Err(invalid("network:nonconservative_stage"));
            }
        }
        let mut reachable = BTreeSet::new();
        for f in &self.feeds {
            if !ids.contains(f.stage_id.as_str())
                || !f.cohort.liquid_volume_m3.is_finite()
                || f.cohort.liquid_volume_m3 <= 0.0
                || f.cohort.oil_mass_kg.len() != bins
                || f.cohort.oil_mass_kg.iter().any(|x| !finite_nonnegative(*x))
            {
                return Err(invalid("network:feed"));
            }
            reachable.insert(f.stage_id.as_str());
        }
        let by_id: BTreeMap<_, _> = self.stages.iter().map(|s| (s.id.as_str(), s)).collect();
        loop {
            let before = reachable.len();
            for id in reachable.clone() {
                for r in &by_id[id].routes {
                    if r.liquid_fraction > 0.0 {
                        if let Destination::Stage { stage_id } = &r.destination {
                            reachable.insert(stage_id);
                        }
                    }
                }
            }
            if before == reachable.len() {
                break;
            }
        }
        if reachable.len() != ids.len() {
            return Err(invalid("network:unreachable_stage"));
        }
        Ok(())
    }
}
