//! Validated bounded field DAG. Scalar level sets are not exact-distance certificates.
use serde::{Deserialize, Serialize};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "field/fidget.rs"]
pub mod candidate;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeometryError {
    InvalidInput(&'static str),
    Unsupported(&'static str),
    NumericalFailure(&'static str),
    Backend(String),
}

impl std::fmt::Display for GeometryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for GeometryError {}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    X,
    Y,
    Z,
}

/// Every arithmetic reference points to an earlier node. No executable callbacks,
/// loops, external state or hidden shape fallback enter a field program.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "op",
    content = "args",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Op {
    Coordinate(Axis),
    Constant(f64),
    Add(usize, usize),
    Subtract(usize, usize),
    Multiply(usize, usize),
    Minimum(usize, usize),
    Maximum(usize, usize),
    SquareRoot(usize),
    Absolute(usize),
    Sine(usize),
    Cosine(usize),
    Atan2(usize, usize),
}

impl Op {
    fn inputs(&self) -> Vec<usize> {
        match self {
            Self::Coordinate(_) | Self::Constant(_) => vec![],
            Self::Add(a, b)
            | Self::Subtract(a, b)
            | Self::Multiply(a, b)
            | Self::Minimum(a, b)
            | Self::Maximum(a, b)
            | Self::Atan2(a, b) => vec![*a, *b],
            Self::SquareRoot(a) | Self::Absolute(a) | Self::Sine(a) | Self::Cosine(a) => vec![*a],
        }
    }
}

/// Algebraic level sets use coordinates in metres. Their scalar outputs are not
/// automatically Euclidean signed distances; distance admission is separate.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct FieldProgram {
    schema_version: u32,
    nodes: Vec<Op>,
    output: usize,
}

impl FieldProgram {
    pub const MAX_NODES: usize = 4096;

    pub fn new(mut nodes: Vec<Op>, output: usize) -> Result<Self, GeometryError> {
        if nodes.is_empty() || nodes.len() > Self::MAX_NODES || output >= nodes.len() {
            return Err(GeometryError::InvalidInput("field program size/output"));
        }
        for (index, node) in nodes.iter().enumerate() {
            if node.inputs().iter().any(|input| *input >= index) {
                return Err(GeometryError::InvalidInput(
                    "field DAG forward/cyclic reference",
                ));
            }
            if matches!(node, Op::Constant(x) if !x.is_finite()) {
                return Err(GeometryError::InvalidInput("nonfinite field constant"));
            }
        }
        for node in &mut nodes {
            if let Op::Constant(x) = node {
                if *x == 0.0 {
                    *x = 0.0;
                }
            }
        }
        Ok(Self {
            schema_version: 1,
            nodes,
            output,
        })
    }

    pub fn nodes(&self) -> &[Op] {
        &self.nodes
    }
    pub fn output(&self) -> usize {
        self.output
    }

    pub fn identity(&self) -> Result<String, GeometryError> {
        // Reordering independent nodes does not change structural identity.
        // Unused nodes remain bound: they consume budget and can fail evaluation.
        let mut hashes: Vec<String> = Vec::with_capacity(self.nodes.len());
        for op in &self.nodes {
            let name = match op {
                Op::Coordinate(_) => "coordinate",
                Op::Constant(_) => "constant",
                Op::Add(..) => "add",
                Op::Subtract(..) => "subtract",
                Op::Multiply(..) => "multiply",
                Op::Minimum(..) => "minimum",
                Op::Maximum(..) => "maximum",
                Op::SquareRoot(..) => "square_root",
                Op::Absolute(..) => "absolute",
                Op::Sine(..) => "sine",
                Op::Cosine(..) => "cosine",
                Op::Atan2(..) => "atan2",
            };
            let inputs: Vec<&str> = op.inputs().iter().map(|i| hashes[*i].as_str()).collect();
            let literal = match op {
                Op::Constant(x) => serde_json::json!(x),
                Op::Coordinate(a) => serde_json::json!(a),
                _ => serde_json::Value::Null,
            };
            hashes.push(
                joil_evidence::identity::scientific_hash(&(name, literal, inputs))
                    .map_err(GeometryError::Backend)?,
            );
        }
        let output = hashes[self.output].clone();
        hashes.sort();
        joil_evidence::identity::scientific_hash(&(self.schema_version, hashes, output))
            .map_err(GeometryError::Backend)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, GeometryError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            schema_version: u32,
            nodes: Vec<Op>,
            output: usize,
        }
        let wire: Wire =
            joil_evidence::identity::strict_decode(bytes).map_err(GeometryError::Backend)?;
        if wire.schema_version != 1 {
            return Err(GeometryError::Unsupported("field schema"));
        }
        Self::new(wire.nodes, wire.output)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point3([f64; 3]);
impl Point3 {
    pub fn metres(coordinates: [f64; 3]) -> Result<Self, GeometryError> {
        if coordinates.iter().any(|x| !x.is_finite()) {
            return Err(GeometryError::InvalidInput("nonfinite coordinate"));
        }
        Ok(Self(coordinates))
    }
    pub fn coordinates_m(self) -> [f64; 3] {
        self.0
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct KernelIdentity {
    pub backend: String,
    pub version: String,
    pub program_sha256: String,
    pub experimental: bool,
    pub arithmetic: String,
}

/// Implementations evaluate the same validated program. Unknown arithmetic,
/// unsupported operators and numerical failures are explicit errors.
pub trait ImplicitEval {
    fn identity(&self) -> &KernelIdentity;
    fn evaluate(&mut self, point: Point3) -> Result<f64, GeometryError>;
}

/// Monotonically consumed operation allowance. Failure never refills it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationBudget {
    remaining: usize,
}
impl EvaluationBudget {
    pub fn new(operations: usize) -> Self {
        Self {
            remaining: operations,
        }
    }
    pub fn remaining(&self) -> usize {
        self.remaining
    }
    pub fn charge(&mut self, operations: usize) -> Result<(), GeometryError> {
        if operations > self.remaining {
            self.remaining = 0;
            return Err(GeometryError::Unsupported("evaluation budget exhausted"));
        }
        self.remaining -= operations;
        Ok(())
    }
}

pub struct ReferenceInterpreter {
    program: FieldProgram,
    values: Vec<f64>,
    identity: KernelIdentity,
}

impl ReferenceInterpreter {
    pub fn compile(program: FieldProgram) -> Result<Self, GeometryError> {
        let identity = KernelIdentity {
            backend: "plain-reference".into(),
            version: "1".into(),
            program_sha256: program.identity()?,
            experimental: false,
            arithmetic: "f64 scalar level-set interpreter; not an exact-distance certificate"
                .into(),
        };
        let values = vec![0.0; program.nodes().len()];
        Ok(Self {
            program,
            values,
            identity,
        })
    }
}

impl ImplicitEval for ReferenceInterpreter {
    fn identity(&self) -> &KernelIdentity {
        &self.identity
    }

    fn evaluate(&mut self, point: Point3) -> Result<f64, GeometryError> {
        self.evaluate_with_budget(point, &mut EvaluationBudget::new(FieldProgram::MAX_NODES))
    }
}

impl ReferenceInterpreter {
    pub fn evaluate_with_budget(
        &mut self,
        point: Point3,
        budget: &mut EvaluationBudget,
    ) -> Result<f64, GeometryError> {
        budget.charge(self.program.nodes().len())?;
        let [x, y, z] = point.coordinates_m();
        for (index, op) in self.program.nodes().iter().enumerate() {
            let v = &self.values;
            let result = match *op {
                Op::Coordinate(axis) => match axis {
                    Axis::X => x,
                    Axis::Y => y,
                    Axis::Z => z,
                },
                Op::Constant(value) => value,
                Op::Add(a, b) => v[a] + v[b],
                Op::Subtract(a, b) => v[a] - v[b],
                Op::Multiply(a, b) => v[a] * v[b],
                Op::Minimum(a, b) => v[a].min(v[b]),
                Op::Maximum(a, b) => v[a].max(v[b]),
                Op::Absolute(a) => v[a].abs(),
                Op::Sine(a) => v[a].sin(),
                Op::Cosine(a) => v[a].cos(),
                Op::Atan2(a, b) => v[a].atan2(v[b]),
                Op::SquareRoot(a) => {
                    if v[a] < 0.0 {
                        return Err(GeometryError::NumericalFailure("sqrt outside real domain"));
                    }
                    v[a].sqrt()
                }
            };
            if !result.is_finite() {
                return Err(GeometryError::NumericalFailure(
                    "nonfinite field evaluation",
                ));
            }
            self.values[index] = result;
        }
        Ok(self.values[self.program.output()])
    }
}

/// Diagnostic finite-point agreement. It certifies neither an unsampled domain
/// nor a Euclidean distance, volume, topology, model or physical performance.
pub fn compare_points(
    reference: &mut impl ImplicitEval,
    candidate: &mut impl ImplicitEval,
    points: &[Point3],
    absolute_tolerance: f64,
) -> Result<f64, GeometryError> {
    if points.is_empty()
        || points.len() > 4096
        || !absolute_tolerance.is_finite()
        || absolute_tolerance < 0.0
    {
        return Err(GeometryError::InvalidInput(
            "finite comparison sample/tolerance",
        ));
    }
    if reference.identity().program_sha256 != candidate.identity().program_sha256 {
        return Err(GeometryError::InvalidInput(
            "kernel program identity mismatch",
        ));
    }
    let mut maximum: f64 = 0.0;
    for point in points {
        let a = reference.evaluate(*point)?;
        let b = candidate.evaluate(*point)?;
        // The caller cannot pass a backend that hides failure as NaN.
        if !a.is_finite() || !b.is_finite() {
            return Err(GeometryError::NumericalFailure(
                "nonfinite comparison result",
            ));
        }
        let error = (a - b).abs();
        if !error.is_finite() || error > absolute_tolerance {
            return Err(GeometryError::Unsupported(
                "kernel disagreement; no admission",
            ));
        }
        maximum = maximum.max(error);
    }
    Ok(maximum)
}
