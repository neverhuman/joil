use super::*;

#[test]
fn v5_25_02_full_liner_corpus_family_separation_and_head_reuse_guard() {
    let raw = include_str!("../../../../../experiments/datasets/v5-25-02-full-liner-corpus.json");
    let report = verify_full_liner_corpus_json(raw).unwrap();
    assert_eq!(report.held_out_cases, 2);
    assert_eq!(report.synthetic_cases, 12);
    assert!(report.held_out_tags.contains(&"oil_grade".to_string()));
    assert_eq!(report.declared_gaps, vec!["wet_liner_surface_chemistry"]);
    let lease = Field::Admitted {
        value: MeasuredLease {
            cpu_cores: 2,
            memory_gib: 6.0,
            output_gib: 1.0,
            gpu: false,
            lease_sha256: Sha256Hex::of_bytes(b"synthetic measured lease"),
        },
        source: Sha256Hex::of_bytes(b"synthetic lease source"),
    };
    let (_, built_report) = build_preregistered_full_liner_corpus(
        lease,
        vec![DeclaredGap {
            tag: "wet_liner_surface_chemistry".into(),
            reason: "physical wet-liner measurements are governed by V5-30".into(),
        }],
    )
    .unwrap();
    assert_eq!(
        built_report.declared_gaps,
        vec!["wet_liner_surface_chemistry"]
    );
    let mut pre: Preregistration = serde_json::from_str(raw).unwrap();
    let held = pre
        .cases
        .iter()
        .position(|c| c.split == Split::HeldOut)
        .unwrap();
    pre.cases[held].geometry_family = pre.cases[0].geometry_family.clone();
    assert!(verify_full_liner_corpus_json(&serde_json::to_string(&pre).unwrap()).is_err());
    pre.cases[held].geometry_family = "helical_liner_d".into();
    pre.cases[held].geometry_sha256 = pre.cases[0].geometry_sha256.clone();
    assert!(verify_full_liner_corpus_json(&serde_json::to_string(&pre).unwrap()).is_err());
}

#[test]
fn v5_25_03_coupled_head_tail_reference_agreement_and_unclosed_fallback() {
    let reference = Observables {
        reject_split: 0.08,
        treated_oil_mg_l: 24.0,
        pressure_drop_pa: 42_000.0,
    };
    let coupler = ReferenceHeadTailCoupler {
        oil_partition: vec![0.65, 0.35],
        reverse_core_target_kg_s: 0.1,
        feedback_fraction: 0.3,
        observables: reference.clone(),
        omit_reverse_core_in_sent: false,
        oil_closure_scale: 1.0,
    };
    let cfg = CouplingConfig {
        max_exchanges: 20,
        relaxation: 1.0,
        residual_tol: 0.01,
        closure_tol: 1e-9,
    };
    let budget = Field::Admitted {
        value: DecisionBudget {
            reject_split_rel: 0.01,
            treated_oil_rel: 0.01,
            pressure_rel: 0.01,
        },
        source: Sha256Hex::of_bytes(b"synthetic decision budget"),
    };
    let (outcome, verdict) = evaluate_coupled_head_tail_against_reference(
        &coupler,
        Interface {
            oil_kg_s: vec![0.8, 0.2],
            reverse_core_kg_s: 0.2,
        },
        &cfg,
        &reference,
        &budget,
    )
    .unwrap();
    assert!(matches!(&outcome, CouplingOutcome::Converged { exchanges, .. } if *exchanges >= 4));
    assert_eq!(verdict, Verdict::Pass);
    let mut omitted = coupler.clone();
    omitted.omit_reverse_core_in_sent = true;
    let (fallback, verdict) = evaluate_coupled_head_tail_against_reference(
        &omitted,
        Interface {
            oil_kg_s: vec![0.8, 0.2],
            reverse_core_kg_s: 0.2,
        },
        &cfg,
        &reference,
        &budget,
    )
    .unwrap();
    assert!(matches!(
        fallback,
        CouplingOutcome::UnclosedInterface { .. }
    ));
    assert!(matches!(verdict, Verdict::Unresolved(_)));
    let mut unclosed_oil = coupler.clone();
    unclosed_oil.oil_closure_scale = 0.9;
    let (fallback, _) = evaluate_coupled_head_tail_against_reference(
        &unclosed_oil,
        Interface {
            oil_kg_s: vec![0.8, 0.2],
            reverse_core_kg_s: 0.2,
        },
        &cfg,
        &reference,
        &budget,
    )
    .unwrap();
    assert!(matches!(
        fallback,
        CouplingOutcome::UnclosedInterface { .. }
    ));
    let too_short = CouplingConfig {
        max_exchanges: 3,
        ..cfg
    };
    assert!(matches!(
        couple(
            &omitted,
            &omitted,
            Interface {
                oil_kg_s: vec![0.8, 0.2],
                reverse_core_kg_s: 0.2
            },
            &too_short
        ),
        CouplingOutcome::NumericalFailure { .. }
    ));
}
