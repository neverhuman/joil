use crucible_particles::{forces::*, inject::*, inventory::*};

fn fluid(u: [f64; 3], a: [f64; 3], gp: [f64; 3]) -> FluidState {
    FluidState {
        velocity: u,
        material_acceleration: a,
        pressure_gradient: gp,
        gravity: [0.0, 0.0, -9.81],
        rho: 998.0,
        mu: 0.001,
    }
}
fn drop_state(u: [f64; 3], d: f64) -> DropState {
    DropState {
        velocity: u,
        diameter: d,
        rho: 850.0,
    }
}

#[test]
fn tracer_follows_uniform_accelerating_carrier_and_pressure_term_has_light_drop_sign() {
    let zero = acceleration(
        drop_state([1.0, 0.0, 0.0], 20e-6),
        fluid([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [-998.0, 0.0, 0.0]),
    )
    .unwrap();
    assert!(zero[0] > 0.0); // carrier pressure acceleration plus virtual mass, no drag at zero slip
    let tracer = DropState {
        rho: 998.0,
        ..drop_state([1.0, 0.0, 0.0], 20e-6)
    };
    let follow = acceleration(
        tracer,
        fluid([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [-998.0, 0.0, 0.0]),
    )
    .unwrap();
    assert!((follow[0] - 1.0).abs() < 1e-12);
    let tracer = drop_state([1.0, 0.0, 0.0], 20e-6);
    let a = acceleration(tracer, fluid([1.0, 0.0, 0.0], [0.0; 3], [1000.0, 0.0, 0.0])).unwrap();
    assert!(a[0] < 0.0); // -grad(p)/rho is negative
}

#[test]
fn terminal_rise_acceleration_and_midpoint_substep_refinement() {
    let f = fluid([0.0; 3], [0.0; 3], [0.0; 3]);
    let d = drop_state([0.0; 3], 50e-6);
    let a0 = acceleration(d, f).unwrap()[2];
    assert!(a0 > 0.0); // buoyant rise against gravity
    let end = |n| {
        advance_midpoint([0.0; 3], d, f, 0.0001, n)
            .unwrap()
            .1
            .velocity[2]
    };
    let coarse = end(2);
    let medium = end(4);
    let fine = end(8);
    assert!((fine - medium).abs() < (medium - coarse).abs());
    assert!(fine > 0.0);
    let d_m = 50e-6;
    let mut terminal: f64 = (998.0 - 850.0) * 9.81 * d_m * d_m / (18.0 * 0.001);
    for _ in 0..8 {
        let re = 998.0 * terminal * d_m / 0.001;
        terminal =
            (998.0 - 850.0) * 9.81 * d_m * d_m / (18.0 * 0.001 * (1.0 + 0.15 * re.powf(0.687)));
    }
    let term = acceleration(drop_state([0.0, 0.0, terminal], d_m), f).unwrap()[2];
    assert!(term.abs() < 1e-9); // Stokes terminal rise; Re_p is below unity
}

#[test]
fn stationary_rotation_has_no_added_centrifugal_term() {
    // Solid-body rotation pressure balances the carrier's centripetal material acceleration.
    let omega = 10.0;
    let radius = 0.02;
    let af = -omega * omega * radius;
    let f = fluid(
        [0.0, omega * radius, 0.0],
        [af, 0.0, 0.0],
        [998.0 * omega * omega * radius, 0.0, 0.0],
    );
    let tracer = DropState {
        rho: 998.0,
        ..drop_state(f.velocity, 20e-6)
    };
    let a = acceleration(tracer, f).unwrap();
    assert!((a[0] - af).abs() < 1e-12); // extra standalone centrifugal term breaks this
    assert!(a[0] < 0.0 && a[1].abs() < 1e-12);
}

#[test]
fn substep_order_does_not_apply_force_after_position_update() {
    let f = fluid([0.0; 3], [0.0; 3], [0.0; 3]);
    let d = drop_state([0.0, 0.0, 0.01], 20e-6);
    let (x, p) = advance_midpoint([0.0; 3], d, f, 0.001, 1).unwrap();
    let expected_midpoint = d.velocity[2] * 0.001 + (p.velocity[2] - d.velocity[2]) * 0.0005;
    assert!((x[2] - expected_midpoint).abs() < 1e-15); // position uses midpoint velocity from this substep
}

fn face(id: &str, normal: [f64; 3], vel: [f64; 3], area: f64, c: f64, dsd: Vec<f64>) -> InletFace {
    InletFace {
        semantic_id: id.into(),
        outward_normal: normal,
        area_m2: area,
        liquid_velocity_m_s: vel,
        oil_concentration_kg_m3: c,
        size_mass_fractions: dsd,
    }
}

#[test]
fn unequal_nonuniform_inlet_faces_integrate_mass_and_momentum_by_normal_flux() {
    let faces = vec![
        face(
            "branch-a",
            [-1.0, 0.0, 0.0],
            [2.0, 4.0, 0.0],
            0.5,
            0.01,
            vec![0.25, 0.75],
        ),
        face(
            "branch-b",
            [-1.0, 0.0, 0.0],
            [1.0, -2.0, 0.0],
            2.0,
            0.02,
            vec![1.0],
        ),
    ];
    let s = integrate_inlet(&faces, 2.0).unwrap();
    let mass: f64 = s.iter().map(|x| x.represented_oil_mass_kg).sum();
    assert!((mass - 0.10).abs() < 1e-12); // 0.5*2*2*.01 + 2*1*2*.02
    let px: f64 = s.iter().map(|x| x.momentum_kg_m_s[0]).sum();
    let py: f64 = s.iter().map(|x| x.momentum_kg_m_s[1]).sum();
    assert!((px - 0.12).abs() < 1e-12 && (py + 0.08).abs() < 1e-12);
    let wrong_speed_norm = faces
        .iter()
        .map(|f| {
            f.liquid_velocity_m_s
                .iter()
                .map(|v| v * v)
                .sum::<f64>()
                .sqrt()
                * f.area_m2
                * 2.0
                * f.oil_concentration_kg_m3
        })
        .sum::<f64>();
    assert!((wrong_speed_norm - mass).abs() > 1e-3);
}

#[test]
fn backflow_and_nonsemantic_masks_cannot_become_injection() {
    let f = face(
        "inlet",
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        1.0,
        0.1,
        vec![1.0],
    );
    assert!(reject_backflow_as_injection(&f).is_err());
    assert_eq!(
        integrate_inlet(&[f], 0.1).unwrap()[0].represented_oil_mass_kg,
        0.0
    );
}

fn parcel(id: u64, parent: Option<u64>, mass: f64, terminal: TerminalState) -> ParcelRecord {
    ParcelRecord {
        parcel_id: id,
        cohort_id: 1,
        injected_mass_kg: if parent.is_none() { mass } else { 0.0 },
        represented_mass_kg: mass,
        injection_size_bin: 0,
        initial_diameter_m: 20e-6,
        current_diameter_m: 18e-6,
        injection_stratum: "branch-a/bin-0".into(),
        injection_time_s: 0.0,
        parent_id: parent,
        daughter_ids: vec![],
        random_stream: 44,
        terminal,
        marker_diameter_px: 3.0,
    }
}

#[test]
fn restart_identity_daughters_and_first_pass_recross_are_checked() {
    let mut root = parcel(1, None, 0.8, TerminalState::Active);
    root.daughter_ids = vec![2];
    let mut i = Inventory {
        parcels: vec![root, parcel(2, Some(1), 0.2, TerminalState::Active)],
        unresolved_mass_kg: 0.0,
        synchronized_window_s: [1.0, 2.0],
    };
    i.validate_restart().unwrap();
    assert!(!i.parcels[1].is_independent_injection());
    i.mark_first_pass(1, TerminalState::RejectFirstPass)
        .unwrap();
    assert!(i
        .mark_first_pass(1, TerminalState::TreatedFirstPass)
        .is_err());
    let bytes = serde_json::to_vec(&i).unwrap();
    let restored: Inventory = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(i, restored);
    restored.validate_restart().unwrap();
}

#[test]
fn breakup_transfers_mass_and_keeps_one_injected_cohort() {
    let mut i = Inventory {
        parcels: vec![parcel(10, None, 1.0, TerminalState::Active)],
        unresolved_mass_kg: 0.0,
        synchronized_window_s: [0.0, 1.0],
    };
    let mut d1 = parcel(11, Some(10), 0.4, TerminalState::Active);
    let mut d2 = parcel(12, Some(10), 0.6, TerminalState::Active);
    d1.injected_mass_kg = 0.0;
    d2.injected_mass_kg = 0.0;
    assert!(i.split(10, vec![d1.clone(), d2.clone()]).is_ok());
    assert!(i.validate_restart().is_ok());
    assert_eq!(
        i.parcels.iter().map(|p| p.injected_mass_kg).sum::<f64>(),
        1.0
    );
    assert!(i.split(10, vec![d1, d2]).is_err());
}

#[test]
fn adverse_inventory_and_97_percent_retained_prevent_qualified_grade() {
    let i = Inventory {
        parcels: vec![
            parcel(1, None, 0.97, TerminalState::DomainRetained),
            parcel(2, None, 0.03, TerminalState::TreatedFirstPass),
        ],
        unresolved_mass_kg: 0.0,
        synchronized_window_s: [0.0, 10.0],
    };
    let c = i.closure(1.0, 1.0, 0.0).unwrap();
    assert!(c.concentration_upper_mg_l >= 1_000_000.0);
    assert!(!grade_qualified(true, &c, 30.0));
    let by_size = i.closure_by_size(&[1.0], &[1.0], &[0.0], &[0.0]).unwrap();
    assert_eq!(by_size[0], c);
    let plan = plan_sampling(2.0, 100.0, 4.0, true, 32).unwrap();
    assert_eq!(plan.duration_s, 12.0);
    assert_eq!(plan.independent_replicates, 3);
    assert!(plan.parcel_budget > 32);
    assert!(plan_sampling(2.0, 100.0, 0.01, false, 32).is_err()); // fixed 0.01 s without complete residence evidence cannot pass
}

#[test]
fn unresolved_mass_is_reconciled_within_each_injection_size_class() {
    let mut p0 = parcel(1, None, 0.7, TerminalState::RejectFirstPass);
    let mut p1 = parcel(2, None, 0.3, TerminalState::DomainRetained);
    p1.injection_size_bin = 1;
    let i = Inventory {
        parcels: vec![p0.clone(), p1.clone()],
        unresolved_mass_kg: 0.1,
        synchronized_window_s: [2.0, 3.0],
    };
    assert!(i
        .closure_by_size(&[0.7, 0.3], &[1.0, 1.0], &[0.0, 0.1], &[0.0, 0.0])
        .is_ok());
    p0.represented_mass_kg = 0.0;
    assert!(i
        .closure_by_size(&[0.7, 0.3], &[1.0, 1.0], &[0.0, 0.0], &[0.0, 0.0])
        .is_err());
}
