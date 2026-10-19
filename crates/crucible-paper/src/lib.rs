use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub id: String,
    pub subject_sha256: String,
    pub scenario_id: String,
    pub estimand_id: String,
    pub unit: String,
    pub denominator: String,
    pub independent_unit_ids: Vec<String>,
    pub tier: String,
    pub tier_current: bool,
    pub value: String,
    pub interpretation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimQuery {
    pub observation_id: String,
    pub subject_sha256: String,
    pub scenario_id: String,
    pub estimand_id: String,
    pub unit: String,
    pub denominator: String,
    pub tier: String,
    pub allowed_interpretation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FigureManifest {
    pub id: String,
    pub observation_ids: Vec<String>,
    pub generator: String,
    pub source_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostQualityPoint {
    pub finalist_id: String,
    pub attempt_id: String,
    pub confirmation_data_id: String,
    pub normalized_resource_cost: String,
    pub quality_estimate: String,
    pub uncertainty_low: String,
    pub uncertainty_high: String,
    pub independent_unit_ids: Vec<String>,
    pub outcome: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaperBundle {
    pub schema_version: u32,
    pub observations: Vec<Observation>,
    pub queries: Vec<ClaimQuery>,
    pub figures: Vec<FigureManifest>,
    pub attempted_finalist_ids: Vec<String>,
    pub cost_quality: Vec<CostQualityPoint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaperError {
    Version,
    DuplicateObservation(String),
    MissingObservation(String),
    WrongBinding(String),
    StaleTier(String),
    Interpretation(String),
    InvalidFigure(String),
}

impl std::fmt::Display for PaperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PaperError {}

fn validate_observation_context(observation: &Observation) -> Result<(), PaperError> {
    if !observation.tier_current {
        return Err(PaperError::StaleTier(observation.id.clone()));
    }
    let units: std::collections::BTreeSet<_> = observation.independent_unit_ids.iter().collect();
    if !matches!(
        observation.denominator.as_str(),
        "independent_search_seed" | "independent_preparation_and_build"
    ) || observation.independent_unit_ids.is_empty()
        || units.len() != observation.independent_unit_ids.len()
        || observation
            .independent_unit_ids
            .iter()
            .any(|id| id.trim().is_empty())
    {
        return Err(PaperError::WrongBinding(observation.id.clone()));
    }
    Ok(())
}

impl PaperBundle {
    pub fn validate(&self) -> Result<(), PaperError> {
        if self.schema_version != 1 {
            return Err(PaperError::Version);
        }
        let mut observations = BTreeMap::new();
        for observation in &self.observations {
            if observations.insert(&observation.id, observation).is_some() {
                return Err(PaperError::DuplicateObservation(observation.id.clone()));
            }
        }
        for query in &self.queries {
            let Some(observation) = observations.get(&query.observation_id) else {
                return Err(PaperError::MissingObservation(query.observation_id.clone()));
            };
            if observation.subject_sha256 != query.subject_sha256
                || observation.scenario_id != query.scenario_id
                || observation.estimand_id != query.estimand_id
                || observation.unit != query.unit
                || observation.denominator != query.denominator
                || observation.tier != query.tier
            {
                return Err(PaperError::WrongBinding(query.observation_id.clone()));
            }
            validate_observation_context(observation)?;
            if observation.interpretation != query.allowed_interpretation {
                return Err(PaperError::Interpretation(query.observation_id.clone()));
            }
        }
        for figure in &self.figures {
            if figure.generator.trim().is_empty()
                || figure.source_sha256.len() != 64
                || figure.observation_ids.is_empty()
                || figure
                    .observation_ids
                    .iter()
                    .any(|id| !observations.contains_key(id))
            {
                return Err(PaperError::InvalidFigure(figure.id.clone()));
            }
        }
        for figure in &self.figures {
            for id in &figure.observation_ids {
                validate_observation_context(observations[id])?;
            }
        }
        let finalists: std::collections::BTreeSet<_> = self.attempted_finalist_ids.iter().collect();
        if finalists.len() != self.attempted_finalist_ids.len() {
            return Err(PaperError::WrongBinding("duplicate finalist".into()));
        }
        let mut represented = std::collections::BTreeSet::new();
        let mut attempts = std::collections::BTreeSet::new();
        for point in &self.cost_quality {
            let parsed = [
                point.normalized_resource_cost.parse::<f64>(),
                point.quality_estimate.parse::<f64>(),
                point.uncertainty_low.parse::<f64>(),
                point.uncertainty_high.parse::<f64>(),
            ];
            if !finalists.contains(&point.finalist_id)
                || point.confirmation_data_id.trim().is_empty()
                || point.independent_unit_ids.is_empty()
                || !matches!(point.outcome.as_str(), "negative" | "feasible" | "failed")
                || !attempts.insert(&point.attempt_id)
                || parsed.iter().any(Result::is_err)
            {
                return Err(PaperError::WrongBinding(point.finalist_id.clone()));
            }
            let [Ok(cost), Ok(estimate), Ok(low), Ok(high)] = parsed else {
                unreachable!()
            };
            if !cost.is_finite()
                || cost < 0.0
                || !estimate.is_finite()
                || !low.is_finite()
                || !high.is_finite()
                || low > estimate
                || estimate > high
            {
                return Err(PaperError::WrongBinding(point.finalist_id.clone()));
            }
            let units: std::collections::BTreeSet<_> = point.independent_unit_ids.iter().collect();
            if units.len() != point.independent_unit_ids.len() {
                return Err(PaperError::WrongBinding(point.finalist_id.clone()));
            }
            represented.insert(&point.finalist_id);
        }
        if finalists != represented {
            return Err(PaperError::WrongBinding(
                "attempted finalist missing from cost-quality report".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlantedErrorAuditReport {
    pub baseline_valid: bool,
    pub planted_mutations_tested: usize,
    pub planted_mutations_rejected: usize,
    pub rejected_classes: Vec<&'static str>,
}

/// query_id is the exact observation_id of a registered typed query, never prose similarity.
pub fn verify_exact_claim_token(
    bundle: &PaperBundle,
    query_id: &str,
    prose_value: &str,
    prose_unit: &str,
) -> Result<(), PaperError> {
    bundle.validate()?;
    let query = bundle
        .queries
        .iter()
        .find(|q| q.observation_id == query_id)
        .ok_or_else(|| PaperError::MissingObservation(query_id.into()))?;
    if query.observation_id.trim().is_empty()
        || query.estimand_id.trim().is_empty()
        || query.unit.trim().is_empty()
        || query.subject_sha256.trim().is_empty()
    {
        return Err(PaperError::WrongBinding(query_id.into()));
    }
    let observation = bundle
        .observations
        .iter()
        .find(|o| o.id == query.observation_id)
        .ok_or_else(|| PaperError::MissingObservation(query_id.into()))?;
    if observation.value != prose_value || observation.unit != prose_unit {
        return Err(PaperError::WrongBinding(query_id.into()));
    }
    Ok(())
}

/// Execute mutation controls against a validated baseline; inability to plant is an error.
pub fn verify_planted_error_negative_corpus(
    bundle: &PaperBundle,
) -> Result<PlantedErrorAuditReport, PaperError> {
    bundle.validate()?;
    let query = bundle
        .queries
        .first()
        .ok_or_else(|| PaperError::MissingObservation("negative_corpus_query".into()))?;
    let index = bundle
        .observations
        .iter()
        .position(|o| o.id == query.observation_id)
        .ok_or_else(|| PaperError::MissingObservation(query.observation_id.clone()))?;
    let observation = &bundle.observations[index];
    verify_exact_claim_token(
        bundle,
        &query.observation_id,
        &observation.value,
        &observation.unit,
    )?;
    let value = observation
        .value
        .parse::<f64>()
        .map_err(|_| PaperError::WrongBinding("negative_corpus_numeric_value".into()))?;
    if !value.is_finite() || value == 0.0 || query.unit == "mg/L" {
        return Err(PaperError::WrongBinding(
            "negative_corpus_inapplicable_baseline".into(),
        ));
    }
    let mut report = PlantedErrorAuditReport {
        baseline_valid: true,
        planted_mutations_tested: 0,
        planted_mutations_rejected: 0,
        rejected_classes: Vec::new(),
    };
    for class in [
        "wrong_denominator",
        "wrong_unit",
        "wrong_subject",
        "stale_tier",
        "fuzzy_1000x_or_0_15pct_prose_match",
    ] {
        let mut mutated = bundle.clone();
        let rejected = match class {
            "wrong_denominator" => {
                mutated.observations[index].denominator = "parcel".into();
                mutated.validate().is_err()
            }
            "wrong_unit" => {
                mutated.queries[0].unit = "mg/L".into();
                mutated.validate().is_err()
            }
            "wrong_subject" => {
                mutated.queries[0].subject_sha256 = if query.subject_sha256 == "b".repeat(64) {
                    "c".repeat(64)
                } else {
                    "b".repeat(64)
                };
                mutated.validate().is_err()
            }
            "stale_tier" => {
                mutated.observations[index].tier_current = false;
                mutated.validate().is_err()
            }
            _ => {
                [value * 1000.0, value * 1.0015].iter().all(|wrong| {
                    verify_exact_claim_token(
                        &mutated,
                        &query.observation_id,
                        &wrong.to_string(),
                        &observation.unit,
                    )
                    .is_err()
                }) && verify_exact_claim_token(
                    &mutated,
                    "unbound-prose-number",
                    &observation.value,
                    &observation.unit,
                )
                .is_err()
            }
        };
        report.planted_mutations_tested += 1;
        if !rejected {
            return Err(PaperError::WrongBinding(format!(
                "planted_error_accepted:{class}"
            )));
        }
        report.planted_mutations_rejected += 1;
        report.rejected_classes.push(class);
    }
    Ok(report)
}

/// One executed search arm/seed; all finalist attempts, including failures, are retained.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchSeedRun {
    pub arm: String,
    pub search_seed_id: u64,
    pub search_data_id: String,
    pub attempted_finalist_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationRunRecord {
    pub arm: String,
    pub finalist_id: String,
    pub confirmation_seed_id: u64,
    pub confirmation_data_id: String,
    pub estimand_id: String,
    pub outcome: String,
    /// Normalized resource cost and quality intervals [low, estimate, high].
    pub cost_interval: [f64; 3],
    pub quality_interval: [f64; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationCampaignLedger {
    pub synthetic: bool,
    pub scientific_admission: bool,
    pub frozen_before_search: bool,
    pub estimand_id: String,
    pub search_seed_ids: Vec<u64>,
    pub confirmation_seed_ids: Vec<u64>,
    pub search_runs: Vec<SearchSeedRun>,
    pub attempted_finalist_ids: Vec<String>,
    pub confirmation_runs: Vec<ConfirmationRunRecord>,
    /// Published ledger must preserve every raw attempt and interval without endpoint changes.
    pub reported_runs: Vec<ConfirmationRunRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfirmationCampaignAudit {
    pub search_runs: usize,
    pub confirmation_runs: usize,
    pub finalists_retained: usize,
    pub negative_or_null_or_failed: usize,
    pub preserved_cost_quality: Vec<ConfirmationRunRecord>,
    pub scientific_admission: bool,
}

pub fn evaluate_confirmation_campaign_without_winner_leakage(
    ledger: &ConfirmationCampaignLedger,
) -> Result<ConfirmationCampaignAudit, PaperError> {
    use std::collections::BTreeSet;
    let invalid = |reason: &str| PaperError::WrongBinding(reason.into());
    let arms = ["R0", "R1", "E1", "E2", "A1", "A2"];
    let search: BTreeSet<_> = ledger.search_seed_ids.iter().copied().collect();
    let confirmation: BTreeSet<_> = ledger.confirmation_seed_ids.iter().copied().collect();
    if !ledger.frozen_before_search
        || ledger.estimand_id.trim().is_empty()
        || (ledger.synthetic && ledger.scientific_admission)
        || search.len() < 2
        || confirmation.is_empty()
        || !search.is_disjoint(&confirmation)
        || search.len() != ledger.search_seed_ids.len()
        || confirmation.len() != ledger.confirmation_seed_ids.len()
    {
        return Err(invalid(
            "unfrozen endpoint, synthetic admission, or reused/missing seeds",
        ));
    }
    let mut search_pairs = BTreeSet::new();
    let mut search_data = BTreeSet::new();
    let mut attempted = BTreeMap::new();
    for run in &ledger.search_runs {
        if !arms.contains(&run.arm.as_str())
            || !search.contains(&run.search_seed_id)
            || run.search_data_id.trim().is_empty()
            || run.attempted_finalist_ids.is_empty()
            || !search_pairs.insert((run.arm.as_str(), run.search_seed_id))
        {
            return Err(invalid("invalid or duplicate search run"));
        }
        search_data.insert(run.search_data_id.as_str());
        for finalist in &run.attempted_finalist_ids {
            if finalist.trim().is_empty()
                || attempted
                    .insert(finalist.as_str(), run.arm.as_str())
                    .is_some()
            {
                return Err(invalid("empty or duplicate finalist attempt"));
            }
        }
    }
    if search_pairs.len() != arms.len() * search.len() {
        return Err(invalid("missing preregistered arm/seed execution"));
    }
    let declared: BTreeSet<_> = ledger
        .attempted_finalist_ids
        .iter()
        .map(String::as_str)
        .collect();
    if declared.len() != ledger.attempted_finalist_ids.len()
        || declared != attempted.keys().copied().collect()
    {
        return Err(invalid("attempted finalists omitted from campaign ledger"));
    }
    let mut confirmations = BTreeMap::new();
    let mut fresh_data = BTreeSet::new();
    let mut negatives = 0;
    for run in &ledger.confirmation_runs {
        if attempted.get(run.finalist_id.as_str()) != Some(&run.arm.as_str())
            || !confirmation.contains(&run.confirmation_seed_id)
            || run.confirmation_data_id.trim().is_empty()
            || search_data.contains(run.confirmation_data_id.as_str())
            || !fresh_data.insert(run.confirmation_data_id.as_str())
            || run.estimand_id != ledger.estimand_id
            || !matches!(
                run.outcome.as_str(),
                "feasible" | "negative" | "null" | "dominated" | "failed"
            )
            || confirmations
                .insert((run.finalist_id.as_str(), run.confirmation_seed_id), run)
                .is_some()
        {
            return Err(invalid(
                "confirmation seed/data reuse, duplicate attempt or endpoint substitution",
            ));
        }
        for interval in [run.cost_interval, run.quality_interval] {
            if interval.iter().any(|v| !v.is_finite())
                || interval[0] > interval[1]
                || interval[1] > interval[2]
            {
                return Err(invalid("non-finite or unordered Pareto interval"));
            }
        }
        if run.cost_interval[0] < 0.0 {
            return Err(invalid("negative normalized resource cost"));
        }
        if run.outcome != "feasible" {
            negatives += 1;
        }
    }
    if confirmations.len() != attempted.len() * confirmation.len() {
        return Err(invalid("missing finalist/confirmation-seed execution"));
    }
    let mut reported = BTreeSet::new();
    for run in &ledger.reported_runs {
        let key = (run.finalist_id.as_str(), run.confirmation_seed_id);
        if !reported.insert(key) || confirmations.get(&key) != Some(&run) {
            return Err(invalid("report changed outcome, interval or endpoint"));
        }
    }
    if reported.len() != confirmations.len() {
        return Err(invalid("confirmation report omits attempts"));
    }
    Ok(ConfirmationCampaignAudit {
        search_runs: search_pairs.len(),
        confirmation_runs: confirmations.len(),
        finalists_retained: attempted.len(),
        negative_or_null_or_failed: negatives,
        preserved_cost_quality: ledger.reported_runs.clone(),
        scientific_admission: false,
    })
}

pub fn verify_confirmation_campaign_json(
    json_str: &str,
) -> Result<ConfirmationCampaignAudit, PaperError> {
    let ledger =
        serde_json::from_str(json_str).map_err(|e| PaperError::WrongBinding(e.to_string()))?;
    evaluate_confirmation_campaign_without_winner_leakage(&ledger)
}

/// Evidence inventory supplied by an upstream packet owner.
#[derive(Clone, Debug)]
pub struct PacketIntegrationRecord {
    pub packet_id: String,
    pub tested_commit: String,
    pub changed_paths: Vec<String>,
    pub command_exit_codes: Vec<i32>,
    pub non_summary_logs: bool,
    pub positive_checks: usize,
    pub negative_checks: usize,
    pub next_dependency: String,
}

fn is_hex_identity(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn release_error(reason: &str) -> PaperError {
    PaperError::WrongBinding(reason.to_owned())
}

/// Require exactly one complete record for each of the 31 upstream packets.
/// This checks supplied metadata, not the existence or authenticity of logs.
pub fn verify_packet_integration_completeness(
    records: &[PacketIntegrationRecord],
) -> Result<(), PaperError> {
    let mut seen = std::collections::BTreeSet::new();
    for record in records {
        if !(0..31).any(|index| record.packet_id == format!("V5-{index:02}"))
            || !seen.insert(&record.packet_id)
        {
            return Err(release_error("unknown or duplicate upstream packet"));
        }
        if !is_hex_identity(&record.tested_commit, 40) {
            return Err(release_error("invalid tested commit"));
        }
        // Canonical relative paths prevent aliases bypassing the private roots.
        if record.changed_paths.is_empty()
            || record.changed_paths.iter().any(|path| {
                path.trim().is_empty()
                    || path.starts_with('/')
                    || path.contains('\\')
                    || path.contains('*')
                    || path.split('/').any(|part| matches!(part, "" | "." | ".."))
                    || matches!(
                        path.split('/').next(),
                        Some(".private" | "runs" | "artifacts")
                    )
            })
        {
            return Err(release_error("unscoped or forbidden changed path"));
        }
        if record.command_exit_codes.is_empty()
            || record.command_exit_codes.iter().any(|code| *code != 0)
            || !record.non_summary_logs
            || record.positive_checks == 0
            || record.negative_checks == 0
            || record.next_dependency.trim().is_empty()
        {
            return Err(release_error("incomplete packet execution evidence"));
        }
    }
    if seen.len() != 31 {
        return Err(release_error("missing upstream packet"));
    }
    Ok(())
}

/// A parity comparison retains explicit unavailable-input provenance.
#[derive(Clone, Debug)]
pub struct LegacyMigrationParityRecord {
    pub legacy_path: String,
    pub native_replacement: String,
    pub golden_input_sha256: String,
    pub mesh_inputs_present: bool,
    pub max_rel_discrepancy: f64,
    pub tolerance: f64,
    pub unavailable_input_disposition: Option<String>,
    pub claims_restored_provenance: bool,
}

/// A matching scalar never restores missing governing mesh evidence.
pub fn verify_legacy_migration_parity_and_provenance(
    records: &[LegacyMigrationParityRecord],
) -> Result<(), PaperError> {
    if records.is_empty() {
        return Err(release_error("missing migration records"));
    }
    for record in records {
        if record.legacy_path.trim().is_empty()
            || record.native_replacement.trim().is_empty()
            || !is_hex_identity(&record.golden_input_sha256, 64)
        {
            return Err(release_error("missing migration identity"));
        }
        if !record.max_rel_discrepancy.is_finite()
            || record.max_rel_discrepancy < 0.0
            || !record.tolerance.is_finite()
            || record.tolerance < 0.0
        {
            return Err(release_error("invalid relative discrepancy or tolerance"));
        }
        if record.mesh_inputs_present {
            if record.max_rel_discrepancy > record.tolerance {
                return Err(release_error("migration parity exceeded tolerance"));
            }
        } else if record.claims_restored_provenance
            || record
                .unavailable_input_disposition
                .as_ref()
                .is_none_or(|s| s.trim().is_empty())
        {
            return Err(release_error("missing mesh provenance or disposition"));
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct CiTierSuiteRecord {
    pub suite_id: String,
    pub head_sha: String,
    pub passed: bool,
}

#[derive(Clone, Debug)]
pub struct ScientificStatusSummary {
    pub head_sha: String,
    pub structural_qualification: bool,
    pub qualified_baseline: bool,
    pub scientific_admission: bool,
    pub physical_rig_evidence_admitted: bool,
}

/// Check all Foundation suites on one head and prohibit CI-only science promotion.
/// Physical evidence admission must be established by the caller independently.
pub fn verify_ci_tiers_and_conservative_status_derivation(
    suites: &[CiTierSuiteRecord],
    status: &ScientificStatusSummary,
) -> Result<(), PaperError> {
    const REQUIRED: [&str; 8] = [
        "rust-tests",
        "python-tests",
        "ui",
        "clippy",
        "fmt",
        "claims-audit",
        "baseline-controls",
        "public-tree",
    ];
    if !is_hex_identity(&status.head_sha, 40) {
        return Err(release_error("invalid status head"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for suite in suites {
        if !suite.passed || suite.head_sha != status.head_sha || !seen.insert(&suite.suite_id) {
            return Err(release_error("failed, stale or duplicate CI suite"));
        }
    }
    if REQUIRED
        .iter()
        .any(|required| !suites.iter().any(|s| s.suite_id == *required))
    {
        return Err(release_error("missing Foundation suite"));
    }
    if !status.physical_rig_evidence_admitted
        && (status.structural_qualification
            || status.qualified_baseline
            || status.scientific_admission)
    {
        return Err(release_error(
            "software CI cannot promote scientific status",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct FourTrackReleaseScope {
    pub software_engine_level: String,
    pub computational_comparator_level: String,
    pub discovery_method_level: String,
    pub physical_candidate_level: String,
    pub physical_programme_complete: bool,
    pub gpu_fallback_disposition: String,
    pub operator_fallback_disposition: String,
    pub absent_inputs: Vec<String>,
}

/// Track labels use their own prefix and a numeric level; they are not evidence.
/// R4 additionally requires completion of the physical programme.
pub fn reconcile_four_track_release_scope(scope: &FourTrackReleaseScope) -> Result<(), PaperError> {
    for (level, prefix) in [
        (&scope.software_engine_level, 'S'),
        (&scope.computational_comparator_level, 'C'),
        (&scope.discovery_method_level, 'D'),
        (&scope.physical_candidate_level, 'R'),
    ] {
        let Some(number) = level.strip_prefix(prefix) else {
            return Err(release_error("wrong independent track prefix"));
        };
        if number.len() != 1 || !number.bytes().all(|b| b.is_ascii_digit()) {
            return Err(release_error("invalid track level"));
        }
        if prefix == 'R' && !matches!(number, "0" | "1" | "2" | "3" | "4") {
            return Err(release_error("unknown physical release level"));
        }
    }
    if !scope.physical_programme_complete {
        if scope.physical_candidate_level == "R4" {
            return Err(release_error("R4 requires physical programme completion"));
        }
        if scope.gpu_fallback_disposition.trim().is_empty()
            || scope.operator_fallback_disposition.trim().is_empty()
            || scope.absent_inputs.is_empty()
            || scope
                .absent_inputs
                .iter()
                .any(|input| input.trim().is_empty())
        {
            return Err(release_error(
                "missing fallback or absent-input disposition",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ExactHeadReleaseHandoff {
    pub pr_head_sha: String,
    pub ci_verified_sha: String,
    pub review_approved_sha: String,
    pub completed_tasks: usize,
    pub total_tasks: usize,
    pub unacknowledged_items: usize,
    pub immutable_receipt_count: usize,
}

/// Verify supplied exact-head handoff metadata; this does not merge or approve a PR.
pub fn verify_exact_head_release_handoff(
    handoff: &ExactHeadReleaseHandoff,
) -> Result<(), PaperError> {
    if !is_hex_identity(&handoff.pr_head_sha, 40)
        || handoff.pr_head_sha != handoff.ci_verified_sha
        || handoff.pr_head_sha != handoff.review_approved_sha
    {
        return Err(release_error("release head, CI and review must match"));
    }
    if handoff.completed_tasks != handoff.total_tasks || handoff.total_tasks != 160 {
        return Err(release_error("release requires all 160 tasks"));
    }
    if handoff.unacknowledged_items != 0 || handoff.immutable_receipt_count == 0 {
        return Err(release_error(
            "unreconciled handoff or absent immutable receipts",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
