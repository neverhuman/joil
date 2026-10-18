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
    let mut f = fluid([0.0; 3], [0.0; 3], [0.0; 3]);
    f.pressure_gradient = f.gravity.map(|g| f.rho * g);
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

#[test]
fn full_static_pressure_keeps_a_hydrostatic_neutral_tracer_stationary() {
    let mut f = fluid([0.0; 3], [0.0; 3], [0.0; 3]);
    f.pressure_gradient = f.gravity.map(|g| f.rho * g);
    let tracer = DropState {
        rho: f.rho,
        ..drop_state([0.0; 3], 20e-6)
    };
    let a = acceleration(tracer, f).unwrap();
    assert!(
        a.iter().all(|x| x.abs() < 1e-12),
        "hydrostatic tracer drift: {a:?}"
    );
}

#[test]
fn full_static_pressure_matches_independent_displaced_fluid_balance() {
    let f = FluidState {
        rho: 1000.0,
        gravity: [0.0, 0.0, -9.81],
        pressure_gradient: [0.0, 0.0, -9810.0],
        ..fluid([0.0; 3], [0.0; 3], [0.0; 3])
    };
    // At zero slip the displaced-fluid force is (rho_p-rho_f)*V*g.
    // The effective inertial masses are 1000*V and 2000*V respectively.
    for (rho, expected) in [(500.0, 4.905), (1500.0, -2.4525)] {
        let p = DropState {
            rho,
            ..drop_state([0.0; 3], 20e-6)
        };
        let a = acceleration(p, f).unwrap();
        assert!((a[2] - expected).abs() < 1e-12, "rho={rho}: {a:?}");
    }
}

#[test]
fn full_static_pressure_tracer_follows_all_carrier_acceleration_components() {
    let desired = [1.0, 2.0, 3.0];
    let mut f = fluid([0.0; 3], desired, [0.0; 3]);
    // Inviscid carrier momentum: grad(p) = rho_f*(g-DU/Dt).
    f.pressure_gradient = std::array::from_fn(|i| f.rho * (f.gravity[i] - desired[i]));
    let tracer = DropState {
        rho: f.rho,
        ..drop_state([0.0; 3], 20e-6)
    };
    let a = acceleration(tracer, f).unwrap();
    for i in 0..3 {
        assert!((a[i] - desired[i]).abs() < 1e-12, "{a:?}");
    }
}

#[test]
fn unrepresentable_force_arithmetic_is_refused() {
    let p = drop_state([0.0; 3], 1e-300);
    assert!(acceleration(p, fluid([0.0; 3], [0.0; 3], [0.0; 3])).is_err());
}

#[test]
fn unrepresentable_sphere_mass_arithmetic_is_refused() {
    for diameter in [1e-300, 1e300] {
        assert!(spherical_mass(diameter, 1000.0).is_err());
    }
    assert!((spherical_mass(1.0, 1.0).unwrap() - std::f64::consts::PI / 6.0).abs() < 1e-15);
}

fn guard_inventory() -> Inventory {
    Inventory {
        parcels: vec![parcel(1, None, 1.0, TerminalState::Active)],
        unresolved_mass_kg: 0.0,
        synchronized_window_s: [0.0, 1.0],
    }
}

#[test]
fn inventory_guard_closure_rejects_invalid_records_and_windows() {
    for window in [[f64::NAN, 1.0], [0.0, f64::INFINITY]] {
        let mut bad = guard_inventory();
        bad.synchronized_window_s = window;
        assert!(bad.closure(1.0, 1.0, 0.0).is_err());
    }
    let mut bad = guard_inventory();
    bad.parcels[0].represented_mass_kg = -1.0;
    assert!(bad.closure(1.0, 1.0, 0.0).is_err());
    assert!(bad.closure_by_size(&[1.0], &[1.0], &[0.0], &[0.0]).is_err());
    assert!(guard_inventory().closure(0.5, 1.0, 0.0).is_err());
}

#[test]
fn inventory_guard_duplicate_daughters_are_rejected_atomically() {
    let mut inventory = guard_inventory();
    let before = inventory.clone();
    let d = parcel(2, Some(1), 0.5, TerminalState::Active);
    assert!(inventory.split(1, vec![d.clone(), d]).is_err());
    assert_eq!(inventory, before);
}

#[test]
fn inventory_guard_tiny_parcel_cannot_lose_all_mass_within_absolute_tolerance() {
    let mut inventory = guard_inventory();
    inventory.parcels[0].represented_mass_kg = 1e-18;
    inventory.parcels[0].injected_mass_kg = 1e-18;
    let before = inventory.clone();
    assert!(inventory
        .split(1, vec![parcel(2, Some(1), 0.0, TerminalState::Active)])
        .is_err());
    assert_eq!(inventory, before);
    inventory
        .split(
            1,
            vec![
                parcel(2, Some(1), 4e-19, TerminalState::Active),
                parcel(3, Some(1), 6e-19, TerminalState::Active),
            ],
        )
        .unwrap();
    inventory.validate_restart().unwrap();
    assert!((inventory.closure(1e-18, 1.0, 0.0).unwrap().domain_kg - 1e-18).abs() < 1e-32);
}

#[test]
fn inventory_guard_grade_and_sampling_refuse_unrepresentable_values() {
    let mut closure = guard_inventory().closure(1.0, 1.0, 0.0).unwrap();
    closure.concentration_lower_mg_l = -2.0;
    closure.concentration_upper_mg_l = -1.0;
    assert!(!grade_qualified(true, &closure, 30.0));
    assert!(plan_sampling(1e-300, 1e300, 1.0, true, 32).is_err());
    assert!(plan_sampling(1.0, 1.0, f64::MAX, true, 32).is_err());
}

#[test]
fn inventory_guard_size_closure_and_display_only_markers_preserve_valid_accounting() {
    let mut inventory = guard_inventory();
    inventory.parcels[0].terminal = TerminalState::RejectFirstPass;
    let whole = inventory.closure(1.0, 1.0, 0.0).unwrap();
    let per_size = inventory
        .closure_by_size(&[1.0], &[1.0], &[0.0], &[0.0])
        .unwrap();
    assert_eq!(per_size, [whole.clone()]);
    assert!(grade_qualified(true, &whole, 30.0));
    inventory.parcels[0].marker_diameter_px = 0.0;
    assert_eq!(inventory.closure(1.0, 1.0, 0.0).unwrap(), whole);
}

#[test]
fn restart_guard_rejects_cyclic_lineage() {
    let mut a = parcel(1, Some(2), 0.5, TerminalState::Active);
    let mut b = parcel(2, Some(1), 0.5, TerminalState::Active);
    a.daughter_ids = vec![2];
    b.daughter_ids = vec![1];
    let inventory = Inventory {
        parcels: vec![a, b],
        ..guard_inventory()
    };
    assert!(inventory.validate_restart().is_err());
}

#[test]
fn restart_guard_rejects_changed_immutable_injection_size_class() {
    let mut inventory = guard_inventory();
    inventory
        .split(
            1,
            vec![
                parcel(2, Some(1), 0.4, TerminalState::Active),
                parcel(3, Some(1), 0.6, TerminalState::Active),
            ],
        )
        .unwrap();
    inventory.parcels[1].injection_size_bin = 7;
    assert!(inventory.validate_restart().is_err());
}

#[test]
fn derived_injection_mass_and_momentum_must_be_representable() {
    let huge = face(
        "synthetic-overflow",
        [-1.0, 0.0, 0.0],
        [1e308, 0.0, 0.0],
        10.0,
        1.0,
        vec![1.0],
    );
    assert!(integrate_inlet(&[huge], 1.0).is_err());
}

#[test]
fn inventory_guard_size_classes_cannot_drop_tiny_unresolved_mass() {
    let mut inventory = guard_inventory();
    inventory.unresolved_mass_kg = 1e-18;
    assert!(inventory
        .closure_by_size(&[1.0], &[1.0], &[0.0], &[0.0])
        .is_err());
    assert!(inventory
        .closure_by_size(&[1.0], &[1.0], &[1e-18], &[0.0])
        .is_ok());
}
