use super::*;

impl IrCompiledMesh {
    /// Evaluate local inner fluid wall radius `R(theta, z_m)` in SI metres from `DesignIrV1`.
    pub fn inner_radius_m(ir: &DesignIrV1, theta: f64, z_m: f64) -> f64 {
        let r1_m = 0.035_f64;
        let r2_m = 0.0175_f64;
        let r3_m = 0.00875_f64;
        let l1_m = 0.070_f64;
        let lc_m = ((0.070_f64 - 0.035_f64) / 2.0) / 10.0_f64.to_radians().tan();
        let l2_m = 0.770_f64;
        let zc_m = l1_m + lc_m;
        let zt_m = zc_m + l2_m;

        let mut r_wall = if z_m <= l1_m {
            let mut delta = 0.0;
            if (0.020..=0.065).contains(&z_m) {
                let s = (z_m - 0.020) / 0.045;
                let env = (PI * s).sin().powi(2);
                for node in &ir.nodes {
                    if let IrNode::ChamberHarmonicMode {
                        azimuthal_order,
                        cos_m,
                        sin_m,
                        twist_rad,
                        ..
                    } = node
                    {
                        let phase = (*azimuthal_order as f64) * theta + twist_rad * s;
                        delta += env * (cos_m * phase.cos() + sin_m * phase.sin());
                    }
                }
            }
            r1_m + delta
        } else if z_m <= zc_m {
            let r_lin = r1_m + (r2_m - r1_m) * ((z_m - l1_m) / lc_m);
            let mut delta = 0.0;
            if (0.072..=0.166).contains(&z_m) {
                let s = (z_m - 0.072) / (0.166 - 0.072);
                let env = (PI * s).sin().powi(2);
                for node in &ir.nodes {
                    if let IrNode::ConeSplineProfile {
                        mode1_m,
                        mode2_m,
                        mode3_m,
                        ..
                    } = node
                    {
                        delta += env
                            * (mode1_m * (PI * s).sin()
                                + mode2_m * (2.0 * PI * s).sin()
                                + mode3_m * (3.0 * PI * s).sin());
                    }
                }
            }
            r_lin + delta
        } else if z_m <= zt_m {
            r2_m + (r3_m - r2_m) * ((z_m - zc_m) / l2_m)
        } else {
            r3_m
        };

        // HelicalRibbonCascade modulation
        for node in &ir.nodes {
            if let IrNode::HelicalRibbonCascade {
                vane_count,
                axial_start_m,
                axial_end_m,
                helical_twist_rad,
                radial_chord_m,
                ..
            } = node
            {
                if (*axial_start_m..=*axial_end_m).contains(&z_m)
                    && *axial_end_m > *axial_start_m + 1e-3
                {
                    let s = (z_m - axial_start_m) / (axial_end_m - axial_start_m);
                    let env = (PI * s).sin().powi(2);
                    let m = *vane_count as f64;
                    let phase = m * theta + helical_twist_rad * s;
                    let vane_profile = phase.cos().max(0.0).powi(4);
                    r_wall -= env * (0.35 * radial_chord_m) * vane_profile;
                }
            }
        }

        // TpmsPeripheralShell modulation
        for node in &ir.nodes {
            if let IrNode::TpmsPeripheralShell {
                basis,
                axial_start_m,
                axial_end_m,
                axial_wavelength_m,
                sheet_thickness_m,
                ..
            } = node
            {
                if (*axial_start_m..=*axial_end_m).contains(&z_m)
                    && *axial_end_m > *axial_start_m + 1e-3
                {
                    let s = (z_m - axial_start_m) / (axial_end_m - axial_start_m);
                    let env = (PI * s).sin().powi(2);
                    let kz = 2.0 * PI * (z_m - axial_start_m) / axial_wavelength_m;
                    let kt = 4.0 * theta;
                    let wave = match basis {
                        TpmsBasisIr::None | TpmsBasisIr::UnsupportedCustom => 0.0,
                        TpmsBasisIr::Gyroid | TpmsBasisIr::CylindricalGyroid => {
                            0.55 * (kt.sin() * kz.cos() + (kt + kz).sin() * 0.5)
                        }
                        TpmsBasisIr::SchwarzP => 0.50 * (kt.cos() + kz.cos()) * 0.5,
                        TpmsBasisIr::Diamond => 0.50 * (kt.sin() * kz.sin()),
                    };
                    r_wall += env * (0.45 * sheet_thickness_m) * wave;
                }
            }
        }

        r_wall
    }
}
