//! Experimental Fidget 0.5.1 CPU prototype. Every point is checked against both
//! its VM and the plain f64 reference; this is diagnostic admission, not a domain
//! error certificate, a speedup claim, or a replacement geometry authority.
use super::{
    Axis, EvaluationBudget, FieldProgram, GeometryError, ImplicitEval, KernelIdentity, Op, Point3,
    ReferenceInterpreter,
};
use fidget::{context::Context, jit::JitShape, shape::EzShape, vm::VmShape};
use std::time::Instant;

type PointEvaluator = Box<dyn FnMut([f32; 3]) -> Result<f32, GeometryError>>;
pub struct FidgetKernel {
    identity: KernelIdentity,
    reference: ReferenceInterpreter,
    vm: PointEvaluator,
    jit: PointEvaluator,
    tolerance: f64,
    pub compilation_nanoseconds: u128,
}
fn f32_input(value: f64) -> Result<f32, GeometryError> {
    let v = value as f32;
    if !v.is_finite() || value != 0.0 && v == 0.0 {
        return Err(GeometryError::Unsupported(
            "f32 coordinate/constant overflow or underflow",
        ));
    }
    Ok(v)
}
fn backend(e: impl std::fmt::Display) -> GeometryError {
    GeometryError::Backend(e.to_string())
}
impl FidgetKernel {
    pub const VERSION: &'static str = "0.5.1";
    pub const MAX_PROTOTYPE_NODES: usize = 1024;
    pub fn compile(
        program: FieldProgram,
        absolute_tolerance: f64,
        budget: &mut EvaluationBudget,
    ) -> Result<Self, GeometryError> {
        if program.nodes().len() > Self::MAX_PROTOTYPE_NODES
            || !absolute_tolerance.is_finite()
            || absolute_tolerance < 0.0
        {
            return Err(GeometryError::Unsupported(
                "candidate prototype size/tolerance envelope",
            ));
        }
        budget.charge(
            program
                .nodes()
                .len()
                .checked_mul(2)
                .ok_or(GeometryError::Unsupported("candidate compilation budget"))?,
        )?;
        let start = Instant::now();
        let mut ctx = Context::new();
        let mut nodes = vec![];
        for op in program.nodes() {
            let n = match *op {
                Op::Coordinate(Axis::X) => ctx.x(),
                Op::Coordinate(Axis::Y) => ctx.y(),
                Op::Coordinate(Axis::Z) => ctx.z(),
                Op::Constant(x) => ctx.constant(f32_input(x)?),
                Op::Add(a, b) => ctx.add(nodes[a], nodes[b]).map_err(backend)?,
                Op::Subtract(a, b) => ctx.sub(nodes[a], nodes[b]).map_err(backend)?,
                Op::Multiply(a, b) => ctx.mul(nodes[a], nodes[b]).map_err(backend)?,
                Op::Minimum(a, b) => ctx.min(nodes[a], nodes[b]).map_err(backend)?,
                Op::Maximum(a, b) => ctx.max(nodes[a], nodes[b]).map_err(backend)?,
                Op::SquareRoot(a) => ctx.sqrt(nodes[a]).map_err(backend)?,
                Op::Absolute(a) => ctx.abs(nodes[a]).map_err(backend)?,
                Op::Sine(a) => ctx.sin(nodes[a]).map_err(backend)?,
                Op::Cosine(a) => ctx.cos(nodes[a]).map_err(backend)?,
                Op::Atan2(a, b) => ctx.atan2(nodes[a], nodes[b]).map_err(backend)?,
            };
            nodes.push(n);
        }
        let root = nodes[program.output()];
        let vm = VmShape::new(&ctx, root).map_err(backend)?;
        let jit = JitShape::new(&ctx, root).map_err(backend)?;
        let vm_tape = vm.ez_point_tape();
        let jit_tape = jit.ez_point_tape();
        let mut vm_eval = VmShape::new_point_eval();
        let mut jit_eval = JitShape::new_point_eval();
        let vm: PointEvaluator = Box::new(move |p| {
            vm_eval
                .eval(&vm_tape, p[0], p[1], p[2])
                .map(|v| v.0)
                .map_err(backend)
        });
        let jit: PointEvaluator = Box::new(move |p| {
            jit_eval
                .eval(&jit_tape, p[0], p[1], p[2])
                .map(|v| v.0)
                .map_err(backend)
        });
        Ok(Self{identity:KernelIdentity{backend:"fidget-vm+jit-checked".into(),version:Self::VERSION.into(),program_sha256:program.identity()?,experimental:true,arithmetic:"f32 candidate guarded by f64 reference at every diagnostic point; no global error certificate".into()},reference:ReferenceInterpreter::compile(program)?,vm,jit,tolerance:absolute_tolerance,compilation_nanoseconds:start.elapsed().as_nanos()})
    }
}
impl ImplicitEval for FidgetKernel {
    fn identity(&self) -> &KernelIdentity {
        &self.identity
    }
    fn evaluate_with_budget(
        &mut self,
        point: Point3,
        budget: &mut EvaluationBudget,
    ) -> Result<f64, GeometryError> {
        // Three source-DAG passes: f64 reference, f32 VM and f32 JIT.
        // This counts logical source operations, not native instructions/time.
        budget.charge(3 * self.reference.program.nodes().len())?;
        self.evaluate(point)
    }
    fn evaluate(&mut self, point: Point3) -> Result<f64, GeometryError> {
        // Validate every source node, including dead nodes optimized away by a
        // backend. Constant folding cannot hide invalid source arithmetic.
        let reference = self.reference.evaluate(point)?;
        let p = point.coordinates_m();
        let p = [f32_input(p[0])?, f32_input(p[1])?, f32_input(p[2])?];
        let vm = (self.vm)(p)? as f64;
        let jit = (self.jit)(p)? as f64;
        if !vm.is_finite() || !jit.is_finite() {
            return Err(GeometryError::NumericalFailure(
                "nonfinite candidate result",
            ));
        }
        if (vm - jit).abs() > self.tolerance
            || (reference - vm).abs() > self.tolerance
            || (reference - jit).abs() > self.tolerance
        {
            return Err(GeometryError::Unsupported(
                "candidate/reference or JIT/VM disagreement; no admission",
            ));
        }
        Ok(jit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn injected_jit_vm_disagreement_blocks_candidate() {
        let p = FieldProgram::new(vec![Op::Coordinate(Axis::X)], 0).unwrap();
        let mut backend = FidgetKernel::compile(p, 1e-6, &mut EvaluationBudget::new(100)).unwrap();
        let mut original = std::mem::replace(&mut backend.jit, Box::new(|_| unreachable!()));
        backend.jit = Box::new(move |p| original(p).map(|value| value + 0.25));
        assert!(matches!(
            backend.evaluate(Point3::metres([1.0, 0.0, 0.0]).unwrap()),
            Err(GeometryError::Unsupported(
                "candidate/reference or JIT/VM disagreement; no admission"
            ))
        ));
    }
}
