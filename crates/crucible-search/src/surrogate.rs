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
        let test_lineage: BTreeSet<_> = test
            .iter()
            .flat_map(|i| rows[*i].lineage_groups.iter().map(String::as_str))
            .collect();
        let train: Vec<_> = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                !group_names(r, geometry).contains(&g)
                    && !test_prep.contains(r.preparation_id.as_str())
                    && !r
                        .lineage_groups
                        .iter()
                        .any(|group| test_lineage.contains(group.as_str()))
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

/// Reproducible independent-label admission result. Sandbox sources stay addressable but never
/// enter the independent training namespace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OracleDatasetManifest {
    pub admitted: Vec<AdmittedLabel>,
    pub sandbox_priors: Vec<String>,
    pub sandbox_pruned_inputs: Vec<String>,
    pub excluded: Vec<(String, ExclusionReason)>,
    pub geometry_family_folds: Vec<GroupFold>,
    pub lineage_folds: Vec<GroupFold>,
    pub ev09_ready: bool,
    pub ev09_reasons: Vec<String>,
    pub ev12_complete: bool,
    pub ev12_reasons: Vec<String>,
}

/// Admit only subject-bound, independently sourced oracle labels, then build two auditable
/// grouped holdout plans. Every fold is checked across preparation, geometry and lineage identity.
pub fn build_numerical_oracle_dataset_with_splits(
    rows: &[LabelCandidate],
    readiness: &DatasetReadiness,
) -> Result<OracleDatasetManifest, &'static str> {
    let mut admitted = Vec::new();
    let (mut sandbox_priors, mut sandbox_pruned_inputs, mut excluded) =
        (Vec::new(), Vec::new(), Vec::new());
    for row in rows {
        if row.namespace == DatasetNamespace::SandboxPrior {
            sandbox_priors.push(row.id.clone());
            continue;
        }
        if row.namespace == DatasetNamespace::SandboxPrunedInput {
            sandbox_pruned_inputs.push(row.id.clone());
            excluded.push((row.id.clone(), ExclusionReason::PrunedRawInput));
            continue;
        }
        let reason = if row.value.is_none() {
            Some(ExclusionReason::Missing)
        } else if row.value.is_some_and(|v| !v.is_finite() || v < 0.0) {
            Some(ExclusionReason::Invalid)
        } else if row.quantity != "cout" || row.unit != "mg/L_dispersed_oil" {
            Some(ExclusionReason::OutOfScope)
        } else if row.origin == "synthetic" {
            Some(ExclusionReason::Synthetic)
        } else if row.origin == "recipe_prior" || row.origin == "surrogate_prior" {
            Some(ExclusionReason::PriorDerived)
        } else if !matches!(
            (row.origin.as_str(), row.evidence_tier.as_str()),
            ("measured_independent", "qualified_independent")
                | ("native_high_fidelity_oracle", "numerical_oracle")
        ) {
            Some(ExclusionReason::UnqualifiedTier)
        } else if row.subject.is_none() {
            Some(ExclusionReason::Missing)
        } else if !row.subject.as_ref().is_some_and(SubjectIdentity::valid) {
            Some(ExclusionReason::Ev12IdentityIncomplete)
        } else if row
            .observer_sha256
            .as_ref()
            .is_none_or(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            Some(ExclusionReason::MissingObserver)
        } else if row.lineage_groups.is_empty()
            || row.geometry_family_groups.is_empty()
            || row.preparation_id.as_deref().is_none_or(str::is_empty)
        {
            Some(ExclusionReason::Missing)
        } else if !readiness.ev09_ready {
            Some(ExclusionReason::Ev09NotReady)
        } else if !readiness.ev12_complete {
            Some(ExclusionReason::Ev12IdentityIncomplete)
        } else {
            None
        };
        if let Some(reason) = reason {
            excluded.push((row.id.clone(), reason));
        } else {
            admitted.push(AdmittedLabel {
                id: row.id.clone(),
                subject: row.subject.clone().expect("validated subject"),
                cout_mg_per_litre_dispersed_oil: row.value.expect("validated value"),
                lineage_groups: row.lineage_groups.clone(),
                geometry_family_groups: row.geometry_family_groups.clone(),
                preparation_id: row.preparation_id.clone().expect("validated preparation"),
            });
        }
    }
    let geometry_family_folds = grouped_folds(&admitted, true)?;
    let lineage_folds = grouped_folds(&admitted, false)?;
    for fold in geometry_family_folds.iter().chain(&lineage_folds) {
        for &test_i in &fold.test {
            let test = &admitted[test_i];
            for &train_i in &fold.train {
                let train = &admitted[train_i];
                if test.preparation_id == train.preparation_id
                    || test
                        .geometry_family_groups
                        .iter()
                        .any(|g| train.geometry_family_groups.contains(g))
                    || test
                        .lineage_groups
                        .iter()
                        .any(|g| train.lineage_groups.contains(g))
                {
                    return Err("grouped_fold_identity_overlap");
                }
            }
        }
    }
    Ok(OracleDatasetManifest {
        admitted,
        sandbox_priors,
        sandbox_pruned_inputs,
        excluded,
        geometry_family_folds,
        lineage_folds,
        ev09_ready: readiness.ev09_ready,
        ev09_reasons: readiness.ev09_reasons.clone(),
        ev12_complete: readiness.ev12_complete,
        ev12_reasons: readiness.ev12_reasons.clone(),
    })
}

pub mod calibration;
pub use calibration::*;
