//! Equal-budget random-search control. It draws genomes from the same sampler that fills
//! generation 0 and scores them with the same evaluator, so a run can be compared with
//! plain sampling at the same number of evaluations.

use crate::archive::Archive;
use crate::campaign::Campaign;
use crate::evolve::{evaluate_batch, Cache};
use crate::random::random_genome;
use crate::rng::Rng;
use crate::score::Evaluator;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlPoint {
    pub evaluations: u64,
    pub best_fitness: Option<f64>,
    pub best_q_treated_m3_h: f64,
    pub feasible_so_far: u64,
    pub niches: usize,
    pub feasible_niches: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlReport {
    pub kind: String,
    pub campaign_sha256: String,
    pub evaluations: u64,
    pub feasible: u64,
    pub trajectory: Vec<ControlPoint>,
    pub best: Option<crate::evolve::Individual>,
    pub note: String,
}

/// Score `budget` random genomes in batches of `batch` (one batch per trajectory point).
pub fn random_search(c: &Campaign, budget: u64, batch: usize) -> ControlReport {
    let ev = Evaluator::new(c);
    let mut cache = Cache::default();
    let mut rng = Rng::derive(c.seed, &[7]);
    let mut next_id = 1000;
    let mut archive = Archive::default();
    let (mut done, mut feasible) = (0u64, 0u64);
    let mut best: Option<crate::evolve::Individual> = None;
    let mut trajectory = Vec::new();
    while done < budget {
        let n = (batch as u64).min(budget - done) as usize;
        let items = (0..n)
            .map(|_| {
                let g = random_genome(&mut rng, next_id);
                next_id += 16;
                (g, vec![], vec!["random".to_string()])
            })
            .collect();
        let (pop, fresh) = evaluate_batch(&ev, &mut cache, items, 0);
        done += fresh;
        for ind in pop {
            feasible += u64::from(ind.score.feasible);
            archive.offer(&ind);
            let wins = best.as_ref().is_none_or(|b| {
                ind.score
                    .compare(&b.score)
                    .then_with(|| ind.hash.cmp(&b.hash))
                    .is_lt()
            });
            if wins {
                best = Some(ind);
            }
        }
        let b = best.as_ref().map(|b| &b.score);
        trajectory.push(ControlPoint {
            evaluations: done,
            best_fitness: b.filter(|s| s.feasible).map(|s| s.fitness),
            best_q_treated_m3_h: b.map(|s| s.q_treated_m3_h).unwrap_or(0.0),
            feasible_so_far: feasible,
            niches: archive.cells.len(),
            feasible_niches: archive.feasible(),
        });
    }
    ControlReport {
        kind: "equal_budget_random_search".into(),
        campaign_sha256: c.sha256(),
        evaluations: done,
        feasible,
        trajectory,
        best,
        note: "Screen-model search control, not physical evidence.".into(),
    }
}
