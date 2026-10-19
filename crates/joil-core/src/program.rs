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

    /// Validate all parameter bounds, enforce interface hash, and canonicalize inactive genes.
    pub fn normalize(&self) -> Result<Self, String> {
        fn contains_null(value: &serde_json::Value) -> bool {
            match value {
                serde_json::Value::Null => true,
                serde_json::Value::Array(values) => values.iter().any(contains_null),
                serde_json::Value::Object(values) => values.values().any(contains_null),
                _ => false,
            }
        }
        // serde_json represents non-finite floats as null. This schema has no
        // nullable fields, including inactive numeric genes.
        if contains_null(&serde_json::to_value(self).map_err(|e| e.to_string())?) {
            return Err("non-finite program parameter; repair prohibited".into());
        }
        let expected_sha = baseline_file_sha256();
        if self.baseline_file_sha256 != expected_sha {
            return Err("baseline_file_sha256 mismatch with frozen CT-35 genome".into());
        }
        if !matches!(
            (self.schema_version, self.family.as_str()),
            (2, "ct35-exotic-head-v2") | (3, "ct35-exotic-head-v3") | (4, "ct35-legacy-macro-v4")
        ) {
            return Err("unsupported design program schema_version or family".into());
        }

        let drop_mm = self.inlet_ramp.axial_drop_mm;
        let slot_factor = self.inlet_ramp.radial_slot_factor;
        let se_exp = self.inlet_ramp.superellipse_exponent;
        if !drop_mm.is_finite() || !(0.0..=8.0).contains(&drop_mm) {
            return Err(format!(
                "inlet_ramp.axial_drop_mm {drop_mm} out of [0, 8.0]"
            ));
        }
        if !slot_factor.is_finite() || !(0.72..=1.05).contains(&slot_factor) {
            return Err(format!(
                "inlet_ramp.radial_slot_factor {slot_factor} out of [0.72, 1.05]"
            ));
        }
        if !se_exp.is_finite() || !(2.0..=4.0).contains(&se_exp) {
            return Err(format!(
                "inlet_ramp.superellipse_exponent {se_exp} out of [2.0, 4.0]"
            ));
        }
        let canon_se = if (drop_mm.abs() < 1e-12) && ((slot_factor - 1.0).abs() < 1e-12) {
            2.0
        } else {
            round4(se_exp)
        };

        let mut split_enabled = self.inlet_split.enabled;
        let mut vane_t = self.inlet_split.vane_thickness_mm;
        let mut vane_z = self.inlet_split.vane_z_fraction;
        let mut branches = self.inlet_split.branches_per_port;
        let mut phase_deg = self.inlet_split.azimuthal_phase_deg;
        if !(1..=4).contains(&branches) {
            return Err("inlet_split.branches_per_port must be in [1, 4]".into());
        }
        if split_enabled {
            if !vane_t.is_finite()
                || !vane_z.is_finite()
                || !phase_deg.is_finite()
                || !(1.0..=1.8).contains(&vane_t)
                || !(0.35..=0.65).contains(&vane_z)
                || !(0.0..=45.0).contains(&phase_deg)
            {
                return Err("invalid inlet_split vane parameters".into());
            }
            if !(2..=4).contains(&branches) {
                return Err("enabled inlet_split requires 2..=4 branches_per_port".into());
            }
            vane_t = round4(vane_t);
            vane_z = round4(vane_z);
            phase_deg = round4(phase_deg);
        } else {
            split_enabled = false;
            vane_t = 1.2;
            vane_z = 0.5;
            branches = 1;
            phase_deg = 0.0;
        }

        // Canonicalize Wave-2 HelicalRibbon3D
        let canon_ribbon = if self.ribbon_cascade.is_inactive() {
            HelicalRibbon3D::default()
        } else {
            let vc = self.ribbon_cascade.vane_count;
            let span = self.ribbon_cascade.axial_span_mm;
            let twist = self.ribbon_cascade.helical_twist_rad;
            let rt = self.ribbon_cascade.thickness_mm;
            let rc = self.ribbon_cascade.radial_chord_mm;
            if !matches!(vc, 2 | 3 | 4 | 6)
                || !span.is_finite()
                || !(8.0..=48.0).contains(&span)
                || !twist.is_finite()
                || !(-4.5..=4.5).contains(&twist)
                || !rt.is_finite()
                || !(1.10..=1.80).contains(&rt)
                || !rc.is_finite()
                || !(1.50..=4.20).contains(&rc)
            {
                return Err("invalid ribbon_cascade parameters".into());
            }
            HelicalRibbon3D {
                vane_count: vc,
                axial_span_mm: round4(span),
                helical_twist_rad: round4(twist),
                thickness_mm: round4(rt),
                radial_chord_mm: round4(rc),
            }
        };

        // Canonicalize Wave-2 TpmsInsert3D
        let canon_tpms = if self.tpms_insert.is_inactive() {
            TpmsInsert3D::default()
        } else {
            let rif = self.tpms_insert.radial_inner_fraction;
            let zs = self.tpms_insert.axial_start_mm;
            let ze = self.tpms_insert.axial_end_mm;
            let lr = self.tpms_insert.radial_wavelength_mm;
            let lz = self.tpms_insert.axial_wavelength_mm;
            let st = self.tpms_insert.sheet_thickness_mm;
            if !rif.is_finite()
                || !(0.58..=0.85).contains(&rif)
                || !zs.is_finite()
                || !(15.0..=55.0).contains(&zs)
                || !ze.is_finite()
                || !(50.0..=95.0).contains(&ze)
                || (ze - zs) < 16.0
                || !lr.is_finite()
                || !(6.0..=14.0).contains(&lr)
                || !lz.is_finite()
                || !(6.0..=14.0).contains(&lz)
                || !st.is_finite()
                || !(1.20..=2.20).contains(&st)
            {
                return Err("invalid tpms_insert parameters".into());
            }
            TpmsInsert3D {
                basis: self.tpms_insert.basis,
                radial_inner_fraction: round4(rif),
                axial_start_mm: round4(zs),
                axial_end_mm: round4(ze),
                radial_wavelength_mm: round4(lr),
                axial_wavelength_mm: round4(lz),
                sheet_thickness_mm: round4(st),
            }
        };

        if self.chamber_modes.len() > 8 {
            return Err("at most 8 chamber_modes allowed".into());
        }
        let mut amp_sum = 0.0;
        let mut canon_modes = Vec::new();
        for m in &self.chamber_modes {
            if !(2..=8).contains(&m.azimuthal_order) {
                return Err(format!(
                    "azimuthal_order {} out of [2, 8]",
                    m.azimuthal_order
                ));
            }
            if !m.cos_mm.is_finite() || !m.sin_mm.is_finite() || !m.twist_rad.is_finite() {
                return Err("non-finite chamber_mode parameter".into());
            }
            if m.twist_rad.abs() > 2.0 * PI {
                return Err("chamber_mode twist_rad exceeds 2*pi".into());
            }
            let amp = m.cos_mm.hypot(m.sin_mm);
            amp_sum += amp;
            if amp > 1e-12 {
                canon_modes.push(ChamberMode3D {
                    azimuthal_order: m.azimuthal_order,
                    cos_mm: round4(m.cos_mm),
                    sin_mm: round4(m.sin_mm),
                    twist_rad: round4(m.twist_rad),
                });
            }
        }
        if amp_sum > 1.8 + 1e-12 {
            return Err(format!(
                "chamber_modes total amplitude {amp_sum:.4} exceeds 1.8 mm wall budget"
            ));
        }
        canon_modes.sort_by(|a, b| {
            a.azimuthal_order
                .cmp(&b.azimuthal_order)
                .then_with(|| a.cos_mm.total_cmp(&b.cos_mm))
                .then_with(|| a.sin_mm.total_cmp(&b.sin_mm))
        });

        let m1 = self.cone_profile.mode1_mm;
        let m2 = self.cone_profile.mode2_mm;
        let m3 = self.cone_profile.mode3_mm;
        if !m1.is_finite()
            || !m2.is_finite()
            || !m3.is_finite()
            || (m1.abs() + m2.abs() + m3.abs()) > 1.6 + 1e-12
        {
            return Err("cone_profile amplitude exceeds 1.6 mm budget".into());
        }

        let mut skim_len = self.skimmer.length_mm;
        let mut skim_r = self.skimmer.outer_radius_mm;
        let mut flutes = self.skimmer.capture_flutes;
        if !skim_len.is_finite() || !(0.0..=10.0).contains(&skim_len) {
            return Err("skimmer.length_mm out of [0, 10.0]".into());
        }
        if skim_len <= 1e-12 {
            skim_len = 0.0;
            skim_r = 2.4;
            flutes = 0;
        } else {
            if !skim_r.is_finite() || !(1.8..=4.0).contains(&skim_r) {
                return Err("skimmer.outer_radius_mm out of [1.8, 4.0]".into());
            }
            if !matches!(flutes, 0 | 4 | 6 | 8) {
                return Err("skimmer.capture_flutes must be 0, 4, 6, or 8".into());
            }
            skim_len = round4(skim_len);
            skim_r = round4(skim_r);
        }

        let mut out = Self {
            schema_version: self.schema_version,
            family: self.family.clone(),
            topology_class: self.topology_class.clone(),
            baseline_file_sha256: expected_sha,
            inlet_ramp: InletRamp3D {
                axial_drop_mm: round4(drop_mm),
                radial_slot_factor: round4(slot_factor),
                superellipse_exponent: canon_se,
            },
            inlet_split: InletSplit3D {
                enabled: split_enabled,
                vane_thickness_mm: vane_t,
                vane_z_fraction: vane_z,
                branches_per_port: branches,
                azimuthal_phase_deg: phase_deg,
            },
            ribbon_cascade: canon_ribbon,
            tpms_insert: canon_tpms,
            chamber_modes: canon_modes,
            cone_profile: ConeProfile3D {
                mode1_mm: round4(m1),
                mode2_mm: round4(m2),
                mode3_mm: round4(m3),
            },
            skimmer: Skimmer3D {
                length_mm: skim_len,
                outer_radius_mm: skim_r,
                capture_flutes: flutes,
            },
        };
        out.topology_class = out.classify_topology();
        Ok(out)
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

pub use self::catalog::*;
