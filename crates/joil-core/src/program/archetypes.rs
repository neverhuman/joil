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

    pub fn mutate_diffeomorphic_spline(
        &self,
        rng: &mut Rng,
    ) -> Result<VariationOutcome, VariationError> {
        variation(
            &[self],
            "mutate_diffeomorphic_spline",
            rng,
            |rng, projection| self.raw_mutate_diffeomorphic_spline(rng, projection),
        )
    }

    pub fn mutate_branch_topology(
        &self,
        rng: &mut Rng,
    ) -> Result<VariationOutcome, VariationError> {
        variation(&[self], "mutate_branch_topology", rng, |rng, projection| {
            self.raw_mutate_branch_topology(rng, projection)
        })
    }

    pub fn mutate_tpms_grading(&self, rng: &mut Rng) -> Result<VariationOutcome, VariationError> {
        variation(&[self], "mutate_tpms_grading", rng, |rng, projection| {
            self.raw_mutate_tpms_grading(rng, projection)
        })
    }

    pub fn crossover_non_homologous_macro(
        &self,
        other: &Self,
        rng: &mut Rng,
    ) -> Result<VariationOutcome, VariationError> {
        variation(
            &[self, other],
            "non_homologous_macro_crossover",
            rng,
            |rng, projection| self.raw_crossover_non_homologous_macro(other, rng, projection),
        )
    }

    pub fn mutate(&self, rng: &mut Rng) -> Result<VariationOutcome, VariationError> {
        variation(&[self], "mutate", rng, |rng, projection| {
            self.raw_mutate(rng, projection)
        })
    }

    pub fn mutate_conventional_axisymmetric(
        &self,
        rng: &mut Rng,
    ) -> Result<VariationOutcome, VariationError> {
        variation(
            &[self],
            "mutate_conventional_axisymmetric",
            rng,
            |rng, projection| self.raw_mutate_conventional_axisymmetric(rng, projection),
        )
    }

    pub fn sample_random_exotic(rng: &mut Rng) -> Result<VariationOutcome, VariationError> {
        variation(&[], "sample_random_exotic", rng, |rng, projection| {
            Self::raw_sample_random_exotic(rng, projection)
        })
    }

    pub fn crossover(
        &self,
        other: &Self,
        rng: &mut Rng,
    ) -> Result<VariationOutcome, VariationError> {
        variation(
            &[self, other],
            "non_homologous_macro_crossover",
            rng,
            |rng, projection| self.raw_crossover(other, rng, projection),
        )
    }

    pub fn sample_stratified_11family(
        rng: &mut Rng,
        family_idx: usize,
    ) -> Result<VariationOutcome, VariationError> {
        variation(&[], "sample_stratified_11family", rng, |rng, projection| {
            Self::raw_sample_stratified_11family(rng, projection, family_idx)
        })
    }

    pub fn mutate_wave3_progressive(
        &self,
        rng: &mut Rng,
    ) -> Result<VariationOutcome, VariationError> {
        variation(
            &[self],
            "mutate_wave3_progressive",
            rng,
            |rng, projection| self.raw_mutate_wave3_progressive(rng, projection),
        )
    }

    /// Submit an externally authored edit through the same transaction boundary.
    pub fn propose_edit(
        &self,
        proposal: Self,
        rng: &mut Rng,
    ) -> Result<VariationOutcome, VariationError> {
        variation(&[self], "authored_edit", rng, |_, _| proposal.clone())
    }
}
