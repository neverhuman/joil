//! V5-04-02: capture efficiency by represented mass with clustered uncertainty.
//!
//! A parcel is a weighted representative of injected mass, not a Bernoulli trial.
//! Parcels and daughters sharing a carrier realization, time block or quadrature
//! randomization are dependent, so uncertainty is estimated only at the declared
//! independent level: carrier realization/time-block clusters (stratified separate
//! ratio estimator, linearization variance) or independently randomized quadrature
//! replicates (replicate-mean estimator). Daughter ancestry is kept and checked;
//! splitting a parcel into daughters with the same fate cannot change the estimate.
//! Interpretation and applicability are stated in every result. Not physical evidence.
use crate::distribution::student_t_quantile;
use crate::StatsError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The level at which observations are declared independent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndependentUnit {
    /// Independent carrier realizations or separated time blocks.
    CarrierCluster,
    /// Independently randomized quadrature replicates (each a full point set).
    RandomizedQuadratureReplicate,
    /// Refused: parcels in one carrier realization share its flow.
    Parcel,
    /// Refused: daughters of one parent share ancestry and carrier.
    Daughter,
    /// Refused: points within one quasi-random set are deterministically dependent.
    QuadraturePoint,
}

/// A leaf parcel after any breakup; `root_id` names the injected parcel it came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parcel {
    pub parcel_id: String,
    pub root_id: String,
    pub stratum: String,
    pub cluster: String,
    pub represented_mass: f64,
    pub captured: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cluster {
    pub cluster_id: String,
    /// Required and unique for randomized quadrature replicates; `None` for carrier clusters.
    pub randomization_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureDesign {
    pub independent_unit: IndependentUnit,
    /// Declared population mass fraction of each injection stratum; must sum to one.
    pub stratum_mass_fractions: BTreeMap<String, f64>,
    pub clusters: Vec<Cluster>,
    /// Two-sided confidence level.
    pub confidence: f64,
    /// Explicit applicability declaration (assumptions under which coverage is claimed).
    pub applicability: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StratumEstimate {
    pub stratum: String,
    pub mass_fraction: f64,
    pub clusters: usize,
    pub captured_mass: f64,
    pub represented_mass: f64,
    pub ratio: f64,
    /// Stratum-only linearization variance (carrier clusters); the combined
    /// variance additionally keeps the covariance between strata sharing clusters.
    pub variance: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaptureEstimate {
    pub estimate: f64,
    pub standard_error: f64,
    pub degrees_of_freedom: usize,
    pub t_critical: f64,
    pub lower: f64,
    pub upper: f64,
    pub strata: Vec<StratumEstimate>,
    pub independent_unit: IndependentUnit,
    pub interpretation: String,
    pub applicability: String,
}

fn identity(value: &str) -> Result<(), StatsError> {
    joil_evidence::identity::check_id(value)
        .map_err(|_| StatsError::InvalidInput("invalid identity"))
}

struct Checked {
    // stratum -> cluster -> (captured mass, represented mass)
    sums: BTreeMap<String, BTreeMap<String, (f64, f64)>>,
}

fn check(design: &CaptureDesign, parcels: &[Parcel]) -> Result<Checked, StatsError> {
    match design.independent_unit {
        IndependentUnit::Parcel => {
            return Err(StatsError::Unsupported(
                "parcels within a carrier realization are not independent units",
            ))
        }
        IndependentUnit::Daughter => {
            return Err(StatsError::Unsupported(
                "daughters sharing ancestry and carrier are not independent units",
            ))
        }
        IndependentUnit::QuadraturePoint => {
            return Err(StatsError::Unsupported(
                "points of one quasi-random set are not independent replicates",
            ))
        }
        IndependentUnit::CarrierCluster | IndependentUnit::RandomizedQuadratureReplicate => {}
    }
    identity(&design.applicability)?;
    if !(design.confidence > 0.5 && design.confidence < 1.0) {
        return Err(StatsError::InvalidInput("confidence must be in (0.5, 1)"));
    }
    if design.stratum_mass_fractions.is_empty() {
        return Err(StatsError::InvalidInput("no injection strata declared"));
    }
    let mut total = 0.0;
    for (stratum, w) in &design.stratum_mass_fractions {
        identity(stratum)?;
        if !w.is_finite() || *w <= 0.0 {
            return Err(StatsError::InvalidInput(
                "stratum mass fractions must be finite and positive",
            ));
        }
        total += w;
    }
    if (total - 1.0).abs() > 1e-9 {
        return Err(StatsError::InvalidInput(
            "stratum mass fractions must sum to one",
        ));
    }
    let mut cluster_ids = BTreeSet::new();
    let mut randomizations = BTreeSet::new();
    for c in &design.clusters {
        identity(&c.cluster_id)?;
        if !cluster_ids.insert(c.cluster_id.as_str()) {
            return Err(StatsError::InvalidInput("duplicate cluster identity"));
        }
        match (design.independent_unit, &c.randomization_id) {
            (IndependentUnit::RandomizedQuadratureReplicate, Some(r)) => {
                identity(r)?;
                if !randomizations.insert(r.as_str()) {
                    return Err(StatsError::Unsupported(
                        "replicates sharing one randomization are dependent pseudo-replicates",
                    ));
                }
            }
            (IndependentUnit::RandomizedQuadratureReplicate, None) => {
                return Err(StatsError::InvalidInput(
                    "randomized replicate without its randomization",
                ))
            }
            (_, Some(_)) => {
                return Err(StatsError::InvalidInput(
                    "carrier cluster declares a quadrature randomization",
                ))
            }
            (_, None) => {}
        }
    }
    let mut parcel_ids = BTreeSet::new();
    // root -> (stratum, cluster); every daughter must stay with its root's stratum and cluster
    let mut roots: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
    let mut sums: BTreeMap<String, BTreeMap<String, (f64, f64)>> = BTreeMap::new();
    for p in parcels {
        identity(&p.parcel_id)?;
        identity(&p.root_id)?;
        if !parcel_ids.insert(p.parcel_id.as_str()) {
            return Err(StatsError::InvalidInput("duplicate parcel identity"));
        }
        if !design.stratum_mass_fractions.contains_key(&p.stratum) {
            return Err(StatsError::InvalidInput("parcel in an undeclared stratum"));
        }
        if !cluster_ids.contains(p.cluster.as_str()) {
            return Err(StatsError::InvalidInput("parcel in an undeclared cluster"));
        }
        if !p.represented_mass.is_finite() || p.represented_mass <= 0.0 {
            return Err(StatsError::InvalidInput(
                "represented mass must be finite and positive",
            ));
        }
        let home = roots
            .entry(p.root_id.as_str())
            .or_insert((p.stratum.as_str(), p.cluster.as_str()));
        if *home != (p.stratum.as_str(), p.cluster.as_str()) {
            return Err(StatsError::InvalidInput(
                "daughters must keep their root's stratum and independent cluster",
            ));
        }
        let cell = sums
            .entry(p.stratum.clone())
            .or_default()
            .entry(p.cluster.clone())
            .or_insert((0.0, 0.0));
        if p.captured {
            cell.0 += p.represented_mass;
        }
        cell.1 += p.represented_mass;
    }
    for c in &design.clusters {
        if !sums
            .values()
            .any(|by_cluster| by_cluster.contains_key(&c.cluster_id))
        {
            return Err(StatsError::InvalidInput(
                "declared cluster without represented mass",
            ));
        }
    }
    for stratum in design.stratum_mass_fractions.keys() {
        if sums.get(stratum).map_or(0, |c| c.len()) < 2 {
            return Err(StatsError::Unsupported(
                "each stratum needs represented mass in at least two independent units",
            ));
        }
    }
    if design.independent_unit == IndependentUnit::RandomizedQuadratureReplicate {
        for c in &design.clusters {
            if sums
                .values()
                .any(|by_cluster| !by_cluster.contains_key(&c.cluster_id))
            {
                return Err(StatsError::InvalidInput(
                    "each replicate must cover every stratum",
                ));
            }
        }
    }
    Ok(Checked { sums })
}

/// Estimate capture efficiency (captured represented mass / represented mass).
pub fn estimate_capture(
    design: &CaptureDesign,
    parcels: &[Parcel],
) -> Result<CaptureEstimate, StatsError> {
    let checked = check(design, parcels)?;
    let alpha = 1.0 - design.confidence;
    let mut strata = Vec::new();
    let (estimate, variance, df, interpretation) = match design.independent_unit {
        IndependentUnit::CarrierCluster => {
            // Strata are injection domains inside each independent carrier cluster, so the
            // combined estimator is linearized per cluster to keep between-stratum covariance.
            let clusters: Vec<&str> = design
                .clusters
                .iter()
                .map(|c| c.cluster_id.as_str())
                .collect();
            let c = clusters.len() as f64;
            let cell = |stratum: &str, cluster: &str| {
                checked.sums[stratum]
                    .get(cluster)
                    .copied()
                    .unwrap_or((0.0, 0.0))
            };
            let mut estimate = 0.0;
            let mut u = vec![0.0; clusters.len()];
            for (stratum, w) in &design.stratum_mass_fractions {
                let y: f64 = checked.sums[stratum].values().map(|v| v.0).sum();
                let x: f64 = checked.sums[stratum].values().map(|v| v.1).sum();
                let ratio = y / x;
                let x_bar = x / c;
                let mut own = 0.0;
                for (k, cluster) in clusters.iter().enumerate() {
                    let (yc, xc) = cell(stratum, cluster);
                    let d = (yc - ratio * xc) / x_bar;
                    own += d * d;
                    u[k] += w * d;
                }
                estimate += w * ratio;
                strata.push(StratumEstimate {
                    stratum: stratum.clone(),
                    mass_fraction: *w,
                    clusters: checked.sums[stratum].len(),
                    captured_mass: y,
                    represented_mass: x,
                    ratio,
                    variance: Some(own / (c * (c - 1.0))),
                });
            }
            let variance = u.iter().map(|d| d * d).sum::<f64>() / (c * (c - 1.0));
            (estimate, variance, clusters.len() - 1,
             "mass-weighted stratified ratio of captured to represented mass; per-cluster linearization keeping between-stratum covariance; t with clusters-1 degrees of freedom")
        }
        IndependentUnit::RandomizedQuadratureReplicate => {
            let replicates: Vec<&String> = design.clusters.iter().map(|c| &c.cluster_id).collect();
            let mut per_replicate = Vec::with_capacity(replicates.len());
            for r in &replicates {
                let mut e = 0.0;
                for (stratum, w) in &design.stratum_mass_fractions {
                    let (y, x) = checked.sums[stratum][r.as_str()];
                    e += w * y / x;
                }
                per_replicate.push(e);
            }
            for (stratum, w) in &design.stratum_mass_fractions {
                let y: f64 = checked.sums[stratum].values().map(|v| v.0).sum();
                let x: f64 = checked.sums[stratum].values().map(|v| v.1).sum();
                strata.push(StratumEstimate {
                    stratum: stratum.clone(),
                    mass_fraction: *w,
                    clusters: checked.sums[stratum].len(),
                    captured_mass: y,
                    represented_mass: x,
                    ratio: y / x,
                    variance: None,
                });
            }
            let (mean, var) = super::sample_moments(&per_replicate)?;
            (mean, var / per_replicate.len() as f64, per_replicate.len() - 1,
             "mean of independently randomized quadrature replicate estimates; between-replicate variance; t with replicates-1 degrees of freedom")
        }
        _ => unreachable!("refused in check"),
    };
    let standard_error = variance.sqrt();
    let t_critical = student_t_quantile(1.0 - alpha / 2.0, df)?;
    if !estimate.is_finite() || !standard_error.is_finite() {
        return Err(StatsError::NumericalFailure("nonfinite capture estimate"));
    }
    Ok(CaptureEstimate {
        estimate,
        standard_error,
        degrees_of_freedom: df,
        t_critical,
        lower: (estimate - t_critical * standard_error).max(0.0),
        upper: (estimate + t_critical * standard_error).min(1.0),
        strata,
        independent_unit: design.independent_unit,
        interpretation: format!("{interpretation}; interval clipped to [0, 1]"),
        applicability: design.applicability.clone(),
    })
}
