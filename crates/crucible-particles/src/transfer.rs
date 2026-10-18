//! Scenario reuse and cluster-aware uncertainty contracts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinearConditions {
    pub noninteracting: bool,
    pub coalescence_enabled: bool,
    pub breakup_concentration_coupled: bool,
    pub two_way_coupling: bool,
    pub concentration_fraction: f64,
}

impl LinearConditions {
    pub fn validate_reuse(&self) -> Result<(), String> {
        if !self.concentration_fraction.is_finite() || self.concentration_fraction < 0.0 {
            return Err("invalid transfer concentration".into());
        }
        if !self.noninteracting
            || self.coalescence_enabled
            || self.breakup_concentration_coupled
            || self.two_way_coupling
            || self.concentration_fraction > 0.01
        {
            return Err("DSD transfer reuse is invalid with interactions, concentration coupling, two-way coupling, or concentration above 1%".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DsdTransferMatrix {
    pub entries: Vec<Vec<f64>>,
    pub outcome_labels: Vec<String>,
    pub condition_id: String,
}

impl DsdTransferMatrix {
    pub fn validate(&self, conditions: &LinearConditions) -> Result<(), String> {
        conditions.validate_reuse()?;
        if self.condition_id.is_empty()
            || self.entries.is_empty()
            || self.outcome_labels.is_empty()
            || self.entries.iter().any(|r| {
                r.len() != self.outcome_labels.len()
                    || r.iter().any(|v| !v.is_finite() || *v < 0.0)
                    || (r.iter().sum::<f64>() - 1.0).abs() > 1e-12
            })
        {
            return Err(
                "transfer rows must be finite, nonnegative, and conserve unit injected mass".into(),
            );
        }
        Ok(())
    }

    pub fn reweight(
        &self,
        injected_mass_kg: &[f64],
        conditions: &LinearConditions,
    ) -> Result<Vec<f64>, String> {
        self.validate(conditions)?;
        if injected_mass_kg.len() != self.entries.len()
            || injected_mass_kg.iter().any(|m| !m.is_finite() || *m < 0.0)
        {
            return Err("injected DSD mass does not align with transfer rows".into());
        }
        let mut outcome = vec![0.0; self.outcome_labels.len()];
        for (mass, row) in injected_mass_kg.iter().zip(&self.entries) {
            for (out, p) in outcome.iter_mut().zip(row) {
                *out += mass * p;
            }
        }
        let input: f64 = injected_mass_kg.iter().sum();
        let output: f64 = outcome.iter().sum();
        if !input.is_finite()
            || !output.is_finite()
            || outcome.iter().any(|v| !v.is_finite())
            || (input - output).abs() > 1e-12 * input
        {
            return Err("reweighted outcomes fail injected mass closure".into());
        }
        Ok(outcome)
    }

    pub fn compare_held_out(
        &self,
        injected_mass_kg: &[f64],
        direct_outcomes_kg: &[f64],
        budget_relative: f64,
        conditions: &LinearConditions,
    ) -> Result<f64, String> {
        if !budget_relative.is_finite()
            || budget_relative < 0.0
            || direct_outcomes_kg.len() != self.outcome_labels.len()
            || direct_outcomes_kg
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err("invalid held-out comparison inputs".into());
        }
        let predicted = self.reweight(injected_mass_kg, conditions)?;
        let err = predicted
            .iter()
            .zip(direct_outcomes_kg)
            .map(|(a, b)| (a - b).abs())
            .sum::<f64>()
            / direct_outcomes_kg.iter().sum::<f64>().max(1e-30);
        if err > budget_relative {
            return Err(format!(
                "held-out direct scenario exceeds reuse budget: relative L1 error {err}"
            ));
        }
        Ok(err)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeightedGradeSample {
    pub cluster_id: String,
    pub ancestry_id: String,
    pub time_block_id: String,
    pub value: f64,
    pub sampling_weight: f64,
    pub units: String,
    pub basis: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClusterUncertainty {
    pub mean: f64,
    pub standard_error: f64,
    pub interval_lower: f64,
    pub interval_upper: f64,
    pub independent_cluster_count: usize,
    pub units: String,
    pub basis: String,
    pub sampling_design: String,
    pub critical_value_source: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorComponents {
    pub class_refinement_error: f64,
    pub substep_error: f64,
    pub model_error: f64,
}

impl ErrorComponents {
    pub fn validate(&self) -> Result<(), String> {
        if [
            self.class_refinement_error,
            self.substep_error,
            self.model_error,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err(
                "class, substep and model errors must be separately finite and nonnegative".into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradeObservation {
    pub estimate: ClusterUncertainty,
    pub applicability_exclusions: Vec<String>,
    pub errors: ErrorComponents,
}

impl GradeObservation {
    pub fn bind(
        estimate: ClusterUncertainty,
        applicability: &Applicability,
        errors: ErrorComponents,
    ) -> Result<Self, String> {
        errors.validate()?;
        if estimate.units.is_empty()
            || estimate.basis.is_empty()
            || estimate.sampling_design.is_empty()
            || estimate.critical_value_source.is_empty()
        {
            return Err("grade observation must bind units, concentration basis, sampling design and uncertainty source".into());
        }
        Ok(Self {
            estimate,
            applicability_exclusions: applicability.declared_exclusions(),
            errors,
        })
    }
}

/// First aggregates by independent cluster (therefore daughters from one parent
/// cannot masquerade as IID), then forms a t interval using a quantile from the
/// crucible-stats caller. Separate ancestry and time blocks remain traceable.
pub fn cluster_weighted_interval(
    samples: &[WeightedGradeSample],
    t_critical: f64,
    quantile_source: &str,
) -> Result<ClusterUncertainty, String> {
    if samples.len() < 2
        || !t_critical.is_finite()
        || t_critical <= 0.0
        || quantile_source.is_empty()
        || samples.iter().any(|s| {
            s.cluster_id.is_empty()
                || s.ancestry_id.is_empty()
                || s.time_block_id.is_empty()
                || !s.value.is_finite()
                || !s.sampling_weight.is_finite()
                || s.sampling_weight <= 0.0
                || s.units.is_empty()
                || s.basis.is_empty()
        })
    {
        return Err("weighted uncertainty needs at least two valid independent clusters and a declared t quantile source".into());
    }
    let units = &samples[0].units;
    let basis = &samples[0].basis;
    if samples
        .iter()
        .any(|s| &s.units != units || &s.basis != basis)
    {
        return Err("grade observations mix units or concentration basis".into());
    }
    let mut ancestry_clusters = std::collections::BTreeMap::new();
    for sample in samples {
        if ancestry_clusters
            .insert(sample.ancestry_id.as_str(), sample.cluster_id.as_str())
            .is_some_and(|cluster| cluster != sample.cluster_id)
        {
            return Err("same ancestry assigned to different independent clusters".into());
        }
    }
    let mut clusters: std::collections::BTreeMap<&str, (f64, f64)> =
        std::collections::BTreeMap::new();
    for s in samples {
        let e = clusters.entry(&s.cluster_id).or_insert((0.0, 0.0));
        e.0 += s.value * s.sampling_weight;
        e.1 += s.sampling_weight;
    }
    if clusters.len() < 2 {
        return Err(
            "daughter ancestry or repeated blocks do not create independent clusters".into(),
        );
    }
    let values: Vec<f64> = clusters.values().map(|(v, w)| v / w).collect();
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance =
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    let se = (variance / values.len() as f64).sqrt();
    let half = t_critical * se;
    if ![mean, se, mean - half, mean + half]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err("cluster uncertainty arithmetic overflow".into());
    }
    Ok(ClusterUncertainty { mean, standard_error:se, interval_lower:mean-half, interval_upper:mean+half, independent_cluster_count:values.len(), units:units.clone(), basis:basis.clone(), sampling_design:"mass/importance-weighted observations aggregated within independent cluster; ancestry and time-block IDs retained".into(), critical_value_source:quantile_source.into() })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Applicability {
    pub films: bool,
    pub enrichment: bool,
    pub free_gas: bool,
    pub solids: bool,
    pub two_way_coupling: bool,
}
impl Applicability {
    pub fn declared_exclusions(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !self.films {
            out.push("wall films excluded".into());
        }
        if !self.enrichment {
            out.push("local oil enrichment excluded".into());
        }
        if !self.free_gas {
            out.push("free gas excluded".into());
        }
        if !self.solids {
            out.push("solids excluded".into());
        }
        if !self.two_way_coupling {
            out.push("two-way coupling excluded".into());
        }
        out
    }
}
