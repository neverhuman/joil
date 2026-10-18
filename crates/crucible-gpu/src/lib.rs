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
}
