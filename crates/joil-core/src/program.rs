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
        if self.schema_version != 2 || self.family != "ct35-exotic-head-v2" {
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
            if !(2..=3).contains(&branches) {
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
            if !matches!(flutes, 0 | 4 | 6) {
                return Err("skimmer.capture_flutes must be 0, 4, or 6".into());
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

    /// True if all exotic perturbations vanish and the design is the exact CT-35 baseline.
    pub fn is_exact_baseline(&self) -> Result<bool, String> {
        let p = self.normalize()?;
        Ok(p.inlet_ramp.axial_drop_mm.abs() < 1e-12
            && (p.inlet_ramp.radial_slot_factor - 1.0).abs() < 1e-12
            && !p.inlet_split.enabled
            && p.chamber_modes.is_empty()
            && p.cone_profile.mode1_mm.abs() < 1e-12
            && p.cone_profile.mode2_mm.abs() < 1e-12
            && p.cone_profile.mode3_mm.abs() < 1e-12
            && p.skimmer.length_mm.abs() < 1e-12)
    }

    /// Classify the candidate into one of 7 canonical 3D topological families.
    pub fn classify_topology(&self) -> String {
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
        let swirl_flux_idx = (r_centroid_new / r_centroid_base)
            * (1.0 + 0.38 * (jet_accel - 1.0))
            * (1.0 + 0.015 * (se_exp - 2.0));

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
        let symmetry_order = base_sym + (0.2 * mode_amp).min(0.45);

        let mut shear_mitigation = 1.0 - 0.22 * (drop_mm / 8.0) - 0.14 * (m1 / 1.2).max(0.0)
            + 0.35 * (((0.85 - slot_factor) / 0.15).max(0.0)).powi(2)
            - 0.02 * (se_exp - 2.0);
        if split_enabled {
            shear_mitigation *= if p.inlet_split.branches_per_port == 3 {
                0.84
            } else {
                0.88
            };
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
        let wall_rifling_stab = helical_alignment.min(1.25);

        let hydraulic_resistance_factor = 1.0 - 0.065 * (drop_mm / 8.0) - 0.055 * (m1 / 1.2)
            + 0.025 * (mode_amp / 1.5)
            + if split_enabled {
                0.014 * (branches - 1.0)
            } else {
                0.0
            }
            + 0.12 * (1.0 - slot_factor).max(0.0);

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

    /// Generate initial diverse 3D archetype seeds spanning all 7 topological families.
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

    /// Structure-aware Gaussian and topological 3D mutation operator respecting >= 1.0 mm clearance.
    pub fn mutate(&self, rng: &mut Rng) -> Self {
        let mut c = self.normalize().unwrap_or_else(|_| Self::baseline_ct35());
        c.inlet_ramp.axial_drop_mm =
            round4((c.inlet_ramp.axial_drop_mm + rng.gauss(0.0, 0.85)).clamp(0.0, 7.9));
        c.inlet_ramp.radial_slot_factor =
            round4((c.inlet_ramp.radial_slot_factor + rng.gauss(0.0, 0.025)).clamp(0.78, 1.0));
        c.inlet_ramp.superellipse_exponent =
            round4((c.inlet_ramp.superellipse_exponent + rng.gauss(0.0, 0.25)).clamp(2.0, 3.8));

        if rng.f64() < 0.25 {
            c.inlet_split.enabled = !c.inlet_split.enabled;
        }
        if c.inlet_split.enabled {
            c.inlet_split.vane_thickness_mm =
                round4((c.inlet_split.vane_thickness_mm + rng.gauss(0.0, 0.06)).clamp(1.05, 1.50));
            c.inlet_split.vane_z_fraction =
                round4((c.inlet_split.vane_z_fraction + rng.gauss(0.0, 0.03)).clamp(0.40, 0.60));
            if rng.f64() < 0.20 {
                c.inlet_split.branches_per_port = if c.inlet_split.branches_per_port == 2 {
                    3
                } else {
                    2
                };
            } else if c.inlet_split.branches_per_port < 2 {
                c.inlet_split.branches_per_port = 2;
            }
            c.inlet_split.azimuthal_phase_deg =
                round4((c.inlet_split.azimuthal_phase_deg + rng.gauss(0.0, 4.0)).clamp(0.0, 40.0));
        }

        let mut m1 = (c.cone_profile.mode1_mm + rng.gauss(0.0, 0.14)).clamp(-0.60, 1.15);
        let mut m2 = (c.cone_profile.mode2_mm + rng.gauss(0.0, 0.10)).clamp(-0.40, 0.35);
        let mut m3 = (c.cone_profile.mode3_mm + rng.gauss(0.0, 0.05)).clamp(-0.15, 0.15);
        let sum_m = m1.abs() + m2.abs() + m3.abs();
        if sum_m > 1.52 {
            let s = 1.52 / sum_m;
            m1 *= s;
            m2 *= s;
            m3 *= s;
        }
        c.cone_profile = ConeProfile3D {
            mode1_mm: round4(m1),
            mode2_mm: round4(m2),
            mode3_mm: round4(m3),
        };

        c.skimmer.length_mm = round4((c.skimmer.length_mm + rng.gauss(0.0, 0.90)).clamp(0.0, 9.8));
        if c.skimmer.length_mm > 0.1 {
            c.skimmer.outer_radius_mm =
                round4((c.skimmer.outer_radius_mm + rng.gauss(0.0, 0.08)).clamp(2.1, 3.2));
            if rng.f64() < 0.20 {
                c.skimmer.capture_flutes = match (rng.next_u64() % 3) as u32 {
                    0 => 0,
                    1 => 4,
                    _ => 6,
                };
            }
        }

        if c.chamber_modes.is_empty() && rng.f64() < 0.55 {
            let order = 2 + (rng.next_u64() % 3) as u32;
            c.chamber_modes.push(ChamberMode3D {
                azimuthal_order: order,
                cos_mm: round4(rng.range(0.30, 0.80)),
                sin_mm: round4(rng.range(-0.40, 0.40)),
                twist_rad: round4(rng.range(-5.5, -2.0)),
            });
        } else if !c.chamber_modes.is_empty() {
            if c.chamber_modes.len() < 3 && rng.f64() < 0.20 {
                let order = 2 + (rng.next_u64() % 5) as u32;
                if !c.chamber_modes.iter().any(|m| m.azimuthal_order == order) {
                    c.chamber_modes.push(ChamberMode3D {
                        azimuthal_order: order,
                        cos_mm: round4(rng.range(0.15, 0.45)),
                        sin_mm: round4(rng.range(-0.25, 0.25)),
                        twist_rad: round4(rng.range(-5.0, -2.2)),
                    });
                }
            }
            for m in &mut c.chamber_modes {
                m.cos_mm = round4(m.cos_mm + rng.gauss(0.0, 0.10));
                m.sin_mm = round4(m.sin_mm + rng.gauss(0.0, 0.10));
                m.twist_rad = round4(
                    (m.twist_rad + rng.gauss(0.0, 0.35)).clamp(-2.0 * PI + 0.05, 2.0 * PI - 0.05),
                );
            }
            let total_amp: f64 = c
                .chamber_modes
                .iter()
                .map(|m| m.cos_mm.hypot(m.sin_mm))
                .sum();
            if total_amp > 1.70 {
                let s = 1.70 / total_amp;
                for m in &mut c.chamber_modes {
                    m.cos_mm = round4(m.cos_mm * s);
                    m.sin_mm = round4(m.sin_mm * s);
                }
            }
        }

        c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
    }

    /// Compositional layer-wise crossover followed by mutation.
    pub fn crossover(&self, other: &Self, rng: &mut Rng) -> Self {
        let mut child = self.normalize().unwrap_or_else(|_| Self::baseline_ct35());
        let b = other.normalize().unwrap_or_else(|_| Self::baseline_ct35());
        if rng.f64() < 0.5 {
            child.inlet_ramp = b.inlet_ramp.clone();
        }
        if rng.f64() < 0.5 {
            child.inlet_split = b.inlet_split.clone();
        }
        if rng.f64() < 0.5 {
            child.cone_profile = b.cone_profile.clone();
        }
        if rng.f64() < 0.5 {
            child.skimmer = b.skimmer.clone();
        }
        if rng.f64() < 0.5 {
            child.chamber_modes = b.chamber_modes.clone();
        }
        child.mutate(rng)
    }
}
