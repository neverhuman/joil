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
fn glb(surface: &TaggedSurface, points: &[Point]) -> Result<(Vec<u8>, f64)> {
    let mut binary = Vec::new();
    let mut error: f64 = 0.0;
    let mut quantized = Vec::new();
    for p in points {
        let q = p.map(|x| x as f32);
        if q.iter().any(|x| !x.is_finite()) {
            return Err(unsupported("glTF float32 coordinate overflow"));
        }
        let qm = q.map(f64::from);
        error = error.max(norm(sub(qm, *p)));
        quantized.push(qm);
        for x in q {
            binary.extend_from_slice(&x.to_le_bytes());
        }
    }
    // A display conversion must not silently collapse a valid governing facet.
    for f in &surface.boundary().facets {
        let [a, b, c] = f.vertices.map(|i| quantized[i]);
        let [x, y, z] = f.vertices.map(|i| points[i]);
        if dot(cross(sub(b, a), sub(c, a)), cross(sub(y, x), sub(z, x))) <= 0.0 {
            return Err(unsupported("glTF float32 collapses/reverses a facet"));
        }
    }
    let bbox = bounding_box(&quantized);
    let positions_len = binary.len();
    let mut views =
        vec![json!({"buffer":0,"byteOffset":0,"byteLength":positions_len,"target":34962})];
    let mut accessors = vec![
        json!({"bufferView":0,"componentType":5126,"count":points.len(),"type":"VEC3","min":bbox[0],"max":bbox[1]}),
    ];
    let patch_names: BTreeSet<_> = surface
        .boundary()
        .facets
        .iter()
        .map(|f| f.tag.patch.clone())
        .collect();
    let mut primitives = Vec::new();
    for name in patch_names {
        let facets: Vec<_> = surface
            .boundary()
            .facets
            .iter()
            .filter(|f| f.tag.patch == name)
            .collect();
        let offset = binary.len();
        for f in &facets {
            for i in f.vertices {
                binary.extend_from_slice(&(i as u32).to_le_bytes());
            }
        }
        let index = views.len();
        views.push(
            json!({"buffer":0,"byteOffset":offset,"byteLength":binary.len()-offset,"target":34963}),
        );
        accessors.push(
            json!({"bufferView":index,"componentType":5125,"count":facets.len()*3,"type":"SCALAR"}),
        );
        primitives.push(json!({"attributes":{"POSITION":0},"indices":index,"mode":4,"extras":{"tag":facets[0].tag}}));
    }
    let doc = json!({"asset":{"version":"2.0","generator":"crucible-geom bounded-facet-export/1"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],
        "meshes":[{"primitives":primitives}],"buffers":[{"byteLength":binary.len()}],"bufferViews":views,"accessors":accessors,
        "extras":{"units":"metre","boundary_sha256":surface.boundary_sha256(),"coordinate_axes":"preserved from partition; orientation recorded by transform","float32_error_bound_m":error}});
    let mut json_bytes = serde_json::to_vec(&doc).map_err(|e| invalid(&e.to_string()))?;
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }
    let mut out = b"glTF".to_vec();
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&((12 + 8 + json_bytes.len() + 8 + binary.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4e4f534au32.to_le_bytes());
    out.extend(json_bytes);
    out.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004e4942u32.to_le_bytes());
    out.extend(binary);
    Ok((out, error))
}
fn three_mf(surface: &TaggedSurface, points: &[Point], unit: BuildUnit) -> Result<Vec<u8>> {
    let mut model = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><model unit="{}" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02"><metadata name="Title">partition {}</metadata><resources><object id="1" type="model"><mesh><vertices>"#,
        unit.name(),
        surface.boundary_sha256()
    );
    for p in points {
        let p = p.map(|x| x * unit.per_metre());
        if p.iter().any(|x| !x.is_finite()) {
            return Err(invalid("3MF build unit overflow"));
        }
        model.push_str(&format!(
            r#"<vertex x="{}" y="{}" z="{}"/>"#,
            p[0], p[1], p[2]
        ));
    }
    model.push_str("</vertices><triangles>");
    for f in &surface.boundary().facets {
        let [a, b, c] = f.vertices;
        model.push_str(&format!(r#"<triangle v1="{a}" v2="{b}" v3="{c}"/>"#));
    }
    model.push_str(
        "</triangles></mesh></object></resources><build><item objectid=\"1\"/></build></model>",
    );
    let content=br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>"#;
    let relationships=br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>"#;
    Ok(zip_stored(&[
        ("[Content_Types].xml", content),
        ("_rels/.rels", relationships),
        ("3D/3dmodel.model", model.as_bytes()),
    ]))
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for b in bytes {
        c ^= *b as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ if c & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    !c
}
/// Minimal ZIP store writer, single disk, no ZIP64, fixed 1980 date. The bounded
/// surface route keeps every member far below u32 lengths. CRCs cover raw bytes.
fn zip_stored(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, bytes) in entries {
        let offset = out.len() as u32;
        let crc = crc32(bytes);
        let len = bytes.len() as u32;
        out.extend_from_slice(&0x04034b50u32.to_le_bytes());
        for x in [20u16, 0, 0, 0, 33] {
            out.extend_from_slice(&x.to_le_bytes());
        }
        for x in [crc, len, len] {
            out.extend_from_slice(&x.to_le_bytes());
        }
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(bytes);
        central.extend_from_slice(&0x02014b50u32.to_le_bytes());
        for x in [20u16, 20, 0, 0, 0, 33] {
            central.extend_from_slice(&x.to_le_bytes());
        }
        for x in [crc, len, len] {
            central.extend_from_slice(&x.to_le_bytes());
        }
        for x in [name.len() as u16, 0, 0, 0, 0] {
            central.extend_from_slice(&x.to_le_bytes());
        }
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let offset = out.len() as u32;
    let len = central.len() as u32;
    out.extend(central);
    out.extend_from_slice(&0x06054b50u32.to_le_bytes());
    for x in [0u16, 0, entries.len() as u16, entries.len() as u16] {
        out.extend_from_slice(&x.to_le_bytes());
    }
    for x in [len, offset] {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RetainedRole {
    RawInput,
    Provenance,
    GoverningCfdMesh,
    GoverningFeaMesh,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetainedObject {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    pub role: RetainedRole,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetentionClosure {
    pub objects: Vec<RetainedObject>,
}
impl RetentionClosure {
    /// Verify actual archive bytes before *any* external cleanup decision. A
    /// scalar receipt, symlink, path traversal or missing governing mesh fails.
    /// This method performs no deletion and grants no physical qualification.
    pub fn verify(
        &self,
        archive: &Path,
        required: &BTreeSet<RetainedRole>,
        expected_governing: &BTreeMap<RetainedRole, String>,
    ) -> Result<()> {
        if required.is_empty()
            || !required.contains(&RetainedRole::RawInput)
            || !required.contains(&RetainedRole::Provenance)
            || !required.iter().any(|r| {
                matches!(
                    r,
                    RetainedRole::GoverningCfdMesh | RetainedRole::GoverningFeaMesh
                )
            })
        {
            return Err(invalid(
                "retention requires raw input, provenance and governing mesh",
            ));
        }
        // Expected governing identities come from the accepted subject manifest,
        // independently of this archive receipt. Relabeling a scalar as a mesh
        // cannot satisfy those identities.
        for role in required.iter().filter(|r| {
            matches!(
                r,
                RetainedRole::GoverningCfdMesh | RetainedRole::GoverningFeaMesh
            )
        }) {
            if !expected_governing.get(role).is_some_and(|h| hash_valid(h)) {
                return Err(invalid("accepted governing mesh identity is mandatory"));
            }
        }
        let mut paths = BTreeSet::new();
        let mut roles = BTreeSet::new();
        if fs::symlink_metadata(archive)?.file_type().is_symlink() {
            return Err(invalid("archive symlink"));
        }
        for object in &self.objects {
            let relative = Path::new(&object.path);
            if !hash_valid(&object.sha256)
                || object.bytes == 0
                || relative.components().count() != 1
                || !matches!(relative.components().next(), Some(Component::Normal(_)))
                || !paths.insert(&object.path)
            {
                return Err(invalid("unsafe/duplicate retention object"));
            }
            let path = archive.join(relative);
            let metadata = fs::symlink_metadata(&path)?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() != object.bytes
            {
                return Err(invalid("missing or replaced retention object"));
            }
            if sha256(&fs::read(path)?) != object.sha256 {
                return Err(invalid("retention digest mismatch"));
            }
            if matches!(
                object.role,
                RetainedRole::GoverningCfdMesh | RetainedRole::GoverningFeaMesh
            ) && expected_governing.get(&object.role) != Some(&object.sha256)
            {
                return Err(invalid(
                    "archive does not contain the accepted governing mesh",
                ));
            }
            roles.insert(object.role.clone());
        }
        if !required.is_subset(&roles) {
            return Err(invalid("scalar-only or incomplete retention closure"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExportAttemptOutcome {
    AcceptedArtifact,
    NumericalArtifactFailure(String),
    Unsupported(String),
    InvalidInput(String),
    InfrastructureFailure(String),
}
pub type AttemptOutcome = ExportAttemptOutcome;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub fixture: String,
    pub input_sha256: String,
    pub compile_ns: u128,
    pub mesh_ns: u128,
    pub elapsed_ns: u128,
    pub outcome: AttemptOutcome,
    pub retention: RetentionClosure,
    pub physical_infeasibility: bool,
}
/// Actual stage costs, failure diagnosis and raw inputs survive every attempt.
/// Returned mesh bytes are the governing object, never a scalar stand-in.
/// Corpus definitions and source identities are supplied in retained raw input.
pub fn measure_attempt<C>(
    fixture: &str,
    raw_input: &[u8],
    destination: &Path,
    role: RetainedRole,
    compile: impl FnOnce(&[u8]) -> Result<C>,
    mesh: impl FnOnce(C) -> Result<Vec<u8>>,
) -> Result<AttemptRecord> {
    if fixture.is_empty()
        || raw_input.is_empty()
        || !matches!(
            role,
            RetainedRole::GoverningCfdMesh | RetainedRole::GoverningFeaMesh
        )
    {
        return Err(invalid("benchmark fixture/raw input/governing role"));
    }
    fresh_run(destination)?;
    write_new(&destination.join("raw-input.json"), raw_input)?;
    let start = Instant::now();
    let compile_start = Instant::now();
    let compiled = compile(raw_input);
    let compile_ns = compile_start.elapsed().as_nanos();
    let (output, mesh_ns) = match compiled {
        Ok(compiled) => {
            let mesh_start = Instant::now();
            let output = mesh(compiled);
            (output, mesh_start.elapsed().as_nanos())
        }
        Err(error) => (Err(error), 0),
    };
    let mut objects = vec![RetainedObject {
        path: "raw-input.json".into(),
        sha256: sha256(raw_input),
        bytes: raw_input.len() as u64,
        role: RetainedRole::RawInput,
    }];
    let outcome = match output {
        Ok(bytes) if !bytes.is_empty() => {
            write_new(&destination.join("governing-mesh.json"), &bytes)?;
            objects.push(RetainedObject {
                path: "governing-mesh.json".into(),
                sha256: sha256(&bytes),
                bytes: bytes.len() as u64,
                role,
            });
            AttemptOutcome::AcceptedArtifact
        }
        Ok(_) => AttemptOutcome::NumericalArtifactFailure("empty governing mesh".into()),
        Err(ArtifactError::NumericalFailure(s)) => AttemptOutcome::NumericalArtifactFailure(s),
        Err(ArtifactError::Unsupported(s)) => AttemptOutcome::Unsupported(s),
        Err(ArtifactError::Invalid(s)) => AttemptOutcome::InvalidInput(s),
        Err(ArtifactError::Io(s)) => AttemptOutcome::InfrastructureFailure(s),
    };
    let provenance = bytes_json(
        &json!({"fixture":fixture,"input_sha256":sha256(raw_input),"compile_ns":compile_ns,"mesh_ns":mesh_ns,"outcome":outcome,"physical_infeasibility":false}),
    )?;
    write_new(&destination.join("provenance.json"), &provenance)?;
    objects.push(RetainedObject {
        path: "provenance.json".into(),
        sha256: sha256(&provenance),
        bytes: provenance.len() as u64,
        role: RetainedRole::Provenance,
    });
    let record = AttemptRecord {
        fixture: fixture.into(),
        input_sha256: sha256(raw_input),
        compile_ns,
        mesh_ns,
        elapsed_ns: start.elapsed().as_nanos(),
        outcome,
        retention: RetentionClosure { objects },
        physical_infeasibility: false,
    };
    write_new(&destination.join("attempt.json"), &bytes_json(&record)?)?;
    Ok(record)
}
