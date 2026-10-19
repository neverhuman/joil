//! Confirmation campaign audit, packet integration completeness, legacy migration parity,
//! CI tier verification, and four-track release scope reconciliation.

use super::*;

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
