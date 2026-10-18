//! Deterministic, single-proposal MPG operators. Every proposal is accounted for,
//! including inapplicable, invalid, neutral, duplicate and over-budget attempts.
use crate::schema::{EdgeRelation, FieldOp, Genome, InnovationId, Module, ModuleEdge, ModuleKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operator", rename_all = "snake_case", deny_unknown_fields)]
pub enum Proposal {
    ParameterPerturb {
        field: usize,
        delta: f64,
    },
    ThicknessPerturb {
        field: usize,
        delta: f64,
    },
    SplitEdge {
        edge: usize,
        inserted: Module,
    },
    MergeModule {
        module: usize,
    },
    Reconnect {
        edge: usize,
        from_module: usize,
        from_port: String,
        to_module: usize,
        to_port: String,
    },
    AddModule {
        module: Module,
    },
    RemoveModule {
        module: usize,
    },
    DuplicateModule {
        module: usize,
    },
    /// Adds or removes a structurally paired copy. This is graph symmetry only;
    /// it does not assert geometric reflection or a physical mirror plane.
    ChangeGraphSymmetry {
        module: usize,
        enable: bool,
    },
    HomologousCrossover {
        other: Genome,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    Accepted,
    Neutral,
    Duplicate,
    Rejected,
    BudgetExceeded,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorCost {
    pub attempts: u64,
    pub modules_examined: u64,
    pub edges_examined: u64,
    pub fields_examined: u64,
    pub proposed_modules: u64,
    pub proposed_edges: u64,
    pub proposed_fields: u64,
    pub over_budget: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorOutcome {
    pub kind: OutcomeKind,
    pub reason: Option<String>,
    pub proposal: Option<Proposal>,
    pub cost: OperatorCost,
    pub parent_id: String,
    pub proposal_id: Option<String>,
    pub child: Option<Genome>,
    pub changed_observables: Vec<String>,
    pub locality: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperatorBudget {
    pub max_modules: usize,
    pub max_edges: usize,
    pub max_fields: usize,
}

fn id(g: &Genome) -> Result<String, String> {
    g.semantic_identity().map_err(|e| e.to_string())
}
fn next_innovation(g: &Genome) -> Result<InnovationId, String> {
    g.modules
        .iter()
        .map(|m| m.innovation.0)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .map(InnovationId)
        .ok_or_else(|| "innovation id exhausted".into())
}
fn reject(
    parent_id: String,
    cost: OperatorCost,
    why: impl Into<String>,
    kind: OutcomeKind,
) -> OperatorOutcome {
    OperatorOutcome {
        kind,
        reason: Some(why.into()),
        proposal: None,
        cost,
        parent_id,
        proposal_id: None,
        child: None,
        changed_observables: vec![],
        locality: "none".into(),
    }
}
fn reindex_edges(g: &mut Genome, removed: usize) {
    g.edges
        .retain(|e| e.from_module as usize != removed && e.to_module as usize != removed);
    for e in &mut g.edges {
        if e.from_module as usize > removed {
            e.from_module -= 1;
        }
        if e.to_module as usize > removed {
            e.to_module -= 1;
        }
    }
}

/// Apply exactly one proposal. No retry, clamping, fallback, or baseline substitution occurs.
pub fn attempt(parent: &Genome, proposal: Proposal, budget: OperatorBudget) -> OperatorOutcome {
    let retained = proposal.clone();
    let mut outcome = attempt_inner(parent, proposal, budget);
    outcome.proposal = Some(retained);
    outcome
}

fn attempt_inner(parent: &Genome, proposal: Proposal, budget: OperatorBudget) -> OperatorOutcome {
    let parent_id = match id(parent) {
        Ok(v) => v,
        Err(e) => {
            return reject(
                String::new(),
                OperatorCost {
                    attempts: 1,
                    ..Default::default()
                },
                format!("invalid parent: {e}"),
                OutcomeKind::Rejected,
            )
        }
    };
    let mut cost = OperatorCost {
        attempts: 1,
        modules_examined: parent.modules.len() as u64,
        edges_examined: parent.edges.len() as u64,
        fields_examined: parent.fields.len() as u64,
        ..Default::default()
    };
    let mut child = parent.clone();
    let thickness_only = matches!(&proposal, Proposal::ThicknessPerturb { .. });
    let (locality, observable) = match proposal {
        Proposal::ParameterPerturb { field, delta }
        | Proposal::ThicknessPerturb { field, delta } => {
            cost.fields_examined = 1;
            cost.proposed_fields = 1;
            let Some(op) = child.fields.get_mut(field) else {
                return reject(
                    parent_id,
                    cost,
                    "field index not applicable",
                    OutcomeKind::Rejected,
                );
            };
            if !delta.is_finite() {
                return reject(
                    parent_id,
                    cost,
                    "nonfinite perturbation",
                    OutcomeKind::Rejected,
                );
            }
            let value = match op {
                FieldOp::Constant { value } if !thickness_only => value,
                FieldOp::Constant { .. } => {
                    return reject(
                        parent_id,
                        cost,
                        "thickness perturbation requires shell or rib primitive",
                        OutcomeKind::Rejected,
                    )
                }
                FieldOp::Primitive {
                    kind, coefficients, ..
                } => {
                    if thickness_only && !matches!(kind, ModuleKind::Shell | ModuleKind::Rib) {
                        return reject(
                            parent_id,
                            cost,
                            "thickness perturbation requires shell or rib primitive",
                            OutcomeKind::Rejected,
                        );
                    }
                    let Some(v) = coefficients.first_mut() else {
                        return reject(
                            parent_id,
                            cost,
                            "primitive has no coefficient",
                            OutcomeKind::Rejected,
                        );
                    };
                    v
                }
                _ => {
                    return reject(
                        parent_id,
                        cost,
                        "field has no directly perturbable scalar",
                        OutcomeKind::Rejected,
                    )
                }
            };
            let next = *value + delta;
            if !next.is_finite() {
                return reject(parent_id, cost, "nonfinite result", OutcomeKind::Rejected);
            }
            *value = next;
            (
                format!("field:{field}"),
                if matches!(op, FieldOp::Primitive { .. }) {
                    "primitive_coefficient"
                } else {
                    "field_constant"
                },
            )
        }
        Proposal::AddModule { mut module } => {
            let innovation = match next_innovation(&child) {
                Ok(v) => v,
                Err(e) => return reject(parent_id, cost, e, OutcomeKind::Rejected),
            };
            module.innovation = innovation;
            child.modules.push(module);
            ("modules:append".into(), "module_count")
        }
        Proposal::DuplicateModule { module }
        | Proposal::ChangeGraphSymmetry {
            module,
            enable: true,
        } => {
            let Some(mut copy) = child.modules.get(module).cloned() else {
                return reject(
                    parent_id,
                    cost,
                    "module index not applicable",
                    OutcomeKind::Rejected,
                );
            };
            copy.innovation = match next_innovation(&child) {
                Ok(v) => v,
                Err(e) => return reject(parent_id, cost, e, OutcomeKind::Rejected),
            };
            child.modules.push(copy);
            (format!("module:{module}"), "module_count")
        }
        Proposal::ChangeGraphSymmetry {
            module,
            enable: false,
        } => {
            let Some(target) = child.modules.get(module).cloned() else {
                return reject(
                    parent_id,
                    cost,
                    "module index not applicable",
                    OutcomeKind::Rejected,
                );
            };
            let Some(last) = child.modules.iter().rposition(|m| {
                m.kind == target.kind
                    && m.innovation > target.innovation
                    && m.ports == target.ports
                    && m.field_roots == target.field_roots
            }) else {
                return reject(
                    parent_id,
                    cost,
                    "no graph-symmetry partner",
                    OutcomeKind::Rejected,
                );
            };
            if child
                .edges
                .iter()
                .any(|e| e.from_module as usize == last || e.to_module as usize == last)
            {
                return reject(
                    parent_id,
                    cost,
                    "partner still referenced",
                    OutcomeKind::Rejected,
                );
            }
            child.modules.remove(last);
            reindex_edges(&mut child, last);
            (format!("module:{last}"), "module_count")
        }
        Proposal::RemoveModule { module } => {
            if module >= child.modules.len() {
                return reject(
                    parent_id,
                    cost,
                    "module index not applicable",
                    OutcomeKind::Rejected,
                );
            }
            if child
                .edges
                .iter()
                .any(|e| e.from_module as usize == module || e.to_module as usize == module)
            {
                return reject(
                    parent_id,
                    cost,
                    "module is referenced by graph edges",
                    OutcomeKind::Rejected,
                );
            }
            child.modules.remove(module);
            reindex_edges(&mut child, module);
            (format!("module:{module}"), "module_count")
        }
        Proposal::MergeModule { module } => {
            if module >= child.modules.len() {
                return reject(
                    parent_id,
                    cost,
                    "module index not applicable",
                    OutcomeKind::Rejected,
                );
            }
            if child.edges.iter().any(|e| {
                e.relation == EdgeRelation::FluidFlow
                    && (e.from_module as usize == module || e.to_module as usize == module)
            }) {
                return reject(
                    parent_id,
                    cost,
                    "merge cannot remap incident fluid-flow references",
                    OutcomeKind::Rejected,
                );
            }
            let incoming: Vec<usize> = child
                .edges
                .iter()
                .enumerate()
                .filter_map(|(i, e)| {
                    (e.relation == EdgeRelation::Construction && e.to_module as usize == module)
                        .then_some(i)
                })
                .collect();
            if incoming.len() != 1 {
                return reject(
                    parent_id,
                    cost,
                    "merge requires exactly one construction predecessor",
                    OutcomeKind::Rejected,
                );
            }
            let in_edge = child.edges[incoming[0]].clone();
            let outgoing: Vec<usize> = child
                .edges
                .iter()
                .enumerate()
                .filter_map(|(i, e)| {
                    (e.relation == EdgeRelation::Construction && e.from_module as usize == module)
                        .then_some(i)
                })
                .collect();
            if outgoing.is_empty() {
                return reject(
                    parent_id,
                    cost,
                    "merge requires outgoing construction edges",
                    OutcomeKind::Rejected,
                );
            }
            let up = in_edge.from_module as usize;
            let predecessor_port = child.modules[up]
                .ports
                .iter()
                .find(|p| p.name == in_edge.from_port)
                .cloned()
                .unwrap();
            for i in &outgoing {
                let old_port = child.modules[module]
                    .ports
                    .iter()
                    .find(|p| p.name == child.edges[*i].from_port)
                    .unwrap();
                if old_port.domain != predecessor_port.domain
                    || old_port.section != predecessor_port.section
                    || old_port.frame != predecessor_port.frame
                    || old_port.unit != predecessor_port.unit
                {
                    return reject(
                        parent_id,
                        cost,
                        "predecessor port cannot preserve outgoing typed-port contract",
                        OutcomeKind::Rejected,
                    );
                }
            }
            for i in outgoing {
                child.edges[i].from_module = up as u32;
                child.edges[i].from_port = in_edge.from_port.clone();
            }
            child.edges.remove(incoming[0]);
            child.modules.remove(module);
            for e in &mut child.edges {
                if e.from_module as usize > module {
                    e.from_module -= 1;
                }
                if e.to_module as usize > module {
                    e.to_module -= 1;
                }
            }
            (format!("module:{module}"), "module_count")
        }
        Proposal::Reconnect {
            edge,
            from_module,
            from_port,
            to_module,
            to_port,
        } => {
            let Some(e) = child.edges.get_mut(edge) else {
                return reject(
                    parent_id,
                    cost,
                    "edge index not applicable",
                    OutcomeKind::Rejected,
                );
            };
            e.from_module = from_module as u32;
            e.from_port = from_port;
            e.to_module = to_module as u32;
            e.to_port = to_port;
            (format!("edge:{edge}"), "edge_endpoints")
        }
        Proposal::SplitEdge { edge, mut inserted } => {
            let Some(old) = child.edges.get(edge).cloned() else {
                return reject(
                    parent_id,
                    cost,
                    "edge index not applicable",
                    OutcomeKind::Rejected,
                );
            };
            if old.relation != EdgeRelation::Construction {
                return reject(
                    parent_id,
                    cost,
                    "only construction edges can be split",
                    OutcomeKind::Rejected,
                );
            }
            let target_index = old.to_module as usize;
            let Some(target) = child.modules.get(target_index) else {
                return reject(
                    parent_id,
                    cost,
                    "edge target is invalid",
                    OutcomeKind::Rejected,
                );
            };
            let Some(target_port) = target.ports.iter().find(|p| p.name == old.to_port) else {
                return reject(
                    parent_id,
                    cost,
                    "edge target port is invalid",
                    OutcomeKind::Rejected,
                );
            };
            let Some(in_port) = inserted
                .ports
                .iter()
                .find(|p| {
                    p.domain == target_port.domain
                        && p.section == target_port.section
                        && p.frame == target_port.frame
                        && p.unit == target_port.unit
                })
                .map(|p| p.name.clone())
            else {
                return reject(
                    parent_id,
                    cost,
                    "inserted module has no compatible input port",
                    OutcomeKind::Rejected,
                );
            };
            let Some(out_port) = inserted
                .ports
                .iter()
                .find(|p| {
                    p.name != in_port
                        && p.domain == target_port.domain
                        && p.section == target_port.section
                        && p.frame == target_port.frame
                        && p.unit == target_port.unit
                })
                .map(|p| p.name.clone())
            else {
                return reject(
                    parent_id,
                    cost,
                    "inserted module has no compatible output port",
                    OutcomeKind::Rejected,
                );
            };
            let idx = target_index as u32;
            inserted.innovation = match next_innovation(&child) {
                Ok(v) => v,
                Err(e) => return reject(parent_id, cost, e, OutcomeKind::Rejected),
            };
            child.modules.insert(target_index, inserted);
            for e in &mut child.edges {
                if e.from_module as usize >= target_index {
                    e.from_module += 1;
                }
                if e.to_module as usize >= target_index {
                    e.to_module += 1;
                }
            }
            child.edges[edge].to_module = idx;
            child.edges[edge].to_port = in_port;
            child.edges.push(ModuleEdge {
                relation: EdgeRelation::Construction,
                from_module: idx,
                from_port: out_port,
                to_module: old.to_module + 1,
                to_port: old.to_port,
            });
            (format!("edge:{edge}"), "module_count,edge_count")
        }
        Proposal::HomologousCrossover { other } => {
            if let Err(e) = other.validate() {
                return reject(
                    parent_id,
                    cost,
                    format!("invalid second parent: {e}"),
                    OutcomeKind::Rejected,
                );
            }
            if parent.interface != other.interface || parent.schema_version != other.schema_version
            {
                return reject(
                    parent_id,
                    cost,
                    "parents have incompatible interfaces or schemas",
                    OutcomeKind::Rejected,
                );
            }
            // Align modules strictly by stable innovation ID and kind. Copy only same-ID homologues.
            for m in &mut child.modules {
                if let Some(m2) = other
                    .modules
                    .iter()
                    .find(|x| x.innovation == m.innovation && x.kind == m.kind)
                {
                    *m = m2.clone();
                }
            }
            ("homologous innovations only".into(), "homologous_modules")
        }
    };
    cost.proposed_modules += child.modules.len().saturating_sub(parent.modules.len()) as u64;
    cost.proposed_edges += child.edges.len().saturating_sub(parent.edges.len()) as u64;
    cost.proposed_fields += child.fields.len().saturating_sub(parent.fields.len()) as u64;
    if child.modules.len() > budget.max_modules
        || child.edges.len() > budget.max_edges
        || child.fields.len() > budget.max_fields
    {
        cost.over_budget = true;
        return reject(
            parent_id,
            cost,
            "operator budget exceeded",
            OutcomeKind::BudgetExceeded,
        );
    }
    if let Err(e) = child.validate() {
        return reject(
            parent_id,
            cost,
            format!("proposal invalid: {e}"),
            OutcomeKind::Rejected,
        );
    }
    let proposal_id = match id(&child) {
        Ok(v) => v,
        Err(e) => return reject(parent_id, cost, e, OutcomeKind::Rejected),
    };
    if proposal_id == parent_id {
        return OperatorOutcome {
            kind: OutcomeKind::Neutral,
            reason: Some("valid proposal leaves semantic subject unchanged".into()),
            proposal: None,
            cost,
            parent_id,
            proposal_id: Some(proposal_id),
            child: Some(child),
            changed_observables: vec![],
            locality,
        };
    }
    // A repeated valid subject is explicitly a duplicate; callers compare against their archive.
    OperatorOutcome {
        kind: OutcomeKind::Accepted,
        reason: None,
        proposal: None,
        cost,
        parent_id,
        proposal_id: Some(proposal_id),
        child: Some(child),
        changed_observables: vec![observable.into()],
        locality,
    }
}

/// Classify a valid attempt against a caller-owned set of previously seen subjects.
pub fn classify_duplicate(
    outcome: &mut OperatorOutcome,
    seen: &std::collections::BTreeSet<String>,
) {
    if outcome.kind == OutcomeKind::Accepted
        && outcome
            .proposal_id
            .as_ref()
            .is_some_and(|id| seen.contains(id))
    {
        outcome.kind = OutcomeKind::Duplicate;
        outcome.reason = Some("subject already present in archive".into());
    }
}
