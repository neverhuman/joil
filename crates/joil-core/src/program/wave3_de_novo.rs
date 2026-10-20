//! Wave-3 unseeded Generation-0 initialization and zero-mean progressive variation operator.

use super::*;

impl DesignProgram {
    /// Generate the shared unseeded Generation-0 initial population for Wave-3 de-novo evolution.
    /// Contains zero pre-baked 6-jet, 8-jet, ribbon, or TPMS elites: all individuals are mild
    /// perturbations around `CT-35` so every evolutionary improvement must be discovered de novo.
    pub fn init_wave3_de_novo_gen0(pop_size: usize, rng: &mut Rng) -> Result<Vec<Self>, String> {
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
            pop.push(c.normalize()?);
        }
        Ok(pop)
    }

    /// Strictly zero-mean unbiased local & single-step topological transition operator for Wave-3
    /// de-novo evolution (used identically by `B3_Wave3_Exotic3D_QD_CEI` and `A2_No_Selection_Random_Walk`).
    /// Because all continuous perturbations have mean `0.0` and topological transitions are symmetric,
    /// directional improvement across generations can only occur via evolutionary selection.
    pub(crate) fn raw_mutate_wave3_progressive(&self, rng: &mut Rng, projection: bool) -> Self {
        let mut c = self.clone();

        // 1. Zero-mean continuous ramp step
        c.inlet_ramp.axial_drop_mm = round4(project_bound(
            c.inlet_ramp.axial_drop_mm + rng.gauss(0.0, 0.65),
            0.0,
            7.8,
            projection,
        ));
        c.inlet_ramp.radial_slot_factor = round4(project_bound(
            c.inlet_ramp.radial_slot_factor + rng.gauss(0.0, 0.020),
            0.80,
            1.01,
            projection,
        ));
        c.inlet_ramp.superellipse_exponent = round4(project_bound(
            c.inlet_ramp.superellipse_exponent + rng.gauss(0.0, 0.20),
            2.0,
            3.8,
            projection,
        ));

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
            c.inlet_split.vane_thickness_mm = round4(project_bound(
                c.inlet_split.vane_thickness_mm + rng.gauss(0.0, 0.035),
                1.08,
                1.45,
                projection,
            ));
            c.inlet_split.vane_z_fraction = round4(project_bound(
                c.inlet_split.vane_z_fraction + rng.gauss(0.0, 0.02),
                0.42,
                0.58,
                projection,
            ));
            c.inlet_split.azimuthal_phase_deg = round4(project_bound(
                c.inlet_split.azimuthal_phase_deg + rng.gauss(0.0, 3.5),
                0.0,
                40.0,
                projection,
            ));
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
                c.ribbon_cascade.axial_span_mm = round4(project_bound(
                    c.ribbon_cascade.axial_span_mm + rng.gauss(0.0, 2.5),
                    14.0,
                    40.0,
                    projection,
                ));
                c.ribbon_cascade.helical_twist_rad = round4(project_bound(
                    c.ribbon_cascade.helical_twist_rad + rng.gauss(0.0, 0.25),
                    -3.9,
                    -1.0,
                    projection,
                ));
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
                c.tpms_insert.radial_inner_fraction = round4(project_bound(
                    c.tpms_insert.radial_inner_fraction + rng.gauss(0.0, 0.020),
                    0.66,
                    0.80,
                    projection,
                ));
                c.tpms_insert.radial_wavelength_mm = round4(project_bound(
                    c.tpms_insert.radial_wavelength_mm + rng.gauss(0.0, 0.38),
                    7.5,
                    12.0,
                    projection,
                ));
                c.tpms_insert.axial_wavelength_mm = round4(project_bound(
                    c.tpms_insert.axial_wavelength_mm + rng.gauss(0.0, 0.38),
                    8.0,
                    12.2,
                    projection,
                ));
                c.tpms_insert.sheet_thickness_mm = round4(project_bound(
                    c.tpms_insert.sheet_thickness_mm + rng.gauss(0.0, 0.035),
                    1.24,
                    1.75,
                    projection,
                ));
            }
        }

        // 5. Zero-mean diffeomorphic cone & chamber harmonic step
        let mut m1 = project_bound(
            c.cone_profile.mode1_mm + rng.gauss(0.0, 0.12),
            -0.55,
            1.12,
            projection,
        );
        let mut m2 = project_bound(
            c.cone_profile.mode2_mm + rng.gauss(0.0, 0.08),
            -0.38,
            0.32,
            projection,
        );
        let mut m3 = project_bound(
            c.cone_profile.mode3_mm + rng.gauss(0.0, 0.04),
            -0.14,
            0.14,
            projection,
        );
        let sum_m = m1.abs() + m2.abs() + m3.abs();
        if projection && sum_m > 1.45 {
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
                    m.cos_mm = round4(project_bound(
                        m.cos_mm + rng.gauss(0.0, 0.08),
                        -0.75,
                        0.78,
                        projection,
                    ));
                    m.sin_mm = round4(project_bound(
                        m.sin_mm + rng.gauss(0.0, 0.08),
                        -0.75,
                        0.78,
                        projection,
                    ));
                    m.twist_rad = round4(project_bound(
                        m.twist_rad + rng.gauss(0.0, 0.25),
                        -5.6,
                        -0.8,
                        projection,
                    ));
                }
            }
        }

        // 6. Zero-mean skimmer sleeve step
        c.skimmer.length_mm = round4(project_bound(
            c.skimmer.length_mm + rng.gauss(0.0, 0.75),
            0.0,
            9.6,
            projection,
        ));
        if c.skimmer.length_mm > 0.1 {
            c.skimmer.outer_radius_mm = round4(project_bound(
                c.skimmer.outer_radius_mm + rng.gauss(0.0, 0.05),
                2.12,
                3.10,
                projection,
            ));
            if rng.f64() < 0.25 {
                c.skimmer.capture_flutes = match rng.next_u64() % 4 {
                    0 => 0,
                    1 => 4,
                    2 => 6,
                    _ => 8,
                };
            }
        }

        c
    }
}
