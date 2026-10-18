//! Dataset namespaces and conservative surrogate calibration state.
//! This crate stores decisions; upstream joil-evidence admission remains authoritative.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetNamespace {
    IndependentQualified,
    SandboxPrior,
    SandboxPrunedInput,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionReason {
    Missing,
    Invalid,
    OutOfScope,
    UnqualifiedTier,
    Synthetic,
    PrunedRawInput,
    PriorDerived,
    MissingObserver,
    Ev09NotReady,
    Ev12IdentityIncomplete,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatasetReadiness {
    pub ev09_ready: bool,
    pub ev09_reasons: Vec<String>,
    pub ev12_complete: bool,
    pub ev12_reasons: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubjectIdentity {
    pub requirement_sha256: String,
    pub geometry_sha256: String,
    pub case_sha256: String,
    pub model_sha256: String,
    pub solver_sha256: String,
    pub scenario_sha256: String,
}
impl SubjectIdentity {
    fn valid(&self) -> bool {
        [
            &self.requirement_sha256,
            &self.geometry_sha256,
            &self.case_sha256,
            &self.model_sha256,
            &self.solver_sha256,
            &self.scenario_sha256,
        ]
        .iter()
        .all(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LabelCandidate {
    pub id: String,
    pub namespace: DatasetNamespace,
    pub subject: Option<SubjectIdentity>,
    pub quantity: String,
    pub unit: String,
    pub value: Option<f64>,
    pub evidence_tier: String,
    pub origin: String,
    pub observer_sha256: Option<String>,
    pub lineage_groups: Vec<String>,
    pub geometry_family_groups: Vec<String>,
    pub preparation_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdmittedLabel {
    pub id: String,
    pub subject: SubjectIdentity,
    pub cout_mg_per_litre_dispersed_oil: f64,
    pub lineage_groups: Vec<String>,
    pub geometry_family_groups: Vec<String>,
    pub preparation_id: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatasetBuild {
    pub independent: Vec<AdmittedLabel>,
    pub sandbox_priors: Vec<String>,
    pub sandbox_pruned_inputs: Vec<String>,
    pub excluded: Vec<(String, ExclusionReason)>,
}
/// Admission is deliberately strict and leaves all source/evidence verification to the evidence registry.
pub fn build_dataset(rows: &[LabelCandidate], readiness: &DatasetReadiness) -> DatasetBuild {
    let mut out = DatasetBuild {
        independent: vec![],
        sandbox_priors: vec![],
        sandbox_pruned_inputs: vec![],
        excluded: vec![],
    };
    for r in rows {
        if r.namespace == DatasetNamespace::SandboxPrior {
            out.sandbox_priors.push(r.id.clone());
            continue;
        }
        if r.namespace == DatasetNamespace::SandboxPrunedInput {
            out.sandbox_pruned_inputs.push(r.id.clone());
            out.excluded
                .push((r.id.clone(), ExclusionReason::PrunedRawInput));
            continue;
        }
        let reason = if !readiness.ev09_ready {
            Some(ExclusionReason::Ev09NotReady)
        } else if !readiness.ev12_complete {
            Some(ExclusionReason::Ev12IdentityIncomplete)
        } else if r.origin == "synthetic" {
            Some(ExclusionReason::Synthetic)
        } else if r.origin != "measured_independent" {
            Some(ExclusionReason::PriorDerived)
        } else if r.evidence_tier != "qualified_independent" {
            Some(ExclusionReason::UnqualifiedTier)
        } else if r.quantity != "cout" || r.unit != "mg/L_dispersed_oil" {
            Some(ExclusionReason::OutOfScope)
        } else if r.value.is_none() || r.value.is_some_and(|v| !v.is_finite() || v < 0.0) {
            Some(if r.value.is_none() {
                ExclusionReason::Missing
            } else {
                ExclusionReason::Invalid
            })
        } else if r.subject.as_ref().is_none_or(|s| !s.valid()) {
            Some(ExclusionReason::Ev12IdentityIncomplete)
        } else if r.observer_sha256.as_ref().is_none_or(|s| s.len() != 64) {
            Some(ExclusionReason::MissingObserver)
        } else if r.lineage_groups.is_empty()
            || r.geometry_family_groups.is_empty()
            || r.preparation_id.is_none()
        {
            Some(ExclusionReason::Missing)
        } else {
            None
        };
        if let Some(e) = reason {
            out.excluded.push((r.id.clone(), e));
            continue;
        }
        out.independent.push(AdmittedLabel {
            id: r.id.clone(),
            subject: r.subject.clone().unwrap(),
            cout_mg_per_litre_dispersed_oil: r.value.unwrap(),
            lineage_groups: r.lineage_groups.clone(),
            geometry_family_groups: r.geometry_family_groups.clone(),
            preparation_id: r.preparation_id.clone().unwrap(),
        });
    }
    out
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupFold {
    pub group: String,
    pub train: Vec<usize>,
    pub test: Vec<usize>,
}
fn group_names(row: &AdmittedLabel, geometry: bool) -> &[String] {
    if geometry {
        &row.geometry_family_groups
    } else {
        &row.lineage_groups
    }
}
/// Keep every member of a lineage/family and preparation on one side of a split.
pub fn grouped_folds(
    rows: &[AdmittedLabel],
    geometry: bool,
) -> Result<Vec<GroupFold>, &'static str> {
    if rows.len() < 5 {
        return Err("insufficient_independent_labels");
    };
    let names: BTreeSet<_> = rows
        .iter()
        .flat_map(|r| group_names(r, geometry).iter().cloned())
        .collect();
    if names.len() < 2 {
        return Err("insufficient_group_inventory");
    };
    let mut folds = vec![];
    for g in names {
        let test: Vec<_> = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| group_names(r, geometry).contains(&g))
            .map(|(i, _)| i)
            .collect();
        let test_prep: BTreeSet<_> = test
            .iter()
            .map(|i| rows[*i].preparation_id.as_str())
            .collect();
        let test_geom: BTreeSet<_> = test
            .iter()
            .flat_map(|i| rows[*i].geometry_family_groups.iter().map(String::as_str))
            .collect();
        let train: Vec<_> = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                !group_names(r, geometry).contains(&g)
                    && !test_prep.contains(r.preparation_id.as_str())
                    && !r
                        .geometry_family_groups
                        .iter()
                        .any(|x| test_geom.contains(x.as_str()))
            })
            .map(|(i, _)| i)
            .collect();
        if train.len() < 4 {
            return Err("insufficient_disjoint_group_support");
        };
        folds.push(GroupFold {
            group: g,
            train,
            test,
        });
    }
    Ok(folds)
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalibrationIdentity {
    pub training_dataset_sha256: String,
    pub observer_sha256: String,
    pub model_sha256: String,
    pub epoch: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalibrationState {
    pub identity: CalibrationIdentity,
    pub heldout_count: usize,
    pub interval_coverage: f64,
    pub coverage_lcb_95: f64,
    pub max_ood_distance: f64,
    pub supported_geometry_families: Vec<String>,
    pub calibrated: bool,
    pub abstain_reason: Option<String>,
}
/// Recompute interval coverage from held-out direct labels. A small sample or failed lower
/// confidence bound leaves the model uncalibrated; no prediction can be promoted from this state.
pub fn calibrate_heldout(
    identity: CalibrationIdentity,
    intervals: &[(f64, f64, f64)],
    supported_geometry_families: Vec<String>,
    max_ood_distance: f64,
    minimum_coverage_lcb: f64,
) -> Result<CalibrationState, &'static str> {
    if intervals
        .iter()
        .any(|(lo, hi, y)| !lo.is_finite() || !hi.is_finite() || !y.is_finite() || lo > hi)
        || !max_ood_distance.is_finite()
        || max_ood_distance < 0.0
        || !(0.0..=1.0).contains(&minimum_coverage_lcb)
    {
        return Err("invalid held-out calibration inputs");
    }
    let n = intervals.len();
    let covered = intervals
        .iter()
        .filter(|(lo, hi, y)| y >= lo && y <= hi)
        .count();
    let (coverage, lcb) = if n == 0 {
        (0.0, 0.0)
    } else {
        let p = covered as f64 / n as f64;
        let z = 1.959963984540054;
        let den = 1.0 + z * z / n as f64;
        let center = (p + z * z / (2.0 * n as f64)) / den;
        let half =
            z * ((p * (1.0 - p) / n as f64 + z * z / (4.0 * n as f64 * n as f64)).sqrt()) / den;
        (p, (center - half).max(0.0))
    };
    let calibrated =
        n >= 20 && lcb >= minimum_coverage_lcb && !supported_geometry_families.is_empty();
    Ok(CalibrationState {
        identity,
        heldout_count: n,
        interval_coverage: coverage,
        coverage_lcb_95: lcb,
        max_ood_distance,
        supported_geometry_families,
        calibrated,
        abstain_reason: if calibrated {
            None
        } else {
            Some("held-out coverage or geometry support insufficient".into())
        },
    })
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prediction {
    pub mean: f64,
    pub lower: f64,
    pub upper: f64,
    pub geometry_family: String,
    pub ood_distance: f64,
    pub namespace: DatasetNamespace,
}
impl CalibrationState {
    pub fn verify(
        &self,
        p: &Prediction,
        min_coverage_lcb: f64,
        max_ood: f64,
    ) -> Result<(f64, f64), &'static str> {
        if !self.calibrated {
            return Err("surrogate_miscalibrated_or_unready");
        }
        if !self
            .supported_geometry_families
            .contains(&p.geometry_family)
        {
            return Err("unsupported_geometry_abstain");
        }
        if !p.ood_distance.is_finite()
            || p.ood_distance > max_ood
            || p.ood_distance > self.max_ood_distance
        {
            return Err("out_of_distribution_abstain");
        }
        if self.heldout_count == 0 || self.coverage_lcb_95 < min_coverage_lcb {
            return Err("heldout_coverage_insufficient_abstain");
        }
        if !p.mean.is_finite()
            || !p.lower.is_finite()
            || !p.upper.is_finite()
            || p.lower > p.mean
            || p.mean > p.upper
        {
            return Err("invalid_prediction_interval");
        };
        Ok((p.lower, p.upper))
    }
}
/// Metric estimates are never archive candidates: callers receive only intervals for scheduling.
#[cfg(test)]
mod tests {
    use super::*;
    fn row(id: &str, ns: DatasetNamespace) -> LabelCandidate {
        LabelCandidate {
            id: id.into(),
            namespace: ns,
            subject: Some(SubjectIdentity {
                requirement_sha256: "a".repeat(64),
                geometry_sha256: "b".repeat(64),
                case_sha256: "c".repeat(64),
                model_sha256: "d".repeat(64),
                solver_sha256: "e".repeat(64),
                scenario_sha256: "f".repeat(64),
            }),
            quantity: "cout".into(),
            unit: "mg/L_dispersed_oil".into(),
            value: Some(2.),
            evidence_tier: "qualified_independent".into(),
            origin: "measured_independent".into(),
            observer_sha256: Some("a".repeat(64)),
            lineage_groups: vec![format!("l{id}")],
            geometry_family_groups: vec![format!("g{id}")],
            preparation_id: Some(id.into()),
        }
    }
    #[test]
    fn sandbox_and_invalid_labels_never_enter_independent_namespace() {
        let mut prior = row("p", DatasetNamespace::SandboxPrior);
        prior.evidence_tier = "qualified_independent".into();
        let mut pruned = row("x", DatasetNamespace::SandboxPrunedInput);
        pruned.namespace = DatasetNamespace::SandboxPrunedInput;
        let mut bad = row("bad", DatasetNamespace::IndependentQualified);
        bad.origin = "recipe_prior".into();
        let ready = DatasetReadiness {
            ev09_ready: true,
            ev09_reasons: vec![],
            ev12_complete: true,
            ev12_reasons: vec![],
        };
        let d = build_dataset(&[prior, pruned, bad], &ready);
        assert!(d.independent.is_empty());
        assert_eq!(d.sandbox_priors.len(), 1);
        assert_eq!(d.sandbox_pruned_inputs.len(), 1);
    }
    #[test]
    fn calibration_abstains_out_of_scope_or_low_coverage() {
        let s = CalibrationState {
            identity: CalibrationIdentity {
                training_dataset_sha256: "a".repeat(64),
                observer_sha256: "b".repeat(64),
                model_sha256: "c".repeat(64),
                epoch: 0,
            },
            heldout_count: 20,
            interval_coverage: 0.95,
            coverage_lcb_95: 0.8,
            max_ood_distance: 1.,
            supported_geometry_families: vec!["g".into()],
            calibrated: true,
            abstain_reason: None,
        };
        let p = Prediction {
            mean: 1.,
            lower: 0.,
            upper: 2.,
            geometry_family: "other".into(),
            ood_distance: 0.1,
            namespace: DatasetNamespace::IndependentQualified,
        };
        assert_eq!(s.verify(&p, 0.7, 0.9), Err("unsupported_geometry_abstain"));
    }
    #[test]
    fn lineage_folds_exclude_overlapping_geometry_and_preparation() {
        let rows: Vec<_> = (0..24)
            .map(|i| AdmittedLabel {
                id: format!("s{i}"),
                subject: SubjectIdentity {
                    requirement_sha256: "a".repeat(64),
                    geometry_sha256: format!("{:064x}", i),
                    case_sha256: "b".repeat(64),
                    model_sha256: "c".repeat(64),
                    solver_sha256: "d".repeat(64),
                    scenario_sha256: "e".repeat(64),
                },
                cout_mg_per_litre_dispersed_oil: i as f64,
                lineage_groups: vec![format!("l{}", i / 4)],
                geometry_family_groups: vec![format!("g{}", i % 6)],
                preparation_id: format!("p{i}"),
            })
            .collect();
        for fold in grouped_folds(&rows, false).unwrap() {
            for &test in &fold.test {
                for &train in &fold.train {
                    assert_ne!(rows[test].preparation_id, rows[train].preparation_id);
                    assert!(rows[test]
                        .geometry_family_groups
                        .iter()
                        .all(|g| !rows[train].geometry_family_groups.contains(g)));
                }
            }
        }
    }
}
