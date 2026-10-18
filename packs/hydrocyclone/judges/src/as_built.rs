//! V5-20-03 and V5-20-04: as-built subject identity, metrology bindings and correlated bounds.
//!
//! The nominal design and the scanned as-built part are two subjects with two identities, two
//! masses and deviations measured between them. A CT voxel pitch, a calibration uncertainty and
//! a measured Ra are different observables: a CT budget yields a dimensional accuracy only from
//! its declared components (see [`crate::metrology`]), and Ra yields an equivalent sand roughness
//! only through an admitted finish calibration (see [`crate::surface`]). Corner enumeration over
//! deterministic tolerances gives bounded scenarios; a 95th-percentile claim needs an admitted
//! i.i.d. sample, and corners never provide one.
use crate::metrology::{verify_dimensional_accuracy, verify_wall_detectable, CtBudget};
use crate::surface::{equivalent_sand_roughness, FaceFinish, RaToKs, Roughness};
use crate::{invalid, Field, JudgeError, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The nominal design and the as-built part, each with its own digest. They must differ.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectIdentity {
    pub nominal_sha256: Sha256Hex,
    pub as_built_sha256: Sha256Hex,
}

impl SubjectIdentity {
    pub fn check(&self) -> Result<(), JudgeError> {
        if self.nominal_sha256 == self.as_built_sha256 {
            return Err(invalid(
                "the as-built subject needs its own identity, distinct from the nominal",
            ));
        }
        Ok(())
    }
}

/// Nominal and as-built masses in kg; their difference is recorded separately.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Masses {
    pub nominal_kg: f64,
    pub as_built_kg: f64,
}

/// One measured feature: nominal and as-built dimension in mm and the admitted physics-rerun
/// threshold for that feature's shape change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureMeasurement {
    pub feature_id: String,
    pub nominal_mm: f64,
    pub as_built_mm: f64,
    pub rerun_threshold_mm: Field<f64>,
}

/// The as-built partition: identities, masses and deviations kept as separate fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsBuiltPartition {
    pub identity: SubjectIdentity,
    pub nominal_kg: f64,
    pub as_built_kg: f64,
    pub mass_deviation_kg: f64,
    pub deviations_mm: BTreeMap<String, f64>,
    /// Features whose shape change exceeds its admitted threshold and must be rerun through physics.
    pub physics_reruns: Vec<String>,
    pub verdict: Verdict,
}

pub fn build_partition(
    identity: &SubjectIdentity,
    masses: &Masses,
    features: &[FeatureMeasurement],
) -> Result<AsBuiltPartition, JudgeError> {
    identity.check()?;
    for (name, value) in [
        ("nominal mass", masses.nominal_kg),
        ("as-built mass", masses.as_built_kg),
    ] {
        if !(value.is_finite() && value > 0.0) {
            return Err(invalid(format!("{name} must be finite and positive")));
        }
    }
    let mut ids = BTreeSet::new();
    let mut deviations_mm = BTreeMap::new();
    let mut physics_reruns = Vec::new();
    let mut open = Vec::new();
    for f in features {
        if f.feature_id.trim().is_empty() || !ids.insert(f.feature_id.as_str()) {
            return Err(invalid("empty or duplicate feature id"));
        }
        if !(f.nominal_mm.is_finite() && f.as_built_mm.is_finite()) {
            return Err(invalid("feature dimensions must be finite"));
        }
        let deviation = f.as_built_mm - f.nominal_mm;
        deviations_mm.insert(f.feature_id.clone(), deviation);
        match f.rerun_threshold_mm.admitted() {
            None => open.push(format!("{}:rerun_threshold_not_admitted", f.feature_id)),
            Some(threshold) if !(threshold.is_finite() && *threshold > 0.0) => {
                open.push(format!(
                    "{}:rerun_threshold_not_finite_positive",
                    f.feature_id
                ));
            }
            Some(threshold) if deviation.abs() > *threshold => {
                physics_reruns.push(f.feature_id.clone());
            }
            Some(_) => {}
        }
    }
    physics_reruns.sort();
    let mut reasons: Vec<String> = physics_reruns
        .iter()
        .map(|id| format!("physics_rerun_required:{id}"))
        .collect();
    reasons.extend(open);
    let verdict = if reasons.is_empty() {
        Verdict::Pass
    } else {
        Verdict::Unresolved(reasons)
    };
    Ok(AsBuiltPartition {
        identity: identity.clone(),
        nominal_kg: masses.nominal_kg,
        as_built_kg: masses.as_built_kg,
        mass_deviation_kg: masses.as_built_kg - masses.nominal_kg,
        deviations_mm,
        physics_reruns,
        verdict,
    })
}

/// Registration residual of scan to nominal, RMS in mm, against an admitted limit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registration {
    pub residual_rms_mm: f64,
    pub limit_mm: Field<f64>,
}

pub fn judge_registration(registration: &Registration) -> Verdict {
    let Some(limit) = registration.limit_mm.admitted().copied() else {
        return Verdict::Unresolved(vec!["registration_limit:not_admitted".into()]);
    };
    let residual = registration.residual_rms_mm;
    if !(residual.is_finite() && residual >= 0.0 && limit.is_finite() && limit > 0.0) {
        return Verdict::Unresolved(vec!["registration:invalid".into()]);
    }
    if residual <= limit {
        Verdict::Pass
    } else {
        Verdict::Fail(vec![format!(
            "registration:residual_{residual:.4}mm_exceeds_{limit:.4}mm"
        )])
    }
}

/// Dimensional accuracy claim for one as-built subject. The CT budget must declare each component.
pub fn dimensional_claim(
    identity: &SubjectIdentity,
    budget: &CtBudget,
    coverage_k: f64,
    required_um: &Field<f64>,
) -> Result<Verdict, JudgeError> {
    identity.check()?;
    Ok(verify_dimensional_accuracy(budget, coverage_k, required_um))
}

/// Feature detectability claim for one as-built subject.
pub fn detectability_claim(
    identity: &SubjectIdentity,
    budget: &CtBudget,
    coverage_k: f64,
    wall_um: f64,
    min_voxels: &Field<f64>,
) -> Result<Verdict, JudgeError> {
    identity.check()?;
    Ok(verify_wall_detectable(
        budget, coverage_k, wall_um, min_voxels,
    ))
}

/// Equivalent sand roughness of one as-built face. Ra alone never becomes `ks`.
pub fn roughness_claim(
    identity: &SubjectIdentity,
    face: &FaceFinish,
    calibrations: &BTreeMap<String, Field<RaToKs>>,
) -> Result<Roughness, JudgeError> {
    identity.check()?;
    Ok(equivalent_sand_roughness(face, calibrations))
}

/// Whether a deterministic tolerance, or a probabilistic distribution, drives a factor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Uncertainty {
    Deterministic,
    Probabilistic,
}

/// A bounded input factor. Factors sharing a `group` are perfectly correlated and move to the
/// same end of their intervals together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundedFactor {
    pub name: String,
    pub group: String,
    pub low: f64,
    pub high: f64,
    pub kind: Uncertainty,
}

/// Largest number of correlated groups enumerated; 2^16 corner evaluations at most.
pub const MAX_CORNER_GROUPS: usize = 16;

/// Bounded scenario from the corners of correlated deterministic tolerances. It is an envelope,
/// not a distribution.
#[derive(Clone, Debug, PartialEq)]
pub struct CornerBounds {
    pub lower: f64,
    pub upper: f64,
    pub corners: usize,
    pub groups: usize,
}

/// Enumerate the corners of the correlated groups and report the response range. A probabilistic
/// factor cannot enter corner enumeration: it needs a distribution.
pub fn corner_bounds<F: Fn(&[f64]) -> f64>(
    factors: &[BoundedFactor],
    response: F,
) -> Result<CornerBounds, JudgeError> {
    if factors.is_empty() {
        return Err(invalid("corner enumeration needs at least one factor"));
    }
    let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, factor) in factors.iter().enumerate() {
        if factor.kind != Uncertainty::Deterministic {
            return Err(invalid(format!(
                "{}: a probabilistic factor needs a distribution, not corners",
                factor.name
            )));
        }
        if !(factor.low.is_finite() && factor.high.is_finite() && factor.low <= factor.high) {
            return Err(invalid(format!(
                "{}: needs finite ordered bounds",
                factor.name
            )));
        }
        groups.entry(factor.group.as_str()).or_default().push(index);
    }
    if groups.len() > MAX_CORNER_GROUPS {
        return Err(invalid("too many correlated groups for corner enumeration"));
    }
    let count = groups.len();
    let mut values = vec![0.0; factors.len()];
    let (mut lower, mut upper) = (f64::INFINITY, f64::NEG_INFINITY);
    for mask in 0..(1usize << count) {
        for (group_index, members) in groups.values().enumerate() {
            let high = ((mask >> group_index) & 1) == 1;
            for &i in members {
                values[i] = if high {
                    factors[i].high
                } else {
                    factors[i].low
                };
            }
        }
        let y = response(&values);
        if !y.is_finite() {
            return Err(invalid("response is not finite at a corner"));
        }
        lower = lower.min(y);
        upper = upper.max(y);
    }
    Ok(CornerBounds {
        lower,
        upper,
        corners: 1 << count,
        groups: count,
    })
}

/// Smallest i.i.d. sample size whose maximum bounds the 95th percentile with 95 % confidence:
/// the smallest n with 1 - 0.95^n >= 0.95.
pub const DISTRIBUTION_FREE_95_95_DRAWS: usize = 59;

/// What supports a 95th-percentile claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleBasis {
    /// Corner scenarios from a bounded envelope; never a distribution.
    Corners(usize),
    /// Independent draws; `admitted` means the draw design and its source are admitted.
    IidDraws { n: usize, admitted: bool },
}

pub fn p95_claim(basis: SampleBasis) -> Verdict {
    match basis {
        SampleBasis::Corners(n) => Verdict::Unresolved(vec![format!(
            "p95:{n}_corner_scenarios_are_not_a_distribution"
        )]),
        SampleBasis::IidDraws {
            admitted: false, ..
        } => Verdict::Unresolved(vec!["p95:iid_draws_not_admitted".into()]),
        SampleBasis::IidDraws { n, .. } if n < DISTRIBUTION_FREE_95_95_DRAWS => {
            Verdict::Unresolved(vec![format!(
                "p95:distribution_free_95_95_needs_{DISTRIBUTION_FREE_95_95_DRAWS}_draws_has_{n}"
            )])
        }
        SampleBasis::IidDraws { .. } => Verdict::Pass,
    }
}
