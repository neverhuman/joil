use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Mutation,
    Crossover,
    Repair,
    Novelty,
    LocalSearch,
    Restart,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParentSource {
    Qualified,
    Screen,
    Exploration,
}

pub const OPERATORS: [(Operator, u32); 6] = [
    (Operator::Mutation, 40),
    (Operator::Crossover, 20),
    (Operator::Repair, 10),
    (Operator::Novelty, 10),
    (Operator::LocalSearch, 10),
    (Operator::Restart, 10),
];
pub const PARENT_SOURCES: [(ParentSource, u32); 3] = [
    (ParentSource::Qualified, 50),
    (ParentSource::Screen, 30),
    (ParentSource::Exploration, 20),
];

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EmitterState {
    pub operator_counts: [u64; 6],
    pub source_counts: [u64; 3],
    pub next_proposal_id: u64,
    pub migration_events: u64,
}

/// Fixed-dimension covariance-adapting control on a normalized [0,1]^d box.
/// The caller charges every `ask` and `tell` inference through the shared ledger.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FixedDimensionCmaEs {
    pub mean: Vec<f64>,
    pub covariance: Vec<Vec<f64>>,
    pub sigma: f64,
    pub population: usize,
    pub rng_state: u64,
    pub best: Option<f64>,
}
impl FixedDimensionCmaEs {
    pub fn new(dimension: usize, population: usize, seed: u64) -> Result<Self, &'static str> {
        if dimension == 0 || population < 2 {
            return Err("CMA-ES requires dimension>0 and population>=2");
        }
        Ok(Self {
            mean: vec![0.5; dimension],
            covariance: (0..dimension)
                .map(|i| {
                    (0..dimension)
                        .map(|j| if i == j { 1.0 } else { 0.0 })
                        .collect()
                })
                .collect(),
            sigma: 0.2,
            population,
            rng_state: seed.max(1),
            best: None,
        })
    }
    fn uniform(&mut self) -> f64 {
        let mut x = self.rng_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        let u1 = self.uniform().max(f64::MIN_POSITIVE);
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
    pub fn ask(&mut self) -> Vec<Vec<f64>> {
        let n = self.mean.len();
        let l = cholesky(&self.covariance);
        (0..self.population)
            .map(|_| {
                let z: Vec<_> = (0..n).map(|_| self.normal()).collect();
                (0..n)
                    .map(|i| {
                        (self.mean[i] + self.sigma * (0..=i).map(|j| l[i][j] * z[j]).sum::<f64>())
                            .clamp(0.0, 1.0)
                    })
                    .collect()
            })
            .collect()
    }
    pub fn tell(&mut self, points: &[Vec<f64>], scores: &[f64]) -> Result<(), &'static str> {
        if points.len() != scores.len()
            || points.len() < 2
            || points.iter().any(|p| p.len() != self.mean.len())
            || points
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || scores.iter().any(|v| !v.is_finite())
        {
            return Err("CMA-ES batch shape mismatch");
        }
        let mut order: Vec<usize> = (0..scores.len()).collect();
        order.sort_by(|a, b| scores[*a].total_cmp(&scores[*b]).then(a.cmp(b)));
        let mu = (points.len() / 2).max(1);
        let next: Vec<f64> = (0..self.mean.len())
            .map(|j| order[..mu].iter().map(|i| points[*i][j]).sum::<f64>() / mu as f64)
            .collect();
        let n = self.mean.len();
        let mut cov = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                cov[i][j] = 0.8 * self.covariance[i][j]
                    + 0.2 / order[..mu].len() as f64
                        * order[..mu]
                            .iter()
                            .map(|k| {
                                (points[*k][i] - next[i]) * (points[*k][j] - next[j])
                                    / (self.sigma * self.sigma)
                            })
                            .sum::<f64>();
            }
            cov[i][i] += 1e-8;
        }
        let improved =
            scores[order[0]].is_finite() && self.best.is_none_or(|b| scores[order[0]] < b);
        if scores[order[0]].is_finite() {
            self.best = Some(scores[order[0]]);
        }
        self.mean = next;
        self.covariance = cov;
        self.sigma = (self.sigma * if improved { 0.95 } else { 1.03 }).clamp(0.01, 0.5);
        Ok(())
    }
}

/// Gaussian-process expected-improvement control with an RBF kernel on [0,1]^d.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FixedDimensionBo {
    pub dimension: usize,
    pub observations: Vec<(Vec<f64>, f64)>,
    pub length_scale: f64,
    pub exploration: f64,
}
impl FixedDimensionBo {
    pub fn new(dimension: usize) -> Result<Self, &'static str> {
        if dimension == 0 {
            return Err("BO dimension must be positive");
        }
        Ok(Self {
            dimension,
            observations: Vec::new(),
            length_scale: 0.2,
            exploration: 0.01,
        })
    }
    pub fn observe(&mut self, x: Vec<f64>, score: f64) -> Result<(), &'static str> {
        if x.len() != self.dimension || x.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0) {
            return Err("BO point outside fixed dimension/domain");
        }
        if score.is_finite() {
            self.observations.push((x, score));
        }
        Ok(())
    }
    pub fn select(&self, candidates: &[Vec<f64>]) -> Result<Vec<f64>, &'static str> {
        if candidates.is_empty()
            || candidates.iter().any(|x| {
                x.len() != self.dimension
                    || x.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0)
            })
        {
            return Err("invalid BO candidate set");
        }
        if self.observations.len() < 2 {
            return Ok(candidates[0].clone());
        }
        let n = self.observations.len();
        let mut k = vec![vec![0.0; n]; n];
        for (i, row) in k.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = rbf(
                    &self.observations[i].0,
                    &self.observations[j].0,
                    self.length_scale,
                );
                if i == j {
                    *value += 1e-8;
                }
            }
        }
        let y: Vec<f64> = self.observations.iter().map(|x| x.1).collect();
        let alpha = solve(k.clone(), y)?;
        let best = self
            .observations
            .iter()
            .map(|x| x.1)
            .fold(f64::INFINITY, f64::min);
        candidates
            .iter()
            .max_by(|a, b| {
                ei(a, &self.observations, &k, &alpha, best, self).total_cmp(&ei(
                    b,
                    &self.observations,
                    &k,
                    &alpha,
                    best,
                    self,
                ))
            })
            .cloned()
            .ok_or("empty BO set")
    }
}
fn rbf(a: &[f64], b: &[f64], l: f64) -> f64 {
    (-a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f64>() / (2.0 * l * l)).exp()
}
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Result<Vec<f64>, &'static str> {
    let n = b.len();
    for i in 0..n {
        let p = (i..n)
            .max_by(|x, y| a[*x][i].abs().total_cmp(&a[*y][i].abs()))
            .unwrap();
        if a[p][i].abs() < 1e-14 {
            return Err("singular GP covariance");
        }
        a.swap(i, p);
        b.swap(i, p);
        let d = a[i][i];
        for value in a[i].iter_mut().skip(i) {
            *value /= d;
        }
        b[i] /= d;
        for r in 0..n {
            if r != i {
                let f = a[r][i];
                let pivot = a[i].clone();
                for (value, pivot_value) in a[r].iter_mut().skip(i).zip(pivot.iter().skip(i)) {
                    *value -= f * pivot_value;
                }
                b[r] -= f * b[i];
            }
        }
    }
    Ok(b)
}
fn ei(
    x: &[f64],
    obs: &[(Vec<f64>, f64)],
    k: &[Vec<f64>],
    alpha: &[f64],
    best: f64,
    bo: &FixedDimensionBo,
) -> f64 {
    let kv: Vec<f64> = obs
        .iter()
        .map(|(p, _)| rbf(x, p, bo.length_scale))
        .collect();
    let mean = kv.iter().zip(alpha).map(|(a, b)| a * b).sum::<f64>();
    let v = solve(k.to_vec(), kv.clone())
        .map(|z| 1.0 - kv.iter().zip(z).map(|(a, b)| a * b).sum::<f64>())
        .unwrap_or(0.0)
        .max(1e-12)
        .sqrt();
    let improvement = best - mean - bo.exploration;
    let z = improvement / v;
    let cdf = normal_cdf(z);
    let pdf = (-0.5 * z * z).exp() / std::f64::consts::TAU.sqrt();
    improvement * cdf + v * pdf
}
fn normal_cdf(x: f64) -> f64 {
    let z = x.abs() / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + 0.3275911 * z);
    let erf = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-z * z).exp();
    if x >= 0.0 {
        0.5 * (1.0 + erf)
    } else {
        0.5 * (1.0 - erf)
    }
}
fn cholesky(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let s = (0..j).map(|k| l[i][k] * l[j][k]).sum::<f64>();
            l[i][j] = if i == j {
                (a[i][i] - s).max(1e-12).sqrt()
            } else {
                (a[i][j] - s) / l[j][j]
            };
        }
    }
    l
}

impl EmitterState {
    pub fn charge(&mut self, operator: Operator, source: ParentSource) -> u64 {
        let oi = OPERATORS.iter().position(|x| x.0 == operator).unwrap();
        let si = PARENT_SOURCES.iter().position(|x| x.0 == source).unwrap();
        self.operator_counts[oi] += 1;
        self.source_counts[si] += 1;
        let id = self.next_proposal_id;
        self.next_proposal_id += 1;
        id
    }
    /// Deterministic weighted schedule with stable tie ordering; one attempt per call.
    pub fn scheduled_operator(attempt: u64) -> Operator {
        Self::scheduled_operator_with(attempt, [40, 20, 10, 10, 10, 10])
    }
    pub fn scheduled_operator_with(attempt: u64, weights: [u32; 6]) -> Operator {
        balanced_weighted_index(
            attempt % 128,
            &OPERATORS.map(|(op, _)| (op, weights[operator_index(op)])),
        )
    }
    pub fn scheduled_source(attempt: u64, available: [bool; 3]) -> Option<ParentSource> {
        Self::scheduled_source_with(attempt, available, [50, 30, 20])
    }
    pub fn scheduled_source_with(
        attempt: u64,
        available: [bool; 3],
        weights: [u32; 3],
    ) -> Option<ParentSource> {
        let selected = balanced_weighted_index(
            attempt % 128,
            &PARENT_SOURCES.map(|(src, _)| (src, weights[source_index(src)])),
        );
        if available[source_index(selected)] {
            return Some(selected);
        }
        // Explicit stable fallback. No synthetic/baseline parent is ever constructed.
        [
            ParentSource::Qualified,
            ParentSource::Screen,
            ParentSource::Exploration,
        ]
        .into_iter()
        .find(|s| available[source_index(*s)])
    }
    pub fn migration_ring(&mut self, epoch: u64, islands: &[Vec<u64>]) -> Vec<(usize, usize, u64)> {
        if epoch == 0 || !epoch.is_multiple_of(10) || islands.len() < 2 {
            return Vec::new();
        }
        self.migration_events += 1;
        islands
            .iter()
            .enumerate()
            .filter_map(|(i, source)| source.first().map(|id| (i, (i + 1) % islands.len(), *id)))
            .collect()
    }
}

fn source_index(s: ParentSource) -> usize {
    match s {
        ParentSource::Qualified => 0,
        ParentSource::Screen => 1,
        ParentSource::Exploration => 2,
    }
}
fn operator_index(o: Operator) -> usize {
    match o {
        Operator::Mutation => 0,
        Operator::Crossover => 1,
        Operator::Repair => 2,
        Operator::Novelty => 3,
        Operator::LocalSearch => 4,
        Operator::Restart => 5,
    }
}
fn balanced_weighted_index<T: Copy>(attempt: u64, values: &[(T, u32)]) -> T {
    let mut counts = vec![0u64; values.len()];
    for step in 0..=attempt {
        let selected = (0..values.len())
            .max_by(|a, b| {
                let da = ((step + 1) * values[*a].1 as u64) as i128 - (counts[*a] * 100) as i128;
                let db = ((step + 1) * values[*b].1 as u64) as i128 - (counts[*b] * 100) as i128;
                da.cmp(&db).then_with(|| b.cmp(a))
            })
            .unwrap();
        if step == attempt {
            return values[selected].0;
        }
        counts[selected] += 1;
    }
    unreachable!()
}
