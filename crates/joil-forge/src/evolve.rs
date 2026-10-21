//! Island evolution with a MAP-Elites archive.
//!
//! Each generation, every island breeds `population` offspring (tournament or archive
//! parents, homologous crossover, structural and parametric mutation), evaluates them in
//! parallel, and keeps the best `population` of parents ∪ offspring with a cap on any
//! single behaviour niche. Islands exchange their best designs in a ring. Evaluation is
//! pure and offspring are bred sequentially from per-island streams, so results are
//! independent of the thread count and a resumed run continues bit-for-bit.

use crate::archive::{niche, Archive};
use crate::campaign::Seeding;
use crate::crossover::crossover;
use crate::develop::Behavior;
use crate::genome::Genome;
use crate::mutate::mutate;
use crate::random::{archetypes, random_genome};
use crate::rng::Rng;
use crate::score::{Evaluator, Score};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Individual {
    pub genome: Genome,
    pub hash: String,
    pub score: Score,
    pub behavior: Option<Behavior>,
    pub born: usize,
    pub parents: Vec<String>,
    pub ops: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Island {
    pub pop: Vec<Individual>,
    pub rng: Rng,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenStats {
    pub generation: usize,
    pub evaluations: u64,
    pub feasible_fraction: f64,
    pub best_feasible: bool,
    pub best_fitness: Option<f64>,
    pub best_violation: f64,
    pub best_q_treated_m3_h: f64,
    pub best_cout_mg_l: Option<f64>,
    pub median_violation: f64,
    pub archive_niches: usize,
    pub archive_feasible: usize,
    pub best_hash: String,
    pub wall_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunState {
    pub generation: usize,
    pub next_id: u32,
    pub evaluations: u64,
    pub islands: Vec<Island>,
    pub archive: Archive,
    pub best: Option<Individual>,
    pub history: Vec<GenStats>,
}

/// Memo of evaluations by canonical hash (pure, so results never depend on it).
#[derive(Default)]
pub struct Cache(HashMap<String, (Score, Option<Behavior>)>);

fn better(a: &Individual, b: &Individual) -> bool {
    a.score
        .compare(&b.score)
        .then_with(|| a.hash.cmp(&b.hash))
        .is_lt()
}

fn sort(pop: &mut [Individual]) {
    pop.sort_by(|a, b| a.score.compare(&b.score).then_with(|| a.hash.cmp(&b.hash)));
}

/// Evaluate genomes in parallel, preserving order.
pub fn evaluate_batch(
    ev: &Evaluator,
    cache: &mut Cache,
    items: Vec<(Genome, Vec<String>, Vec<String>)>,
    born: usize,
) -> (Vec<Individual>, u64) {
    let hashes: Vec<String> = items.iter().map(|(g, _, _)| g.canonical_sha256()).collect();
    let mut todo: Vec<(String, Genome)> = Vec::new();
    let mut queued = HashSet::new();
    for (h, (g, _, _)) in hashes.iter().zip(&items) {
        if !cache.0.contains_key(h) && queued.insert(h.clone()) {
            todo.push((h.clone(), g.clone()));
        }
    }
    // Count scored proposals (cache hits included) so the tally never depends on memo state.
    let fresh = items.len() as u64;
    let results: Vec<(String, (Score, Option<Behavior>))> = todo
        .into_par_iter()
        .map(|(h, g)| (h, ev.evaluate(&g)))
        .collect();
    for (h, r) in results {
        cache.0.insert(h, r);
    }
    let out = items
        .into_iter()
        .zip(hashes)
        .map(|((genome, parents, ops), hash)| {
            let (score, behavior) = cache.0[&hash].clone();
            Individual {
                genome,
                hash,
                score,
                behavior,
                born,
                parents,
                ops,
            }
        })
        .collect();
    (out, fresh)
}

fn tournament<'a>(pop: &'a [Individual], k: usize, r: &mut Rng) -> &'a Individual {
    let mut best = &pop[r.below(pop.len())];
    for _ in 1..k.max(1) {
        let c = &pop[r.below(pop.len())];
        if better(c, best) {
            best = c;
        }
    }
    best
}

/// Keep `mu` survivors: unique genomes, ranked, at most a third from one niche.
fn survive(mut all: Vec<Individual>, mu: usize) -> Vec<Individual> {
    sort(&mut all);
    let mut seen = HashSet::new();
    all.retain(|i| seen.insert(i.hash.clone()));
    let cap = (mu / 3).max(2);
    let mut per: BTreeMap<String, usize> = BTreeMap::new();
    let (mut keep, mut rest) = (Vec::new(), Vec::new());
    for ind in all {
        let key = ind
            .behavior
            .as_ref()
            .map(niche)
            .unwrap_or_else(|| "none".into());
        let n = per.entry(key).or_default();
        if *n < cap && keep.len() < mu {
            *n += 1;
            keep.push(ind);
        } else {
            rest.push(ind);
        }
    }
    for ind in rest {
        if keep.len() >= mu {
            break;
        }
        keep.push(ind);
    }
    sort(&mut keep);
    keep
}

fn update_best(state: &mut RunState, cands: &[Individual]) {
    for c in cands {
        if state.best.as_ref().is_none_or(|b| better(c, b)) {
            state.best = Some(c.clone());
        }
    }
}

/// Initial state: seeds (e.g. elites of a previous run) + optional archetype mutants + random.
pub fn init(ev: &Evaluator, cache: &mut Cache, seeds: Vec<Genome>) -> RunState {
    let c = &ev.campaign;
    let mut next_id = seeds.iter().map(|g| g.max_id()).max().unwrap_or(0) + 1000;
    let mut islands = Vec::with_capacity(c.islands);
    let mut evaluations = 0;
    let mut archive = Archive::default();
    let mut all = Vec::new();
    for i in 0..c.islands {
        let mut rng = Rng::derive(c.seed, &[1, i as u64]);
        let mut items = Vec::new();
        for (k, s) in seeds.iter().enumerate() {
            if k % c.islands == i && items.len() < c.population {
                items.push((s.clone().normalized(), vec![], vec!["seed".to_string()]));
            }
        }
        if c.seeding == Seeding::Archetypes {
            for (name, a) in archetypes() {
                if items.len() >= c.population {
                    break;
                }
                items.push((a.clone(), vec![], vec![format!("archetype:{name}")]));
                if items.len() < c.population {
                    let (m, ops) = mutate(&a, &mut rng, &mut next_id, c.structural_rate);
                    items.push((m, vec![a.canonical_sha256()], ops));
                }
            }
        }
        while items.len() < c.population {
            let g = random_genome(&mut rng, next_id);
            next_id += 16;
            items.push((g, vec![], vec!["random".to_string()]));
        }
        let (pop, fresh) = evaluate_batch(ev, cache, items, 0);
        evaluations += fresh;
        for ind in &pop {
            archive.offer(ind);
        }
        all.extend(pop.iter().cloned());
        islands.push(Island {
            pop: survive(pop, c.population),
            rng,
        });
    }
    let mut state = RunState {
        generation: 0,
        next_id,
        evaluations,
        islands,
        archive,
        best: None,
        history: Vec::new(),
    };
    update_best(&mut state, &all);
    state
}

/// Advance one generation.
pub fn step(ev: &Evaluator, cache: &mut Cache, state: &mut RunState) {
    let c = &ev.campaign;
    let started = std::time::Instant::now();
    let generation = state.generation + 1;
    let elites: Vec<Individual> = state.archive.elites().into_iter().cloned().collect();
    let mut offspring_all = Vec::new();
    for isl in 0..state.islands.len() {
        let mut items = Vec::with_capacity(c.population);
        {
            let island = &mut state.islands[isl];
            let r = &mut island.rng;
            while items.len() < c.population {
                let p1 = if !elites.is_empty() && r.chance(c.archive_parent_rate) {
                    elites[r.below(elites.len())].clone()
                } else {
                    tournament(&island.pop, c.tournament, r).clone()
                };
                let mut parents = vec![p1.hash.clone()];
                let mut ops = Vec::new();
                let mut base = p1.genome.clone();
                if r.chance(c.crossover_rate) {
                    let p2 = tournament(&island.pop, c.tournament, r);
                    if p2.hash != p1.hash {
                        base = if better(&p1, p2) {
                            crossover(&p1.genome, &p2.genome, r)
                        } else {
                            crossover(&p2.genome, &p1.genome, r)
                        };
                        parents.push(p2.hash.clone());
                        ops.push("crossover".to_string());
                    }
                }
                let (child, mops) = mutate(&base, r, &mut state.next_id, c.structural_rate);
                ops.extend(mops);
                items.push((child, parents, ops));
            }
        }
        let (offspring, fresh) = evaluate_batch(ev, cache, items, generation);
        state.evaluations += fresh;
        for ind in &offspring {
            state.archive.offer(ind);
        }
        offspring_all.extend(offspring.iter().cloned());
        let island = &mut state.islands[isl];
        let mut all = std::mem::take(&mut island.pop);
        all.extend(offspring);
        island.pop = survive(all, c.population);
    }
    if state.islands.len() > 1 && c.migrate_every > 0 && generation.is_multiple_of(c.migrate_every)
    {
        let n = state.islands.len();
        let movers: Vec<Vec<Individual>> = state
            .islands
            .iter()
            .map(|i| i.pop.iter().take(c.migrants).cloned().collect())
            .collect();
        for (i, m) in movers.into_iter().enumerate() {
            let dst = &mut state.islands[(i + 1) % n];
            let mut all = std::mem::take(&mut dst.pop);
            all.extend(m);
            dst.pop = survive(all, c.population);
        }
    }
    update_best(state, &offspring_all);
    state.generation = generation;
    let pop: Vec<&Individual> = state.islands.iter().flat_map(|i| i.pop.iter()).collect();
    let feasible = pop.iter().filter(|i| i.score.feasible).count();
    let mut viol: Vec<f64> = pop.iter().map(|i| i.score.violation).collect();
    viol.sort_by(f64::total_cmp);
    let best = state.best.as_ref().expect("population is never empty");
    state.history.push(GenStats {
        generation,
        evaluations: state.evaluations,
        feasible_fraction: feasible as f64 / pop.len().max(1) as f64,
        best_feasible: best.score.feasible,
        best_fitness: best.score.feasible.then_some(best.score.fitness),
        best_violation: best.score.violation,
        best_q_treated_m3_h: best.score.q_treated_m3_h,
        best_cout_mg_l: best.score.cout_mg_l,
        median_violation: viol.get(viol.len() / 2).copied().unwrap_or(0.0),
        archive_niches: state.archive.cells.len(),
        archive_feasible: state.archive.feasible(),
        best_hash: best.hash.clone(),
        wall_s: started.elapsed().as_secs_f64(),
    });
}
