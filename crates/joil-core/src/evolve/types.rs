//! Evolutionary search configuration, candidate metrics, and campaign summary types.

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineHydraulics {
    pub q_in_m3_s: f64,
    pub q_over_m3_s: f64,
    pub q_under_m3_s: f64,
    pub split_ratio: f64,
    pub mass_imbalance_rel: f64,
    pub dp_u_pa: f64,
    pub dp_o_pa: f64,
    pub pdr: f64,
    pub specific_hydraulic_loss_j_m3: f64,
    pub underflow_reverse_m3_s: f64,
    pub overflow_reverse_m3_s: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    #[serde(default = "sandbox_mode")]
    pub campaign_mode: joil_evidence::campaign::CampaignMode,
    pub seed: u64,
    pub islands: usize,
    pub pop_per_island: usize,
    pub generations: usize,
    pub migration_interval: usize,
    pub mc_samples: usize,
    pub crossover_rate: f64,
    /// External CUDA audit binary; `None` requests no GPU (`V5-01-04`). Callers set it only from an
    /// explicit user opt-in with a GPU lease.
    pub cuda_bin_path: Option<PathBuf>,
    /// Optional explicit VRAM workspace target for the CUDA audit; never implied (`V5-01-04`).
    #[serde(default)]
    pub gpu_vram_target_pct: Option<f64>,
    pub baseline_hydraulics: BaselineHydraulics,
}

pub(crate) fn sandbox_mode() -> joil_evidence::campaign::CampaignMode {
    joil_evidence::campaign::CampaignMode::Sandbox
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            campaign_mode: sandbox_mode(),
            seed: 20261015,
            islands: 4,
            pop_per_island: 24,
            generations: 35,
            migration_interval: 5,
            mc_samples: 16,
            crossover_rate: 0.58,
            cuda_bin_path: None,
            gpu_vram_target_pct: None,
            baseline_hydraulics: BaselineHydraulics {
                q_in_m3_s: 0.001111111,
                q_over_m3_s: 2.206626e-5,
                q_under_m3_s: 0.0010890499,
                split_ratio: 0.019859632,
                mass_imbalance_rel: 4.6440003e-6,
                dp_u_pa: 29468.22,
                dp_o_pa: 208358.5,
                pdr: 7.108312,
                specific_hydraulic_loss_j_m3: 33709.55,
                underflow_reverse_m3_s: 0.0,
                overflow_reverse_m3_s: 0.0,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateMetrics {
    pub q_in_m3_h: f64,
    pub split_ratio: f64,
    pub dp_u_pa: f64,
    pub dp_o_pa: f64,
    pub pdr: f64,
    pub specific_energy_j_m3: f64,
    pub max_entry_shear_s_inv: f64,
    pub hinze_dmax_um: f64,
    pub short_circuit_fraction: f64,
    pub reverse_core_coherence: f64,
    pub grade_efficiency_bins: BTreeMap<String, f64>,
    pub d50_um: f64,
    pub d90_um: f64,
    pub overall_oil_capture: f64,
    pub cout_mg_l: f64,
    pub passes_30_mg_l: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloSummary {
    pub samples: usize,
    pub pass_count: usize,
    pub sampled_pass_fraction_ci95: [f64; 2],
    pub sampling_interpretation: String,
    pub tail_quantiles: BTreeMap<String, metrics::QuantileEstimate>,
    #[serde(rename = "cout_sample_screen_lower_mg_l", alias = "cout_p05_mg_l")]
    pub cout_p05_mg_l: f64,
    pub cout_p50_mg_l: f64,
    #[serde(rename = "cout_sample_screen_upper_mg_l", alias = "cout_p95_mg_l")]
    pub cout_p95_mg_l: f64,
    #[serde(rename = "capture_sample_screen_lower", alias = "capture_p05")]
    pub capture_p05: f64,
    pub capture_p50: f64,
    #[serde(rename = "capture_sample_screen_upper", alias = "capture_p95")]
    pub capture_p95: f64,
    #[serde(rename = "d50_sample_screen_lower_um", alias = "d50_p05_um")]
    pub d50_p05_um: f64,
    pub d50_p50_um: f64,
    #[serde(rename = "d50_sample_screen_upper_um", alias = "d50_p95_um")]
    pub d50_p95_um: f64,
    #[serde(rename = "grade_20um_sample_screen_lower", alias = "grade_20um_p05")]
    pub grade_20um_p05: f64,
    #[serde(
        rename = "sampled_pass_fraction_research_limit",
        alias = "probability_passes_30_mg_l"
    )]
    pub probability_passes_30_mg_l: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaSdfSummary {
    pub execution_backend: String,
    pub gpu_device_name: String,
    pub voxel_volume_mm3: f64,
    pub min_wall_clearance_mm: f64,
    pub gpu_max_von_mises_mpa: f64,
    pub gpu_yield_safety_factor: f64,
    pub gpu_d50_um: f64,
    pub gpu_cout_mg_l: f64,
    pub cpu_gpu_parity_abs_err: f64,
    #[serde(default)]
    pub gpu_vram_allocated_mib: f64,
    #[serde(default)]
    pub gpu_vram_used_mib: f64,
    #[serde(default)]
    pub gpu_vram_total_mib: f64,
    #[serde(default)]
    pub gpu_vram_utilization_pct: f64,
    #[serde(default)]
    pub gpu_3d_voxels_evaluated: u64,
    #[serde(default, alias = "gpu_lagrangian_parcels_integrated")]
    pub gpu_workspace_tensor_cells_evaluated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Individual {
    pub id: String,
    pub program: DesignProgram,
    pub genome_sha256: String,
    pub topology_class: String,
    pub generation_born: usize,
    pub island_origin: usize,
    /// Legacy archives without a receipt remain explicitly unidentified.
    #[serde(default)]
    pub lineage: Option<Lineage>,
    pub red_team_passed: bool,
    pub red_team_rejects: Vec<String>,
    /// Cavity/topology facts the partial red-team screen did not evaluate (`V5-01-02`). A pass with
    /// entries is not complete F0/geometry qualification; legacy records read as unevaluated.
    #[serde(default = "legacy_unevaluated_geometry_facts")]
    pub red_team_unevaluated_facts: Vec<String>,
    pub descriptors: MechanismDescriptors,
    pub phenotype: Option<Phenotype3D>,
    #[serde(
        rename = "analytic_structural_screen",
        alias = "structural_qualification"
    )]
    pub structural_qualification: Option<AnalyticStructuralScreen>,
    #[serde(default)]
    pub structural_screen_state: StructuralScreenState,
    pub nominal_metrics: CandidateMetrics,
    pub mc_robustness: MonteCarloSummary,
    pub cuda_sdf_audit: Option<CudaSdfSummary>,
    pub objectives: ObjectiveOutcome,
    pub constraint_violation: f64,
    pub rank: usize,
    pub crowding_distance: f64,
    pub evidence_tier: EvidenceTier,
    pub qualified_multiobjective_vector: Option<serde_json::Value>,
    pub qualified_capacity_gain: Option<f64>,
}

pub(crate) fn legacy_unevaluated_geometry_facts() -> Vec<String> {
    vec!["legacy record: cavity and topology facts not evaluated".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ObjectiveOutcome {
    SandboxEstimate { values: [f64; 5] },
    Insufficient { missing_quantiles: Vec<String> },
}

impl ObjectiveOutcome {
    pub fn values(&self) -> Option<&[f64; 5]> {
        match self {
            Self::SandboxEstimate { values } => Some(values),
            Self::Insufficient { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StructuralScreenState {
    Screened,
    Failed {
        reason: String,
    },
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSummary {
    pub campaign_admission: crate::experiment::CampaignAdmissionReceipt,
    #[serde(default, flatten)]
    pub proposal_budget: ProposalBudget,
    pub schema_version: String,
    pub engine: String,
    pub claim_state: String,
    pub config: EvolutionConfig,
    pub total_evaluations: usize,
    pub valid_f0_evaluations: usize,
    pub red_team_rejections: usize,
    pub map_elites_occupied_cells: usize,
    pub pareto_front_size: usize,
    pub topology_family_counts_in_archive: BTreeMap<String, usize>,
    pub baseline_reference: Individual,
    pub top_pareto_elites: Vec<Individual>,
    pub exported_mesh_files: Vec<String>,
}
