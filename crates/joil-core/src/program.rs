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

fn round4(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

fn round6(x: f64) -> f64 {
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

fn default_superellipse_n() -> f64 {
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

fn default_branches_per_port() -> u32 {
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
        let expected_sha = baseline_file_sha256();
        if self.baseline_file_sha256 != expected_sha {
            return Err("baseline_file_sha256 mismatch with frozen CT-35 genome".into());
        }
        if !matches!(self.schema_version, 2 | 3)
            || !matches!(
                self.family.as_str(),
                "ct35-exotic-head-v2" | "ct35-exotic-head-v3"
            )
        {
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
                branches = 2;
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
            schema_version: 2,
            family: "ct35-exotic-head-v2".into(),
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

    /// Canonicalize genes and perform full F0 3D phenotype, watertight/manifold, and Red-Team preflight.
    pub fn canonicalize_and_preflight(
        &self,
    ) -> Result<
        (
            Self,
            crate::phenotype::Phenotype3D,
            crate::phenotype::SolidMesh,
        ),
        String,
    > {
        let norm = self.normalize()?;
        let (pheno, mesh) = crate::phenotype::Phenotype3D::from_program(&norm)?;
        crate::redteam::check(&norm, &pheno, &mesh)
            .map_err(|e| format!("preflight red-team check failed: {e:?}"))?;
        Ok((norm, pheno, mesh))
    }

    /// True if all exotic perturbations vanish and the design is the exact CT-35 baseline.
    pub fn is_exact_baseline(&self) -> Result<bool, String> {
        let p = self.normalize()?;
        Ok(p.inlet_ramp.axial_drop_mm.abs() < 1e-12
            && (p.inlet_ramp.radial_slot_factor - 1.0).abs() < 1e-12
            && !p.inlet_split.enabled
            && p.ribbon_cascade.is_inactive()
            && p.tpms_insert.is_inactive()
            && p.chamber_modes.is_empty()
            && p.cone_profile.mode1_mm.abs() < 1e-12
            && p.cone_profile.mode2_mm.abs() < 1e-12
            && p.cone_profile.mode3_mm.abs() < 1e-12
            && p.skimmer.length_mm.abs() < 1e-12)
    }

    /// Classify the candidate into one of 11 canonical 3D topological families (7 Wave-1 + 4 Wave-2).
    pub fn classify_topology(&self) -> String {
        match self.tpms_insert.basis {
            TpmsBasis::Gyroid | TpmsBasis::CylindricalGyroid => {
                return "cylindrical-gyroid-coalescer".into();
            }
            TpmsBasis::SchwarzP | TpmsBasis::Diamond => {
                return "schwarz-p-rectifier-head".into();
            }
            TpmsBasis::None => {}
        }
        if self.inlet_split.enabled && self.inlet_split.branches_per_port == 4 {
            return "octa-branch-helical-manifold".into();
        }
        if !self.ribbon_cascade.is_inactive()
            && self.ribbon_cascade.vane_count >= 3
            && self.skimmer.capture_flutes >= 4
        {
            return "counter-fluted-recovery-vortex".into();
        }

        let has_ramp = self.inlet_ramp.axial_drop_mm > 0.25
            || (self.inlet_ramp.radial_slot_factor - 1.0).abs() > 0.01;
        let has_split = self.inlet_split.enabled;
        let has_modes = !self.chamber_modes.is_empty();
        let has_cone = self.cone_profile.mode1_mm.abs() > 0.05
            || self.cone_profile.mode2_mm.abs() > 0.05
            || self.cone_profile.mode3_mm.abs() > 0.05;
        let has_skim = self.skimmer.length_mm > 0.25;

        let active_count = [has_ramp, has_split, has_modes, has_cone, has_skim]
            .iter()
            .filter(|&&b| b)
            .count();
        if active_count == 0 {
            "baseline-ct35".into()
        } else if active_count >= 3 {
            if self.inlet_split.branches_per_port == 3 {
                "trifurcated-6jet-vortex-head".into()
            } else {
                "hybrid-omni-exotic".into()
            }
        } else if has_split {
            if self.inlet_split.branches_per_port == 3 {
                "trifurcated-6jet-vortex-head".into()
            } else {
                "bifurcated-quad-manifold".into()
            }
        } else if has_modes {
            "spiral-rifled-vortex-head".into()
        } else if has_cone || has_skim {
            "contoured-s-cone-skimmer".into()
        } else {
            "helical-pitched-ribbon".into()
        }
    }

    /// Deterministic SHA-256 over the normalized canonical JSON representation.
    pub fn canonical_sha256(&self) -> Result<String, String> {
        let norm = self.normalize()?;
        let bytes = serde_json::to_vec(&norm).map_err(|e| e.to_string())?;
        Ok(Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }

    /// Compute dimensionless 3D mechanism descriptors for MAP-Elites and physics evaluation.
    pub fn descriptors(&self) -> Result<MechanismDescriptors, String> {
        let p = self.normalize()?;
        let d: Dimensions = baseline().derive()?;
        let r1 = d.d1_mm / 2.0;
        let w = d.inlet_width_mm;
        let h = d.inlet_height_mm;

        let drop_mm = p.inlet_ramp.axial_drop_mm;
        let slot_factor = p.inlet_ramp.radial_slot_factor;
        let se_exp = p.inlet_ramp.superellipse_exponent;
        let split_enabled = p.inlet_split.enabled;
        let branches = p.inlet_split.branches_per_port as f64;
        let phase_deg = p.inlet_split.azimuthal_phase_deg;
        let m1 = p.cone_profile.mode1_mm;
        let m2 = p.cone_profile.mode2_mm;
        let m3 = p.cone_profile.mode3_mm;
        let skim_len = p.skimmer.length_mm;
        let flutes = p.skimmer.capture_flutes as f64;

        let r_centroid_base = r1 - 0.5 * w;
        let r_centroid_new = r1 - 0.5 * w * slot_factor;
        let jet_accel = 1.0 / (0.35 + 0.65 * slot_factor);
        let ribbon_swirl_bonus = if !p.ribbon_cascade.is_inactive() {
            0.035
                * (p.ribbon_cascade.vane_count as f64 / 3.0)
                * (p.ribbon_cascade.axial_span_mm / 32.0)
        } else {
            0.0
        };
        let swirl_flux_idx = (r_centroid_new / r_centroid_base)
            * (1.0 + 0.38 * (jet_accel - 1.0))
            * (1.0 + 0.015 * (se_exp - 2.0))
            + ribbon_swirl_bonus;

        let ramp_len_mm = 32.0_f64;
        let axial_pitch_sin = drop_mm / ramp_len_mm.hypot(drop_mm);

        let mode_amp: f64 = p
            .chamber_modes
            .iter()
            .map(|m| m.cos_mm.hypot(m.sin_mm))
            .sum();
        let base_sym = if split_enabled {
            2.0 * branches * (0.91 + 0.002 * phase_deg)
        } else {
            2.0
        };
        let tpms_sym_bonus = match p.tpms_insert.basis {
            TpmsBasis::None => 0.0,
            TpmsBasis::Gyroid | TpmsBasis::CylindricalGyroid => 0.55,
            TpmsBasis::SchwarzP | TpmsBasis::Diamond => 0.45,
        };
        let symmetry_order = base_sym + (0.2 * mode_amp).min(0.45) + tpms_sym_bonus;

        let mut shear_mitigation = 1.0 - 0.22 * (drop_mm / 8.0) - 0.14 * (m1 / 1.2).max(0.0)
            + 0.35 * (((0.85 - slot_factor) / 0.15).max(0.0)).powi(2)
            - 0.02 * (se_exp - 2.0);
        if split_enabled {
            shear_mitigation *= match p.inlet_split.branches_per_port {
                4 => 0.80,
                3 => 0.84,
                _ => 0.88,
            };
        }
        if !p.tpms_insert.is_inactive() {
            // Peripheral TPMS rectifier dampens high-frequency jet shear while promoting sub-15um Stokes coalescence
            shear_mitigation *= 0.86;
        }
        if !p.ribbon_cascade.is_inactive() {
            shear_mitigation *= 0.94;
        }

        let cone_accel_persistence = 1.0 + 0.11 * m1 - 0.07 * m2 + 0.04 * m3;
        let core_shield_ratio = (skim_len / h) * (1.0 + 0.015 * flutes);

        let mut helical_alignment = 0.0;
        for m in &p.chamber_modes {
            let amp = m.cos_mm.hypot(m.sin_mm);
            let sign_bonus = if m.twist_rad < 0.0 { 1.0 } else { 0.4 };
            let order_weight = if (2..=4).contains(&m.azimuthal_order) {
                1.0
            } else {
                0.6
            };
            helical_alignment += (amp / 1.5) * sign_bonus * order_weight;
        }
        if !p.tpms_insert.is_inactive() {
            helical_alignment += 0.28 * (p.tpms_insert.radial_inner_fraction / 0.70);
        }
        if !p.ribbon_cascade.is_inactive() {
            helical_alignment += 0.18 * (p.ribbon_cascade.axial_span_mm / 36.0);
        }
        let wall_rifling_stab = helical_alignment.min(1.38);

        let tpms_drag_penalty = match p.tpms_insert.basis {
            TpmsBasis::None => 0.0,
            TpmsBasis::CylindricalGyroid => 0.012 * (1.0 - p.tpms_insert.radial_inner_fraction),
            TpmsBasis::Gyroid => 0.018 * (1.0 - p.tpms_insert.radial_inner_fraction),
            TpmsBasis::SchwarzP | TpmsBasis::Diamond => {
                0.024 * (1.0 - p.tpms_insert.radial_inner_fraction)
            }
        };
        let ribbon_recovery_bonus = if !p.ribbon_cascade.is_inactive() {
            -0.018 * (p.ribbon_cascade.vane_count as f64 / 4.0)
        } else {
            0.0
        };

        let hydraulic_resistance_factor = 1.0 - 0.065 * (drop_mm / 8.0) - 0.055 * (m1 / 1.2)
            + 0.025 * (mode_amp / 1.5)
            + if split_enabled {
                0.014 * (branches - 1.0)
            } else {
                0.0
            }
            + 0.12 * (1.0 - slot_factor).max(0.0)
            + tpms_drag_penalty
            + ribbon_recovery_bonus;

        Ok(MechanismDescriptors {
            swirl_flux_index: round6(swirl_flux_idx),
            axial_pitch_sin: round6(axial_pitch_sin),
            azimuthal_symmetry_order: round6(symmetry_order),
            shear_mitigation_factor: round6(shear_mitigation),
            cone_accel_persistence: round6(cone_accel_persistence),
            core_shield_ratio: round6(core_shield_ratio),
            wall_rifling_stabilization: round6(wall_rifling_stab),
            hydraulic_resistance_factor: round6(hydraulic_resistance_factor),
        })
    }

    /// Generate initial diverse 3D archetype seeds spanning all 7 Wave-1 topological families.
    pub fn seed_archetypes(rng: &mut Rng) -> Vec<Self> {
        let base = Self::baseline_ct35();
        let mut seeds = vec![base.clone()];

        // 1. Helical-pitched ribbon entries
        for (drop, slot, se) in [(4.5, 0.90, 2.6), (6.5, 0.86, 3.0), (7.5, 0.84, 3.2)] {
            let mut p = base.clone();
            p.inlet_ramp = InletRamp3D {
                axial_drop_mm: drop,
                radial_slot_factor: slot,
                superellipse_exponent: se,
            };
            if let Ok(n) = p.normalize() {
                seeds.push(n);
            }
        }

        // 2. Bifurcated 2->4 & Trifurcated 2->6 phased tangential manifolds
        for (drop, vt, vz, br, ph) in [
            (3.8, 1.20, 0.50, 2, 15.0),
            (5.5, 1.12, 0.48, 2, 22.5),
            (6.4, 1.15, 0.50, 3, 30.0),
        ] {
            let mut p = base.clone();
            p.inlet_ramp = InletRamp3D {
                axial_drop_mm: drop,
                radial_slot_factor: 0.91,
                superellipse_exponent: 2.5,
            };
            p.inlet_split = InletSplit3D {
                enabled: true,
                vane_thickness_mm: vt,
                vane_z_fraction: vz,
                branches_per_port: br,
                azimuthal_phase_deg: ph,
            };
            if let Ok(n) = p.normalize() {
                seeds.push(n);
            }
        }

        // 3. Contoured S-cone + fluted skimmer sleeves
        for (m1, m2, m3, sl, sr, fl) in [
            (0.95, -0.45, 0.08, 5.5, 2.3, 4),
            (1.12, -0.32, 0.10, 7.5, 2.2, 6),
            (0.82, -0.50, 0.05, 8.5, 2.4, 4),
        ] {
            let mut p = base.clone();
            p.cone_profile = ConeProfile3D {
                mode1_mm: m1,
                mode2_mm: m2,
                mode3_mm: m3,
            };
            p.skimmer = Skimmer3D {
                length_mm: sl,
                outer_radius_mm: sr,
                capture_flutes: fl,
            };
            if let Ok(n) = p.normalize() {
                seeds.push(n);
            }
        }

        // 4. Spiral-rifled 3D harmonic swirl chambers
        for (order, amp, twist) in [(2, 0.95, -4.2), (3, 1.10, -5.1), (4, 0.85, -3.8)] {
            let mut p = base.clone();
            p.chamber_modes = vec![ChamberMode3D {
                azimuthal_order: order,
                cos_mm: amp * 0.8,
                sin_mm: amp * 0.6,
                twist_rad: twist,
            }];
            p.cone_profile = ConeProfile3D {
                mode1_mm: 0.75,
                mode2_mm: -0.30,
                mode3_mm: 0.05,
            };
            if let Ok(n) = p.normalize() {
                seeds.push(n);
            }
        }

        // 5. Hybrid omni-exotic & trifurcated 6-jet vortex heads
        for i in 0..6 {
            let mut p = base.clone();
            p.inlet_ramp = InletRamp3D {
                axial_drop_mm: round4(rng.range(4.6, 7.7)),
                radial_slot_factor: round4(rng.range(0.84, 0.93)),
                superellipse_exponent: round4(rng.range(2.2, 3.6)),
            };
            p.inlet_split = InletSplit3D {
                enabled: i % 2 == 0,
                vane_thickness_mm: 1.15,
                vane_z_fraction: 0.50,
                branches_per_port: if i % 4 == 0 { 3 } else { 2 },
                azimuthal_phase_deg: round4(rng.range(10.0, 35.0)),
            };
            p.chamber_modes = vec![ChamberMode3D {
                azimuthal_order: 2 + (i % 3) as u32,
                cos_mm: round4(rng.range(0.45, 0.85)),
                sin_mm: round4(rng.range(-0.50, 0.50)),
                twist_rad: round4(rng.range(-5.5, -2.5)),
            }];
            p.cone_profile = ConeProfile3D {
                mode1_mm: round4(rng.range(0.65, 1.10)),
                mode2_mm: round4(rng.range(-0.40, -0.15)),
                mode3_mm: round4(rng.range(-0.08, 0.12)),
            };
            p.skimmer = Skimmer3D {
                length_mm: round4(rng.range(5.0, 9.2)),
                outer_radius_mm: round4(rng.range(2.1, 2.5)),
                capture_flutes: if i % 2 == 0 { 4 } else { 6 },
            };
            if let Ok(n) = p.normalize() {
                seeds.push(n);
            }
        }

        seeds
    }

    /// Generate Wave-2 diverse 3D archetype seeds spanning all 11 topological families
    /// (7 Wave-1 families + Cylindrical-Gyroid TPMS, Schwarz-P TPMS, Octa-Branch 8-Jet, Counter-Fluted Vortex).
    pub fn seed_wave2_archetypes(rng: &mut Rng) -> Vec<Self> {
        let mut seeds = Self::seed_archetypes(rng);
        let base = Self::baseline_ct35();

        // 8. Cylindrical-Gyroid peripheral TPMS coalescer shell + 6-jet helical distributor
        for (basis, rif, lr, lz, st) in [
            (TpmsBasis::CylindricalGyroid, 0.70, 8.5, 9.0, 1.35),
            (TpmsBasis::Gyroid, 0.72, 9.5, 10.0, 1.40),
        ] {
            let mut p = base.clone();
            p.inlet_ramp = InletRamp3D {
                axial_drop_mm: 6.8,
                radial_slot_factor: 0.88,
                superellipse_exponent: 2.8,
            };
            p.inlet_split = InletSplit3D {
                enabled: true,
                vane_thickness_mm: 1.18,
                vane_z_fraction: 0.50,
                branches_per_port: 3,
                azimuthal_phase_deg: 28.0,
            };
            p.tpms_insert = TpmsInsert3D {
                basis,
                radial_inner_fraction: rif,
                axial_start_mm: 22.0,
                axial_end_mm: 62.0,
                radial_wavelength_mm: lr,
                axial_wavelength_mm: lz,
                sheet_thickness_mm: st,
            };
            p.cone_profile = ConeProfile3D {
                mode1_mm: 0.92,
                mode2_mm: -0.28,
                mode3_mm: 0.06,
            };
            p.skimmer = Skimmer3D {
                length_mm: 7.8,
                outer_radius_mm: 2.25,
                capture_flutes: 6,
            };
            if let Ok(n) = p.normalize() {
                seeds.push(n);
            }
        }

        // 9. Schwarz-P / Diamond peripheral TPMS rectifier head
        for (basis, rif, lr, lz) in [
            (TpmsBasis::SchwarzP, 0.74, 9.0, 9.5),
            (TpmsBasis::Diamond, 0.75, 10.0, 10.5),
        ] {
            let mut p = base.clone();
            p.inlet_ramp = InletRamp3D {
                axial_drop_mm: 6.2,
                radial_slot_factor: 0.89,
                superellipse_exponent: 2.6,
            };
            p.inlet_split = InletSplit3D {
                enabled: true,
                vane_thickness_mm: 1.20,
                vane_z_fraction: 0.50,
                branches_per_port: 2,
                azimuthal_phase_deg: 20.0,
            };
            p.tpms_insert = TpmsInsert3D {
                basis,
                radial_inner_fraction: rif,
                axial_start_mm: 24.0,
                axial_end_mm: 60.0,
                radial_wavelength_mm: lr,
                axial_wavelength_mm: lz,
                sheet_thickness_mm: 1.38,
            };
            p.cone_profile = ConeProfile3D {
                mode1_mm: 0.85,
                mode2_mm: -0.25,
                mode3_mm: 0.05,
            };
            p.skimmer = Skimmer3D {
                length_mm: 6.8,
                outer_radius_mm: 2.30,
                capture_flutes: 4,
            };
            if let Ok(n) = p.normalize() {
                seeds.push(n);
            }
        }

        // 10. Octa-branch (2 -> 8 jet) helical manifold
        for ph in [24.0, 34.0] {
            let mut p = base.clone();
            p.inlet_ramp = InletRamp3D {
                axial_drop_mm: 7.2,
                radial_slot_factor: 0.87,
                superellipse_exponent: 3.1,
            };
            p.inlet_split = InletSplit3D {
                enabled: true,
                vane_thickness_mm: 1.12,
                vane_z_fraction: 0.50,
                branches_per_port: 4,
                azimuthal_phase_deg: ph,
            };
            p.chamber_modes = vec![ChamberMode3D {
                azimuthal_order: 4,
                cos_mm: 0.55,
                sin_mm: -0.35,
                twist_rad: -4.2,
            }];
            p.cone_profile = ConeProfile3D {
                mode1_mm: 1.02,
                mode2_mm: -0.30,
                mode3_mm: 0.08,
            };
            p.skimmer = Skimmer3D {
                length_mm: 8.2,
                outer_radius_mm: 2.22,
                capture_flutes: 6,
            };
            if let Ok(n) = p.normalize() {
                seeds.push(n);
            }
        }

        // 11. Counter-fluted pressure-recovery vortex head with internal helical ribbon cascade
        for vc in [3, 4] {
            let mut p = base.clone();
            p.inlet_ramp = InletRamp3D {
                axial_drop_mm: 6.6,
                radial_slot_factor: 0.88,
                superellipse_exponent: 2.9,
            };
            p.inlet_split = InletSplit3D {
                enabled: true,
                vane_thickness_mm: 1.16,
                vane_z_fraction: 0.50,
                branches_per_port: 3,
                azimuthal_phase_deg: 26.0,
            };
            p.ribbon_cascade = HelicalRibbon3D {
                vane_count: vc,
                axial_span_mm: 32.0,
                helical_twist_rad: -2.8,
                thickness_mm: 1.25,
                radial_chord_mm: 2.40,
            };
            p.cone_profile = ConeProfile3D {
                mode1_mm: 0.98,
                mode2_mm: -0.26,
                mode3_mm: 0.07,
            };
            p.skimmer = Skimmer3D {
                length_mm: 8.4,
                outer_radius_mm: 2.20,
                capture_flutes: 8,
            };
            if let Ok(n) = p.normalize() {
                seeds.push(n);
            }
        }

        seeds
    }

    /// Wave-2 Operator 1: Diffeomorphic / Jacobian-guarded spline & harmonic wall mutation.
    /// Analytically bounds modal amplitudes so `det g(theta, z) > 0.25` and LPBF overhang `<= 42 deg`.
    pub fn mutate_diffeomorphic_spline(&self, rng: &mut Rng) -> Self {
        let mut c = self.normalize().unwrap_or_else(|_| Self::baseline_ct35());
        let mut m1 = (c.cone_profile.mode1_mm + rng.gauss(0.0, 0.14)).clamp(-0.60, 1.15);
        let mut m2 = (c.cone_profile.mode2_mm + rng.gauss(0.0, 0.10)).clamp(-0.40, 0.35);
        let mut m3 = (c.cone_profile.mode3_mm + rng.gauss(0.0, 0.05)).clamp(-0.15, 0.15);
        let sum_m = m1.abs() + m2.abs() + m3.abs();
        if sum_m > 1.48 {
            let s = 1.48 / sum_m;
            m1 *= s;
            m2 *= s;
            m3 *= s;
        }
        c.cone_profile = ConeProfile3D {
            mode1_mm: round4(m1),
            mode2_mm: round4(m2),
            mode3_mm: round4(m3),
        };

        if c.chamber_modes.is_empty() && rng.f64() < 0.55 {
            let order = 2 + (rng.next_u64() % 3) as u32;
            c.chamber_modes.push(ChamberMode3D {
                azimuthal_order: order,
                cos_mm: round4(rng.range(0.30, 0.78)),
                sin_mm: round4(rng.range(-0.38, 0.38)),
                twist_rad: round4(rng.range(-5.2, -2.0)),
            });
        } else if !c.chamber_modes.is_empty() {
            for m in &mut c.chamber_modes {
                m.cos_mm = round4(m.cos_mm + rng.gauss(0.0, 0.09));
                m.sin_mm = round4(m.sin_mm + rng.gauss(0.0, 0.09));
                m.twist_rad = round4((m.twist_rad + rng.gauss(0.0, 0.30)).clamp(-5.8, -0.5));
            }
            let total_amp: f64 = c
                .chamber_modes
                .iter()
                .map(|m| m.cos_mm.hypot(m.sin_mm))
                .sum();
            if total_amp > 1.60 {
                let s = 1.60 / total_amp;
                for m in &mut c.chamber_modes {
                    m.cos_mm = round4(m.cos_mm * s);
                    m.sin_mm = round4(m.sin_mm * s);
                }
            }
        }
        c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
    }

    /// Wave-2 Operator 2: Area-conserving branch topology mutation (`K in {2, 4, 6, 8}` nozzles)
    /// and internal helical guide-ribbon cascade perturbation.
    pub fn mutate_branch_topology(&self, rng: &mut Rng) -> Self {
        let mut c = self.normalize().unwrap_or_else(|_| Self::baseline_ct35());
        c.inlet_ramp.axial_drop_mm =
            round4((c.inlet_ramp.axial_drop_mm + rng.gauss(0.0, 0.75)).clamp(0.0, 7.9));
        c.inlet_ramp.radial_slot_factor =
            round4((c.inlet_ramp.radial_slot_factor + rng.gauss(0.0, 0.022)).clamp(0.80, 1.0));
        c.inlet_ramp.superellipse_exponent =
            round4((c.inlet_ramp.superellipse_exponent + rng.gauss(0.0, 0.22)).clamp(2.0, 3.8));

        if rng.f64() < 0.25 {
            c.inlet_split.enabled = !c.inlet_split.enabled;
        }
        if c.inlet_split.enabled {
            c.inlet_split.vane_thickness_mm =
                round4((c.inlet_split.vane_thickness_mm + rng.gauss(0.0, 0.05)).clamp(1.08, 1.45));
            c.inlet_split.vane_z_fraction =
                round4((c.inlet_split.vane_z_fraction + rng.gauss(0.0, 0.025)).clamp(0.42, 0.58));
            if rng.f64() < 0.30 {
                c.inlet_split.branches_per_port = match rng.next_u64() % 3 {
                    0 => 2,
                    1 => 3,
                    _ => 4,
                };
            } else if c.inlet_split.branches_per_port < 2 {
                c.inlet_split.branches_per_port = 2;
            }
            c.inlet_split.azimuthal_phase_deg =
                round4((c.inlet_split.azimuthal_phase_deg + rng.gauss(0.0, 3.5)).clamp(0.0, 40.0));
        }

        if rng.f64() < 0.30 {
            if c.ribbon_cascade.is_inactive() {
                c.ribbon_cascade = HelicalRibbon3D {
                    vane_count: if rng.f64() < 0.6 { 3 } else { 4 },
                    axial_span_mm: round4(rng.range(20.0, 38.0)),
                    helical_twist_rad: round4(rng.range(-3.6, -1.6)),
                    thickness_mm: round4(rng.range(1.18, 1.45)),
                    radial_chord_mm: round4(rng.range(1.80, 3.20)),
                };
            } else if rng.f64() < 0.25 {
                c.ribbon_cascade = HelicalRibbon3D::default();
            } else {
                c.ribbon_cascade.axial_span_mm = round4(
                    (c.ribbon_cascade.axial_span_mm + rng.gauss(0.0, 2.5)).clamp(14.0, 42.0),
                );
                c.ribbon_cascade.helical_twist_rad = round4(
                    (c.ribbon_cascade.helical_twist_rad + rng.gauss(0.0, 0.25)).clamp(-4.0, -1.0),
                );
            }
        }
        c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
    }

    /// Wave-2 Operator 3: Bi-continuous percolation-window TPMS grading mutation (`0` trapped voids,
    /// central vortex core `r < radial_inner_fraction * R_wall` 100% unobstructed).
    pub fn mutate_tpms_grading(&self, rng: &mut Rng) -> Self {
        let mut c = self.normalize().unwrap_or_else(|_| Self::baseline_ct35());
        if c.tpms_insert.is_inactive() {
            let basis = match rng.next_u64() % 4 {
                0 => TpmsBasis::CylindricalGyroid,
                1 => TpmsBasis::Gyroid,
                2 => TpmsBasis::SchwarzP,
                _ => TpmsBasis::Diamond,
            };
            let zs = round4(rng.range(20.0, 32.0));
            let ze = round4((zs + rng.range(28.0, 42.0)).clamp(52.0, 82.0));
            c.tpms_insert = TpmsInsert3D {
                basis,
                radial_inner_fraction: round4(rng.range(0.66, 0.78)),
                axial_start_mm: zs,
                axial_end_mm: ze,
                radial_wavelength_mm: round4(rng.range(7.5, 11.5)),
                axial_wavelength_mm: round4(rng.range(8.0, 12.0)),
                sheet_thickness_mm: round4(rng.range(1.25, 1.65)),
            };
        } else if rng.f64() < 0.20 {
            c.tpms_insert = TpmsInsert3D::default();
        } else {
            c.tpms_insert.radial_inner_fraction = round4(
                (c.tpms_insert.radial_inner_fraction + rng.gauss(0.0, 0.02)).clamp(0.62, 0.82),
            );
            c.tpms_insert.radial_wavelength_mm =
                round4((c.tpms_insert.radial_wavelength_mm + rng.gauss(0.0, 0.4)).clamp(7.0, 12.5));
            c.tpms_insert.axial_wavelength_mm =
                round4((c.tpms_insert.axial_wavelength_mm + rng.gauss(0.0, 0.4)).clamp(7.0, 12.5));
            c.tpms_insert.sheet_thickness_mm =
                round4((c.tpms_insert.sheet_thickness_mm + rng.gauss(0.0, 0.04)).clamp(1.22, 1.85));
        }
        c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
    }

    /// Wave-2 Operator 4: Homologous feature-aligned graph crossover (`SBX` + subtree exchange).
    pub fn crossover_homologous_graph(&self, other: &Self, rng: &mut Rng) -> Self {
        let mut child = self.normalize().unwrap_or_else(|_| Self::baseline_ct35());
        let b = other.normalize().unwrap_or_else(|_| Self::baseline_ct35());
        let alpha = rng.range(0.25, 0.75);
        child.inlet_ramp = InletRamp3D {
            axial_drop_mm: round4(
                alpha * child.inlet_ramp.axial_drop_mm + (1.0 - alpha) * b.inlet_ramp.axial_drop_mm,
            ),
            radial_slot_factor: round4(
                alpha * child.inlet_ramp.radial_slot_factor
                    + (1.0 - alpha) * b.inlet_ramp.radial_slot_factor,
            ),
            superellipse_exponent: round4(
                alpha * child.inlet_ramp.superellipse_exponent
                    + (1.0 - alpha) * b.inlet_ramp.superellipse_exponent,
            ),
        };
        if rng.f64() < 0.5 {
            child.inlet_split = b.inlet_split.clone();
        }
        if rng.f64() < 0.5 {
            child.ribbon_cascade = b.ribbon_cascade.clone();
        }
        if rng.f64() < 0.5 {
            child.tpms_insert = b.tpms_insert.clone();
        }
        child.cone_profile = ConeProfile3D {
            mode1_mm: round4(
                alpha * child.cone_profile.mode1_mm + (1.0 - alpha) * b.cone_profile.mode1_mm,
            ),
            mode2_mm: round4(
                alpha * child.cone_profile.mode2_mm + (1.0 - alpha) * b.cone_profile.mode2_mm,
            ),
            mode3_mm: round4(
                alpha * child.cone_profile.mode3_mm + (1.0 - alpha) * b.cone_profile.mode3_mm,
            ),
        };
        if rng.f64() < 0.5 {
            child.skimmer = b.skimmer.clone();
        }
        if rng.f64() < 0.5 {
            child.chamber_modes = b.chamber_modes.clone();
        }
        child.normalize().unwrap_or_else(|_| Self::baseline_ct35())
    }

    /// Structure-aware Gaussian and topological 3D mutation operator respecting >= 1.0 mm clearance.
    pub fn mutate(&self, rng: &mut Rng) -> Self {
        let mut c = self.mutate_diffeomorphic_spline(rng);
        c = c.mutate_branch_topology(rng);
        if rng.f64() < 0.22 {
            c = c.mutate_tpms_grading(rng);
        }
        c.skimmer.length_mm = round4((c.skimmer.length_mm + rng.gauss(0.0, 0.85)).clamp(0.0, 9.8));
        if c.skimmer.length_mm > 0.1 {
            c.skimmer.outer_radius_mm =
                round4((c.skimmer.outer_radius_mm + rng.gauss(0.0, 0.07)).clamp(2.1, 3.2));
            if rng.f64() < 0.22 {
                c.skimmer.capture_flutes = match (rng.next_u64() % 4) as u32 {
                    0 => 0,
                    1 => 4,
                    2 => 6,
                    _ => 8,
                };
            }
        }
        c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
    }

    /// Mutate restricted strictly to conventional axisymmetric 2-port parameters (`Comparator B2`).
    pub fn mutate_conventional_axisymmetric(&self, rng: &mut Rng) -> Self {
        let mut c = Self::baseline_ct35();
        c.inlet_ramp.radial_slot_factor =
            round4((self.inlet_ramp.radial_slot_factor + rng.gauss(0.0, 0.025)).clamp(0.82, 1.02));
        let mut m1 = (self.cone_profile.mode1_mm + rng.gauss(0.0, 0.12)).clamp(-0.55, 0.95);
        let mut m2 = (self.cone_profile.mode2_mm + rng.gauss(0.0, 0.08)).clamp(-0.35, 0.30);
        let sum_m = m1.abs() + m2.abs();
        if sum_m > 1.20 {
            let s = 1.20 / sum_m;
            m1 *= s;
            m2 *= s;
        }
        c.cone_profile = ConeProfile3D {
            mode1_mm: round4(m1),
            mode2_mm: round4(m2),
            mode3_mm: 0.0,
        };
        c.skimmer.length_mm =
            round4((self.skimmer.length_mm + rng.gauss(0.0, 0.80)).clamp(0.0, 8.5));
        if c.skimmer.length_mm > 0.1 {
            c.skimmer.outer_radius_mm =
                round4((self.skimmer.outer_radius_mm + rng.gauss(0.0, 0.06)).clamp(2.1, 3.0));
            c.skimmer.capture_flutes = 0;
        }
        c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
    }

    /// Sample a pure uniform/LHS random 3D candidate across all 11 families (`Comparator B1`).
    pub fn sample_random_exotic(rng: &mut Rng) -> Self {
        let mut c = Self::baseline_ct35();
        c.inlet_ramp = InletRamp3D {
            axial_drop_mm: round4(rng.range(0.0, 7.8)),
            radial_slot_factor: round4(rng.range(0.79, 1.01)),
            superellipse_exponent: round4(rng.range(2.0, 3.7)),
        };
        if rng.f64() < 0.65 {
            c.inlet_split = InletSplit3D {
                enabled: true,
                vane_thickness_mm: round4(rng.range(1.10, 1.45)),
                vane_z_fraction: round4(rng.range(0.42, 0.58)),
                branches_per_port: match rng.next_u64() % 3 {
                    0 => 2,
                    1 => 3,
                    _ => 4,
                },
                azimuthal_phase_deg: round4(rng.range(0.0, 38.0)),
            };
        }
        if rng.f64() < 0.30 {
            c = c.mutate_tpms_grading(rng);
        }
        if rng.f64() < 0.30 {
            c.ribbon_cascade = HelicalRibbon3D {
                vane_count: if rng.f64() < 0.5 { 3 } else { 4 },
                axial_span_mm: round4(rng.range(18.0, 38.0)),
                helical_twist_rad: round4(rng.range(-3.6, -1.4)),
                thickness_mm: round4(rng.range(1.18, 1.45)),
                radial_chord_mm: round4(rng.range(1.80, 3.10)),
            };
        }
        if rng.f64() < 0.65 {
            c.chamber_modes = vec![ChamberMode3D {
                azimuthal_order: 2 + (rng.next_u64() % 4) as u32,
                cos_mm: round4(rng.range(-0.65, 0.65)),
                sin_mm: round4(rng.range(-0.65, 0.65)),
                twist_rad: round4(rng.range(-5.2, -1.5)),
            }];
        }
        c.cone_profile = ConeProfile3D {
            mode1_mm: round4(rng.range(-0.45, 0.95)),
            mode2_mm: round4(rng.range(-0.30, 0.25)),
            mode3_mm: round4(rng.range(-0.10, 0.10)),
        };
        if rng.f64() < 0.75 {
            c.skimmer = Skimmer3D {
                length_mm: round4(rng.range(2.5, 9.2)),
                outer_radius_mm: round4(rng.range(2.1, 2.8)),
                capture_flutes: match rng.next_u64() % 4 {
                    0 => 0,
                    1 => 4,
                    2 => 6,
                    _ => 8,
                },
            };
        }
        c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
    }

    /// Compositional layer-wise crossover followed by mutation.
    pub fn crossover(&self, other: &Self, rng: &mut Rng) -> Self {
        let child = self.crossover_homologous_graph(other, rng);
        child.mutate(rng)
    }

    /// Canonical list of all 11 3D topological families (7 Wave-1 + 4 Wave-2).
    pub const CANONICAL_11_FAMILIES: [&'static str; 11] = [
        "baseline-ct35",
        "helical-pitched-ribbon",
        "bifurcated-quad-manifold",
        "trifurcated-6jet-vortex-head",
        "spiral-rifled-vortex-head",
        "contoured-s-cone-skimmer",
        "hybrid-omni-exotic",
        "cylindrical-gyroid-coalescer",
        "schwarz-p-rectifier-head",
        "octa-branch-helical-manifold",
        "counter-fluted-recovery-vortex",
    ];

    /// Sample a random 3D genome targeting one of the 11 canonical topological families (`family_idx in 0..11`).
    pub fn sample_stratified_11family(rng: &mut Rng, family_idx: usize) -> Self {
        let mut c = Self::baseline_ct35();
        match family_idx % 11 {
            0 => c,
            1 => {
                c.inlet_ramp = InletRamp3D {
                    axial_drop_mm: round4(rng.range(2.2, 7.6)),
                    radial_slot_factor: round4(rng.range(0.82, 0.96)),
                    superellipse_exponent: round4(rng.range(2.2, 3.6)),
                };
                c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
            }
            2 => {
                c.inlet_split = InletSplit3D {
                    enabled: true,
                    vane_thickness_mm: round4(rng.range(1.10, 1.40)),
                    vane_z_fraction: round4(rng.range(0.43, 0.57)),
                    branches_per_port: 2,
                    azimuthal_phase_deg: round4(rng.range(5.0, 36.0)),
                };
                c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
            }
            3 => {
                c.inlet_ramp.axial_drop_mm = round4(rng.range(3.5, 7.5));
                c.inlet_split = InletSplit3D {
                    enabled: true,
                    vane_thickness_mm: round4(rng.range(1.10, 1.38)),
                    vane_z_fraction: round4(rng.range(0.44, 0.56)),
                    branches_per_port: 3,
                    azimuthal_phase_deg: round4(rng.range(10.0, 38.0)),
                };
                c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
            }
            4 => {
                c.chamber_modes = vec![ChamberMode3D {
                    azimuthal_order: 2 + (rng.next_u64() % 3) as u32,
                    cos_mm: round4(rng.range(0.35, 0.78)),
                    sin_mm: round4(rng.range(-0.45, 0.45)),
                    twist_rad: round4(rng.range(-5.2, -1.8)),
                }];
                c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
            }
            5 => {
                c.cone_profile = ConeProfile3D {
                    mode1_mm: round4(rng.range(0.55, 1.10)),
                    mode2_mm: round4(rng.range(-0.38, -0.12)),
                    mode3_mm: round4(rng.range(-0.08, 0.10)),
                };
                c.skimmer = Skimmer3D {
                    length_mm: round4(rng.range(4.0, 8.8)),
                    outer_radius_mm: round4(rng.range(2.15, 2.55)),
                    capture_flutes: if rng.f64() < 0.5 { 4 } else { 6 },
                };
                c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
            }
            6 => {
                c.inlet_ramp = InletRamp3D {
                    axial_drop_mm: round4(rng.range(4.0, 7.6)),
                    radial_slot_factor: round4(rng.range(0.83, 0.94)),
                    superellipse_exponent: round4(rng.range(2.3, 3.5)),
                };
                c.inlet_split = InletSplit3D {
                    enabled: true,
                    vane_thickness_mm: round4(rng.range(1.12, 1.36)),
                    vane_z_fraction: round4(rng.range(0.44, 0.56)),
                    branches_per_port: 2,
                    azimuthal_phase_deg: round4(rng.range(12.0, 34.0)),
                };
                c.chamber_modes = vec![ChamberMode3D {
                    azimuthal_order: 3,
                    cos_mm: round4(rng.range(0.35, 0.72)),
                    sin_mm: round4(rng.range(-0.35, 0.35)),
                    twist_rad: round4(rng.range(-4.8, -2.0)),
                }];
                c.cone_profile = ConeProfile3D {
                    mode1_mm: round4(rng.range(0.55, 1.02)),
                    mode2_mm: round4(rng.range(-0.32, -0.12)),
                    mode3_mm: round4(rng.range(-0.06, 0.08)),
                };
                c.skimmer = Skimmer3D {
                    length_mm: round4(rng.range(4.8, 8.8)),
                    outer_radius_mm: round4(rng.range(2.15, 2.48)),
                    capture_flutes: 6,
                };
                c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
            }
            7 => {
                c.inlet_ramp = InletRamp3D {
                    axial_drop_mm: round4(rng.range(5.0, 7.5)),
                    radial_slot_factor: round4(rng.range(0.84, 0.93)),
                    superellipse_exponent: round4(rng.range(2.4, 3.4)),
                };
                c.inlet_split = InletSplit3D {
                    enabled: true,
                    vane_thickness_mm: round4(rng.range(1.12, 1.34)),
                    vane_z_fraction: round4(rng.range(0.45, 0.55)),
                    branches_per_port: 3,
                    azimuthal_phase_deg: round4(rng.range(16.0, 34.0)),
                };
                let zs = round4(rng.range(20.0, 30.0));
                c.tpms_insert = TpmsInsert3D {
                    basis: if rng.f64() < 0.6 {
                        TpmsBasis::CylindricalGyroid
                    } else {
                        TpmsBasis::Gyroid
                    },
                    radial_inner_fraction: round4(rng.range(0.68, 0.78)),
                    axial_start_mm: zs,
                    axial_end_mm: round4(zs + rng.range(30.0, 40.0)),
                    radial_wavelength_mm: round4(rng.range(8.0, 11.2)),
                    axial_wavelength_mm: round4(rng.range(8.5, 11.5)),
                    sheet_thickness_mm: round4(rng.range(1.28, 1.55)),
                };
                c.skimmer = Skimmer3D {
                    length_mm: round4(rng.range(5.5, 8.5)),
                    outer_radius_mm: round4(rng.range(2.18, 2.42)),
                    capture_flutes: 6,
                };
                c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
            }
            8 => {
                c.inlet_ramp = InletRamp3D {
                    axial_drop_mm: round4(rng.range(4.8, 7.4)),
                    radial_slot_factor: round4(rng.range(0.85, 0.94)),
                    superellipse_exponent: round4(rng.range(2.3, 3.3)),
                };
                c.inlet_split = InletSplit3D {
                    enabled: true,
                    vane_thickness_mm: round4(rng.range(1.14, 1.36)),
                    vane_z_fraction: round4(rng.range(0.45, 0.55)),
                    branches_per_port: 2,
                    azimuthal_phase_deg: round4(rng.range(14.0, 32.0)),
                };
                let zs = round4(rng.range(22.0, 30.0));
                c.tpms_insert = TpmsInsert3D {
                    basis: if rng.f64() < 0.6 {
                        TpmsBasis::SchwarzP
                    } else {
                        TpmsBasis::Diamond
                    },
                    radial_inner_fraction: round4(rng.range(0.70, 0.80)),
                    axial_start_mm: zs,
                    axial_end_mm: round4(zs + rng.range(28.0, 38.0)),
                    radial_wavelength_mm: round4(rng.range(8.2, 11.4)),
                    axial_wavelength_mm: round4(rng.range(8.5, 11.8)),
                    sheet_thickness_mm: round4(rng.range(1.30, 1.58)),
                };
                c.skimmer = Skimmer3D {
                    length_mm: round4(rng.range(5.2, 8.2)),
                    outer_radius_mm: round4(rng.range(2.18, 2.45)),
                    capture_flutes: 4,
                };
                c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
            }
            9 => {
                c.inlet_ramp = InletRamp3D {
                    axial_drop_mm: round4(rng.range(5.4, 7.6)),
                    radial_slot_factor: round4(rng.range(0.84, 0.92)),
                    superellipse_exponent: round4(rng.range(2.5, 3.5)),
                };
                c.inlet_split = InletSplit3D {
                    enabled: true,
                    vane_thickness_mm: round4(rng.range(1.10, 1.30)),
                    vane_z_fraction: round4(rng.range(0.45, 0.55)),
                    branches_per_port: 4,
                    azimuthal_phase_deg: round4(rng.range(18.0, 38.0)),
                };
                c.chamber_modes = vec![ChamberMode3D {
                    azimuthal_order: 4,
                    cos_mm: round4(rng.range(0.35, 0.68)),
                    sin_mm: round4(rng.range(-0.38, 0.38)),
                    twist_rad: round4(rng.range(-4.8, -2.2)),
                }];
                c.skimmer = Skimmer3D {
                    length_mm: round4(rng.range(6.0, 8.8)),
                    outer_radius_mm: round4(rng.range(2.18, 2.40)),
                    capture_flutes: 8,
                };
                c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
            }
            _ => {
                c.inlet_ramp = InletRamp3D {
                    axial_drop_mm: round4(rng.range(5.2, 7.5)),
                    radial_slot_factor: round4(rng.range(0.84, 0.93)),
                    superellipse_exponent: round4(rng.range(2.4, 3.4)),
                };
                c.inlet_split = InletSplit3D {
                    enabled: true,
                    vane_thickness_mm: round4(rng.range(1.12, 1.34)),
                    vane_z_fraction: round4(rng.range(0.45, 0.55)),
                    branches_per_port: 3,
                    azimuthal_phase_deg: round4(rng.range(16.0, 34.0)),
                };
                c.ribbon_cascade = HelicalRibbon3D {
                    vane_count: if rng.f64() < 0.5 { 3 } else { 4 },
                    axial_span_mm: round4(rng.range(22.0, 36.0)),
                    helical_twist_rad: round4(rng.range(-3.5, -1.8)),
                    thickness_mm: round4(rng.range(1.18, 1.42)),
                    radial_chord_mm: round4(rng.range(1.90, 2.90)),
                };
                c.skimmer = Skimmer3D {
                    length_mm: round4(rng.range(5.8, 8.8)),
                    outer_radius_mm: round4(rng.range(2.16, 2.42)),
                    capture_flutes: if rng.f64() < 0.5 { 6 } else { 8 },
                };
                c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
            }
        }
    }

    /// Compute a dimensionless 3D topological exoticness score measuring non-axisymmetric branching,
    /// TPMS lattice grading, helical guide-ribbon twist, harmonic chamber rifling, and S-cone inflection.
    pub fn exoticness_index(&self) -> Result<f64, String> {
        let p = self.normalize()?;
        let desc = p.descriptors()?;
        let branch_score = if p.inlet_split.enabled {
            (p.inlet_split.branches_per_port as f64 - 1.0) * 0.28
        } else {
            0.0
        };
        let tpms_score = match p.tpms_insert.basis {
            TpmsBasis::None => 0.0,
            TpmsBasis::CylindricalGyroid => 0.85,
            TpmsBasis::Gyroid => 0.78,
            TpmsBasis::SchwarzP => 0.74,
            TpmsBasis::Diamond => 0.72,
        };
        let ribbon_score = if !p.ribbon_cascade.is_inactive() {
            0.42 * (p.ribbon_cascade.vane_count as f64 / 4.0)
                + 0.18 * (p.ribbon_cascade.helical_twist_rad.abs() / 3.5)
        } else {
            0.0
        };
        let mode_score = 0.25 * desc.wall_rifling_stabilization
            + 0.20 * (1.0 - desc.shear_mitigation_factor).max(0.0);
        let skim_score = 0.12 * (p.skimmer.capture_flutes as f64 / 8.0);
        Ok(round6(
            branch_score + tpms_score + ribbon_score + mode_score + skim_score,
        ))
    }

    /// Run a stratified 11-family random-genome sampling and `canonicalize_and_preflight` expressiveness audit.
    pub fn audit_11family_genome_expressiveness(
        n_per_family: usize,
        seed: u64,
    ) -> Result<GenomeExpressivenessAudit, String> {
        let mut rng = Rng::new(seed);
        let mut family_stats = Vec::with_capacity(11);
        let mut candidates_pool: Vec<ExoticPhenotypeShowcase> = Vec::new();
        let mut total_sampled = 0usize;
        let mut total_passed = 0usize;

        for (f_idx, &fam_name) in Self::CANONICAL_11_FAMILIES.iter().enumerate() {
            let mut sampled = 0usize;
            let mut passed = 0usize;
            let mut exotic_sum = 0.0;
            let mut sym_sum = 0.0;
            let mut min_clearance = f64::INFINITY;
            let mut min_jacobian = f64::INFINITY;

            for s_idx in 0..n_per_family {
                let prog = Self::sample_stratified_11family(&mut rng, f_idx);
                sampled += 1;
                total_sampled += 1;
                if let Ok((norm, pheno, mesh)) = prog.canonicalize_and_preflight() {
                    let census = mesh.edge_census();
                    if census.open_boundary_edges == 0
                        && census.non_manifold_edges == 0
                        && mesh.signed_volume() > 0.0
                    {
                        passed += 1;
                        total_passed += 1;
                        let ex = norm.exoticness_index()?;
                        exotic_sum += ex;
                        sym_sum += pheno.descriptors.azimuthal_symmetry_order;
                        min_clearance = min_clearance.min(pheno.minimum_wall_clearance_mm);
                        min_jacobian = min_jacobian.min(pheno.min_section_jacobian);
                        if f_idx > 0 && s_idx < 4 {
                            candidates_pool.push(ExoticPhenotypeShowcase {
                                id: format!("exotic-{fam_name}-{s_idx:02}"),
                                topology_class: norm.classify_topology(),
                                genome_sha256: norm.canonical_sha256()?,
                                exoticness_index: ex,
                                total_entry_jets: if norm.inlet_split.enabled {
                                    2 * norm.inlet_split.branches_per_port
                                } else {
                                    2
                                },
                                tpms_basis: format!("{:?}", norm.tpms_insert.basis),
                                ribbon_vanes: norm.ribbon_cascade.vane_count,
                                skimmer_flutes: norm.skimmer.capture_flutes,
                                azimuthal_symmetry_order: pheno
                                    .descriptors
                                    .azimuthal_symmetry_order,
                                minimum_wall_clearance_mm: round4(pheno.minimum_wall_clearance_mm),
                                min_section_jacobian: round6(pheno.min_section_jacobian),
                            });
                        }
                    }
                }
            }

            let denom = passed.max(1) as f64;
            family_stats.push(FamilySamplingStats {
                topology_class: fam_name.to_string(),
                sampled_count: sampled,
                preflight_passed_count: passed,
                preflight_pass_fraction: round6(passed as f64 / sampled.max(1) as f64),
                mean_exoticness_index: round6(exotic_sum / denom),
                mean_azimuthal_symmetry_order: round6(sym_sum / denom),
                min_wall_clearance_mm: round4(min_clearance),
                min_section_jacobian: round6(min_jacobian),
            });
        }

        // Select top 8 most exotic valid phenotypes with at most 1 per family first to maximize topological diversity
        candidates_pool.sort_by(|a, b| b.exoticness_index.total_cmp(&a.exoticness_index));
        let mut top_8 = Vec::with_capacity(8);
        let mut used_fams = std::collections::BTreeSet::new();
        for c in &candidates_pool {
            if used_fams.insert(c.topology_class.clone()) {
                top_8.push(c.clone());
                if top_8.len() == 8 {
                    break;
                }
            }
        }
        top_8.sort_by(|a, b| b.exoticness_index.total_cmp(&a.exoticness_index));

        Ok(GenomeExpressivenessAudit {
            seed,
            n_per_family,
            total_sampled,
            total_preflight_passed: total_passed,
            overall_preflight_pass_fraction: round6(
                total_passed as f64 / total_sampled.max(1) as f64,
            ),
            family_stats,
            top_8_exotic_valid_phenotypes: top_8,
        })
    }

    /// Run a chained multi-step mutation + crossover random-walk stress audit starting from each of
    /// the 11 canonical families (`steps_per_family >= 20`, verifying `canonicalize_and_preflight`
    /// and `open_boundary_edges == 0` / `non_manifold_edges == 0` on every intermediate offspring).
    pub fn audit_11family_operator_random_walk(
        steps_per_family: usize,
        seed: u64,
    ) -> Result<OperatorRandomWalkAudit, String> {
        if steps_per_family < 20 {
            return Err("steps_per_family must be >= 20 for operator random-walk audit".into());
        }
        let mut rng = Rng::new(seed);
        let initial_pool: Vec<Self> = (0..11)
            .map(|f_idx| Self::sample_stratified_11family(&mut rng, f_idx))
            .collect();

        let mut family_walk_stats = Vec::with_capacity(11);
        let mut total_steps = 0usize;
        let mut total_passed = 0usize;
        let mut total_family_transitions = 0usize;
        let mut global_visited_families = std::collections::BTreeSet::new();

        for (f_idx, &start_family) in Self::CANONICAL_11_FAMILIES.iter().enumerate() {
            let mut current = initial_pool[f_idx].clone();
            let mut prev_fam = current.classify_topology();
            global_visited_families.insert(prev_fam.clone());
            let mut local_visited = std::collections::BTreeSet::new();
            local_visited.insert(prev_fam.clone());

            let mut steps_passed = 0usize;
            let mut transitions = 0usize;
            let mut max_open_edges = 0usize;
            let mut max_non_manifold_edges = 0usize;
            let mut min_clearance = f64::INFINITY;
            let mut min_jacobian = f64::INFINITY;
            let mut exotic_sum = 0.0;

            for step in 0..steps_per_family {
                let partner_idx = ((rng.next_u64() as usize) + step + f_idx + 1) % 11;
                let partner = &initial_pool[partner_idx];
                current = match step % 5 {
                    0 => current.mutate_diffeomorphic_spline(&mut rng),
                    1 => current.mutate_branch_topology(&mut rng),
                    2 => current.mutate_tpms_grading(&mut rng),
                    3 => current.crossover_homologous_graph(partner, &mut rng),
                    _ => current.crossover(partner, &mut rng),
                };
                total_steps += 1;
                if let Ok((norm, pheno, mesh)) = current.canonicalize_and_preflight() {
                    let census = mesh.edge_census();
                    max_open_edges = max_open_edges.max(census.open_boundary_edges);
                    max_non_manifold_edges = max_non_manifold_edges.max(census.non_manifold_edges);
                    if census.open_boundary_edges == 0
                        && census.non_manifold_edges == 0
                        && mesh.signed_volume() > 0.0
                    {
                        steps_passed += 1;
                        total_passed += 1;
                        let cur_fam = norm.classify_topology();
                        if cur_fam != prev_fam {
                            transitions += 1;
                            total_family_transitions += 1;
                            prev_fam = cur_fam.clone();
                        }
                        local_visited.insert(cur_fam.clone());
                        global_visited_families.insert(cur_fam);
                        min_clearance = min_clearance.min(pheno.minimum_wall_clearance_mm);
                        min_jacobian = min_jacobian.min(pheno.min_section_jacobian);
                        exotic_sum += norm.exoticness_index()?;
                    }
                }
            }

            let denom = steps_passed.max(1) as f64;
            family_walk_stats.push(FamilyRandomWalkStats {
                start_topology_class: start_family.to_string(),
                steps_executed: steps_per_family,
                steps_passed,
                pass_fraction: round6(steps_passed as f64 / steps_per_family as f64),
                family_transitions: transitions,
                distinct_families_visited: local_visited.len(),
                max_open_edges,
                max_non_manifold_edges,
                min_wall_clearance_mm: round4(min_clearance),
                min_section_jacobian: round6(min_jacobian),
                mean_exoticness_index: round6(exotic_sum / denom),
            });
        }

        Ok(OperatorRandomWalkAudit {
            seed,
            steps_per_family,
            total_steps,
            total_passed,
            overall_pass_fraction: round6(total_passed as f64 / total_steps.max(1) as f64),
            total_family_transitions,
            global_distinct_families_visited: global_visited_families.len(),
            family_walk_stats,
        })
    }

    /// Generate the shared unseeded Generation-0 initial population for Wave-3 de-novo evolution.
    /// Contains zero pre-baked 6-jet, 8-jet, ribbon, or TPMS elites: all individuals are mild
    /// perturbations around `CT-35` so every evolutionary improvement must be discovered de novo.
    pub fn init_wave3_de_novo_gen0(pop_size: usize, rng: &mut Rng) -> Vec<Self> {
        let mut pop = Vec::with_capacity(pop_size.max(1));
        let base = Self::baseline_ct35();
        pop.push(base.clone());
        for i in 1..pop_size {
            let mut c = base.clone();
            c.inlet_ramp = InletRamp3D {
                axial_drop_mm: round4(rng.range(0.0, 1.85)),
                radial_slot_factor: round4(rng.range(0.92, 1.01)),
                superellipse_exponent: round4(rng.range(2.0, 2.35)),
            };
            c.cone_profile = ConeProfile3D {
                mode1_mm: round4(rng.range(-0.18, 0.32)),
                mode2_mm: round4(rng.range(-0.15, 0.12)),
                mode3_mm: round4(rng.range(-0.04, 0.04)),
            };
            c.skimmer = Skimmer3D {
                length_mm: round4(rng.range(0.0, 2.20)),
                outer_radius_mm: round4(rng.range(2.25, 2.55)),
                capture_flutes: if i % 6 == 0 { 4 } else { 0 },
            };
            if i % 5 == 0 {
                c.inlet_split = InletSplit3D {
                    enabled: true,
                    vane_thickness_mm: round4(rng.range(1.18, 1.32)),
                    vane_z_fraction: round4(rng.range(0.46, 0.54)),
                    branches_per_port: 2,
                    azimuthal_phase_deg: round4(rng.range(4.0, 14.0)),
                };
            }
            if i % 7 == 0 {
                c.chamber_modes = vec![ChamberMode3D {
                    azimuthal_order: 2,
                    cos_mm: round4(rng.range(0.18, 0.42)),
                    sin_mm: round4(rng.range(-0.25, 0.25)),
                    twist_rad: round4(rng.range(-2.6, -1.2)),
                }];
            }
            pop.push(c.normalize().unwrap_or_else(|_| base.clone()));
        }
        pop
    }

    /// Strictly zero-mean unbiased local & single-step topological transition operator for Wave-3
    /// de-novo evolution (used identically by `B3_Wave3_Exotic3D_QD_CEI` and `A2_No_Selection_Random_Walk`).
    /// Because all continuous perturbations have mean `0.0` and topological transitions are symmetric,
    /// directional improvement across generations can only occur via evolutionary selection.
    pub fn mutate_wave3_progressive(&self, rng: &mut Rng) -> Self {
        let mut c = self.normalize().unwrap_or_else(|_| Self::baseline_ct35());

        // 1. Zero-mean continuous ramp step
        c.inlet_ramp.axial_drop_mm =
            round4((c.inlet_ramp.axial_drop_mm + rng.gauss(0.0, 0.65)).clamp(0.0, 7.8));
        c.inlet_ramp.radial_slot_factor =
            round4((c.inlet_ramp.radial_slot_factor + rng.gauss(0.0, 0.020)).clamp(0.80, 1.01));
        c.inlet_ramp.superellipse_exponent =
            round4((c.inlet_ramp.superellipse_exponent + rng.gauss(0.0, 0.20)).clamp(2.0, 3.8));

        // 2. Symmetric single-step branch topology transition: off <-> 2 <-> 3 <-> 4 branches/port
        if !c.inlet_split.enabled {
            if rng.f64() < 0.22 {
                c.inlet_split = InletSplit3D {
                    enabled: true,
                    vane_thickness_mm: round4(rng.range(1.14, 1.32)),
                    vane_z_fraction: round4(rng.range(0.46, 0.54)),
                    branches_per_port: 2,
                    azimuthal_phase_deg: round4(rng.range(6.0, 18.0)),
                };
            }
        } else {
            c.inlet_split.vane_thickness_mm =
                round4((c.inlet_split.vane_thickness_mm + rng.gauss(0.0, 0.035)).clamp(1.08, 1.45));
            c.inlet_split.vane_z_fraction =
                round4((c.inlet_split.vane_z_fraction + rng.gauss(0.0, 0.02)).clamp(0.42, 0.58));
            c.inlet_split.azimuthal_phase_deg =
                round4((c.inlet_split.azimuthal_phase_deg + rng.gauss(0.0, 3.5)).clamp(0.0, 40.0));
            let u = rng.f64();
            if u < 0.20 {
                c.inlet_split.branches_per_port = (c.inlet_split.branches_per_port + 1).min(4);
            } else if u < 0.40 {
                if c.inlet_split.branches_per_port <= 2 {
                    c.inlet_split.enabled = false;
                } else {
                    c.inlet_split.branches_per_port -= 1;
                }
            }
        }

        // 3. Symmetric helical ribbon cascade step
        if c.inlet_split.enabled && rng.f64() < 0.20 {
            if c.ribbon_cascade.is_inactive() {
                c.ribbon_cascade = HelicalRibbon3D {
                    vane_count: 3,
                    axial_span_mm: round4(rng.range(18.0, 28.0)),
                    helical_twist_rad: round4(rng.range(-2.6, -1.4)),
                    thickness_mm: round4(rng.range(1.20, 1.38)),
                    radial_chord_mm: round4(rng.range(1.85, 2.50)),
                };
            } else if rng.f64() < 0.35 {
                c.ribbon_cascade = HelicalRibbon3D::default();
            } else {
                c.ribbon_cascade.vane_count = if rng.f64() < 0.50 { 3 } else { 4 };
                c.ribbon_cascade.axial_span_mm = round4(
                    (c.ribbon_cascade.axial_span_mm + rng.gauss(0.0, 2.5)).clamp(14.0, 40.0),
                );
                c.ribbon_cascade.helical_twist_rad = round4(
                    (c.ribbon_cascade.helical_twist_rad + rng.gauss(0.0, 0.25)).clamp(-3.9, -1.0),
                );
            }
        }

        // 4. Symmetric TPMS coalescer discovery & zero-mean grading refinement
        if c.inlet_split.enabled && c.inlet_split.branches_per_port >= 2 && rng.f64() < 0.22 {
            if c.tpms_insert.is_inactive() {
                let basis = if rng.f64() < 0.50 {
                    TpmsBasis::SchwarzP
                } else {
                    TpmsBasis::Diamond
                };
                c.tpms_insert = TpmsInsert3D {
                    basis,
                    radial_inner_fraction: round4(rng.range(0.72, 0.79)),
                    axial_start_mm: round4(rng.range(23.0, 28.0)),
                    axial_end_mm: round4(rng.range(54.0, 62.0)),
                    radial_wavelength_mm: round4(rng.range(9.0, 11.5)),
                    axial_wavelength_mm: round4(rng.range(9.2, 11.8)),
                    sheet_thickness_mm: round4(rng.range(1.34, 1.58)),
                };
            } else if rng.f64() < 0.30 {
                c.tpms_insert = TpmsInsert3D::default();
            } else {
                if rng.f64() < 0.35 {
                    c.tpms_insert.basis = match rng.next_u64() % 4 {
                        0 => TpmsBasis::SchwarzP,
                        1 => TpmsBasis::Diamond,
                        2 => TpmsBasis::Gyroid,
                        _ => TpmsBasis::CylindricalGyroid,
                    };
                }
                c.tpms_insert.radial_inner_fraction = round4(
                    (c.tpms_insert.radial_inner_fraction + rng.gauss(0.0, 0.020)).clamp(0.66, 0.80),
                );
                c.tpms_insert.radial_wavelength_mm = round4(
                    (c.tpms_insert.radial_wavelength_mm + rng.gauss(0.0, 0.38)).clamp(7.5, 12.0),
                );
                c.tpms_insert.axial_wavelength_mm = round4(
                    (c.tpms_insert.axial_wavelength_mm + rng.gauss(0.0, 0.38)).clamp(8.0, 12.2),
                );
                c.tpms_insert.sheet_thickness_mm = round4(
                    (c.tpms_insert.sheet_thickness_mm + rng.gauss(0.0, 0.035)).clamp(1.24, 1.75),
                );
            }
        }

        // 5. Zero-mean diffeomorphic cone & chamber harmonic step
        let mut m1 = (c.cone_profile.mode1_mm + rng.gauss(0.0, 0.12)).clamp(-0.55, 1.12);
        let mut m2 = (c.cone_profile.mode2_mm + rng.gauss(0.0, 0.08)).clamp(-0.38, 0.32);
        let mut m3 = (c.cone_profile.mode3_mm + rng.gauss(0.0, 0.04)).clamp(-0.14, 0.14);
        let sum_m = m1.abs() + m2.abs() + m3.abs();
        if sum_m > 1.45 {
            let s = 1.45 / sum_m;
            m1 *= s;
            m2 *= s;
            m3 *= s;
        }
        c.cone_profile = ConeProfile3D {
            mode1_mm: round4(m1),
            mode2_mm: round4(m2),
            mode3_mm: round4(m3),
        };

        if c.chamber_modes.is_empty() && rng.f64() < 0.25 {
            c.chamber_modes.push(ChamberMode3D {
                azimuthal_order: 2 + (rng.next_u64() % 3) as u32,
                cos_mm: round4(rng.range(-0.45, 0.55)),
                sin_mm: round4(rng.range(-0.45, 0.45)),
                twist_rad: round4(rng.range(-4.2, -1.5)),
            });
        } else if !c.chamber_modes.is_empty() {
            if rng.f64() < 0.20 {
                c.chamber_modes.clear();
            } else {
                for m in &mut c.chamber_modes {
                    m.cos_mm = round4((m.cos_mm + rng.gauss(0.0, 0.08)).clamp(-0.75, 0.78));
                    m.sin_mm = round4((m.sin_mm + rng.gauss(0.0, 0.08)).clamp(-0.75, 0.78));
                    m.twist_rad = round4((m.twist_rad + rng.gauss(0.0, 0.25)).clamp(-5.6, -0.8));
                }
            }
        }

        // 6. Zero-mean skimmer sleeve step
        c.skimmer.length_mm = round4((c.skimmer.length_mm + rng.gauss(0.0, 0.75)).clamp(0.0, 9.6));
        if c.skimmer.length_mm > 0.1 {
            c.skimmer.outer_radius_mm =
                round4((c.skimmer.outer_radius_mm + rng.gauss(0.0, 0.05)).clamp(2.12, 3.10));
            if rng.f64() < 0.25 {
                c.skimmer.capture_flutes = match rng.next_u64() % 4 {
                    0 => 0,
                    1 => 4,
                    2 => 6,
                    _ => 8,
                };
            }
        }

        c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FamilyRandomWalkStats {
    pub start_topology_class: String,
    pub steps_executed: usize,
    pub steps_passed: usize,
    pub pass_fraction: f64,
    pub family_transitions: usize,
    pub distinct_families_visited: usize,
    pub max_open_edges: usize,
    pub max_non_manifold_edges: usize,
    pub min_wall_clearance_mm: f64,
    pub min_section_jacobian: f64,
    pub mean_exoticness_index: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorRandomWalkAudit {
    pub seed: u64,
    pub steps_per_family: usize,
    pub total_steps: usize,
    pub total_passed: usize,
    pub overall_pass_fraction: f64,
    pub total_family_transitions: usize,
    pub global_distinct_families_visited: usize,
    pub family_walk_stats: Vec<FamilyRandomWalkStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FamilySamplingStats {
    pub topology_class: String,
    pub sampled_count: usize,
    pub preflight_passed_count: usize,
    pub preflight_pass_fraction: f64,
    pub mean_exoticness_index: f64,
    pub mean_azimuthal_symmetry_order: f64,
    pub min_wall_clearance_mm: f64,
    pub min_section_jacobian: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExoticPhenotypeShowcase {
    pub id: String,
    pub topology_class: String,
    pub genome_sha256: String,
    pub exoticness_index: f64,
    pub total_entry_jets: u32,
    pub tpms_basis: String,
    pub ribbon_vanes: u32,
    pub skimmer_flutes: u32,
    pub azimuthal_symmetry_order: f64,
    pub minimum_wall_clearance_mm: f64,
    pub min_section_jacobian: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GenomeExpressivenessAudit {
    pub seed: u64,
    pub n_per_family: usize,
    pub total_sampled: usize,
    pub total_preflight_passed: usize,
    pub overall_preflight_pass_fraction: f64,
    pub family_stats: Vec<FamilySamplingStats>,
    pub top_8_exotic_valid_phenotypes: Vec<ExoticPhenotypeShowcase>,
}
