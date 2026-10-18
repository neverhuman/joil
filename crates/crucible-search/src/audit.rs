//! Randomized audit summaries; a screen cannot prune until its registered budget passes.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditPolicy {
    pub screen_id: String,
    pub epoch: u64,
    pub audit_budget: u64,
    pub audit_cost: u64,
    pub min_recall_lcb: f64,
    pub max_rank_disagreement: f64,
    pub authority_suspended: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditResult {
    pub sampled: usize,
    pub screened_positive: usize,
    pub missed: usize,
    pub recall: Option<f64>,
    pub recall_wilson_95: Option<(f64, f64)>,
    pub rank_disagreement: Option<f64>,
    pub disagreement_wilson_95: Option<(f64, f64)>,
    pub false_omission_rate: Option<f64>,
    pub cost_charged: u64,
    pub pruning_authorized: bool,
}

/// Deterministic stratified audit sample. Candidate indices are split by direct-screen rank
/// quantiles, then shuffled within each stratum so low-ranked omissions remain observable.
pub fn randomized_audit_stream(
    ranks: &[f64],
    sample_size: usize,
    seed: u64,
) -> Result<Vec<usize>, &'static str> {
    if ranks.is_empty() || ranks.iter().any(|x| !x.is_finite()) || sample_size == 0 {
        return Err("audit stream requires finite ranks and positive sample size");
    }
    let mut order: Vec<usize> = (0..ranks.len()).collect();
    order.sort_by(|a, b| ranks[*a].total_cmp(&ranks[*b]).then(a.cmp(b)));
    let strata = sample_size.min(order.len()).min(4);
    let take = sample_size.min(order.len());
    let mut state = seed.max(1);
    let mut selected = Vec::with_capacity(take);
    let per_stratum = take / strata;
    let remainder = take % strata;
    for stratum in 0..strata {
        let start = stratum * order.len() / strata;
        let end = (stratum + 1) * order.len() / strata;
        let mut bucket = order[start..end].to_vec();
        for i in (1..bucket.len()).rev() {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            bucket.swap(
                i,
                (state.wrapping_mul(0x2545_f491_4f6c_dd1d) as usize) % (i + 1),
            );
        }
        let quota = per_stratum + usize::from(stratum < remainder);
        selected.extend(bucket.into_iter().take(quota));
    }
    if selected.len() != take {
        return Err("audit stratification failed to fill requested sample");
    }
    for idx in &selected {
        if *idx >= ranks.len() {
            return Err("audit stream produced an invalid candidate index");
        }
    }
    Ok(selected)
}
fn wilson(k: usize, n: usize) -> Option<(f64, f64)> {
    if n == 0 {
        return None;
    }
    let z = 1.959963984540054;
    let p = k as f64 / n as f64;
    let d = 1.0 + z * z / n as f64;
    let center = (p + z * z / (2.0 * n as f64)) / d;
    let half = z * ((p * (1.0 - p) / n as f64 + z * z / (4.0 * (n * n) as f64)).sqrt()) / d;
    Some(((center - half).max(0.0), (center + half).min(1.0)))
}
/// Compare ranks on a separately randomized sample, including low-ranked candidates. `screened`
/// means retained by the proposed screen; `direct_positive` is the direct-evaluation audit label.
pub fn audit(
    policy: &AuditPolicy,
    screened: &[bool],
    direct_positive: &[bool],
    screen_rank: &[f64],
    direct_rank: &[f64],
) -> Result<AuditResult, &'static str> {
    let n = screened.len();
    if n == 0
        || direct_positive.len() != n
        || screen_rank.len() != n
        || direct_rank.len() != n
        || screen_rank
            .iter()
            .chain(direct_rank)
            .any(|x| !x.is_finite())
    {
        return Err("audit vectors must be nonempty, equal length and finite");
    }
    let positives = direct_positive.iter().filter(|x| **x).count();
    let missed = (0..n)
        .filter(|i| direct_positive[*i] && !screened[*i])
        .count();
    let recall = if positives == 0 {
        None
    } else {
        Some((positives - missed) as f64 / positives as f64)
    };
    let mut discord = 0usize;
    let mut pairs = 0usize;
    for i in 0..n {
        for j in i + 1..n {
            let a = (screen_rank[i] - screen_rank[j]).signum();
            let b = (direct_rank[i] - direct_rank[j]).signum();
            if a != 0.0 && b != 0.0 {
                pairs += 1;
                if a != b {
                    discord += 1
                }
            }
        }
    }
    let disagreement = if pairs == 0 {
        None
    } else {
        Some(discord as f64 / pairs as f64)
    };
    let budget_ok = policy.audit_cost <= policy.audit_budget;
    let authorized = budget_ok
        && recall.zip(disagreement).is_some_and(|(_r, d)| {
            wilson(positives - missed, positives).is_some_and(|(l, _)| l >= policy.min_recall_lcb)
                && d <= policy.max_rank_disagreement
        })
        && !policy.authority_suspended;
    Ok(AuditResult {
        sampled: n,
        screened_positive: positives - missed,
        missed,
        recall,
        recall_wilson_95: wilson(positives - missed, positives),
        rank_disagreement: disagreement,
        disagreement_wilson_95: wilson(discord, pairs),
        false_omission_rate: if positives == 0 {
            None
        } else {
            Some(missed as f64 / positives as f64)
        },
        cost_charged: policy.audit_cost,
        pruning_authorized: authorized,
    })
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExploitChallenge {
    pub resolution_levels: Vec<String>,
    pub rotations_deg: Vec<f64>,
    pub planted_gap_ids: Vec<String>,
    pub disagreement_island: Option<String>,
    pub detected: bool,
    pub quarantined_claim_ids: Vec<String>,
    pub regression_id: Option<String>,
}
/// A failed rotation, resolution, planted-gap, or disagreement-island challenge quarantines
/// every claim depending on the affected screen and yields a stable regression identifier.
pub fn evaluate_exploit_challenge(
    resolution_stable: bool,
    rotation_stable: bool,
    planted_gaps_found: &[bool],
    disagreement_island_found: Option<bool>,
    dependent_claim_ids: &[String],
    regression_id: &str,
) -> ExploitChallenge {
    let detected = !resolution_stable
        || !rotation_stable
        || planted_gaps_found.iter().any(|x| !x)
        || disagreement_island_found == Some(false);
    ExploitChallenge {
        resolution_levels: vec!["coarse".into(), "medium".into(), "fine".into()],
        rotations_deg: vec![0.0, 90.0, 180.0, 270.0],
        planted_gap_ids: (0..planted_gaps_found.len())
            .map(|i| format!("planted-gap-{i}"))
            .collect(),
        disagreement_island: disagreement_island_found
            .map(|_| "optional-disagreement-island".into()),
        detected,
        quarantined_claim_ids: if detected {
            dependent_claim_ids.to_vec()
        } else {
            Vec::new()
        },
        regression_id: detected.then(|| regression_id.to_string()),
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenEpoch {
    pub epoch: u64,
    pub coordinate_sha256: String,
    pub crosswalk_sha256: String,
    pub policy_version: String,
}
pub fn verify_epoch(old: &FrozenEpoch, new: &FrozenEpoch) -> Result<(), &'static str> {
    if old.epoch != new.epoch
        || old.coordinate_sha256 != new.coordinate_sha256
        || old.crosswalk_sha256 != new.crosswalk_sha256
        || old.policy_version != new.policy_version
    {
        Err("adaptive policy revision requires a new frozen epoch and crosswalk")
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn low_rank_audit_detects_misses_and_failed_budget_suspends_pruning() {
        let p = AuditPolicy {
            screen_id: "s".into(),
            epoch: 0,
            audit_budget: 3,
            audit_cost: 4,
            min_recall_lcb: 0.8,
            max_rank_disagreement: 0.2,
            authority_suspended: false,
        };
        let r = audit(
            &p,
            &[true, true, false, false],
            &[true, false, true, false],
            &[0., 1., 2., 3.],
            &[0., 1., 3., 2.],
        )
        .unwrap();
        assert!(!r.pruning_authorized);
        assert_eq!(r.missed, 1);
        assert_eq!(r.cost_charged, 4);
        assert!(r.recall_wilson_95.is_some());
    }
    #[test]
    fn audit_stream_is_seeded_and_covers_low_rank_stratum() {
        let ranks = [0., 1., 2., 3., 4., 5., 6., 7., 8., 9.];
        let a = randomized_audit_stream(&ranks, 4, 8).unwrap();
        assert_eq!(a, randomized_audit_stream(&ranks, 4, 8).unwrap());
        assert!(a.iter().any(|i| *i >= 8));
    }
    #[test]
    fn exploit_challenge_quarantines_dependent_claims_and_pins_regression() {
        let c = evaluate_exploit_challenge(
            true,
            true,
            &[false],
            Some(true),
            &["claim-1".into()],
            "audit-regression-1",
        );
        assert!(c.detected);
        assert_eq!(c.quarantined_claim_ids, vec!["claim-1"]);
        assert_eq!(c.regression_id.as_deref(), Some("audit-regression-1"));
    }
}
