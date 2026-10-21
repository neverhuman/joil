//! Bounded implicit geometry. Software evidence never grants physical admission.
pub mod certify;
pub mod chamber_wall;
pub mod channel_network;
pub mod csg;
pub mod field;
pub mod interval;
pub mod partition;
pub use field::{
    Axis, EvaluationBudget, FieldProgram, GeometryError, ImplicitEval, KernelIdentity, Op, Point3,
    ReferenceInterpreter,
};
