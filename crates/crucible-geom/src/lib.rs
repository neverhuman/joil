//! Compatibility entry point for geometry observers and mesh backends.
//! Shared types have exactly one implementation in joil-geometry.
pub use joil_geometry::*;

pub mod observe;
pub use observe::volume::Volume as CubicalVolume;
pub use observe::{BoundSurface, BoundVolume, Mesh};
