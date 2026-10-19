//! Radical 3D hydrocyclone design language (`DesignProgram`), deterministic SplitMix64 RNG,
//! structure-aware genetic operators, strict gene canonicalization, and mechanism descriptors.
//!
//! Preserves exact baseline identity (`ct35-parametric-v1`), full backward compatibility with
//! `ct35-chamber-wall-field-v1` and `ct35-exotic-head-v2`, and supports five composable 3D layers
//! across seven topology classes while keeping terminal ports (`x = +-60 mm`), overflow (`z = -22.8 mm`),
//! the last 0.5*D2 collar (`z in [151.75, 169.25] mm`), and the full downstream tail (`z >= 169.25 mm`)
//! strictly frozen.

use crate::geometry::{baseline, Dimensions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::f64::consts::PI;

/// Canonical SHA-256 of the committed `baselines/ct35/genome.json` file.
pub fn baseline_file_sha256() -> String {
    let raw = include_bytes!("../../../baselines/ct35/genome.json");
    Sha256::digest(raw)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Deterministic SplitMix64 pseudo-random number generator (matches `jarambit_genotype::Rng`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rng(pub u64);

impl Rng {
    /// SplitMix advances by an odd constant, whose modular inverse recovers the
    /// number of draws modulo 2^64 even when state endpoints wrap.
    pub fn draws_between(before: u64, after: u64) -> u64 {
        let gamma = 0x9E37_79B9_7F4A_7C15_u64;
        let mut inverse = 1_u64;
        for _ in 0..6 {
            inverse = inverse.wrapping_mul(2_u64.wrapping_sub(gamma.wrapping_mul(inverse)));
        }
        after.wrapping_sub(before).wrapping_mul(inverse)
    }

    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    pub fn normal(&mut self) -> f64 {
        let u1 = self.f64().max(1e-12);
        let u2 = self.f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
    }

    pub fn gauss(&mut self, mean: f64, std_dev: f64) -> f64 {
        mean + std_dev * self.normal()
    }
}

pub(crate) fn round4(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

pub(crate) fn round6(x: f64) -> f64 {
    (x * 1_000_000.0).round() / 1_000_000.0
}

/// Layer A: 3D Helical-pitched and superellipse/slot-morphed tangential entry ramp (inboard of `|x| = 42 mm`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InletRamp3D {
    pub axial_drop_mm: f64,
    pub radial_slot_factor: f64,
    #[serde(default = "default_superellipse_n")]
    pub superellipse_exponent: f64,
}

pub(crate) fn default_superellipse_n() -> f64 {
    2.0
}

/// Layer B: Multi-branch phased tangential manifold (`1` = standard 2-port, `2` = 2->4 bifurcated, `3` = 2->6 trifurcated).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InletSplit3D {
    pub enabled: bool,
    pub vane_thickness_mm: f64,
    pub vane_z_fraction: f64,
    #[serde(default = "default_branches_per_port")]
    pub branches_per_port: u32,
    #[serde(default)]
    pub azimuthal_phase_deg: f64,
}

pub(crate) fn default_branches_per_port() -> u32 {
    1
}

/// Layer C: Azimuthal/helical swirl-chamber Fourier wall mode on `z in [20, 65] mm`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChamberMode3D {
    pub azimuthal_order: u32,
    pub cos_mm: f64,
    pub sin_mm: f64,
    pub twist_rad: f64,
}

/// Layer D: C2-clamped multi-stage reducing cone contour on `z in [72, 166] mm`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConeProfile3D {
    pub mode1_mm: f64,
    pub mode2_mm: f64,
    #[serde(default)]
    pub mode3_mm: f64,
}

/// Layer E: Contoured and fluted axial vortex-finder skimmer sleeve around the 1.4 mm overflow bore.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skimmer3D {
    pub length_mm: f64,
    pub outer_radius_mm: f64,
    #[serde(default)]
    pub capture_flutes: u32,
}

/// Wave-2 Layer B2: Internal helical guide-ribbon and swirl-conditioning vane cascade (`z in [12, 60] mm`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelicalRibbon3D {
    pub vane_count: u32,
    pub axial_span_mm: f64,
    pub helical_twist_rad: f64,
    pub thickness_mm: f64,
    pub radial_chord_mm: f64,
}

impl Default for HelicalRibbon3D {
    fn default() -> Self {
        Self {
            vane_count: 0,
            axial_span_mm: 0.0,
            helical_twist_rad: 0.0,
            thickness_mm: 1.2,
            radial_chord_mm: 2.0,
        }
    }
}

impl HelicalRibbon3D {
    pub fn is_inactive(&self) -> bool {
        self.vane_count == 0 || self.axial_span_mm <= 1e-12
    }
}

/// Wave-2 Layer C2: Triply Periodic Minimal Surface (`TPMS`) basis for peripheral annular coalescer/rectifier shells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TpmsBasis {
    #[default]
    None,
    Gyroid,
    SchwarzP,
    Diamond,
    CylindricalGyroid,
}

/// Wave-2 Layer C2: Spatially graded peripheral TPMS lattice shell (`r >= radial_inner_fraction * R_wall(z)`).
/// Leaves the central Rankine vortex and reverse-flow oil core (`r < radial_inner_fraction * R_wall`) 100% unobstructed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TpmsInsert3D {
    pub basis: TpmsBasis,
    pub radial_inner_fraction: f64,
    pub axial_start_mm: f64,
    pub axial_end_mm: f64,
    pub radial_wavelength_mm: f64,
    pub axial_wavelength_mm: f64,
    pub sheet_thickness_mm: f64,
}

/// Alias matching WAVE2_PLAN.md W2-S1 specification for spatially graded TPMS shell fields.
pub type TpmsInsertField3D = TpmsInsert3D;

impl Default for TpmsInsert3D {
    fn default() -> Self {
        Self {
            basis: TpmsBasis::None,
            radial_inner_fraction: 0.68,
            axial_start_mm: 24.0,
            axial_end_mm: 64.0,
            radial_wavelength_mm: 8.5,
            axial_wavelength_mm: 9.0,
            sheet_thickness_mm: 1.35,
        }
    }
}

impl TpmsInsert3D {
    pub fn is_inactive(&self) -> bool {
        self.basis == TpmsBasis::None
    }
}

/// Compositional 3D hydrocyclone design program (`ct35-exotic-head-v2` / `v3` superset).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignProgram {
    pub schema_version: u32,
    pub family: String,
    #[serde(default, skip_serializing)]
    pub topology_class: String,
    pub baseline_file_sha256: String,
    pub inlet_ramp: InletRamp3D,
    pub inlet_split: InletSplit3D,
    #[serde(default, skip_serializing_if = "HelicalRibbon3D::is_inactive")]
    pub ribbon_cascade: HelicalRibbon3D,
    #[serde(default, skip_serializing_if = "TpmsInsert3D::is_inactive")]
    pub tpms_insert: TpmsInsert3D,
    pub chamber_modes: Vec<ChamberMode3D>,
    pub cone_profile: ConeProfile3D,
    pub skimmer: Skimmer3D,
}

/// Explicit migration of the legacy macro language; this does not implement the
/// proposed typed developmental v4 graph or establish geometric equivalence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MigrationReceipt {
    pub source_hash: String,
    pub target_hash: String,
    pub implementation_hash: String,
    pub semantics_version: String,
    pub transcript: Vec<String>,
    pub information_loss: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenomeEdit {
    pub path: String,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RepairRecord {
    pub rule: String,
    /// Hashes of the complete serialized proposals, including invalid proposals.
    pub before_hash: String,
    pub after_hash: String,
    pub edits: Vec<GenomeEdit>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RngSpan {
    pub before: u64,
    pub after: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalityMetrics {
    pub changed_fields: usize,
    /// Parameter edit counts only; no geometric distance has been measured.
    pub geometric_distance: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VariationOutcome {
    #[serde(default)]
    pub candidate_id: Option<String>,
    #[serde(default)]
    pub parent_ids: Vec<String>,
    pub child: Option<DesignProgram>,
    pub proposal: Option<DesignProgram>,
    pub edits: Vec<GenomeEdit>,
    pub repairs: Vec<RepairRecord>,
    pub failure: Option<String>,
    pub rng_span: RngSpan,
    pub locality: LocalityMetrics,
    pub operator_id: String,
    pub operator_version: String,
    pub parent_hashes: Vec<String>,
}

/// Infrastructure errors are distinct from an ordinary rejected proposal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VariationError(pub String);

impl From<VariationError> for String {
    fn from(error: VariationError) -> Self {
        error.0
    }
}

impl VariationOutcome {
    pub fn with_parent_ids(mut self, ids: Vec<String>) -> Self {
        self.parent_ids = ids;
        self
    }
    /// Explicit generation/control operator, never an error recovery path.
    pub fn introduce(
        program: DesignProgram,
        operator: &str,
        rng: &mut Rng,
    ) -> Result<Self, VariationError> {
        variation(&[], operator, rng, |_, _| program.clone())
    }

    /// Compose stages into one proposal while preserving all edit/repair receipts.
    pub fn then(
        self,
        next: impl FnOnce(&DesignProgram) -> Result<Self, VariationError>,
    ) -> Result<Self, VariationError> {
        let Some(child) = self.child.as_ref() else {
            return Ok(self);
        };
        let mut out = next(child)?;
        out.parent_hashes = self.parent_hashes;
        out.parent_ids = self.parent_ids;
        out.operator_id = format!("{}+{}", self.operator_id, out.operator_id);
        out.rng_span.before = self.rng_span.before;
        let mut edits = self.edits;
        edits.extend(out.edits);
        out.edits = edits;
        let mut repairs = self.repairs;
        repairs.extend(out.repairs);
        out.repairs = repairs;
        out.locality.changed_fields = out.edits.len();
        Ok(out)
    }

    pub fn into_child(self) -> Result<DesignProgram, String> {
        self.child.ok_or_else(|| {
            self.failure
                .unwrap_or_else(|| "variation has no child".into())
        })
    }
}

pub(crate) fn program_wire_hash(p: &DesignProgram) -> Result<String, VariationError> {
    let wire = serde_json::to_vec(p).map_err(|e| VariationError(e.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(wire)))
}

pub(crate) fn genome_edits(
    before: &DesignProgram,
    after: &DesignProgram,
) -> Result<Vec<GenomeEdit>, VariationError> {
    let a = serde_json::to_value(before).map_err(|e| VariationError(e.to_string()))?;
    let b = serde_json::to_value(after).map_err(|e| VariationError(e.to_string()))?;
    let mut edits = Vec::new();
    // Complete top-level module replacements form an exact replayable edit script.
    let a = a
        .as_object()
        .ok_or_else(|| VariationError("program must be an object".into()))?;
    let b = b
        .as_object()
        .ok_or_else(|| VariationError("program must be an object".into()))?;
    for key in a
        .keys()
        .chain(b.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        let old = a.get(key).cloned().unwrap_or(serde_json::Value::Null);
        let new = b.get(key).cloned().unwrap_or(serde_json::Value::Null);
        if old != new {
            edits.push(GenomeEdit {
                path: format!("/{key}"),
                before: old,
                after: new,
            });
        }
    }
    Ok(edits)
}

pub(crate) fn project_bound(value: f64, lo: f64, hi: f64, projection: bool) -> f64 {
    if projection {
        value.clamp(lo, hi)
    } else {
        value
    }
}

pub(crate) fn variation(
    parents: &[&DesignProgram],
    operator: &str,
    rng: &mut Rng,
    kernel: impl Fn(&mut Rng, bool) -> DesignProgram,
) -> Result<VariationOutcome, VariationError> {
    let mut out = VariationOutcome {
        candidate_id: None,
        parent_ids: Vec::new(),
        child: None,
        proposal: None,
        edits: Vec::new(),
        repairs: Vec::new(),
        failure: None,
        rng_span: RngSpan {
            before: rng.0,
            after: rng.0,
        },
        locality: LocalityMetrics {
            changed_fields: 0,
            geometric_distance: None,
        },
        operator_id: operator.into(),
        operator_version: "2".into(),
        parent_hashes: Vec::new(),
    };
    for parent in parents {
        match parent.canonical_sha256() {
            Ok(hash) => out.parent_hashes.push(hash),
            Err(reason) => {
                out.failure = Some(format!("invalid parent: {reason}"));
                return Ok(out);
            }
        }
    }
    if parents
        .windows(2)
        .any(|p| p[0].schema_version != p[1].schema_version || p[0].family != p[1].family)
    {
        out.failure = Some("crossover schema mismatch; explicit migration required".into());
        return Ok(out);
    }
    // Retain the deterministic unprojected proposal and the explicitly projected
    // proposal. Each replay begins at the same RNG state. The actual operator's
    // consumed span is the projected replay; projection is a versioned operator.
    let raw = kernel(&mut rng.clone(), false);
    let projected = kernel(rng, true);
    out.rng_span.after = rng.0;
    if let Some(parent) = parents.first() {
        out.edits = genome_edits(parent, &raw)?;
    } else {
        out.edits = genome_edits(&DesignProgram::baseline_ct35(), &raw)?;
    }
    out.locality.changed_fields = out.edits.len();
    let repair_edits = genome_edits(&raw, &projected)?;
    if !repair_edits.is_empty() {
        out.repairs.push(RepairRecord {
            rule: format!("{operator}/bounded-macro-projection-v1"),
            before_hash: program_wire_hash(&raw)?,
            after_hash: program_wire_hash(&projected)?,
            edits: repair_edits,
        });
    }
    out.proposal = Some(raw);
    match projected.canonicalize() {
        Ok(child) => out.child = Some(child),
        Err(reason) => out.failure = Some(reason),
    }
    Ok(out)
}

/// Dimensionless physical mechanism descriptors for Quality-Diversity (MAP-Elites) and screening.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MechanismDescriptors {
    pub swirl_flux_index: f64,
    pub axial_pitch_sin: f64,
    pub azimuthal_symmetry_order: f64,
    pub shear_mitigation_factor: f64,
    pub cone_accel_persistence: f64,
    pub core_shield_ratio: f64,
    pub wall_rifling_stabilization: f64,
    pub hydraulic_resistance_factor: f64,
}

impl DesignProgram {
    /// Check the declared schema and parameter bounds without changing the input.
    pub fn validate(&self) -> Result<(), String> {
        self.normalize().map(|_| ())
    }

    /// Canonicalization never migrates the declared schema.
    pub fn canonicalize(&self) -> Result<Self, String> {
        self.normalize()
    }

    /// Wrap v2 macro semantics in an explicitly identified legacy-v4 schema.
    /// The developmental graph remains a separate, unimplemented language.
    pub fn migrate_v2_to_v4(&self) -> Result<(Self, MigrationReceipt), String> {
        self.validate()?;
        if self.schema_version != 2 {
            return Err("migrate_v2_to_v4 requires schema v2".into());
        }
        let source_hash = self.canonical_sha256()?;
        let mut target = self.canonicalize()?;
        target.schema_version = 4;
        target.family = "ct35-legacy-macro-v4".into();
        let target_hash = target.canonical_sha256()?;
        let receipt = MigrationReceipt {
            source_hash,
            target_hash,
            implementation_hash: format!("{:x}", Sha256::digest(include_bytes!("program.rs"))),
            semantics_version: "legacy-macro-v2-to-v4/1".into(),
            transcript: vec![
                "preserved legacy macro fields and frozen interface identity".into(),
                "changed schema envelope only; no developmental graph synthesized".into(),
                "geometry equivalence has not been independently audited".into(),
            ],
            information_loss: Vec::new(),
        };
        Ok((target, receipt))
    }

    /// Construct the exact zero-mutation CT-35 baseline program.
    pub fn baseline_ct35() -> Self {
        Self {
            schema_version: 2,
            family: "ct35-exotic-head-v2".into(),
            topology_class: "baseline-ct35".into(),
            baseline_file_sha256: baseline_file_sha256(),
            inlet_ramp: InletRamp3D {
                axial_drop_mm: 0.0,
                radial_slot_factor: 1.0,
                superellipse_exponent: 2.0,
            },
            inlet_split: InletSplit3D {
                enabled: false,
                vane_thickness_mm: 1.2,
                vane_z_fraction: 0.5,
                branches_per_port: 1,
                azimuthal_phase_deg: 0.0,
            },
            ribbon_cascade: HelicalRibbon3D::default(),
            tpms_insert: TpmsInsert3D::default(),
            chamber_modes: Vec::new(),
            cone_profile: ConeProfile3D {
                mode1_mm: 0.0,
                mode2_mm: 0.0,
                mode3_mm: 0.0,
            },
            skimmer: Skimmer3D {
                length_mm: 0.0,
                outer_radius_mm: 2.4,
                capture_flutes: 0,
            },
        }
    }
}

/// Canonical 11-family sampling label note preserved for `NUMBERS_TRACE.md`:
/// `across all 11 families`.
#[path = "program/archetypes.rs"]
mod archetypes;
#[path = "program/catalog.rs"]
mod catalog;
#[path = "program/variation.rs"]
mod variation;

#[path = "program/normalize.rs"]
mod normalize;

pub use self::catalog::*;
