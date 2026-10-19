use super::*;

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
        if !min_coverage_lcb.is_finite()
            || !(0.0..=1.0).contains(&min_coverage_lcb)
            || !max_ood.is_finite()
            || max_ood < 0.0
            || !self.coverage_lcb_95.is_finite()
            || !(0.0..=1.0).contains(&self.coverage_lcb_95)
            || !self.max_ood_distance.is_finite()
            || self.max_ood_distance < 0.0
        {
            return Err("invalid_surrogate_policy_or_calibration");
        }
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
            || p.ood_distance < 0.0
            || p.ood_distance > max_ood
            || p.ood_distance > self.max_ood_distance
        {
            return Err("out_of_distribution_abstain");
        }
        if self.heldout_count < 20 || self.coverage_lcb_95 < min_coverage_lcb {
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

    #[test]
    fn v5_22_01_oracle_label_dataset_grouped_splits_and_sandbox_separation() {
        #[derive(Deserialize)]
        struct Fixture {
            rows: Vec<LabelCandidate>,
        }
        let fixture: Fixture = serde_json::from_str(include_str!(
            "../../../../experiments/datasets/v5-22-01-oracle-labels.json"
        ))
        .unwrap();
        let readiness = DatasetReadiness {
            ev09_ready: true,
            ev09_reasons: vec!["substitute fixture only".into()],
            ev12_complete: true,
            ev12_reasons: vec![],
        };
        let manifest =
            build_numerical_oracle_dataset_with_splits(&fixture.rows, &readiness).unwrap();
        assert_eq!(manifest.admitted.len(), 24);
        assert_eq!(manifest.geometry_family_folds.len(), 6);
        assert_eq!(manifest.lineage_folds.len(), 6);
        assert!(manifest.ev09_ready && manifest.ev12_complete);
        assert_eq!(manifest.sandbox_priors, vec!["sandbox-prior"]);
        assert_eq!(manifest.sandbox_pruned_inputs, vec!["sandbox-pruned"]);
        for fold in manifest
            .geometry_family_folds
            .iter()
            .chain(&manifest.lineage_folds)
        {
            for &test in &fold.test {
                for &train in &fold.train {
                    assert_ne!(
                        manifest.admitted[test].preparation_id,
                        manifest.admitted[train].preparation_id
                    );
                    assert!(manifest.admitted[test]
                        .geometry_family_groups
                        .iter()
                        .all(|g| !manifest.admitted[train].geometry_family_groups.contains(g)));
                    assert!(manifest.admitted[test]
                        .lineage_groups
                        .iter()
                        .all(|g| !manifest.admitted[train].lineage_groups.contains(g)));
                }
            }
        }
        for id in [
            "recipe-prior",
            "invalid-unit",
            "missing-observer",
            "missing-value",
            "synthetic-control",
        ] {
            assert!(manifest.excluded.iter().any(|(excluded, _)| excluded == id));
        }
        assert_eq!(
            manifest
                .excluded
                .iter()
                .find(|(id, _)| id == "recipe-prior")
                .unwrap()
                .1,
            ExclusionReason::PriorDerived
        );
        assert_eq!(
            manifest
                .excluded
                .iter()
                .find(|(id, _)| id == "invalid-unit")
                .unwrap()
                .1,
            ExclusionReason::OutOfScope
        );
        assert_eq!(
            manifest
                .excluded
                .iter()
                .find(|(id, _)| id == "missing-observer")
                .unwrap()
                .1,
            ExclusionReason::MissingObserver
        );
    }
}
