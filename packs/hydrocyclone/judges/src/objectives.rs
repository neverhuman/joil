//! V5-18-04 and V5-18-05: four bounded objectives behind hard gates, and decision profiles.
//!
//! Order of evaluation: (1) every mandatory gate must pass, (2) every objective bound must be
//! resolved, (3) baseline no-regression floors must hold, and only then (4) a preference
//! utility is computed. A missing or unresolved gate can never be offset by light mass or a
//! high utility. Weights `0.35 / 0.20 / 0.30 / 0.15` are a proposed sensitivity profile until
//! frozen; the decision records that status. Nothing here is a physical qualification.
use crate::{canonical_hash, invalid, positive, Field, JudgeError, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Mandatory gates. A candidate without an explicit verdict for each one is unresolved.
pub const REQUIRED_GATES: [&str; 7] = [
    "buckling",
    "material",
    "reserve",
    "oil_quality",
    "capacity",
    "energy",
    "mass",
];

/// The four bounds used by the decision. Each is `None` when unresolved.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Objectives {
    /// Upper bound on actual printed mass, kg.
    pub printed_mass_kg_upper: Option<f64>,
    /// Lower bound on the common-demand structural reserve, load multiplier.
    pub reserve_lower: Option<f64>,
    /// Lower bound on compliant treated capacity, m³/h.
    pub treated_capacity_lower_m3_h: Option<f64>,
    /// Upper bound on common-duty energy per compliant treated m³, J/m³.
    pub energy_upper_j_per_m3: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateRecord {
    pub id: String,
    pub objectives: Objectives,
    pub gates: BTreeMap<String, Verdict>,
}

/// Qualified baseline. Each value is an admitted field; undefined values block comparison.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub id: String,
    pub reserve_lower: Field<f64>,
    pub treated_capacity_lower_m3_h: Field<f64>,
    pub energy_upper_j_per_m3: Field<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileStatus {
    Proposed,
    Frozen,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Weights {
    pub printed_mass: f64,
    pub reserve: f64,
    pub capacity: f64,
    pub energy: f64,
}

/// Declared reservation (worst acceptable) and aspiration (target) for one objective. They are
/// the reference points of the utility and must differ in the preferred direction.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scale {
    pub reservation: f64,
    pub aspiration: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scales {
    pub printed_mass: Scale,
    pub reserve: Scale,
    pub capacity: Scale,
    pub energy: Scale,
}

/// Preference profile. `multiplicity` comparisons share `alpha` by Bonferroni splitting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionProfile {
    pub id: String,
    pub status: ProfileStatus,
    pub weights: Weights,
    pub scales: Scales,
    pub multiplicity: usize,
    pub alpha: f64,
}

impl DecisionProfile {
    /// Weights must be finite, nonnegative and sum to one; scales must be finite.
    pub fn check(&self) -> Result<(), JudgeError> {
        let w = [
            self.weights.printed_mass,
            self.weights.reserve,
            self.weights.capacity,
            self.weights.energy,
        ];
        if w.iter().any(|x| !x.is_finite() || *x < 0.0)
            || (w.iter().sum::<f64>() - 1.0).abs() > 1e-9
        {
            return Err(invalid(
                "profile weights must be nonnegative and sum to one",
            ));
        }
        if self.multiplicity == 0
            || !(self.alpha.is_finite() && self.alpha > 0.0 && self.alpha < 1.0)
        {
            return Err(invalid("multiplicity >= 1 and alpha in (0, 1) required"));
        }
        for s in [
            self.scales.printed_mass,
            self.scales.reserve,
            self.scales.capacity,
            self.scales.energy,
        ] {
            if !(s.reservation.is_finite() && s.aspiration.is_finite()) {
                return Err(invalid("scale reference points must be finite"));
            }
        }
        Ok(())
    }

    /// Bonferroni-adjusted significance for one of the declared comparisons.
    pub fn per_comparison_alpha(&self) -> Result<f64, JudgeError> {
        self.check()?;
        Ok(self.alpha / self.multiplicity as f64)
    }
}

/// Utility in `[0, 1]`: `0` at reservation, `1` at aspiration. Equal or inverted reference points
/// cannot produce a finite utility.
fn utility(x: f64, s: Scale, minimise: bool) -> Result<f64, JudgeError> {
    let (gap, moved) = if minimise {
        (s.reservation - s.aspiration, s.reservation - x)
    } else {
        (s.aspiration - s.reservation, x - s.reservation)
    };
    if !(gap.is_finite() && gap > 0.0) {
        return Err(invalid(
            "aspiration must differ from reservation in the preferred direction",
        ));
    }
    Ok((moved / gap).clamp(0.0, 1.0))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CandidateState {
    Feasible { utility: f64 },
    Infeasible { reasons: Vec<String> },
    Unresolved { reasons: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub profile_sha256: Sha256Hex,
    pub observations_sha256: Sha256Hex,
    pub decision_sha256: Sha256Hex,
    pub profile_status: ProfileStatus,
    pub states: BTreeMap<String, CandidateState>,
    /// Unique best feasible candidate; `None` on a tie or when nothing is feasible.
    pub winner: Option<String>,
}

/// Every required gate must carry a verdict; a missing one is unresolved, never a pass.
fn gate_verdict(record: &CandidateRecord) -> Verdict {
    Verdict::combine(
        REQUIRED_GATES
            .iter()
            .map(|name| match record.gates.get(*name) {
                Some(v) => v.clone(),
                None => Verdict::Unresolved(vec![format!("gate:{name}:missing")]),
            }),
    )
}

/// Resolved bounds, or the names of the unresolved ones.
struct Resolved {
    mass: f64,
    reserve: f64,
    capacity: f64,
    energy: f64,
}

fn resolve(o: &Objectives) -> Result<Resolved, Vec<String>> {
    let invalid: Vec<String> = [
        ("printed_mass", o.printed_mass_kg_upper),
        ("reserve", o.reserve_lower),
        ("capacity", o.treated_capacity_lower_m3_h),
        ("energy", o.energy_upper_j_per_m3),
    ]
    .iter()
    .filter(|(_, value)| value.is_some_and(|v| !v.is_finite() || v < 0.0))
    .map(|(name, _)| format!("objective:{name}:invalid_bound"))
    .collect();
    if !invalid.is_empty() {
        return Err(invalid);
    }
    match (
        o.printed_mass_kg_upper,
        o.reserve_lower,
        o.treated_capacity_lower_m3_h,
        o.energy_upper_j_per_m3,
    ) {
        (Some(mass), Some(reserve), Some(capacity), Some(energy)) => Ok(Resolved {
            mass,
            reserve,
            capacity,
            energy,
        }),
        (mass, reserve, capacity, energy) => Err([
            ("printed_mass", mass.is_none()),
            ("reserve", reserve.is_none()),
            ("capacity", capacity.is_none()),
            ("energy", energy.is_none()),
        ]
        .iter()
        .filter(|(_, missing)| *missing)
        .map(|(name, _)| format!("objective:{name}:unresolved"))
        .collect()),
    }
}

/// Baseline no-regression floors: capacity and reserve may not fall, energy may not rise.
fn floors(r: &Resolved, b: &Baseline) -> Verdict {
    let mut fail = Vec::new();
    let mut open = Vec::new();
    let checks = [
        ("reserve", &b.reserve_lower, r.reserve, true),
        ("capacity", &b.treated_capacity_lower_m3_h, r.capacity, true),
        ("energy", &b.energy_upper_j_per_m3, r.energy, false),
    ];
    for (name, baseline, value, higher_is_better) in checks {
        let Some(&base) = baseline.admitted() else {
            open.push(format!("baseline:{name}:not_admitted"));
            continue;
        };
        if !base.is_finite() || (name == "capacity" && base <= 0.0) {
            open.push(format!("baseline:{name}:undefined"));
        } else if (higher_is_better && value < base) || (!higher_is_better && value > base) {
            fail.push(format!("floor:{name}:regresses_baseline"));
        }
    }
    if !fail.is_empty() {
        Verdict::Fail(fail)
    } else if !open.is_empty() {
        Verdict::Unresolved(open)
    } else {
        Verdict::Pass
    }
}

fn evaluate(
    record: &CandidateRecord,
    profile: &DecisionProfile,
    baseline: &Baseline,
) -> Result<CandidateState, JudgeError> {
    let gate = gate_verdict(record);
    let resolved = resolve(&record.objectives);
    let verdict = match &resolved {
        Ok(r) => Verdict::combine([gate, floors(r, baseline)]),
        Err(open) => Verdict::combine([gate, Verdict::Unresolved(open.clone())]),
    };
    Ok(match (verdict, resolved) {
        (Verdict::Fail(reasons), _) => CandidateState::Infeasible { reasons },
        (Verdict::Unresolved(reasons), _) => CandidateState::Unresolved { reasons },
        (Verdict::Pass, Ok(r)) => {
            let s = &profile.scales;
            let w = &profile.weights;
            let utility = w.printed_mass * utility(r.mass, s.printed_mass, true)?
                + w.reserve * utility(r.reserve, s.reserve, false)?
                + w.capacity * utility(r.capacity, s.capacity, false)?
                + w.energy * utility(r.energy, s.energy, true)?;
            CandidateState::Feasible { utility }
        }
        // Resolution failures are caught above; this branch keeps the never-pass invariant.
        (Verdict::Pass, Err(open)) => CandidateState::Unresolved { reasons: open },
    })
}

/// Evaluates every candidate, then ranks feasible ones by utility. The decision identity hashes
/// the profile and the observations separately, so changing only the weights creates a new
/// decision over the same compatible observations.
pub fn decide(
    profile: &DecisionProfile,
    baseline: &Baseline,
    candidates: &[CandidateRecord],
) -> Result<Decision, JudgeError> {
    profile.check()?;
    let mut ids = BTreeSet::new();
    let mut states = BTreeMap::new();
    for record in candidates {
        if record.id.trim().is_empty() || !ids.insert(record.id.as_str()) {
            return Err(invalid("candidate ids must be nonempty and unique"));
        }
        states.insert(record.id.clone(), evaluate(record, profile, baseline)?);
    }

    let mut feasible: Vec<(&String, f64)> = states
        .iter()
        .filter_map(|(id, state)| match state {
            CandidateState::Feasible { utility } => Some((id, *utility)),
            _ => None,
        })
        .collect();
    feasible.sort_by(|a, b| b.1.total_cmp(&a.1));
    let winner = match feasible.as_slice() {
        [(id, top), rest @ ..] if rest.first().is_none_or(|(_, second)| second < top) => {
            Some(id.to_string())
        }
        _ => None,
    };

    let profile_sha256 = canonical_hash(profile)?;
    let observations_sha256 = canonical_hash(&(baseline, candidates))?;
    let decision_sha256 = canonical_hash(&(profile_sha256.as_str(), observations_sha256.as_str()))?;
    Ok(Decision {
        profile_sha256,
        observations_sha256,
        decision_sha256,
        profile_status: profile.status,
        states,
        winner,
    })
}

/// Treated-capacity gain over a qualified baseline. Undefined without a positive admitted
/// baseline; no epsilon denominator is inserted.
pub fn gain_over_baseline(candidate_m3_h: f64, baseline: &Field<f64>) -> Result<f64, JudgeError> {
    let base = baseline
        .admitted()
        .copied()
        .ok_or_else(|| invalid("gain undefined: baseline not admitted"))?;
    positive("baseline capacity", base)?;
    if !candidate_m3_h.is_finite() || candidate_m3_h < 0.0 {
        return Err(invalid("candidate capacity must be finite and nonnegative"));
    }
    let gain = candidate_m3_h / base - 1.0;
    if !gain.is_finite() {
        return Err(invalid("capacity gain overflow"));
    }
    Ok(gain)
}
