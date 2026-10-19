//! Parameter bound validation and inactive-gene canonicalization for `DesignProgram`.

use super::*;

impl DesignProgram {
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
