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
pub enum Admission {
    ReferenceOnly { reasons: Vec<&'static str> },
    Exploratory { reasons: Vec<&'static str> },
    Admitted { judge_identity: String },
}

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

/// Bounded reusable allocation. Capacity is fixed at construction and clear never reallocates.
#[derive(Clone, Debug)]
pub struct BoundedPool<T> {
    items: Vec<T>,
    capacity: usize,
}

impl<T> BoundedPool<T> {
    pub fn new(capacity: usize) -> Result<Self, &'static str> {
        if capacity > 1_000_000 {
            return Err("pool_capacity_exceeds_limit");
        }
        let mut items = Vec::new();
        items
            .try_reserve_exact(capacity)
            .map_err(|_| "pool_allocation_failed")?;
        Ok(Self { items, capacity })
    }
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.items.len() == self.capacity {
            Err(value)
        } else {
            self.items.push(value);
            Ok(())
        }
    }
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }
    pub fn clear(&mut self) {
        self.items.clear();
    }
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AcceleratorAlternativeBenchmark {
    pub backend_id: String,
    pub kind: String,
    pub measured_useful_cost_s: Option<f64>,
    pub assumed_gupdates_per_s: Option<f64>,
    pub hypothetical_fleet: bool,
    pub held_out_pairs: usize,
    pub decision_quality_matched: bool,
    pub gpu_lease_present: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomKernelAdrDecision {
    pub pubadr_id: String,
    pub custom_kernels_justified: bool,
    pub selected_backend: String,
    pub reference_fallback_retained: bool,
    pub rejected_alternatives: Vec<(String, &'static str)>,
}

/// Compare useful measured work; throughput assumptions never constitute admission.
pub fn evaluate_accelerator_alternatives(
    alternatives: &[AcceleratorAlternativeBenchmark],
    min_speedup: f64,
    reference_cost_s: f64,
) -> Result<CustomKernelAdrDecision, &'static str> {
    if !reference_cost_s.is_finite() || reference_cost_s <= 0.0 {
        return Err("invalid_reference_cost");
    }
    if !min_speedup.is_finite() || min_speedup <= 0.0 {
        return Err("invalid_min_speedup");
    }
    let mut decision = CustomKernelAdrDecision {
        pubadr_id: "ADR-V5-24-01".into(),
        custom_kernels_justified: false,
        selected_backend: "reference".into(),
        reference_fallback_retained: true,
        rejected_alternatives: Vec::new(),
    };
    let mut best_cost = f64::INFINITY;
    for alternative in alternatives {
        let mut reasons = Vec::new();
        if alternative.backend_id.trim().is_empty() || alternative.kind.trim().is_empty() {
            reasons.push("missing_alternative_identity");
        }
        if alternative.hypothetical_fleet {
            reasons.push("hypothetical_gpu_fleet_rejected");
        }
        if alternative.assumed_gupdates_per_s.is_some()
            && alternative.measured_useful_cost_s.is_none()
        {
            reasons.push("assumed_gupdates_without_measured_cost");
        }
        if !alternative.gpu_lease_present {
            reasons.push("gpu_lease_required");
        }
        if alternative.held_out_pairs < 10 {
            reasons.push("insufficient_held_out_pairs");
        }
        if !alternative.decision_quality_matched {
            reasons.push("decision_quality_not_matched");
        }
        let cost = alternative.measured_useful_cost_s.unwrap_or(f64::NAN);
        if !cost.is_finite() || cost <= 0.0 {
            reasons.push("invalid_or_missing_measured_cost");
        } else if reference_cost_s / cost < min_speedup {
            reasons.push("measured_speedup_below_gate");
        }
        if reasons.is_empty() && cost < best_cost {
            best_cost = cost;
            decision.selected_backend = alternative.backend_id.clone();
            decision.custom_kernels_justified = alternative.kind == "custom_kernel";
        }
        for reason in reasons {
            decision
                .rejected_alternatives
                .push((alternative.backend_id.clone(), reason));
        }
    }
    Ok(decision)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> CapabilityCard {
        CapabilityCard {
            backend: "cpu-reference".into(),
            architecture: "x86_64".into(),
            precision: "f64".into(),
            measured_end_to_end_speedup: Some(1.4),
            held_out_pairs: 12,
            decision_quality_matched: true,
            gpu_lease_present: true,
            model: "toy-lbm".into(),
            level: "screen".into(),
            envelope: "synthetic".into(),
            version_fingerprint: "abc".into(),
        }
    }

    #[test]
    fn absent_lease_forces_reference_fallback_even_if_other_values_look_good() {
        let mut c = card();
        c.gpu_lease_present = false;
        assert!(matches!(
            c.evaluate(1.3, true),
            Admission::ReferenceOnly { .. }
        ));
    }
    #[test]
    fn measured_and_approved_pair_can_admit_only_its_identity() {
        assert!(matches!(
            card().evaluate(1.3, true),
            Admission::Admitted { .. }
        ));
        let mut changed = card();
        changed.precision = "fp16".into();
        assert_ne!(card().judge_identity(), changed.judge_identity());
        assert!(matches!(
            changed.evaluate(1.3, false),
            Admission::Exploratory { .. }
        ));
    }
    #[test]
    fn missing_or_assumed_speedup_is_not_admitted() {
        let mut c = card();
        c.measured_end_to_end_speedup = None;
        assert!(!matches!(c.evaluate(1.3, true), Admission::Admitted { .. }));
        let mut c = card();
        c.measured_end_to_end_speedup = Some(f64::NAN);
        assert!(!matches!(c.evaluate(1.3, true), Admission::Admitted { .. }));
    }
    #[test]
    fn paired_evidence_rejects_nonfinite_and_over_limit_controls() {
        let lim = PairedEvidence {
            profile_max_abs: 1.0,
            grade_abs: 0.01,
            pressure_relative: 0.01,
            load_relative: 0.01,
            ranking_agreement: 0.99,
            omission_agreement: 0.99,
            uncertainty_ratio: 1.0,
        };
        let mut good = lim.clone();
        good.profile_max_abs = 0.8;
        assert!(good.within_gates(&lim));
        good.grade_abs = f64::NAN;
        assert!(!good.within_gates(&lim));
        good.grade_abs = 0.01;
        good.omission_agreement = 0.9;
        assert!(!good.within_gates(&lim));
    }
    #[test]
    fn g009_guard_invalid_policy_limits_cannot_admit_acceleration() {
        for bad in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(!matches!(
                card().evaluate(bad, true),
                Admission::Admitted { .. }
            ));
        }
    }
    #[test]
    fn g009_guard_negative_errors_and_impossible_agreement_are_refused() {
        let limit = PairedEvidence {
            profile_max_abs: 1.0,
            grade_abs: 0.1,
            pressure_relative: 0.1,
            load_relative: 0.1,
            ranking_agreement: 0.9,
            omission_agreement: 0.9,
            uncertainty_ratio: 1.0,
        };
        assert!(limit.within_gates(&limit));
        let mut bad = limit.clone();
        bad.grade_abs = -1.0;
        assert!(!bad.within_gates(&limit));
        let mut bad = limit.clone();
        bad.ranking_agreement = 2.0;
        assert!(!bad.within_gates(&limit));
    }
    #[test]
    fn pool_enforces_fixed_bound() {
        let mut p = BoundedPool::new(1).unwrap();
        assert_eq!(p.capacity(), 1);
        assert!(p.push(3).is_ok());
        assert_eq!(p.push(4), Err(4));
        p.clear();
        assert_eq!(p.push(5), Ok(()));
        assert!(BoundedPool::<u64>::new(1_000_001).is_err());
    }

    #[test]
    fn v5_24_04_held_out_paired_admission_families_revocation_and_winner_only_guard() {
        let gates = PairedEvidence {
            profile_max_abs: 0.1,
            grade_abs: 0.1,
            pressure_relative: 0.1,
            load_relative: 0.1,
            ranking_agreement: 0.9,
            omission_agreement: 0.9,
            uncertainty_ratio: 1.0,
        };
        let pairs = [
            SubjectFamily::Baseline,
            SubjectFamily::Conventional,
            SubjectFamily::NovelInterior,
        ]
        .map(|family| PairedSubjectObservation {
            family,
            profile_max_abs: 0.01,
            grade_abs: 0.01,
            pressure_relative: 0.01,
            load_relative: 0.01,
            uncertainty_ratio: 0.2,
            reference_rank: 1,
            candidate_rank: 1,
            reference_omitted: false,
            candidate_omitted: false,
            selected_winner_agrees: true,
        });
        let r =
            evaluate_held_out_paired_admission(&card(), &pairs, &gates, 1.3, true, "abc").unwrap();
        assert!(r.admitted && !r.revoked_on_version_change);
        assert_eq!(
            evaluate_held_out_paired_admission(&card(), &pairs[..2], &gates, 1.3, true, "abc"),
            Err("all_subject_families_required")
        );
        let mut changed = card();
        changed.version_fingerprint = "old".into();
        let r =
            evaluate_held_out_paired_admission(&changed, &pairs, &gates, 1.3, true, "abc").unwrap();
        assert!(r.revoked_on_version_change && !r.admitted);
        let mut bad = pairs.clone();
        bad[2].pressure_relative = 0.2;
        assert_eq!(
            evaluate_held_out_paired_admission(&card(), &bad, &gates, 1.3, true, "abc"),
            Err("winner_only_agreement_insufficient")
        );
    }

    #[test]
    fn v5_24_03_mrt_benchmarks_and_laminar_cannot_qualify_turbulent_or_collapse() {
        let ids = [
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
        let fixtures: Vec<_> = ids
            .iter()
            .map(|id| FixtureQualificationRecord {
                fixture_id: (*id).into(),
                domain: (*id).into(),
                regime: "laminar".into(),
                observed_order: 2.0,
                max_rel_error: 0.01,
                validated_vortex_or_structure: true,
            })
            .collect();
        assert!(qualify_numerical_and_physical_fixtures(&fixtures, "laminar_reference").is_ok());
        assert_eq!(
            qualify_numerical_and_physical_fixtures(&fixtures, "turbulent_deoiling"),
            Err("laminar_fixture_cannot_qualify_target")
        );
        assert_eq!(
            qualify_numerical_and_physical_fixtures(&fixtures, "nonlinear_collapse"),
            Err("laminar_fixture_cannot_qualify_target")
        );
        let mut bad = fixtures;
        bad[0].observed_order = f64::NAN;
        assert_eq!(
            qualify_numerical_and_physical_fixtures(&bad, "laminar_reference"),
            Err("invalid_fixture_measurement")
        );
    }

    #[test]
    fn v5_24_01_adr_benchmarks_alternatives_retains_reference_fallback_and_rejects_assumed_gupdates_or_hypothetical_fleet(
    ) {
        let good = AcceleratorAlternativeBenchmark {
            backend_id: "synthetic-custom".into(),
            kind: "custom_kernel".into(),
            measured_useful_cost_s: Some(2.0),
            assumed_gupdates_per_s: None,
            hypothetical_fleet: false,
            held_out_pairs: 10,
            decision_quality_matched: true,
            gpu_lease_present: true,
        };
        let admitted =
            evaluate_accelerator_alternatives(std::slice::from_ref(&good), 2.0, 4.0).unwrap();
        assert!(admitted.custom_kernels_justified && admitted.reference_fallback_retained);
        assert_eq!(admitted.selected_backend, good.backend_id);
        for index in 0..8 {
            let mut bad = good.clone();
            let expected = match index {
                0 => {
                    bad.hypothetical_fleet = true;
                    "hypothetical_gpu_fleet_rejected"
                }
                1 => {
                    bad.measured_useful_cost_s = None;
                    bad.assumed_gupdates_per_s = Some(99.0);
                    "assumed_gupdates_without_measured_cost"
                }
                2 => {
                    bad.gpu_lease_present = false;
                    "gpu_lease_required"
                }
                3 => {
                    bad.held_out_pairs = 9;
                    "insufficient_held_out_pairs"
                }
                4 => {
                    bad.decision_quality_matched = false;
                    "decision_quality_not_matched"
                }
                5 => {
                    bad.measured_useful_cost_s = Some(f64::NAN);
                    "invalid_or_missing_measured_cost"
                }
                6 => {
                    bad.measured_useful_cost_s = Some(0.0);
                    "invalid_or_missing_measured_cost"
                }
                _ => {
                    bad.measured_useful_cost_s = Some(3.0);
                    "measured_speedup_below_gate"
                }
            };
            let result = evaluate_accelerator_alternatives(&[bad], 2.0, 4.0).unwrap();
            assert!(!result.custom_kernels_justified);
            assert!(result.reference_fallback_retained);
            assert_eq!(result.selected_backend, "reference");
            assert!(result
                .rejected_alternatives
                .iter()
                .any(|(_, reason)| *reason == expected));
        }
        let mut library = good.clone();
        library.kind = "library".into();
        library.backend_id = "library".into();
        library.measured_useful_cost_s = Some(1.0);
        let result = evaluate_accelerator_alternatives(&[good, library], 2.0, 4.0).unwrap();
        assert_eq!(result.selected_backend, "library");
        assert!(!result.custom_kernels_justified);
        for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(evaluate_accelerator_alternatives(&[], invalid, 4.0).is_err());
            assert!(evaluate_accelerator_alternatives(&[], 2.0, invalid).is_err());
        }
        assert!(
            !evaluate_accelerator_alternatives(&[], 2.0, 4.0)
                .unwrap()
                .custom_kernels_justified
        );
    }
}
