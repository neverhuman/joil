//! Artifact-only observations. No IR, genotype, mechanism label or expected delta
//! enters a measurement. Low-level algorithms are diagnostics; accepted entry
//! requires a `VerifiedSubject`, the requested subject ID and exact artifact bytes.
use crucible_contract::subject::{
    ArtifactFormat, ArtifactRef, ArtifactRole, BlobDigest, LengthUnit, SemanticId, VerifiedSubject,
};
use serde::{Deserialize, Serialize};

pub mod clearance;
pub mod distance;
pub mod enactment;
pub mod mesh;
pub mod topology;
pub mod volume;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObserveError {
    Binding,
    Invalid(&'static str),
    Unsupported(&'static str),
    Numerical(&'static str),
}
impl std::fmt::Display for ObserveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ObserveError {}

/// Exact represented triangle surface, in the subject's artifact frame, metres.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Mesh {
    pub vertices_m: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Provenance {
    pub subject: SemanticId,
    pub artifact: BlobDigest,
    pub frame_transform: SemanticId,
    /// Input bound, preserved separately from sampling and arithmetic errors.
    pub approximation_m: f64,
}

pub struct BoundSurface {
    mesh: Mesh,
    provenance: Provenance,
}
impl BoundSurface {
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
    pub fn measure(&self) -> Result<mesh::Measures, ObserveError> {
        mesh::measure(&self.mesh)
    }
    /// Patch indices refer to this exact bound artifact. The manifest error is
    /// applied automatically; callers cannot silently pass zero input error.
    pub fn patch_gap(&self, a:&[usize], b:&[usize])->Result<distance::Gap,ObserveError> {
        if a.is_empty()||b.is_empty()||a.iter().chain(b).any(|&i|i>=self.mesh.triangles.len())
            || a.iter().any(|i|b.contains(i)) {
            return Err(ObserveError::Invalid("opposing patch indices"));
        }
        let patch=|indices:&[usize]|indices.iter().map(|&i|self.mesh.triangles[i].map(|v|self.mesh.vertices_m[v])).collect::<Vec<_>>();
        distance::patch_gap(&patch(a),&patch(b),self.provenance.approximation_m)
    }
}

pub struct BoundVolume {
    volume: volume::Volume,
    provenance: Provenance,
}
impl BoundVolume {
    pub fn volume(&self) -> &volume::Volume {
        &self.volume
    }
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

fn bind<'a>(
    subject: &'a VerifiedSubject,
    expected: &SemanticId,
    digest: &BlobDigest,
    role: ArtifactRole,
    format: ArtifactFormat,
    bytes: &[u8],
) -> Result<&'a ArtifactRef, ObserveError> {
    if subject.identity() != expected {
        return Err(ObserveError::Binding);
    }
    let a = subject
        .bundle()
        .artifacts
        .iter()
        .find(|a| a.role == role)
        .ok_or(ObserveError::Binding)?;
    if &a.blob != digest || BlobDigest::of_bytes(bytes) != *digest || bytes.len() as u64 != a.bytes
    {
        return Err(ObserveError::Binding);
    }
    if a.format != format || a.length_unit != LengthUnit::Metre {
        return Err(ObserveError::Unsupported("artifact format or SI units"));
    }
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(ObserveError::Unsupported("artifact byte budget"));
    }
    Ok(a)
}

pub fn read_surface(
    subject: &VerifiedSubject,
    expected: &SemanticId,
    digest: &BlobDigest,
    bytes: &[u8],
) -> Result<BoundSurface, ObserveError> {
    let a = bind(
        subject,
        expected,
        digest,
        ArtifactRole::SurfaceMesh,
        ArtifactFormat::TriangleSurfaceJson,
        bytes,
    )?;
    let mesh: Mesh = crucible_contract::strict_decode(bytes)
        .map_err(|_| ObserveError::Invalid("surface wire"))?;
    mesh::validate(&mesh)?;
    Ok(BoundSurface {
        mesh,
        provenance: Provenance {
            subject: expected.clone(),
            artifact: digest.clone(),
            frame_transform: SemanticId::of(&subject.bundle().transform).map_err(|_| ObserveError::Binding)?,
            approximation_m: a.approximation_bound_m.get(),
        },
    })
}

/// DRAFT `observer-cubical-volume/1` encoding under NativeVolumeMesh. Final
/// V5-08/10 readers must negotiate this encoding; other native meshes are refused.
pub fn read_volume(
    subject: &VerifiedSubject,
    expected: &SemanticId,
    digest: &BlobDigest,
    bytes: &[u8],
) -> Result<BoundVolume, ObserveError> {
    let a = bind(
        subject,
        expected,
        digest,
        ArtifactRole::VolumeMesh,
        ArtifactFormat::NativeVolumeMesh,
        bytes,
    )?;
    let volume: volume::Volume = crucible_contract::strict_decode(bytes)
        .map_err(|_| ObserveError::Invalid("volume wire"))?;
    volume.validate()?;
    // Tags map to physical contract ports. Their cell-plane intersections must
    // be consistent; the accepted artifact review owns the full patch mapping.
    if volume.ports.len() != subject.bundle().ports.len() {
        return Err(ObserveError::Binding);
    }
    for seed in &volume.ports {
        let port = subject
            .bundle()
            .ports
            .iter()
            .find(|p| p.tag == seed.tag)
            .ok_or(ObserveError::Binding)?;
        let centre = port.centre_m.map(|x| x.get());
        let normal = port.outward_normal.map(|x| x.get());
        for &i in &seed.cells {
            let transform = &subject.bundle().transform;
            let local = volume.centre(i);
            let global = std::array::from_fn(|a| {
                transform.translation_m[a].get() + (0..3).map(|b| transform.rotation[a][b].get()*local[b]).sum::<f64>()
            });
            let distance = dot(sub(global, centre), normal).abs();
            if distance > 3_f64.sqrt() * volume.pitch_m / 2.0 + a.approximation_bound_m.get() {
                return Err(ObserveError::Invalid("port seed outside contract plane"));
            }
            let want = match port.purpose {
                crucible_contract::subject::PortPurpose::Inlet
                | crucible_contract::subject::PortPurpose::Outlet => volume::Cell::Fluid,
                _ => volume::Cell::Solid,
            };
            if volume.cells[i] != want {
                return Err(ObserveError::Invalid("port material"));
            }
        }
    }
    Ok(BoundVolume {
        volume,
        provenance: Provenance {
            subject: expected.clone(),
            artifact: digest.clone(),
            frame_transform: SemanticId::of(&subject.bundle().transform).map_err(|_| ObserveError::Binding)?,
            approximation_m: a.approximation_bound_m.get(),
        },
    })
}

pub(crate) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(crate) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
