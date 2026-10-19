//! Surrogate feature extraction, failure probability model, active prescreen routing, and cross-family holdout qualification.

use super::*;

/// Extract the standardized 12D physics-guided feature vector directly from a `DesignProgram`.
pub fn extract_program_features(prog: &DesignProgram) -> Result<[f64; 12], SurrogateAbstention> {
    let d = prog
        .descriptors()
        .map_err(|reason| SurrogateAbstention::ExtractionFailure {
            stage: "descriptors".into(),
            reason,
        })?;
    let (ph, _) = Phenotype3D::from_program(prog).map_err(|reason| {
        SurrogateAbstention::ExtractionFailure {
            stage: "phenotype".into(),
            reason,
        }
    })?;
    let st = structure::analytic_structural_screen(prog, &ph, 29468.22, 0.001111111).map_err(
        |reason| SurrogateAbstention::ExtractionFailure {
            stage: "structural_screen".into(),
            reason,
        },
    )?;
    finite_features([
        d.swirl_flux_index,
        d.axial_pitch_sin,
        d.shear_mitigation_factor,
        d.cone_accel_persistence,
        prog.inlet_ramp.axial_drop_mm / 8.0,
        prog.inlet_ramp.radial_slot_factor,
        (ph.fluid_volume_mm3 - 1_105_802.5) / 5_000.0,
        ph.minimum_wall_clearance_mm / 3.0,
        ph.minimum_passage_diameter_mm / 10.0,
        st.max_wall_von_mises_mpa / 100.0,
        st.vane_modal_freq_ratio / 50.0,
        st.skimmer_tip_deflection_mm * 1e4,
    ])
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SurrogateAbstention {
    NumericalPredictionFailure,
    ObservedFeaturesRequired,
    InvalidIncumbent,
    DistanceOverflow,
    MissingFeatures { fields: Vec<String> },
    ExtractionFailure { stage: String, reason: String },
    NonfiniteFeatures,
    OutOfDistribution { distance: f64, support_radius: f64 },
}
impl SurrogateAbstention {
    pub(crate) fn missing(field: &str) -> Self {
        Self::MissingFeatures {
            fields: vec![field.into()],
        }
    }
}
pub(crate) fn finite_features(values: [f64; 12]) -> Result<[f64; 12], SurrogateAbstention> {
    if values.iter().all(|x| x.is_finite()) {
        Ok(values)
    } else {
        Err(SurrogateAbstention::NonfiniteFeatures)
    }
}

/// Empirical attempt-outcome model, separate from the performance regressor.
/// Counts are not probabilities of physical feasibility.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FailureModel {
    pub attempts: usize,
    pub complete_records: usize,
    pub missing_features: usize,
    pub numerical_failures: usize,
    pub infrastructure_failures: usize,
}
impl FailureModel {
    pub fn from_attempts(records: &[SurrogateRecord]) -> Self {
        let mut model = Self::default();
        for record in records {
            model.attempts += 1;
            match record.status {
                SurrogateRecordStatus::NumericalFailure => model.numerical_failures += 1,
                SurrogateRecordStatus::InfrastructureFailure => model.infrastructure_failures += 1,
                SurrogateRecordStatus::MissingFeatures => model.missing_features += 1,
                _ if record.features.is_none() => model.missing_features += 1,
                _ => model.complete_records += 1,
            }
        }
        model
    }
}

pub(crate) fn normal_cdf(z: f64) -> f64 {
    // Abramowitz & Stegun 7.1.26 rational approximation
    let sign = if z < 0.0 { -1.0 } else { 1.0 };
    let x = z.abs() / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let erf = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-x * x).exp();
    0.5 * (1.0 + sign * erf)
}

/// Active-learning acquisition score for a candidate 3D design program (`P07`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SurrogateAcquisition {
    pub posterior_mean_cout_mg_l: f64,
    pub posterior_std_cout_mg_l: f64,
    pub lcb95_cout_mg_l: f64,
    pub probability_of_improvement: f64,
    pub expected_improvement_mg_l: f64,
    pub feasibility_probability_30_mg_l: f64,
    pub constrained_expected_improvement: f64,
}

/// Pre-screen a pool of mutated 3D candidate programs using `GaussianProcessSurrogate`
/// and return the top `keep_top_k` candidates ranked by Constrained Expected Improvement.
pub fn active_prescreen_offspring(
    gp: &GaussianProcessSurrogate,
    candidates: &[DesignProgram],
    incumbent_best_cout_mg_l: f64,
    keep_top_k: usize,
) -> Vec<(DesignProgram, SurrogateAcquisition)> {
    route_offspring(gp, candidates, incumbent_best_cout_mg_l, keep_top_k).ranked
}

#[derive(Clone, Debug, Default)]
pub struct PrescreenRouting {
    pub ranked: Vec<(DesignProgram, SurrogateAcquisition)>,
    pub direct_evaluation: Vec<(DesignProgram, SurrogateAbstention)>,
}

/// Abstentions are preserved for direct evaluation and never assigned a fake score.
pub fn route_offspring(
    gp: &GaussianProcessSurrogate,
    candidates: &[DesignProgram],
    incumbent: f64,
    keep_top_k: usize,
) -> PrescreenRouting {
    let mut routing = PrescreenRouting::default();
    for candidate in candidates {
        match gp.expected_constrained_improvement(candidate, incumbent) {
            Ok(acquisition) => routing.ranked.push((candidate.clone(), acquisition)),
            Err(reason) => routing.direct_evaluation.push((candidate.clone(), reason)),
        }
    }
    routing.ranked.sort_by(|a, b| {
        b.1.constrained_expected_improvement
            .total_cmp(&a.1.constrained_expected_improvement)
    });
    routing.ranked.truncate(keep_top_k);
    routing
}

/// Per-topology-family Leave-One-Family-Out Cross-Validation metrics (`ENGINE-04`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FamilyHoldoutMetrics {
    pub held_out_family: String,
    pub test_count: usize,
    pub train_count: usize,
    pub rmse_cout_mg_l: f64,
    pub mae_cout_mg_l: f64,
    pub mean_posterior_std_mg_l: f64,
    pub coverage_within_2sigma: f64,
}

/// Exact prediction row from the existing authored-family sandbox fold fit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FamilyHoldoutPredictionRow {
    pub subject_id: String,
    pub target_cout: f64,
    pub predicted_cout: f64,
    pub sigma_cout: f64,
    pub lcb_2sigma: f64,
    pub ucb_2sigma: f64,
}

/// Raw fold membership and predictions; not independent oil/geometry labels.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FamilyHoldoutRows {
    pub topology_class: String,
    pub train_subject_ids: Vec<String>,
    pub predictions: Vec<FamilyHoldoutPredictionRow>,
}

/// Complete `ENGINE-02..05` Search & Surrogate Qualification Report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SurrogateQualificationReport {
    pub schema_version: u32,
    pub kind: String,
    pub independent_label_validation: joil_evidence::observation::Observation,
    pub total_records_input: usize,
    pub quarantined_failure_records: usize,
    pub valid_training_records: usize,
    pub in_distribution_rmse_mg_l: f64,
    #[serde(rename = "sandbox_authored_family_holdout_mean_rmse_mg_l")]
    pub ood_mean_rmse_mg_l: f64,
    #[serde(rename = "sandbox_authored_family_holdout_mean_2sigma_coverage")]
    pub ood_mean_2sigma_coverage: f64,
    #[serde(rename = "sandbox_holdout_std_exceeds_training_std")]
    pub ood_std_exceeds_in_dist_std: bool,
    pub failure_quarantine_verified: bool,
    /// Self-test of `verify_resume_compatibility` on a synthetic empty-island checkpoint with
    /// literal hashes (`V5-01-03`); reads the legacy `checkpoint_determinism_verified` name.
    #[serde(alias = "checkpoint_determinism_verified")]
    pub checkpoint_compatibility_selftest_passed: bool,
    pub evaluator_drift_rejection_verified: bool,
    pub budget_conservation_verified: bool,
    /// Real campaign resume/replay is unsupported (`V5-01-03`); never a verified record.
    #[serde(default = "legacy_campaign_replay")]
    pub campaign_replay: joil_evidence::observation::Observation,
    pub family_holdouts: Vec<FamilyHoldoutMetrics>,
}

/// Run Leave-One-Topology-Family-Out Cross-Validation (`LOFO-CV`) and `ENGINE-02..04` checks
/// across a set of evaluated 3D candidates.
pub fn run_surrogate_and_engine_qualification(
    candidates: &[Individual],
) -> Result<SurrogateQualificationReport, String> {
    run_surrogate_and_engine_qualification_with_fold_rows(candidates).map(|(report, _)| report)
}

/// Companion for raw capture: returns rows from the same fits/predictions as the
/// legacy report, preserving its quarantine, missingness and insufficient rules.
pub fn run_surrogate_and_engine_qualification_with_fold_rows(
    candidates: &[Individual],
) -> Result<(SurrogateQualificationReport, Vec<FamilyHoldoutRows>), String> {
    let mut records: Vec<SurrogateRecord> = candidates
        .iter()
        .map(SurrogateRecord::from_candidate)
        .collect();

    // Inject 2 synthetic infrastructure/numerical failure records to verify quarantine (ENGINE-04)
    records.push(SurrogateRecord {
        candidate_id: "quarantine-test-infra-fail-01".to_string(),
        topology_class: "helical-pitched-ribbon".to_string(),
        status: SurrogateRecordStatus::InfrastructureFailure,
        features: None,
        feature_failure: Some(SurrogateAbstention::missing("failed attempt")),
        admitted_label: None,
        target_cout_mg_l: None,
        target_dp_pa: None,
        target_d50_um: None,
    });
    records.push(SurrogateRecord {
        candidate_id: "quarantine-test-num-fail-02".to_string(),
        topology_class: "spiral-rifled-vortex-head".to_string(),
        status: SurrogateRecordStatus::NumericalFailure,
        features: None,
        feature_failure: Some(SurrogateAbstention::missing("failed attempt")),
        admitted_label: None,
        target_cout_mg_l: Some(f64::NAN),
        target_dp_pa: None,
        target_d50_um: None,
    });

    // Fit full GP on all valid records
    let full_gp = GaussianProcessSurrogate::fit_sandbox_prior(&records, 0.85, 0.02)?;
    let failure_quarantine_verified = full_gp.quarantined_failures == 2;

    let valid_records: Vec<SurrogateRecord> = records
        .iter()
        .filter(|r| r.status == SurrogateRecordStatus::SandboxPriorScreen)
        .cloned()
        .collect();

    let mut in_dist_sq_err = 0.0;
    let mut in_dist_std_sum = 0.0;
    for r in &valid_records {
        let pred = full_gp.predict_cout(&r.features.ok_or("missing validation features")?);
        let y = r.target_cout_mg_l.ok_or("missing validation label")?;
        in_dist_sq_err += (pred.mean - y).powi(2);
        in_dist_std_sum += pred.std_dev;
    }
    let in_dist_rmse = (in_dist_sq_err / (valid_records.len() as f64)).sqrt();
    let in_dist_mean_std = in_dist_std_sum / (valid_records.len() as f64);

    // Leave-One-Topology-Family-Out CV across all families present
    let families: BTreeSet<String> = valid_records
        .iter()
        .map(|r| r.topology_class.clone())
        .collect();
    let mut holdouts = Vec::new();
    let mut fold_rows = Vec::new();
    let mut ood_rmse_sum = 0.0;
    let mut ood_cov_sum = 0.0;
    let mut ood_std_sum = 0.0;

    for fam in &families {
        let train_subset: Vec<SurrogateRecord> = valid_records
            .iter()
            .filter(|r| &r.topology_class != fam)
            .cloned()
            .collect();
        let test_subset: Vec<SurrogateRecord> = valid_records
            .iter()
            .filter(|r| &r.topology_class == fam)
            .cloned()
            .collect();
        if train_subset.len() < 4 || test_subset.is_empty() {
            continue;
        }
        let gp = GaussianProcessSurrogate::fit_sandbox_prior(&train_subset, 0.85, 0.02)?;
        let mut sq_err = 0.0;
        let mut abs_err = 0.0;
        let mut std_sum = 0.0;
        let mut covered = 0usize;
        let mut predictions = Vec::with_capacity(test_subset.len());
        for r in &test_subset {
            let pred = gp.predict_cout(&r.features.ok_or("missing validation features")?);
            let y = r.target_cout_mg_l.ok_or("missing validation label")?;
            let err = (pred.mean - y).abs();
            sq_err += err * err;
            abs_err += err;
            std_sum += pred.std_dev;
            if y >= pred.lcb_2sigma && y <= pred.ucb_2sigma {
                covered += 1;
            }
            predictions.push(FamilyHoldoutPredictionRow {
                subject_id: r.candidate_id.clone(),
                target_cout: y,
                predicted_cout: pred.mean,
                sigma_cout: pred.std_dev,
                lcb_2sigma: pred.lcb_2sigma,
                ucb_2sigma: pred.ucb_2sigma,
            });
        }
        let n_t = test_subset.len() as f64;
        let rmse = (sq_err / n_t).sqrt();
        let mae = abs_err / n_t;
        let mean_std = std_sum / n_t;
        let cov = (covered as f64) / n_t;
        ood_rmse_sum += rmse;
        ood_cov_sum += cov;
        ood_std_sum += mean_std;
        holdouts.push(FamilyHoldoutMetrics {
            held_out_family: fam.clone(),
            test_count: test_subset.len(),
            train_count: train_subset.len(),
            rmse_cout_mg_l: rmse,
            mae_cout_mg_l: mae,
            mean_posterior_std_mg_l: mean_std,
            coverage_within_2sigma: cov,
        });
        fold_rows.push(FamilyHoldoutRows {
            topology_class: fam.clone(),
            train_subject_ids: train_subset
                .iter()
                .map(|r| r.candidate_id.clone())
                .collect(),
            predictions,
        });
    }

    if holdouts.is_empty() {
        return Err("insufficient authored-family folds; no validation metric is available".into());
    }
    let n_f = holdouts.len() as f64;
    let ood_mean_rmse = ood_rmse_sum / n_f;
    let ood_mean_cov = ood_cov_sum / n_f;
    let ood_mean_std = ood_std_sum / n_f;

    // Verify ENGINE-02 checkpoint & evaluator-drift guard and ENGINE-03 budget ledger conservation
    let ledger = BudgetLedger {
        total_attempts: valid_records.len() + 12,
        redteam_rejected: 5,
        structural_rejected: 2,
        phenotype_cache_hits: 3,
        infrastructure_quarantined: 2,
        unique_f0_evaluated: valid_records.len(),
        total_mc_realizations: valid_records.len() * 16,
    };
    let budget_conservation_verified = ledger.is_conserved();

    let ckpt = CampaignCheckpoint {
        schema_version: 1,
        evaluator_sha256: "eval-v1-sha256".to_string(),
        cuda_binary_sha256: "cuda-v1-sha256".to_string(),
        generation: 10,
        rng_states: vec![0x1234_5678, 0x9abc_def0],
        islands: vec![],
        seen_phenotype_hashes: vec![],
        ledger,
    };
    let resume_ok = ckpt
        .verify_resume_compatibility(1, "eval-v1-sha256", "cuda-v1-sha256")
        .is_ok();
    let drift_rejected = ckpt
        .verify_resume_compatibility(1, "eval-v2-mutated-sha256", "cuda-v1-sha256")
        .is_err();
    let campaign_replay = campaign_replay_status(&ckpt);

    Ok((SurrogateQualificationReport {
        schema_version: 3,
        kind: "sandbox_prior_screen_cross_validation_not_independent_ood".to_string(),
        independent_label_validation: joil_evidence::observation::Observation::Insufficient {
            reason: "Legacy candidate targets are derived from the reduced model; family folds are sandbox diagnostics".into(),
            missing: vec!["independently labelled Cout observations".into(), "observed geometry and mechanism holdouts".into()],
        },
        total_records_input: records.len(),
        quarantined_failure_records: full_gp.quarantined_failures,
        valid_training_records: valid_records.len(),
        in_distribution_rmse_mg_l: in_dist_rmse,
        ood_mean_rmse_mg_l: ood_mean_rmse,
        ood_mean_2sigma_coverage: ood_mean_cov,
        ood_std_exceeds_in_dist_std: ood_mean_std > in_dist_mean_std,
        failure_quarantine_verified,
        checkpoint_compatibility_selftest_passed: resume_ok,
        evaluator_drift_rejection_verified: drift_rejected,
        budget_conservation_verified,
        campaign_replay,
        family_holdouts: holdouts,
    }, fold_rows))
}

#[path = "qualification_fold.rs"]
mod qualification_fold;
pub use qualification_fold::*;
