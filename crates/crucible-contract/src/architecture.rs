//! Cargo resolve graph traversal includes normal, build and dev dependencies.
use crate::ContractError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    Normal,
    Build,
    Dev,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub kind: DependencyKind,
}
pub fn enforce_blind_boundary(
    roots: &[String],
    edges: &[DependencyEdge],
    forbidden: &BTreeSet<String>,
) -> Result<(), ContractError> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for e in edges {
        adjacency.entry(&e.from).or_default().push(&e.to);
    }
    let mut queue: VecDeque<Vec<String>> = roots.iter().map(|r| vec![r.clone()]).collect();
    let mut visited = BTreeSet::new();
    while let Some(path) = queue.pop_front() {
        let current = path.last().unwrap();
        if forbidden.contains(current) {
            return Err(ContractError::ForbiddenDependency(path));
        }
        if !visited.insert(current.clone()) {
            continue;
        }
        for child in adjacency.get(current.as_str()).into_iter().flatten() {
            let mut next = path.clone();
            next.push((*child).into());
            queue.push_back(next);
        }
    }
    Ok(())
}
/// Parse Cargo's resolved package IDs, rather than guessing from manifest aliases.
pub fn edges_from_cargo_metadata(bytes: &[u8]) -> Result<Vec<DependencyEdge>, ContractError> {
    let data: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| ContractError::Wire(e.to_string()))?;
    let packages = data["packages"]
        .as_array()
        .ok_or(ContractError::InvalidKey("cargo_packages"))?;
    let mut names = BTreeMap::new();
    for p in packages {
        names.insert(
            p["id"]
                .as_str()
                .ok_or(ContractError::InvalidKey("package_id"))?,
            p["name"]
                .as_str()
                .ok_or(ContractError::InvalidKey("package_name"))?,
        );
    }
    let nodes = data["resolve"]["nodes"]
        .as_array()
        .ok_or(ContractError::InvalidKey("cargo_resolve"))?;
    let mut edges = Vec::new();
    for node in nodes {
        let from = names
            .get(
                node["id"]
                    .as_str()
                    .ok_or(ContractError::InvalidKey("node_id"))?,
            )
            .ok_or(ContractError::InvalidKey("unknown_node"))?;
        for dep in node["deps"]
            .as_array()
            .ok_or(ContractError::InvalidKey("node_deps"))?
        {
            let to = names
                .get(
                    dep["pkg"]
                        .as_str()
                        .ok_or(ContractError::InvalidKey("dep_id"))?,
                )
                .ok_or(ContractError::InvalidKey("unknown_dep"))?;
            let kinds = dep["dep_kinds"]
                .as_array()
                .ok_or(ContractError::InvalidKey("dep_kinds"))?;
            if kinds.is_empty() {
                return Err(ContractError::InvalidKey("missing_dependency_kind"));
            }
            for kind in kinds {
                let kind = match kind.get("kind") {
                    Some(serde_json::Value::Null) => DependencyKind::Normal,
                    Some(serde_json::Value::String(s)) if s == "build" => DependencyKind::Build,
                    Some(serde_json::Value::String(s)) if s == "dev" => DependencyKind::Dev,
                    _ => return Err(ContractError::InvalidKey("unknown_dependency_kind")),
                };
                edges.push(DependencyEdge {
                    from: (*from).into(),
                    to: (*to).into(),
                    kind,
                });
            }
        }
    }
    Ok(edges)
}
