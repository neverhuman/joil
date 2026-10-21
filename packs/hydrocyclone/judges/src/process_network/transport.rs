use super::*;
use crate::{canonical_hash, invalid, JudgeError, Verdict};
use std::collections::BTreeMap;

impl Cohort {
    pub fn oil_total_kg(&self) -> f64 {
        self.oil_mass_kg.iter().sum()
    }

    fn zero(bins: usize) -> Self {
        Self {
            liquid_volume_m3: 0.0,
            oil_mass_kg: vec![0.0; bins],
        }
    }

    fn nonzero(&self) -> bool {
        self.liquid_volume_m3 > 0.0 || self.oil_mass_kg.iter().any(|x| *x > 0.0)
    }

    fn add(&mut self, other: &Self) -> Result<(), JudgeError> {
        self.liquid_volume_m3 += other.liquid_volume_m3;
        for (a, b) in self.oil_mass_kg.iter_mut().zip(&other.oil_mass_kg) {
            *a += b;
        }
        if !self.liquid_volume_m3.is_finite() || !self.oil_total_kg().is_finite() {
            return Err(invalid("network:cohort_accumulation_overflow"));
        }
        Ok(())
    }
}

fn total(cohorts: impl Iterator<Item = Cohort>, bins: usize) -> Result<Cohort, JudgeError> {
    let mut sum = Cohort::zero(bins);
    for c in cohorts {
        sum.add(&c)?;
    }
    Ok(sum)
}

fn route(input: &Cohort, r: &Route) -> Cohort {
    let mut out = Cohort::zero(input.oil_mass_kg.len());
    out.liquid_volume_m3 = input.liquid_volume_m3 * r.liquid_fraction;
    for (mass, row) in input.oil_mass_kg.iter().zip(&r.oil_mass_transfer) {
        for (out_mass, fraction) in out.oil_mass_kg.iter_mut().zip(row) {
            *out_mass += mass * fraction;
        }
    }
    out
}

fn below_tail(pending: &Cohort, fresh: &Cohort, tolerance: f64) -> bool {
    let oil = fresh.oil_total_kg();
    pending.liquid_volume_m3 <= tolerance * fresh.liquid_volume_m3
        && (oil == 0.0 || pending.oil_total_kg() <= tolerance * oil)
}

/// Propagate first-passage cohorts. Cycles consume the same explicit work budget;
/// remaining and wall oil are inventory, never silently counted as captured.
pub fn evaluate(request: &NetworkRequest) -> Result<NetworkReport, JudgeError> {
    request.validate()?;
    let bins = request.diameter_edges_m.len() - 1;
    let mut pending: BTreeMap<_, _> = request
        .stages
        .iter()
        .map(|s| (s.id.clone(), Cohort::zero(bins)))
        .collect();
    for f in &request.feeds {
        pending.get_mut(&f.stage_id).unwrap().add(&f.cohort)?;
    }
    let fresh = total(request.feeds.iter().map(|f| f.cohort.clone()), bins)?;
    let mut processed = pending
        .keys()
        .map(|id| (id.clone(), Cohort::zero(bins)))
        .collect::<BTreeMap<_, _>>();
    let (mut product, mut reject, mut retained, mut wall) = (
        Cohort::zero(bins),
        Cohort::zero(bins),
        Cohort::zero(bins),
        Cohort::zero(bins),
    );
    let (mut steps, mut visits, mut terms) = (0usize, 0u64, 0u64);
    let termination = loop {
        let circulating = total(pending.values().cloned(), bins)?;
        if !circulating.nonzero() {
            break Termination::Complete;
        }
        if steps > 0 && below_tail(&circulating, &fresh, request.budget.relative_tail_tolerance) {
            break Termination::TailBoundReached;
        }
        let active: Vec<_> = request
            .stages
            .iter()
            .filter(|s| pending[&s.id].nonzero())
            .collect();
        let next_terms: u64 = active
            .iter()
            .map(|s| (s.routes.len() * bins * bins) as u64)
            .sum();
        if steps >= request.budget.max_steps
            || visits + active.len() as u64 > request.budget.max_stage_visits
            || terms + next_terms > request.budget.max_matrix_terms
        {
            break Termination::WorkBudgetExhausted;
        }
        let mut next: BTreeMap<_, _> = pending
            .keys()
            .map(|id| (id.clone(), Cohort::zero(bins)))
            .collect();
        for s in active {
            let input = &pending[&s.id];
            processed.get_mut(&s.id).unwrap().add(input)?;
            for r in &s.routes {
                let out = route(input, r);
                let destination = match &r.destination {
                    Destination::Stage { stage_id } => next.get_mut(stage_id).unwrap(),
                    Destination::Product => &mut product,
                    Destination::Reject => &mut reject,
                    Destination::Retained => &mut retained,
                    Destination::Wall => &mut wall,
                };
                destination.add(&out)?;
            }
            visits += 1;
        }
        terms += next_terms;
        steps += 1;
        pending = next;
    };
    let circulating = total(pending.values().cloned(), bins)?;
    let uncertain = total([&retained, &wall, &circulating].into_iter().cloned(), bins)?;
    let accounted = total(
        [&product, &reject, &retained, &wall, &circulating]
            .into_iter()
            .cloned(),
        bins,
    )?;
    let volume_error = accounted.liquid_volume_m3 - fresh.liquid_volume_m3;
    let oil_error = accounted.oil_total_kg() - fresh.oil_total_kg();
    // A response matrix accepted within roundoff must not accumulate material
    // balance error indefinitely through recycle. Never repair by normalization.
    if volume_error.abs() > 1e-9 * fresh.liquid_volume_m3
        || oil_error.abs() > 1e-9 * fresh.oil_total_kg()
    {
        return Err(invalid("network:accumulated_balance_error"));
    }
    let concentration = if product.liquid_volume_m3 > 0.0 {
        // kg/m3 -> mg/L. Unknown inventory may all become product oil, while
        // its carrier volume may all become product water in the lower bound.
        let lower = 1000.0 * product.oil_total_kg()
            / (product.liquid_volume_m3 + uncertain.liquid_volume_m3);
        let upper =
            1000.0 * (product.oil_total_kg() + uncertain.oil_total_kg()) / product.liquid_volume_m3;
        if !lower.is_finite() || !upper.is_finite() {
            return Err(invalid("network:concentration_overflow"));
        }
        Some(Bounds { lower, upper })
    } else {
        None
    };
    Ok(NetworkReport {
        schema_version: SCHEMA_VERSION,
        request_sha256: canonical_hash(request)?,
        evidence_class: request.evidence_class,
        physical_verdict: Verdict::Unresolved(vec![
            "fixed_linear_response:geometry_hydraulics_and_physics_not_qualified".into(),
        ]),
        termination,
        steps,
        stage_visits: visits,
        matrix_terms: terms,
        external_reject_fraction: Bounds {
            lower: reject.liquid_volume_m3 / fresh.liquid_volume_m3,
            upper: (reject.liquid_volume_m3 + uncertain.liquid_volume_m3) / fresh.liquid_volume_m3,
        },
        fresh_feed: fresh,
        product,
        reject,
        retained,
        wall,
        circulating,
        processed_by_stage: processed,
        liquid_balance_error_m3: volume_error,
        oil_balance_error_kg: oil_error,
        product_concentration_mg_l: concentration,
    })
}
