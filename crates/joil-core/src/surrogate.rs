//! Search Engine Qualification, Checkpoint/Restart, Budget Ledger & GP Surrogate (`P07`, `ENGINE-02..05`).
//!
//! Implements:
//! - `ENGINE-02`: Checkpoint compatibility checks (`CampaignCheckpoint`): schema-version, evaluator
//!   and CUDA label drift rejection plus ledger conservation. No code path saves, restores or replays
//!   a campaign; real resume is unsupported until native kill/resume tests exist (`V5-01-03`).
//! - `ENGINE-03`: Exhaustive evaluation budget ledger (`BudgetLedger`) accounting for every F0 attempt,
//!   red-team rejection, P08 structural rejection, phenotype deduplication cache hit, and MC sample.
//! - `ENGINE-04`: Matérn-5/2 GP with explicit missingness, failure quarantine and abstention.
//!   Legacy candidate targets train only the named sandbox prior-screen model. Its authored-family
//!   folds are software diagnostics, not independently labelled OOD validation.

use crate::evolve::Individual;
use crate::phenotype::Phenotype3D;
use crate::program::DesignProgram;
use crate::redteam;
use crate::structure;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Exhaustive evaluation budget ledger (`ENGINE-03`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetLedger {
    pub total_attempts: usize,
    pub redteam_rejected: usize,
    pub structural_rejected: usize,
    pub phenotype_cache_hits: usize,
    pub infrastructure_quarantined: usize,
    pub unique_f0_evaluated: usize,
    pub total_mc_realizations: usize,
}

impl BudgetLedger {
    /// Verify the exact budget conservation identity:
    /// `total_attempts == redteam_rejected + structural_rejected + phenotype_cache_hits + infrastructure_quarantined + unique_f0_evaluated`.
    pub fn is_conserved(&self) -> bool {
        self.total_attempts
            == self.redteam_rejected
                + self.structural_rejected
                + self.phenotype_cache_hits
                + self.infrastructure_quarantined
                + self.unique_f0_evaluated
    }
}

/// Checkpoint compatibility record for multi-island search (`ENGINE-02`). Its compatibility check
/// compares labels and ledger arithmetic only; it does not restore or replay a campaign.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CampaignCheckpoint {
    pub schema_version: u32,
    pub evaluator_sha256: String,
    pub cuda_binary_sha256: String,
    pub generation: usize,
    pub rng_states: Vec<u64>,
    pub islands: Vec<Vec<Individual>>,
    pub seen_phenotype_hashes: Vec<String>,
    pub ledger: BudgetLedger,
}

impl CampaignCheckpoint {
    /// Validate checkpoint compatibility before resuming (`ENGINE-02`).
    /// Rejects silent resume if schema version, evaluator hash, or CUDA binary hash drifted.
    pub fn verify_resume_compatibility(
        &self,
        expected_schema_version: u32,
        expected_evaluator_sha256: &str,
        expected_cuda_binary_sha256: &str,
    ) -> Result<(), String> {
        if self.schema_version != expected_schema_version {
            return Err(format!(
                "checkpoint schema_version drift: checkpoint={} vs expected={}",
                self.schema_version, expected_schema_version
            ));
        }
        if self.evaluator_sha256 != expected_evaluator_sha256 {
            return Err(format!(
                "evaluator_drift_detected: checkpoint evaluator_sha256={} != expected={}",
                self.evaluator_sha256, expected_evaluator_sha256
            ));
        }
        if self.cuda_binary_sha256 != expected_cuda_binary_sha256 {
            return Err(format!(
                "cuda_binary_drift_detected: checkpoint cuda_binary_sha256={} != expected={}",
                self.cuda_binary_sha256, expected_cuda_binary_sha256
            ));
        }
        if !self.ledger.is_conserved() {
            return Err("checkpoint budget ledger violates conservation identity".into());
        }
        Ok(())
    }
}

pub(crate) fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Campaign replay status for `ckpt` (`V5-01-03`). Never a verified record: real resume and replay
/// are unsupported until a native kill/resume/replay test exists. Missing campaign state is listed.
pub fn campaign_replay_status(
    ckpt: &CampaignCheckpoint,
) -> joil_evidence::observation::Observation {
    let mut missing = vec!["native kill/resume/replay test of a real campaign".to_string()];
    if ckpt.islands.iter().all(|island| island.is_empty()) {
        missing.push("non-empty island populations".into());
    }
    if ckpt.seen_phenotype_hashes.is_empty() {
        missing.push("phenotype deduplication state".into());
    }
    if !is_sha256_hex(&ckpt.evaluator_sha256) {
        missing.push("content-addressed evaluator SHA-256".into());
    }
    if !is_sha256_hex(&ckpt.cuda_binary_sha256) {
        missing.push("content-addressed CUDA binary SHA-256".into());
    }
    joil_evidence::observation::Observation::Insufficient {
        reason:
            "campaign resume and replay are unsupported; the checkpoint check compares labels only"
                .into(),
        missing,
    }
}

pub(crate) fn legacy_campaign_replay() -> joil_evidence::observation::Observation {
    joil_evidence::observation::Observation::Insufficient {
        reason: "legacy record: no campaign was saved, restored or replayed".into(),
        missing: vec!["native kill/resume/replay test of a real campaign".into()],
    }
}

/// Status of an evaluation record submitted to the surrogate dataset (`ENGINE-04`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurrogateRecordStatus {
    SandboxPriorScreen,
    IndependentObservation,
    MissingFeatures,
    NumericalFailure,
    InfrastructureFailure,
}

/// Training/validation record for the physics-guided surrogate (`ENGINE-04`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SurrogateRecord {
    pub candidate_id: String,
    pub topology_class: String,
    pub status: SurrogateRecordStatus,
    pub features: Option<[f64; 12]>,
    pub feature_failure: Option<SurrogateAbstention>,
    #[serde(skip)]
    admitted_label: Option<joil_evidence::training::AdmittedSample>,
    pub target_cout_mg_l: Option<f64>,
    pub target_dp_pa: Option<f64>,
    pub target_d50_um: Option<f64>,
}

impl SurrogateRecord {
    /// Extract a standardized 12D physics-guided feature vector from an `Individual`.
    pub fn from_candidate(cand: &Individual) -> Self {
        let extracted = (|| {
            let ph = cand
                .phenotype
                .as_ref()
                .ok_or_else(|| SurrogateAbstention::missing("phenotype"))?;
            let st = cand
                .structural_qualification
                .as_ref()
                .ok_or_else(|| SurrogateAbstention::missing("structural_screen"))?;
            let d = &cand.descriptors;
            finite_features([
                d.swirl_flux_index,
                d.axial_pitch_sin,
                d.shear_mitigation_factor,
                d.cone_accel_persistence,
                cand.program.inlet_ramp.axial_drop_mm / 8.0,
                cand.program.inlet_ramp.radial_slot_factor,
                (ph.fluid_volume_mm3 - 1_105_802.5) / 5_000.0,
                ph.minimum_wall_clearance_mm / 3.0,
                ph.minimum_passage_diameter_mm / 10.0,
                st.max_wall_von_mises_mpa / 100.0,
                st.vane_modal_freq_ratio / 50.0,
                st.skimmer_tip_deflection_mm * 1e4,
            ])
        })();
        let (features, feature_failure) = match extracted {
            Ok(values) => (Some(values), None),
            Err(reason) => (None, Some(reason)),
        };
        Self {
            candidate_id: cand.id.clone(),
            topology_class: cand.topology_class.clone(),
            status: if features.is_some() {
                SurrogateRecordStatus::SandboxPriorScreen
            } else {
                SurrogateRecordStatus::MissingFeatures
            },
            features,
            feature_failure,
            admitted_label: None,
            target_cout_mg_l: Some(cand.mc_robustness.cout_p95_mg_l),
            target_dp_pa: Some(cand.nominal_metrics.dp_u_pa),
            target_d50_um: Some(cand.mc_robustness.d50_p95_um),
        }
    }
}

/// Posterior prediction with epistemic uncertainty `(mean, std_dev)` (`ENGINE-04`).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PosteriorPrediction {
    pub mean: f64,
    pub std_dev: f64,
    pub lcb_2sigma: f64,
    pub ucb_2sigma: f64,
}

/// Matérn-5/2 Gaussian Process surrogate with automatic failure quarantine (`ENGINE-04`).
#[derive(Clone, Debug)]
pub struct GaussianProcessSurrogate {
    length_scale: f64,
    signal_variance: f64,
    noise_variance: f64,
    pub quarantined_failures: usize,
    pub failure_model: FailureModel,
    support_radius: f64,
    independent_labels: bool,
    train_x: Vec<[f64; 12]>,
    y_mean_cout: f64,
    y_std_cout: f64,
    alpha_cout: Vec<f64>,
    chol_l: Vec<Vec<f64>>,
}

#[allow(clippy::needless_range_loop)]
impl GaussianProcessSurrogate {
    /// Rechecks the complete, live evidence closure on every training call.
    /// Neither a submitted status string nor deserialization can confer label admission.
    pub fn fit_independent(
        registry: &joil_evidence::claims::Registry,
        requirements: &[joil_evidence::identity::ContentRef],
        feature_contract: &joil_evidence::identity::ContentRef,
        inputs: &[joil_evidence::training::TrainingInput],
        length_scale: f64,
        noise_variance: f64,
    ) -> Result<Self, String> {
        joil_evidence::identity::unique_ids(inputs.iter().map(|input| input.label.id.as_str()))?;
        let records: Result<Vec<_>, String> = inputs
            .iter()
            .map(|input| {
                let sample = joil_evidence::training::admit_sample(
                    registry,
                    requirements,
                    feature_contract,
                    input,
                )?;
                Ok(SurrogateRecord {
                    candidate_id: sample.id().into(),
                    topology_class: "observed-independent-subject".into(),
                    status: SurrogateRecordStatus::IndependentObservation,
                    features: Some(*sample.features()),
                    feature_failure: None,
                    target_cout_mg_l: Some(sample.target().get()),
                    target_dp_pa: None,
                    target_d50_um: None,
                    admitted_label: Some(sample),
                })
            })
            .collect();
        Self::fit_records(&records?, length_scale, noise_variance, false)
    }

    pub fn uses_independent_labels(&self) -> bool {
        self.independent_labels
    }

    /// Matérn-5/2 covariance kernel $k(\mathbf{x}, \mathbf{x}')$.
    pub fn matern52(&self, a: &[f64; 12], b: &[f64; 12]) -> f64 {
        let mut d2 = 0.0;
        for i in 0..12 {
            let diff = (a[i] - b[i]) / self.length_scale;
            d2 += diff * diff;
        }
        let r = d2.sqrt();
        let s5r = 5.0_f64.sqrt() * r;
        self.signal_variance * (1.0 + s5r + (5.0 / 3.0) * d2) * (-s5r).exp()
    }

    /// Fit the Matérn-5/2 GP on a dataset, strictly quarantining any `NumericalFailure`
    /// or `InfrastructureFailure` records (`ENGINE-04`).
    pub fn fit(
        records: &[SurrogateRecord],
        length_scale: f64,
        noise_variance: f64,
    ) -> Result<Self, String> {
        // Independently admitted labels are supplied by the EV-09 dataset boundary.
        if records
            .iter()
            .any(|r| r.status == SurrogateRecordStatus::SandboxPriorScreen)
        {
            return Err(
                "prior-derived labels are sandbox only; independent observation labels required"
                    .into(),
            );
        }
        Self::fit_records(records, length_scale, noise_variance, false)
    }

    /// Explicit software/search diagnostic model. Never independent physical evidence.
    pub fn fit_sandbox_prior(
        records: &[SurrogateRecord],
        length_scale: f64,
        noise_variance: f64,
    ) -> Result<Self, String> {
        Self::fit_records(records, length_scale, noise_variance, true)
    }

    pub(crate) fn fit_records(
        records: &[SurrogateRecord],
        length_scale: f64,
        noise_variance: f64,
        sandbox: bool,
    ) -> Result<Self, String> {
        if !length_scale.is_finite()
            || length_scale <= 0.0
            || !noise_variance.is_finite()
            || noise_variance < 0.0
        {
            return Err("invalid GP hyperparameters".into());
        }
        let mut valid = Vec::new();
        let mut quarantined = 0usize;
        for rec in records {
            match rec.status {
                SurrogateRecordStatus::IndependentObservation if !sandbox => {
                    let admitted = rec
                        .admitted_label
                        .as_ref()
                        .ok_or("independent label authority missing")?;
                    if rec.features.as_ref() != Some(admitted.features())
                        || rec.target_cout_mg_l != Some(admitted.target().get())
                    {
                        return Err("admitted training record was mutated".into());
                    }
                    valid.push((*admitted.features(), admitted.target().get()));
                }
                SurrogateRecordStatus::SandboxPriorScreen if sandbox => {
                    let cout = rec
                        .target_cout_mg_l
                        .ok_or_else(|| "valid record missing target_cout_mg_l".to_string())?;
                    let features = rec
                        .features
                        .ok_or("missing features cannot enter performance regression")?;
                    if !cout.is_finite() || features.iter().any(|v| !v.is_finite()) {
                        return Err(
                            "non-finite feature or target in sandbox prior-screen record".into(),
                        );
                    }
                    valid.push((features, cout));
                }
                SurrogateRecordStatus::NumericalFailure
                | SurrogateRecordStatus::InfrastructureFailure
                | SurrogateRecordStatus::MissingFeatures => {
                    quarantined += 1;
                }
                _ => return Err("independent label admission not established".into()),
            }
        }
        if valid.len() < 4 {
            return Err("at least 4 valid physical records required to fit GP surrogate".into());
        }

        let n = valid.len();
        let y_mean = valid.iter().map(|(_, y)| *y).sum::<f64>() / (n as f64);
        let y_var = valid.iter().map(|(_, y)| (y - y_mean).powi(2)).sum::<f64>() / (n as f64);
        let y_std = y_var.sqrt().max(1e-6);
        if !y_mean.is_finite() || !y_std.is_finite() {
            return Err("nonfinite training reduction".into());
        }

        let mut gp = Self {
            length_scale: length_scale.max(0.05),
            signal_variance: 1.0,
            noise_variance: noise_variance.max(1e-6),
            quarantined_failures: quarantined,
            failure_model: FailureModel::from_attempts(records),
            support_radius: (length_scale * 12.0_f64.sqrt()).max(1e-6),
            independent_labels: !sandbox,
            train_x: valid.iter().map(|(x, _)| *x).collect(),
            y_mean_cout: y_mean,
            y_std_cout: y_std,
            alpha_cout: vec![0.0; n],
            chol_l: vec![vec![0.0; n]; n],
        };

        // Cholesky factorization K + sigma_n^2 I = L * L^T
        let mut l = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..=i {
                let mut sum = gp.matern52(&gp.train_x[i], &gp.train_x[j]);
                if i == j {
                    sum += gp.noise_variance;
                }
                for k in 0..j {
                    sum -= l[i][k] * l[j][k];
                }
                if i == j {
                    if !sum.is_finite() || sum <= 0.0 {
                        return Err("GP factorization failed; no prediction available".into());
                    }
                    l[i][j] = sum.sqrt();
                } else {
                    l[i][j] = sum / l[j][j];
                }
            }
        }

        // Solve L * z = y_norm, then L^T * alpha = z
        let mut z = vec![0.0; n];
        for i in 0..n {
            let mut sum = (valid[i].1 - y_mean) / y_std;
            for k in 0..i {
                sum -= l[i][k] * z[k];
            }
            z[i] = sum / l[i][i];
        }
        let mut alpha = vec![0.0; n];
        for i in (0..n).rev() {
            let mut sum = z[i];
            for k in (i + 1)..n {
                sum -= l[k][i] * alpha[k];
            }
            alpha[i] = sum / l[i][i];
        }

        gp.chol_l = l;
        gp.alpha_cout = alpha;
        Ok(gp)
    }

    pub fn check_support(&self, x: &[f64; 12]) -> Result<(), SurrogateAbstention> {
        finite_features(*x)?;
        let distance = self
            .train_x
            .iter()
            .map(|row| {
                row.iter()
                    .zip(x)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt()
            })
            .fold(f64::INFINITY, f64::min);
        if !distance.is_finite() {
            return Err(SurrogateAbstention::DistanceOverflow);
        }
        if distance > self.support_radius {
            Err(SurrogateAbstention::OutOfDistribution {
                distance,
                support_radius: self.support_radius,
            })
        } else {
            Ok(())
        }
    }

    pub fn predict_in_domain(
        &self,
        x: &[f64; 12],
    ) -> Result<PosteriorPrediction, SurrogateAbstention> {
        self.check_support(x)?;
        let prediction = self.predict_cout(x);
        if [
            prediction.mean,
            prediction.std_dev,
            prediction.lcb_2sigma,
            prediction.ucb_2sigma,
        ]
        .iter()
        .any(|x| !x.is_finite())
        {
            return Err(SurrogateAbstention::NumericalPredictionFailure);
        }
        Ok(prediction)
    }

    /// Predict posterior mean and epistemic standard deviation at `x` (`ENGINE-04`).
    pub(crate) fn predict_cout(&self, x: &[f64; 12]) -> PosteriorPrediction {
        let n = self.train_x.len();
        let mut k_star = vec![0.0; n];
        let mut norm_mean = 0.0;
        for i in 0..n {
            k_star[i] = self.matern52(x, &self.train_x[i]);
            norm_mean += k_star[i] * self.alpha_cout[i];
        }
        // Solve L * v = k_star for posterior variance k(x, x) - v^T v
        let mut v = vec![0.0; n];
        for i in 0..n {
            let mut sum = k_star[i];
            for k in 0..i {
                sum -= self.chol_l[i][k] * v[k];
            }
            v[i] = sum / self.chol_l[i][i];
        }
        let v_norm2: f64 = v.iter().map(|vi| vi * vi).sum();
        let k_xx = self.signal_variance + self.noise_variance;
        let norm_var = (k_xx - v_norm2).max(1e-9);

        let mean = self.y_mean_cout + self.y_std_cout * norm_mean;
        let std_dev = self.y_std_cout * norm_var.sqrt();
        PosteriorPrediction {
            mean,
            std_dev,
            lcb_2sigma: mean - 2.0 * std_dev,
            ucb_2sigma: mean + 2.0 * std_dev,
        }
    }

    /// Evaluate Constrained Expected Improvement (`CEI`) and Lower Confidence Bound (`LCB95`)
    /// for active-learning offspring pre-screening (`P07`, `ENGINE-04`).
    pub fn expected_constrained_improvement(
        &self,
        prog: &DesignProgram,
        incumbent_best_cout_mg_l: f64,
    ) -> Result<SurrogateAcquisition, SurrogateAbstention> {
        if self.independent_labels {
            return Err(SurrogateAbstention::ObservedFeaturesRequired);
        }
        if !incumbent_best_cout_mg_l.is_finite() {
            return Err(SurrogateAbstention::InvalidIncumbent);
        }
        let x = extract_program_features(prog)?;
        let pred = self.predict_in_domain(&x)?;
        let sigma = pred.std_dev.max(1e-6);
        let z_imp = (incumbent_best_cout_mg_l - pred.mean) / sigma;
        let phi_pdf = (-0.5 * z_imp * z_imp).exp() / (2.0 * std::f64::consts::PI).sqrt();
        let phi_cdf = normal_cdf(z_imp);
        let ei = ((incumbent_best_cout_mg_l - pred.mean) * phi_cdf + sigma * phi_pdf).max(0.0);

        let requirement = joil_evidence::campaign::Requirements::from_toml(include_bytes!(
            "../../../requirements/joil-hydrocyclone-v1.toml"
        ))
        .map_err(|reason| SurrogateAbstention::ExtractionFailure {
            stage: "requirements".into(),
            reason,
        })?;
        let z_feas = (requirement.thresholds.quality_limit_mg_l.get() - pred.mean) / sigma;
        let p_feas = normal_cdf(z_feas).clamp(1e-4, 1.0);
        // Combine feasibility-weighted EI with an epistemic exploration term
        let cei = p_feas * (ei + 0.15 * sigma);

        Ok(SurrogateAcquisition {
            posterior_mean_cout_mg_l: pred.mean,
            posterior_std_cout_mg_l: pred.std_dev,
            lcb95_cout_mg_l: pred.mean - 1.95996 * pred.std_dev,
            probability_of_improvement: phi_cdf,
            expected_improvement_mg_l: ei,
            feasibility_probability_30_mg_l: p_feas,
            constrained_expected_improvement: cei,
        })
    }
}

#[path = "surrogate/qualification.rs"]
mod qualification;

pub use self::qualification::*;
