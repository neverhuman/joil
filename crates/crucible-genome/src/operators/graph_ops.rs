use super::{next_innovation, reject, OperatorCost, OperatorOutcome, OutcomeKind, Proposal};
use crate::schema::{EdgeRelation, Genome, ModuleEdge};

pub(super) fn apply_graph_proposal(
    parent: &Genome,
    parent_id: &str,
    child: &mut Genome,
    cost: OperatorCost,
    proposal: Proposal,
) -> Result<((String, &'static str), OperatorCost), Box<OperatorOutcome>> {
    let pid = || parent_id.to_string();
    let res = match proposal {
        Proposal::MergeModule { module } => {
            if module >= child.modules.len() {
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "module index not applicable",
                    OutcomeKind::Rejected,
                )));
            }
            if child.edges.iter().any(|e| {
                e.relation == EdgeRelation::FluidFlow
                    && (e.from_module as usize == module || e.to_module as usize == module)
            }) {
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "merge cannot remap incident fluid-flow references",
                    OutcomeKind::Rejected,
                )));
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
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "merge requires exactly one construction predecessor",
                    OutcomeKind::Rejected,
                )));
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
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "merge requires outgoing construction edges",
                    OutcomeKind::Rejected,
                )));
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
                    return Err(Box::new(reject(
                        pid(),
                        cost,
                        "predecessor port cannot preserve outgoing typed-port contract",
                        OutcomeKind::Rejected,
                    )));
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
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "edge index not applicable",
                    OutcomeKind::Rejected,
                )));
            };
            e.from_module = from_module as u32;
            e.from_port = from_port;
            e.to_module = to_module as u32;
            e.to_port = to_port;
            (format!("edge:{edge}"), "edge_endpoints")
        }
        Proposal::SplitEdge { edge, mut inserted } => {
            let Some(old) = child.edges.get(edge).cloned() else {
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "edge index not applicable",
                    OutcomeKind::Rejected,
                )));
            };
            if old.relation != EdgeRelation::Construction {
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "only construction edges can be split",
                    OutcomeKind::Rejected,
                )));
            }
            let target_index = old.to_module as usize;
            let Some(target) = child.modules.get(target_index) else {
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "edge target is invalid",
                    OutcomeKind::Rejected,
                )));
            };
            let Some(target_port) = target.ports.iter().find(|p| p.name == old.to_port) else {
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "edge target port is invalid",
                    OutcomeKind::Rejected,
                )));
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
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "inserted module has no compatible input port",
                    OutcomeKind::Rejected,
                )));
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
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "inserted module has no compatible output port",
                    OutcomeKind::Rejected,
                )));
            };
            let idx = target_index as u32;
            inserted.innovation = match next_innovation(child) {
                Ok(v) => v,
                Err(e) => return Err(Box::new(reject(pid(), cost, e, OutcomeKind::Rejected))),
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
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    format!("invalid second parent: {e}"),
                    OutcomeKind::Rejected,
                )));
            }
            if parent.interface != other.interface || parent.schema_version != other.schema_version
            {
                return Err(Box::new(reject(
                    pid(),
                    cost,
                    "parents have incompatible interfaces or schemas",
                    OutcomeKind::Rejected,
                )));
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
        _ => unreachable!("handled in operators::mod"),
    };
    Ok((res, cost))
}
