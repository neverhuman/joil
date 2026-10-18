//! Finite sample summaries and packet-local inference. A summary by itself does not
//! assert independence, coverage or power; each submodule states its applicability.
use crate::StatsError;
use statrs::distribution::ContinuousCDF;

pub mod calibration;
pub mod capture;
pub mod joint;
pub mod order;
pub mod pilot;

/// Reviewable, software-only closure record for the synthetic confirmation rig.
#[derive(Clone, Debug)]
pub struct RigProtocolClosure {
    pub containment_reviewed: bool,
    pub comparator_defined: bool,
    pub pilot_pairs: usize,
    pub blinded_aba_blocks: usize,
    pub calibration_subjects: Vec<String>,
    pub holdout_subjects: Vec<String>,
    pub calibration_batches: Vec<String>,
    pub holdout_batches: Vec<String>,
    pub owner_signed: bool,
    pub described_as_registered_or_approved: bool,
}
pub fn verify_rig_protocol_closure(p: &RigProtocolClosure) -> Result<(), StatsError> {
    if !p.containment_reviewed
        || !p.comparator_defined
        || p.pilot_pairs < 3
        || p.blinded_aba_blocks < 3
    {
        return Err(StatsError::InvalidInput(
            "protocol_closure_missing_review_comparator_power_or_blinded_blocks",
        ));
    }
    if p.calibration_subjects
        .iter()
        .any(|x| p.holdout_subjects.contains(x))
        || p.calibration_batches
            .iter()
            .any(|x| p.holdout_batches.contains(x))
    {
        return Err(StatsError::InvalidInput(
            "calibration_holdout_partition_leakage",
        ));
    }
    if p.described_as_registered_or_approved && !p.owner_signed {
        return Err(StatsError::InvalidInput(
            "unsigned_draft_cannot_be_registered_or_approved",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct TelemetryPoint {
    pub time_s: f64,
    pub value: f64,
    pub lower: f64,
    pub upper: f64,
    pub censored: bool,
}
#[derive(Clone, Debug)]
pub struct TelemetryStream {
    pub delay_s: f64,
    pub calibrated: bool,
    pub posthoc_filtered: bool,
    pub points: Vec<TelemetryPoint>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ReducedInterval {
    pub start_s: f64,
    pub end_s: f64,
    pub lower: f64,
    pub upper: f64,
    pub right_censored: bool,
}
/// Shift samples to process time and preserve every censoring interval as bounds.
pub fn reduce_delayed_censored_streams(
    streams: &[TelemetryStream],
) -> Result<Vec<Vec<ReducedInterval>>, StatsError> {
    if streams.is_empty() {
        return Err(StatsError::InvalidInput("telemetry_streams_required"));
    }
    let mut corrected = Vec::new();
    for stream in streams {
        if !stream.calibrated
            || stream.posthoc_filtered
            || !stream.delay_s.is_finite()
            || stream.delay_s < 0.0
        {
            return Err(StatsError::InvalidInput(
                "uncalibrated_filtered_or_invalid_delay_stream",
            ));
        }
        if stream.points.len() < 2 {
            return Err(StatsError::InvalidInput("two_telemetry_points_required"));
        }
        let mut rows = Vec::new();
        let mut previous = f64::NEG_INFINITY;
        for point in &stream.points {
            let time = point.time_s - stream.delay_s;
            if !time.is_finite()
                || time <= previous
                || !point.lower.is_finite()
                || !point.upper.is_finite()
                || point.lower > point.upper
            {
                return Err(StatsError::InvalidInput(
                    "invalid_or_unordered_telemetry_bounds",
                ));
            }
            rows.push((time, point.lower, point.upper, point.censored));
            previous = time;
        }
        corrected.push(rows);
    }
    let start = corrected
        .iter()
        .map(|r| r[0].0)
        .fold(f64::NEG_INFINITY, f64::max);
    let end = corrected
        .iter()
        .map(|r| r[r.len() - 1].0)
        .fold(f64::INFINITY, f64::min);
    if end <= start {
        return Err(StatsError::InvalidInput(
            "delayed_streams_have_no_common_time_window",
        ));
    }
    let mut grid = vec![start, end];
    for rows in &corrected {
        for row in rows {
            if row.0 > start && row.0 < end {
                grid.push(row.0);
            }
        }
    }
    grid.sort_by(f64::total_cmp);
    grid.dedup_by(|a, b| *a == *b);
    let mut output = Vec::new();
    for rows in corrected {
        let mut aligned = Vec::new();
        for interval in grid.windows(2) {
            let pair = rows
                .windows(2)
                .find(|w| w[0].0 <= interval[0] && w[1].0 >= interval[1])
                .ok_or(StatsError::InvalidInput("aligned_grid_extrapolation"))?;
            let (a, b) = (&pair[0], &pair[1]);
            aligned.push(ReducedInterval {
                start_s: interval[0],
                end_s: interval[1],
                lower: a.1.min(b.1),
                upper: a.2.max(b.2),
                right_censored: a.3 || b.3,
            });
        }
        output.push(aligned);
    }
    Ok(output)
}

#[derive(Clone, Debug)]
pub struct SpecimenPassport {
    pub coupon_sha256: String,
    pub ct_sha256: String,
    pub ra_um: f64,
    pub rz_um: f64,
    pub orifice_mm: f64,
    pub proof_sha256: String,
    pub tap_sha256s: Vec<String>,
    pub as_built_subject_sha256: String,
    pub computational_subject_sha256: String,
}
fn valid_digest(x: &str) -> bool {
    x.len() == 64
        && x.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn verify_specimen_passport_and_deviation_trigger(
    p: &SpecimenPassport,
) -> Result<bool, StatsError> {
    if [
        p.coupon_sha256.as_str(),
        p.ct_sha256.as_str(),
        p.proof_sha256.as_str(),
        p.as_built_subject_sha256.as_str(),
        p.computational_subject_sha256.as_str(),
    ]
    .iter()
    .any(|x| !valid_digest(x))
        || p.tap_sha256s.is_empty()
        || p.tap_sha256s.iter().any(|x| !valid_digest(x))
        || ![p.ra_um, p.rz_um, p.orifice_mm]
            .iter()
            .all(|x| x.is_finite() && *x >= 0.0)
        || p.orifice_mm == 0.0
    {
        return Err(StatsError::InvalidInput(
            "specimen_passport_evidence_or_measurement_invalid",
        ));
    }
    Ok(p.as_built_subject_sha256 != p.computational_subject_sha256)
}

#[derive(Clone, Debug)]
pub struct ConfirmationUnit {
    pub build_a1: String,
    pub build_b: String,
    pub build_a2: String,
    pub fluid_batch: String,
    pub a1: f64,
    pub b: f64,
    pub a2: f64,
    pub duty: String,
    pub band: String,
    pub pressure_policy: String,
    pub decision_profile: String,
    pub material_model: String,
    pub scenarios_pass: Vec<bool>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ConfirmationSummary {
    pub mean_pct: f64,
    pub lower_bound_pct: f64,
    pub upper_bound_pct: f64,
    pub claim_at_least_ten_pct: bool,
    pub failed_scenarios: usize,
}
/// Paired A/B/A estimate; a favorable mean is not a threshold claim without its lower bound.
pub fn analyze_preregistered_confirmation_units(
    units: &[ConfirmationUnit],
    required_scenarios: usize,
    duty: &str,
    band: &str,
    pressure: &str,
    profile: &str,
    material_model: &str,
) -> Result<ConfirmationSummary, StatsError> {
    if units.len() < 3 || required_scenarios == 0 {
        return Err(StatsError::InvalidInput(
            "three_units_and_required_scenarios_needed",
        ));
    }
    let mut builds = Vec::new();
    let mut batches = Vec::new();
    let mut effects = Vec::new();
    let mut failures = 0;
    for u in units {
        if u.duty != duty
            || u.band != band
            || u.pressure_policy != pressure
            || u.decision_profile != profile
            || u.material_model != material_model
        {
            return Err(StatsError::InvalidInput(
                "confirmation_identity_or_duty_policy_mismatch",
            ));
        }
        if u.scenarios_pass.len() != required_scenarios {
            return Err(StatsError::InvalidInput(
                "required_scenario_missing_or_extra",
            ));
        }
        failures += u.scenarios_pass.iter().filter(|pass| !**pass).count();
        if ![u.a1, u.b, u.a2].iter().all(|v| v.is_finite() && *v > 0.0) {
            return Err(StatsError::InvalidInput(
                "positive_finite_capacity_required",
            ));
        }
        let a = (u.a1 + u.a2) / 2.0;
        effects.push(100.0 * (u.b - a) / a);
        builds.extend([&u.build_a1, &u.build_b, &u.build_a2]);
        batches.push(&u.fluid_batch);
    }
    if builds.iter().any(|x| x.is_empty())
        || batches.iter().any(|x| x.is_empty())
        || builds
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != builds.len()
        || batches
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != batches.len()
    {
        return Err(StatsError::InvalidInput("repeated_build_or_fluid_batch"));
    }
    let (mean, variance) = sample_moments(&effects)?;
    let t = statrs::distribution::StudentsT::new(0.0, 1.0, (effects.len() - 1) as f64)
        .map_err(|_| StatsError::InvalidInput("invalid_degrees_of_freedom"))?
        .inverse_cdf(0.975);
    let half = t * (variance.sqrt()) / (effects.len() as f64).sqrt();
    Ok(ConfirmationSummary {
        mean_pct: mean,
        lower_bound_pct: mean - half,
        upper_bound_pct: mean + half,
        claim_at_least_ten_pct: failures == 0 && mean - half >= 10.0,
        failed_scenarios: failures,
    })
}

/// Return the arithmetic mean and unbiased sample variance. No artificial
/// variance floor converts zero variation into observed information.
pub fn sample_moments(values: &[f64]) -> Result<(f64, f64), StatsError> {
    if values.len() < 2 || values.iter().any(|v| !v.is_finite()) {
        return Err(StatsError::InvalidInput("need at least two finite values"));
    }
    let mut mean = 0.0;
    let mut m2 = 0.0;
    for (i, value) in values.iter().enumerate() {
        let delta = value - mean;
        mean += delta / (i + 1) as f64;
        m2 += delta * (value - mean);
    }
    let variance = m2 / (values.len() - 1) as f64;
    if !mean.is_finite() || !variance.is_finite() || variance < 0.0 {
        return Err(StatsError::NumericalFailure("sample moments overflow"));
    }
    Ok((mean, variance))
}

#[cfg(test)]
mod v5_30_tests {
    use super::*;
    fn digest(c: char) -> String {
        std::iter::repeat_n(c, 64).collect()
    }
    fn unit(i: usize, b: f64) -> ConfirmationUnit {
        ConfirmationUnit {
            build_a1: format!("a{i}"),
            build_b: format!("b{i}"),
            build_a2: format!("c{i}"),
            fluid_batch: format!("f{i}"),
            a1: 100.,
            b,
            a2: 100.,
            duty: "d".into(),
            band: "band".into(),
            pressure_policy: "p".into(),
            decision_profile: "profile".into(),
            material_model: "mat-model".into(),
            scenarios_pass: vec![true, true],
        }
    }
    #[test]
    fn v5_30_01_protocol_closure_and_unsigned_draft_guard() {
        let mut p = RigProtocolClosure {
            containment_reviewed: true,
            comparator_defined: true,
            pilot_pairs: 3,
            blinded_aba_blocks: 3,
            calibration_subjects: vec!["cal".into()],
            holdout_subjects: vec!["held".into()],
            calibration_batches: vec!["batch1".into()],
            holdout_batches: vec!["batch2".into()],
            owner_signed: false,
            described_as_registered_or_approved: false,
        };
        assert!(verify_rig_protocol_closure(&p).is_ok());
        p.described_as_registered_or_approved = true;
        assert!(verify_rig_protocol_closure(&p).is_err());
        p.described_as_registered_or_approved = false;
        p.holdout_batches = vec!["batch1".into()];
        assert!(verify_rig_protocol_closure(&p).is_err());
    }
    #[test]
    fn v5_30_02_delayed_censored_stream_reduction() {
        let s = TelemetryStream {
            delay_s: 2.,
            calibrated: true,
            posthoc_filtered: false,
            points: vec![
                TelemetryPoint {
                    time_s: 3.,
                    value: 0.,
                    lower: 0.,
                    upper: 1.,
                    censored: true,
                },
                TelemetryPoint {
                    time_s: 4.,
                    value: 0.,
                    lower: 0.,
                    upper: 2.,
                    censored: false,
                },
            ],
        };
        let r = reduce_delayed_censored_streams(std::slice::from_ref(&s)).unwrap();
        assert_eq!(r[0][0].start_s, 1.);
        assert!(r[0][0].right_censored);
        assert_eq!(r[0][0].upper, 2.);
        let second = TelemetryStream {
            delay_s: 1.,
            calibrated: true,
            posthoc_filtered: false,
            points: vec![
                TelemetryPoint {
                    time_s: 2.5,
                    value: 0.,
                    lower: 4.,
                    upper: 5.,
                    censored: false,
                },
                TelemetryPoint {
                    time_s: 3.,
                    value: 0.,
                    lower: 3.,
                    upper: 6.,
                    censored: false,
                },
            ],
        };
        let aligned = reduce_delayed_censored_streams(&[s.clone(), second]).unwrap();
        assert_eq!(aligned[0][0].start_s, aligned[1][0].start_s);
        assert_eq!(aligned[0][0].end_s, aligned[1][0].end_s);
        let mut bad = s;
        bad.posthoc_filtered = true;
        assert!(reduce_delayed_censored_streams(&[bad]).is_err());
    }
    #[test]
    fn v5_30_03_specimen_passport_deviation_reevaluation() {
        let p = SpecimenPassport {
            coupon_sha256: digest('a'),
            ct_sha256: digest('b'),
            ra_um: 2.,
            rz_um: 8.,
            orifice_mm: 1.4,
            proof_sha256: digest('c'),
            tap_sha256s: vec![digest('d')],
            as_built_subject_sha256: digest('e'),
            computational_subject_sha256: digest('f'),
        };
        assert_eq!(verify_specimen_passport_and_deviation_trigger(&p), Ok(true));
        let mut bad = p;
        bad.ct_sha256 = "bad".into();
        assert!(verify_specimen_passport_and_deviation_trigger(&bad).is_err());
    }
    #[test]
    fn v5_30_04_preregistered_confirmation_bound_and_negatives() {
        let units = vec![unit(1, 112.), unit(2, 112.), unit(3, 112.)];
        let r = analyze_preregistered_confirmation_units(
            &units,
            2,
            "d",
            "band",
            "p",
            "profile",
            "mat-model",
        )
        .unwrap();
        assert!(r.claim_at_least_ten_pct);
        assert!((r.mean_pct - 12.).abs() < 1e-10);
        let noisy = vec![unit(1, 110.), unit(2, 110.), unit(3, 135.)];
        let r = analyze_preregistered_confirmation_units(
            &noisy,
            2,
            "d",
            "band",
            "p",
            "profile",
            "mat-model",
        )
        .unwrap();
        assert!(r.mean_pct > 10.);
        assert!(r.lower_bound_pct < 10.);
        assert!(!r.claim_at_least_ten_pct);
        let mut bad = units;
        bad[2].fluid_batch = "f1".into();
        assert!(analyze_preregistered_confirmation_units(
            &bad,
            2,
            "d",
            "band",
            "p",
            "profile",
            "mat-model"
        )
        .is_err());
    }
}
