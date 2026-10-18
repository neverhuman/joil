//! V5-13-03: geometric manufacturing gates driven by passport thresholds.
//!
//! Measurements of one accepted subject are compared with thresholds that must be admitted
//! with a source; no threshold is copied from the legacy source. A threshold or measurement
//! that is missing makes the gate `Unresolved`. A machined feature needs its own explicit
//! process/access plan, and that exception never qualifies a printed channel.
use crate::{invalid, Field, JudgeError, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessThresholds {
    pub build_volume_m: Field<[f64; 3]>,
    pub min_wall_m: Field<f64>,
    pub max_overhang_deg_from_vertical: Field<f64>,
    pub max_unsupported_span_m: Field<f64>,
    pub min_support_access_m: Field<f64>,
    pub min_printed_passage_m: Field<f64>,
    pub min_powder_escape_clearance_m: Field<f64>,
    pub tool_clearance_margin_m: Field<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportRegion {
    pub id: String,
    pub access_diameter_m: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrintedChannel {
    pub id: String,
    pub min_diameter_m: Option<f64>,
}

/// Swept volume of a named tool along its approach: the clear radius found around the path.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolAccess {
    pub tool_id: String,
    pub tool_radius_m: f64,
    pub swept_clear_radius_m: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cavity {
    pub id: String,
    /// `false` = the observer could not resolve this cavity (unknown, not a pass).
    pub resolved: bool,
    /// Bottleneck diameter of the best finite-clearance escape path; `None` = no path found.
    pub escape_clearance_diameter_m: Option<f64>,
    pub reamer: Option<ToolAccess>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessPlan {
    pub process_plan_source: Sha256Hex,
    pub access: ToolAccess,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachinedFeature {
    pub id: String,
    pub plan: Option<AccessPlan>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectMeasurements {
    pub subject_sha256: Sha256Hex,
    pub bounding_box_m: Option<[f64; 3]>,
    pub min_wall_m: Option<f64>,
    pub max_overhang_deg_from_vertical: Option<f64>,
    pub max_unsupported_span_m: Option<f64>,
    pub supports: Vec<SupportRegion>,
    pub printed_channels: Vec<PrintedChannel>,
    pub cavities: Vec<Cavity>,
    pub machined_features: Vec<MachinedFeature>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GateReport {
    pub gates: Vec<(String, Verdict)>,
    pub overall: Verdict,
}

#[derive(Clone, Copy)]
enum Op {
    AtLeast,
    AtMost,
}

fn threshold(name: &str, field: &Field<f64>) -> Result<f64, Verdict> {
    match field.admitted() {
        None => Err(Verdict::Unresolved(vec![format!(
            "{name}:threshold_not_admitted"
        )])),
        Some(v) if v.is_finite() && *v > 0.0 => Ok(*v),
        Some(_) => Err(Verdict::Unresolved(vec![format!(
            "{name}:threshold_invalid"
        )])),
    }
}

fn compare(name: &str, measured: Option<f64>, field: &Field<f64>, op: Op) -> Verdict {
    let limit = match threshold(name, field) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let Some(m) = measured else {
        return Verdict::Unresolved(vec![format!("{name}:not_measured")]);
    };
    if !m.is_finite() {
        return Verdict::Fail(vec![format!("{name}:nonfinite_measurement")]);
    }
    let ok = match op {
        Op::AtLeast => m >= limit,
        Op::AtMost => m <= limit,
    };
    if ok {
        Verdict::Pass
    } else {
        Verdict::Fail(vec![format!("{name}:{m:.6e}_violates_{limit:.6e}")])
    }
}

/// Tool reaches its target when the swept clear radius covers the tool radius plus margin.
fn tool_reaches(name: &str, tool: &ToolAccess, margin: &Field<f64>) -> Verdict {
    if !(tool.tool_radius_m.is_finite() && tool.tool_radius_m > 0.0) {
        return Verdict::Fail(vec![format!("{name}:invalid_tool_radius")]);
    }
    let margin = match threshold("tool_clearance_margin", margin) {
        Ok(v) => v,
        Err(v) => return v,
    };
    match tool.swept_clear_radius_m {
        None => Verdict::Fail(vec![format!(
            "{name}:tool_{}_has_no_clear_swept_path",
            tool.tool_id
        )]),
        Some(r) if r.is_finite() && r >= tool.tool_radius_m + margin => Verdict::Pass,
        Some(_) => Verdict::Fail(vec![format!(
            "{name}:tool_{}_swept_volume_blocked",
            tool.tool_id
        )]),
    }
}

fn unique<'a>(ids: impl Iterator<Item = &'a str>) -> Result<(), JudgeError> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() || !seen.insert(id) {
            return Err(invalid("empty or duplicate feature id"));
        }
    }
    Ok(())
}

/// Evaluate every geometric gate for the subject named by `expected_subject`.
pub fn evaluate(
    expected_subject: &Sha256Hex,
    thresholds: &ProcessThresholds,
    m: &SubjectMeasurements,
) -> Result<GateReport, JudgeError> {
    if &m.subject_sha256 != expected_subject {
        return Err(invalid(
            "measurements belong to a different subject than the accepted one",
        ));
    }
    unique(m.supports.iter().map(|s| s.id.as_str()))?;
    unique(m.printed_channels.iter().map(|s| s.id.as_str()))?;
    unique(m.cavities.iter().map(|s| s.id.as_str()))?;
    unique(m.machined_features.iter().map(|s| s.id.as_str()))?;
    for v in m.bounding_box_m.iter().flatten().chain(m.min_wall_m.iter()) {
        if v.is_nan() {
            return Err(invalid("NaN measurement"));
        }
    }
    let mut gates: Vec<(String, Verdict)> = Vec::new();

    // Build fit, per axis in the build frame.
    let fit = match (thresholds.build_volume_m.admitted(), m.bounding_box_m) {
        (None, _) => Verdict::Unresolved(vec!["build_volume:threshold_not_admitted".into()]),
        (Some(_), None) => Verdict::Unresolved(vec!["build_volume:not_measured".into()]),
        (Some(limit), Some(b)) => {
            if limit.iter().any(|v| !(v.is_finite() && *v > 0.0)) {
                Verdict::Unresolved(vec!["build_volume:threshold_invalid".into()])
            } else if b
                .iter()
                .zip(limit)
                .any(|(have, cap)| !(have.is_finite() && *have > 0.0 && *have <= *cap))
            {
                Verdict::Fail(vec!["build_volume:subject_exceeds_build_envelope".into()])
            } else {
                Verdict::Pass
            }
        }
    };
    gates.push(("build_fit".into(), fit));
    gates.push((
        "min_wall".into(),
        compare(
            "min_wall",
            m.min_wall_m,
            &thresholds.min_wall_m,
            Op::AtLeast,
        ),
    ));
    gates.push((
        "internal_downskin".into(),
        compare(
            "internal_downskin",
            m.max_overhang_deg_from_vertical,
            &thresholds.max_overhang_deg_from_vertical,
            Op::AtMost,
        ),
    ));
    gates.push((
        "unsupported_span".into(),
        compare(
            "unsupported_span",
            m.max_unsupported_span_m,
            &thresholds.max_unsupported_span_m,
            Op::AtMost,
        ),
    ));
    gates.push((
        "support_accessibility".into(),
        Verdict::combine(m.supports.iter().map(|s| {
            compare(
                &format!("support:{}", s.id),
                s.access_diameter_m,
                &thresholds.min_support_access_m,
                Op::AtLeast,
            )
        })),
    ));
    gates.push((
        "printed_channels".into(),
        Verdict::combine(m.printed_channels.iter().map(|c| {
            compare(
                &format!("printed_channel:{}", c.id),
                c.min_diameter_m,
                &thresholds.min_printed_passage_m,
                Op::AtLeast,
            )
        })),
    ));
    gates.push((
        "powder_escape".into(),
        Verdict::combine(m.cavities.iter().map(|c| cavity(c, thresholds))),
    ));
    gates.push((
        "machined_access".into(),
        Verdict::combine(m.machined_features.iter().map(|f| match &f.plan {
            None => Verdict::Fail(vec![format!(
                "machined:{}:no_explicit_process_access_plan",
                f.id
            )]),
            Some(plan) => tool_reaches(
                &format!("machined:{}", f.id),
                &plan.access,
                &thresholds.tool_clearance_margin_m,
            ),
        })),
    ));
    let overall = Verdict::combine(gates.iter().map(|(_, v)| v.clone()));
    Ok(GateReport { gates, overall })
}

fn cavity(c: &Cavity, t: &ProcessThresholds) -> Verdict {
    let name = format!("cavity:{}", c.id);
    if !c.resolved {
        return Verdict::Unresolved(vec![format!("{name}:unresolved_by_observer")]);
    }
    let escape = match c.escape_clearance_diameter_m {
        None => Verdict::Fail(vec![format!("{name}:no_finite_clearance_escape_path")]),
        Some(d) => compare(
            &name,
            Some(d),
            &t.min_powder_escape_clearance_m,
            Op::AtLeast,
        ),
    };
    let reamer = match &c.reamer {
        None => Verdict::Fail(vec![format!("{name}:no_reamer_access")]),
        Some(tool) => tool_reaches(&name, tool, &t.tool_clearance_margin_m),
    };
    if escape.is_pass() || reamer.is_pass() {
        return Verdict::Pass;
    }
    let open = [&escape, &reamer]
        .iter()
        .any(|v| matches!(v, Verdict::Unresolved(_)));
    if open {
        Verdict::combine(
            [escape, reamer]
                .into_iter()
                .filter(|v| matches!(v, Verdict::Unresolved(_))),
        )
    } else {
        Verdict::combine([escape, reamer])
    }
}
