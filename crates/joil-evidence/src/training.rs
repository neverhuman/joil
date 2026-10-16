//! Independent-label admission and geometry/mechanism holdouts. No gene-derived targets.
use crate::claims::Registry;
use crate::identity::{check_id, strict_decode, ContentRef, SubjectRef};
use crate::observation::{EvidenceTier, Finite, Origin, Unit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedFeatureSpec {
    pub quantity: String,
    pub unit: Unit,
    pub offset: Finite,
    pub scale: Finite,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureContract {
    pub id: String,
    pub features: Vec<ObservedFeatureSpec>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupKind {
    GeometryFamily,
    Mechanism,
}

/// Reviewed observer output, distinct from an authored genome family name.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedGroups {
    pub kind: GroupKind,
    pub identity_sha256: String,
    pub labels: Vec<String>,
    pub observer: ContentRef,
    pub observations: Vec<ContentRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainingInput {
    pub id: String,
    pub label: ContentRef,
    pub features: [ContentRef; 12],
    pub geometry_groups: ContentRef,
    pub mechanism_groups: ContentRef,
}

/// Opaque admission; deserializing a submitted record cannot create this authority.
#[derive(Clone, Debug)]
pub struct AdmittedSample {
    id: String,
    features: [f64; 12],
    target: Finite,
    geometry_hash: String,
    family_groups: Vec<String>,
    mechanism_groups: Vec<String>,
    label: ContentRef,
}
impl AdmittedSample {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn features(&self) -> &[f64; 12] {
        &self.features
    }
    pub fn target(&self) -> Finite {
        self.target
    }
    pub fn label(&self) -> &ContentRef {
        &self.label
    }
}

pub fn admit_sample(
    registry: &Registry,
    requirements: &[ContentRef],
    feature_contract: &ContentRef,
    input: &TrainingInput,
) -> Result<AdmittedSample, String> {
    check_id(&input.id)?;
    let contract: FeatureContract =
        strict_decode(registry.checked_artifact(feature_contract, requirements)?)?;
    if contract.id != feature_contract.id || contract.features.len() != 12 {
        return Err("observed_feature_contract_mismatch".into());
    }
    let label = registry.checked_measurement(&input.label, requirements)?;
    if label.origin == Origin::Synthetic
        || label.tier < EvidenceTier::CoupledScreen
        || label.quantity != "cout"
        || label.unit != Unit::MgPerLitreDispersedOil
        || label.identity.requirements != requirements
        || label.value.get() < 0.0
    {
        return Err("independent_cout_label_required".into());
    }
    let SubjectRef::Compiled { bundle } = &label.identity.subject else {
        return Err("compiled_training_subject_required".into());
    };
    let mut features = [0.0; 12];
    // Every slot is overwritten from a measured source; no partial vector is returned.
    for (i, (reference, spec)) in input.features.iter().zip(&contract.features).enumerate() {
        let measured = registry.checked_measurement(reference, requirements)?;
        if measured.identity != label.identity
            || measured.origin == Origin::Synthetic
            || matches!(
                measured.tier,
                EvidenceTier::SoftwareContract | EvidenceTier::AnalyticScreen
            )
            || measured.quantity != spec.quantity
            || measured.unit != spec.unit
            || spec.scale.get() <= 0.0
        {
            return Err("missing_prior_or_mismatched_observed_feature".into());
        }
        features[i] =
            Finite::new((measured.value.get() - spec.offset.get()) / spec.scale.get())?.get();
    }
    let identity_hash = label.identity.sha256()?;
    let groups = |reference: &ContentRef, kind: GroupKind| -> Result<Vec<String>, String> {
        let group: ObservedGroups =
            strict_decode(registry.checked_artifact(reference, requirements)?)?;
        if group.kind != kind
            || group.identity_sha256 != identity_hash
            || group.labels.is_empty()
            || group.observations.is_empty()
        {
            return Err("observed_group_membership_missing_or_stale".into());
        }
        crate::identity::unique_ids(group.labels.iter().map(String::as_str))?;
        registry.checked_artifact(&group.observer, requirements)?;
        for reference in &group.observations {
            let observed = registry.checked_measurement(reference, requirements)?;
            if observed.identity != label.identity
                || observed.origin == Origin::Synthetic
                || matches!(
                    observed.tier,
                    EvidenceTier::SoftwareContract | EvidenceTier::AnalyticScreen
                )
                || (kind == GroupKind::Mechanism && observed.tier < EvidenceTier::CoupledScreen)
            {
                return Err("group_not_bound_to_independent_observer".into());
            }
        }
        Ok(group.labels)
    };
    Ok(AdmittedSample {
        id: input.id.clone(),
        features,
        target: label.value,
        geometry_hash: bundle.geometry.sha256.clone(),
        label: input.label.clone(),
        family_groups: groups(&input.geometry_groups, GroupKind::GeometryFamily)?,
        mechanism_groups: groups(&input.mechanism_groups, GroupKind::Mechanism)?,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HoldoutFold {
    pub kind: GroupKind,
    pub group: String,
    pub train: Vec<usize>,
    pub test: Vec<usize>,
}

/// Same geometry across solver runs/scenarios cannot leak across either holdout boundary.
pub fn holdout_folds(samples: &[AdmittedSample]) -> Result<Vec<HoldoutFold>, String> {
    crate::identity::unique_ids(samples.iter().map(|s| s.id.as_str()))?;
    if samples.len() < 5 {
        return Err("insufficient_independent_label_inventory".into());
    }
    let mut folds = vec![];
    for kind in [GroupKind::GeometryFamily, GroupKind::Mechanism] {
        let names: BTreeSet<_> = samples
            .iter()
            .flat_map(|s| groups(s, kind))
            .cloned()
            .collect();
        if names.len() < 2 {
            return Err("independent_holdout_groups_insufficient".into());
        }
        for group in names {
            let test: Vec<_> = samples
                .iter()
                .enumerate()
                .filter(|(_, s)| groups(s, kind).contains(&group))
                .map(|(i, _)| i)
                .collect();
            let geometry: BTreeSet<_> = test.iter().map(|&i| &samples[i].geometry_hash).collect();
            let train: Vec<_> = samples
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    !geometry.contains(&s.geometry_hash) && !groups(s, kind).contains(&group)
                })
                .map(|(i, _)| i)
                .collect();
            if train.len() < 4 || test.is_empty() {
                return Err("insufficient_geometry_disjoint_holdout_support".into());
            }
            folds.push(HoldoutFold {
                kind,
                group,
                train,
                test,
            });
        }
    }
    Ok(folds)
}

fn groups(sample: &AdmittedSample, kind: GroupKind) -> &[String] {
    if kind == GroupKind::GeometryFamily {
        &sample.family_groups
    } else {
        &sample.mechanism_groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holdouts_exclude_repeated_geometry_and_missing_groups() {
        // Synthetic software fixture: private admission fields are not exposed to importers.
        let mut samples: Vec<_> = (0..12)
            .map(|i| AdmittedSample {
                id: format!("synthetic-{i}"),
                features: [i as f64; 12],
                target: Finite::new(i as f64).unwrap(),
                geometry_hash: format!("geometry-{i}"),
                family_groups: vec![format!("family-{}", i % 2)],
                mechanism_groups: vec![format!("observed-mechanism-{}", i % 3)],
                label: ContentRef::from_bytes(
                    &format!("label-{i}"),
                    "synthetic",
                    b"not-physical-data",
                ),
            })
            .collect();
        let folds = holdout_folds(&samples).unwrap();
        assert_eq!(folds.len(), 5);
        for fold in &folds {
            assert!(fold.train.iter().all(|i| !fold.test.contains(i)));
            assert!(fold.train.iter().all(|i| fold
                .test
                .iter()
                .all(|j| samples[*i].geometry_hash != samples[*j].geometry_hash)));
        }
        samples[1].geometry_hash = samples[0].geometry_hash.clone();
        for fold in holdout_folds(&samples).unwrap() {
            if fold.test.contains(&0) {
                assert!(!fold.train.contains(&1));
            }
        }
        for sample in &mut samples {
            sample.mechanism_groups = vec!["only-one-mechanism".into()];
        }
        assert!(holdout_folds(&samples).is_err());
        assert!(holdout_folds(&[]).is_err());
    }
}
