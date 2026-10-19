//! Honest, content-bound mesh exports and retention. All destinations are fresh
//! directories; every file uses create_new (D-24). No existing evidence is erased.
//!
//! glTF 2.0: https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html
//! 3MF core: https://github.com/3MFConsortium/spec_core
use crate::surface::{
    bounding_box, cross, det, dot, finite_nonnegative, hash_valid, identity, invalid, mul, norm,
    sha256, sub, unsupported, ArtifactError, BoundaryTag, Feature, Point, Result, TaggedSurface,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path};
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverageCard {
    pub backend: String,
    pub version: String,
    pub compiler: String,
    pub kernel: String,
    pub input_tolerance_m: f64,
    pub supported_features: BTreeSet<Feature>,
    pub admissible_consumers: BTreeSet<String>,
    pub limitations: Vec<String>,
}
impl CoverageCard {
    pub fn admit(&self, surface: &TaggedSurface, consumer: &str) -> Result<()> {
        if self.backend.is_empty()
            || self.version.is_empty()
            || self.compiler != surface.boundary().compiler
            || self.kernel != surface.boundary().kernel
            || !finite_nonnegative(self.input_tolerance_m)
            || surface.audit().deviation_bound_m > self.input_tolerance_m
        {
            return Err(invalid("coverage compiler/kernel/input tolerance"));
        }
        if !self.admissible_consumers.contains(consumer)
            || !surface
                .boundary()
                .active_features
                .is_subset(&self.supported_features)
        {
            return Err(unsupported("active geometry/consumer outside backend coverage; no feature homogenization fallback"));
        }
        Ok(())
    }
}
/// Each source column distributes one extensive quantity to target rows.
/// Nonnegative weights and unit column sums enforce conservative transfer.
#[derive(Clone, Debug, Serialize)]
pub struct LinerReuse {
    pub source_interface_sha256: String,
    pub target_interface_sha256: String,
    pub source_topology_sha256: String,
    pub target_topology_sha256: String,
    pub source_areas_m2: Vec<f64>,
    pub target_areas_m2: Vec<f64>,
    pub weights: Vec<Vec<f64>>,
    pub relative_tolerance: f64,
}
impl LinerReuse {
    pub fn verify(&self) -> Result<String> {
        if [
            &self.source_interface_sha256,
            &self.target_interface_sha256,
            &self.source_topology_sha256,
            &self.target_topology_sha256,
        ]
        .into_iter()
        .any(|h| !hash_valid(h))
            || self.source_interface_sha256 != self.target_interface_sha256
            || self.source_topology_sha256 != self.target_topology_sha256
        {
            return Err(unsupported("frozen liner interface/topology mismatch"));
        }
        if !finite_nonnegative(self.relative_tolerance)
            || self.relative_tolerance > 1e-6
            || self.source_areas_m2.is_empty()
            || self.target_areas_m2.is_empty()
            || self.weights.len() != self.target_areas_m2.len()
            || self
                .source_areas_m2
                .iter()
                .chain(&self.target_areas_m2)
                .any(|a| !a.is_finite() || *a <= 0.0)
            || self.weights.iter().any(|row| {
                row.len() != self.source_areas_m2.len()
                    || row.iter().any(|w| !finite_nonnegative(*w))
            })
        {
            return Err(invalid("conservative mapping dimensions/weights/tolerance"));
        }
        for i in 0..self.source_areas_m2.len() {
            if (self.weights.iter().map(|row| row[i]).sum::<f64>() - 1.0).abs()
                > self.relative_tolerance
            {
                return Err(invalid("nonconservative mapping column"));
            }
        }
        for (row, target_area) in self.weights.iter().zip(&self.target_areas_m2) {
            let area = row
                .iter()
                .zip(&self.source_areas_m2)
                .map(|(w, a)| w * a)
                .sum::<f64>();
            if !area.is_finite()
                || (area - target_area).abs() > self.relative_tolerance * target_area
            {
                return Err(invalid("mapping fails uniform-field/area compatibility"));
            }
        }
        identity(self)
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RigidTransform {
    /// Row-major rotation; translations are always metres.
    pub rotation: [[f64; 3]; 3],
    pub translation_m: Point,
}
impl Default for RigidTransform {
    fn default() -> Self {
        Self {
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation_m: [0.0; 3],
        }
    }
}
impl RigidTransform {
    pub fn validate(&self) -> Result<()> {
        if self
            .rotation
            .iter()
            .flatten()
            .chain(&self.translation_m)
            .any(|v| !v.is_finite())
            || (det(self.rotation[0], self.rotation[1], self.rotation[2]) - 1.0).abs() > 1e-12
            || (0..3).any(|i| {
                (0..3).any(|j| {
                    (dot(self.rotation[i], self.rotation[j]) - if i == j { 1.0 } else { 0.0 }).abs()
                        > 1e-12
                })
            })
        {
            return Err(invalid(
                "export transform must be finite rigid SI; scale/reflection forbidden",
            ));
        }
        Ok(())
    }
    pub fn apply(&self, p: Point) -> Point {
        std::array::from_fn(|i| dot(self.rotation[i], p) + self.translation_m[i])
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum BuildUnit {
    Metre,
    Millimetre,
}
impl BuildUnit {
    fn name(self) -> &'static str {
        match self {
            Self::Metre => "meter",
            Self::Millimetre => "millimeter",
        }
    }
    fn per_metre(self) -> f64 {
        match self {
            Self::Metre => 1.0,
            Self::Millimetre => 1000.0,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum StepLabel {
    Exact,
    TessellatedApproximation,
}
/// No sewn B-rep authority exists in this route. A closed triangulation alone
/// cannot be called exact STEP. Optional STEP remains unsupported explicitly.
pub fn request_step(_surface: &TaggedSurface, _label: StepLabel) -> Result<Vec<u8>> {
    Err(unsupported(
        "STEP requires a qualified sewn B-rep or AP242 tessellated backend; none registered",
    ))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportFile {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    pub units: String,
    pub approximation_bound_m: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportManifest {
    pub schema: String,
    pub partition_sha256: String,
    pub upstream_acceptance_receipt_sha256: String,
    pub boundary_sha256: String,
    pub compiler: String,
    pub kernel: String,
    pub coverage: CoverageCard,
    pub consumer: String,
    pub transform: RigidTransform,
    pub transform_sha256: String,
    pub bounds_sha256: String,
    pub bounding_box_m: [Point; 2],
    pub surface_tolerance_m: f64,
    pub partition_deviation_bound_m: f64,
    pub transform_error_bound_m: f64,
    pub build_unit_error_bound_m: f64,
    pub patches: BTreeMap<String, BoundaryTag>,
    pub patch_stl_paths: BTreeMap<String, String>,
    pub files: Vec<ExportFile>,
    pub scientific_admission: bool,
}
fn bytes_json<T: Serialize>(v: &T) -> Result<Vec<u8>> {
    serde_json::to_vec_pretty(v).map_err(|e| invalid(&e.to_string()))
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}
/// Requires an absent run directory, even if an existing one is empty. The
/// parent is caller-controlled private storage; no overwrite/truncate API exists.
fn fresh_run(path: &Path) -> Result<()> {
    fs::create_dir(path)?;
    Ok(())
}

pub fn export_bundle(
    surface: &TaggedSurface,
    destination: &Path,
    coverage: &CoverageCard,
    consumer: &str,
    transform: RigidTransform,
    unit: BuildUnit,
) -> Result<ExportManifest> {
    coverage.admit(surface, consumer)?;
    transform.validate()?;
    let points = surface
        .boundary()
        .vertices_m
        .iter()
        .map(|p| transform.apply(*p))
        .collect::<Vec<_>>();
    if points.iter().flatten().any(|x| !x.is_finite()) {
        return Err(invalid("export transformed coordinate overflow"));
    }
    let bbox = bounding_box(&points);
    let max_input = surface
        .boundary()
        .vertices_m
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0.0, f64::max);
    let max_output = points.iter().flatten().map(|x| x.abs()).fold(0.0, f64::max);
    let translation = transform
        .translation_m
        .iter()
        .map(|x| x.abs())
        .fold(0.0, f64::max);
    let rigidity_defect = (0..3)
        .flat_map(|i| {
            (0..3).map(move |j| {
                (dot(transform.rotation[i], transform.rotation[j]) - if i == j { 1.0 } else { 0.0 })
                    .abs()
            })
        })
        .fold(0.0, f64::max);
    let transform_error = 64.0 * f64::EPSILON * (max_input + max_output + translation)
        + 4.0 * rigidity_defect * max_input;
    let unit_error = 16.0 * f64::EPSILON * max_output;
    if !transform_error.is_finite() || !unit_error.is_finite() {
        return Err(unsupported("export transform error exponent range"));
    }
    let mut patches = BTreeMap::new();
    for f in &surface.boundary().facets {
        patches.insert(f.tag.patch.clone(), f.tag.clone());
    }
    let glb = glb(surface, &points)?;
    let model = three_mf(surface, &points, unit)?;
    // Precompute and validate all serializations before reserving a destination.
    let mut outputs: Vec<(String, Vec<u8>, String, f64)> = Vec::new();
    let upstream = surface.audit().deviation_bound_m;
    let transformed_bound = upstream + transform_error;
    for (index, name) in patches.keys().enumerate() {
        let mut text = format!("solid patch_{index}\n");
        for f in surface
            .boundary()
            .facets
            .iter()
            .filter(|f| &f.tag.patch == name)
        {
            let [a, b, c] = f.vertices.map(|i| points[i]);
            let n = cross(sub(b, a), sub(c, a));
            let n = mul(n, 1.0 / norm(n));
            text.push_str(&format!(
                " facet normal {} {} {}\n  outer loop\n",
                n[0], n[1], n[2]
            ));
            for p in [a, b, c] {
                text.push_str(&format!("   vertex {} {} {}\n", p[0], p[1], p[2]));
            }
            text.push_str("  endloop\n endfacet\n");
        }
        text.push_str(&format!("endsolid patch_{index}\n"));
        outputs.push((
            format!("patch-{index:04}.stl"),
            text.into_bytes(),
            "metre".into(),
            transformed_bound,
        ));
    }
    outputs.push(("scene.glb".into(), glb.0, "metre".into(), upstream + glb.1));
    outputs.push(("build.3mf".into(), model, unit.name().into(), upstream));
    let files = outputs
        .iter()
        .map(|(path, b, units, error)| ExportFile {
            path: path.clone(),
            sha256: sha256(b),
            bytes: b.len() as u64,
            units: units.clone(),
            approximation_bound_m: *error,
        })
        .collect();
    let manifest = ExportManifest {
        schema: "joil.derived-artifacts.v1-draft".into(),
        partition_sha256: surface.boundary().partition_sha256.clone(),
        upstream_acceptance_receipt_sha256: surface.boundary().acceptance_receipt_sha256.clone(),
        boundary_sha256: surface.boundary_sha256().into(),
        compiler: surface.boundary().compiler.clone(),
        kernel: surface.boundary().kernel.clone(),
        coverage: coverage.clone(),
        consumer: consumer.into(),
        transform,
        transform_sha256: identity(&transform)?,
        bounds_sha256: identity(&(
            surface.policy(),
            surface.audit(),
            transform_error,
            unit_error,
            glb.1,
        ))?,
        bounding_box_m: bbox,
        surface_tolerance_m: surface.policy().length_tolerance_m,
        partition_deviation_bound_m: upstream,
        transform_error_bound_m: transform_error,
        build_unit_error_bound_m: unit_error,
        patch_stl_paths: patches
            .keys()
            .enumerate()
            .map(|(index, name)| (name.clone(), format!("patch-{index:04}.stl")))
            .collect(),
        patches,
        files,
        scientific_admission: false,
    };
    fresh_run(destination)?;
    for (name, bytes, _, _) in outputs {
        write_new(&destination.join(name), &bytes)?;
    }
    write_new(&destination.join("manifest.json"), &bytes_json(&manifest)?)?;
    Ok(manifest)
}

#[path = "export/retention.rs"]
mod retention;
pub use self::retention::*;
