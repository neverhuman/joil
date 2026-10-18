//! Strict, label-independent MPG records. This crate does not grant geometry admission.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceRef {
    pub schema: String,
    pub semantic_sha256: String,
}
impl InterfaceRef {
    pub fn validate(&self) -> Result<(), SchemaError> {
        if self.schema.is_empty()
            || self.semantic_sha256.len() != 64
            || !self.semantic_sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(SchemaError::Invalid("interface reference"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InnovationId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Fluid,
    Solid,
    Porous,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedPort {
    pub name: String,
    pub domain: Domain,
    pub section: String,
    pub frame: String,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    Chamber,
    Sweep,
    Junction,
    Insert,
    Vane,
    Shell,
    Rib,
    ResolvedLattice,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub innovation: InnovationId,
    pub kind: ModuleKind,
    pub ports: Vec<TypedPort>,
    pub field_roots: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleEdge {
    pub relation: EdgeRelation,
    pub from_module: u32,
    pub from_port: String,
    pub to_module: u32,
    pub to_port: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRelation {
    Construction,
    FluidFlow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "choice", deny_unknown_fields)]
pub enum ProcessChoice {
    Frozen { passport_sha256: String },
    Unspecified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    X,
    Y,
    Z,
}

/// V5-08 preview-aligned arithmetic. High-frequency fields and folding warps are
/// deliberately not representable in the admitted vocabulary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldOp {
    Coordinate {
        axis: Axis,
    },
    Constant {
        value: f64,
    },
    Add {
        a: u32,
        b: u32,
    },
    Subtract {
        a: u32,
        b: u32,
    },
    Multiply {
        a: u32,
        b: u32,
    },
    Minimum {
        a: u32,
        b: u32,
    },
    Maximum {
        a: u32,
        b: u32,
    },
    SquareRoot {
        a: u32,
    },
    Absolute {
        a: u32,
    },
    Sine {
        a: u32,
    },
    Cosine {
        a: u32,
    },
    Primitive {
        kind: ModuleKind,
        inputs: Vec<u32>,
        coefficients: Vec<f64>,
        max_spatial_frequency_m_inv: f64,
        min_feature_m: f64,
    },
    NeuralField {
        weights: Vec<f64>,
    },
}
impl FieldOp {
    pub fn inputs(&self) -> Vec<u32> {
        match self {
            Self::Coordinate { .. } | Self::Constant { .. } | Self::NeuralField { .. } => vec![],
            Self::Add { a, b }
            | Self::Subtract { a, b }
            | Self::Multiply { a, b }
            | Self::Minimum { a, b }
            | Self::Maximum { a, b } => vec![*a, *b],
            Self::SquareRoot { a }
            | Self::Absolute { a }
            | Self::Sine { a }
            | Self::Cosine { a } => vec![*a],
            Self::Primitive { inputs, .. } => inputs.clone(),
        }
    }
    pub fn coefficient_count(&self) -> usize {
        match self {
            Self::Constant { .. } => 1,
            Self::Primitive { coefficients, .. } => coefficients.len(),
            Self::NeuralField { weights } => weights.len(),
            _ => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Genome {
    pub schema_version: u32,
    pub interface: InterfaceRef,
    pub modules: Vec<Module>,
    pub edges: Vec<ModuleEdge>,
    pub fields: Vec<FieldOp>,
    pub process: ProcessChoice,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaError {
    Wire(String),
    Invalid(&'static str),
    Unsupported(&'static str),
}
impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SchemaError {}

impl Genome {
    pub fn validate(&self) -> Result<(), SchemaError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(SchemaError::Unsupported("genome schema version"));
        }
        self.interface.validate()?;
        let mut innovations = BTreeSet::new();
        for m in &self.modules {
            if !innovations.insert(m.innovation) {
                return Err(SchemaError::Invalid("duplicate innovation id"));
            }
            let mut names = BTreeSet::new();
            for p in &m.ports {
                if p.name.is_empty() || !names.insert(&p.name) {
                    return Err(SchemaError::Invalid("duplicate/empty port"));
                }
            }
            for root in &m.field_roots {
                if *root as usize >= self.fields.len() {
                    return Err(SchemaError::Invalid("field root out of range"));
                }
            }
        }
        let mut pairs = BTreeSet::new();
        for e in &self.edges {
            let from = self
                .modules
                .get(e.from_module as usize)
                .ok_or(SchemaError::Invalid("edge source"))?;
            let to = self
                .modules
                .get(e.to_module as usize)
                .ok_or(SchemaError::Invalid("edge target"))?;
            // Construction references are ordered and acyclic. Fluid transport may recirculate.
            if e.relation == EdgeRelation::Construction && e.from_module >= e.to_module {
                return Err(SchemaError::Invalid(
                    "forward/cyclic construction reference",
                ));
            }
            let a = from
                .ports
                .iter()
                .find(|p| p.name == e.from_port)
                .ok_or(SchemaError::Invalid("source port missing"))?;
            let b = to
                .ports
                .iter()
                .find(|p| p.name == e.to_port)
                .ok_or(SchemaError::Invalid("target port missing"))?;
            if a.domain != b.domain
                || a.section != b.section
                || a.frame != b.frame
                || a.unit != b.unit
            {
                return Err(SchemaError::Invalid("incompatible typed ports"));
            }
            if !pairs.insert((
                e.relation,
                e.from_module,
                e.from_port.as_str(),
                e.to_module,
                e.to_port.as_str(),
            )) {
                return Err(SchemaError::Invalid("duplicate module edge"));
            }
        }
        let mut coefficients = 0usize;
        for (i, field) in self.fields.iter().enumerate() {
            if field.inputs().iter().any(|j| *j as usize >= i) {
                return Err(SchemaError::Invalid(
                    "field forward/cyclic evaluation reference",
                ));
            }
            if matches!(field, FieldOp::Constant { value } if !value.is_finite()) {
                return Err(SchemaError::Invalid("nonfinite coefficient"));
            }
            if let FieldOp::Primitive {
                coefficients: c,
                max_spatial_frequency_m_inv,
                min_feature_m,
                ..
            } = field
            {
                if c.iter().any(|x| !x.is_finite())
                    || !max_spatial_frequency_m_inv.is_finite()
                    || *max_spatial_frequency_m_inv < 0.0
                    || !min_feature_m.is_finite()
                    || *min_feature_m <= 0.0
                {
                    return Err(SchemaError::Invalid("primitive bounds"));
                }
            }
            if matches!(field, FieldOp::NeuralField { .. }) {
                return Err(SchemaError::Unsupported(
                    "neural fields lack spectral/Jacobian/resolution proof",
                ));
            }
            coefficients = coefficients
                .checked_add(field.coefficient_count())
                .ok_or(SchemaError::Invalid("coefficient overflow"))?;
        }
        if let ProcessChoice::Frozen { passport_sha256 } = &self.process {
            if passport_sha256.len() != 64
                || !passport_sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(SchemaError::Invalid("process passport digest"));
            }
        }
        Ok(())
    }
    pub fn canonical_json(&self) -> Result<Vec<u8>, SchemaError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|e| SchemaError::Wire(e.to_string()))
    }
    pub fn strict_from_json(bytes: &[u8]) -> Result<Self, SchemaError> {
        let genome: Self =
            joil_evidence::identity::strict_decode(bytes).map_err(SchemaError::Wire)?;
        genome.validate()?;
        if genome.canonical_json()?.as_slice() != bytes {
            return Err(SchemaError::Wire("noncanonical genome JSON".into()));
        }
        Ok(genome)
    }
    pub fn semantic_identity(&self) -> Result<String, SchemaError> {
        self.validate()?;
        joil_evidence::identity::scientific_hash(self).map_err(SchemaError::Wire)
    }
    pub fn coefficient_count(&self) -> usize {
        self.fields.iter().map(FieldOp::coefficient_count).sum()
    }
}

/// Strategy state affects search policy only and can never enter Genome serialization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategyState {
    pub operator_logits: BTreeMap<String, f64>,
    pub step_scales: BTreeMap<String, f64>,
}
