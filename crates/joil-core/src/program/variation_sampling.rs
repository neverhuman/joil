use super::*;

impl DesignProgram {
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
