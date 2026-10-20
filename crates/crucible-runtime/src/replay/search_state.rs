//! `SearchState` epoch preparation, result collection, deterministic application, and lineage forks.

use super::*;

impl SearchState {
    pub fn new(campaign: SemanticId, seeds: Vec<[i64; 2]>, weights: [i64; 2]) -> Result<Self> {
        if seeds.is_empty() || seeds.len() > 2 {
            return Err(RuntimeError::Invalid("fixture_islands_1_or_2"));
        }
        let islands = seeds
            .into_iter()
            .enumerate()
            .map(|(id, v)| {
                Ok(Island {
                    id: id as u32,
                    population: vec![ToyGenome::compile(v)?],
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let assignments = islands
            .iter()
            .map(|i| (i.population[0].compiled.clone(), i.id))
            .collect();
        let lineage = DecisionLineage::new(&campaign, None, weights)?;
        Ok(Self {
            schema_version: 1,
            campaign,
            completed_epochs: 0,
            next_ordinal: 0,
            islands,
            archives: Archives {
                observations: BTreeMap::new(),
                best: Vec::new(),
                audit: Vec::new(),
                diagnostics: BTreeSet::new(),
            },
            innovations: InnovationState {
                next: 0,
                records: BTreeMap::new(),
            },
            species: SpeciesState {
                generation: 0,
                assignments,
            },
            operator: OperatorState {
                mean: [0, 0],
                covariance: [[4, 1], [1, 2]],
                applications: 0,
            },
            rng_counters: BTreeMap::new(),
            in_flight: BTreeMap::new(),
            pending_results: BTreeMap::new(),
            acquisition: AcquisitionSplit {
                exploit: 0,
                random: 0,
                audit: 0,
            },
            applied_ordinals: Vec::new(),
            arrival_order: Vec::new(),
            lineage,
            stop: StopState::Running,
        })
    }
    pub fn check(&self) -> Result<()> {
        if self.schema_version != 1
            || self.islands.is_empty()
            || self.islands.len() > 2
            || self.next_ordinal > 32
            || self.completed_epochs > 16
            || self.operator.applications > 16
            || self.species.generation != self.completed_epochs
            || self.innovations.next != self.next_ordinal
            || self.innovations.records.len() as u64 != self.next_ordinal
            || self.acquisition.exploit + self.acquisition.random + self.acquisition.audit
                != self.next_ordinal
        {
            return Err(RuntimeError::Invalid("complete_bounded_fixture_state"));
        }
        self.operator.check()?;
        let mut islands = BTreeSet::new();
        for i in &self.islands {
            if !islands.insert(i.id) || i.population.len() != 1 {
                return Err(RuntimeError::Invalid("fixture_population"));
            }
            i.population[0].check()?;
        }
        for (name, t) in &self.in_flight {
            if name != &t.id || self.pending_results.contains_key(name) {
                return Err(RuntimeError::Invalid("inflight_pending_overlap"));
            }
            self.check_planned(t)?;
        }
        for (name, p) in &self.pending_results {
            if name != &p.task.id {
                return Err(RuntimeError::Invalid("pending_task_name"));
            }
            self.check_result(&p.task, &p.observation)?;
        }
        for (id, r) in &self.archives.observations {
            r.genome.check()?;
            if id != &r.observation.request
                || !r.observation.synthetic_only
                || r.observation.score != r.genome.values.iter().map(|v| v * v).sum::<i64>()
            {
                return Err(RuntimeError::Invalid("archive_observation"));
            }
        }
        let mut lineage = DecisionLineage::new(
            &self.campaign,
            self.lineage.parent.clone(),
            self.lineage.weights,
        )?;
        lineage.recompute(&self.archives)?;
        if lineage != self.lineage
            || self
                .archives
                .best
                .iter()
                .any(|id| !lineage.decisions.contains_key(id))
            || self.applied_ordinals.windows(2).any(|w| w[0] >= w[1])
        {
            return Err(RuntimeError::Invalid("lineage_or_application_order"));
        }
        Ok(())
    }
    fn check_planned(&self, t: &PlannedTask) -> Result<()> {
        t.genome.check()?;
        t.spec.check()?;
        let Request::SoftwareFixture { request } = &t.spec.request else {
            return Err(RuntimeError::Unsupported("physical_checkpoint_fixture"));
        };
        if request.address.campaign != self.campaign
            || request.values != t.genome.values
            || request.address.epoch != t.spec.due_epoch
            || request.address.proposal != t.spec.ordinal
            || request.address.epoch != self.completed_epochs + 1
        {
            return Err(RuntimeError::Invalid("planned_fixture_binding"));
        }
        Ok(())
    }
    fn check_result(&self, t: &PlannedTask, o: &FixtureObservation) -> Result<()> {
        self.check_planned(t)?;
        let Request::SoftwareFixture { request } = &t.spec.request else {
            unreachable!()
        };
        if o.schema_version != 1
            || !o.synthetic_only
            || o.request != t.spec.request.identity()?
            || o.address != request.address
            || o.score != request.synthetic_score()?
        {
            return Err(RuntimeError::Invalid("pending_result_binding"));
        }
        Ok(())
    }
    pub fn prepare_epoch(
        &mut self,
        executable: &BlobDigest,
        reserve: Resources,
        mode: FixtureMode,
    ) -> Result<()> {
        self.check()?;
        if !self.in_flight.is_empty()
            || !self.pending_results.is_empty()
            || self.stop != StopState::Running
            || self.completed_epochs >= 16
        {
            return Err(RuntimeError::Conflict("epoch_boundary_required"));
        }
        for island in self.islands.clone() {
            let ordinal = self.next_ordinal;
            let key = format!("{}/correlated-integer", island.id);
            let draw = *self.rng_counters.get(&key).unwrap_or(&0);
            let address = RngAddress {
                campaign: self.campaign.clone(),
                island: island.id,
                epoch: self.completed_epochs + 1,
                proposal: ordinal,
                operator: "correlated-integer/1".into(),
                draw,
            };
            let genome = if self.completed_epochs == 0 {
                island.population[0].clone()
            } else {
                self.operator.mutate(&island.population[0], &address)?
            };
            let request = FixtureRequest {
                schema_version: 1,
                values: genome.values.to_vec(),
                address,
                mode: mode.clone(),
                executable: executable.clone(),
            };
            let id = format!(
                "fixture-epoch-{}-ordinal-{ordinal}",
                self.completed_epochs + 1
            );
            let task = PlannedTask {
                id: id.clone(),
                spec: TaskSpec {
                    request: Request::SoftwareFixture { request },
                    due_epoch: self.completed_epochs + 1,
                    ordinal,
                    class: crate::ledger::WorkClass::Evaluation,
                    reserve,
                    ranks: 1,
                    threads: 1,
                },
                genome: genome.clone(),
            };
            self.in_flight.insert(id, task);
            self.rng_counters.insert(key, draw + 2);
            self.innovations
                .records
                .insert(ordinal, genome.compiled.clone());
            self.innovations.next += 1;
            self.next_ordinal += 1;
            self.species.assignments.insert(genome.compiled, island.id);
            match ordinal % 3 {
                0 => self.acquisition.exploit += 1,
                1 => self.acquisition.random += 1,
                _ => self.acquisition.audit += 1,
            }
        }
        self.check()
    }
    pub fn collect(
        &mut self,
        name: &str,
        output: BlobDigest,
        observation: FixtureObservation,
    ) -> Result<()> {
        if let Some(p) = self.pending_results.get(name) {
            if p.output == output && p.observation == observation {
                return Ok(());
            }
            return Err(RuntimeError::Conflict("pending_duplicate_differs"));
        }
        let t = self
            .in_flight
            .get(name)
            .ok_or(RuntimeError::Invalid("result_not_scheduled"))?
            .clone();
        self.check_result(&t, &observation)?;
        self.in_flight.remove(name);
        self.pending_results.insert(
            name.into(),
            PendingResult {
                task: t,
                output,
                observation,
            },
        );
        self.arrival_order.push(name.into());
        Ok(())
    }
    pub fn apply_epoch(&mut self) -> Result<()> {
        self.check()?;
        if !self.in_flight.is_empty() || self.pending_results.len() != self.islands.len() {
            return Err(RuntimeError::Conflict("epoch_results_incomplete"));
        }
        let mut results: Vec<_> = std::mem::take(&mut self.pending_results)
            .into_values()
            .collect();
        results.sort_by_key(|p| (p.task.spec.due_epoch, p.task.spec.ordinal));
        for p in results {
            self.applied_ordinals
                .push((p.task.spec.due_epoch, p.task.spec.ordinal));
            let island = p.observation.address.island;
            self.islands
                .iter_mut()
                .find(|i| i.id == island)
                .ok_or(RuntimeError::Invalid("result_island"))?
                .population = vec![p.task.genome.clone()];
            self.archives.observations.insert(
                p.observation.request.clone(),
                ObservationRecord {
                    genome: p.task.genome,
                    output: p.output,
                    observation: p.observation,
                },
            );
        }
        self.completed_epochs += 1;
        self.species.generation = self.completed_epochs;
        self.operator.applications += 1;
        self.operator.covariance[0][0] = (self.operator.covariance[0][0] + 1).min(64);
        self.operator.covariance[1][1] = (self.operator.covariance[1][1] + 1).min(64);
        self.archives.audit.push(format!(
            "synthetic epoch {} applied in due/ordinal order",
            self.completed_epochs
        ));
        self.rerank()?;
        self.check()
    }
    fn rerank(&mut self) -> Result<()> {
        self.lineage.recompute(&self.archives)?;
        let mut ranked: Vec<_> = self
            .lineage
            .decisions
            .iter()
            .map(|(id, score)| (*score, id.clone()))
            .collect();
        ranked.sort();
        self.archives.best = ranked.into_iter().map(|(_, id)| id).take(2).collect();
        Ok(())
    }
    pub fn fork(&self, weights: [i64; 2]) -> Result<Self> {
        self.check()?;
        if !self.in_flight.is_empty() || !self.pending_results.is_empty() {
            return Err(RuntimeError::Conflict("fork_requires_epoch_boundary"));
        }
        let mut next = self.clone();
        next.lineage =
            DecisionLineage::new(&self.campaign, Some(self.lineage.id.clone()), weights)?;
        next.stop = StopState::Running;
        next.rerank()?;
        next.check()?;
        Ok(next)
    }
    pub fn logical_digest(&self) -> Result<SemanticId> {
        self.check()?;
        let mut state = self.clone();
        state.arrival_order.clear();
        identity(&("fixture-logical-state/1", state))
    }
    pub fn object_references(&self) -> BTreeSet<BlobDigest> {
        self.archives
            .observations
            .values()
            .map(|r| r.output.clone())
            .chain(self.pending_results.values().map(|r| r.output.clone()))
            .chain(self.archives.diagnostics.iter().cloned())
            .collect()
    }
}
