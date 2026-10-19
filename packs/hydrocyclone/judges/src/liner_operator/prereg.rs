//! Preregistered case splits, resource lease verification, and 12-case substitute corpus for `liner_operator`.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Split {
    Training,
    Calibration,
    HeldOut,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseRecord {
    pub case_id: String,
    pub split: Split,
    pub geometry_family: String,
    /// Full accepted geometry, identified by its digest.
    pub geometry_sha256: Sha256Hex,
    pub carrier_evidence_sha256: Sha256Hex,
    pub transport_evidence_sha256: Sha256Hex,
    /// Mechanism and state-domain tags the case's generated states cover.
    pub domain_tags: Vec<String>,
    /// Generated fixtures are synthetic and can never close a scientific gate.
    pub synthetic: bool,
}

/// Compute resources measured for the case-generation and reference lease.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasuredLease {
    pub cpu_cores: u32,
    pub memory_gib: f64,
    pub output_gib: f64,
    pub gpu: bool,
    pub lease_sha256: Sha256Hex,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeclaredGap {
    pub tag: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preregistration {
    pub required_domain_tags: Vec<String>,
    pub declared_gaps: Vec<DeclaredGap>,
    pub cases: Vec<CaseRecord>,
    pub lease: Field<MeasuredLease>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreregistrationReport {
    /// Tags covered by at least one held-out case.
    pub held_out_tags: Vec<String>,
    /// Required tags that no held-out case covers and that are declared as gaps.
    pub declared_gaps: Vec<String>,
    pub held_out_cases: usize,
    pub synthetic_cases: usize,
    /// True only when the protocol passes and every case is non-synthetic.
    pub science_admissible: bool,
    pub verdict: Verdict,
}

/// Check the preregistered case protocol. Geometry-family separation and held-out independence
/// are `Fail`, because a reused head or family leaks the answer into validation. Missing evidence,
/// an unmeasured lease or an unexplained coverage gap is `Unresolved`.
pub fn judge_preregistration(pre: &Preregistration) -> PreregistrationReport {
    let mut fails = Vec::new();
    let mut open_items = Vec::new();
    match pre.lease.admitted() {
        Some(l) => {
            let within = l.cpu_cores >= 1
                && l.cpu_cores <= INITIAL_MAX_CORES
                && l.memory_gib > 0.0
                && l.memory_gib <= INITIAL_MAX_MEMORY_GIB
                && l.output_gib >= 0.0
                && l.output_gib <= INITIAL_MAX_OUTPUT_GIB
                && !l.gpu;
            if !within {
                fails.push("lease:exceeds_initial_joil_allocation".to_string());
            }
        }
        None => open_items.push("lease:not_measured_or_admitted".to_string()),
    }

    let mut ids = std::collections::BTreeSet::new();
    for c in &pre.cases {
        if !ids.insert(c.case_id.as_str()) {
            fails.push(format!("split:duplicate_case_id:{}", c.case_id));
        }
    }
    let training: Vec<&CaseRecord> = pre
        .cases
        .iter()
        .filter(|c| c.split != Split::HeldOut)
        .collect();
    let held: Vec<&CaseRecord> = pre
        .cases
        .iter()
        .filter(|c| c.split == Split::HeldOut)
        .collect();
    if held.is_empty() {
        open_items.push("split:no_held_out_cases".to_string());
    }
    for h in &held {
        for t in &training {
            if t.geometry_family == h.geometry_family {
                fails.push(format!("split:family_overlap:{}", h.geometry_family));
                break;
            }
        }
        if training.iter().any(|t| {
            t.geometry_sha256 == h.geometry_sha256
                || t.carrier_evidence_sha256 == h.carrier_evidence_sha256
                || t.transport_evidence_sha256 == h.transport_evidence_sha256
        }) {
            fails.push(format!(
                "split:held_out_reuses_training_or_calibration_head:{}",
                h.case_id
            ));
        }
    }

    let held_tags: std::collections::BTreeSet<&str> = held
        .iter()
        .flat_map(|c| c.domain_tags.iter().map(String::as_str))
        .collect();
    let mut declared = Vec::new();
    for tag in &pre.required_domain_tags {
        if held_tags.contains(tag.as_str()) {
            continue;
        }
        if pre.declared_gaps.iter().any(|g| &g.tag == tag) {
            declared.push(tag.clone());
        } else {
            open_items.push(format!("coverage:unexplained_gap:{tag}"));
        }
    }
    let synthetic_cases = pre.cases.iter().filter(|c| c.synthetic).count();
    let verdict = if !fails.is_empty() {
        Verdict::Fail(fails)
    } else if !open_items.is_empty() {
        Verdict::Unresolved(open_items)
    } else {
        Verdict::Pass
    };
    PreregistrationReport {
        held_out_tags: held_tags.iter().map(|t| t.to_string()).collect(),
        declared_gaps: declared,
        held_out_cases: held.len(),
        synthetic_cases,
        science_admissible: verdict.is_pass() && synthetic_cases == 0,
        verdict,
    }
}

/// Build a fixed 12-case substitute corpus with disjoint geometry families and evidence IDs.
/// The corpus is a protocol fixture only; its synthetic labels do not admit physical science.
pub fn build_preregistered_full_liner_corpus(
    lease: Field<MeasuredLease>,
    declared_gaps: Vec<DeclaredGap>,
) -> Result<(Preregistration, PreregistrationReport), JudgeError> {
    if lease.admitted().is_none() {
        return Err(invalid("full_liner:measured_lease_required"));
    }
    let plan = [
        ("cyl_conical_a", Split::Training, 4),
        ("cyl_conical_b", Split::Training, 4),
        ("tapered_swirl_c", Split::Calibration, 2),
        ("helical_liner_d", Split::HeldOut, 1),
        ("stepped_cone_e", Split::HeldOut, 1),
    ];
    let tags = [
        "reverse_core",
        "swirl",
        "oil_grade",
        "pressure_drop",
        "low_flow",
        "high_flow",
    ];
    let mut cases = Vec::new();
    let mut index = 0usize;
    for (family, split, count) in plan {
        for member in 0..count {
            let case_id = format!("full-liner-{family}-{member:02}");
            let digest = |kind: &str| Sha256Hex::of_bytes(format!("{case_id}:{kind}").as_bytes());
            let domain_tags = if split == Split::HeldOut {
                if family == "helical_liner_d" {
                    tags[..3].iter().map(|tag| tag.to_string()).collect()
                } else {
                    tags[3..].iter().map(|tag| tag.to_string()).collect()
                }
            } else {
                vec![
                    tags[index % tags.len()].to_string(),
                    tags[(index + 1) % tags.len()].to_string(),
                ]
            };
            cases.push(CaseRecord {
                case_id: case_id.clone(),
                split,
                geometry_family: family.to_string(),
                geometry_sha256: digest("geometry"),
                carrier_evidence_sha256: digest("carrier"),
                transport_evidence_sha256: digest("transport"),
                domain_tags,
                synthetic: true,
            });
            index += 1;
        }
    }
    let pre = Preregistration {
        required_domain_tags: tags
            .iter()
            .map(|x| x.to_string())
            .chain(std::iter::once("wet_liner_surface_chemistry".to_string()))
            .collect(),
        declared_gaps,
        cases,
        lease,
    };
    let report = judge_preregistration(&pre);
    if !report.verdict.is_pass() {
        return Err(invalid(format!(
            "full_liner:preregistration_rejected:{:?}",
            report.verdict
        )));
    }
    Ok((pre, report))
}

/// Parse and rejudge a persisted corpus before it is used by a consumer.
pub fn verify_full_liner_corpus_json(raw_json: &str) -> Result<PreregistrationReport, JudgeError> {
    let pre: Preregistration = serde_json::from_str(raw_json)
        .map_err(|e| invalid(format!("full_liner:invalid_json:{e}")))?;
    let report = judge_preregistration(&pre);
    if report.verdict.is_pass() {
        Ok(report)
    } else {
        Err(invalid(format!(
            "full_liner:verification_failed:{:?}",
            report.verdict
        )))
    }
}
