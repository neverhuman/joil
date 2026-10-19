use super::*;

// ---------------------------------------------------------------- V5-19-03 native static FEA

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FixtureKind {
    Patch,
    RigidBody,
    LameCylinder,
    Cantilever,
    ReactionEnergyBalance,
    MeshConvergence,
}

pub const ALL_FIXTURES: [FixtureKind; 6] = [
    FixtureKind::Patch,
    FixtureKind::RigidBody,
    FixtureKind::LameCylinder,
    FixtureKind::Cantilever,
    FixtureKind::ReactionEnergyBalance,
    FixtureKind::MeshConvergence,
];

impl FixtureKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Patch => "patch",
            Self::RigidBody => "rigid_body",
            Self::LameCylinder => "lame_cylinder",
            Self::Cantilever => "cantilever",
            Self::ReactionEnergyBalance => "reaction_energy_balance",
            Self::MeshConvergence => "mesh_convergence",
        }
    }
}

/// Analytic Lamé thick-cylinder oracle (closed ends): `(sigma_r, sigma_theta, sigma_z, u_r)`.
pub fn lame_oracle(
    a: f64,
    b: f64,
    p_i: f64,
    p_o: f64,
    youngs: f64,
    nu: f64,
    r: f64,
) -> Result<(f64, f64, f64, f64), JudgeError> {
    let valid = a.is_finite()
        && b.is_finite()
        && 0.0 < a
        && a < b
        && (a..=b).contains(&r)
        && [p_i, p_o, youngs, nu].iter().all(|x| x.is_finite())
        && youngs > 0.0
        && (0.0..0.5).contains(&nu);
    if !valid {
        return Err(invalid(
            "Lamé oracle needs 0 < a < r < b, finite loads, E > 0, nu in [0, 0.5)",
        ));
    }
    let c1 = (a * a * p_i - b * b * p_o) / (b * b - a * a);
    let c2 = a * a * b * b * (p_i - p_o) / ((b * b - a * a) * r * r);
    let u = r / youngs * ((1.0 - 2.0 * nu) * c1 + (1.0 + nu) * c2);
    Ok((c1 - c2, c1 + c2, c1, u))
}

/// Euler-Bernoulli tip deflection `P L^3 / (3 E I)` of a rectangular cantilever.
pub fn cantilever_bending_deflection(l: f64, w: f64, h: f64, p: f64, youngs: f64) -> f64 {
    p * l.powi(3) / (3.0 * youngs * w * h.powi(3) / 12.0)
}

/// Timoshenko tip deflection: bending plus shear with `kappa = 5/6`.
pub fn cantilever_timoshenko_deflection(
    l: f64,
    w: f64,
    h: f64,
    p: f64,
    youngs: f64,
    nu: f64,
) -> f64 {
    let g = youngs / (2.0 * (1.0 + nu));
    cantilever_bending_deflection(l, w, h, p, youngs) + p * l / (5.0 / 6.0 * g * w * h)
}

/// Solver outputs for one verification fixture, with the analytic inputs the oracle needs.
#[derive(Clone, Debug, PartialEq)]
pub enum Fixture {
    Patch {
        applied_stress_pa: f64,
        solver_axial_stress_pa: Vec<f64>,
        solver_off_axis_max_pa: f64,
        tolerance_rel: Field<f64>,
    },
    RigidBody {
        youngs_pa: f64,
        imposed_translation_m: f64,
        body_length_m: f64,
        solver_max_stress_pa: f64,
        tolerance_rel: Field<f64>,
    },
    LameCylinder {
        inner_radius_m: f64,
        outer_radius_m: f64,
        internal_pa: f64,
        external_pa: f64,
        youngs_pa: f64,
        poisson: f64,
        solver_hoop_inner_pa: f64,
        solver_radial_displacement_inner_m: f64,
        tolerance_rel: Field<f64>,
    },
    Cantilever {
        length_m: f64,
        width_m: f64,
        height_m: f64,
        tip_load_n: f64,
        youngs_pa: f64,
        poisson: f64,
        solver_tip_deflection_m: f64,
        tolerance_rel: Field<f64>,
    },
    ReactionEnergyBalance {
        applied_force_n: V3,
        solver_reaction_force_n: V3,
        external_work_j: f64,
        strain_energy_j: f64,
        tolerance_rel: Field<f64>,
    },
    MeshConvergence {
        values_coarse_to_fine: Vec<f64>,
        refinement_ratio: f64,
        tolerance_rel: Field<f64>,
    },
}

pub(super) fn tolerance(f: &Field<f64>) -> Result<f64, Verdict> {
    match f.admitted() {
        None => Err(open("tolerance:not_admitted")),
        Some(t) if t.is_finite() && 0.0 < *t && *t <= 0.5 => Ok(*t),
        Some(_) => Err(open("tolerance:outside_(0,0.5]")),
    }
}

impl Fixture {
    pub fn kind(&self) -> FixtureKind {
        match self {
            Self::Patch { .. } => FixtureKind::Patch,
            Self::RigidBody { .. } => FixtureKind::RigidBody,
            Self::LameCylinder { .. } => FixtureKind::LameCylinder,
            Self::Cantilever { .. } => FixtureKind::Cantilever,
            Self::ReactionEnergyBalance { .. } => FixtureKind::ReactionEnergyBalance,
            Self::MeshConvergence { .. } => FixtureKind::MeshConvergence,
        }
    }

    /// Compare solver output with the analytic oracle. Non-finite output or unusable oracle
    /// inputs are `Unresolved`; a finite mismatch is `Fail`.
    pub fn judge(&self) -> Verdict {
        let k = self.kind().name();
        let tol_field = match self {
            Self::Patch { tolerance_rel, .. }
            | Self::RigidBody { tolerance_rel, .. }
            | Self::LameCylinder { tolerance_rel, .. }
            | Self::Cantilever { tolerance_rel, .. }
            | Self::ReactionEnergyBalance { tolerance_rel, .. }
            | Self::MeshConvergence { tolerance_rel, .. } => tolerance_rel,
        };
        let tol = match tolerance(tol_field) {
            Ok(t) => t,
            Err(v) => return prefixed(&format!("fixture:{k}"), &v),
        };
        let bad_input = || open(format!("fixture:{k}:non_finite_or_degenerate_input"));
        let mismatch = |what: &str| fail(format!("fixture:{k}:{what}_outside_tolerance"));
        let close = |solver: f64, oracle: f64| (solver - oracle).abs() <= tol * oracle.abs();
        let usable = |xs: &[f64]| xs.iter().all(|x| x.is_finite());
        match self {
            Self::Patch {
                applied_stress_pa,
                solver_axial_stress_pa,
                solver_off_axis_max_pa,
                ..
            } => {
                if solver_axial_stress_pa.is_empty()
                    || !usable(solver_axial_stress_pa)
                    || !usable(&[*applied_stress_pa, *solver_off_axis_max_pa])
                    || *applied_stress_pa == 0.0
                {
                    return bad_input();
                }
                let axial_ok = solver_axial_stress_pa
                    .iter()
                    .all(|s| close(*s, *applied_stress_pa));
                if axial_ok && solver_off_axis_max_pa.abs() <= tol * applied_stress_pa.abs() {
                    Verdict::Pass
                } else {
                    mismatch("uniform_stress_state")
                }
            }
            Self::RigidBody {
                youngs_pa,
                imposed_translation_m,
                body_length_m,
                solver_max_stress_pa,
                ..
            } => {
                let scale = youngs_pa * imposed_translation_m.abs() / body_length_m;
                if !usable(&[
                    *youngs_pa,
                    *imposed_translation_m,
                    *body_length_m,
                    *solver_max_stress_pa,
                ]) || !scale.is_finite()
                    || scale <= 0.0
                {
                    return bad_input();
                }
                if solver_max_stress_pa.abs() <= tol * scale {
                    Verdict::Pass
                } else {
                    mismatch("rigid_body_stress")
                }
            }
            Self::LameCylinder {
                inner_radius_m,
                outer_radius_m,
                internal_pa,
                external_pa,
                youngs_pa,
                poisson,
                solver_hoop_inner_pa,
                solver_radial_displacement_inner_m,
                ..
            } => {
                let Ok((_, hoop, _, u)) = lame_oracle(
                    *inner_radius_m,
                    *outer_radius_m,
                    *internal_pa,
                    *external_pa,
                    *youngs_pa,
                    *poisson,
                    *inner_radius_m,
                ) else {
                    return bad_input();
                };
                if !usable(&[*solver_hoop_inner_pa, *solver_radial_displacement_inner_m])
                    || hoop == 0.0
                    || u == 0.0
                {
                    return bad_input();
                }
                Verdict::combine([
                    if close(*solver_hoop_inner_pa, hoop) {
                        Verdict::Pass
                    } else {
                        mismatch("hoop_stress")
                    },
                    if close(*solver_radial_displacement_inner_m, u) {
                        Verdict::Pass
                    } else {
                        mismatch("radial_displacement")
                    },
                ])
            }
            Self::Cantilever {
                length_m,
                width_m,
                height_m,
                tip_load_n,
                youngs_pa,
                poisson,
                solver_tip_deflection_m,
                ..
            } => {
                let oracle = cantilever_timoshenko_deflection(
                    *length_m,
                    *width_m,
                    *height_m,
                    *tip_load_n,
                    *youngs_pa,
                    *poisson,
                );
                if !usable(&[*length_m, *width_m, *height_m, *solver_tip_deflection_m])
                    || !oracle.is_finite()
                    || oracle == 0.0
                    || *length_m <= 0.0
                    || *height_m <= 0.0
                {
                    return bad_input();
                }
                if length_m / height_m < 8.0 {
                    return open("fixture:cantilever:not_slender_beam_theory_invalid");
                }
                if close(*solver_tip_deflection_m, oracle) {
                    Verdict::Pass
                } else {
                    mismatch("tip_deflection")
                }
            }
            Self::ReactionEnergyBalance {
                applied_force_n,
                solver_reaction_force_n,
                external_work_j,
                strain_energy_j,
                ..
            } => {
                if !finite3(applied_force_n)
                    || !finite3(solver_reaction_force_n)
                    || !usable(&[*external_work_j, *strain_energy_j])
                    || norm(*applied_force_n) == 0.0
                    || *external_work_j <= 0.0
                {
                    return bad_input();
                }
                Verdict::combine([
                    if norm(add(*applied_force_n, *solver_reaction_force_n))
                        <= tol * norm(*applied_force_n)
                    {
                        Verdict::Pass
                    } else {
                        mismatch("reaction_balance")
                    },
                    if (external_work_j - strain_energy_j).abs() <= tol * external_work_j {
                        Verdict::Pass
                    } else {
                        mismatch("energy_balance")
                    },
                ])
            }
            Self::MeshConvergence {
                values_coarse_to_fine: v,
                refinement_ratio,
                ..
            } => {
                if v.len() < 3 {
                    return open("fixture:mesh_convergence:fewer_than_three_grids");
                }
                if !usable(v) || !refinement_ratio.is_finite() || *refinement_ratio <= 1.0 {
                    return bad_input();
                }
                let d: Vec<f64> = v.windows(2).map(|w| w[1] - w[0]).collect();
                let monotone = d.iter().all(|x| *x > 0.0) || d.iter().all(|x| *x < 0.0);
                let shrinking = d.windows(2).all(|w| w[1].abs() < w[0].abs());
                if !monotone || !shrinking {
                    return open("fixture:mesh_convergence:not_monotone_and_shrinking");
                }
                let fine = v[v.len() - 1];
                if fine == 0.0 {
                    return bad_input();
                }
                if d[d.len() - 1].abs() <= tol * fine.abs() {
                    Verdict::Pass
                } else {
                    mismatch("finest_grid_change")
                }
            }
        }
    }
}
