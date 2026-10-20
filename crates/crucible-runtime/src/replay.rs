//! Addressed, bounded integer fixture state. No physical search or verdicts.
use crate::{ledger::Resources, task::*, BlobDigest, Result, RuntimeError, SemanticId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

fn identity(value: &impl Serialize) -> Result<SemanticId> {
    SemanticId::of(value).map_err(|e| RuntimeError::Wire(e.to_string()))
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Experiment {
    /// Actual private Store objects, including a synthetic material/interface/solver recipe.
    pub dependencies: BTreeMap<String, BlobDigest>,
}
impl Experiment {
    pub fn check(&self) -> Result<()> {
        let required: BTreeSet<_> = [
            "solver",
            "material",
            "interface",
            "requirements",
            "recipe",
            "config",
            "executable",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        if self.dependencies.keys().cloned().collect::<BTreeSet<_>>() != required {
            return Err(RuntimeError::Invalid("complete_experiment_dependencies"));
        }
        Ok(())
    }
    pub fn identity(&self) -> Result<SemanticId> {
        self.check()?;
        identity(&("crucible.synthetic-experiment/1", self))
    }
    pub fn require_resume(&self, requested: &Self) -> Result<()> {
        if self != requested {
            return Err(RuntimeError::Conflict(
                "immutable_experiment_drift_use_new_experiment",
            ));
        }
        self.check()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToyGenome {
    pub schema_version: u32,
    pub values: [i64; 2],
    pub compiled: SemanticId,
}
impl ToyGenome {
    pub fn compile(values: [i64; 2]) -> Result<Self> {
        if values.iter().any(|v| v.unsigned_abs() > 100) {
            return Err(RuntimeError::Invalid("toy_seed_bounds"));
        }
        Ok(Self {
            schema_version: 1,
            compiled: identity(&("toy-integer-compiler/1", values))?,
            values,
        })
    }
    fn check(&self) -> Result<()> {
        if *self != Self::compile(self.values)? {
            return Err(RuntimeError::Invalid("toy_compilation_identity"));
        }
        Ok(())
    }
}
/// Import only values. Old identity, score, PASS and eligibility have no destination field.
pub fn import_seed(raw: &[u8]) -> Result<ToyGenome> {
    if raw.len() > 16_384 {
        return Err(RuntimeError::Invalid("seed_input_budget"));
    }
    let value: serde_json::Value =
        joil_evidence::identity::strict_decode(raw).map_err(RuntimeError::Wire)?;
    let values: [i64; 2] = serde_json::from_value(
        value
            .get("values")
            .cloned()
            .ok_or(RuntimeError::Invalid("seed_values"))?,
    )
    .map_err(|e| RuntimeError::Wire(e.to_string()))?;
    ToyGenome::compile(values)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Island {
    pub id: u32,
    pub population: Vec<ToyGenome>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorState {
    pub mean: [i64; 2],
    pub covariance: [[i64; 2]; 2],
    pub applications: u64,
}
impl OperatorState {
    fn check(&self) -> Result<()> {
        let c = self.covariance;
        if self.mean.iter().any(|x| x.unsigned_abs() > 100)
            || c.iter().flatten().any(|x| x.unsigned_abs() > 64)
            || c[0][1] != c[1][0]
            || c[0][0] <= 0
            || c[1][1] <= 0
            || c[0][0] * c[1][1] <= c[0][1] * c[1][0]
        {
            return Err(RuntimeError::Invalid("positive_bounded_fixture_covariance"));
        }
        Ok(())
    }
    fn mutate(&self, parent: &ToyGenome, address: &RngAddress) -> Result<ToyGenome> {
        self.check()?;
        let a = addressed_draw(address, 0)?;
        let b = addressed_draw(address, 1)?;
        // A bounded correlated integer operator; this is not a Gaussian sampler.
        ToyGenome::compile(std::array::from_fn(|i| {
            (parent.values[i]
                + self.mean[i]
                + self.covariance[i][0] * a
                + self.covariance[i][1] * b)
                .clamp(-100, 100)
        }))
    }
}
pub fn addressed_draw(address: &RngAddress, offset: u64) -> Result<i64> {
    let mut address = address.clone();
    address.draw = address
        .draw
        .checked_add(offset)
        .ok_or(RuntimeError::Invalid("rng_counter_overflow"))?;
    let hash = identity(&("fixture-counter-rng/1", address))?;
    let n = u64::from_str_radix(&hash.as_str()[..16], 16)
        .map_err(|_| RuntimeError::Invalid("rng_digest"))?;
    Ok((n % 3) as i64 - 1)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InnovationState {
    pub next: u64,
    pub records: BTreeMap<u64, SemanticId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeciesState {
    pub generation: u64,
    pub assignments: BTreeMap<SemanticId, u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedTask {
    pub id: String,
    pub spec: TaskSpec,
    pub genome: ToyGenome,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingResult {
    pub task: PlannedTask,
    pub output: BlobDigest,
    pub observation: FixtureObservation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationRecord {
    pub genome: ToyGenome,
    pub output: BlobDigest,
    pub observation: FixtureObservation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Archives {
    pub observations: BTreeMap<SemanticId, ObservationRecord>,
    pub best: Vec<SemanticId>,
    pub audit: Vec<String>,
    pub diagnostics: BTreeSet<BlobDigest>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionSplit {
    pub exploit: u64,
    pub random: u64,
    pub audit: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum StopState {
    Running,
    Requested { reason: String },
    Completed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionLineage {
    pub id: SemanticId,
    pub parent: Option<SemanticId>,
    pub weights: [i64; 2],
    pub decisions: BTreeMap<SemanticId, i64>,
}
impl DecisionLineage {
    pub fn new(
        campaign: &SemanticId,
        parent: Option<SemanticId>,
        weights: [i64; 2],
    ) -> Result<Self> {
        if weights == [0, 0] || weights.iter().any(|x| x.unsigned_abs() > 1000) {
            return Err(RuntimeError::Invalid("bounded_nonzero_fixture_weights"));
        }
        Ok(Self {
            id: identity(&("fixture-decision-lineage/1", campaign, &parent, weights))?,
            parent,
            weights,
            decisions: BTreeMap::new(),
        })
    }
    fn recompute(&mut self, archive: &Archives) -> Result<()> {
        self.decisions.clear();
        for (id, r) in &archive.observations {
            let utility = self.weights[0]
                .checked_mul(r.observation.score)
                .and_then(|n| {
                    n.checked_add(
                        self.weights[1] * (r.genome.values[0].abs() + r.genome.values[1].abs()),
                    )
                })
                .ok_or(RuntimeError::Invalid("fixture_utility_overflow"))?;
            self.decisions.insert(id.clone(), utility);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchState {
    pub schema_version: u32,
    pub campaign: SemanticId,
    pub completed_epochs: u64,
    pub next_ordinal: u64,
    pub islands: Vec<Island>,
    pub archives: Archives,
    pub innovations: InnovationState,
    pub species: SpeciesState,
    pub operator: OperatorState,
    pub rng_counters: BTreeMap<String, u64>,
    pub in_flight: BTreeMap<String, PlannedTask>,
    pub pending_results: BTreeMap<String, PendingResult>,
    pub acquisition: AcquisitionSplit,
    pub applied_ordinals: Vec<(u64, u64)>,
    pub arrival_order: Vec<String>,
    pub lineage: DecisionLineage,
    pub stop: StopState,
}

#[path = "replay/search_state.rs"]
mod search_state;
