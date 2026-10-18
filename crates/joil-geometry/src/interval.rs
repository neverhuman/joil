//! Outward-rounded algebraic enclosures and conservative spatial pruning.
//! Transcendentals use full mathematical ranges; no libm accuracy claim is needed.
use crate::{Axis, EvaluationBudget, FieldProgram, GeometryError, ImplicitEval, Op, Point3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub(crate) lo: f64,
    pub(crate) hi: f64,
}
impl Interval {
    pub fn lower(self) -> f64 {
        self.lo
    }
    pub fn upper(self) -> f64 {
        self.hi
    }
    pub fn new(lo: f64, hi: f64) -> Result<Self, GeometryError> {
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return Err(GeometryError::InvalidInput(
                "finite ordered interval required",
            ));
        }
        Ok(Self { lo, hi })
    }
    pub fn point(x: f64) -> Result<Self, GeometryError> {
        Self::new(x, x)
    }
    pub fn contains(self, x: f64) -> bool {
        x >= self.lo && x <= self.hi
    }
    pub fn width(self) -> f64 {
        self.hi - self.lo
    }
    fn outward(lo: f64, hi: f64) -> Result<Self, GeometryError> {
        Self::new(lo.next_down(), hi.next_up())
            .map_err(|_| GeometryError::Unsupported("interval overflow; refine or reject"))
    }
    pub fn plus(self, b: Self) -> Result<Self, GeometryError> {
        let mut v = Self::outward(self.lo + b.lo, self.hi + b.hi)?;
        if self.lo >= 0.0 && b.lo >= 0.0 {
            v.lo = v.lo.max(0.0);
        }
        if self.hi <= 0.0 && b.hi <= 0.0 {
            v.hi = v.hi.min(0.0);
        }
        Ok(v)
    }
    pub fn minus(self, b: Self) -> Result<Self, GeometryError> {
        self.plus(Self {
            lo: -b.hi,
            hi: -b.lo,
        })
    }
    pub fn times(self, b: Self) -> Result<Self, GeometryError> {
        let v = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        let mut out = Self::outward(
            v.iter().copied().fold(f64::INFINITY, f64::min),
            v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )?;
        if self.lo >= 0.0 && b.lo >= 0.0 || self.hi <= 0.0 && b.hi <= 0.0 {
            out.lo = out.lo.max(0.0);
        }
        Ok(out)
    }
    pub fn square(self) -> Result<Self, GeometryError> {
        let hi = self.lo.abs().max(self.hi.abs()).powi(2);
        let lo = if self.contains(0.0) {
            0.0
        } else {
            self.lo.abs().min(self.hi.abs()).powi(2)
        };
        let mut out = Self::outward(lo, hi)?;
        out.lo = out.lo.max(0.0);
        Ok(out)
    }
    pub fn sqrt(self) -> Result<Self, GeometryError> {
        if self.lo < 0.0 {
            return Err(GeometryError::Unsupported(
                "sqrt domain not proven over cell",
            ));
        }
        let mut out = Self::outward(self.lo.sqrt(), self.hi.sqrt())?;
        out.lo = out.lo.max(0.0);
        Ok(out)
    }
    pub fn absolute(self) -> Self {
        Self {
            lo: if self.contains(0.0) {
                0.0
            } else {
                self.lo.abs().min(self.hi.abs())
            },
            hi: self.lo.abs().max(self.hi.abs()),
        }
    }
    fn intersection(self, b: Self) -> Result<Self, GeometryError> {
        Self::new(self.lo.max(b.lo), self.hi.min(b.hi))
            .map_err(|_| GeometryError::NumericalFailure("inconsistent interval enclosures"))
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub axes: [Interval; 3],
}
impl Cell {
    pub fn new(lower: [f64; 3], upper: [f64; 3]) -> Result<Self, GeometryError> {
        Ok(Self {
            axes: [
                Interval::new(lower[0], upper[0])?,
                Interval::new(lower[1], upper[1])?,
                Interval::new(lower[2], upper[2])?,
            ],
        })
    }
    pub fn volume_bounds(self) -> Result<Interval, GeometryError> {
        let mut v = Interval::point(1.0)?;
        for a in self.axes {
            let width = Interval::point(a.hi)?.minus(Interval::point(a.lo)?)?;
            v = v.times(Interval {
                lo: width.lo.max(0.0),
                hi: width.hi,
            })?;
        }
        Ok(v)
    }
    pub fn split(self) -> Result<[Self; 2], GeometryError> {
        let i = (0..3)
            .max_by(|&a, &b| self.axes[a].width().total_cmp(&self.axes[b].width()))
            .unwrap();
        let a = self.axes[i];
        let mid = a.lo / 2.0 + a.hi / 2.0;
        if mid <= a.lo || mid >= a.hi {
            return Err(GeometryError::Unsupported(
                "cell cannot refine at floating resolution",
            ));
        }
        let mut left = self;
        let mut right = self;
        left.axes[i].hi = mid;
        right.axes[i].lo = mid;
        Ok([left, right])
    }
}

/// Affine form c + sum(a_i*x_i) + remainder, with interval coefficients to
/// enclose their rounding. Nonlinear operations fall back to a proven enclosure.
#[derive(Clone, Copy, Debug)]
pub struct AffineForm {
    pub centre: Interval,
    pub coefficients: [Interval; 3],
    pub remainder: Interval,
}
impl AffineForm {
    fn constant(x: f64) -> Self {
        let z = Interval { lo: 0.0, hi: 0.0 };
        Self {
            centre: Interval { lo: x, hi: x },
            coefficients: [z; 3],
            remainder: z,
        }
    }
    fn enclosure(x: Interval) -> Self {
        let mut a = Self::constant(0.0);
        a.remainder = x;
        a
    }
    fn combine(self, b: Self, subtract: bool) -> Result<Self, GeometryError> {
        let op = |a: Interval, b: Interval| if subtract { a.minus(b) } else { a.plus(b) };
        let mut a = Self::constant(0.0);
        a.centre = op(self.centre, b.centre)?;
        a.remainder = op(self.remainder, b.remainder)?;
        for i in 0..3 {
            a.coefficients[i] = op(self.coefficients[i], b.coefficients[i])?;
        }
        Ok(a)
    }
    fn scaled(self, s: f64) -> Result<Self, GeometryError> {
        let s = Interval::point(s)?;
        let mut a = self;
        a.centre = a.centre.times(s)?;
        a.remainder = a.remainder.times(s)?;
        for c in &mut a.coefficients {
            *c = c.times(s)?;
        }
        Ok(a)
    }
    pub fn range(self, cell: Cell) -> Result<Interval, GeometryError> {
        let mut v = self.centre.plus(self.remainder)?;
        for i in 0..3 {
            v = v.plus(self.coefficients[i].times(cell.axes[i])?)?;
        }
        Ok(v)
    }
}
pub fn evaluate(
    program: &FieldProgram,
    cell: Cell,
    budget: &mut EvaluationBudget,
) -> Result<Interval, GeometryError> {
    budget.charge(program.nodes().len())?;
    let mut values: Vec<Interval> = Vec::with_capacity(program.nodes().len());
    for op in program.nodes() {
        values.push(range_op(op, &values, cell)?);
    }
    Ok(values[program.output()])
}
fn range_op(op: &Op, v: &[Interval], cell: Cell) -> Result<Interval, GeometryError> {
    Ok(match *op {
        Op::Coordinate(a) => {
            cell.axes[match a {
                Axis::X => 0,
                Axis::Y => 1,
                Axis::Z => 2,
            }]
        }
        Op::Constant(x) => Interval::point(x)?,
        Op::Add(a, b) => v[a].plus(v[b])?,
        Op::Subtract(a, b) => {
            if a == b {
                Interval::point(0.0)?
            } else {
                v[a].minus(v[b])?
            }
        }
        Op::Multiply(a, b) => {
            if a == b {
                v[a].square()?
            } else {
                v[a].times(v[b])?
            }
        }
        Op::Minimum(a, b) => Interval::new(v[a].lo.min(v[b].lo), v[a].hi.min(v[b].hi))?,
        Op::Maximum(a, b) => Interval::new(v[a].lo.max(v[b].lo), v[a].hi.max(v[b].hi))?,
        Op::Absolute(a) => v[a].absolute(),
        Op::SquareRoot(a) => v[a].sqrt()?,
        Op::Sine(_) | Op::Cosine(_) => Interval::new(-1.0, 1.0)?,
        Op::Atan2(_, _) => Interval::new(
            -std::f64::consts::PI.next_up(),
            std::f64::consts::PI.next_up(),
        )?,
    })
}
pub fn evaluate_affine(
    program: &FieldProgram,
    cell: Cell,
    budget: &mut EvaluationBudget,
) -> Result<Interval, GeometryError> {
    // Charge both the natural enclosure and affine propagation.
    budget.charge(
        program
            .nodes()
            .len()
            .checked_mul(2)
            .ok_or(GeometryError::Unsupported("affine budget overflow"))?,
    )?;
    let mut forms: Vec<AffineForm> = Vec::new();
    let mut ranges: Vec<Interval> = Vec::new();
    for op in program.nodes() {
        let natural = range_op(op, &ranges, cell)?;
        let form = match *op {
            Op::Constant(x) => AffineForm::constant(x),
            Op::Coordinate(axis) => {
                let mut a = AffineForm::constant(0.0);
                a.coefficients[match axis {
                    Axis::X => 0,
                    Axis::Y => 1,
                    Axis::Z => 2,
                }] = Interval::point(1.0)?;
                a
            }
            Op::Add(a, b) => forms[a].combine(forms[b], false)?,
            Op::Subtract(a, b) => {
                if a == b {
                    AffineForm::constant(0.0)
                } else {
                    forms[a].combine(forms[b], true)?
                }
            }
            Op::Multiply(a, b) => match (&program.nodes()[a], &program.nodes()[b]) {
                (Op::Constant(s), _) => forms[b].scaled(*s)?,
                (_, Op::Constant(s)) => forms[a].scaled(*s)?,
                _ => AffineForm::enclosure(natural),
            },
            _ => AffineForm::enclosure(natural),
        };
        ranges.push(natural.intersection(form.range(cell)?)?);
        forms.push(form);
    }
    Ok(ranges[program.output()])
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    Inside,
    Outside,
    Unresolved,
}
pub fn classify(range: Interval) -> Sign {
    if range.hi < 0.0 {
        Sign::Inside
    } else if range.lo > 0.0 {
        Sign::Outside
    } else {
        Sign::Unresolved
    }
}

/// Finite-difference diagnostic only. A unit vector is not an error certificate.
pub fn estimate_normal(
    field: &mut impl ImplicitEval,
    point: Point3,
    step_m: f64,
    min_gradient: f64,
    budget: &mut EvaluationBudget,
) -> Result<[f64; 3], GeometryError> {
    if !step_m.is_finite() || step_m <= 0.0 || !min_gradient.is_finite() || min_gradient <= 0.0 {
        return Err(GeometryError::InvalidInput("normal step/threshold"));
    }
    let p = point.coordinates_m();
    let mut gradient = [0.0; 3];
    for i in 0..3 {
        let mut a = p;
        let mut b = p;
        a[i] += step_m;
        b[i] -= step_m;
        if a[i] == p[i] || b[i] == p[i] {
            return Err(GeometryError::Unsupported(
                "normal step below coordinate resolution",
            ));
        }
        let hi = field.evaluate_with_budget(Point3::metres(a)?, budget)?;
        let lo = field.evaluate_with_budget(Point3::metres(b)?, budget)?;
        gradient[i] = (hi - lo) / (2.0 * step_m);
    }
    let norm = gradient[0].hypot(gradient[1]).hypot(gradient[2]);
    if !norm.is_finite() || norm < min_gradient {
        return Err(GeometryError::Unsupported(
            "degenerate or nonfinite estimated gradient",
        ));
    }
    Ok(gradient.map(|x| x / norm))
}
