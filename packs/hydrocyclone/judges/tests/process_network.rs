//! Independent algebraic oracles on synthetic cohorts, never separator predictions.
use pack_hydrocyclone_judges::process_network::*;
use pack_hydrocyclone_judges::{Sha256Hex, Verdict};

fn diag(values: &[f64]) -> Vec<Vec<f64>> {
    values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            (0..values.len())
                .map(|j| if i == j { *v } else { 0.0 })
                .collect()
        })
        .collect()
}
fn route(id: &str, destination: Destination, liquid_fraction: f64, oil: &[f64]) -> Route {
    Route {
        id: id.into(),
        destination,
        liquid_fraction,
        oil_mass_transfer: diag(oil),
    }
}
fn stage(id: &str, routes: Vec<Route>) -> Stage {
    Stage {
        id: id.into(),
        response_source_sha256: Sha256Hex::of_bytes(b"synthetic kernel"),
        routes,
    }
}
fn to(id: &str) -> Destination {
    Destination::Stage {
        stage_id: id.into(),
    }
}
fn separator(id: &str, product: Destination, r: f64, capture: &[f64]) -> Stage {
    stage(
        id,
        vec![
            route(
                "water",
                product,
                1.0 - r,
                &capture.iter().map(|e| 1.0 - e).collect::<Vec<_>>(),
            ),
            route("oil", Destination::Reject, r, capture),
        ],
    )
}
fn request(stages: Vec<Stage>, mass: &[f64]) -> NetworkRequest {
    NetworkRequest {
        schema_version: 1,
        evidence_class: EvidenceClass::SyntheticFixture,
        diameter_edges_m: (1..=mass.len() + 1).map(|i| i as f64 * 10e-6).collect(),
        feeds: vec![Feed {
            stage_id: stages[0].id.clone(),
            cohort: Cohort {
                liquid_volume_m3: 0.001,
                oil_mass_kg: mass.to_vec(),
            },
        }],
        stages,
        budget: TransportBudget::default(),
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-10 * b.abs().max(1e-10), "{a} != {b}");
}

#[test]
fn single_stage_matches_the_500_mg_l_science_oracle() {
    let q = request(
        vec![separator("one", Destination::Product, 0.02, &[0.94])],
        &[0.0005],
    );
    let r = evaluate(&q).unwrap();
    near(
        r.product_concentration_mg_l.unwrap().upper,
        500.0 * 0.06 / 0.98,
    );
    near(r.external_reject_fraction.lower, 0.02);
    assert_eq!(r.termination, Termination::Complete);
    assert!(matches!(r.physical_verdict, Verdict::Unresolved(_)));
}

#[test]
fn two_stages_charge_cumulative_reject_and_treat_only_the_survivors() {
    let q = request(
        vec![
            separator("first", to("second"), 0.02, &[0.8]),
            separator("second", Destination::Product, 0.02, &[0.8]),
        ],
        &[0.0005],
    );
    let r = evaluate(&q).unwrap();
    near(r.external_reject_fraction.lower, 0.0396);
    near(r.product.liquid_volume_m3, 0.001 * 0.98 * 0.98);
    near(
        r.product_concentration_mg_l.unwrap().upper,
        500.0 * 0.2 * 0.2 / (0.98 * 0.98),
    );
    near(r.processed_by_stage["second"].oil_total_kg(), 0.0001);
}

#[test]
fn conditional_population_refutes_multiplying_average_efficiencies() {
    let q = request(
        vec![
            separator("a", to("b"), 0.02, &[0.1, 0.9]),
            separator("b", Destination::Product, 0.02, &[0.1, 0.9]),
        ],
        &[0.00025, 0.00025],
    );
    let r = evaluate(&q).unwrap();
    near(r.product.oil_total_kg() / 0.0005, 0.41);
    near(r.product.oil_mass_kg[0], 0.00025 * 0.9 * 0.9);
    near(r.product.oil_mass_kg[1], 0.00025 * 0.1 * 0.1);
    assert!((r.product.oil_total_kg() / 0.0005 - 0.25).abs() > 0.1);
}

#[test]
fn breakup_changes_the_downstream_response_and_preserves_oil_mass() {
    let mut transfer = route("all", to("capture"), 1.0, &[1.0, 1.0]);
    transfer.oil_mass_transfer = vec![vec![1.0, 0.0], vec![1.0, 0.0]];
    let q = request(
        vec![
            stage("breakup", vec![transfer]),
            separator("capture", Destination::Product, 0.02, &[0.0, 1.0]),
        ],
        &[0.0, 0.0005],
    );
    let r = evaluate(&q).unwrap();
    near(r.product.oil_mass_kg[0], 0.0005);
    near(r.reject.oil_total_kg(), 0.0);
    near(r.oil_balance_error_kg, 0.0);
}

#[test]
fn stage_order_matters_for_a_size_changing_response() {
    let mut q = request(
        vec![
            stage(
                "change",
                vec![route("all", to("capture"), 1.0, &[1.0, 1.0])],
            ),
            separator("capture", Destination::Product, 0.02, &[0.0, 1.0]),
        ],
        &[0.0, 0.0005],
    );
    q.stages[0].routes[0].oil_mass_transfer = vec![vec![1., 0.], vec![1., 0.]];
    let before = evaluate(&q).unwrap();
    q.feeds[0].stage_id = "capture".into();
    q.stages[1].routes[0].destination = to("change");
    q.stages[0].routes[0].destination = Destination::Product;
    let after = evaluate(&q).unwrap();
    near(before.product.oil_total_kg(), 0.0005);
    near(after.product.oil_total_kg(), 0.0);
}

#[test]
fn parallel_results_are_volume_weighted_not_an_average_of_concentrations() {
    let q = request(
        vec![
            stage(
                "split",
                vec![
                    route("a", to("a"), 0.25, &[0.25]),
                    route("b", to("b"), 0.75, &[0.75]),
                ],
            ),
            separator("a", Destination::Product, 0.02, &[0.9]),
            separator("b", Destination::Product, 0.02, &[0.1]),
        ],
        &[0.0005],
    );
    let r = evaluate(&q).unwrap();
    near(
        r.product_concentration_mg_l.unwrap().upper,
        500.0 * (0.25 * 0.1 + 0.75 * 0.9) / 0.98,
    );
    near(r.external_reject_fraction.lower, 0.02);
}

#[test]
fn rejoining_paths_mix_mass_without_resetting_the_dsd() {
    let q = request(
        vec![
            stage(
                "split",
                vec![
                    route("a", to("a"), 0.3, &[0.9, 0.1]),
                    route("b", to("b"), 0.7, &[0.1, 0.9]),
                ],
            ),
            stage("a", vec![route("join", to("join"), 1., &[1., 1.])]),
            stage("b", vec![route("join", to("join"), 1., &[1., 1.])]),
            separator("join", Destination::Product, 0.02, &[0.1, 0.9]),
        ],
        &[0.0001, 0.0004],
    );
    let r = evaluate(&q).unwrap();
    near(r.processed_by_stage["join"].oil_mass_kg[0], 0.0001);
    near(r.processed_by_stage["join"].oil_mass_kg[1], 0.0004);
    near(r.product.oil_total_kg(), 0.00013);
}

#[test]
fn passive_tracer_has_no_concentration_gain_even_with_recycle() {
    let q = request(
        vec![stage(
            "loop",
            vec![
                route("return", to("loop"), 0.5, &[0.5]),
                route("water", Destination::Product, 0.4, &[0.4]),
                route("reject", Destination::Reject, 0.1, &[0.1]),
            ],
        )],
        &[0.0005],
    );
    let r = evaluate(&q).unwrap();
    let bounds = r.product_concentration_mg_l.unwrap();
    assert!(bounds.lower <= 500.0 && bounds.upper >= 500.0);
    near(r.processed_by_stage["loop"].liquid_volume_m3, 0.002);
    assert!(r.product.liquid_volume_m3 < r.fresh_feed.liquid_volume_m3);
    assert_eq!(r.termination, Termination::TailBoundReached);
    assert!(r.circulating.liquid_volume_m3 > 0.0);
}

#[test]
fn oil_and_volume_tails_are_both_required_for_stopping() {
    let mut q = request(
        vec![stage(
            "loop",
            vec![
                route("return", to("loop"), 0.1, &[0.9]),
                route("water", Destination::Product, 0.9, &[0.1]),
            ],
        )],
        &[0.0005],
    );
    q.budget.relative_tail_tolerance = 0.01;
    let r = evaluate(&q).unwrap();
    assert!(r.steps > 40);
    assert!(r.circulating.oil_total_kg() / 0.0005 <= 0.01);
    let b = r.product_concentration_mg_l.unwrap();
    assert!(b.lower <= 500.0 && b.upper >= 500.0);
}

#[test]
fn a_closed_loop_exhausts_its_budget_and_keeps_the_entire_inventory() {
    let mut q = request(
        vec![stage("loop", vec![route("return", to("loop"), 1., &[1.])])],
        &[0.0005],
    );
    q.budget.max_steps = 4;
    let r = evaluate(&q).unwrap();
    assert_eq!(r.termination, Termination::WorkBudgetExhausted);
    near(r.circulating.oil_total_kg(), 0.0005);
    near(r.circulating.liquid_volume_m3, 0.001);
    assert!(r.product_concentration_mg_l.is_none());
    near(r.external_reject_fraction.upper, 1.0);
}

#[test]
fn wall_and_retained_oil_are_uncertain_not_captured() {
    let q = request(
        vec![stage(
            "one",
            vec![
                route("water", Destination::Product, 1., &[0.2]),
                route("wall", Destination::Wall, 0., &[0.3]),
                route("retained", Destination::Retained, 0., &[0.5]),
            ],
        )],
        &[0.0005],
    );
    let r = evaluate(&q).unwrap();
    near(r.reject.oil_total_kg(), 0.0);
    near(r.wall.oil_total_kg(), 0.00015);
    near(r.retained.oil_total_kg(), 0.00025);
    let b = r.product_concentration_mg_l.unwrap();
    near(b.lower, 100.0);
    near(b.upper, 500.0);
    assert!(matches!(r.physical_verdict, Verdict::Unresolved(_)));
}

#[test]
fn budget_stops_before_a_partial_step_and_does_not_drop_pending_mass() {
    let mut q = request(
        vec![separator("one", Destination::Product, 0.02, &[0.9])],
        &[0.0005],
    );
    q.budget.max_matrix_terms = 1;
    let r = evaluate(&q).unwrap();
    assert_eq!(r.termination, Termination::WorkBudgetExhausted);
    assert_eq!(r.stage_visits, 0);
    near(r.circulating.oil_total_kg(), 0.0005);
}

#[test]
fn multiple_feeds_and_scale_changes_preserve_weighted_concentration() {
    let mut q = request(
        vec![stage(
            "one",
            vec![route("all", Destination::Product, 1., &[1.])],
        )],
        &[0.0005],
    );
    q.feeds.push(Feed {
        stage_id: "one".into(),
        cohort: Cohort {
            liquid_volume_m3: 0.003,
            oil_mass_kg: vec![0.0003],
        },
    });
    for scale in [1e-6, 1., 1e6] {
        let mut scaled = q.clone();
        for f in &mut scaled.feeds {
            f.cohort.liquid_volume_m3 *= scale;
            f.cohort.oil_mass_kg[0] *= scale;
        }
        near(
            evaluate(&scaled)
                .unwrap()
                .product_concentration_mg_l
                .unwrap()
                .upper,
            200.0,
        );
    }
}

#[test]
fn stage_storage_order_does_not_change_the_answer() {
    let mut q = request(
        vec![
            separator("a", to("b"), 0.02, &[0.3]),
            separator("b", Destination::Product, 0.02, &[0.7]),
        ],
        &[0.0005],
    );
    let first = evaluate(&q).unwrap();
    q.stages.reverse();
    let second = evaluate(&q).unwrap();
    assert_eq!(first.product, second.product);
    assert_eq!(first.processed_by_stage, second.processed_by_stage);
    assert_ne!(first.request_sha256, second.request_sha256);
}

#[test]
fn invalid_kernels_topology_and_nonfinite_inputs_are_refused() {
    let base = request(
        vec![separator("one", Destination::Product, 0.02, &[0.9])],
        &[0.0005],
    );
    let mut changes = Vec::new();
    let mut q = base.clone();
    q.stages[0].routes[0].oil_mass_transfer[0][0] = 0.5;
    changes.push(q);
    let mut q = base.clone();
    q.stages[0].routes[0].liquid_fraction = f64::NAN;
    changes.push(q);
    let mut q = base.clone();
    q.stages[0].routes[0].destination = to("absent");
    changes.push(q);
    let mut q = base.clone();
    q.stages[0].routes[1].id = "water".into();
    changes.push(q);
    let mut q = base.clone();
    q.diameter_edges_m[1] = q.diameter_edges_m[0];
    changes.push(q);
    let mut q = base.clone();
    q.feeds[0].cohort.oil_mass_kg[0] = -1.;
    changes.push(q);
    let mut q = base.clone();
    q.stages[0].routes[0].oil_mass_transfer[0].push(0.);
    changes.push(q);
    let mut q = base.clone();
    q.stages[0].routes[0].liquid_fraction = 0.;
    q.stages[0].routes[1].liquid_fraction = 1.;
    changes.push(q);
    let mut q = base.clone();
    q.stages
        .push(separator("orphan", Destination::Product, 0.02, &[0.9]));
    changes.push(q);
    let mut q = base.clone();
    q.budget.relative_tail_tolerance = f64::INFINITY;
    changes.push(q);
    let mut q = base.clone();
    q.budget.max_stage_visits = 0;
    changes.push(q);
    let mut q = base;
    q.schema_version = 2;
    changes.push(q);
    for (i, q) in changes.into_iter().enumerate() {
        assert!(evaluate(&q).is_err(), "invalid case {i} was accepted");
    }
}

#[test]
fn strict_wire_format_cannot_forge_physical_admission_or_number_basis() {
    let q = request(
        vec![separator("one", Destination::Product, 0.02, &[0.9])],
        &[0.0005],
    );
    let mut wire = serde_json::to_value(&q).unwrap();
    wire["evidence_class"] = "qualified".into();
    assert!(serde_json::from_value::<NetworkRequest>(wire).is_err());
    let mut wire = serde_json::to_value(&q).unwrap();
    wire["feeds"][0]["cohort"]["number_weights"] = serde_json::json!([1.0]);
    assert!(serde_json::from_value::<NetworkRequest>(wire).is_err());
}

#[test]
fn roundoff_tolerance_cannot_accumulate_into_a_mass_balance_claim() {
    let mut q = request(
        vec![stage(
            "loop",
            vec![
                route("return", to("loop"), 1., &[1.]),
                route("leak", Destination::Product, 9e-13, &[0.]),
            ],
        )],
        &[0.0005],
    );
    q.budget.max_steps = 2000;
    q.budget.relative_tail_tolerance = 0.;
    assert!(q.validate().is_ok());
    assert!(evaluate(&q).is_err());
}

#[test]
fn committed_command_line_fixtures_keep_their_independent_oracles() {
    let serial: NetworkRequest = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/process-network/series.json"
    ))
    .unwrap();
    let r = evaluate(&serial).unwrap();
    near(r.external_reject_fraction.lower, 0.0396);
    near(
        r.product_concentration_mg_l.unwrap().upper,
        20.824656393169513,
    );
    let recycle: NetworkRequest = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/process-network/recycle.json"
    ))
    .unwrap();
    let r = evaluate(&recycle).unwrap();
    let b = r.product_concentration_mg_l.unwrap();
    assert!(b.lower <= 500.0 && b.upper >= 500.0);
}
