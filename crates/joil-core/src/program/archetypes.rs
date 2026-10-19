//! Archetype seeding, topology classification, mechanism descriptors, and public genetic operators.

use super::*;

impl DesignProgram {
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
}

#[path = "archetypes_seeding.rs"]
mod archetypes_seeding;
