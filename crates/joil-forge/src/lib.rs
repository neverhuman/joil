//! joil forge: evolutionary design of multi-stage deoiling cartridges.
//!
//! * [`genome`]: variable-length process-network genome (series, parallel cells, splitters,
//!   reject scavengers, pumped recycles, coalescer beds, free-form wall profiles).
//! * [`develop`]: genome → hardware plant + automatic cartridge layout + geometry gates.
//! * [`score`]: genome-blind physics from `joil-forge-judges`, conservative over closure
//!   samples, combined into one weighted objective.
//! * [`evolve`] / [`run`]: island evolution with a MAP-Elites archive, deterministic and
//!   resumable from atomic manifest-bound checkpoints.
//! * [`cad`] and [`render`]: watertight fluid and printed-part meshes, STL/GLB, PNG renders,
//!   SVG diagrams and an HTML report.
//!
//! Every physical number is a screen result of an unvalidated reduced-order model.

pub mod archive;
pub mod cad;
pub mod campaign;
pub mod checkpoint;
pub mod compact;
pub mod control;
pub mod crossover;
pub mod develop;
pub mod evolve;
pub mod genome;
pub mod layout;
pub mod mutate;
pub mod random;
pub mod render;
pub mod rng;
pub mod routing;
pub mod run;
pub mod score;
pub mod shapes;
