//! Exact set booleans and explicitly bounded scalar smoothing.
//! `min`/`max` preserve zero-set CSG; they are not generally exact signed distances.
use crate::{Axis, FieldProgram, GeometryError, Op};

#[derive(Clone, Debug)]
pub struct BoundedField {
    pub(crate) program: FieldProgram,
    pub(crate) lipschitz: Option<f64>,
    pub(crate) scalar_error: f64,
    pub(crate) exact_distance: bool,
}
impl BoundedField {
    pub fn program(&self) -> &FieldProgram {
        &self.program
    }
    pub fn lipschitz_bound(&self) -> Option<f64> {
        self.lipschitz
    }
    pub fn scalar_approximation_error(&self) -> f64 {
        self.scalar_error
    }
    pub fn is_exact_distance(&self) -> bool {
        self.exact_distance
    }
    /// Arbitrary validated algebra has no admitted global distance/Lipschitz bound.
    pub fn algebraic(program: FieldProgram) -> Self {
        Self {
            program,
            lipschitz: None,
            scalar_error: 0.0,
            exact_distance: false,
        }
    }
}

#[derive(Default)]
pub(crate) struct Builder {
    pub nodes: Vec<Op>,
}
impl Builder {
    pub fn push(&mut self, op: Op) -> usize {
        let n = self.nodes.len();
        self.nodes.push(op);
        n
    }
    pub fn constant(&mut self, x: f64) -> usize {
        self.push(Op::Constant(x))
    }
    pub fn scale(&mut self, a: usize, x: f64) -> usize {
        let c = self.constant(x);
        self.push(Op::Multiply(a, c))
    }
    pub fn offset(&mut self, a: usize, x: f64) -> usize {
        let c = self.constant(x);
        self.push(Op::Add(a, c))
    }
    pub fn coordinate(&mut self, axis: Axis, centre: f64) -> usize {
        let x = self.push(Op::Coordinate(axis));
        self.offset(x, -centre)
    }
    pub fn length(&mut self, axes: &[usize]) -> usize {
        let mut sum = self.constant(0.0);
        for &a in axes {
            let sq = self.push(Op::Multiply(a, a));
            sum = self.push(Op::Add(sum, sq));
        }
        self.push(Op::SquareRoot(sum))
    }
    pub fn append(&mut self, p: &FieldProgram) -> usize {
        let n = self.nodes.len();
        for op in p.nodes() {
            self.push(match *op {
                Op::Coordinate(a) => Op::Coordinate(a),
                Op::Constant(x) => Op::Constant(x),
                Op::Add(a, b) => Op::Add(a + n, b + n),
                Op::Subtract(a, b) => Op::Subtract(a + n, b + n),
                Op::Multiply(a, b) => Op::Multiply(a + n, b + n),
                Op::Minimum(a, b) => Op::Minimum(a + n, b + n),
                Op::Maximum(a, b) => Op::Maximum(a + n, b + n),
                Op::Atan2(a, b) => Op::Atan2(a + n, b + n),
                Op::SquareRoot(a) => Op::SquareRoot(a + n),
                Op::Absolute(a) => Op::Absolute(a + n),
                Op::Sine(a) => Op::Sine(a + n),
                Op::Cosine(a) => Op::Cosine(a + n),
            });
        }
        n + p.output()
    }
    pub fn finish(self, output: usize) -> Result<FieldProgram, GeometryError> {
        FieldProgram::new(self.nodes, output)
    }
}
fn dimensions(centre: [f64; 3], sizes: &[f64]) -> Result<(), GeometryError> {
    if centre.iter().any(|x| !x.is_finite()) || sizes.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return Err(GeometryError::InvalidInput(
            "finite centre and positive dimensions required",
        ));
    }
    Ok(())
}
fn exact(program: FieldProgram) -> BoundedField {
    BoundedField {
        program,
        lipschitz: Some(1.0),
        scalar_error: 0.0,
        exact_distance: true,
    }
}
pub fn sphere(centre: [f64; 3], radius: f64) -> Result<BoundedField, GeometryError> {
    dimensions(centre, &[radius])?;
    let mut b = Builder::default();
    let x = b.coordinate(Axis::X, centre[0]);
    let y = b.coordinate(Axis::Y, centre[1]);
    let z = b.coordinate(Axis::Z, centre[2]);
    let r = b.length(&[x, y, z]);
    let out = b.offset(r, -radius);
    Ok(exact(b.finish(out)?))
}
pub fn cuboid(centre: [f64; 3], half_extents: [f64; 3]) -> Result<BoundedField, GeometryError> {
    dimensions(centre, &half_extents)?;
    let mut b = Builder::default();
    let zero = b.constant(0.0);
    let mut q = [0; 3];
    let mut positive = [0; 3];
    for (i, axis) in [Axis::X, Axis::Y, Axis::Z].into_iter().enumerate() {
        let a = b.coordinate(axis, centre[i]);
        let a = b.push(Op::Absolute(a));
        q[i] = b.offset(a, -half_extents[i]);
        positive[i] = b.push(Op::Maximum(q[i], zero));
    }
    let outside = b.length(&positive);
    let m = b.push(Op::Maximum(q[0], q[1]));
    let m = b.push(Op::Maximum(m, q[2]));
    let inside = b.push(Op::Minimum(m, zero));
    let out = b.push(Op::Add(outside, inside));
    Ok(exact(b.finish(out)?))
}
pub fn cylinder(
    centre: [f64; 3],
    radius: f64,
    half_height: f64,
) -> Result<BoundedField, GeometryError> {
    dimensions(centre, &[radius, half_height])?;
    let mut b = Builder::default();
    let x = b.coordinate(Axis::X, centre[0]);
    let y = b.coordinate(Axis::Y, centre[1]);
    let z = b.coordinate(Axis::Z, centre[2]);
    let rho = b.length(&[x, y]);
    let radial = b.offset(rho, -radius);
    let az = b.push(Op::Absolute(z));
    let axial = b.offset(az, -half_height);
    let zero = b.constant(0.0);
    let p = b.push(Op::Maximum(radial, zero));
    let q = b.push(Op::Maximum(axial, zero));
    let outside = b.length(&[p, q]);
    let inside = b.push(Op::Maximum(radial, axial));
    let inside = b.push(Op::Minimum(inside, zero));
    let out = b.push(Op::Add(outside, inside));
    Ok(exact(b.finish(out)?))
}

#[derive(Clone, Copy, Debug)]
pub enum Boolean {
    Union,
    Intersection,
    Difference,
}
pub fn boolean(
    a: &BoundedField,
    b: &BoundedField,
    operation: Boolean,
) -> Result<BoundedField, GeometryError> {
    let mut ir = Builder::default();
    let x = ir.append(&a.program);
    let y = ir.append(&b.program);
    let out = match operation {
        Boolean::Union => ir.push(Op::Minimum(x, y)),
        Boolean::Intersection => ir.push(Op::Maximum(x, y)),
        Boolean::Difference => {
            let minus = ir.scale(y, -1.0);
            ir.push(Op::Maximum(x, minus))
        }
    };
    Ok(BoundedField {
        program: ir.finish(out)?,
        lipschitz: a.lipschitz.zip(b.lipschitz).map(|(x, y)| x.max(y)),
        scalar_error: a.scalar_error.max(b.scalar_error),
        exact_distance: false,
    })
}

#[derive(Clone, Copy, Debug)]
pub enum Blend {
    /// Piecewise-linear bevel, scalar deviation at most width/2.
    Chamfer,
    /// Polynomial C1 fillet, scalar deviation at most width/4.
    Fillet,
}
/// Width is in the operands' scalar units. Without a gradient lower bound this
/// scalar error cannot be relabeled as a geometric displacement in metres.
pub fn blend_union(
    a: &BoundedField,
    b: &BoundedField,
    width: f64,
    kind: Blend,
) -> Result<BoundedField, GeometryError> {
    if !width.is_finite() || width <= 0.0 {
        return Err(GeometryError::InvalidInput(
            "positive finite blend width required",
        ));
    }
    let mut ir = Builder::default();
    let x = ir.append(&a.program);
    let y = ir.append(&b.program);
    let min = ir.push(Op::Minimum(x, y));
    let delta = ir.push(Op::Subtract(x, y));
    let delta = ir.push(Op::Absolute(delta));
    let h = ir.scale(delta, -1.0);
    let h = ir.offset(h, width);
    let zero = ir.constant(0.0);
    let h = ir.push(Op::Maximum(h, zero));
    let (correction, extra) = match kind {
        Blend::Chamfer => (ir.scale(h, 0.5), width / 2.0),
        Blend::Fillet => {
            let square = ir.push(Op::Multiply(h, h));
            (ir.scale(square, 0.25 / width), width / 4.0)
        }
    };
    let out = ir.push(Op::Subtract(min, correction));
    let error = (a.scalar_error.max(b.scalar_error) + extra).next_up();
    if !error.is_finite() {
        return Err(GeometryError::NumericalFailure("blend bound overflow"));
    }
    Ok(BoundedField {
        program: ir.finish(out)?,
        lipschitz: a.lipschitz.zip(b.lipschitz).map(|(x, y)| x.max(y)),
        scalar_error: error,
        exact_distance: false,
    })
}
