use super::*;

// ---------------------------------------------------------------------------
// V5-02 audit follow-up: the executable 1.4 mm wrong-context negative and the nine-input by
// claim-scope matrix, through `DecisionProfile::qualified_admission`. Every source below is
// SYNTHETIC and approved only inside the test registry; none is a physical limit or measurement.

const CLAIM_SCOPES: [&str; 4] = [
    "computational_comparator",
    "physical_candidate",
    "commercial_claim",
    "protected_export",
];
const COMPUTATIONAL: &[&str] = &[
    "computational_comparator",
    "physical_candidate",
    "commercial_claim",
];
/// Independently stated expectation: external input, accountable owner, claims it must block.
const EXTERNAL_EXPECTATIONS: [(&str, &str, &[&str]); 9] = [
    (
        "installation_interfaces",
        "installation_engineer",
        COMPUTATIONAL,
    ),
    ("system_pressure_and_loads", "process_owner", COMPUTATIONAL),
    ("fluid_and_dsd", "fluid_measurement_lead", COMPUTATIONAL),
    (
        "manufacturing_passport",
        "manufacturing_lead",
        COMPUTATIONAL,
    ),
    ("material_allowables", "materials_engineer", COMPUTATIONAL),
    ("independent_validation", "validation_lead", COMPUTATIONAL),
    (
        "laboratory_access",
        "experimental_lead",
        &["physical_candidate", "commercial_claim"],
    ),
    (
        "contemporary_comparator",
        "project_and_experimental_owners",
        &["commercial_claim"],
    ),
    (
        "license_and_disclosure",
        "project_owner",
        &["protected_export"],
    ),
];

/// SYNTHETIC values that only complete the policy so one change can be isolated.
fn synthetic_limit_value(id: &str) -> (f64, f64) {
    match id {
        "pressure_budget_pa" => (1.0e5, 1.0e5),
        "energy_budget_j_per_treated_m3" => (500.0, 500.0),
        "absolute_pressure_min_pa" => (2.0e5, 2.0e5),
        "continuous_duty_band_m3_h" => (2.6, 5.4),
        "external_demand_pa" => (0.0, 2.5e6),
        "quality_limit_mg_l" => (30.0, 30.0),
        "split_fraction" => (0.0198, 0.0202),
        "printed_min_passage_mm" => (3.0, 3.0),
        "machined_overflow_nominal_mm" => (1.4, 1.4),
        other => panic!("unexpected draft limit {other}; extend the synthetic table"),
    }
}

type Pending = Vec<(ContentRef, Vec<u8>)>;

fn pending_artifact<T: Serialize>(pending: &mut Pending, id: &str, value: &T) -> ContentRef {
    let bytes = serde_json::to_vec(value).unwrap();
    let reference = ContentRef::from_bytes(id, "1", &bytes);
    pending.push((reference.clone(), bytes));
    reference
}

fn toml_of<T: Serialize>(value: &T) -> toml::Value {
    toml::Value::try_from(value).unwrap()
}

fn entry_mut<'a>(raw: &'a mut toml::Value, list: &str, id: &str) -> &'a mut toml::Value {
    raw[list]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e["id"].as_str() == Some(id))
        .unwrap()
}

/// The draft with admission for evidence review and every limit, conflict and external input
/// bound to an approved SYNTHETIC source; `edit` makes the single change under test.
fn complete_bound(edit: impl FnOnce(&mut toml::Value, &mut Pending, &ContentRef)) -> Fixture {
    let mut raw: toml::Value = toml::from_str(std::str::from_utf8(DRAFT).unwrap()).unwrap();
    let scope: ContentRef = raw["applicability"].clone().try_into().unwrap();
    let mut pending = Pending::new();
    raw["admission"] = toml::Value::String("admitted_for_evidence_review".into());
    for limit in raw["limits"].as_array_mut().unwrap() {
        let id = limit["id"].as_str().unwrap().to_string();
        let unit: DecisionUnit = limit["unit"].clone().try_into().unwrap();
        let comparison: Comparison = limit["comparison"].clone().try_into().unwrap();
        let (lower, upper) = synthetic_limit_value(&id);
        let value = bounded(unit, lower, upper);
        let source = pending_artifact(
            &mut pending,
            &format!("synthetic-limit-source-{id}"),
            &LimitSource {
                limit_id: id.clone(),
                value: value.clone(),
                comparison,
                applicability: scope.clone(),
            },
        );
        limit["value"] = toml_of(&RequirementValue::Admitted {
            value,
            source,
            applicability: scope.clone(),
        });
    }
    let statement = pending_artifact(
        &mut pending,
        "synthetic-conflict-statement",
        &serde_json::json!({"origin": "synthetic", "statement": "fixture resolution, not owner authority"}),
    );
    for conflict in raw["conflicts"].as_array_mut().unwrap() {
        let id = conflict["id"].as_str().unwrap().to_string();
        let alternatives: Vec<Alternative> = conflict["alternatives"].clone().try_into().unwrap();
        let resolution = pending_artifact(
            &mut pending,
            &format!("synthetic-conflict-resolution-{id}"),
            &ConflictResolution {
                conflict_id: id,
                alternatives,
                applicability: scope.clone(),
                resolution_statement: statement.clone(),
            },
        );
        conflict
            .as_table_mut()
            .unwrap()
            .insert("resolution".into(), toml_of(&resolution));
    }
    for input in raw["external_inputs"].as_array_mut().unwrap() {
        let id = input["id"].as_str().unwrap().to_string();
        let claims: Vec<String> = input["blocks"].clone().try_into().unwrap();
        let artifact = pending_artifact(
            &mut pending,
            &format!("synthetic-external-evidence-{id}"),
            &serde_json::json!({"origin": "synthetic", "input": id}),
        );
        let source = pending_artifact(
            &mut pending,
            &format!("synthetic-external-source-{id}"),
            &ExternalSource {
                input_id: id,
                artifact: artifact.clone(),
                applicability: scope.clone(),
                claims,
            },
        );
        input["evidence"] = toml_of(&RequirementValue::Admitted {
            value: artifact,
            source,
            applicability: scope.clone(),
        });
    }
    edit(&mut raw, &mut pending, &scope);
    let bytes = toml::to_string(&raw).unwrap();
    let mut x = Fixture::new(bytes.as_bytes());
    for (reference, bytes) in pending {
        x.approve(reference, bytes);
    }
    x
}

/// Public admission path for one claim scope (profile and reviewed family policy both bound).
fn admit(x: &mut Fixture, claim: &str) -> Result<(), DecisionError> {
    x.profile.claim_scope = claim.into();
    x.policy.claim_scope = claim.into();
    x.install_policy(&format!("synthetic-family-policy-{claim}"));
    x.profile.qualified_admission(&x.requirements, &x.registry)
}

fn unresolved(reason: &str, owner: &str, blocks: &toml::Value) -> toml::Value {
    let blocks: Vec<String> = blocks.clone().try_into().unwrap();
    toml_of(&RequirementValue::<ContentRef>::Unresolved {
        reason: reason.into(),
        owner: owner.into(),
        blocks,
    })
}

#[test]
fn complete_bound_synthetic_policy_admits_every_claim_scope_for_evidence_review() {
    let mut x = complete_bound(|_, _, _| {});
    x.valid().unwrap();
    for claim in CLAIM_SCOPES {
        assert_eq!(admit(&mut x, claim), Ok(()), "{claim}");
    }
}

#[test]
fn borrowed_machined_overflow_source_cannot_admit_a_printed_passage_limit() {
    // Exact borrow: the machined-overflow source record that admits 1.4 mm "within" is reused
    // as the printed-passage source; the refusal names the printed limit.
    let mut x = complete_bound(|raw, _, scope| {
        let machined: ContentRef = entry_mut(raw, "limits", "machined_overflow_nominal_mm")
            ["value"]["source"]
            .clone()
            .try_into()
            .unwrap();
        entry_mut(raw, "limits", "printed_min_passage_mm")["value"] =
            toml_of(&RequirementValue::Admitted {
                value: bounded(DecisionUnit::Millimetre, 1.4, 1.4),
                source: machined,
                applicability: scope.clone(),
            });
    });
    x.valid().unwrap();
    assert_eq!(
        admit(&mut x, "computational_comparator"),
        Err(DecisionError::Invalid(
            "limit_source_drift:printed_min_passage_mm".into()
        ))
    );
    // The draft's machined source (the baseline genome, do_over_d2 x D2 = 1.4 mm) is not a
    // printed-passage limit record either.
    let mut x = complete_bound(|raw, _, scope| {
        let genome: ContentRef = raw["baseline"].clone().try_into().unwrap();
        entry_mut(raw, "limits", "printed_min_passage_mm")["value"] =
            toml_of(&RequirementValue::Admitted {
                value: bounded(DecisionUnit::Millimetre, 1.4, 1.4),
                source: genome,
                applicability: scope.clone(),
            });
    });
    let err = admit(&mut x, "computational_comparator").unwrap_err();
    assert!(
        matches!(&err, DecisionError::Invalid(m) if m.contains("unknown field")),
        "{err}"
    );
}

#[test]
fn proper_printed_context_admits_without_a_numeric_ban_and_proposed_3mm_stays_blocked() {
    // 1.4 mm from a printed-process source bound to the printed limit is not banned by value.
    let mut x = complete_bound(|raw, pending, scope| {
        let value = bounded(DecisionUnit::Millimetre, 1.4, 1.4);
        let source = pending_artifact(
            pending,
            "synthetic-printed-process-passage-source",
            &LimitSource {
                limit_id: "printed_min_passage_mm".into(),
                value: value.clone(),
                comparison: Comparison::AtLeast,
                applicability: scope.clone(),
            },
        );
        entry_mut(raw, "limits", "printed_min_passage_mm")["value"] =
            toml_of(&RequirementValue::Admitted {
                value,
                source,
                applicability: scope.clone(),
            });
    });
    assert_eq!(admit(&mut x, "computational_comparator"), Ok(()));
    // The draft's proposed 3 mm is never silently admitted.
    let mut x = complete_bound(|raw, _, _| {
        let mut draft: toml::Value = toml::from_str(std::str::from_utf8(DRAFT).unwrap()).unwrap();
        let proposed = entry_mut(&mut draft, "limits", "printed_min_passage_mm")["value"].clone();
        assert_eq!(proposed["status"].as_str(), Some("proposed"));
        entry_mut(raw, "limits", "printed_min_passage_mm")["value"] = proposed;
    });
    for claim in COMPUTATIONAL {
        assert_eq!(
            admit(&mut x, claim),
            Err(DecisionError::Unresolved(vec![
                "limit:printed_min_passage_mm".into()
            ])),
            "{claim}"
        );
    }
    assert_eq!(admit(&mut x, "protected_export"), Ok(()));
}

/// Formerly the G-004 ignored contract-blocker control (red log 2ef2a8f2…12c6 at 794d044): a
/// record restating the machined 1.4 mm with the printed comparison was admitted. With
/// `LimitSource.limit_id` the machined identity is refused even though value, comparison and
/// applicability all match the printed limit, in both directions.
#[test]
fn restated_machined_limit_identity_is_refused_for_the_printed_limit() {
    let mut x = complete_bound(|raw, pending, scope| {
        let value = bounded(DecisionUnit::Millimetre, 1.4, 1.4);
        let source = pending_artifact(
            pending,
            "synthetic-machined-overflow-1.4mm-restated-at-least",
            &LimitSource {
                limit_id: "machined_overflow_nominal_mm".into(),
                value: value.clone(),
                comparison: Comparison::AtLeast,
                applicability: scope.clone(),
            },
        );
        entry_mut(raw, "limits", "printed_min_passage_mm")["value"] =
            toml_of(&RequirementValue::Admitted {
                value,
                source,
                applicability: scope.clone(),
            });
    });
    x.valid().unwrap();
    assert_eq!(
        admit(&mut x, "computational_comparator"),
        Err(DecisionError::Invalid(
            "limit_source_drift:printed_min_passage_mm".into()
        ))
    );
    // Reverse borrow: a printed-passage record restated as the machined "within" value.
    let mut x = complete_bound(|raw, pending, scope| {
        let value = bounded(DecisionUnit::Millimetre, 1.4, 1.4);
        let source = pending_artifact(
            pending,
            "synthetic-printed-passage-1.4mm-restated-within",
            &LimitSource {
                limit_id: "printed_min_passage_mm".into(),
                value: value.clone(),
                comparison: Comparison::Within,
                applicability: scope.clone(),
            },
        );
        entry_mut(raw, "limits", "machined_overflow_nominal_mm")["value"] =
            toml_of(&RequirementValue::Admitted {
                value,
                source,
                applicability: scope.clone(),
            });
    });
    assert_eq!(
        admit(&mut x, "computational_comparator"),
        Err(DecisionError::Invalid(
            "limit_source_drift:machined_overflow_nominal_mm".into()
        ))
    );
}

/// Old generic limit records (a `SourceValue` without `limit_id`) are not migrated automatically.
#[test]
fn generic_limit_record_without_limit_id_is_refused_not_migrated() {
    for (limit, unit, lower, upper, comparison) in [
        (
            "printed_min_passage_mm",
            DecisionUnit::Millimetre,
            1.4,
            1.4,
            Comparison::AtLeast,
        ),
        (
            "pressure_budget_pa",
            DecisionUnit::Pascal,
            1.0e5,
            1.0e5,
            Comparison::AtMost,
        ),
    ] {
        let mut x = complete_bound(|raw, pending, scope| {
            let value = bounded(unit, lower, upper);
            let source = pending_artifact(
                pending,
                &format!("synthetic-generic-limit-record-{limit}"),
                &SourceValue {
                    value: value.clone(),
                    comparison,
                    applicability: scope.clone(),
                },
            );
            entry_mut(raw, "limits", limit)["value"] = toml_of(&RequirementValue::Admitted {
                value,
                source,
                applicability: scope.clone(),
            });
        });
        x.valid().unwrap();
        let err = admit(&mut x, "computational_comparator").unwrap_err();
        assert!(
            matches!(&err, DecisionError::Invalid(m) if m.contains("missing field `limit_id`")),
            "{limit}: {err}"
        );
    }
}

/// Integrator counterexample (Codex 01:05Z, red log 40b33dff…5056 at 63c93dd): a reviewed
/// `LimitSource` whose lower bound is -0.0 equalled a valid +0.0 requirement under float equality.
/// The source bound is now checked before comparison; a genuine +0.0 source still admits.
#[test]
fn zero_limit_source_admits_and_negative_zero_source_is_refused_before_equality() {
    let with_source_lower = |lower: f64| {
        complete_bound(move |raw, pending, scope| {
            let source = pending_artifact(
                pending,
                &format!("synthetic-external-demand-source-lower-{lower:?}"),
                &LimitSource {
                    limit_id: "external_demand_pa".into(),
                    value: bounded(DecisionUnit::Pascal, lower, 2.5e6),
                    comparison: Comparison::Within,
                    applicability: scope.clone(),
                },
            );
            entry_mut(raw, "limits", "external_demand_pa")["value"] =
                toml_of(&RequirementValue::Admitted {
                    value: bounded(DecisionUnit::Pascal, 0.0, 2.5e6),
                    source,
                    applicability: scope.clone(),
                });
        })
    };
    let mut x = with_source_lower(0.0);
    x.valid().unwrap();
    for claim in CLAIM_SCOPES {
        assert_eq!(admit(&mut x, claim), Ok(()), "{claim}");
    }
    let mut x = with_source_lower(-0.0);
    x.valid().unwrap();
    for claim in COMPUTATIONAL {
        assert_eq!(
            admit(&mut x, claim),
            Err(DecisionError::Invalid(
                "reversed_bound_or_negative_zero".into()
            )),
            "{claim}"
        );
    }
}

#[test]
fn each_missing_external_input_blocks_exactly_its_claims_with_its_owner() {
    let probe = complete_bound(|_, _, _| {});
    let ids: std::collections::BTreeSet<_> = probe
        .requirements
        .external_inputs
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    let expected: std::collections::BTreeSet<_> =
        EXTERNAL_EXPECTATIONS.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(ids, expected);
    for (input, owner, blocked) in EXTERNAL_EXPECTATIONS {
        let mut x = complete_bound(|raw, _, _| {
            let entry = entry_mut(raw, "external_inputs", input);
            let missing = unresolved(
                "SYNTHETIC missing input: owner data not supplied",
                owner,
                &entry["blocks"],
            );
            entry["evidence"] = missing;
        });
        x.valid().unwrap();
        for claim in CLAIM_SCOPES {
            let expected = if blocked.contains(&claim) {
                Err(DecisionError::Unresolved(vec![format!(
                    "external:{input}:{owner}"
                )]))
            } else {
                Ok(())
            };
            assert_eq!(admit(&mut x, claim), expected, "{input} x {claim}");
        }
    }
    // All nine missing together: each claim lists exactly the inputs the matrix assigns to it.
    let mut x = complete_bound(|raw, _, _| {
        for (input, owner, _) in EXTERNAL_EXPECTATIONS {
            let entry = entry_mut(raw, "external_inputs", input);
            let missing = unresolved("SYNTHETIC missing input", owner, &entry["blocks"]);
            entry["evidence"] = missing;
        }
    });
    for claim in CLAIM_SCOPES {
        let mut expected: Vec<String> = EXTERNAL_EXPECTATIONS
            .iter()
            .filter(|(_, _, blocked)| blocked.contains(&claim))
            .map(|(input, owner, _)| format!("external:{input}:{owner}"))
            .collect();
        expected.sort();
        assert_eq!(
            admit(&mut x, claim),
            Err(DecisionError::Unresolved(expected)),
            "{claim}"
        );
    }
}

#[test]
fn waiting_data_timeout_stays_unresolved_with_reason_not_infeasible_or_zero_uncertainty() {
    let reason =
        "SYNTHETIC waiting-data timeout: laboratory access data not received before review";
    let mut x = complete_bound(|raw, _, _| {
        let entry = entry_mut(raw, "external_inputs", "laboratory_access");
        let timeout = unresolved(reason, "experimental_lead", &entry["blocks"]);
        entry["evidence"] = timeout;
    });
    let bounds_before = (
        x.profile.numerical_bounds.clone(),
        x.profile.model_discrepancy.clone(),
    );
    // Semantically valid policy: the timeout is not an infeasible or invalid design.
    x.valid().unwrap();
    for claim in ["physical_candidate", "commercial_claim"] {
        assert_eq!(
            admit(&mut x, claim),
            Err(DecisionError::Unresolved(vec![
                "external:laboratory_access:experimental_lead".into()
            ])),
            "{claim}"
        );
    }
    for claim in ["computational_comparator", "protected_export"] {
        assert_eq!(admit(&mut x, claim), Ok(()), "{claim}");
    }
    // The reviewed requirement record keeps the timeout and its reason; nothing became admitted
    // and no uncertainty bound was created or narrowed.
    let lab = x
        .requirements
        .external_inputs
        .iter()
        .find(|e| e.id == "laboratory_access")
        .unwrap();
    assert!(matches!(
        &lab.evidence,
        RequirementValue::Unresolved { reason: r, owner, blocks }
            if r == reason && owner == "experimental_lead"
                && blocks == &vec!["physical_candidate".to_string(), "commercial_claim".to_string()]
    ));
    assert!(!lab.evidence.is_admitted());
    assert_eq!(
        (
            x.profile.numerical_bounds.clone(),
            x.profile.model_discrepancy.clone()
        ),
        bounds_before
    );
    // A timeout cannot be recorded as affecting no claims.
    let mut raw: toml::Value = toml::from_str(std::str::from_utf8(DRAFT).unwrap()).unwrap();
    entry_mut(&mut raw, "external_inputs", "laboratory_access")["evidence"]["blocks"] =
        toml::Value::Array(vec![]);
    assert_eq!(
        RequirementSet::from_toml(toml::to_string(&raw).unwrap().as_bytes()),
        Err(DecisionError::Invalid(
            "unresolved_input_needs_affected_claims".into()
        ))
    );
}
