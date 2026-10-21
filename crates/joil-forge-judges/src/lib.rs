//! Genome-blind physics judges for the joil forge lane.
//!
//! This crate scores a *physical description* of a separation plant ([`plant::Plant`]):
//! hydrocyclone cells, coalescer beds and flow splitters wired into a directed network
//! with series stages, parallel cells and recycle loops. It never sees a genome. Every
//! number it produces comes from a reduced-order model whose closure constants carry
//! explicit uncertainty ranges ([`closure::Closure`]). The evidence class of every result
//! is [`EVIDENCE_CLASS`]: an unvalidated screen, not a measurement or a qualified model.
//!
//! Model chain (see `docs/forge/SCIENCE.md`):
//! 1. [`flows`] solves the volumetric network `(I - A) Q = b`, recycles included.
//! 2. [`cyclone`] integrates swirl decay `dΓ/dz = -π c_f Γ² / Q_f` and derives
//!    energy-consistent underflow/overflow pressure drops and dissipation rates.
//! 3. [`pressure`] finds the minimum feed pressure, throttles and pump lifts.
//! 4. [`breakup`] applies Davies/Hinze maximum stable diameters at every
//!    high-dissipation location (inlets, throttles, pumps, channels).
//! 5. [`separation`] marches a radial migration–diffusion equation in the
//!    flow-fraction coordinate `s`, which makes a passive tracer report exactly the
//!    volumetric split `r` by construction.
//! 6. [`network`] propagates size-resolved oil mass through the graph and closes the
//!    oil balance; [`capacity`] searches the largest compliant feed flow at a fixed
//!    pressure budget.

pub mod breakup;
pub mod capacity;
pub mod closure;
pub mod coalescer;
pub mod cyclone;
pub mod evaluation;
pub mod flows;
pub mod fluid;
pub mod math;
pub mod network;
pub mod plant;
pub mod pressure;
pub mod reference;
pub mod separation;
pub mod sizes;
pub mod transfer;

/// Version string bound into every evaluation and run manifest.
pub const MODEL_VERSION: &str = "joil-forge-judges/1.0.0";

/// Evidence class of every number this crate emits.
pub const EVIDENCE_CLASS: &str = "unvalidated_reduced_order_screen";

/// Failure states. Numerical failure is kept distinct from physical infeasibility.
#[derive(Clone, Debug, PartialEq)]
pub enum JudgeError {
    /// The described hardware is not buildable or violates a declared geometric bound.
    Geometry(String),
    /// The described network cannot carry flow (closed loop, no product path, ...).
    Hydraulic(String),
    /// A passive loop needs pressure gain that no declared pump supplies.
    PassiveLoop(String),
    /// Arithmetic did not converge or produced a non-finite value. Not a physical verdict.
    Numerical(String),
    /// The hardware lies outside the declared applicability envelope of the reduced-order
    /// model (e.g. cone angles where swirl would separate). Not a physical verdict either:
    /// the screen simply cannot score it.
    Applicability(String),
    /// Caller supplied an invalid argument (negative flow, unknown property, ...).
    Input(String),
}

impl std::fmt::Display for JudgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JudgeError::Geometry(m) => write!(f, "geometry: {m}"),
            JudgeError::Hydraulic(m) => write!(f, "hydraulic: {m}"),
            JudgeError::PassiveLoop(m) => write!(f, "passive loop: {m}"),
            JudgeError::Numerical(m) => write!(f, "numerical: {m}"),
            JudgeError::Applicability(m) => write!(f, "outside model applicability: {m}"),
            JudgeError::Input(m) => write!(f, "input: {m}"),
        }
    }
}

impl std::error::Error for JudgeError {}

impl JudgeError {
    /// Stable machine-readable category used in run records.
    pub fn kind(&self) -> &'static str {
        match self {
            JudgeError::Geometry(_) => "geometry",
            JudgeError::Hydraulic(_) => "hydraulic",
            JudgeError::PassiveLoop(_) => "passive_loop",
            JudgeError::Numerical(_) => "numerical",
            JudgeError::Applicability(_) => "applicability",
            JudgeError::Input(_) => "input",
        }
    }
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, JudgeError>;

pub(crate) fn finite(name: &str, x: f64) -> Result<f64> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(JudgeError::Numerical(format!("{name} is not finite")))
    }
}
