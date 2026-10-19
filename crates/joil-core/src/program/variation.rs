//! Raw genetic mutation, crossover, and stratified 11-family sampling operators.

use super::*;

impl DesignProgram {
    /// Wave-2 Operator 1: Diffeomorphic / Jacobian-guarded spline & harmonic wall mutation.
    /// Analytically bounds modal amplitudes so `det g(theta, z) > 0.25` and LPBF overhang `<= 42 deg`.
    pub(crate) fn raw_mutate_diffeomorphic_spline(&self, rng: &mut Rng, projection: bool) -> Self {
        let mut c = self.clone();
        let mut m1 = project_bound(
            c.cone_profile.mode1_mm + rng.gauss(0.0, 0.14),
            -0.60,
            1.15,
            projection,
        );
        let mut m2 = project_bound(
            c.cone_profile.mode2_mm + rng.gauss(0.0, 0.10),
            -0.40,
            0.35,
            projection,
        );
        let mut m3 = project_bound(
            c.cone_profile.mode3_mm + rng.gauss(0.0, 0.05),
            -0.15,
            0.15,
            projection,
        );
        let sum_m = m1.abs() + m2.abs() + m3.abs();
        if projection && sum_m > 1.48 {
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
                m.twist_rad = round4(project_bound(
                    m.twist_rad + rng.gauss(0.0, 0.30),
                    -5.8,
                    -0.5,
                    projection,
                ));
            }
            let total_amp: f64 = c
                .chamber_modes
                .iter()
                .map(|m| m.cos_mm.hypot(m.sin_mm))
                .sum();
            if projection && total_amp > 1.60 {
                let s = 1.60 / total_amp;
                for m in &mut c.chamber_modes {
                    m.cos_mm = round4(m.cos_mm * s);
                    m.sin_mm = round4(m.sin_mm * s);
                }
            }
        }
        c
    }

    /// Wave-2 Operator 2: Area-conserving branch topology mutation (`K in {2, 4, 6, 8}` nozzles)
    /// and internal helical guide-ribbon cascade perturbation.
    pub(crate) fn raw_mutate_branch_topology(&self, rng: &mut Rng, projection: bool) -> Self {
        let mut c = self.clone();
        c.inlet_ramp.axial_drop_mm = round4(project_bound(
            c.inlet_ramp.axial_drop_mm + rng.gauss(0.0, 0.75),
            0.0,
            7.9,
            projection,
        ));
        c.inlet_ramp.radial_slot_factor = round4(project_bound(
            c.inlet_ramp.radial_slot_factor + rng.gauss(0.0, 0.022),
            0.80,
            1.0,
            projection,
        ));
        c.inlet_ramp.superellipse_exponent = round4(project_bound(
            c.inlet_ramp.superellipse_exponent + rng.gauss(0.0, 0.22),
            2.0,
            3.8,
            projection,
        ));

        if rng.f64() < 0.25 {
            c.inlet_split.enabled = !c.inlet_split.enabled;
        }
        if c.inlet_split.enabled {
            c.inlet_split.vane_thickness_mm = round4(project_bound(
                c.inlet_split.vane_thickness_mm + rng.gauss(0.0, 0.05),
                1.08,
                1.45,
                projection,
            ));
            c.inlet_split.vane_z_fraction = round4(project_bound(
                c.inlet_split.vane_z_fraction + rng.gauss(0.0, 0.025),
                0.42,
                0.58,
                projection,
            ));
            if rng.f64() < 0.30 {
                c.inlet_split.branches_per_port = match rng.next_u64() % 3 {
                    0 => 2,
                    1 => 3,
                    _ => 4,
                };
            } else if c.inlet_split.branches_per_port < 2 {
                c.inlet_split.branches_per_port = 2;
            }
            c.inlet_split.azimuthal_phase_deg = round4(project_bound(
                c.inlet_split.azimuthal_phase_deg + rng.gauss(0.0, 3.5),
                0.0,
                40.0,
                projection,
            ));
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
                c.ribbon_cascade.axial_span_mm = round4(project_bound(
                    c.ribbon_cascade.axial_span_mm + rng.gauss(0.0, 2.5),
                    14.0,
                    42.0,
                    projection,
                ));
                c.ribbon_cascade.helical_twist_rad = round4(project_bound(
                    c.ribbon_cascade.helical_twist_rad + rng.gauss(0.0, 0.25),
                    -4.0,
                    -1.0,
                    projection,
                ));
            }
        }
        c
    }

    /// Wave-2 Operator 3: Bi-continuous percolation-window TPMS grading mutation (`0` trapped voids,
    /// central vortex core `r < radial_inner_fraction * R_wall` 100% unobstructed).
    pub(crate) fn raw_mutate_tpms_grading(&self, rng: &mut Rng, projection: bool) -> Self {
        let mut c = self.clone();
        if c.tpms_insert.is_inactive() {
            let basis = match rng.next_u64() % 4 {
                0 => TpmsBasis::CylindricalGyroid,
                1 => TpmsBasis::Gyroid,
                2 => TpmsBasis::SchwarzP,
                _ => TpmsBasis::Diamond,
            };
            let zs = round4(rng.range(20.0, 32.0));
            let ze = round4(project_bound(
                zs + rng.range(28.0, 42.0),
                52.0,
                82.0,
                projection,
            ));
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
            c.tpms_insert.radial_inner_fraction = round4(project_bound(
                c.tpms_insert.radial_inner_fraction + rng.gauss(0.0, 0.02),
                0.62,
                0.82,
                projection,
            ));
            c.tpms_insert.radial_wavelength_mm = round4(project_bound(
                c.tpms_insert.radial_wavelength_mm + rng.gauss(0.0, 0.4),
                7.0,
                12.5,
                projection,
            ));
            c.tpms_insert.axial_wavelength_mm = round4(project_bound(
                c.tpms_insert.axial_wavelength_mm + rng.gauss(0.0, 0.4),
                7.0,
                12.5,
                projection,
            ));
            c.tpms_insert.sheet_thickness_mm = round4(project_bound(
                c.tpms_insert.sheet_thickness_mm + rng.gauss(0.0, 0.04),
                1.22,
                1.85,
                projection,
            ));
        }
        c
    }

    /// Legacy non-homologous macro crossover: scalar blending and whole-feature exchange.
    /// No innovation IDs or graph alignment exist in this representation.
    pub(crate) fn raw_crossover_non_homologous_macro(
        &self,
        other: &Self,
        rng: &mut Rng,
        projection: bool,
    ) -> Self {
        let _ = projection;
        let mut child = self.clone();
        let b = other.clone();
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
        child
    }

    /// Structure-aware Gaussian and topological 3D mutation operator respecting >= 1.0 mm clearance.
    pub(crate) fn raw_mutate(&self, rng: &mut Rng, projection: bool) -> Self {
        let mut c = self.raw_mutate_diffeomorphic_spline(rng, projection);
        c = c.raw_mutate_branch_topology(rng, projection);
        if rng.f64() < 0.22 {
            c = c.raw_mutate_tpms_grading(rng, projection);
        }
        c.skimmer.length_mm = round4(project_bound(
            c.skimmer.length_mm + rng.gauss(0.0, 0.85),
            0.0,
            9.8,
            projection,
        ));
        if c.skimmer.length_mm > 0.1 {
            c.skimmer.outer_radius_mm = round4(project_bound(
                c.skimmer.outer_radius_mm + rng.gauss(0.0, 0.07),
                2.1,
                3.2,
                projection,
            ));
            if rng.f64() < 0.22 {
                c.skimmer.capture_flutes = match (rng.next_u64() % 4) as u32 {
                    0 => 0,
                    1 => 4,
                    2 => 6,
                    _ => 8,
                };
            }
        }
        c
    }

    /// Mutate restricted strictly to conventional axisymmetric 2-port parameters (`Comparator B2`).
    pub(crate) fn raw_mutate_conventional_axisymmetric(
        &self,
        rng: &mut Rng,
        projection: bool,
    ) -> Self {
        let mut c = Self::baseline_ct35();
        c.schema_version = self.schema_version;
        c.family = self.family.clone();
        c.inlet_ramp.radial_slot_factor = round4(project_bound(
            self.inlet_ramp.radial_slot_factor + rng.gauss(0.0, 0.025),
            0.82,
            1.02,
            projection,
        ));
        let mut m1 = project_bound(
            self.cone_profile.mode1_mm + rng.gauss(0.0, 0.12),
            -0.55,
            0.95,
            projection,
        );
        let mut m2 = project_bound(
            self.cone_profile.mode2_mm + rng.gauss(0.0, 0.08),
            -0.35,
            0.30,
            projection,
        );
        let sum_m = m1.abs() + m2.abs();
        if projection && sum_m > 1.20 {
            let s = 1.20 / sum_m;
            m1 *= s;
            m2 *= s;
        }
        c.cone_profile = ConeProfile3D {
            mode1_mm: round4(m1),
            mode2_mm: round4(m2),
            mode3_mm: 0.0,
        };
        c.skimmer.length_mm = round4(project_bound(
            self.skimmer.length_mm + rng.gauss(0.0, 0.80),
            0.0,
            8.5,
            projection,
        ));
        if c.skimmer.length_mm > 0.1 {
            c.skimmer.outer_radius_mm = round4(project_bound(
                self.skimmer.outer_radius_mm + rng.gauss(0.0, 0.06),
                2.1,
                3.0,
                projection,
            ));
            c.skimmer.capture_flutes = 0;
        }
        c
    }
}

#[path = "variation_sampling.rs"]
mod variation_sampling;
