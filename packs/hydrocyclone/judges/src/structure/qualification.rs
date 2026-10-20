use super::*;
use crate::material::{ClaimKind, MaterialPassport};
use crate::{invalid, positive, Field, JudgeError, Sha256Hex, Verdict};
use std::collections::{BTreeMap, BTreeSet};

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

mod reserve;
pub use reserve::*;
