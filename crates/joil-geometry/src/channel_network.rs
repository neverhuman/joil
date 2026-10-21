//! Genuine 3D channel volumes assembled from chambers and swept segments.
//!
//! The output is an implicit field consumed by the shared material partition.
//! This compiler does not certify connectivity, minimum gap, manufacturability,
//! frozen CT-35 interfaces, meshing coverage, or physical performance. Those
//! properties require independent artifact observers and a declared campaign.
use crate::csg::{self, Boolean, BoundedField, Builder};
use crate::{Axis, GeometryError, Op};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_SEGMENTS: usize = 64;

/// Sphere, box, or z-aligned capped cylinder. Positions and dimensions are metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChamberShape {
    Sphere { radius_m: f64 },
    Box { half_extents_m: [f64; 3] },
    Cylinder { radius_m: f64, half_height_m: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chamber {
    pub id: String,
    pub centre_m: [f64; 3],
    pub geometry: ChamberShape,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub id: String,
    pub from_chamber: String,
    pub to_chamber: String,
    /// Interior control points. Endpoints are the referenced chamber centres.
    pub via_m: Vec<[f64; 3]>,
    pub radius_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelNetwork {
    pub schema_version: u32,
    pub chambers: Vec<Chamber>,
    pub channels: Vec<Channel>,
}

#[derive(Clone, Debug)]
pub struct CompiledNetwork {
    pub recipe_sha256: String,
    pub fluid: BoundedField,
    pub segment_count: usize,
    /// Names bind source provenance, not an observed count of physical passages.
    pub declared_channels: Vec<String>,
}

/// Exact capsule zero set in real arithmetic: a circular segment sweep with
/// spherical endcaps. Global real-arithmetic Lipschitz bound one; floating-point
/// point evaluations are still subject to the existing kernel's failure rules.
pub fn capsule(
    start_m: [f64; 3],
    end_m: [f64; 3],
    radius_m: f64,
) -> Result<BoundedField, GeometryError> {
    let bad = || GeometryError::InvalidInput("finite nonzero segment and positive radius required");
    if start_m.iter().chain(&end_m).any(|x| !x.is_finite())
        || !radius_m.is_finite()
        || radius_m <= 0.0
    {
        return Err(bad());
    }
    let delta = std::array::from_fn::<_, 3, _>(|i| end_m[i] - start_m[i]);
    let length = delta[0].hypot(delta[1]).hypot(delta[2]);
    if !length.is_finite() || length <= 0.0 {
        return Err(bad());
    }
    let direction = delta.map(|x| x / length);
    let mut b = Builder::default();
    let p = [Axis::X, Axis::Y, Axis::Z].map(|axis| b.coordinate(axis, start_m[axis_index(axis)]));
    let mut projection = b.constant(0.0);
    for (coordinate, unit) in p.into_iter().zip(direction) {
        let term = b.scale(coordinate, unit);
        projection = b.push(Op::Add(projection, term));
    }
    let zero = b.constant(0.0);
    let extent = b.constant(length);
    let clipped = b.push(Op::Maximum(projection, zero));
    let clipped = b.push(Op::Minimum(clipped, extent));
    let mut residual = [0; 3];
    for i in 0..3 {
        let along = b.scale(clipped, direction[i]);
        residual[i] = b.push(Op::Subtract(p[i], along));
    }
    let distance = b.length(&residual);
    let output = b.offset(distance, -radius_m);
    Ok(BoundedField {
        program: b.finish(output)?,
        lipschitz: Some(1.0),
        scalar_error: 0.0,
        exact_distance: true,
    })
}

fn axis_index(axis: Axis) -> usize {
    match axis {
        Axis::X => 0,
        Axis::Y => 1,
        Axis::Z => 2,
    }
}

fn union(acc: &mut Option<BoundedField>, field: BoundedField) -> Result<(), GeometryError> {
    *acc = Some(match acc.take() {
        Some(previous) => csg::boolean(&previous, &field, Boolean::Union)?,
        None => field,
    });
    Ok(())
}

fn id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
}

impl ChannelNetwork {
    /// Compile actual interior volumes. No authored topology label affects the field.
    /// Self-loops with nondegenerate via points and parallel edges are expressible.
    pub fn compile(&self) -> Result<CompiledNetwork, GeometryError> {
        if self.schema_version != SCHEMA_VERSION
            || self.chambers.is_empty()
            || self.chambers.len() > 32
            || self.channels.len() > 32
        {
            return Err(GeometryError::InvalidInput("channel network schema/size"));
        }
        let mut names = BTreeSet::new();
        let mut chambers = BTreeMap::new();
        let mut fluid = None;
        for c in &self.chambers {
            if !id_ok(&c.id) || !names.insert(c.id.as_str()) {
                return Err(GeometryError::InvalidInput("duplicate/invalid feature id"));
            }
            let field = match c.geometry {
                ChamberShape::Sphere { radius_m } => csg::sphere(c.centre_m, radius_m)?,
                ChamberShape::Box { half_extents_m } => csg::cuboid(c.centre_m, half_extents_m)?,
                ChamberShape::Cylinder {
                    radius_m,
                    half_height_m,
                } => csg::cylinder(c.centre_m, radius_m, half_height_m)?,
            };
            union(&mut fluid, field)?;
            chambers.insert(c.id.as_str(), c);
        }
        let mut segments = 0usize;
        for channel in &self.channels {
            if !id_ok(&channel.id) || !names.insert(channel.id.as_str()) {
                return Err(GeometryError::InvalidInput("duplicate/invalid feature id"));
            }
            if channel.via_m.len() > 16 {
                return Err(GeometryError::InvalidInput("channel waypoint budget"));
            }
            segments += channel.via_m.len() + 1;
            if segments > MAX_SEGMENTS {
                return Err(GeometryError::InvalidInput("channel segment budget"));
            }
            let start = chambers
                .get(channel.from_chamber.as_str())
                .ok_or(GeometryError::InvalidInput("missing source chamber"))?;
            let end = chambers
                .get(channel.to_chamber.as_str())
                .ok_or(GeometryError::InvalidInput("missing target chamber"))?;
            let points: Vec<_> = std::iter::once(start.centre_m)
                .chain(channel.via_m.iter().copied())
                .chain(std::iter::once(end.centre_m))
                .collect();
            for p in points.windows(2) {
                union(&mut fluid, capsule(p[0], p[1], channel.radius_m)?)?;
            }
        }
        let recipe_sha256 =
            joil_evidence::identity::scientific_hash(self).map_err(GeometryError::Backend)?;
        Ok(CompiledNetwork {
            recipe_sha256,
            fluid: fluid.unwrap(),
            segment_count: segments,
            declared_channels: self.channels.iter().map(|c| c.id.clone()).collect(),
        })
    }
}
