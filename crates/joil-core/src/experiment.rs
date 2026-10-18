//! Experimental Rig Protocol, Statistical Power, As-Built CT Metrology & Holdout Gate
//! (`P06`, `EXPERIMENT-01`..`EXPERIMENT-06`, `CAD-09`, `METRIC-07`).
//!
//! Implements deterministic verification and statistical qualification gates for:
//! - `EXPERIMENT-01` (`verify_instrument_calibration`): Solvent-extraction GC-FID/IR reference
//!   vs. online UV fluorescence calibration curve with oil-batch SHA-256 binding and LOQ/LOD
//!   zero-censoring (`METRIC-07`).
//! - `EXPERIMENT-02` (`analyze_blinded_aba_crossover`): Randomized blinded `A/B/A` block crossover
//!   analysis on independent fluid/rig preparations, explicitly rejecting high-speed camera frames
//!   or pseudo-time iterations disguised as independent replicates.
//! - `EXPERIMENT-03` (`analyze_prospective_aba_crossover`, `required_blocks_for_power`): paired
//!   block qualification against a frozen prospective plan (independent pilot variance, declared
//!   useful effect, alpha and power; pinned t quantiles). The unplanned legacy analysis reports
//!   no power and never qualifies.
//! - `EXPERIMENT-04` (`verify_holdout_isolation`): Cryptographic holdout geometry/operating-point
//!   separation audit blocking surrogate training leakage or repeated holdout queries.
//! - `EXPERIMENT-05` (`evaluate_as_built_ct_metrology`): X-ray CT dimensional metrology + $R_a \to k_s$
//!   hydrodynamic roughness conversion (`CAD-09`), blocking oversized printed overflow bores from
//!   claiming geometric separation superiority.
//! - `EXPERIMENT-06` (`verify_shared_duty_comparator`): Enforces matched feed DSD, reject split $r$,
//!   temperature, and $\Delta p$ budget across CT-35 baseline, candidate, and commercial comparator.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Immutable, compiled campaign contract. Physical admission remains separate.
pub fn research_requirements() -> Result<&'static joil_evidence::campaign::Requirements, String> {
    static REQUIREMENTS: std::sync::OnceLock<
        Result<joil_evidence::campaign::Requirements, String>,
    > = std::sync::OnceLock::new();
    REQUIREMENTS
        .get_or_init(|| {
            joil_evidence::campaign::Requirements::from_toml(include_bytes!(
                "../../../requirements/joil-hydrocyclone-v1.toml"
            ))
        })
        .as_ref()
        .map_err(Clone::clone)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CampaignAdmissionReceipt {
    pub schema: String,
    pub mode: joil_evidence::campaign::CampaignMode,
    pub requirements: joil_evidence::identity::ContentRef,
    pub objective_sha256: String,
    pub executed_objectives: Vec<joil_evidence::campaign::ObjectiveSpec>,
    pub unresolved_claim_gates: Vec<String>,
}

/// The legacy prior screen has no trusted physical P0 receipt resolver. Sandbox
/// runs are explicit; claim-bearing mode must refuse before any computation.
pub fn admit_search_campaign(
    mode: joil_evidence::campaign::CampaignMode,
) -> Result<CampaignAdmissionReceipt, String> {
    let requirements = research_requirements()?;
    let objective_sha256 = requirements.objective_hash()?;
    joil_evidence::campaign::admit_campaign(
        mode,
        requirements,
        &requirements.objectives,
        &objective_sha256,
        &[],
        |_| joil_evidence::Verdict::Incomplete,
    )?;
    Ok(CampaignAdmissionReceipt {
        schema: "joil.campaign.v1".into(),
        mode,
        requirements: joil_evidence::identity::ContentRef::from_bytes(
            &requirements.id,
            &requirements.revision,
            include_bytes!("../../../requirements/joil-hydrocyclone-v1.toml"),
        ),
        objective_sha256,
        executed_objectives: requirements.objectives.clone(),
        unresolved_claim_gates: joil_evidence::campaign::REQUIRED_P0_GATES
            .iter()
            .map(|id| (*id).into())
            .collect(),
    })
}

/// Mutually exclusive proposal-budget categories. `evaluations` counts only
/// unrepaired evaluated proposals; actual evaluations also include repaired ones.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProposalBudget {
    pub budget: usize,
    /// Separately charged common comparator evaluations outside the proposal slots.
    #[serde(default)]
    pub reference_evaluations: usize,
    pub proposals_invalid: usize,
    pub proposals_repaired: usize,
    pub evaluations: usize,
    pub operator_statistics: std::collections::BTreeMap<String, OperatorStatistics>,
    pub proposal_receipts: Vec<crate::program::VariationOutcome>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OperatorStatistics {
    pub attempts: usize,
    pub invalid: usize,
    pub repaired: usize,
    pub evaluated: usize,
}

impl ProposalBudget {
    pub fn record(&mut self, outcome: crate::program::VariationOutcome) {
        self.budget += 1;
        let statistics = self
            .operator_statistics
            .entry(outcome.operator_id.clone())
            .or_default();
        statistics.attempts += 1;
        if outcome.failure.is_some() || outcome.child.is_none() {
            self.proposals_invalid += 1;
            statistics.invalid += 1;
        } else if !outcome.repairs.is_empty() {
            self.proposals_repaired += 1;
            statistics.repaired += 1;
        } else {
            self.evaluations += 1;
            statistics.evaluated += 1;
        }
        self.proposal_receipts.push(outcome);
    }

    pub fn absorb(&mut self, other: Self) {
        self.reference_evaluations += other.reference_evaluations;
        for receipt in other.proposal_receipts {
            self.record(receipt);
        }
    }
}

/// `EXPERIMENT-01` & `METRIC-07`: Calibration curve and LOQ/LOD zero-censoring result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstrumentCalibrationAudit {
    pub oil_batch_sha256: String,
    pub reference_method: String,
    pub r_squared: f64,
    pub lod_mg_l: f64,
    pub loq_mg_l: f64,
    pub raw_reading_mg_l: f64,
    pub censored_upper_bound_mg_l: f64,
    pub cross_oil_transfer_rejected: bool,
    pub passed: bool,
}

pub fn verify_instrument_calibration(
    calibrated_oil_sha256: &str,
    tested_oil_sha256: &str,
    r_squared: f64,
    lod_mg_l: f64,
    loq_mg_l: f64,
    raw_reading_mg_l: f64,
) -> Result<InstrumentCalibrationAudit, String> {
    if calibrated_oil_sha256.is_empty() || calibrated_oil_sha256 != tested_oil_sha256 {
        return Err(
            "EXPERIMENT-01 negative control triggered: fluorescence calibration transferred across mismatched oil batches"
                .into(),
        );
    }
    if !r_squared.is_finite() || r_squared < 0.995 || lod_mg_l <= 0.0 || loq_mg_l <= lod_mg_l {
        return Err("invalid calibration R^2 or non-monotonic LOD/LOQ limits".into());
    }
    if !raw_reading_mg_l.is_finite() {
        return Err("non-finite instrument concentration reading".into());
    }
    // METRIC-07: Readings below LOQ are never reported as 0.0 mg/L; they are censored at LOQ.
    let censored = raw_reading_mg_l.max(loq_mg_l);

    Ok(InstrumentCalibrationAudit {
        oil_batch_sha256: tested_oil_sha256.to_string(),
        reference_method: "GC-FID_ISO_9377-2_solvent_extraction".to_string(),
        r_squared,
        lod_mg_l,
        loq_mg_l,
        raw_reading_mg_l,
        censored_upper_bound_mg_l: censored,
        cross_oil_transfer_rejected: true,
        passed: censored >= loq_mg_l && r_squared >= 0.995,
    })
}

/// Single `A/B/A` crossover block measurement (`A1` baseline, `B` candidate, `A2` baseline return).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbaCrossoverBlock {
    pub block_id: String,
    pub independent_preparation: bool,
    pub baseline_a1_qmax_m3_h: f64,
    pub candidate_b_qmax_m3_h: f64,
    pub baseline_a2_qmax_m3_h: f64,
    pub split_mismatch_abs: f64,
}

/// `EXPERIMENT-02` & `EXPERIMENT-03`: descriptive paired `A/B/A` summary without a protocol.
/// Power is never derived from the observed effect; qualification needs
/// [`analyze_prospective_aba_crossover`] with a frozen prospective plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairedTrialPowerAudit {
    pub n_blocks: usize,
    pub mean_baseline_qmax_m3_h: f64,
    pub mean_candidate_qmax_m3_h: f64,
    pub mean_paired_gain_fraction: f64,
    /// Actual sample SD; no artificial floor.
    pub paired_sd_gain_fraction: f64,
    /// Pinned-t one-sided 97.5% lower bound; `None` when the sample variance is zero.
    pub lcb95_paired_gain_fraction: Option<f64>,
    /// Always `None`: an unplanned analysis has no sample plan.
    pub required_blocks_for_90pct_power: Option<usize>,
    pub frames_as_replicates_rejected: bool,
    pub underpowered_n3_rejected: bool,
    /// Always `false` without a prospective design.
    pub passed_10pct_lcb95_gate: bool,
    pub power_status: String,
}

pub use crucible_stats::analysis::pilot::{ConfirmationResult, ProspectiveDesign, SamplePlan};

/// Qualification of a blinded `A/B/A` trial against a frozen prospective plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProspectiveTrialAudit {
    pub plan: SamplePlan,
    pub confirmation: ConfirmationResult,
    pub mean_baseline_qmax_m3_h: f64,
    pub mean_candidate_qmax_m3_h: f64,
    pub frames_as_replicates_rejected: bool,
}

/// Minimum paired `A/B/A` blocks for one-sided $\alpha = 0.025$ and power $0.90$ against a
/// declared margin above the threshold (`EXPERIMENT-03`). Both inputs must be prospective
/// design values, never estimates from the confirmation data. Uses the pinned t quantiles.
pub fn required_blocks_for_power(
    sd_gain: f64,
    effect_margin_above_threshold: f64,
) -> Result<usize, String> {
    crucible_stats::analysis::pilot::required_blocks(
        sd_gain,
        effect_margin_above_threshold,
        0.025,
        0.90,
        5,
    )
    .map_err(|e| e.to_string())
}

fn aba_gains(blocks: &[AbaCrossoverBlock]) -> Result<(Vec<f64>, f64, f64), String> {
    if blocks.is_empty() {
        return Err("at least one ABA block required".into());
    }
    for b in blocks {
        if !b.independent_preparation {
            return Err(
                "EXPERIMENT-02 negative control triggered: camera frames or time-series sub-samples cannot be treated as independent experimental preparations"
                    .into(),
            );
        }
        if !b.split_mismatch_abs.is_finite() || b.split_mismatch_abs > 0.0002 {
            return Err(format!(
                "block {} violates frozen reject split tolerance +-0.0002",
                b.block_id
            ));
        }
    }
    if blocks.len() < 5 {
        return Err(
            "EXPERIMENT-03 negative control triggered: fewer than 5 independent ABA blocks cannot establish statistical power"
                .into(),
        );
    }
    let n = blocks.len() as f64;
    let mut gains = Vec::with_capacity(blocks.len());
    let mut base_sum = 0.0;
    let mut cand_sum = 0.0;
    for b in blocks {
        let a_bar = 0.5 * (b.baseline_a1_qmax_m3_h + b.baseline_a2_qmax_m3_h);
        if !a_bar.is_finite() || a_bar <= 0.0 || !b.candidate_b_qmax_m3_h.is_finite() {
            return Err("invalid baseline or candidate Q_max in ABA block".into());
        }
        base_sum += a_bar;
        cand_sum += b.candidate_b_qmax_m3_h;
        gains.push((b.candidate_b_qmax_m3_h - a_bar) / a_bar);
    }
    Ok((gains, base_sum / n, cand_sum / n))
}

/// Describe blinded `A/B/A` crossover blocks (`EXPERIMENT-02` & `EXPERIMENT-03`) without
/// fabricating power: there is no prospective protocol, so the audit never qualifies.
pub fn analyze_blinded_aba_crossover(
    blocks: &[AbaCrossoverBlock],
) -> Result<PairedTrialPowerAudit, String> {
    let (gains, mean_baseline, mean_candidate) = aba_gains(blocks)?;
    let n = gains.len();
    let (mean_gain, var_gain) =
        crucible_stats::analysis::sample_moments(&gains).map_err(|e| e.to_string())?;
    let sd_gain = var_gain.sqrt();
    let lcb95 = if var_gain > 0.0 {
        let t_975 = crucible_stats::distribution::student_t_quantile(0.975, n - 1)
            .map_err(|e| e.to_string())?;
        Some(mean_gain - t_975 * sd_gain / (n as f64).sqrt())
    } else {
        None
    };
    Ok(PairedTrialPowerAudit {
        n_blocks: n,
        mean_baseline_qmax_m3_h: mean_baseline,
        mean_candidate_qmax_m3_h: mean_candidate,
        mean_paired_gain_fraction: mean_gain,
        paired_sd_gain_fraction: sd_gain,
        lcb95_paired_gain_fraction: lcb95,
        required_blocks_for_90pct_power: None,
        frames_as_replicates_rejected: true,
        underpowered_n3_rejected: true,
        passed_10pct_lcb95_gate: false,
        power_status: "unplanned: no prospective design; use analyze_prospective_aba_crossover"
            .into(),
    })
}

/// Qualify blinded `A/B/A` blocks against a frozen prospective plan. The paired effect is
/// `(B - mean(A1, A2)) / mean(A1, A2)`; existing independence, split, five-block and
/// positive-baseline gates are kept, and the plan must not fall below five blocks.
pub fn analyze_prospective_aba_crossover(
    blocks: &[AbaCrossoverBlock],
    plan: &SamplePlan,
) -> Result<ProspectiveTrialAudit, String> {
    if plan.required_blocks < 5 {
        return Err(
            "EXPERIMENT-03 negative control triggered: a plan below 5 independent ABA blocks cannot establish statistical power"
                .into(),
        );
    }
    let (gains, mean_baseline, mean_candidate) = aba_gains(blocks)?;
    let confirmation_blocks: Vec<crucible_stats::analysis::pilot::ConfirmationBlock> = blocks
        .iter()
        .zip(gains)
        .map(
            |(b, paired_effect)| crucible_stats::analysis::pilot::ConfirmationBlock {
                block_id: b.block_id.clone(),
                independent_preparation: b.independent_preparation,
                paired_effect,
            },
        )
        .collect();
    let confirmation = crucible_stats::analysis::pilot::confirm(plan, &confirmation_blocks)
        .map_err(|e| e.to_string())?;
    Ok(ProspectiveTrialAudit {
        plan: plan.clone(),
        confirmation,
        mean_baseline_qmax_m3_h: mean_baseline,
        mean_candidate_qmax_m3_h: mean_candidate,
        frames_as_replicates_rejected: true,
    })
}

/// `EXPERIMENT-04`: Holdout geometry and operating-condition isolation verifier.
pub fn verify_holdout_isolation(
    training_digests: &[String],
    holdout_digests: &[String],
    holdout_evaluation_count: usize,
) -> Result<bool, String> {
    if training_digests.is_empty() || holdout_digests.is_empty() {
        return Err("training and holdout digest sets must be non-empty".into());
    }
    if holdout_evaluation_count > 1 {
        return Err(
            "EXPERIMENT-04 negative control triggered: repeated holdout tuning/evaluation (>1 query) invalidates holdout"
                .into(),
        );
    }
    let train_set: BTreeSet<&String> = training_digests.iter().collect();
    for h in holdout_digests {
        if train_set.contains(h) {
            return Err(format!(
                "EXPERIMENT-04 negative control triggered: holdout digest {h} leaked into surrogate training set"
            ));
        }
    }
    Ok(true)
}

/// `EXPERIMENT-05` & `CAD-09`: As-built X-ray CT metrology and roughness $R_a \to k_s$ audit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AsBuiltMetrologyAudit {
    pub part_serial: String,
    pub nominal_overflow_do_mm: f64,
    pub measured_overflow_do_mm: f64,
    pub overflow_bore_deviation_mm: f64,
    pub measured_ra_um: f64,
    pub measured_rz_um: f64,
    pub ks_lower_um: f64,
    pub ks_nominal_um: f64,
    pub ks_upper_um: f64,
    pub raw_ra_equals_ks_rejected: bool,
    pub oversized_bore_confounder_rejected: bool,
    pub passed: bool,
}

pub fn evaluate_as_built_ct_metrology(
    part_serial: &str,
    nominal_do_mm: f64,
    measured_do_mm: f64,
    measured_ra_um: f64,
    measured_rz_um: f64,
    treat_ra_directly_as_ks_mistake: bool,
) -> Result<AsBuiltMetrologyAudit, String> {
    if treat_ra_directly_as_ks_mistake {
        return Err(
            "CAD-09 / EXPERIMENT-05 negative control triggered: profilometer Ra silently treated as equivalent sand-grain roughness ks"
                .into(),
        );
    }
    let dev = measured_do_mm - nominal_do_mm;
    if dev > 0.025 {
        return Err(format!(
            "EXPERIMENT-05 negative control triggered: oversized printed overflow bore (+{dev:.4} mm > +0.025 mm) confounds reject split and oil capture"
        ));
    }
    if dev.abs() > 0.035 || measured_ra_um <= 0.0 || measured_rz_um < measured_ra_um {
        return Err("CT bore tolerance exceeded or invalid Ra/Rz roughness measurements".into());
    }
    // Adams-Bons-Taylor / Flack LPBF Ti-6Al-4V conversion interval: k_s in [2.0 * Ra, 0.95 * Rz]
    let ks_lower = 2.0 * measured_ra_um;
    let ks_nom = 3.1 * measured_ra_um;
    let ks_upper = (0.95 * measured_rz_um).max(ks_nom * 1.15);

    Ok(AsBuiltMetrologyAudit {
        part_serial: part_serial.to_string(),
        nominal_overflow_do_mm: nominal_do_mm,
        measured_overflow_do_mm: measured_do_mm,
        overflow_bore_deviation_mm: dev,
        measured_ra_um,
        measured_rz_um,
        ks_lower_um: ks_lower,
        ks_nominal_um: ks_nom,
        ks_upper_um: ks_upper,
        raw_ra_equals_ks_rejected: true,
        oversized_bore_confounder_rejected: true,
        passed: dev.abs() <= 0.025 && ks_upper <= 65.0,
    })
}

/// `EXPERIMENT-06`: Commercial comparator shared-duty audit.
pub fn verify_shared_duty_comparator(
    candidate_duty_sha256: &str,
    ct35_duty_sha256: &str,
    commercial_comparator_duty_sha256: Option<&str>,
    claim_industry_superiority: bool,
) -> Result<bool, String> {
    if candidate_duty_sha256 != ct35_duty_sha256 {
        return Err("candidate and CT-35 baseline duty digests do not match".into());
    }
    if claim_industry_superiority {
        match commercial_comparator_duty_sha256 {
            Some(comp_sha) if comp_sha == candidate_duty_sha256 => Ok(true),
            _ => Err(
                "EXPERIMENT-06 negative control triggered: CT-35 patent reconstruction alone cannot be claimed as commercial industry superiority without matched commercial comparator evidence"
                    .into(),
            ),
        }
    } else {
        Ok(true)
    }
}
