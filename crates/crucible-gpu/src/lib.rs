//! Accelerator admission policy. A passing software fixture is never GPU or science admission.

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityCard {
    pub backend: String,
    pub architecture: String,
    pub precision: String,
    pub measured_end_to_end_speedup: Option<f64>,
    pub held_out_pairs: u32,
    pub decision_quality_matched: bool,
    pub gpu_lease_present: bool,
    pub model: String,
    pub level: String,
    pub envelope: String,
    pub version_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GpuAdmission {
    ReferenceOnly { reasons: Vec<&'static str> },
    Exploratory { reasons: Vec<&'static str> },
    Admitted { judge_identity: String },
}
pub type Admission = GpuAdmission;

impl CapabilityCard {
    pub fn judge_identity(&self) -> Result<String, &'static str> {
        if [
            self.backend.as_str(),
            self.architecture.as_str(),
            self.precision.as_str(),
            self.model.as_str(),
            self.level.as_str(),
            self.envelope.as_str(),
            self.version_fingerprint.as_str(),
        ]
        .iter()
        .any(|v| v.trim().is_empty())
        {
            return Err("missing_judge_identity_component");
        }
        Ok(format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.backend,
            self.architecture,
            self.precision,
            self.model,
            self.level,
            self.envelope,
            self.version_fingerprint
        ))
    }

    /// Admission is envelope-specific and revocable: changing any identity component changes the key.
    pub fn evaluate(&self, required_speedup: f64, policy_approved: bool) -> Admission {
        let mut reasons = Vec::new();
        if !required_speedup.is_finite() || required_speedup <= 0.0 {
            reasons.push("invalid_speedup_policy_limit");
        }
        let identity = match self.judge_identity() {
            Ok(value) => value,
            Err(reason) => {
                return Admission::ReferenceOnly {
                    reasons: vec![reason],
                }
            }
        };
        if !self.gpu_lease_present {
            reasons.push("no_gpu_lease");
        }
        if !self.decision_quality_matched {
            reasons.push("decision_quality_not_matched");
        }
        if self.held_out_pairs == 0 {
            reasons.push("no_held_out_pairs");
        }
        if !policy_approved {
            reasons.push("policy_not_approved");
        }
        let speed = self.measured_end_to_end_speedup;
        if !speed.is_some_and(|v| v.is_finite() && v >= required_speedup && v > 0.0) {
            reasons.push("measured_speedup_below_gate_or_absent");
        }
        if reasons.is_empty() {
            Admission::Admitted {
                judge_identity: identity,
            }
        } else if !self.gpu_lease_present {
            Admission::ReferenceOnly { reasons }
        } else {
            Admission::Exploratory { reasons }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairedEvidence {
    pub profile_max_abs: f64,
    pub grade_abs: f64,
    pub pressure_relative: f64,
    pub load_relative: f64,
    pub ranking_agreement: f64,
    pub omission_agreement: f64,
    pub uncertainty_ratio: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SubjectFamily {
    Baseline,
    Conventional,
    NovelInterior,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PairedSubjectObservation {
    pub family: SubjectFamily,
    pub profile_max_abs: f64,
    pub grade_abs: f64,
    pub pressure_relative: f64,
    pub load_relative: f64,
    pub uncertainty_ratio: f64,
    pub reference_rank: usize,
    pub candidate_rank: usize,
    pub reference_omitted: bool,
    pub candidate_omitted: bool,
    pub selected_winner_agrees: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HeldOutAdmissionReport {
    pub evidence: PairedEvidence,
    pub revoked_on_version_change: bool,
    pub admitted: bool,
}

pub fn evaluate_held_out_paired_admission(
    card: &CapabilityCard,
    pairs: &[PairedSubjectObservation],
    gates: &PairedEvidence,
    required_speedup: f64,
    policy_approved: bool,
    active_version_fingerprint: &str,
) -> Result<HeldOutAdmissionReport, &'static str> {
    if pairs.is_empty() {
        return Err("held_out_pairs_required");
    }
    if !required_speedup.is_finite() || required_speedup <= 0.0 {
        return Err("invalid_speedup_policy_limit");
    }
    let families: [SubjectFamily; 3] = [
        SubjectFamily::Baseline,
        SubjectFamily::Conventional,
        SubjectFamily::NovelInterior,
    ];
    if families
        .iter()
        .any(|f| !pairs.iter().any(|p| p.family == *f))
    {
        return Err("all_subject_families_required");
    }
    let mut e = PairedEvidence {
        profile_max_abs: 0.0,
        grade_abs: 0.0,
        pressure_relative: 0.0,
        load_relative: 0.0,
        ranking_agreement: 0.0,
        omission_agreement: 0.0,
        uncertainty_ratio: 0.0,
    };
    for p in pairs {
        let vals = [
            p.profile_max_abs,
            p.grade_abs,
            p.pressure_relative,
            p.load_relative,
            p.uncertainty_ratio,
        ];
        if vals.iter().any(|x| !x.is_finite() || *x < 0.0) {
            return Err("invalid_paired_observation");
        }
        e.profile_max_abs = e.profile_max_abs.max(vals[0]);
        e.grade_abs = e.grade_abs.max(vals[1]);
        e.pressure_relative = e.pressure_relative.max(vals[2]);
        e.load_relative = e.load_relative.max(vals[3]);
        e.uncertainty_ratio = e.uncertainty_ratio.max(vals[4]);
        e.ranking_agreement += if p.reference_rank == p.candidate_rank {
            1.0
        } else {
            0.0
        };
        e.omission_agreement += if p.reference_omitted == p.candidate_omitted {
            1.0
        } else {
            0.0
        };
    }
    e.ranking_agreement /= pairs.len() as f64;
    e.omission_agreement /= pairs.len() as f64;
    let winner_only = pairs.iter().all(|p| p.selected_winner_agrees);
    if winner_only && (!e.within_gates(gates)) {
        return Err("winner_only_agreement_insufficient");
    }
    if !e.within_gates(gates) {
        return Err("paired_evidence_gate_failed");
    }
    let revoked = card.version_fingerprint != active_version_fingerprint;
    if revoked {
        return Ok(HeldOutAdmissionReport {
            evidence: e,
            revoked_on_version_change: true,
            admitted: false,
        });
    }
    if required_speedup >= 1.3 && !policy_approved {
        return Err("policy_approval_required_for_speedup_gate");
    }
    let admission = matches!(
        card.evaluate(required_speedup, policy_approved),
        Admission::Admitted { .. }
    );
    Ok(HeldOutAdmissionReport {
        evidence: e,
        revoked_on_version_change: false,
        admitted: admission,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixtureQualificationRecord {
    pub fixture_id: String,
    pub domain: String,
    pub regime: String,
    pub observed_order: f64,
    pub max_rel_error: f64,
    pub validated_vortex_or_structure: bool,
}
pub fn qualify_numerical_and_physical_fixtures(
    fixtures: &[FixtureQualificationRecord],
    target_application: &str,
) -> Result<Vec<String>, &'static str> {
    let required = [
        "poiseuille",
        "taylor_green",
        "couette",
        "swirl",
        "oil",
        "structural_patch",
        "beam",
        "cylinder",
        "buckling",
    ];
    for tag in required {
        if !fixtures.iter().any(|f| {
            f.fixture_id.to_lowercase().contains(tag) || f.domain.to_lowercase().contains(tag)
        }) {
            return Err("required_fixture_missing");
        }
    }
    if ["turbulent_deoiling", "nonlinear_collapse"].contains(&target_application)
        && fixtures.iter().any(|f| f.regime == "laminar")
    {
        return Err("laminar_fixture_cannot_qualify_target");
    }
    if fixtures.iter().any(|f| {
        !f.observed_order.is_finite() || !f.max_rel_error.is_finite() || f.max_rel_error < 0.0
    }) {
        return Err("invalid_fixture_measurement");
    }
    Ok(fixtures.iter().map(|f| f.fixture_id.clone()).collect())
}

impl PairedEvidence {
    pub fn within_gates(&self, gates: &Self) -> bool {
        let upper_values = [
            self.profile_max_abs,
            self.grade_abs,
            self.pressure_relative,
            self.load_relative,
            self.uncertainty_ratio,
        ];
        let upper_limits = [
            gates.profile_max_abs,
            gates.grade_abs,
            gates.pressure_relative,
            gates.load_relative,
            gates.uncertainty_ratio,
        ];
        upper_values
            .iter()
            .zip(upper_limits)
            .all(|(v, l)| v.is_finite() && l.is_finite() && *v >= 0.0 && l >= 0.0 && *v <= l)
            && (0.0..=1.0).contains(&self.ranking_agreement)
            && (0.0..=1.0).contains(&gates.ranking_agreement)
            && (0.0..=1.0).contains(&self.omission_agreement)
            && (0.0..=1.0).contains(&gates.omission_agreement)
            && self.ranking_agreement.is_finite()
            && gates.ranking_agreement.is_finite()
            && self.ranking_agreement >= gates.ranking_agreement
            && self.omission_agreement.is_finite()
            && gates.omission_agreement.is_finite()
            && self.omission_agreement >= gates.omission_agreement
    }
}

pub mod alternatives;
pub use alternatives::*;
