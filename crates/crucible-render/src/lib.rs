//! Evidence-bound scene contracts and deterministic plate planning.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DigestRef {
    pub sha256: String,
}
impl DigestRef {
    pub fn parse(value: &str) -> Result<Self, RenderError> {
        if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(RenderError::InvalidDigest);
        }
        Ok(Self {
            sha256: value.to_ascii_lowercase(),
        })
    }
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }
    }
    fn is_valid(&self) -> bool {
        self.sha256.len() == 64
            && self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Measured,
    ValidatedModel,
    UnvalidatedModel,
    Synthetic,
    Illustration,
    Unavailable,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disclosure {
    PublicResearch,
    PrivateGeometry,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub role: String,
    pub digest: DigestRef,
    pub bytes: u64,
    pub disclosure: Disclosure,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub start_s: f64,
    pub end_s: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarScale {
    pub minimum: f64,
    pub maximum: f64,
    pub unit: String,
    pub colormap: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneBundle {
    pub schema_version: u32,
    pub subject: DigestRef,
    pub source_subject: DigestRef,
    pub artifacts: Vec<Artifact>,
    pub observable: String,
    pub unit: String,
    pub frame: crucible_units::CoordinateFrame,
    pub transform: crucible_units::FrameTransform,
    pub window: Window,
    pub mapping: DigestRef,
    pub mask: DigestRef,
    pub scale: ScalarScale,
    pub exaggeration: f64,
    pub recipe: DigestRef,
    pub evidence_class: EvidenceClass,
    pub private_export_approved: bool,
    pub motion_is_simulated: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    InvalidDigest,
    InvalidBundle(&'static str),
    SubjectMismatch,
    UnapprovedPrivateGeometry,
    InvalidScale,
    InvalidFrame,
    CatalogueCoverage(Vec<String>),
    UnavailableEvidence,
}
impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RenderError {}
impl SceneBundle {
    pub fn identity(&self) -> Result<DigestRef, RenderError> {
        self.validate()?;
        let bytes =
            serde_json::to_vec(self).map_err(|_| RenderError::InvalidBundle("serialize"))?;
        Ok(DigestRef::of(&bytes))
    }
    pub fn validate(&self) -> Result<(), RenderError> {
        if self.schema_version != 1
            || !self.subject.is_valid()
            || !self.source_subject.is_valid()
            || self.subject != self.source_subject
            || !self.mapping.is_valid()
            || !self.mask.is_valid()
            || !self.recipe.is_valid()
            || self.artifacts.is_empty()
            || self.observable.trim().is_empty()
            || self.unit.trim().is_empty()
            || self.window.start_s < 0.0
            || self.window.end_s <= self.window.start_s
            || !self.window.start_s.is_finite()
            || !self.window.end_s.is_finite()
        {
            return Err(RenderError::InvalidBundle("identity_time_or_units"));
        }
        if self.artifacts.iter().any(|a| !a.digest.is_valid()) {
            return Err(RenderError::InvalidDigest);
        }
        if self.scale.unit != self.unit
            || !self.scale.minimum.is_finite()
            || !self.scale.maximum.is_finite()
            || self.scale.maximum <= self.scale.minimum
            || self.scale.colormap.trim().is_empty()
        {
            return Err(RenderError::InvalidScale);
        }
        if !self.exaggeration.is_finite() || self.exaggeration <= 0.0 {
            return Err(RenderError::InvalidBundle("exaggeration"));
        }
        self.transform
            .check()
            .map_err(|_| RenderError::InvalidFrame)?;
        if self.transform.from != self.frame || self.transform.to != self.frame {
            return Err(RenderError::InvalidFrame);
        }
        if self.artifacts.iter().any(|a| a.bytes == 0) {
            return Err(RenderError::InvalidBundle("empty_artifact"));
        }
        if !self.private_export_approved
            && self
                .artifacts
                .iter()
                .any(|a| a.disclosure == Disclosure::PrivateGeometry)
        {
            return Err(RenderError::UnapprovedPrivateGeometry);
        }
        if self.evidence_class == EvidenceClass::Unavailable
            && !self.artifacts.iter().any(|a| a.role == "unavailable_card")
        {
            return Err(RenderError::UnavailableEvidence);
        }
        Ok(())
    }
    pub fn validate_verified_subject(
        &self,
        verified_subject: &VerifiedSubject,
    ) -> Result<(), RenderError> {
        self.validate()?;
        if self.subject != verified_subject.digest {
            return Err(RenderError::SubjectMismatch);
        }
        Ok(())
    }
}

/// Validate an adapter input against bytes observed at the adapter boundary.
/// A data card has no geometry; a rendered scene must declare the exact GLB bytes.
pub fn validate_adapter_input(
    bundle_bytes: &[u8],
    glb_path: Option<&Path>,
    source_subject_path: Option<&Path>,
) -> Result<SceneBundle, RenderError> {
    let bundle: SceneBundle = serde_json::from_slice(bundle_bytes)
        .map_err(|_| RenderError::InvalidBundle("bundle_json"))?;
    bundle.validate()?;
    match glb_path {
        Some(path) => {
            let bytes =
                std::fs::read(path).map_err(|_| RenderError::InvalidBundle("glb_unreadable"))?;
            let declared = bundle.artifacts.iter().any(|artifact| {
                artifact.role == "glb"
                    && artifact.digest == DigestRef::of(&bytes)
                    && artifact.bytes == bytes.len() as u64
            });
            if !declared {
                return Err(RenderError::InvalidBundle("glb_artifact_mismatch"));
            }
            let source_path =
                source_subject_path.ok_or(RenderError::InvalidBundle("source_subject_required"))?;
            let source = std::fs::read(source_path)
                .map_err(|_| RenderError::InvalidBundle("source_subject_unreadable"))?;
            bundle.validate_verified_subject(&VerifiedSubject::from_observed_bytes(&source))?;
        }
        None if bundle.evidence_class != EvidenceClass::Unavailable => {
            return Err(RenderError::InvalidBundle("glb_required"));
        }
        None => {}
    }
    Ok(bundle)
}
/// Capability produced only after hashing the observed subject bytes. A digest string alone
/// cannot be passed where this token is required. This proves byte identity only, not qualification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedSubject {
    digest: DigestRef,
}
impl VerifiedSubject {
    pub fn from_observed_bytes(bytes: &[u8]) -> Self {
        Self {
            digest: DigestRef::of(bytes),
        }
    }
    pub fn digest(&self) -> &DigestRef {
        &self.digest
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub id: String,
    pub recipe: String,
    pub evidence_available: bool,
    pub render_completed: bool,
    pub data_card: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeRegistry {
    pub schema_version: u32,
    pub catalogue_ids: Vec<String>,
    pub recipes: BTreeMap<String, Recipe>,
    #[serde(default)]
    pub counts: Option<RegistryCounts>,
    #[serde(default)]
    pub oil_data_policy: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryCounts {
    pub recipe_implemented: usize,
    pub evidence_available: usize,
    pub render_completed: usize,
}
impl RecipeRegistry {
    pub fn validate(&self) -> Result<(), RenderError> {
        if self.schema_version != 1
            || self.catalogue_ids.iter().collect::<BTreeSet<_>>().len() != self.catalogue_ids.len()
        {
            return Err(RenderError::InvalidBundle(
                "registry_version_or_duplicate_ids",
            ));
        }
        let missing: Vec<_> = self
            .catalogue_ids
            .iter()
            .filter(|id| !self.recipes.contains_key(*id))
            .cloned()
            .collect();
        if !missing.is_empty() {
            return Err(RenderError::CatalogueCoverage(missing));
        }
        if self.recipes.iter().any(|(id, r)| {
            id != &r.id
                || r.recipe.is_empty()
                || (r.evidence_available && r.data_card.as_deref().unwrap_or("").is_empty())
        }) {
            return Err(RenderError::InvalidBundle("recipe_registry"));
        }
        if self.oil_data_policy.as_deref()
            != Some("unavailable_data_card_only; authored trajectories are prohibited")
        {
            return Err(RenderError::InvalidBundle("oil_data_policy"));
        }
        if let Some(c) = &self.counts {
            if (
                c.recipe_implemented,
                c.evidence_available,
                c.render_completed,
            ) != self.counts()
            {
                return Err(RenderError::InvalidBundle("recipe_counts_mismatch"));
            }
        }
        Ok(())
    }
    pub fn counts(&self) -> (usize, usize, usize) {
        (
            self.recipes.len(),
            self.recipes
                .values()
                .filter(|r| r.evidence_available)
                .count(),
            self.recipes.values().filter(|r| r.render_completed).count(),
        )
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, RenderError> {
        let r: Self = serde_json::from_slice(bytes)
            .map_err(|_| RenderError::InvalidBundle("registry_json"))?;
        r.validate()?;
        Ok(r)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarSample {
    pub value: f64,
    pub valid: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ColorSample {
    Rgba([f64; 4]),
    Masked,
}
/// Numerical color mapping is independent of pixels, denoisers and Blender post-processing.
pub fn scalar_color(sample: ScalarSample, scale: &ScalarScale) -> Result<ColorSample, RenderError> {
    if !sample.valid {
        return Ok(ColorSample::Masked);
    }
    if !sample.value.is_finite() || scale.maximum <= scale.minimum {
        return Err(RenderError::InvalidScale);
    }
    let t = ((sample.value - scale.minimum) / (scale.maximum - scale.minimum)).clamp(0.0, 1.0);
    Ok(ColorSample::Rgba([t, 0.0, 1.0 - t, 1.0]))
}
pub fn validate_orthonormal_basis(basis: [[f64; 3]; 3]) -> Result<(), RenderError> {
    for i in 0..3 {
        for j in 0..3 {
            let d = (0..3).map(|k| basis[i][k] * basis[j][k]).sum::<f64>();
            if (d - if i == j { 1.0 } else { 0.0 }).abs() > 2e-12 {
                return Err(RenderError::InvalidFrame);
            }
        }
    }
    let d = basis[0][0] * (basis[1][1] * basis[2][2] - basis[1][2] * basis[2][1])
        - basis[0][1] * (basis[1][0] * basis[2][2] - basis[1][2] * basis[2][0])
        + basis[0][2] * (basis[1][0] * basis[2][1] - basis[1][1] * basis[2][0]);
    if (d - 1.0).abs() > 2e-12 {
        return Err(RenderError::InvalidFrame);
    }
    Ok(())
}
pub fn cache_key(
    bundle: &DigestRef,
    recipe: &DigestRef,
    blender_build: &str,
    config: &DigestRef,
) -> Result<DigestRef, RenderError> {
    if blender_build.trim().is_empty() {
        return Err(RenderError::InvalidBundle("build_identity"));
    }
    let v = (
        "crucible.plate-cache/1",
        bundle,
        recipe,
        blender_build,
        config,
    );
    Ok(DigestRef::of(&serde_json::to_vec(&v).map_err(|_| {
        RenderError::InvalidBundle("cache_identity")
    })?))
}
pub fn missing_catalogue_ids(
    catalogue: &BTreeSet<String>,
    mapped: &BTreeSet<String>,
) -> Vec<String> {
    catalogue.difference(mapped).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_units::{CoordinateFrame, FrameTransform, Metres};
    use joil_evidence::observation::Finite;
    fn hash() -> DigestRef {
        DigestRef::parse(&"a".repeat(64)).unwrap()
    }
    fn scene() -> SceneBundle {
        SceneBundle {
            schema_version: 1,
            subject: hash(),
            source_subject: hash(),
            artifacts: vec![Artifact {
                role: "surface".into(),
                digest: hash(),
                bytes: 12,
                disclosure: Disclosure::PublicResearch,
            }],
            observable: "pressure".into(),
            unit: "Pa".into(),
            frame: CoordinateFrame::RightHandedSiZDownstream,
            transform: FrameTransform {
                from: CoordinateFrame::RightHandedSiZDownstream,
                to: CoordinateFrame::RightHandedSiZDownstream,
                translation_m: [Metres::new(0.).unwrap(); 3],
                rotation: [
                    [
                        Finite::new(1.0).unwrap(),
                        Finite::new(0.0).unwrap(),
                        Finite::new(0.0).unwrap(),
                    ],
                    [
                        Finite::new(0.0).unwrap(),
                        Finite::new(1.0).unwrap(),
                        Finite::new(0.0).unwrap(),
                    ],
                    [
                        Finite::new(0.0).unwrap(),
                        Finite::new(0.0).unwrap(),
                        Finite::new(1.0).unwrap(),
                    ],
                ],
            },
            window: Window {
                start_s: 1.,
                end_s: 2.,
            },
            mapping: hash(),
            mask: hash(),
            scale: ScalarScale {
                minimum: 0.,
                maximum: 1.,
                unit: "Pa".into(),
                colormap: "viridis".into(),
            },
            exaggeration: 1.,
            recipe: hash(),
            evidence_class: EvidenceClass::Synthetic,
            private_export_approved: false,
            motion_is_simulated: false,
        }
    }
    #[test]
    fn exact_subject_and_semantics_fail_closed() {
        let s = scene();
        assert!(s.validate().is_ok());
        let other = VerifiedSubject::from_observed_bytes(b"different observed subject bytes");
        assert_eq!(
            s.validate_verified_subject(&other),
            Err(RenderError::SubjectMismatch)
        );
        let mut bad = s.clone();
        bad.window.end_s = bad.window.start_s;
        assert!(bad.validate().is_err());
        let mut bad = s.clone();
        bad.unit = "mm".into();
        assert_eq!(bad.validate(), Err(RenderError::InvalidScale));
        let mut bad = s.clone();
        bad.transform.to = CoordinateFrame::RightHandedSiZUp;
        assert_eq!(bad.validate(), Err(RenderError::InvalidFrame));
    }
    #[test]
    fn private_geometry_requires_approval() {
        let mut s = scene();
        s.artifacts[0].disclosure = Disclosure::PrivateGeometry;
        assert_eq!(s.validate(), Err(RenderError::UnapprovedPrivateGeometry));
    }
    #[test]
    fn a_digest_string_does_not_prove_subject_equivalence() {
        let a = hash();
        let b = hash();
        assert_eq!(a, b);
        let actual = VerifiedSubject::from_observed_bytes(b"subject bytes");
        assert_eq!(
            scene().validate_verified_subject(&actual),
            Err(RenderError::SubjectMismatch)
        );
    }
    #[test]
    fn scalar_mapping_saturates_and_mask_is_preserved() {
        let sc = scene().scale;
        assert_eq!(
            scalar_color(
                ScalarSample {
                    value: 2.,
                    valid: true
                },
                &sc
            )
            .unwrap(),
            ColorSample::Rgba([1., 0., 0., 1.])
        );
        assert_eq!(
            scalar_color(
                ScalarSample {
                    value: 0.5,
                    valid: false
                },
                &sc
            )
            .unwrap(),
            ColorSample::Masked
        );
    }
    #[test]
    fn mirrored_basis_is_rejected() {
        assert_eq!(
            validate_orthonormal_basis([[-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]),
            Err(RenderError::InvalidFrame)
        );
    }
    #[test]
    fn cache_key_changes_with_build() {
        let a = cache_key(&hash(), &hash(), "blender-test-build-a", &hash()).unwrap();
        let b = cache_key(&hash(), &hash(), "blender-test-build-b", &hash()).unwrap();
        assert_ne!(a, b);
    }
    #[test]
    fn adapter_binds_observed_glb_length_digest_and_source_subject() {
        let dir = std::env::temp_dir().join(format!("crucible-render-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let glb_path = dir.join("scene.glb");
        let subject_path = dir.join("subject.bin");
        let glb = b"synthetic GLB bytes";
        let subject = b"synthetic source subject";
        std::fs::write(&glb_path, glb).unwrap();
        std::fs::write(&subject_path, subject).unwrap();
        let mut scene = scene();
        scene.subject = DigestRef::of(subject);
        scene.source_subject = scene.subject.clone();
        scene.artifacts[0].role = "glb".into();
        scene.artifacts[0].digest = DigestRef::of(glb);
        scene.artifacts[0].bytes = glb.len() as u64;
        let bundle = serde_json::to_vec(&scene).unwrap();
        assert!(validate_adapter_input(&bundle, Some(&glb_path), Some(&subject_path)).is_ok());
        std::fs::write(&glb_path, b"changed bytes").unwrap();
        assert!(validate_adapter_input(&bundle, Some(&glb_path), Some(&subject_path)).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
    #[test]
    fn complete_registry_covers_catalogue_and_rejects_a_removed_mapping() {
        let bytes = include_bytes!("../../../recipes/v5/registry.json");
        let mut registry = RecipeRegistry::from_json(bytes).unwrap();
        assert_eq!(registry.counts(), (74, 0, 0));
        registry.recipes.remove("CAD-01");
        assert!(matches!(
            registry.validate(),
            Err(RenderError::CatalogueCoverage(_))
        ));
    }
    #[test]
    fn oil_without_evidence_is_an_unavailable_card_not_trajectory_data() {
        let bytes = include_bytes!("../../../recipes/v5/registry.json");
        let registry = RecipeRegistry::from_json(bytes).unwrap();
        assert!(registry
            .recipes
            .values()
            .all(|r| !r.evidence_available || r.data_card.is_some()));
        let json: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(
            json["oil_data_policy"],
            "unavailable_data_card_only; authored trajectories are prohibited"
        );
        assert!(json.to_string().contains("prohibited"));
    }
}
