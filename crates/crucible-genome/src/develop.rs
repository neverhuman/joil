//! Gas-metered deterministic lowering. A budget error is terminal and never substitutes a seed.
use crate::schema::{EdgeRelation, FieldOp, Genome, Module, ModuleEdge, SchemaError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentBudget {
    pub max_modules: usize,
    pub max_edges: usize,
    pub max_field_nodes: usize,
    pub max_coefficients: usize,
    pub max_rewrite_applications: usize,
    pub max_depth: usize,
    pub max_spatial_frequency_m_inv: f64,
    pub min_resolved_feature_m: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub enum DevelopmentError {
    Schema(SchemaError),
    Budget {
        resource: &'static str,
        limit: usize,
        attempted: usize,
    },
    ResolutionBudget {
        resource: &'static str,
        limit: f64,
        attempted: f64,
    },
}
impl std::fmt::Display for DevelopmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DevelopmentError {}
impl From<SchemaError> for DevelopmentError {
    fn from(e: SchemaError) -> Self {
        Self::Schema(e)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "rewrite", rename_all = "snake_case", deny_unknown_fields)]
pub enum Rewrite {
    AddModule {
        module: Module,
        edge_from: Option<u32>,
        edge_from_port: Option<String>,
        edge_to_port: Option<String>,
    },
    AddField {
        field: FieldOp,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentRequest {
    pub genome: Genome,
    pub rewrites: Vec<Rewrite>,
    pub mechanism_labels: std::collections::BTreeMap<u32, String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceStep {
    pub ordinal: usize,
    pub operation: String,
    pub subject_before: String,
    pub subject_after: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginTrace {
    pub source_genome: String,
    pub steps: Vec<TraceStep>,
}
/// Staged MPG data, not an evaluated V5-08 material partition or geometry admission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalPartition {
    pub interface: crate::schema::InterfaceRef,
    pub modules: Vec<Module>,
    pub edges: Vec<ModuleEdge>,
    pub fields: Vec<FieldOp>,
    pub process: crate::schema::ProcessChoice,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderSidecar {
    pub labels: Vec<(u32, Option<String>)>,
    pub origin: OriginTrace,
}
/// The only view intended for evaluator adapters. Origin and display labels are not fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JudgeInput {
    pub partition_id: String,
    pub subject: PhysicalPartition,
    pub contract_tags: ContractTags,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractTags {
    pub interface: crate::schema::InterfaceRef,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Developed {
    pub partition_id: String,
    pub partition: PhysicalPartition,
    pub origin: OriginTrace,
    pub render: RenderSidecar,
    pub charged: DevelopmentCost,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentCost {
    pub modules: usize,
    pub edges: usize,
    pub field_nodes: usize,
    pub coefficients: usize,
    pub rewrite_applications: usize,
    pub depth: usize,
}

fn partition(genome: &Genome) -> PhysicalPartition {
    PhysicalPartition {
        interface: genome.interface.clone(),
        modules: genome.modules.clone(),
        edges: genome.edges.clone(),
        fields: genome.fields.clone(),
        process: genome.process.clone(),
    }
}
fn digest<T: Serialize>(v: &T) -> Result<String, DevelopmentError> {
    joil_evidence::identity::scientific_hash(v)
        .map_err(|e| DevelopmentError::Schema(SchemaError::Wire(e)))
}
fn max_depth(genome: &Genome) -> usize {
    let mut d = vec![1usize; genome.modules.len()];
    for (i, _m) in genome.modules.iter().enumerate() {
        for e in genome
            .edges
            .iter()
            .filter(|e| e.relation == EdgeRelation::Construction && e.to_module as usize == i)
        {
            d[i] = d[i].max(d[e.from_module as usize] + 1);
        }
    }
    let module_depth = d.into_iter().max().unwrap_or(0);
    let mut fd = vec![1usize; genome.fields.len()];
    for (i, f) in genome.fields.iter().enumerate() {
        fd[i] = f
            .inputs()
            .iter()
            .map(|j| fd[*j as usize] + 1)
            .max()
            .unwrap_or(1);
    }
    module_depth.max(fd.into_iter().max().unwrap_or(0))
}
fn check(cost: &DevelopmentCost, b: &DevelopmentBudget) -> Result<(), DevelopmentError> {
    for (name, value, limit) in [
        ("modules", cost.modules, b.max_modules),
        ("edges", cost.edges, b.max_edges),
        ("field_nodes", cost.field_nodes, b.max_field_nodes),
        ("coefficients", cost.coefficients, b.max_coefficients),
        (
            "rewrite_applications",
            cost.rewrite_applications,
            b.max_rewrite_applications,
        ),
        ("depth", cost.depth, b.max_depth),
    ] {
        if value > limit {
            return Err(DevelopmentError::Budget {
                resource: name,
                limit,
                attempted: value,
            });
        }
    }
    Ok(())
}
fn check_resolution(genome: &Genome, b: &DevelopmentBudget) -> Result<(), DevelopmentError> {
    if !b.max_spatial_frequency_m_inv.is_finite()
        || b.max_spatial_frequency_m_inv < 0.0
        || !b.min_resolved_feature_m.is_finite()
        || b.min_resolved_feature_m <= 0.0
    {
        return Err(DevelopmentError::Schema(SchemaError::Invalid(
            "field resolution budget",
        )));
    }
    for f in &genome.fields {
        if let FieldOp::Primitive {
            max_spatial_frequency_m_inv,
            min_feature_m,
            ..
        } = f
        {
            if *max_spatial_frequency_m_inv > b.max_spatial_frequency_m_inv {
                return Err(DevelopmentError::ResolutionBudget {
                    resource: "spatial_frequency_m_inv",
                    limit: b.max_spatial_frequency_m_inv,
                    attempted: *max_spatial_frequency_m_inv,
                });
            }
            if *min_feature_m < b.min_resolved_feature_m {
                return Err(DevelopmentError::ResolutionBudget {
                    resource: "minimum_resolved_feature_m",
                    limit: b.min_resolved_feature_m,
                    attempted: *min_feature_m,
                });
            }
        }
    }
    Ok(())
}
pub fn develop(
    mut request: DevelopmentRequest,
    budget: &DevelopmentBudget,
) -> Result<Developed, DevelopmentError> {
    if request.rewrites.len() > budget.max_rewrite_applications {
        return Err(DevelopmentError::Budget {
            resource: "rewrite_applications",
            limit: budget.max_rewrite_applications,
            attempted: request.rewrites.len(),
        });
    }
    request.genome.validate()?;
    check_resolution(&request.genome, budget)?;
    let source_genome = request.genome.semantic_identity()?;
    let mut cost = DevelopmentCost {
        modules: request.genome.modules.len(),
        edges: request.genome.edges.len(),
        field_nodes: request.genome.fields.len(),
        coefficients: request.genome.coefficient_count(),
        rewrite_applications: 0,
        depth: max_depth(&request.genome),
    };
    check(&cost, budget)?;
    let mut steps = Vec::with_capacity(request.rewrites.len());
    for (ordinal, rewrite) in request.rewrites.into_iter().enumerate() {
        let before = digest(&partition(&request.genome))?;
        cost.rewrite_applications = ordinal + 1;
        let operation = match rewrite {
            Rewrite::AddModule {
                module,
                edge_from,
                edge_from_port,
                edge_to_port,
            } => {
                cost.modules += 1;
                if let Some(from) = edge_from {
                    cost.edges += 1;
                    request.genome.edges.push(ModuleEdge {
                        relation: EdgeRelation::Construction,
                        from_module: from,
                        from_port: edge_from_port
                            .ok_or(SchemaError::Invalid("missing source port"))?,
                        to_module: request.genome.modules.len() as u32,
                        to_port: edge_to_port.ok_or(SchemaError::Invalid("missing target port"))?,
                    });
                }
                request.genome.modules.push(module);
                "add_module"
            }
            Rewrite::AddField { field } => {
                cost.field_nodes += 1;
                cost.coefficients += field.coefficient_count();
                request.genome.fields.push(field);
                "add_field"
            }
        };
        request.genome.validate()?;
        cost.depth = max_depth(&request.genome);
        check(&cost, budget)?;
        check_resolution(&request.genome, budget)?;
        let after = digest(&partition(&request.genome))?;
        steps.push(TraceStep {
            ordinal,
            operation: operation.into(),
            subject_before: before,
            subject_after: after,
        });
    }
    let p = partition(&request.genome);
    let partition_id = digest(&p)?;
    let origin = OriginTrace {
        source_genome,
        steps,
    };
    let render = RenderSidecar {
        labels: (0..request.genome.modules.len())
            .map(|i| (i as u32, request.mechanism_labels.get(&(i as u32)).cloned()))
            .collect(),
        origin: origin.clone(),
    };
    Ok(Developed {
        partition_id,
        partition: p,
        origin,
        render,
        charged: cost,
    })
}

impl Developed {
    pub fn judge_input(&self) -> JudgeInput {
        JudgeInput {
            partition_id: self.partition_id.clone(),
            subject: self.partition.clone(),
            contract_tags: ContractTags {
                interface: self.partition.interface.clone(),
            },
        }
    }
}
