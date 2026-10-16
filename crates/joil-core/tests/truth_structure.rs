use joil_core::{
    evolve::{constrained_dominates, evaluate_individual, EvolutionConfig},
    program::DesignProgram,
    structure::{
        audit_load_transfer, four_grid_gci, three_grid_order, GciState, NodalReaction,
        SurfaceTraction,
    },
};

#[test]
fn truth_gci_order() {
    for p in [1.5, 2.0, 3.2] {
        let values = [1.0_f64, 0.5, 0.25, 0.125].map(|h| 7.0 + 3.0 * h.powf(p));
        let diagnostic = three_grid_order([values[1], values[2], values[3]], 2.0).unwrap();
        assert_eq!(diagnostic.state, GciState::Monotone);
        assert!(diagnostic.gci_fine_rel.is_none());
        let report = four_grid_gci(values, 2.0, 0.01).unwrap();
        assert_eq!(report.state, GciState::Asymptotic);
        assert!((report.observed_order.unwrap() - p).abs() < 0.01);
        assert!(report.gci_fine_rel.unwrap() > 0.0);
    }
    for (values, state) in [
        ([1.0, 0.9, 0.95], GciState::Oscillatory),
        ([1.0, 2.0, 4.0], GciState::Divergent),
        ([1.0, 1.0, 1.0 + f64::EPSILON], GciState::Indeterminate),
    ] {
        let report = three_grid_order(values, 2.0).unwrap();
        assert_eq!(report.state, state);
        assert!(report.gci_fine_rel.is_none());
    }
    assert!(three_grid_order([1.0, f64::NAN, 2.0], 2.0).is_err());
    assert!(three_grid_order([1.0, 2.0, 3.0], 1.0).is_err());
    assert!(four_grid_gci([8.0, 4.0, 2.0, 1.5], 2.0, 0.01)
        .unwrap()
        .gci_fine_rel
        .is_none());
}

#[test]
fn truth_load_transfer_independent_reactions() {
    // Synthetic pressure/shear patch: force=(-10, 2, 0) N at y=2 m;
    // independently supplied equilibrium reaction=(10,-2,0) N at the same point.
    let patch = SurfaceTraction {
        position_m: [0.0, 2.0, 0.0],
        normal: [1.0, 0.0, 0.0],
        area_m2: 0.5,
        pressure_pa: 20.0,
        viscous_traction_pa: [0.0, 4.0, 0.0],
    };
    let mut reaction = NodalReaction {
        position_m: [0.0, 2.0, 0.0],
        force_n: [10.0, -2.0, 0.0],
    };
    let check = |reaction| {
        audit_load_transfer(
            std::slice::from_ref(&patch),
            &[reaction],
            [0.0; 3],
            1e-8,
            1e-10,
            1e-10,
        )
        .unwrap()
    };
    let pass = check(reaction.clone());
    assert_eq!(pass.source_force_n, [-10.0, 2.0, 0.0]);
    assert_eq!(pass.source_moment_n_m, [0.0, 0.0, 20.0]);
    assert!(pass.passed);
    reaction.force_n[0] *= 1.01;
    assert!(!check(reaction.clone()).passed);
    reaction.force_n[0] = 10.0;
    reaction.position_m[1] = 2.01;
    let moment_failure = check(reaction);
    assert_eq!(moment_failure.force_residual_n, 0.0);
    assert!(!moment_failure.passed);
    assert!(audit_load_transfer(&[], &[], [0.0; 3], 0.01, 0.0, 0.0).is_err());
}

#[test]
fn truth_candidate_structure_constrains_dominance_and_identity() {
    let cfg = EvolutionConfig {
        mc_samples: 4,
        ..EvolutionConfig::default()
    };
    let (mut feasible, _) = evaluate_individual(
        "synthetic-control".into(),
        DesignProgram::baseline_ct35(),
        0,
        0,
        &cfg,
    )
    .unwrap();
    // Isolate structural constrained-dominance behavior from hydraulic feasibility.
    feasible.constraint_violation = 0.0;
    feasible.objectives = [100.0; 5];
    let mut failed = feasible.clone();
    failed.objectives = [0.0; 5];
    failed.structural_qualification.as_mut().unwrap().passed = false;
    assert!(!constrained_dominates(&failed, &feasible));
    assert!(constrained_dominates(&feasible, &failed));
    let mut stale = feasible.clone();
    stale.nominal_metrics.dp_u_pa *= 2.0;
    stale.objectives = [0.0; 5];
    assert!(!constrained_dominates(&stale, &feasible));
    let mut unknown = feasible.clone();
    unknown.structural_qualification = None;
    assert!(!constrained_dominates(&unknown, &feasible));

    let mut split = DesignProgram::baseline_ct35();
    split.inlet_split.enabled = true;
    split.inlet_split.branches_per_port = 2;
    let mut high_flow = cfg;
    high_flow.baseline_hydraulics.q_in_m3_s = 0.1;
    let (loaded, _) =
        evaluate_individual("synthetic-overload".into(), split, 0, 0, &high_flow).unwrap();
    let screen = loaded.structural_qualification.as_ref().unwrap();
    assert!(
        !screen.passed,
        "overload fixture must fail a candidate-specific bound"
    );
    assert!(loaded.constraint_violation >= 1.0 + screen.failure_reasons.len() as f64);
    assert!(screen.load_transfer_audit.is_none());
}
