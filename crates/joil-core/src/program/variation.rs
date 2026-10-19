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

    /// Sample an independent uniform random 3D candidate across all 11 families (`Comparator B1`).
    pub(crate) fn raw_sample_random_exotic(rng: &mut Rng, projection: bool) -> Self {
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
            c = c.raw_mutate_tpms_grading(rng, projection);
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
        c
    }

    /// Compositional layer-wise crossover followed by mutation.
    pub(crate) fn raw_crossover(&self, other: &Self, rng: &mut Rng, projection: bool) -> Self {
        let child = self.raw_crossover_non_homologous_macro(other, rng, projection);
        child.raw_mutate(rng, projection)
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
    pub(crate) fn raw_sample_stratified_11family(
        rng: &mut Rng,
        projection: bool,
        family_idx: usize,
    ) -> Self {
        let _ = projection;
        let mut c = Self::baseline_ct35();
        match family_idx % 11 {
            0 => c,
            1 => {
                c.inlet_ramp = InletRamp3D {
                    axial_drop_mm: round4(rng.range(2.2, 7.6)),
                    radial_slot_factor: round4(rng.range(0.82, 0.96)),
                    superellipse_exponent: round4(rng.range(2.2, 3.6)),
                };
                c
            }
            2 => {
                c.inlet_split = InletSplit3D {
                    enabled: true,
                    vane_thickness_mm: round4(rng.range(1.10, 1.40)),
                    vane_z_fraction: round4(rng.range(0.43, 0.57)),
                    branches_per_port: 2,
                    azimuthal_phase_deg: round4(rng.range(5.0, 36.0)),
                };
                c
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
                c
            }
            4 => {
                c.chamber_modes = vec![ChamberMode3D {
                    azimuthal_order: 2 + (rng.next_u64() % 3) as u32,
                    cos_mm: round4(rng.range(0.35, 0.78)),
                    sin_mm: round4(rng.range(-0.45, 0.45)),
                    twist_rad: round4(rng.range(-5.2, -1.8)),
                }];
                c
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
                c
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
                c
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
                c
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
                c
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
                c
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
                c
            }
        }
    }
}
