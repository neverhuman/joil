use crate::archive::{ArchiveConfig, Candidate, SearchArchive};
use crate::emitters::{EmitterState, Operator, ParentSource};
use serde::{Deserialize, Serialize};

pub const POLICY_TOML: &str = include_str!("../../../policies/reference-search-v1.toml");

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    pub policy_id: String,
    pub islands: usize,
    pub population_per_island: usize,
    pub attempted_offspring_per_island_epoch: usize,
    pub migration_interval_epochs: usize,
    pub protection_lifetime_epochs: usize,
    pub initialization_attempt_limit_per_slot: usize,
    pub refill_attempt_limit_per_epoch_slot: usize,
    pub archive_capacity_per_pool: usize,
    pub nondominated_capacity_per_descriptor_bin: usize,
    pub operator_shares: Shares,
    pub parent_source_shares: SourceShares,
    pub controls: Controls,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shares {
    pub mutation: u32,
    pub crossover: u32,
    pub repair: u32,
    pub novelty: u32,
    pub local_search: u32,
    pub restart: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceShares {
    pub qualified: u32,
    pub screen: u32,
    pub exploration: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Controls {
    pub include_random: bool,
    pub include_cma_es_fixed_dimension: bool,
    pub include_bayesian_optimization_fixed_dimension: bool,
    pub shared_initialization_sampler: bool,
    pub shared_attempt_budget: bool,
    pub charge_initialization: bool,
    pub charge_failed_proposals: bool,
    pub charge_inference: bool,
    pub charge_audits: bool,
    pub charge_confirmation: bool,
}

impl Policy {
    pub fn reference_v1() -> Result<Self, String> {
        let p: Self = toml::from_str(POLICY_TOML).map_err(|e| e.to_string())?;
        p.validate()?;
        Ok(p)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.policy_id != "reference-search/1"
            || self.islands != 4
            || self.population_per_island != 32
            || self.attempted_offspring_per_island_epoch != 32
            || self.migration_interval_epochs != 10
        {
            return Err("reference policy constants changed".into());
        }
        let op = [
            self.operator_shares.mutation,
            self.operator_shares.crossover,
            self.operator_shares.repair,
            self.operator_shares.novelty,
            self.operator_shares.local_search,
            self.operator_shares.restart,
        ];
        let src = [
            self.parent_source_shares.qualified,
            self.parent_source_shares.screen,
            self.parent_source_shares.exploration,
        ];
        if op.iter().sum::<u32>() != 100
            || src.iter().sum::<u32>() != 100
            || op.contains(&0)
            || src.contains(&0)
            || op != [40, 20, 10, 10, 10, 10]
            || src != [50, 30, 20]
        {
            return Err("invalid fixed shares".into());
        }
        let c = &self.controls;
        if !(c.include_random
            && c.include_cma_es_fixed_dimension
            && c.include_bayesian_optimization_fixed_dimension
            && c.shared_initialization_sampler
            && c.shared_attempt_budget
            && c.charge_initialization
            && c.charge_failed_proposals
            && c.charge_inference
            && c.charge_audits
            && c.charge_confirmation)
        {
            return Err(
                "reference controls must share sampler/budget and charge all stages".into(),
            );
        }
        if self.initialization_attempt_limit_per_slot == 0
            || self.refill_attempt_limit_per_epoch_slot == 0
            || self.archive_capacity_per_pool == 0
            || self.nondominated_capacity_per_descriptor_bin == 0
        {
            return Err("policy bounds must be positive".into());
        }
        Ok(())
    }
    pub fn archive_config(&self, descriptor_bin_widths: Vec<f64>) -> ArchiveConfig {
        ArchiveConfig {
            capacity_per_pool: self.archive_capacity_per_pool,
            nondominated_capacity_per_descriptor_bin: self.nondominated_capacity_per_descriptor_bin,
            descriptor_bin_widths,
            protection_lifetime_epochs: self.protection_lifetime_epochs as u64,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AttemptKind {
    Initialization,
    Offspring,
    Refill,
    Inference,
    Audit,
    Confirmation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub proposal_id: u64,
    pub island: usize,
    pub epoch: usize,
    pub kind: AttemptKind,
    pub operator: Option<Operator>,
    pub source: Option<ParentSource>,
    pub outcome: String,
    pub charged_units: u64,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    pub limit: u64,
    pub charged: u64,
    pub records: Vec<AttemptRecord>,
}
impl Budget {
    pub fn charge(&mut self, rec: AttemptRecord) -> Result<(), &'static str> {
        let next = self
            .charged
            .checked_add(rec.charged_units)
            .filter(|value| *value <= self.limit)
            .ok_or("attempt budget exhausted")?;
        self.charged = next;
        self.records.push(rec);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub seed: u64,
    pub epoch: usize,
    pub epoch_progress: usize,
    pub rng_state: u64,
    pub next_proposal_id: u64,
    pub operator_shares: [u32; 6],
    pub parent_source_shares: [u32; 3],
    pub emitters: Vec<EmitterState>,
    pub island_members: Vec<Vec<u64>>,
    pub archive: SearchArchive,
    pub budget: Budget,
    pub refill_attempts: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CostEvent {
    pub representation: String,
    pub optimizer: String,
    pub kind: AttemptKind,
    pub quality: Option<f64>,
    pub charged_units: u64,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CostCurve {
    pub events: Vec<CostEvent>,
}
impl CostCurve {
    pub fn record(&mut self, event: CostEvent) {
        self.events.push(event);
    }
    /// Failures and every downstream evidence stage stay in the cost denominator.
    pub fn points(&self, representation: &str, optimizer: &str) -> Vec<(u64, Option<f64>)> {
        let mut cost = 0;
        self.events
            .iter()
            .filter(|e| e.representation == representation && e.optimizer == optimizer)
            .map(|e| {
                cost += e.charged_units;
                (cost, e.quality.filter(|q| q.is_finite()))
            })
            .collect()
    }
    pub fn same_representation_contrast(&self, representation: &str, a: &str, b: &str) -> bool {
        self.events
            .iter()
            .any(|e| e.representation == representation && e.optimizer == a)
            && self
                .events
                .iter()
                .any(|e| e.representation == representation && e.optimizer == b)
    }
}

impl Checkpoint {
    pub fn new(seed: u64, budget: u64, p: &Policy) -> Self {
        Self {
            seed,
            epoch: 0,
            epoch_progress: 0,
            rng_state: seed,
            next_proposal_id: 0,
            operator_shares: [
                p.operator_shares.mutation,
                p.operator_shares.crossover,
                p.operator_shares.repair,
                p.operator_shares.novelty,
                p.operator_shares.local_search,
                p.operator_shares.restart,
            ],
            parent_source_shares: [
                p.parent_source_shares.qualified,
                p.parent_source_shares.screen,
                p.parent_source_shares.exploration,
            ],
            emitters: vec![EmitterState::default(); p.islands],
            island_members: vec![Vec::new(); p.islands],
            archive: SearchArchive::default(),
            budget: Budget {
                limit: budget,
                ..Budget::default()
            },
            refill_attempts: vec![0; p.islands * p.population_per_island],
        }
    }
    pub fn next_random(&mut self) -> u64 {
        // xorshift64*; checkpoint the full state, not only the initial seed.
        let mut x = self.rng_state.max(1);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    /// Random arm delegates representation construction to the same grammar sampler as search.
    pub fn random_control<T>(&mut self, sampler: impl FnOnce(u64) -> T) -> T {
        sampler(self.next_random())
    }
    pub fn select_parent_source(&self, attempt: u64, available: [bool; 3]) -> Option<ParentSource> {
        EmitterState::scheduled_source_with(attempt, available, self.parent_source_shares)
    }
    pub fn charge_attempt(
        &mut self,
        island: usize,
        epoch: usize,
        kind: AttemptKind,
        outcome: &str,
    ) -> Result<u64, &'static str> {
        let id = self.next_proposal_id;
        let emitter = &self.emitters[island];
        let local_attempt = emitter.operator_counts.iter().sum::<u64>() % 32;
        let operator = if kind == AttemptKind::Offspring || kind == AttemptKind::Refill {
            Some(EmitterState::scheduled_operator_with(
                self.epoch as u64 * 128 + island as u64 * 32 + local_attempt,
                self.operator_shares,
            ))
        } else {
            None
        };
        let source = if operator.is_some() {
            Some(
                EmitterState::scheduled_source_with(
                    self.epoch as u64 * 128 + island as u64 * 32 + local_attempt,
                    [true; 3],
                    self.parent_source_shares,
                )
                .unwrap(),
            )
        } else {
            None
        };
        self.budget.charge(AttemptRecord {
            proposal_id: id,
            island,
            epoch,
            kind,
            operator,
            source,
            outcome: outcome.into(),
            charged_units: 1,
        })?;
        if let (Some(op), Some(src)) = (operator, source) {
            self.emitters[island].charge(op, src);
        } else {
            self.emitters[island].next_proposal_id += 1;
        }
        self.next_proposal_id += 1;
        Ok(id)
    }
    /// Fixed quota: every planned child consumes one attempt, even a failed/inapplicable operator.
    pub fn charge_epoch(&mut self, p: &Policy) -> Result<(), &'static str> {
        let total = p.islands * p.attempted_offspring_per_island_epoch;
        while self.epoch_progress < total {
            let island = self.epoch_progress / p.attempted_offspring_per_island_epoch;
            self.charge_attempt(island, self.epoch, AttemptKind::Offspring, "attempted")?;
            self.epoch_progress += 1;
        }
        self.epoch += 1;
        self.epoch_progress = 0;
        self.archive.expire_protection(self.epoch as u64);
        if self.epoch.is_multiple_of(p.migration_interval_epochs) {
            self.migrate_ring(p.population_per_island);
        }
        Ok(())
    }
    pub fn set_island_members(&mut self, island: usize, ids: Vec<u64>) {
        self.island_members[island] = ids;
    }
    pub fn migrate_ring(&mut self, capacity: usize) {
        if self.epoch == 0 || !self.epoch.is_multiple_of(10) {
            return;
        }
        let moves = self.emitters[0].migration_ring(self.epoch as u64, &self.island_members);
        for (_, destination, id) in moves {
            if !self.island_members[destination].contains(&id) && capacity > 0 {
                if self.island_members[destination].len() >= capacity {
                    self.island_members[destination].pop();
                }
                self.island_members[destination].push(id);
                self.island_members[destination].sort_unstable();
            }
        }
    }
    pub fn bounded_initialization(
        &mut self,
        p: &Policy,
        island: usize,
        slot: usize,
        valid: impl FnMut(usize) -> bool,
    ) -> Result<Option<u64>, &'static str> {
        self.try_bounded(
            island,
            slot,
            p.initialization_attempt_limit_per_slot,
            AttemptKind::Initialization,
            valid,
        )
    }
    pub fn bounded_refill(
        &mut self,
        p: &Policy,
        island: usize,
        slot: usize,
        valid: impl FnMut(usize) -> bool,
    ) -> Result<Option<u64>, &'static str> {
        self.try_bounded(
            island,
            slot,
            p.refill_attempt_limit_per_epoch_slot,
            AttemptKind::Refill,
            valid,
        )
    }
    fn try_bounded(
        &mut self,
        island: usize,
        slot: usize,
        limit: usize,
        kind: AttemptKind,
        mut valid: impl FnMut(usize) -> bool,
    ) -> Result<Option<u64>, &'static str> {
        let mut accepted = None;
        for attempt in 0..limit {
            let id = self.charge_attempt(island, self.epoch, kind.clone(), "attempted")?;
            if valid(attempt) {
                accepted = Some(id);
                break;
            }
        }
        if let Some(x) = self.refill_attempts.get_mut(island * 32 + slot) {
            *x = x.saturating_add(accepted.is_none() as u8);
        }
        Ok(accepted)
    }
    pub fn charge_stage(&mut self, kind: AttemptKind, outcome: &str) -> Result<(), &'static str> {
        if !matches!(
            kind,
            AttemptKind::Inference | AttemptKind::Audit | AttemptKind::Confirmation
        ) {
            return Err("not a post-evaluation stage");
        }
        self.charge_attempt(0, self.epoch, kind, outcome)
            .map(|_| ())
    }
}

pub fn constrained_dominates(a: &[f64], a_violation: f64, b: &[f64], b_violation: f64) -> bool {
    let av = if a_violation.is_finite() {
        a_violation.max(0.0)
    } else {
        f64::INFINITY
    };
    let bv = if b_violation.is_finite() {
        b_violation.max(0.0)
    } else {
        f64::INFINITY
    };
    if av != bv {
        return av < bv;
    }
    if av > 0.0 {
        return av < bv;
    }
    a.len() == b.len()
        && a.iter().all(|x| x.is_finite())
        && b.iter().all(|x| x.is_finite())
        && a.iter().zip(b).all(|(x, y)| x <= y)
        && a.iter().zip(b).any(|(x, y)| x < y)
}

pub fn insert_candidate(checkpoint: &mut Checkpoint, candidate: Candidate, cfg: &ArchiveConfig) {
    checkpoint
        .archive
        .insert(candidate, checkpoint.epoch as u64, cfg);
}

/// Exact 2D minimization hypervolume for a finite front and a dominated reference point.
pub fn hypervolume_2d(front: &[[f64; 2]], reference: [f64; 2]) -> Result<f64, &'static str> {
    if front
        .iter()
        .flatten()
        .chain(reference.iter())
        .any(|x| !x.is_finite())
    {
        return Err("nonfinite hypervolume input");
    }
    if front
        .iter()
        .any(|p| p[0] >= reference[0] || p[1] >= reference[1])
    {
        return Err("reference point must dominate all points");
    }
    let mut points = front.to_vec();
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    points.dedup();
    let mut area = 0.0;
    let mut best_y = reference[1];
    for [x, y] in points {
        if y < best_y {
            area += (reference[0] - x) * (best_y - y);
            best_y = y;
        }
    }
    Ok(area)
}
