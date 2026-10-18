//! Fixed, deterministic multi-fidelity allocation. This module allocates attempts only.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arm {
    Improvement,
    Uncertainty,
    Diversity,
    Random,
    RankCrowding,
    EligibleRandom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub arm: Arm,
    pub cost: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Allocation {
    pub policy: String,
    pub shares: Vec<(Arm, usize)>,
    pub selected: Vec<Proposal>,
    pub charged_cost: u64,
}

/// Floors shares, then awards remaining slots by descending fractional remainder and fixed arm order.
pub fn largest_remainder(
    batch: usize,
    weights: &[(Arm, u32)],
) -> Result<Vec<(Arm, usize)>, &'static str> {
    if weights.is_empty() || weights.iter().any(|(_, w)| *w == 0) {
        return Err("shares must be nonempty and positive");
    }
    let total: u64 = weights.iter().map(|(_, w)| *w as u64).sum();
    let mut rows: Vec<_> = weights
        .iter()
        .enumerate()
        .map(|(i, (arm, w))| {
            let product = batch as u128 * *w as u128;
            (
                i,
                *arm,
                (product / total as u128) as usize,
                product % total as u128,
            )
        })
        .collect();
    let assigned: usize = rows.iter().map(|r| r.2).sum();
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|a, b| {
        rows[*b]
            .3
            .cmp(&rows[*a].3)
            .then(rows[*a].0.cmp(&rows[*b].0))
    });
    for i in order.into_iter().take(batch - assigned) {
        rows[i].2 += 1;
    }
    Ok(rows.into_iter().map(|(_, arm, n, _)| (arm, n)).collect())
}

/// Warmup is direct evaluation only. Thereafter fixed shares apply; unavailable categories spill
/// forward in canonical order, and every accepted proposal is unique and fully affordable.
pub fn allocate(
    batch: usize,
    direct_evaluations: usize,
    warmup: usize,
    calibrated: bool,
    budget: u64,
    candidates: &[Proposal],
) -> Result<Allocation, &'static str> {
    let (policy, weights) = if direct_evaluations < warmup {
        ("direct_warmup", vec![(Arm::Random, 100)])
    } else if calibrated {
        (
            "calibrated_v1",
            vec![
                (Arm::Improvement, 60),
                (Arm::Uncertainty, 20),
                (Arm::Diversity, 10),
                (Arm::Random, 10),
            ],
        )
    } else {
        (
            "uncalibrated_v1",
            vec![(Arm::RankCrowding, 50), (Arm::EligibleRandom, 50)],
        )
    };
    let shares = largest_remainder(batch, &weights)?;
    let mut remaining = budget;
    let mut used = BTreeSet::new();
    let mut selected = Vec::new();
    for (arm, quota) in &shares {
        let mut added = 0;
        for c in candidates.iter().filter(|c| c.arm == *arm) {
            if added == *quota {
                break;
            }
            if used.contains(&c.id) || c.cost > remaining {
                continue;
            }
            remaining -= c.cost;
            used.insert(c.id);
            selected.push(c.clone());
            added += 1;
        }
    }
    // Unfilled slots remain uncommitted; an empty arm cannot transfer preference to another emitter.
    let charged_cost = budget - remaining;
    Ok(Allocation {
        policy: policy.into(),
        shares,
        selected,
        charged_cost,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn odd_batch_and_empty_categories_are_explicit_and_affordable() {
        assert_eq!(
            largest_remainder(
                5,
                &[
                    (Arm::Improvement, 60),
                    (Arm::Uncertainty, 20),
                    (Arm::Diversity, 10),
                    (Arm::Random, 10)
                ]
            )
            .unwrap(),
            vec![
                (Arm::Improvement, 3),
                (Arm::Uncertainty, 1),
                (Arm::Diversity, 1),
                (Arm::Random, 0)
            ]
        );
        let cs = vec![
            Proposal {
                id: 1,
                arm: Arm::Improvement,
                cost: 9,
            },
            Proposal {
                id: 1,
                arm: Arm::Improvement,
                cost: 1,
            },
            Proposal {
                id: 2,
                arm: Arm::Random,
                cost: 9,
            },
        ];
        let a = allocate(5, 10, 1, true, 5, &cs).unwrap();
        assert_eq!(a.selected.len(), 1);
        assert_eq!(a.selected[0].cost, 1);
        assert_eq!(a.charged_cost, 1);
        let w = allocate(3, 0, 2, false, 100, &cs).unwrap();
        assert_eq!(w.selected.len(), 1);
        assert_eq!(w.policy, "direct_warmup");
    }
}
