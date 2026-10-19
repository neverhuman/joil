use super::*;

/// Evaluate a stream of raw candidate programs while maintaining an exact `BudgetLedger`
/// and phenotype deduplication set (`ENGINE-03`).
pub fn evaluate_batch_with_ledger(
    programs: &[DesignProgram],
    inject_infra_failures: usize,
) -> Result<(BudgetLedger, BTreeMap<String, usize>), String> {
    let mut ledger = BudgetLedger::default();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut family_counts: BTreeMap<String, usize> = BTreeMap::new();

    for prog in programs {
        ledger.total_attempts += 1;
        let (pheno, mesh) = match Phenotype3D::from_program(prog) {
            Ok(v) => v,
            Err(_) => {
                ledger.redteam_rejected += 1;
                continue;
            }
        };
        if redteam::check(prog, &pheno, &mesh).is_err() {
            ledger.redteam_rejected += 1;
            continue;
        }
        let st = structure::analytic_structural_screen(prog, &pheno, 29468.22, 0.001111111)?;
        if !st.passed {
            ledger.structural_rejected += 1;
            continue;
        }
        if !seen.insert(pheno.design_sha256.clone()) {
            ledger.phenotype_cache_hits += 1;
            continue;
        }
        ledger.unique_f0_evaluated += 1;
        ledger.total_mc_realizations += 16;
        *family_counts.entry(pheno.topology_class).or_insert(0) += 1;
    }

    for _ in 0..inject_infra_failures {
        ledger.total_attempts += 1;
        ledger.infrastructure_quarantined += 1;
    }

    Ok((ledger, family_counts))
}

#[cfg(test)]
mod evidence_tests {
    use super::*;
    use crate::evolve::{evaluate_individual, EvolutionConfig};

    pub(crate) fn prior_records() -> Vec<SurrogateRecord> {
        let program = DesignProgram::baseline_ct35();
        let config = EvolutionConfig {
            mc_samples: 6,
            ..EvolutionConfig::default()
        };
        let (candidate, _) =
            evaluate_individual("synthetic-software-fixture".into(), program, 0, 0, &config)
                .unwrap();
        (0..6)
            .map(|i| {
                let mut record = SurrogateRecord::from_candidate(&candidate);
                record.candidate_id = format!("synthetic-{i}");
                record.target_cout_mg_l = Some(20.0 + i as f64);
                record
            })
            .collect()
    }

    #[test]
    pub(crate) fn truth_surrogate_extraction_abstains_without_baseline_features() {
        let mut invalid = DesignProgram::baseline_ct35();
        invalid.schema_version = 999;
        assert!(matches!(
            extract_program_features(&invalid),
            Err(SurrogateAbstention::ExtractionFailure { .. })
        ));
        let config = EvolutionConfig {
            mc_samples: 6,
            ..EvolutionConfig::default()
        };
        let (mut candidate, _) = evaluate_individual(
            "fixture".into(),
            DesignProgram::baseline_ct35(),
            0,
            0,
            &config,
        )
        .unwrap();
        candidate.phenotype = None;
        let record = SurrogateRecord::from_candidate(&candidate);
        assert_eq!(record.status, SurrogateRecordStatus::MissingFeatures);
        assert!(record.features.is_none());
        assert!(record.feature_failure.is_some());
    }

    #[test]
    pub(crate) fn truth_surrogate_ood_and_missingness_route_to_direct_evaluation() {
        let records = prior_records();
        assert!(GaussianProcessSurrogate::fit(&records, 0.85, 0.015).is_err());
        let gp = GaussianProcessSurrogate::fit_sandbox_prior(&records, 0.85, 0.015).unwrap();
        assert!(matches!(
            gp.predict_in_domain(&[1e6; 12]),
            Err(SurrogateAbstention::OutOfDistribution { .. })
        ));
        assert!(gp.predict_in_domain(&[f64::NAN; 12]).is_err());
        let mut invalid = DesignProgram::baseline_ct35();
        invalid.schema_version = 999;
        let routed = route_offspring(&gp, &[DesignProgram::baseline_ct35(), invalid], 25.0, 1);
        assert_eq!(routed.ranked.len(), 1);
        assert_eq!(routed.direct_evaluation.len(), 1);
    }

    #[test]
    pub(crate) fn truth_surrogate_failure_model_is_separate_from_performance() {
        let mut records = prior_records();
        let mut failed = records[0].clone();
        failed.status = SurrogateRecordStatus::NumericalFailure;
        failed.features = None;
        failed.target_cout_mg_l = None;
        records.push(failed);
        let gp = GaussianProcessSurrogate::fit_sandbox_prior(&records, 0.85, 0.015).unwrap();
        assert_eq!(gp.quarantined_failures, 1);
        assert_eq!(gp.failure_model.numerical_failures, 1);
        assert_eq!(gp.failure_model.complete_records, 6);
        assert_eq!(gp.failure_model.attempts, 7);
        records[0].features = None;
        assert!(GaussianProcessSurrogate::fit_sandbox_prior(&records, 0.85, 0.015).is_err());
    }

    #[test]
    pub(crate) fn truth_surrogate_submitted_status_cannot_create_independent_authority() {
        let mut records = prior_records();
        for record in &mut records {
            record.status = SurrogateRecordStatus::IndependentObservation;
        }
        let bytes = serde_json::to_vec(&records).unwrap();
        let submitted: Vec<SurrogateRecord> = serde_json::from_slice(&bytes).unwrap();
        assert!(GaussianProcessSurrogate::fit(&submitted, 0.85, 0.015).is_err());
        assert!(GaussianProcessSurrogate::fit_sandbox_prior(&submitted, 0.85, 0.015).is_err());
    }

    #[test]
    pub(crate) fn truth_surrogate_absent_holdouts_do_not_become_zero_error() {
        let config = EvolutionConfig {
            mc_samples: 6,
            ..EvolutionConfig::default()
        };
        let (candidate, _) = evaluate_individual(
            "software-fixture".into(),
            DesignProgram::baseline_ct35(),
            0,
            0,
            &config,
        )
        .unwrap();
        let error = run_surrogate_and_engine_qualification(&vec![candidate; 6]).unwrap_err();
        assert!(error.contains("insufficient authored-family folds"));
    }

    #[test]
    pub(crate) fn truth_surrogate_fold_rows_reproduce_legacy_metrics_and_preserve_abstention() {
        let config = EvolutionConfig {
            mc_samples: 72,
            ..EvolutionConfig::default()
        };
        let (base, _) = evaluate_individual(
            "synthetic-fold-fixture".into(),
            DesignProgram::baseline_ct35(),
            0,
            0,
            &config,
        )
        .unwrap();
        // Authored synthetic groups exercise capture only, never independent labels.
        let mut candidates: Vec<_> = (0..12)
            .map(|i| {
                let mut c = base.clone();
                c.id = format!("synthetic-{i}");
                c.topology_class = format!("synthetic-family-{}", i / 4);
                c.mc_robustness.cout_p95_mg_l += i as f64 * 0.1;
                c
            })
            .collect();
        let legacy = run_surrogate_and_engine_qualification(&candidates).unwrap();
        let (report, rows) =
            run_surrogate_and_engine_qualification_with_fold_rows(&candidates).unwrap();
        assert_eq!(
            serde_json::to_value(legacy).unwrap(),
            serde_json::to_value(&report).unwrap()
        );
        assert_eq!(rows.len(), 3);
        for fold in rows {
            let metrics = report
                .family_holdouts
                .iter()
                .find(|f| f.held_out_family == fold.topology_class)
                .unwrap();
            assert_eq!(fold.train_subject_ids.len(), metrics.train_count);
            assert_eq!(fold.predictions.len(), metrics.test_count);
            let mut sq_err = 0.0;
            let mut covered = 0;
            for row in &fold.predictions {
                assert!(!fold.train_subject_ids.contains(&row.subject_id));
                assert!(!row.subject_id.starts_with("quarantine-test"));
                sq_err += (row.predicted_cout - row.target_cout).powi(2);
                covered += usize::from(
                    row.target_cout >= row.lcb_2sigma && row.target_cout <= row.ucb_2sigma,
                );
                assert_eq!(row.lcb_2sigma, row.predicted_cout - 2.0 * row.sigma_cout);
                assert_eq!(row.ucb_2sigma, row.predicted_cout + 2.0 * row.sigma_cout);
            }
            let n = fold.predictions.len() as f64;
            assert!(((sq_err / n).sqrt() - metrics.rmse_cout_mg_l).abs() < 1e-12);
            assert_eq!(covered as f64 / n, metrics.coverage_within_2sigma);
        }
        for candidate in &mut candidates {
            candidate.phenotype = None;
        }
        assert!(run_surrogate_and_engine_qualification_with_fold_rows(&candidates).is_err());
    }
}
