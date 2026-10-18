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
mod tests {
    type ReleaseMutation<T> = (&'static str, fn(&mut T));

    fn release_check<T: Clone>(
        good: &T,
        verify: impl Fn(&T) -> Result<(), PaperError>,
        mutations: &[ReleaseMutation<T>],
    ) {
        assert!(verify(good).is_ok());
        println!("positive: complete synthetic metadata accepted");
        for (label, mutate) in mutations {
            let mut bad = good.clone();
            mutate(&mut bad);
            assert!(verify(&bad).is_err(), "accepted negative: {label}");
            println!("negative: {label} rejected");
        }
    }

    #[test]
    fn v5_31_01_packet_integration_completeness_rejects_unscoped_paths_and_missing_logs() {
        let good: Vec<_> = (0..31)
            .map(|i| PacketIntegrationRecord {
                packet_id: format!("V5-{i:02}"),
                tested_commit: "a".repeat(40),
                changed_paths: vec!["crates/crucible-paper/src/lib.rs".into()],
                command_exit_codes: vec![0],
                non_summary_logs: true,
                positive_checks: 1,
                negative_checks: 1,
                next_dependency: "V5-31".into(),
            })
            .collect();
        release_check(
            &good,
            |r| verify_packet_integration_completeness(r),
            &[
                ("missing packet", |r| {
                    r.pop();
                }),
                ("duplicate packet", |r| r[1].packet_id = "V5-00".into()),
                ("unknown packet", |r| r[0].packet_id = "V5-31".into()),
                ("short commit", |r| r[0].tested_commit = "a".repeat(39)),
                ("nonhex commit", |r| r[0].tested_commit = "z".repeat(40)),
                ("empty scope", |r| r[0].changed_paths.clear()),
                ("private scope", |r| {
                    r[0].changed_paths = vec![".private/secret".into()]
                }),
                ("runs scope", |r| {
                    r[0].changed_paths = vec!["runs/log".into()]
                }),
                ("artifacts scope", |r| {
                    r[0].changed_paths = vec!["artifacts/data".into()]
                }),
                ("wildcard", |r| r[0].changed_paths = vec!["*".into()]),
                ("path traversal", |r| {
                    r[0].changed_paths = vec!["docs/../.private/secret".into()]
                }),
                ("missing commands", |r| r[0].command_exit_codes.clear()),
                ("failed command", |r| r[0].command_exit_codes.push(1)),
                ("missing raw logs", |r| r[0].non_summary_logs = false),
                ("missing positive", |r| r[0].positive_checks = 0),
                ("missing negative", |r| r[0].negative_checks = 0),
                ("blank dependency", |r| r[0].next_dependency = " ".into()),
            ],
        );
    }

    #[test]
    fn v5_31_02_legacy_migration_parity_rejects_scalar_match_without_mesh_provenance() {
        let good = vec![LegacyMigrationParityRecord {
            legacy_path: "legacy/script.py".into(),
            native_replacement: "native/replacement.rs".into(),
            golden_input_sha256: "b".repeat(64),
            mesh_inputs_present: true,
            max_rel_discrepancy: 0.01,
            tolerance: 0.01,
            unavailable_input_disposition: None,
            claims_restored_provenance: true,
        }];
        release_check(
            &good,
            |r| verify_legacy_migration_parity_and_provenance(r),
            &[
                ("empty records", |r| r.clear()),
                ("scalar match without mesh", |r| {
                    r[0].mesh_inputs_present = false;
                    r[0].max_rel_discrepancy = 0.0;
                    r[0].unavailable_input_disposition = Some("missing mesh".into());
                }),
                ("absent disposition", |r| {
                    r[0].mesh_inputs_present = false;
                    r[0].claims_restored_provenance = false;
                }),
                ("blank disposition", |r| {
                    r[0].mesh_inputs_present = false;
                    r[0].claims_restored_provenance = false;
                    r[0].unavailable_input_disposition = Some(" ".into());
                }),
                ("above tolerance", |r| r[0].max_rel_discrepancy = 0.010001),
                ("NaN discrepancy", |r| r[0].max_rel_discrepancy = f64::NAN),
                ("infinite tolerance", |r| r[0].tolerance = f64::INFINITY),
                ("negative tolerance", |r| r[0].tolerance = -0.01),
                ("negative discrepancy", |r| r[0].max_rel_discrepancy = -0.01),
                ("invalid golden identity", |r| {
                    r[0].golden_input_sha256.clear()
                }),
            ],
        );
        let mut absent = good;
        absent[0].mesh_inputs_present = false;
        absent[0].claims_restored_provenance = false;
        absent[0].unavailable_input_disposition =
            Some("mesh unavailable; reproduction deferred".into());
        assert!(verify_legacy_migration_parity_and_provenance(&absent).is_ok());
        println!(
            "positive: explicit unavailable-input disposition accepted without restored provenance"
        );
    }

    #[test]
    fn v5_31_03_ci_tiers_enforce_conservative_status_and_forbid_manual_qualification_flip() {
        let suites: Vec<_> = [
            "rust-tests",
            "python-tests",
            "ui",
            "clippy",
            "fmt",
            "claims-audit",
            "baseline-controls",
            "public-tree",
        ]
        .iter()
        .map(|id| CiTierSuiteRecord {
            suite_id: (*id).into(),
            head_sha: "a".repeat(40),
            passed: true,
        })
        .collect();
        let status = ScientificStatusSummary {
            head_sha: "a".repeat(40),
            structural_qualification: false,
            qualified_baseline: false,
            scientific_admission: false,
            physical_rig_evidence_admitted: false,
        };
        release_check(
            &(suites, status),
            |(s, t)| verify_ci_tiers_and_conservative_status_derivation(s, t),
            &[
                ("missing suite", |r| {
                    r.0.pop();
                }),
                ("failed suite", |r| r.0[0].passed = false),
                ("stale suite", |r| r.0[0].head_sha = "b".repeat(40)),
                ("duplicate suite", |r| r.0.push(r.0[0].clone())),
                ("invalid head", |r| r.1.head_sha = "invalid".into()),
                ("manual structural flip", |r| {
                    r.1.structural_qualification = true
                }),
                ("manual baseline flip", |r| r.1.qualified_baseline = true),
                ("manual scientific flip", |r| {
                    r.1.scientific_admission = true
                }),
            ],
        );
    }

    #[test]
    fn v5_31_04_four_track_release_scope_forbids_r4_without_physical_measurements() {
        let good = FourTrackReleaseScope {
            software_engine_level: "S1".into(),
            computational_comparator_level: "C0".into(),
            discovery_method_level: "D0".into(),
            physical_candidate_level: "R0".into(),
            physical_programme_complete: false,
            gpu_fallback_disposition: "CPU reference only".into(),
            operator_fallback_disposition: "reference operator".into(),
            absent_inputs: vec!["independent rig measurements".into()],
        };
        release_check(
            &good,
            reconcile_four_track_release_scope,
            &[
                ("R4 without physical programme", |r| {
                    r.physical_candidate_level = "R4".into()
                }),
                ("wrong software track", |r| {
                    r.software_engine_level = "R1".into()
                }),
                ("wrong comparator track", |r| {
                    r.computational_comparator_level = "S1".into()
                }),
                ("wrong discovery track", |r| {
                    r.discovery_method_level = "C1".into()
                }),
                ("bare prefix", |r| r.discovery_method_level = "D".into()),
                ("unknown physical level", |r| {
                    r.physical_candidate_level = "R9".into()
                }),
                ("blank GPU fallback", |r| {
                    r.gpu_fallback_disposition = " ".into()
                }),
                ("blank operator fallback", |r| {
                    r.operator_fallback_disposition.clear()
                }),
                ("missing absent-input inventory", |r| {
                    r.absent_inputs.clear()
                }),
                ("blank absent input", |r| r.absent_inputs = vec![" ".into()]),
            ],
        );
        let mut complete = good;
        complete.physical_candidate_level = "R4".into();
        complete.physical_programme_complete = true;
        assert!(reconcile_four_track_release_scope(&complete).is_ok());
        println!(
            "positive: synthetic completed-programme metadata accepted; no real physical admission"
        );
    }

    #[test]
    fn v5_31_05_exact_head_handoff_rejects_stale_sha_and_unreconciled_tasks() {
        let good = ExactHeadReleaseHandoff {
            pr_head_sha: "a".repeat(40),
            ci_verified_sha: "a".repeat(40),
            review_approved_sha: "a".repeat(40),
            completed_tasks: 160,
            total_tasks: 160,
            unacknowledged_items: 0,
            immutable_receipt_count: 5,
        };
        release_check(
            &good,
            verify_exact_head_release_handoff,
            &[
                ("stale CI", |r| r.ci_verified_sha = "b".repeat(40)),
                ("stale review", |r| r.review_approved_sha = "b".repeat(40)),
                ("short SHA", |r| r.pr_head_sha = "a".repeat(39)),
                ("matching nonhex SHAs", |r| {
                    r.pr_head_sha = "z".repeat(40);
                    r.ci_verified_sha = r.pr_head_sha.clone();
                    r.review_approved_sha = r.pr_head_sha.clone();
                }),
                ("incomplete tasks", |r| r.completed_tasks = 159),
                ("wrong total", |r| {
                    r.completed_tasks = 159;
                    r.total_tasks = 159;
                }),
                ("unacknowledged item", |r| r.unacknowledged_items = 1),
                ("no immutable receipts", |r| r.immutable_receipt_count = 0),
            ],
        );
    }

    use super::*;

    fn bundle() -> PaperBundle {
        let observation = Observation {
            id: "obs-screen-001".into(),
            subject_sha256: "a".repeat(64),
            scenario_id: "synthetic-scenario-1".into(),
            estimand_id: "screen-score".into(),
            unit: "dimensionless".into(),
            denominator: "independent_search_seed".into(),
            independent_unit_ids: vec!["seed-01".into(), "seed-02".into()],
            tier: "F1_screen".into(),
            tier_current: true,
            value: "0.42".into(),
            interpretation: "synthetic search-screen result; not physical performance".into(),
        };
        let query = ClaimQuery {
            observation_id: observation.id.clone(),
            subject_sha256: observation.subject_sha256.clone(),
            scenario_id: observation.scenario_id.clone(),
            estimand_id: observation.estimand_id.clone(),
            unit: observation.unit.clone(),
            denominator: observation.denominator.clone(),
            tier: observation.tier.clone(),
            allowed_interpretation: observation.interpretation.clone(),
        };
        PaperBundle {
            schema_version: 1,
            observations: vec![observation],
            queries: vec![query],
            figures: vec![],
            attempted_finalist_ids: vec!["synthetic-arm".into()],
            cost_quality: vec![CostQualityPoint {
                finalist_id: "synthetic-arm".into(),
                attempt_id: "attempt-1".into(),
                confirmation_data_id: "synthetic-confirmation-only".into(),
                normalized_resource_cost: "1.0".into(),
                quality_estimate: "0.0".into(),
                uncertainty_low: "-0.1".into(),
                uncertainty_high: "0.1".into(),
                independent_unit_ids: vec!["seed-2301".into(), "seed-2302".into()],
                outcome: "negative".into(),
            }],
        }
    }

    #[test]
    fn g009_guard_figure_only_observation_cannot_bypass_revocation() {
        let mut b = bundle();
        b.queries.clear();
        b.observations[0].tier_current = false;
        b.figures.push(FigureManifest {
            id: "figure".into(),
            observation_ids: vec![b.observations[0].id.clone()],
            generator: "synthetic generator".into(),
            source_sha256: "a".repeat(64),
        });
        assert!(b.validate().is_err());
    }
    #[test]
    fn exact_typed_query_builds() {
        assert!(bundle().validate().is_ok());
    }

    #[test]
    fn wrong_denominator_unit_subject_and_stale_tier_fail() {
        for mutate in 0..4 {
            let mut value = bundle();
            match mutate {
                0 => value.observations[0].denominator = "parcel".into(),
                1 => value.queries[0].unit = "mg/L".into(),
                2 => value.queries[0].subject_sha256 = "b".repeat(64),
                _ => value.observations[0].tier_current = false,
            }
            assert!(value.validate().is_err(), "mutation {mutate} passed");
        }
    }

    #[test]
    fn parcels_and_frames_cannot_inflate_independent_n() {
        let mut value = bundle();
        value.observations[0].denominator = "parcel".into();
        value.observations[0].independent_unit_ids = vec!["frame-1".into(); 1000];
        assert!(value.validate().is_err());
        value.observations[0].denominator = "independent_search_seed".into();
        value.observations[0].independent_unit_ids = vec!["seed-1".into(), "seed-1".into()];
        assert!(value.validate().is_err());
    }

    #[test]
    fn every_attempted_finalist_and_uncertainty_interval_is_reported() {
        let mut value = bundle();
        value.attempted_finalist_ids.push("unreported".into());
        assert!(value.validate().is_err());
        value.attempted_finalist_ids.pop();
        value.cost_quality[0].uncertainty_low = "0.2".into();
        assert!(value.validate().is_err());
    }

    #[test]
    fn near_numeric_value_is_never_claim_identity() {
        let mut value = bundle();
        value.observations[0].value = "420".into();
        assert!(value.validate().is_ok());
        value.queries[0].unit = "mg/L".into();
        assert!(value.validate().is_err());
    }

    #[test]
    fn figure_must_reference_source_manifest_observations() {
        let mut value = bundle();
        value.figures.push(FigureManifest {
            id: "fig-1".into(),
            observation_ids: vec!["missing".into()],
            generator: "tools/build_figures.py".into(),
            source_sha256: "c".repeat(64),
        });
        assert!(matches!(
            value.validate(),
            Err(PaperError::InvalidFigure(_))
        ));
    }

    #[test]
    fn v5_28_29_30_sc_dec_041_to_052_prospective_study_arms_winner_leakage_typed_queries_and_public_bundle(
    ) {
        use serde_json::Value;

        let preregistration: Value = serde_json::from_str(include_str!(
            "../../../experiments/v5-study/preregistration-v1.json"
        ))
        .expect("prospective study protocol is valid JSON");
        assert!(preregistration["status"]
            .as_str()
            .unwrap()
            .contains("frozen-prospective"));
        assert_eq!(preregistration["family_policy"], "all-attempts-retained; no post-result endpoint substitution; any endpoint change creates a new protocol identity");
        let arms = preregistration["arms"].as_array().unwrap();
        for required_arm in ["R0", "R1", "E1", "E2", "A1", "A2"] {
            assert!(arms.iter().any(|arm| arm["id"] == required_arm));
        }
        assert_eq!(preregistration["independent_seeds"]["disjoint"], true);
        assert_eq!(preregistration["confirmation"]["fresh_data"], true);
        assert_eq!(
            preregistration["unavailable_l4_negative_control"]["forbidden"],
            "replace endpoint after inspecting winners"
        );

        let interventions: Value = serde_json::from_str(include_str!(
            "../../../experiments/v5-study/mechanism-interventions-v1.json"
        ))
        .expect("mechanistic intervention plan is valid JSON");
        for required_intervention in [
            "intact",
            "knockout",
            "sham",
            "rescue",
            "matched-volume",
            "matched-area",
        ] {
            assert!(interventions["interventions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == required_intervention));
        }

        let fixture_bytes = include_bytes!("../../../paper/fixtures/negative-result.json");
        let report: PaperBundle =
            serde_json::from_slice(fixture_bytes).expect("synthetic review bundle deserializes");
        report
            .validate()
            .expect("exactly bound typed queries and all attempted outcomes validate");
        assert!(report
            .cost_quality
            .iter()
            .any(|point| point.outcome == "negative"));
        assert_eq!(
            report.attempted_finalist_ids.len(),
            report
                .cost_quality
                .iter()
                .map(|point| point.finalist_id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );

        let mut fuzzy = report.clone();
        fuzzy.queries[0].observation_id = "obs-synthetic-search-001-with-suffix".into();
        assert!(matches!(
            fuzzy.validate(),
            Err(PaperError::MissingObservation(_))
        ));
        let mut winner_leakage = report.clone();
        winner_leakage.cost_quality[0].confirmation_data_id = "".into();
        assert!(winner_leakage.validate().is_err());
        let mut inflated_n = report.clone();
        inflated_n.cost_quality[0].independent_unit_ids = vec!["frame-1"; 1000]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert!(inflated_n.validate().is_err());

        let public_bundle = serde_json::to_string(&report).expect("bundle serializes");
        for private_marker in ["/home/", "/Users/", ".private/", "customer", "facility"] {
            assert!(
                !public_bundle.contains(private_marker),
                "public bundle leaked private marker {private_marker}"
            );
        }
    }

    #[test]
    fn v5_28_04_planted_error_negative_corpus_rejects_denominator_unit_subject_stale_tier_and_fuzzy_numeric_matches(
    ) {
        let baseline = bundle();
        let report = verify_planted_error_negative_corpus(&baseline).unwrap();
        assert!(report.baseline_valid);
        assert_eq!(report.planted_mutations_tested, 5);
        assert_eq!(report.planted_mutations_rejected, 5);
        assert_eq!(
            report.rejected_classes,
            vec![
                "wrong_denominator",
                "wrong_unit",
                "wrong_subject",
                "stale_tier",
                "fuzzy_1000x_or_0_15pct_prose_match"
            ]
        );
        let id = &baseline.queries[0].observation_id;
        assert!(verify_exact_claim_token(&baseline, id, "0.42", "dimensionless").is_ok());
        for value in ["420", "0.42063", "0.420", "NaN"] {
            assert!(verify_exact_claim_token(&baseline, id, value, "dimensionless").is_err());
        }
        assert!(verify_exact_claim_token(&baseline, id, "0.42", "mg/L").is_err());
        assert!(verify_exact_claim_token(&baseline, "absent", "0.42", "dimensionless").is_err());
        let mut bad = baseline.clone();
        bad.observations[0].tier_current = false;
        assert!(verify_planted_error_negative_corpus(&bad).is_err());
        let mut bad = baseline.clone();
        bad.queries.clear();
        assert!(verify_planted_error_negative_corpus(&bad).is_err());
        let mut bad = baseline.clone();
        bad.queries[0].estimand_id.clear();
        assert!(verify_exact_claim_token(&bad, id, "0.42", "dimensionless").is_err());
        assert_eq!(baseline, bundle());
    }
    #[test]
    fn v5_28_02_confirmation_campaign_enforces_fresh_confirmation_seeds_and_retains_negative_finalists(
    ) {
        let text =
            include_str!("../../../experiments/datasets/v5-28-02-confirmation-campaign.json");
        let ledger: ConfirmationCampaignLedger = serde_json::from_str(text).unwrap();
        let audit = verify_confirmation_campaign_json(text).unwrap();
        assert_eq!(audit.search_runs, 12);
        assert_eq!(audit.confirmation_runs, 24);
        assert_eq!(audit.finalists_retained, 12);
        assert_eq!(audit.negative_or_null_or_failed, 20);
        assert_eq!(audit.preserved_cost_quality, ledger.reported_runs);
        assert!(!audit.scientific_admission);
        for mutation in 0..13 {
            let mut bad = ledger.clone();
            match mutation {
                0 => bad.confirmation_seed_ids[0] = bad.search_seed_ids[0],
                1 => {
                    bad.reported_runs.pop();
                }
                2 => {
                    bad.confirmation_runs[0].confirmation_data_id =
                        bad.search_runs[0].search_data_id.clone()
                }
                3 => bad.scientific_admission = true,
                4 => {
                    bad.search_runs.pop();
                }
                5 => {
                    bad.confirmation_runs.pop();
                }
                6 => bad.reported_runs[0].quality_interval[1] = 0.05,
                7 => bad.reported_runs[0].outcome = "feasible".into(),
                8 => bad.confirmation_runs[0].estimand_id = "post-hoc".into(),
                9 => {
                    bad.attempted_finalist_ids.pop();
                }
                10 => bad.confirmation_runs[0].cost_interval[0] = f64::NAN,
                11 => bad.frozen_before_search = false,
                _ => {
                    bad.confirmation_runs[0].confirmation_data_id =
                        bad.confirmation_runs[1].confirmation_data_id.clone()
                }
            }
            assert!(
                evaluate_confirmation_campaign_without_winner_leakage(&bad).is_err(),
                "mutation {mutation}"
            );
        }
    }

    #[test]
    fn v5_28_01_prospective_estimand_and_all_arm_seed_matrix_are_frozen() {
        let text =
            include_str!("../../../experiments/datasets/v5-28-02-confirmation-campaign.json");
        let mut ledger: ConfirmationCampaignLedger = serde_json::from_str(text).unwrap();
        let protocol: serde_json::Value = serde_json::from_str(include_str!(
            "../../../experiments/v5-study/preregistration-v1.json"
        ))
        .unwrap();
        assert!(protocol["estimands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == ledger.estimand_id));
        for run in &ledger.search_runs {
            assert!(protocol["arms"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["id"] == run.arm));
            assert!(protocol["independent_seeds"]["pilot"]
                .as_array()
                .unwrap()
                .contains(&run.search_seed_id.into()));
        }
        assert!(evaluate_confirmation_campaign_without_winner_leakage(&ledger).is_ok());
        ledger.frozen_before_search = false;
        assert!(evaluate_confirmation_campaign_without_winner_leakage(&ledger).is_err());
        ledger.frozen_before_search = true;
        ledger.confirmation_runs[0].estimand_id = "substituted-after-winners".into();
        ledger.reported_runs[0] = ledger.confirmation_runs[0].clone();
        assert!(evaluate_confirmation_campaign_without_winner_leakage(&ledger).is_err());
    }
}
