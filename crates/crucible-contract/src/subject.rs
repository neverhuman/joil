//! Physical artifact view. Provenance/feature names are deliberately absent.
use crate::ContractError;
use crucible_units::{CoordinateFrame, FrameTransform, Metres};
use joil_evidence::claims::Registry;
use joil_evidence::identity::{is_hash, scientific_hash, strict_decode, ContentRef};
use joil_evidence::observation::Finite;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

macro_rules! digest_type {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            pub fn parse(value: &str) -> Result<Self, ContractError> {
                if !is_hash(value) {
                    return Err(ContractError::InvalidDigest);
                }
                Ok(Self(value.into()))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                Self::parse(&String::deserialize(d)?).map_err(serde::de::Error::custom)
            }
        }
    };
}
digest_type!(BlobDigest);
digest_type!(SemanticId);
impl BlobDigest {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(ContentRef::from_bytes("blob", "1", bytes).sha256)
    }
    pub fn reference(&self, namespace: &str) -> ContentRef {
        ContentRef {
            id: format!("{namespace}:{}", self.0),
            revision: "1".into(),
            sha256: self.0.clone(),
        }
    }
}
impl SemanticId {
    pub fn of<T: Serialize>(value: &T) -> Result<Self, ContractError> {
        scientific_hash(value)
            .map(Self)
            .map_err(ContractError::Wire)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    Geometry,
    SurfaceMesh,
    VolumeMesh,
    SolverCase,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactFormat {
    TriangleSurfaceJson,
    Step,
    NativeVolumeMesh,
    NativeSolverCase,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LengthUnit {
    Metre,
    Millimetre,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub role: ArtifactRole,
    pub format: ArtifactFormat,
    pub blob: BlobDigest,
    pub bytes: u64,
    pub length_unit: LengthUnit,
    pub approximation_bound_m: Metres,
    pub acceptance: BlobDigest,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactAcceptance {
    pub artifact: BlobDigest,
    pub format: ArtifactFormat,
    pub length_unit: LengthUnit,
    pub approximation_bound_m: Metres,
    pub interface_contract: BlobDigest,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortPurpose {
    Inlet,
    Outlet,
    Support,
    FixedInterface,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Port {
    pub tag: u16,
    pub purpose: PortPurpose,
    pub frame: CoordinateFrame,
    pub centre_m: [Metres; 3],
    pub area_m2: Finite,
    pub outward_normal: [Finite; 3],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenRegion {
    pub tag: u16,
    pub boundary: BlobDigest,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum MaterialState {
    Identified { passport: BlobDigest },
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum RoughnessState {
    Characterized {
        sand_grain_m: Metres,
        source: BlobDigest,
    },
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoughnessRecord {
    pub sand_grain_m: Metres,
    pub interface_contract: BlobDigest,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectBundle {
    pub schema_version: u32,
    pub artifacts: Vec<ArtifactRef>,
    pub transform: FrameTransform,
    pub material: MaterialState,
    pub roughness: RoughnessState,
    pub interface_contract: BlobDigest,
    pub ports: Vec<Port>,
    pub frozen_regions: Vec<FrozenRegion>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceContract {
    pub ports: Vec<Port>,
    pub frozen_regions: Vec<FrozenRegion>,
    pub spatial_tolerance_m: Metres,
}

impl SubjectBundle {
    pub fn dependencies(&self) -> BTreeSet<BlobDigest> {
        let mut refs = BTreeSet::from([self.interface_contract.clone()]);
        for a in &self.artifacts {
            refs.extend([a.blob.clone(), a.acceptance.clone()]);
        }
        for r in &self.frozen_regions {
            refs.insert(r.boundary.clone());
        }
        if let MaterialState::Identified { passport } = &self.material {
            refs.insert(passport.clone());
        }
        if let RoughnessState::Characterized { source, .. } = &self.roughness {
            refs.insert(source.clone());
        }
        refs
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, ContractError> {
        let value: Self = strict_decode(bytes).map_err(ContractError::Wire)?;
        value.check_structure()?;
        Ok(value)
    }
    pub fn identity(&self) -> Result<SemanticId, ContractError> {
        self.check_structure()?;
        SemanticId::of(&("crucible.subject/1", self))
    }
    pub fn check_structure(&self) -> Result<(), ContractError> {
        if self.schema_version != 1
            || self.artifacts.len() < 2
            || self.ports.is_empty()
            || self.frozen_regions.is_empty()
        {
            return Err(ContractError::InvalidSubject("missing_physical_contract"));
        }
        self.transform.check()?;
        let roles: BTreeSet<_> = self.artifacts.iter().map(|a| a.role).collect();
        if roles.len() != self.artifacts.len()
            || !roles.contains(&ArtifactRole::Geometry)
            || !roles.contains(&ArtifactRole::SurfaceMesh)
        {
            return Err(ContractError::InvalidSubject("artifact_roles"));
        }
        for a in &self.artifacts {
            if a.bytes == 0 || a.approximation_bound_m.get() <= 0.0 {
                return Err(ContractError::InvalidSubject(
                    "missing_length_or_approximation_bound",
                ));
            }
            if a.format == ArtifactFormat::TriangleSurfaceJson && a.length_unit != LengthUnit::Metre
            {
                return Err(ContractError::InvalidSubject("si_surface_unit_mismatch"));
            }
            let compatible = match a.role {
                ArtifactRole::Geometry => matches!(
                    a.format,
                    ArtifactFormat::TriangleSurfaceJson | ArtifactFormat::Step
                ),
                ArtifactRole::SurfaceMesh => a.format == ArtifactFormat::TriangleSurfaceJson,
                ArtifactRole::VolumeMesh => a.format == ArtifactFormat::NativeVolumeMesh,
                ArtifactRole::SolverCase => a.format == ArtifactFormat::NativeSolverCase,
            };
            if !compatible {
                return Err(ContractError::InvalidSubject(
                    "artifact_role_format_mismatch",
                ));
            }
        }
        if self.artifacts.windows(2).any(|a| a[0].role >= a[1].role)
            || self.ports.windows(2).any(|p| p[0].tag >= p[1].tag)
            || self.frozen_regions.windows(2).any(|p| p[0].tag >= p[1].tag)
        {
            return Err(ContractError::InvalidSubject(
                "unordered_or_duplicate_contract_tags",
            ));
        }
        for p in &self.ports {
            if p.frame != self.transform.to
                || p.area_m2.get() <= 0.0
                || !p
                    .outward_normal
                    .iter()
                    .all(|v| v.get().is_finite() && !(v.get() == 0.0 && v.get().is_sign_negative()))
            {
                return Err(ContractError::InvalidSubject("port_units_frame_or_area"));
            }
            if (p
                .outward_normal
                .iter()
                .map(|v| v.get().powi(2))
                .sum::<f64>()
                - 1.0)
                .abs()
                > 2e-12
            {
                return Err(ContractError::InvalidSubject("port_normal"));
            }
        }
        if let RoughnessState::Characterized { sand_grain_m, .. } = &self.roughness {
            if sand_grain_m.get() < 0.0 {
                return Err(ContractError::InvalidSubject("negative_roughness"));
            }
        }
        Ok(())
    }
    pub fn check_interface(&self, expected: &InterfaceContract) -> Result<(), ContractError> {
        self.check_structure()?;
        let tol = expected.spatial_tolerance_m.get();
        if tol <= 0.0
            || self.ports.len() != expected.ports.len()
            || self.frozen_regions != expected.frozen_regions
        {
            return Err(ContractError::InterfaceMismatch);
        }
        for (actual, want) in self.ports.iter().zip(&expected.ports) {
            if actual.tag != want.tag
                || actual.purpose != want.purpose
                || actual.frame != want.frame
                || actual.area_m2 != want.area_m2
                || actual.outward_normal != want.outward_normal
                || actual
                    .centre_m
                    .iter()
                    .zip(want.centre_m)
                    .any(|(a, b)| (a.get() - b.get()).abs() > tol)
            {
                return Err(ContractError::InterfaceMismatch);
            }
        }
        if self
            .artifacts
            .iter()
            .any(|a| a.approximation_bound_m.get() > tol)
        {
            return Err(ContractError::InterfaceMismatch);
        }
        Ok(())
    }
    /// Exact byte closure and authenticated acceptance/interface reviews. This
    /// does not establish independent geometry or physical qualification.
    pub fn verify_artifacts(
        &self,
        registry: &Registry,
        requirements: &ContentRef,
        read: impl Fn(&BlobDigest) -> Result<Vec<u8>, ContractError>,
    ) -> Result<VerifiedSubject, ContractError> {
        self.check_structure()?;
        let interface_bytes = registry
            .checked_artifact(
                &self.interface_contract.reference("v5.interface"),
                std::slice::from_ref(requirements),
            )
            .map_err(ContractError::Wire)?;
        let interface: InterfaceContract =
            strict_decode(interface_bytes).map_err(ContractError::Wire)?;
        self.check_interface(&interface)?;
        for region in &self.frozen_regions {
            if BlobDigest::of_bytes(&read(&region.boundary)?) != region.boundary {
                return Err(ContractError::ArtifactMismatch);
            }
        }
        for a in &self.artifacts {
            let bytes = read(&a.blob)?;
            if bytes.len() as u64 != a.bytes || BlobDigest::of_bytes(&bytes) != a.blob {
                return Err(ContractError::ArtifactMismatch);
            }
            let bytes = registry
                .checked_artifact(
                    &a.acceptance.reference("v5.artifact-acceptance"),
                    std::slice::from_ref(requirements),
                )
                .map_err(ContractError::Wire)?;
            let receipt: ArtifactAcceptance = strict_decode(bytes).map_err(ContractError::Wire)?;
            if receipt.artifact != a.blob
                || receipt.format != a.format
                || receipt.length_unit != a.length_unit
                || receipt.approximation_bound_m != a.approximation_bound_m
                || receipt.interface_contract != self.interface_contract
            {
                return Err(ContractError::ArtifactMismatch);
            }
        }
        for r in match &self.material {
            MaterialState::Identified { passport } => vec![passport],
            MaterialState::Unknown => vec![],
        } {
            registry
                .checked_artifact(
                    &r.reference("v5.material"),
                    std::slice::from_ref(requirements),
                )
                .map_err(ContractError::Wire)?;
        }
        if let RoughnessState::Characterized {
            sand_grain_m,
            source,
        } = &self.roughness
        {
            let bytes = registry
                .checked_artifact(
                    &source.reference("v5.roughness"),
                    std::slice::from_ref(requirements),
                )
                .map_err(ContractError::Wire)?;
            let record: RoughnessRecord = strict_decode(bytes).map_err(ContractError::Wire)?;
            if record.sand_grain_m != *sand_grain_m
                || record.interface_contract != self.interface_contract
            {
                return Err(ContractError::ArtifactMismatch);
            }
        }
        Ok(VerifiedSubject {
            bundle: self.clone(),
            identity: self.identity()?,
        })
    }
}
/// Cannot be deserialized or constructed by a submitted worker payload.
pub struct VerifiedSubject {
    bundle: SubjectBundle,
    identity: SemanticId,
}
impl VerifiedSubject {
    pub fn bundle(&self) -> &SubjectBundle {
        &self.bundle
    }
    pub fn identity(&self) -> &SemanticId {
        &self.identity
    }
}

/// Minimal canonical physical surface for small contract fixtures. Large/native
/// formats have separate qualified readers; this is not a mesh-quality oracle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriangleSurface {
    pub vertices_m: Vec<[Metres; 3]>,
    pub triangles: Vec<[u32; 3]>,
}
impl TriangleSurface {
    pub fn from_json(bytes: &[u8]) -> Result<Self, ContractError> {
        let value: Self = strict_decode(bytes).map_err(ContractError::Wire)?;
        if value.vertices_m.is_empty()
            || value.triangles.is_empty()
            || value
                .triangles
                .iter()
                .flatten()
                .any(|&i| i as usize >= value.vertices_m.len())
        {
            return Err(ContractError::InvalidSubject("surface_indices"));
        }
        Ok(value)
    }
}
