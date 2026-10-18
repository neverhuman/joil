//! V5-19: candidate-specific structural loads, static FEA qualification, buckling and reserve.
//!
//! Nothing here computes a structural answer from a closed-form load. The analytic Lamé and
//! cantilever expressions are *oracles* a native solver has to reproduce before it may be used;
//! a run that rests on a fixed-load formula is refused. Every threshold, tolerance, budget and
//! required factor is a status-bearing [`Field`]; missing ones make the judgement `Unresolved`.
//! The draft 2.5 MPa vessel overpressure is `Proposed` and never decides anything.
//! Nothing in this module is a physical qualification.
use crate::material::{ClaimKind, MaterialPassport};
use crate::{canonical_hash, invalid, positive, Field, JudgeError, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

type V3 = [f64; 3];

/// Legacy draft external differential (joil-core `DESIGN_VESSEL_DELTA_P_PA`): provisional.
pub const DRAFT_EXTERNAL_DELTA_P_PA: f64 = 2.5e6;
/// CT-35 reference pressure that earlier screens copied onto every candidate.
pub const COPIED_CT35_PRESSURE_PA: f64 = 29468.22;

fn norm(v: V3) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn finite3(v: &V3) -> bool {
    v.iter().all(|x| x.is_finite())
}
fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
fn cross(r: V3, f: V3) -> V3 {
    std::array::from_fn(|i| r[(i + 1) % 3] * f[(i + 2) % 3] - r[(i + 2) % 3] * f[(i + 1) % 3])
}
fn prefixed(prefix: &str, v: &Verdict) -> Verdict {
    let tag = |r: &Vec<String>| r.iter().map(|s| format!("{prefix}:{s}")).collect();
    match v {
        Verdict::Pass => Verdict::Pass,
        Verdict::Fail(r) => Verdict::Fail(tag(r)),
        Verdict::Unresolved(r) => Verdict::Unresolved(tag(r)),
    }
}
fn open(reason: impl Into<String>) -> Verdict {
    Verdict::Unresolved(vec![reason.into()])
}
fn fail(reason: impl Into<String>) -> Verdict {
    Verdict::Fail(vec![reason.into()])
}

// ---------------------------------------------------------------- V5-19-01 load families

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadFamily {
    OperatingFlow,
    ExternalDifferentialUpset,
    SupportPreloadContact,
    Handling,
    Thermal,
}

pub const ALL_FAMILIES: [LoadFamily; 5] = [
    LoadFamily::OperatingFlow,
    LoadFamily::ExternalDifferentialUpset,
    LoadFamily::SupportPreloadContact,
    LoadFamily::Handling,
    LoadFamily::Thermal,
];

impl LoadFamily {
    pub fn name(self) -> &'static str {
        match self {
            Self::OperatingFlow => "operating_flow",
            Self::ExternalDifferentialUpset => "external_differential_upset",
            Self::SupportPreloadContact => "support_preload_contact",
            Self::Handling => "handling",
            Self::Thermal => "thermal",
        }
    }
}

/// One family's external demand. `reference` is the demand at multiplier 1 and belongs to the
/// installation, not to a design; `required_factor` is what the allowable multiplier must beat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyDemand {
    pub unit: String,
    pub description: String,
    pub reference: Field<f64>,
    pub required_factor: Field<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadFamilySet {
    pub schema: String,
    pub set_id: String,
    pub synthetic: bool,
    pub families: BTreeMap<LoadFamily, FamilyDemand>,
}

/// A load set whose identity hash is what every design and reserve must cite.
#[derive(Clone, Debug, PartialEq)]
pub struct FrozenLoads {
    pub sha256: Sha256Hex,
    pub set: LoadFamilySet,
}

impl LoadFamilySet {
    pub fn from_json(bytes: &[u8]) -> Result<Self, JudgeError> {
        let set: Self = serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
        set.check()?;
        Ok(set)
    }

    pub fn check(&self) -> Result<(), JudgeError> {
        if self.schema != "joil.hydrocyclone.load-family-set.v1" || self.set_id.trim().is_empty() {
            return Err(invalid("unknown load-family schema or empty id"));
        }
        for family in ALL_FAMILIES {
            let Some(d) = self.families.get(&family) else {
                return Err(invalid(format!(
                    "family {} missing; use explicit unresolved fields",
                    family.name()
                )));
            };
            if d.unit.trim().is_empty() {
                return Err(invalid(format!("family {} needs a unit", family.name())));
            }
            for f in [&d.reference, &d.required_factor] {
                if let Field::Admitted { value, .. } | Field::Proposed { value, .. } = f {
                    positive(family.name(), *value)?;
                }
            }
        }
        Ok(())
    }

    pub fn freeze(&self) -> Result<FrozenLoads, JudgeError> {
        self.check()?;
        Ok(FrozenLoads {
            sha256: canonical_hash(self)?,
            set: self.clone(),
        })
    }

    /// Unresolved while any reference or required factor is not admitted. Never Fail.
    pub fn verdict(&self) -> Verdict {
        let mut blockers = Vec::new();
        for (family, d) in &self.families {
            for (what, f) in [
                ("reference", &d.reference),
                ("required_factor", &d.required_factor),
            ] {
                if f.admitted().is_none() {
                    blockers.push(format!("load_family:{}:{what}:not_admitted", family.name()));
                }
            }
        }
        if blockers.is_empty() {
            Verdict::Pass
        } else {
            Verdict::Unresolved(blockers)
        }
    }
}

impl FrozenLoads {
    /// External demand of `family` at `multiplier`: identical for every design.
    pub fn demand_at(&self, family: LoadFamily, multiplier: f64) -> Result<f64, JudgeError> {
        positive("load multiplier", multiplier)?;
        let reference = self
            .set
            .families
            .get(&family)
            .and_then(|d| d.reference.admitted())
            .ok_or_else(|| invalid(format!("{} reference not admitted", family.name())))?;
        Ok(reference * multiplier)
    }
}

/// The reference loads a design says it was solved under.
#[derive(Clone, Debug, PartialEq)]
pub struct DesignLoadClaim {
    pub design_sha256: Sha256Hex,
    pub frozen_loads_sha256: Sha256Hex,
    pub reference_loads: BTreeMap<LoadFamily, f64>,
}

/// A design must cite the frozen set and use its exact reference values; a weaker design cannot
/// pick an easier reference load.
pub fn check_design_loads(frozen: &FrozenLoads, claim: &DesignLoadClaim) -> Verdict {
    let mut parts = vec![frozen.set.verdict()];
    if claim.frozen_loads_sha256 != frozen.sha256 {
        parts.push(fail("design_cites_a_different_load_set"));
    }
    for family in ALL_FAMILIES {
        let frozen_ref = frozen
            .set
            .families
            .get(&family)
            .and_then(|d| d.reference.admitted());
        match (claim.reference_loads.get(&family), frozen_ref) {
            (None, _) => parts.push(fail(format!("reference_load_missing:{}", family.name()))),
            (Some(v), _) if !v.is_finite() => {
                parts.push(fail(format!("reference_load_not_finite:{}", family.name())))
            }
            (Some(v), Some(r)) if v != r => parts.push(fail(format!(
                "reference_load_differs_from_frozen:{}",
                family.name()
            ))),
            _ => {}
        }
    }
    Verdict::combine(parts)
}

// ---------------------------------------------------------------- V5-19-02 traction mapping

/// A traction facet. Force on the solid is `(-pressure * normal + shear) * area`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Facet {
    pub patch: String,
    pub position_m: V3,
    pub normal: V3,
    pub area_m2: f64,
    pub pressure_pa: f64,
    pub shear_pa: V3,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reaction {
    pub position_m: V3,
    pub force_n: V3,
}

/// Preallocated mapping budget: errors are `absolute + relative * |source|`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingBudget {
    pub relative: f64,
    pub force_abs_n: f64,
    pub moment_abs_n_m: f64,
    pub area_relative: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TractionTransfer {
    pub design_sha256: Sha256Hex,
    /// Subject the accepted CFD traction field belongs to.
    pub source_design_sha256: Sha256Hex,
    pub interpolation_id: String,
    pub interpolation_sha256: Sha256Hex,
    pub solid_mesh_sha256: Sha256Hex,
    pub origin_m: V3,
    pub required_patches: Vec<String>,
    pub source: Vec<Facet>,
    pub mapped: Vec<Facet>,
    pub reactions: Vec<Reaction>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Sums {
    force: V3,
    shear_force: V3,
    moment: V3,
    vector_area: V3,
    area: f64,
}

fn integrate<'a>(facets: impl Iterator<Item = &'a Facet>, origin: V3) -> Sums {
    let mut s = Sums::default();
    for f in facets {
        let shear: V3 = std::array::from_fn(|i| f.shear_pa[i] * f.area_m2);
        let force: V3 =
            std::array::from_fn(|i| -f.pressure_pa * f.normal[i] * f.area_m2 + shear[i]);
        s.force = add(s.force, force);
        s.shear_force = add(s.shear_force, shear);
        s.moment = add(s.moment, cross(sub(f.position_m, origin), force));
        s.vector_area = add(
            s.vector_area,
            std::array::from_fn(|i| f.normal[i] * f.area_m2),
        );
        s.area += f.area_m2;
    }
    s
}

fn check_facets(name: &str, facets: &[Facet]) -> Result<(), JudgeError> {
    if facets.is_empty() {
        return Err(invalid(format!("{name} has no facets")));
    }
    for f in facets {
        if f.patch.trim().is_empty()
            || !f.area_m2.is_finite()
            || f.area_m2 <= 0.0
            || !f.pressure_pa.is_finite()
            || !finite3(&f.position_m)
            || !finite3(&f.normal)
            || !finite3(&f.shear_pa)
            || (norm(f.normal) - 1.0).abs() > 1e-9
        {
            return Err(invalid(format!(
                "{name}: finite SI facets with positive area, unit normals and a patch id required"
            )));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatchError {
    pub force_residual_n: f64,
    pub shear_residual_n: f64,
    pub moment_residual_n_m: f64,
    pub area_relative_error: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransferReport {
    pub design_sha256: Sha256Hex,
    pub interpolation_id: String,
    pub interpolation_sha256: Sha256Hex,
    pub solid_mesh_sha256: Sha256Hex,
    pub source_force_n: V3,
    pub mapped_force_n: V3,
    pub source_moment_n_m: V3,
    pub mapped_moment_n_m: V3,
    pub reaction_force_n: V3,
    pub reaction_moment_n_m: V3,
    pub patch_errors: BTreeMap<String, PatchError>,
    pub verdict: Verdict,
}

fn group_by_patch(facets: &[Facet]) -> BTreeMap<&str, Vec<&Facet>> {
    let mut m: BTreeMap<&str, Vec<&Facet>> = BTreeMap::new();
    for f in facets {
        m.entry(f.patch.as_str()).or_default().push(f);
    }
    m
}

/// Judge a CFD-to-solid traction transfer: patches, shear, normals, conservation and reactions.
pub fn judge_transfer(
    t: &TractionTransfer,
    budget: &Field<MappingBudget>,
    structural_mesh: &Sha256Hex,
) -> Result<TransferReport, JudgeError> {
    check_facets("source", &t.source)?;
    check_facets("mapped", &t.mapped)?;
    if !finite3(&t.origin_m)
        || t.interpolation_id.trim().is_empty()
        || t.required_patches.is_empty()
        || t.reactions
            .iter()
            .any(|r| !finite3(&r.position_m) || !finite3(&r.force_n))
    {
        return Err(invalid(
            "transfer needs finite origin/reactions, an interpolation id and required patches",
        ));
    }
    let mut parts = Vec::new();
    let b = match budget {
        Field::Admitted { value, .. } => {
            let ok = [value.force_abs_n, value.moment_abs_n_m]
                .iter()
                .all(|x| x.is_finite() && *x >= 0.0)
                && [value.relative, value.area_relative]
                    .iter()
                    .all(|x| x.is_finite() && (0.0..1.0).contains(x));
            if !ok {
                return Err(invalid(
                    "mapping budget needs finite nonnegative terms, ratios in [0,1)",
                ));
            }
            Some(value)
        }
        _ => {
            parts.push(open("mapping_budget:not_admitted"));
            None
        }
    };
    if t.source_design_sha256 != t.design_sha256 {
        parts.push(fail("source_field_belongs_to_another_design"));
    }
    if &t.solid_mesh_sha256 != structural_mesh {
        parts.push(fail("solid_mesh_is_not_the_structural_mesh"));
    }
    let (src, map) = (group_by_patch(&t.source), group_by_patch(&t.mapped));
    let within = |err: f64, abs: f64, rel: f64, reference: f64| err <= abs + rel * reference;

    let mut patch_errors = BTreeMap::new();
    for patch in t.required_patches.iter().collect::<BTreeSet<_>>() {
        let (Some(s), Some(m)) = (src.get(patch.as_str()), map.get(patch.as_str())) else {
            parts.push(fail(format!("patch_missing:{patch}")));
            continue;
        };
        let (s, m) = (
            integrate(s.iter().copied(), t.origin_m),
            integrate(m.iter().copied(), t.origin_m),
        );
        let err = PatchError {
            force_residual_n: norm(sub(m.force, s.force)),
            shear_residual_n: norm(sub(m.shear_force, s.shear_force)),
            moment_residual_n_m: norm(sub(m.moment, s.moment)),
            area_relative_error: (m.area - s.area).abs() / s.area,
        };
        if let Some(b) = b {
            if !within(
                err.force_residual_n,
                b.force_abs_n,
                b.relative,
                norm(s.force),
            ) {
                parts.push(fail(format!("patch_force_not_conserved:{patch}")));
            }
            if !within(
                err.shear_residual_n,
                b.force_abs_n,
                b.relative,
                norm(s.shear_force),
            ) {
                parts.push(fail(format!("patch_shear_not_conserved:{patch}")));
            }
            if !within(
                err.moment_residual_n_m,
                b.moment_abs_n_m,
                b.relative,
                norm(s.moment),
            ) {
                parts.push(fail(format!("patch_moment_not_conserved:{patch}")));
            }
            if err.area_relative_error > b.area_relative
                || norm(sub(m.vector_area, s.vector_area)) > b.area_relative * s.area
            {
                parts.push(fail(format!("patch_area_or_normals_not_preserved:{patch}")));
            }
        }
        patch_errors.insert(patch.clone(), err);
    }

    let (s, m) = (
        integrate(t.source.iter(), t.origin_m),
        integrate(t.mapped.iter(), t.origin_m),
    );
    if norm(s.shear_force) == 0.0 {
        parts.push(fail("shear_absent_in_source_field"));
    } else if norm(m.shear_force) == 0.0 {
        parts.push(fail("shear_missing_after_transfer"));
    }
    if t.mapped
        .iter()
        .all(|f| (f.pressure_pa - COPIED_CT35_PRESSURE_PA).abs() <= 0.005)
    {
        parts.push(fail("copied_ct35_reference_pressure"));
    }
    let (mut rf, mut rm) = ([0.0; 3], [0.0; 3]);
    for r in &t.reactions {
        rf = add(rf, r.force_n);
        rm = add(rm, cross(sub(r.position_m, t.origin_m), r.force_n));
    }
    if t.reactions.is_empty() {
        parts.push(open("reactions:not_supplied"));
    } else if let Some(b) = b {
        if !within(
            norm(add(m.force, rf)),
            b.force_abs_n,
            b.relative,
            norm(m.force),
        ) || !within(
            norm(add(m.moment, rm)),
            b.moment_abs_n_m,
            b.relative,
            norm(m.moment),
        ) {
            parts.push(fail("reactions_do_not_balance_mapped_load"));
        }
    }
    if let Some(b) = b {
        if !within(
            norm(sub(m.force, s.force)),
            b.force_abs_n,
            b.relative,
            norm(s.force),
        ) || !within(
            norm(sub(m.moment, s.moment)),
            b.moment_abs_n_m,
            b.relative,
            norm(s.moment),
        ) {
            parts.push(fail("total_force_or_moment_not_conserved"));
        }
    }
    Ok(TransferReport {
        design_sha256: t.design_sha256.clone(),
        interpolation_id: t.interpolation_id.clone(),
        interpolation_sha256: t.interpolation_sha256.clone(),
        solid_mesh_sha256: t.solid_mesh_sha256.clone(),
        source_force_n: s.force,
        mapped_force_n: m.force,
        source_moment_n_m: s.moment,
        mapped_moment_n_m: m.moment,
        reaction_force_n: rf,
        reaction_moment_n_m: rm,
        patch_errors,
        verdict: Verdict::combine(parts),
    })
}

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

fn tolerance(f: &Field<f64>) -> Result<f64, Verdict> {
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

/// Isolated process adapter contract for a pinned mature solver (for example CalculiX).
/// This crate builds the command; it does not spawn it, and no solver is installed here.
#[derive(Clone, Debug, PartialEq)]
pub struct ProcessSpec {
    pub solver: String,
    pub version: String,
    pub binary_sha256: Field<Sha256Hex>,
    pub program: String,
    pub args: Vec<String>,
    pub work_dir: String,
    pub max_threads: u32,
    pub wall_limit_s: u32,
    pub max_rss_mib: u32,
}

impl ProcessSpec {
    pub fn judge(&self) -> Verdict {
        let mut parts = Vec::new();
        if self.solver.trim().is_empty() || self.version.trim().is_empty() {
            parts.push(open("solver_name_or_version:empty"));
        }
        if self.binary_sha256.admitted().is_none() {
            parts.push(open("solver_binary:not_admitted"));
        }
        if !self.program.starts_with('/') {
            parts.push(fail("process_not_isolated:program_not_absolute_path"));
        }
        if !self.work_dir.starts_with('/') || self.work_dir.starts_with("/tmp") {
            parts.push(fail(
                "process_not_isolated:work_dir_not_private_absolute_path",
            ));
        }
        if self.max_threads == 0 || self.wall_limit_s == 0 || self.max_rss_mib == 0 {
            parts.push(fail("process_not_isolated:resource_limits_missing"));
        }
        Verdict::combine(parts)
    }

    /// Cleared environment, one numerical thread count, private working directory.
    pub fn command(&self) -> std::process::Command {
        let mut c = std::process::Command::new(&self.program);
        c.args(&self.args)
            .env_clear()
            .env("OMP_NUM_THREADS", self.max_threads.to_string())
            .env("CCX_NPROC_EQUATION_SOLVER", self.max_threads.to_string())
            .current_dir(&self.work_dir);
        c
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SolverQualificationReport {
    pub process: Verdict,
    pub fixtures: BTreeMap<FixtureKind, Verdict>,
    pub verdict: Verdict,
}

/// A solver is qualified only when it is pinned/isolated and reproduces every oracle fixture.
pub fn qualify_solver(process: &ProcessSpec, fixtures: &[Fixture]) -> SolverQualificationReport {
    let process_verdict = process.judge();
    let mut by_kind = BTreeMap::new();
    for kind in ALL_FIXTURES {
        let results: Vec<Verdict> = fixtures
            .iter()
            .filter(|f| f.kind() == kind)
            .map(Fixture::judge)
            .collect();
        by_kind.insert(
            kind,
            if results.is_empty() {
                open(format!("fixture:{}:not_run", kind.name()))
            } else {
                Verdict::combine(results)
            },
        );
    }
    let verdict = Verdict::combine(
        by_kind
            .values()
            .cloned()
            .chain([prefixed("process", &process_verdict)]),
    );
    SolverQualificationReport {
        process: process_verdict,
        fixtures: by_kind,
        verdict,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuralBasis {
    NativeSolid,
    AnalyticFixedLoad,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObservableKind {
    /// Stress averaged over a declared patch of finite area.
    PatchAveragedStress {
        patch_area_m2: f64,
    },
    Displacement,
    ReactionForce,
    /// Singular-point peak: not a finite predeclared observable.
    NodalPeakStress,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Observable {
    pub name: String,
    pub kind: ObservableKind,
    pub unit: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StaticFeaRun {
    pub basis: StructuralBasis,
    pub design_sha256: Sha256Hex,
    pub mesh_sha256: Sha256Hex,
    pub frozen_loads_sha256: Sha256Hex,
    pub material_passport_id: String,
    pub support_evidence: Field<Sha256Hex>,
    pub observables: Vec<Observable>,
}

/// Static-FEA qualification of one candidate run. Requires a native solid solve on the mapped
/// mesh, a qualified solver, admitted material/process and support evidence, and finite
/// predeclared observables.
pub fn qualify_static_run(
    run: &StaticFeaRun,
    solver: &SolverQualificationReport,
    transfer: &TransferReport,
    frozen: &FrozenLoads,
    passport: &MaterialPassport,
) -> Verdict {
    let mut parts = Vec::new();
    if run.basis == StructuralBasis::AnalyticFixedLoad {
        parts.push(fail(
            "analytic_fixed_load_is_an_oracle_not_a_structural_qualification",
        ));
    }
    parts.push(prefixed("solver", &solver.verdict));
    parts.push(prefixed("load_transfer", &transfer.verdict));
    parts.push(prefixed("loads", &frozen.set.verdict()));
    parts.push(prefixed(
        "material",
        &passport.qualify(ClaimKind::StaticStrength),
    ));
    if run.mesh_sha256 != transfer.solid_mesh_sha256 {
        parts.push(fail("run_mesh_is_not_the_mapped_mesh"));
    }
    if run.design_sha256 != transfer.design_sha256 {
        parts.push(fail("run_design_is_not_the_mapped_design"));
    }
    if run.frozen_loads_sha256 != frozen.sha256 {
        parts.push(fail("run_cites_a_different_load_set"));
    }
    if run.material_passport_id != passport.passport_id {
        parts.push(fail("run_cites_a_different_material_passport"));
    }
    if run.support_evidence.admitted().is_none() {
        parts.push(open("support_evidence:not_admitted"));
    }
    if run.observables.is_empty() {
        parts.push(open("observables:none_declared"));
    }
    for o in &run.observables {
        if !o.value.is_finite() || o.unit.trim().is_empty() {
            parts.push(open(format!("observable:{}:not_finite_or_no_unit", o.name)));
        }
        match o.kind {
            ObservableKind::NodalPeakStress => {
                parts.push(fail(format!("singular_point_stress_observable:{}", o.name)))
            }
            ObservableKind::PatchAveragedStress { patch_area_m2 }
                if !(patch_area_m2.is_finite() && patch_area_m2 > 0.0) =>
            {
                parts.push(fail(format!(
                    "patch_average_without_finite_area:{}",
                    o.name
                )))
            }
            _ => {}
        }
    }
    Verdict::combine(parts)
}

// ---------------------------------------------------------------- V5-19-04 buckling

#[derive(Clone, Debug, PartialEq)]
pub struct EigenSolve {
    pub mesh_sha256: Sha256Hex,
    pub element_count: u64,
    /// Load multiplier of the frozen external demand at first eigenmode.
    pub multiplier: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollapseSolve {
    pub imperfection_label: String,
    pub imperfection_amplitude_m: f64,
    pub imperfection_shape_sha256: Sha256Hex,
    pub contact_declared: bool,
    pub mesh_sha256: Sha256Hex,
    pub converged: bool,
    /// Largest load multiplier at which the nonlinear solve was stable.
    pub limit_multiplier: f64,
    pub solve_sha256: Sha256Hex,
}

/// Plain thin circular cylinder under external pressure (no stiffeners, no corrugation).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CylinderReference {
    pub youngs_pa: f64,
    pub poisson: f64,
    pub wall_m: f64,
    pub radius_m: f64,
    pub length_m: f64,
}

impl CylinderReference {
    /// Classical elastic collapse of a *long* tube, `E/(4(1-nu^2)) (t/R)^3`, only inside its
    /// envelope (`t/R <= 0.1`, `L >= 1.11 D sqrt(D/t)`); otherwise `None`.
    pub fn long_tube_collapse_pa(&self) -> Option<f64> {
        let d = 2.0 * self.radius_m;
        let ok = [self.youngs_pa, self.wall_m, self.radius_m, self.length_m]
            .iter()
            .all(|x| x.is_finite() && *x > 0.0)
            && (0.0..0.5).contains(&self.poisson)
            && self.wall_m / self.radius_m <= 0.1
            && self.length_m >= 1.11 * d * (d / self.wall_m).sqrt();
        ok.then(|| {
            self.youngs_pa / (4.0 * (1.0 - self.poisson.powi(2)))
                * (self.wall_m / self.radius_m).powi(3)
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BucklingInput {
    pub finalist: bool,
    pub contact_applicable: bool,
    /// External-pressure reference demand of the frozen set, in Pa.
    pub demand_reference_pa: f64,
    pub eigen: Vec<EigenSolve>,
    pub collapse: Vec<CollapseSolve>,
    pub reference: Option<CylinderReference>,
    /// Allowed relative spread of the eigen multiplier across meshes.
    pub mesh_spread_tolerance: Field<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BucklingReport {
    pub governing_multiplier: Option<f64>,
    pub method: &'static str,
    pub validity: Vec<String>,
    /// Governing multiplier over the classical long-tube multiplier, when that applies.
    pub reference_ratio: Option<f64>,
    pub verdict: Verdict,
}

pub fn judge_buckling(input: &BucklingInput) -> Result<BucklingReport, JudgeError> {
    positive("demand reference pressure", input.demand_reference_pa)?;
    let mut parts = Vec::new();
    let mut validity = Vec::new();
    if input
        .eigen
        .iter()
        .any(|e| !e.multiplier.is_finite() || e.multiplier <= 0.0)
        || input
            .collapse
            .iter()
            .any(|c| c.converged && !(c.limit_multiplier.is_finite() && c.limit_multiplier > 0.0))
    {
        return Err(invalid("buckling multipliers must be finite and positive"));
    }
    if input.eigen.is_empty() {
        parts.push(open("eigenbuckling:not_run"));
    } else {
        let meshes: BTreeSet<_> = input.eigen.iter().map(|e| &e.mesh_sha256).collect();
        let (lo, hi) = input.eigen.iter().fold((f64::MAX, f64::MIN), |(l, h), e| {
            (l.min(e.multiplier), h.max(e.multiplier))
        });
        match tolerance(&input.mesh_spread_tolerance) {
            Err(v) => parts.push(prefixed("eigen_mesh_sensitivity", &v)),
            Ok(_) if meshes.len() < 3 => {
                parts.push(open("eigen_mesh_sensitivity:fewer_than_three_meshes"))
            }
            Ok(t) if (hi - lo) / lo > t => {
                parts.push(open("eigen_mesh_sensitivity:spread_exceeds_budget"))
            }
            Ok(_) => validity.push(format!("eigen_mesh_spread={:.6}", (hi - lo) / lo)),
        }
    }
    let eigen_min = input
        .eigen
        .iter()
        .map(|e| e.multiplier)
        .fold(f64::INFINITY, f64::min);
    let converged: Vec<&CollapseSolve> = input.collapse.iter().filter(|c| c.converged).collect();
    for c in input.collapse.iter().filter(|c| !c.converged) {
        parts.push(open(format!(
            "collapse:not_converged:{}",
            c.imperfection_label
        )));
    }
    let collapse_min = converged
        .iter()
        .map(|c| c.limit_multiplier)
        .fold(f64::INFINITY, f64::min);
    let method = if input.finalist && !converged.is_empty() {
        if !converged.iter().any(|c| c.imperfection_amplitude_m > 0.0) {
            parts.push(open("collapse:no_declared_imperfection"));
        }
        if input.contact_applicable && !converged.iter().any(|c| c.contact_declared) {
            parts.push(open("collapse:contact_not_modelled"));
        }
        validity.push("nonlinear_collapse_with_declared_imperfections".into());
        "eigenbuckling_screen_then_nonlinear_collapse"
    } else {
        if input.finalist {
            parts.push(open("nonlinear_collapse:not_run"));
        } else {
            parts.push(open("buckling:eigen_screen_only_not_collapse_validated"));
        }
        validity.push("screen_not_collapse_validation".into());
        "eigenbuckling_screen_only"
    };
    let governing = eigen_min.min(collapse_min);
    let governing_multiplier = governing.is_finite().then_some(governing);
    if let Some(g) = governing_multiplier {
        if g < 1.0 {
            parts.push(fail("buckling_multiplier_below_frozen_demand"));
        }
    }
    let mut reference_ratio = None;
    if let (Some(r), Some(g)) = (input.reference, governing_multiplier) {
        match r.long_tube_collapse_pa() {
            Some(p) => {
                let ratio = g / (p / input.demand_reference_pa);
                validity.push(format!("long_tube_reference_applies;ratio={ratio:.4}"));
                reference_ratio = Some(ratio);
            }
            None => validity.push("long_tube_reference_outside_envelope".into()),
        }
    }
    Ok(BucklingReport {
        governing_multiplier,
        method,
        validity,
        reference_ratio,
        verdict: Verdict::combine(parts),
    })
}

// ---------------------------------------------------------------- V5-19-05 reserve

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LimitKind {
    Yield,
    Buckling,
    Contact,
    Modal,
    Fatigue,
}

impl LimitKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Yield => "yield",
            Self::Buckling => "buckling",
            Self::Contact => "contact",
            Self::Modal => "modal",
            Self::Fatigue => "fatigue",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    Linear,
    Nonlinear,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Derivation {
    /// Scaled from one solve at multiplier 1; valid only for a linear response.
    ScaledSingleSolve,
    /// Re-solved at `resolved_multiplier`, which must reach the claimed allowable.
    ExplicitResolves {
        solve_sha256: Sha256Hex,
        resolved_multiplier: f64,
    },
}

/// An allowable load multiplier of the frozen demand for one family.
#[derive(Clone, Debug, PartialEq)]
pub struct Limit {
    pub family: LoadFamily,
    pub kind: LimitKind,
    pub response: Response,
    pub derivation: Derivation,
    pub allowable_multiplier: f64,
    pub load_basis_sha256: Sha256Hex,
    /// Required before a modal or fatigue number may be treated as a load multiplier.
    pub multiplier_definition: Option<Field<String>>,
}

/// A frequency ratio or fatigue safety factor judged against its own requirement.
#[derive(Clone, Debug, PartialEq)]
pub struct SeparateRequirement {
    pub kind: LimitKind,
    pub name: String,
    pub value: f64,
    pub required: Field<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReserveReport {
    /// Allowable multiplier over required factor, per family, from compatible limits only.
    pub family_reserve: BTreeMap<LoadFamily, f64>,
    pub governing: Option<(LoadFamily, LimitKind, f64)>,
    pub separate: Vec<(String, Verdict)>,
    pub verdict: Verdict,
}

pub fn compute_reserve(
    frozen: &FrozenLoads,
    limits: &[Limit],
    separate: &[SeparateRequirement],
) -> Result<ReserveReport, JudgeError> {
    if limits
        .iter()
        .any(|l| !l.allowable_multiplier.is_finite() || l.allowable_multiplier <= 0.0)
        || separate.iter().any(|s| !s.value.is_finite())
    {
        return Err(invalid(
            "allowable multipliers must be finite and positive; separate values finite",
        ));
    }
    let mut parts = Vec::new();
    let mut family_reserve = BTreeMap::new();
    let mut governing: Option<(LoadFamily, LimitKind, f64)> = None;
    for family in ALL_FAMILIES {
        let name = family.name();
        let required = frozen
            .set
            .families
            .get(&family)
            .and_then(|d| d.required_factor.admitted())
            .copied();
        if required.is_none() {
            parts.push(open(format!("family:{name}:required_factor:not_admitted")));
        }
        let mut usable: Vec<&Limit> = Vec::new();
        for l in limits.iter().filter(|l| l.family == family) {
            let kn = l.kind.name();
            if l.load_basis_sha256 != frozen.sha256 {
                parts.push(fail(format!(
                    "family:{name}:{kn}:load_basis_differs_from_frozen_set"
                )));
            } else if matches!(l.kind, LimitKind::Modal | LimitKind::Fatigue)
                && l.multiplier_definition
                    .as_ref()
                    .and_then(Field::admitted)
                    .is_none()
            {
                parts.push(open(format!(
                    "family:{name}:{kn}:not_a_load_multiplier_without_definition"
                )));
            } else if let (Response::Nonlinear, Derivation::ScaledSingleSolve) =
                (l.response, &l.derivation)
            {
                parts.push(open(format!(
                    "family:{name}:{kn}:nonlinear_limit_needs_explicit_resolves"
                )));
            } else if let Derivation::ExplicitResolves {
                resolved_multiplier,
                ..
            } = &l.derivation
            {
                if !resolved_multiplier.is_finite()
                    || *resolved_multiplier < l.allowable_multiplier * (1.0 - 1e-12)
                {
                    parts.push(open(format!(
                        "family:{name}:{kn}:resolve_does_not_reach_claimed_multiplier"
                    )));
                } else {
                    usable.push(l);
                }
            } else {
                usable.push(l);
            }
        }
        if family == LoadFamily::ExternalDifferentialUpset
            && !usable.iter().any(|l| l.kind == LimitKind::Buckling)
        {
            parts.push(open("external_pressure_buckling:untested"));
        }
        let Some(worst) = usable
            .iter()
            .min_by(|a, b| a.allowable_multiplier.total_cmp(&b.allowable_multiplier))
        else {
            parts.push(open(format!("family:{name}:no_usable_limit")));
            continue;
        };
        let Some(required) = required else { continue };
        let reserve = worst.allowable_multiplier / required;
        family_reserve.insert(family, reserve);
        if governing.is_none_or(|(_, _, g)| reserve < g) {
            governing = Some((family, worst.kind, reserve));
        }
        if reserve < 1.0 {
            parts.push(fail(format!(
                "family:{name}:reserve_below_1:{}",
                worst.kind.name()
            )));
        }
    }
    let mut separate_out = Vec::new();
    for s in separate {
        let v = match s.required.admitted() {
            None => open(format!("{}:required_value:not_admitted", s.name)),
            Some(r) if s.value >= *r => Verdict::Pass,
            Some(_) => fail(format!(
                "{}:{}_below_own_requirement",
                s.name,
                s.kind.name()
            )),
        };
        parts.push(v.clone());
        separate_out.push((s.name.clone(), v));
    }
    Ok(ReserveReport {
        family_reserve,
        governing,
        separate: separate_out,
        verdict: Verdict::combine(parts),
    })
}
