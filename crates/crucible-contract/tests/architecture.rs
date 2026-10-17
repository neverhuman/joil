use crucible_contract::architecture::*;
use crucible_contract::ContractError;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

#[test]
fn actual_resolved_normal_build_dev_graph_keeps_judges_blind() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let result = Command::new("rtk")
        .args([
            "proxy",
            "cargo",
            "metadata",
            "--locked",
            "--offline",
            "--all-features",
            "--format-version",
            "1",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let edges = edges_from_cargo_metadata(&result.stdout).unwrap();
    assert!(!edges.is_empty());
    let mut forbidden: BTreeSet<_> = [
        "crucible-genome",
        "crucible-search",
        "joil-core",
        "joil-ir",
        "hydrocyclone-modules",
        "crucible-origin",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let metadata: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let mut roots = Vec::new();
    for p in metadata["packages"].as_array().unwrap() {
        let name = p["name"].as_str().unwrap();
        let role = p["metadata"]["crucible"]["role"].as_str().unwrap_or("");
        if name == "crucible-contract"
            || name.starts_with("crucible-judge")
            || name.ends_with("-judges")
            || name == "crucible-adapters"
            || matches!(role, "judge" | "evaluator_contract" | "native_evaluator")
        {
            roots.push(name.to_string());
        }
        if matches!(
            role,
            "genome_registry" | "module_registry" | "search_state" | "origin_sidecar"
        ) {
            forbidden.insert(name.to_string());
        }
    }
    assert!(roots.contains(&"crucible-contract".to_string()));
    enforce_blind_boundary(&roots, &edges, &forbidden).unwrap();
}
#[test]
fn indirect_build_and_dev_bridges_cannot_hide_forbidden_registry() {
    for kind in [
        DependencyKind::Normal,
        DependencyKind::Build,
        DependencyKind::Dev,
    ] {
        let edges = vec![
            DependencyEdge {
                from: "judge".into(),
                to: "ordinary-helper".into(),
                kind: DependencyKind::Normal,
            },
            DependencyEdge {
                from: "ordinary-helper".into(),
                to: "build-bridge".into(),
                kind: kind.clone(),
            },
            DependencyEdge {
                from: "build-bridge".into(),
                to: "crucible-genome".into(),
                kind: DependencyKind::Dev,
            },
        ];
        let forbidden = BTreeSet::from(["crucible-genome".into()]);
        let error = enforce_blind_boundary(&["judge".into()], &edges, &forbidden).unwrap_err();
        assert!(
            matches!(error,ContractError::ForbiddenDependency(path) if path==vec!["judge","ordinary-helper","build-bridge","crucible-genome"])
        );
        enforce_blind_boundary(&["unrelated-renderer".into()], &edges, &forbidden).unwrap();
    }
}
#[test]
fn cargo_parser_preserves_renamed_package_and_dependency_kinds() {
    let metadata = serde_json::json!({"packages":[{"id":"a-id","name":"judge"},{"id":"b-id","name":"bridge"},{"id":"c-id","name":"crucible-genome"}],"resolve":{"nodes":[{"id":"a-id","deps":[{"name":"innocent_alias","pkg":"b-id","dep_kinds":[{"kind":"build"}]}]},{"id":"b-id","deps":[{"name":"hidden_alias","pkg":"c-id","dep_kinds":[{"kind":"dev"}]}]},{"id":"c-id","deps":[]}]}});
    let edges = edges_from_cargo_metadata(&serde_json::to_vec(&metadata).unwrap()).unwrap();
    assert_eq!(edges[0].kind, DependencyKind::Build);
    assert_eq!(edges[1].kind, DependencyKind::Dev);
    assert!(enforce_blind_boundary(
        &["judge".into()],
        &edges,
        &BTreeSet::from(["crucible-genome".into()])
    )
    .is_err());
}
