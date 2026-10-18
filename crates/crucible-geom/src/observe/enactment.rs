//! Perturbation orchestration is separate from physical observation. Parameter
//! names and steps choose a development request; no expected coefficient or
//! authored mechanism reaches volume measurement, mass or deduplication.
use super::{
    volume::{self, Cell, Volume},
    BoundVolume, ObserveError, Provenance,
};
use crucible_contract::subject::{BlobDigest, SemanticId};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub struct ProbeStep {
    pub parameter: String,
    pub unit: String,
    pub step: f64,
    pub parameter_resolution: f64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DevelopmentFailure {
    Unsupported,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Enactment {
    Changed,
    Neutral,
    Unsupported,
    FailedDevelopment,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ProbeResult {
    pub parameter: String,
    pub state: Enactment,
    pub changed_cells: [usize; 2],
    pub resolved_witness_cells: [usize; 2],
    pub physical_ids: [Option<SemanticId>; 2],
    pub solid_volume_delta_m3: [Option<f64>; 2],
}

/// Representation identity ignores parameter names, lineage, subject ID,
/// whitespace and port-map ordering. It is not a cross-mesh equivalence proof;
/// differently discretized subjects need a separate physical equivalence test.
pub fn physical_id(v: &BoundVolume) -> Result<SemanticId, ObserveError> {
    let g = v.volume();
    SemanticId::of(&(
        "observed-partition/1",
        &v.provenance().frame_transform,
        g.dimensions,
        g.origin_m.map(|x| if x == 0.0 { 0.0 } else { x }),
        g.pitch_m,
        &g.cells,
    ))
    .map_err(|_| ObserveError::Numerical("physical identity"))
}

fn phase_clearance(g: &Volume, i: usize) -> f64 {
    let p = g.point(i);
    let mut d = (0..3)
        .map(|a| {
            ((p[a] as f64 + 0.5) * g.pitch_m)
                .min((g.dimensions[a] as f64 - p[a] as f64 - 0.5) * g.pitch_m)
        })
        .fold(f64::INFINITY, f64::min);
    for (j, &c) in g.cells.iter().enumerate() {
        if c != g.cells[i] {
            let q = g.point(j);
            let distance = (0..3)
                .map(|a| {
                    (((p[a] as f64 - q[a] as f64).abs() - 0.5) * g.pitch_m)
                        .max(0.0)
                        .powi(2)
                })
                .sum::<f64>()
                .sqrt();
            d = d.min(distance);
        }
    }
    (d - 64.0 * f64::EPSILON * g.pitch_m * (*g.dimensions.iter().max().unwrap() as f64)).max(0.0)
}
fn compare(
    a: &BoundVolume,
    b: &BoundVolume,
) -> Result<(Enactment, usize, usize, f64), ObserveError> {
    let x = a.volume();
    let y = b.volume();
    if x.dimensions != y.dimensions
        || x.origin_m != y.origin_m
        || x.pitch_m != y.pitch_m
        || a.provenance().frame_transform != b.provenance().frame_transform
    {
        return Ok((Enactment::Unsupported, 0, 0, 0.0));
    }
    if !x.unresolved_cells().is_empty() || !y.unresolved_cells().is_empty() {
        return Ok((Enactment::Unsupported, 0, 0, 0.0));
    }
    let changed: Vec<_> = (0..x.cells.len())
        .filter(|&i| x.cells[i] != y.cells[i])
        .collect();
    let error = a.provenance().approximation_m + b.provenance().approximation_m;
    let witness = changed
        .iter()
        .filter(|&&i| phase_clearance(x, i) > error && phase_clearance(y, i) > error)
        .count();
    let delta = volume::measure(y)?.solid_volume_m3 - volume::measure(x)?.solid_volume_m3;
    let state = if changed.is_empty() {
        Enactment::Neutral
    } else if witness > 0 {
        Enactment::Changed
    } else {
        Enactment::Unsupported
    };
    Ok((state, changed.len(), witness, delta))
}

/// Both signs are developed for every declared active parameter. The caller
/// supplies only verified output artifacts or typed development failure. An
/// unsupported/failed development cannot be replaced with the baseline.
pub fn probe_active(
    baseline: &BoundVolume,
    active: &[ProbeStep],
    mut develop: impl FnMut(&str, f64) -> Result<BoundVolume, DevelopmentFailure>,
) -> Result<Vec<ProbeResult>, ObserveError> {
    let mut names = BTreeSet::new();
    if active.is_empty()
        || active.len() > 128
        || active.iter().any(|p| {
            p.parameter.is_empty()
                || p.unit.is_empty()
                || !names.insert(&p.parameter)
                || !p.step.is_finite()
                || !p.parameter_resolution.is_finite()
                || p.parameter_resolution <= 0.0
                || p.step <= p.parameter_resolution
        })
    {
        return Err(ObserveError::Invalid(
            "active parameter probe/representation scale",
        ));
    }
    let mut results = Vec::new();
    for probe in active {
        let mut states = [Enactment::Neutral; 2];
        let mut counts = [0; 2];
        let mut witnesses = [0; 2];
        let mut ids = [None, None];
        let mut deltas = [None, None];
        for (index, sign) in [-1.0, 1.0].iter().enumerate() {
            match develop(&probe.parameter, sign * probe.step) {
                Err(DevelopmentFailure::Unsupported) => states[index] = Enactment::Unsupported,
                Err(DevelopmentFailure::Failed) => states[index] = Enactment::FailedDevelopment,
                Ok(value) => {
                    let (state, n, w, d) = compare(baseline, &value)?;
                    states[index] = state;
                    counts[index] = n;
                    witnesses[index] = w;
                    ids[index] = Some(physical_id(&value)?);
                    deltas[index] = Some(d);
                }
            }
        }
        let state = if states.contains(&Enactment::FailedDevelopment) {
            Enactment::FailedDevelopment
        } else if states.contains(&Enactment::Unsupported) {
            Enactment::Unsupported
        } else if states.contains(&Enactment::Changed) {
            Enactment::Changed
        } else {
            Enactment::Neutral
        };
        results.push(ProbeResult {
            parameter: probe.parameter.clone(),
            state,
            changed_cells: counts,
            resolved_witness_cells: witnesses,
            physical_ids: ids,
            solid_volume_delta_m3: deltas,
        });
    }
    Ok(results)
}

#[derive(Clone, Debug, PartialEq)]
pub struct MassObservation {
    pub provenance: Provenance,
    pub density_source: BlobDigest,
    pub solid_volume_m3: f64,
    pub represented_mass_kg: f64,
}
/// Mass comes only from this same accepted solid partition. Density source
/// binding is retained; this function does not qualify that material source.
pub fn mass(
    v: &BoundVolume,
    density_kg_m3: f64,
    density_source: BlobDigest,
) -> Result<MassObservation, ObserveError> {
    if !density_kg_m3.is_finite() || density_kg_m3 <= 0.0 {
        return Err(ObserveError::Invalid("density"));
    }
    if v.volume().cells.contains(&Cell::Unresolved) {
        return Err(ObserveError::Unsupported("unresolved solid mass"));
    }
    let solid_volume_m3 = volume::measure(v.volume())?.solid_volume_m3;
    let represented_mass_kg = solid_volume_m3 * density_kg_m3;
    if !represented_mass_kg.is_finite() {
        return Err(ObserveError::Numerical("mass overflow"));
    }
    Ok(MassObservation {
        provenance: v.provenance().clone(),
        density_source,
        solid_volume_m3,
        represented_mass_kg,
    })
}
