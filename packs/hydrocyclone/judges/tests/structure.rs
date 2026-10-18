//! V5-19 acceptance on SYNTHETIC fixtures: load families, traction transfer, native-FEA oracle
//! fixtures, buckling and reserve. Synthetic admitted values only exercise decision logic; they
//! are not physical data and no native solver was run.
use pack_hydrocyclone_judges::material::*;
use pack_hydrocyclone_judges::structure::*;
use pack_hydrocyclone_judges::{Field, Sha256Hex, Verdict};
use std::collections::BTreeMap;
use std::f64::consts::PI;

const DRAFT: &[u8] = include_bytes!("../../../../physics/structure/v5/load-families.draft.json");

fn sha(label: &str) -> Sha256Hex {
    Sha256Hex::of_bytes(format!("synthetic-source:{label}").as_bytes())
}
fn adm<T>(value: T, label: &str) -> Field<T> {
    Field::Admitted {
        value,
        source: sha(label),
    }
}
fn reasons(v: &Verdict) -> Vec<String> {
    match v {
        Verdict::Pass => vec![],
        Verdict::Fail(r) | Verdict::Unresolved(r) => r.clone(),
    }
}
fn has(v: &Verdict, needle: &str) -> bool {
    reasons(v).iter().any(|r| r.contains(needle))
}
fn is_fail(v: &Verdict) -> bool {
    matches!(v, Verdict::Fail(_))
}
fn is_open(v: &Verdict) -> bool {
    matches!(v, Verdict::Unresolved(_))
}
fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs()
}

const REF: [(LoadFamily, f64); 5] = [
    (LoadFamily::OperatingFlow, 3.0e4),
    (LoadFamily::ExternalDifferentialUpset, 1.0e6),
    (LoadFamily::SupportPreloadContact, 2.0e3),
    (LoadFamily::Handling, 500.0),
    (LoadFamily::Thermal, 40.0),
];

/// SYNTHETIC: the draft set with every reference and required factor admitted.
fn synthetic_frozen() -> FrozenLoads {
    let mut set = LoadFamilySet::from_json(DRAFT).unwrap();
    set.set_id = "synthetic-admitted".into();
    set.synthetic = true;
    for (family, value) in REF {
        let d = set.families.get_mut(&family).unwrap();
        d.reference = adm(value, "ref");
        d.required_factor = adm(1.5, "factor");
    }
    set.freeze().unwrap()
}

// ------------------------------------------------------------------------- V5-19-01

#[test]
fn v5_19_01_draft_set_keeps_2_5_mpa_provisional_and_everything_else_unresolved() {
    let set = LoadFamilySet::from_json(DRAFT).unwrap();
    assert_eq!(set.families.len(), 5);
    let ext = &set.families[&LoadFamily::ExternalDifferentialUpset].reference;
    assert!(matches!(ext, Field::Proposed { value, .. } if *value == DRAFT_EXTERNAL_DELTA_P_PA));
    let v = set.verdict();
    assert!(is_open(&v), "{v:?}");
    assert!(has(
        &v,
        "external_differential_upset:reference:not_admitted"
    ));
    assert!(has(&v, "operating_flow:reference:not_admitted"));
    let frozen = set.freeze().unwrap();
    assert!(frozen
        .demand_at(LoadFamily::ExternalDifferentialUpset, 1.0)
        .is_err());
}

#[test]
fn v5_19_01_multiplier_one_is_the_same_external_demand_for_every_design() {
    let frozen = synthetic_frozen();
    assert!(frozen.set.verdict().is_pass());
    for (family, value) in REF {
        assert_eq!(frozen.demand_at(family, 1.0).unwrap(), value);
        assert_eq!(frozen.demand_at(family, 2.0).unwrap(), 2.0 * value);
    }
    assert!(frozen.demand_at(LoadFamily::Handling, 0.0).is_err());
    assert!(frozen.demand_at(LoadFamily::Handling, f64::NAN).is_err());
    for design in ["design-a", "design-b"] {
        let claim = DesignLoadClaim {
            design_sha256: sha(design),
            frozen_loads_sha256: frozen.sha256.clone(),
            reference_loads: REF.iter().cloned().collect(),
        };
        assert!(check_design_loads(&frozen, &claim).is_pass());
    }
    // identity changes when any reference changes
    let mut other = frozen.set.clone();
    other
        .families
        .get_mut(&LoadFamily::Thermal)
        .unwrap()
        .reference = adm(41.0, "ref");
    assert_ne!(other.freeze().unwrap().sha256, frozen.sha256);
}

#[test]
fn v5_19_01_a_weaker_design_cannot_choose_an_easier_reference_load() {
    let frozen = synthetic_frozen();
    let claim = |f: &dyn Fn(&mut DesignLoadClaim)| {
        let mut c = DesignLoadClaim {
            design_sha256: sha("weak"),
            frozen_loads_sha256: frozen.sha256.clone(),
            reference_loads: REF.iter().cloned().collect(),
        };
        f(&mut c);
        check_design_loads(&frozen, &c)
    };
    let easier = claim(&|c| {
        c.reference_loads
            .insert(LoadFamily::ExternalDifferentialUpset, 0.5e6);
    });
    assert!(is_fail(&easier));
    assert!(has(
        &easier,
        "reference_load_differs_from_frozen:external_differential_upset"
    ));
    // even a 1-ulp-easier value is a different demand
    let ulp = claim(&|c| {
        c.reference_loads.insert(
            LoadFamily::OperatingFlow,
            f64::from_bits(3.0e4_f64.to_bits() - 1),
        );
    });
    assert!(is_fail(&ulp));
    assert!(is_fail(&claim(&|c| {
        c.reference_loads.remove(&LoadFamily::Handling);
    })));
    assert!(is_fail(&claim(&|c| c.frozen_loads_sha256 = sha("own-set"))));
    // the unresolved draft set cannot be silently accepted either
    let draft = LoadFamilySet::from_json(DRAFT).unwrap().freeze().unwrap();
    let c = DesignLoadClaim {
        design_sha256: sha("x"),
        frozen_loads_sha256: draft.sha256.clone(),
        reference_loads: REF.iter().cloned().collect(),
    };
    assert!(!check_design_loads(&draft, &c).is_pass());
}

// ------------------------------------------------------------------------- V5-19-02

const R: f64 = 0.05;
const H: f64 = 0.1;
const P1: f64 = 2000.0;

fn wall_facets(n: usize) -> Vec<Facet> {
    (0..n)
        .map(|k| {
            let th = 2.0 * PI * (k as f64 + 0.5) / n as f64;
            Facet {
                patch: "wall".into(),
                position_m: [R * th.cos(), R * th.sin(), 0.05],
                normal: [th.cos(), th.sin(), 0.0],
                area_m2: 2.0 * PI * R * H / n as f64,
                pressure_pa: 30000.0 + P1 * th.cos(),
                shear_pa: [-3.0 * th.sin(), 3.0 * th.cos(), 1.5],
            }
        })
        .collect()
}
fn cap_facets(n: usize) -> Vec<Facet> {
    (0..n)
        .map(|k| {
            let th = 2.0 * PI * (k as f64 + 0.5) / n as f64;
            Facet {
                patch: "cap".into(),
                position_m: [0.03 * th.cos(), 0.03 * th.sin(), 0.1],
                normal: [0.0, 0.0, 1.0],
                area_m2: PI * 0.03 * 0.03 / n as f64,
                pressure_pa: 5000.0,
                shear_pa: [0.5, -0.25, 0.0],
            }
        })
        .collect()
}
fn force(f: &Facet) -> [f64; 3] {
    std::array::from_fn(|i| (-f.pressure_pa * f.normal[i] + f.shear_pa[i]) * f.area_m2)
}
/// Conservative remap: every source facet becomes two half-area facets with the same traction.
fn split(source: &[Facet]) -> Vec<Facet> {
    source
        .iter()
        .flat_map(|f| {
            let mut half = f.clone();
            half.area_m2 *= 0.5;
            [half.clone(), half]
        })
        .collect()
}
fn reactions_of(mapped: &[Facet]) -> Vec<Reaction> {
    mapped
        .iter()
        .map(|f| Reaction {
            position_m: f.position_m,
            force_n: force(f).map(|x| -x),
        })
        .collect()
}
fn budget() -> Field<MappingBudget> {
    adm(
        MappingBudget {
            relative: 1e-3,
            force_abs_n: 1e-6,
            moment_abs_n_m: 1e-6,
            area_relative: 1e-9,
        },
        "mapping-budget",
    )
}
fn transfer(mutate: &dyn Fn(&mut TractionTransfer)) -> TransferReport {
    let source: Vec<Facet> = wall_facets(64).into_iter().chain(cap_facets(16)).collect();
    let mapped = split(&source);
    let mut t = TractionTransfer {
        design_sha256: sha("candidate"),
        source_design_sha256: sha("candidate"),
        interpolation_id: "conservative-patch-remap-v1".into(),
        interpolation_sha256: sha("interp"),
        solid_mesh_sha256: sha("solid-mesh"),
        origin_m: [0.0; 3],
        required_patches: vec!["wall".into(), "cap".into()],
        reactions: reactions_of(&mapped),
        source,
        mapped,
    };
    mutate(&mut t);
    judge_transfer(&t, &budget(), &sha("solid-mesh")).unwrap()
}

#[test]
fn v5_19_02_conservative_transfer_fits_the_preallocated_budget() {
    let r = transfer(&|_| {});
    assert!(r.verdict.is_pass(), "{:?}", r.verdict);
    // hand value: the p1*cos(theta) part of the wall pressure gives F_x = -p1*pi*R*h
    let wall = &r.patch_errors["wall"];
    assert!(wall.force_residual_n < 1e-9 && wall.moment_residual_n_m < 1e-9);
    let source: Vec<Facet> = wall_facets(64);
    let fx: f64 = source.iter().map(|f| force(f)[0]).sum();
    assert!(close(fx, -P1 * PI * R * H, 1e-9), "{fx}");
    assert!(r.source_force_n[0] < 0.0 && close(r.source_force_n[0], r.mapped_force_n[0], 1e-12));
    // independent reaction integration closes the load
    for i in 0..3 {
        assert!((r.reaction_force_n[i] + r.mapped_force_n[i]).abs() < 1e-9);
    }
    assert_eq!(r.interpolation_id, "conservative-patch-remap-v1");
    // a tiny pressure drift inside the budget still passes; the same drift 100x larger fails
    let drift = |scale: f64| {
        transfer(&|t| {
            for f in &mut t.mapped {
                f.pressure_pa *= 1.0 + scale;
            }
            t.reactions = reactions_of(&t.mapped);
        })
    };
    assert!(drift(1e-5).verdict.is_pass());
    assert!(is_fail(&drift(1e-2).verdict));
}

#[test]
fn v5_19_02_copied_ct35_pressure_wrong_normals_and_missing_shear_or_patches_fail() {
    let copied = transfer(&|t| {
        for f in &mut t.mapped {
            f.pressure_pa = COPIED_CT35_PRESSURE_PA;
        }
        t.reactions = reactions_of(&t.mapped);
    });
    assert!(has(&copied.verdict, "copied_ct35_reference_pressure"));
    assert!(is_fail(&copied.verdict));

    let flipped = transfer(&|t| {
        for f in &mut t.mapped {
            f.normal = f.normal.map(|x| -x);
        }
        t.reactions = reactions_of(&t.mapped);
    });
    assert!(has(&flipped.verdict, "patch_force_not_conserved:wall"));
    assert!(has(
        &flipped.verdict,
        "patch_area_or_normals_not_preserved:cap"
    ));

    let no_shear = transfer(&|t| {
        for f in &mut t.mapped {
            f.shear_pa = [0.0; 3];
        }
        t.reactions = reactions_of(&t.mapped);
    });
    assert!(has(&no_shear.verdict, "shear_missing_after_transfer"));
    assert!(has(&no_shear.verdict, "patch_shear_not_conserved:wall"));

    let no_cap = transfer(&|t| {
        t.mapped.retain(|f| f.patch != "cap");
        t.reactions = reactions_of(&t.mapped);
    });
    assert!(has(&no_cap.verdict, "patch_missing:cap"));

    let pressure_only_source = transfer(&|t| {
        for f in t.source.iter_mut().chain(t.mapped.iter_mut()) {
            f.shear_pa = [0.0; 3];
        }
        t.reactions = reactions_of(&t.mapped);
    });
    assert!(has(
        &pressure_only_source.verdict,
        "shear_absent_in_source_field"
    ));
}

#[test]
fn v5_19_02_identity_budget_and_reactions_are_checked() {
    assert!(has(
        &transfer(&|t| t.source_design_sha256 = sha("other-design")).verdict,
        "source_field_belongs_to_another_design"
    ));
    assert!(has(
        &transfer(&|t| t.solid_mesh_sha256 = sha("other-mesh")).verdict,
        "solid_mesh_is_not_the_structural_mesh"
    ));
    let half_reactions = transfer(&|t| {
        let n = t.reactions.len() / 2;
        t.reactions.truncate(n);
    });
    assert!(has(
        &half_reactions.verdict,
        "reactions_do_not_balance_mapped_load"
    ));
    let none = transfer(&|t| t.reactions.clear());
    assert!(is_open(&none.verdict) && has(&none.verdict, "reactions:not_supplied"));
    // no admitted budget: unresolved, never pass
    let source = wall_facets(8)
        .into_iter()
        .chain(cap_facets(4))
        .collect::<Vec<_>>();
    let t = TractionTransfer {
        design_sha256: sha("c"),
        source_design_sha256: sha("c"),
        interpolation_id: "i".into(),
        interpolation_sha256: sha("i"),
        solid_mesh_sha256: sha("m"),
        origin_m: [0.0; 3],
        required_patches: vec!["wall".into()],
        reactions: reactions_of(&source),
        mapped: source.clone(),
        source,
    };
    let unresolved: Field<MappingBudget> = Field::Unresolved {
        reason: "synthetic".into(),
        owner: "structures_owner".into(),
        blocks: vec![],
    };
    let r = judge_transfer(&t, &unresolved, &sha("m")).unwrap();
    assert!(is_open(&r.verdict) && has(&r.verdict, "mapping_budget:not_admitted"));
    // malformed facets are errors, not verdicts
    let mut bad = t.clone();
    bad.mapped[0].normal = [2.0, 0.0, 0.0];
    assert!(judge_transfer(&bad, &budget(), &sha("m")).is_err());
    bad = t.clone();
    bad.source[0].area_m2 = f64::NAN;
    assert!(judge_transfer(&bad, &budget(), &sha("m")).is_err());
}

// ------------------------------------------------------------------------- V5-19-03

fn tol() -> Field<f64> {
    adm(1e-3, "fixture-tolerance")
}

fn lame_fixture(scale: f64) -> Fixture {
    let (a, b, po, e, nu) = (0.05, 0.06, 1.0e6, 113.8e9, 0.342);
    let (_, hoop, _, u) = lame_oracle(a, b, 0.0, po, e, nu, a).unwrap();
    Fixture::LameCylinder {
        inner_radius_m: a,
        outer_radius_m: b,
        internal_pa: 0.0,
        external_pa: po,
        youngs_pa: e,
        poisson: nu,
        solver_hoop_inner_pa: hoop * scale,
        solver_radial_displacement_inner_m: u * scale,
        tolerance_rel: tol(),
    }
}
fn cantilever_fixture(scale: f64, length_m: f64) -> Fixture {
    let (w, h, p, e, nu) = (0.01, 0.01, 100.0, 113.8e9, 0.342);
    let oracle = cantilever_timoshenko_deflection(length_m, w, h, p, e, nu);
    Fixture::Cantilever {
        length_m,
        width_m: w,
        height_m: h,
        tip_load_n: p,
        youngs_pa: e,
        poisson: nu,
        solver_tip_deflection_m: oracle * scale,
        tolerance_rel: tol(),
    }
}
fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture::Patch {
            applied_stress_pa: 100e6,
            solver_axial_stress_pa: vec![100e6 * (1.0 + 1e-6), 100e6 * (1.0 - 2e-6), 100e6],
            solver_off_axis_max_pa: 2.0e3,
            tolerance_rel: tol(),
        },
        Fixture::RigidBody {
            youngs_pa: 113.8e9,
            imposed_translation_m: 1e-3,
            body_length_m: 0.1,
            solver_max_stress_pa: 1.0e3,
            tolerance_rel: tol(),
        },
        lame_fixture(1.0 + 1e-5),
        cantilever_fixture(1.0 - 1e-4, 0.1),
        Fixture::ReactionEnergyBalance {
            applied_force_n: [0.0, 0.0, 1000.0],
            solver_reaction_force_n: [0.0, 0.0, -1000.0001],
            external_work_j: 0.35,
            strain_energy_j: 0.35 * (1.0 + 1e-6),
            tolerance_rel: tol(),
        },
        Fixture::MeshConvergence {
            values_coarse_to_fine: vec![0.9000, 0.9750, 0.9937, 0.9984],
            refinement_ratio: 2.0,
            tolerance_rel: adm(1e-2, "convergence"),
        },
    ]
}
fn process() -> ProcessSpec {
    ProcessSpec {
        solver: "synthetic-solver".into(),
        version: "0.0-synthetic".into(),
        binary_sha256: adm(sha("solver-binary"), "solver-binary"),
        program: "/opt/joil/solvers/synthetic-solver".into(),
        args: vec!["-i".into(), "job".into()],
        work_dir: "/home/ubuntu/joil/.private/scratch/fea-job".into(),
        max_threads: 2,
        wall_limit_s: 3600,
        max_rss_mib: 6144,
    }
}

#[test]
fn v5_19_03_oracles_match_hand_derived_values() {
    // Lamé, bore under external pressure: sigma_theta = -2 b^2 p_o / (b^2 - a^2), sigma_r(a) = 0
    let (a, b, po) = (0.05, 0.06, 1.0e6);
    let (sr, st, sz, _) = lame_oracle(a, b, 0.0, po, 113.8e9, 0.342, a).unwrap();
    assert!(close(st, -2.0 * b * b * po / (b * b - a * a), 1e-12));
    assert!(sr.abs() < 1e-6 && close(sz, -b * b * po / (b * b - a * a), 1e-12));
    let (sr_b, ..) = lame_oracle(a, b, 0.0, po, 113.8e9, 0.342, b).unwrap();
    assert!(close(sr_b, -po, 1e-12));
    assert!(lame_oracle(b, a, 0.0, po, 113.8e9, 0.342, a).is_err());
    // cantilever bending part: P L^3 / (3 E I) = 3.5149e-4 m for 100 N, 0.1 m, 10x10 mm Ti
    let bend = cantilever_bending_deflection(0.1, 0.01, 0.01, 100.0, 113.8e9);
    assert!(close(bend, 3.5149e-4, 1e-3), "{bend}");
    let timo = cantilever_timoshenko_deflection(0.1, 0.01, 0.01, 100.0, 113.8e9, 0.342);
    assert!(timo > bend && close(timo - bend, 2.83e-6, 2e-2));
}

#[test]
fn v5_19_03_solver_qualifies_only_when_every_oracle_fixture_passes() {
    let q = qualify_solver(&process(), &fixtures());
    assert!(q.verdict.is_pass(), "{:?}", q.verdict);
    assert_eq!(q.fixtures.len(), 6);
    for scale in [1.05, 0.9] {
        let mut f = fixtures();
        f[2] = lame_fixture(scale);
        let q = qualify_solver(&process(), &f);
        assert!(is_fail(&q.verdict) && has(&q.verdict, "lame_cylinder"));
    }
    let mut f = fixtures();
    f[3] = cantilever_fixture(1.02, 0.1);
    assert!(has(
        &qualify_solver(&process(), &f).verdict,
        "cantilever:tip_deflection"
    ));
    // a stiff short beam is outside beam theory: unresolved, not a pass
    f[3] = cantilever_fixture(1.0, 0.05);
    assert!(is_open(&qualify_solver(&process(), &f).verdict));
    let mut f = fixtures();
    if let Fixture::RigidBody {
        solver_max_stress_pa,
        ..
    } = &mut f[1]
    {
        *solver_max_stress_pa = 5.0e8;
    }
    assert!(has(
        &qualify_solver(&process(), &f).verdict,
        "rigid_body_stress"
    ));
    let mut f = fixtures();
    if let Fixture::ReactionEnergyBalance {
        strain_energy_j, ..
    } = &mut f[4]
    {
        *strain_energy_j = 0.2;
    }
    assert!(has(
        &qualify_solver(&process(), &f).verdict,
        "energy_balance"
    ));
    let mut f = fixtures();
    if let Fixture::MeshConvergence {
        values_coarse_to_fine,
        ..
    } = &mut f[5]
    {
        *values_coarse_to_fine = vec![1.0, 0.8, 0.95, 0.9];
    }
    assert!(has(&qualify_solver(&process(), &f).verdict, "not_monotone"));
    let mut f = fixtures();
    if let Fixture::MeshConvergence {
        values_coarse_to_fine,
        ..
    } = &mut f[5]
    {
        *values_coarse_to_fine = vec![0.9, 0.95, 0.97, 0.98];
    }
    assert!(has(
        &qualify_solver(&process(), &f).verdict,
        "finest_grid_change"
    ));
    // NaN output and an unadmitted tolerance are unresolved
    let mut f = fixtures();
    f[2] = lame_fixture(f64::NAN);
    assert!(is_open(&qualify_solver(&process(), &f).verdict));
    let mut f = fixtures();
    if let Fixture::Patch { tolerance_rel, .. } = &mut f[0] {
        *tolerance_rel = Field::Proposed {
            value: 1e-3,
            reason: "draft".into(),
        };
    }
    assert!(has(
        &qualify_solver(&process(), &f).verdict,
        "tolerance:not_admitted"
    ));
    // missing fixture kinds are unresolved
    let q = qualify_solver(&process(), &fixtures()[..5]);
    assert!(is_open(&q.verdict) && has(&q.verdict, "mesh_convergence:not_run"));
}

#[test]
fn v5_19_03_process_adapter_is_pinned_and_isolated() {
    assert!(process().judge().is_pass());
    let cmd = process().command();
    assert_eq!(cmd.get_program(), "/opt/joil/solvers/synthetic-solver");
    assert_eq!(cmd.get_args().count(), 2);
    assert_eq!(
        cmd.get_current_dir().unwrap().to_str().unwrap(),
        "/home/ubuntu/joil/.private/scratch/fea-job"
    );
    let envs: Vec<_> = cmd
        .get_envs()
        .map(|(k, _)| k.to_str().unwrap().to_string())
        .collect();
    assert_eq!(envs, ["CCX_NPROC_EQUATION_SOLVER", "OMP_NUM_THREADS"]);
    let relative = ProcessSpec {
        program: "ccx".into(),
        ..process()
    };
    assert!(has(&relative.judge(), "program_not_absolute_path"));
    let tmp = ProcessSpec {
        work_dir: "/tmp/fea".into(),
        ..process()
    };
    assert!(is_fail(&tmp.judge()));
    let unpinned = ProcessSpec {
        binary_sha256: Field::Proposed {
            value: sha("x"),
            reason: "unpinned".into(),
        },
        ..process()
    };
    assert!(has(&unpinned.judge(), "solver_binary:not_admitted"));
    assert!(is_fail(
        &ProcessSpec {
            max_rss_mib: 0,
            ..process()
        }
        .judge()
    ));
}

fn passport() -> MaterialPassport {
    let values: BTreeMap<ContextKey, String> = ALL_CONTEXT
        .iter()
        .map(|k| (*k, format!("synthetic {}", k.name())))
        .collect();
    MaterialPassport {
        schema: "joil.hydrocyclone.material-passport.v1".into(),
        passport_id: "synthetic-passport".into(),
        synthetic: true,
        context: values
            .iter()
            .map(|(k, v)| (*k, adm(v.clone(), k.name())))
            .collect(),
        density_kg_m3: adm(
            Bounds {
                lower: 4400.0,
                upper: 4460.0,
            },
            "density",
        ),
        allowables: BTreeMap::from([(
            AllowableKind::Yield,
            adm(
                Allowable {
                    value: 1.0,
                    unit: "MPa".into(),
                    evidence_context: values,
                },
                "yield",
            ),
        )]),
    }
}

fn good_run() -> StaticFeaRun {
    StaticFeaRun {
        basis: StructuralBasis::NativeSolid,
        design_sha256: sha("candidate"),
        mesh_sha256: sha("solid-mesh"),
        frozen_loads_sha256: synthetic_frozen().sha256,
        material_passport_id: "synthetic-passport".into(),
        support_evidence: adm(sha("support"), "support"),
        observables: vec![
            Observable {
                name: "wall_patch_mean_von_mises".into(),
                kind: ObservableKind::PatchAveragedStress {
                    patch_area_m2: 4.0e-4,
                },
                unit: "Pa".into(),
                value: 3.1e8,
            },
            Observable {
                name: "tip_deflection".into(),
                kind: ObservableKind::Displacement,
                unit: "m".into(),
                value: 1.2e-5,
            },
        ],
    }
}
fn qualified_solver() -> SolverQualificationReport {
    qualify_solver(&process(), &fixtures())
}
fn judge_run(
    run: &StaticFeaRun,
    solver: &SolverQualificationReport,
    p: &MaterialPassport,
) -> Verdict {
    qualify_static_run(run, solver, &transfer(&|_| {}), &synthetic_frozen(), p)
}

#[test]
fn v5_19_03_static_run_is_qualified_only_on_a_native_solid_solve_with_evidence() {
    let v = judge_run(&good_run(), &qualified_solver(), &passport());
    assert!(v.is_pass(), "{v:?}");
    // unqualified solver, foreign mesh/design/load set and unadmitted support are refused
    let mut f = fixtures();
    f[2] = lame_fixture(1.05);
    assert!(has(
        &judge_run(&good_run(), &qualify_solver(&process(), &f), &passport()),
        "solver:"
    ));
    let mut run = good_run();
    run.mesh_sha256 = sha("other-mesh");
    assert!(has(
        &judge_run(&run, &qualified_solver(), &passport()),
        "run_mesh_is_not_the_mapped_mesh"
    ));
    let mut run = good_run();
    run.design_sha256 = sha("other");
    assert!(is_fail(&judge_run(&run, &qualified_solver(), &passport())));
    let mut run = good_run();
    run.frozen_loads_sha256 = sha("easy-loads");
    assert!(has(
        &judge_run(&run, &qualified_solver(), &passport()),
        "different_load_set"
    ));
    let mut run = good_run();
    run.support_evidence = Field::Unresolved {
        reason: "s".into(),
        owner: "o".into(),
        blocks: vec![],
    };
    assert!(has(
        &judge_run(&run, &qualified_solver(), &passport()),
        "support_evidence:not_admitted"
    ));
    // material without admitted yield evidence blocks the claim
    let mut weak = passport();
    weak.allowables.clear();
    assert!(has(
        &judge_run(&good_run(), &qualified_solver(), &weak),
        "material:"
    ));
    // unresolved load set blocks the claim
    let draft = LoadFamilySet::from_json(DRAFT).unwrap().freeze().unwrap();
    let mut run = good_run();
    run.frozen_loads_sha256 = draft.sha256.clone();
    let v = qualify_static_run(
        &run,
        &qualified_solver(),
        &transfer(&|_| {}),
        &draft,
        &passport(),
    );
    assert!(is_open(&v) && has(&v, "loads:"));
    // a failed load transfer blocks it
    let bad = transfer(&|t| t.source_design_sha256 = sha("x"));
    let v = qualify_static_run(
        &good_run(),
        &qualified_solver(),
        &bad,
        &synthetic_frozen(),
        &passport(),
    );
    assert!(has(&v, "load_transfer:"));
}

#[test]
fn v5_19_03_fixed_load_formula_and_singular_point_stress_cannot_qualify() {
    let mut run = good_run();
    run.basis = StructuralBasis::AnalyticFixedLoad;
    let v = judge_run(&run, &qualified_solver(), &passport());
    assert!(
        is_fail(&v)
            && has(
                &v,
                "analytic_fixed_load_is_an_oracle_not_a_structural_qualification"
            )
    );
    let mut run = good_run();
    run.observables.push(Observable {
        name: "max_nodal_von_mises".into(),
        kind: ObservableKind::NodalPeakStress,
        unit: "Pa".into(),
        value: 5.0e8,
    });
    assert!(has(
        &judge_run(&run, &qualified_solver(), &passport()),
        "singular_point_stress_observable"
    ));
    let mut run = good_run();
    run.observables[0].value = f64::INFINITY;
    assert!(is_open(&judge_run(&run, &qualified_solver(), &passport())));
    let mut run = good_run();
    run.observables.clear();
    assert!(has(
        &judge_run(&run, &qualified_solver(), &passport()),
        "observables:none_declared"
    ));
}

// ------------------------------------------------------------------------- V5-19-04

fn eigen(multipliers: &[f64]) -> Vec<EigenSolve> {
    multipliers
        .iter()
        .enumerate()
        .map(|(i, m)| EigenSolve {
            mesh_sha256: sha(&format!("eigen-mesh-{i}")),
            element_count: 10_000 * (i as u64 + 1),
            multiplier: *m,
        })
        .collect()
}
fn collapse(
    label: &str,
    amplitude: f64,
    contact: bool,
    converged: bool,
    limit: f64,
) -> CollapseSolve {
    CollapseSolve {
        imperfection_label: label.into(),
        imperfection_amplitude_m: amplitude,
        imperfection_shape_sha256: sha("mode-1"),
        contact_declared: contact,
        mesh_sha256: sha("collapse-mesh"),
        converged,
        limit_multiplier: limit,
        solve_sha256: sha(&format!("collapse-{label}")),
    }
}
fn cylinder(length_m: f64) -> CylinderReference {
    CylinderReference {
        youngs_pa: 113.8e9,
        poisson: 0.342,
        wall_m: 0.002,
        radius_m: 0.05,
        length_m,
    }
}
fn buckling_input() -> BucklingInput {
    BucklingInput {
        finalist: true,
        contact_applicable: true,
        demand_reference_pa: 1.0e6,
        eigen: eigen(&[2.1, 2.08, 2.09]),
        collapse: vec![collapse("mode1-1e-4", 1e-4, true, true, 1.4)],
        reference: Some(cylinder(1.0)),
        mesh_spread_tolerance: adm(0.02, "mesh-spread"),
    }
}

#[test]
fn v5_19_04_governing_multiplier_is_reported_with_method_and_validity() {
    // hand value: E/(4(1-nu^2)) (t/R)^3 = 2.062e6 Pa for t/R = 0.04, valid for L >= 0.785 m
    let p = cylinder(1.0).long_tube_collapse_pa().unwrap();
    assert!(close(p, 2.062e6, 1e-3), "{p}");
    assert!(cylinder(0.2).long_tube_collapse_pa().is_none());
    let r = judge_buckling(&buckling_input()).unwrap();
    assert!(r.verdict.is_pass(), "{:?}", r.verdict);
    assert_eq!(r.governing_multiplier, Some(1.4));
    assert_eq!(r.method, "eigenbuckling_screen_then_nonlinear_collapse");
    assert!(r
        .validity
        .iter()
        .any(|v| v.contains("nonlinear_collapse_with_declared_imperfections")));
    assert!(close(r.reference_ratio.unwrap(), 1.4 / (p / 1.0e6), 1e-12));
    // a short cylinder is outside the reference envelope: reported, ratio absent
    let mut i = buckling_input();
    i.reference = Some(cylinder(0.2));
    let r = judge_buckling(&i).unwrap();
    assert!(r.reference_ratio.is_none());
    assert!(r
        .validity
        .iter()
        .any(|v| v == "long_tube_reference_outside_envelope"));
    // collapse above the eigen value never raises the governing multiplier
    let mut i = buckling_input();
    i.collapse = vec![collapse("stiffening", 1e-4, true, true, 3.0)];
    assert_eq!(judge_buckling(&i).unwrap().governing_multiplier, Some(2.08));
}

#[test]
fn v5_19_04_eigen_screen_alone_failed_solves_and_thin_evidence_never_pass() {
    let screen = |f: &dyn Fn(&mut BucklingInput)| {
        let mut i = buckling_input();
        f(&mut i);
        judge_buckling(&i).unwrap()
    };
    let r = screen(&|i| i.collapse.clear());
    assert!(has(&r.verdict, "nonlinear_collapse:not_run"));
    assert_eq!(r.method, "eigenbuckling_screen_only");
    assert!(r
        .validity
        .iter()
        .any(|v| v == "screen_not_collapse_validation"));
    let r = screen(&|i| i.finalist = false);
    assert!(has(&r.verdict, "eigen_screen_only_not_collapse_validated"));
    let r = screen(&|i| i.collapse = vec![collapse("diverged", 1e-4, true, false, 0.0)]);
    assert!(is_open(&r.verdict) && has(&r.verdict, "collapse:not_converged:diverged"));
    assert!(has(
        &screen(&|i| i.collapse[0].imperfection_amplitude_m = 0.0).verdict,
        "no_declared_imperfection"
    ));
    assert!(has(
        &screen(&|i| i.collapse[0].contact_declared = false).verdict,
        "contact_not_modelled"
    ));
    assert!(has(
        &screen(&|i| i.eigen.truncate(2)).verdict,
        "fewer_than_three_meshes"
    ));
    assert!(has(
        &screen(&|i| i.eigen = eigen(&[2.1, 1.6, 2.05])).verdict,
        "spread_exceeds_budget"
    ));
    assert!(has(
        &screen(&|i| i.mesh_spread_tolerance = Field::Proposed {
            value: 0.02,
            reason: "d".into()
        })
        .verdict,
        "tolerance:not_admitted"
    ));
    assert!(has(
        &screen(&|i| i.eigen.clear()).verdict,
        "eigenbuckling:not_run"
    ));
    // buckling below the frozen demand is a physical fail
    let r = screen(&|i| i.collapse = vec![collapse("weak", 1e-4, true, true, 0.8)]);
    assert!(is_fail(&r.verdict) && has(&r.verdict, "buckling_multiplier_below_frozen_demand"));
    assert!(judge_buckling(&BucklingInput {
        demand_reference_pa: 0.0,
        ..buckling_input()
    })
    .is_err());
    let mut i = buckling_input();
    i.eigen[0].multiplier = f64::NAN;
    assert!(judge_buckling(&i).is_err());
}

// ------------------------------------------------------------------------- V5-19-05

fn limit(family: LoadFamily, kind: LimitKind, allowable: f64) -> Limit {
    Limit {
        family,
        kind,
        response: Response::Linear,
        derivation: Derivation::ScaledSingleSolve,
        allowable_multiplier: allowable,
        load_basis_sha256: synthetic_frozen().sha256,
        multiplier_definition: None,
    }
}
fn buckling_limit(allowable: f64, resolved: f64) -> Limit {
    Limit {
        response: Response::Nonlinear,
        derivation: Derivation::ExplicitResolves {
            solve_sha256: sha("collapse"),
            resolved_multiplier: resolved,
        },
        ..limit(
            LoadFamily::ExternalDifferentialUpset,
            LimitKind::Buckling,
            allowable,
        )
    }
}
fn base_limits() -> Vec<Limit> {
    let mut v: Vec<Limit> = ALL_FAMILIES
        .iter()
        .map(|f| limit(*f, LimitKind::Yield, 3.0))
        .collect();
    v.push(buckling_limit(1.8, 1.8));
    v
}
fn separate(modal_value: f64, fatigue_value: f64) -> Vec<SeparateRequirement> {
    vec![
        SeparateRequirement {
            kind: LimitKind::Modal,
            name: "frequency_ratio".into(),
            value: modal_value,
            required: adm(2.2, "modal"),
        },
        SeparateRequirement {
            kind: LimitKind::Fatigue,
            name: "fatigue_safety_factor".into(),
            value: fatigue_value,
            required: adm(2.0, "fatigue"),
        },
    ]
}

#[test]
fn v5_19_05_reserve_is_allowable_over_required_per_family_with_a_common_governing_limit() {
    let r = compute_reserve(&synthetic_frozen(), &base_limits(), &separate(3.1, 2.4)).unwrap();
    assert!(r.verdict.is_pass(), "{:?}", r.verdict);
    // yield 3.0 / 1.5 = 2.0 everywhere; external buckling 1.8 / 1.5 = 1.2 governs
    assert!(close(
        r.family_reserve[&LoadFamily::OperatingFlow],
        2.0,
        1e-12
    ));
    assert!(close(
        r.family_reserve[&LoadFamily::ExternalDifferentialUpset],
        1.2,
        1e-12
    ));
    assert_eq!(
        r.governing.map(|g| (g.0, g.1)),
        Some((LoadFamily::ExternalDifferentialUpset, LimitKind::Buckling))
    );
    assert!(close(r.governing.unwrap().2, 1.2, 1e-12));
    assert!(r.separate.iter().all(|(_, v)| v.is_pass()));
}

#[test]
fn v5_19_05_high_yield_margin_cannot_mask_untested_external_pressure_buckling() {
    let mut limits = base_limits();
    limits.pop();
    limits[1].allowable_multiplier = 50.0;
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(is_open(&r.verdict) && has(&r.verdict, "external_pressure_buckling:untested"));
    assert!(close(
        r.family_reserve[&LoadFamily::ExternalDifferentialUpset],
        50.0 / 1.5,
        1e-12
    ));
    // a buckling limit whose re-solve never reached its claimed multiplier does not count
    let mut limits = base_limits();
    *limits.last_mut().unwrap() = buckling_limit(1.8, 1.2);
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(has(&r.verdict, "resolve_does_not_reach_claimed_multiplier"));
    assert!(has(&r.verdict, "external_pressure_buckling:untested"));
}

#[test]
fn v5_19_05_frequency_ratio_and_fatigue_factor_are_not_min_combined_as_load_multipliers() {
    // a frequency ratio of 1.1 and a fatigue factor of 0.9 fail their own requirements only
    let r = compute_reserve(&synthetic_frozen(), &base_limits(), &separate(1.1, 0.9)).unwrap();
    assert!(is_fail(&r.verdict));
    assert!(has(
        &r.verdict,
        "frequency_ratio:modal_below_own_requirement"
    ));
    assert!(has(
        &r.verdict,
        "fatigue_safety_factor:fatigue_below_own_requirement"
    ));
    assert!(
        close(r.governing.unwrap().2, 1.2, 1e-12),
        "separate numbers must not enter the min"
    );
    // presenting them as multiplier limits without a definition is refused and excluded
    for kind in [LimitKind::Modal, LimitKind::Fatigue] {
        let mut limits = base_limits();
        limits.push(limit(LoadFamily::OperatingFlow, kind, 0.4));
        let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
        assert!(is_open(&r.verdict) && has(&r.verdict, "not_a_load_multiplier_without_definition"));
        assert!(close(
            r.family_reserve[&LoadFamily::OperatingFlow],
            2.0,
            1e-12
        ));
    }
    // a justified, admitted multiplier definition makes the limit usable
    let mut limits = base_limits();
    let mut modal = limit(LoadFamily::OperatingFlow, LimitKind::Modal, 1.65);
    modal.multiplier_definition = Some(adm(
        "synthetic: pressure scale at which f1 reaches the excitation band".to_string(),
        "defn",
    ));
    limits.push(modal);
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(close(
        r.family_reserve[&LoadFamily::OperatingFlow],
        1.1,
        1e-12
    ));
}

#[test]
fn v5_19_05_nonlinear_limits_need_resolves_and_every_limit_cites_the_frozen_demand() {
    let mut limits = base_limits();
    let mut scaled = limit(LoadFamily::SupportPreloadContact, LimitKind::Contact, 9.0);
    scaled.response = Response::Nonlinear;
    limits.push(scaled);
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(has(
        &r.verdict,
        "contact:nonlinear_limit_needs_explicit_resolves"
    ));
    let mut limits = base_limits();
    limits[2].load_basis_sha256 = sha("easier-load-set");
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(is_fail(&r.verdict) && has(&r.verdict, "load_basis_differs_from_frozen_set"));
    let mut limits = base_limits();
    limits[3].allowable_multiplier = 1.2;
    let r = compute_reserve(&synthetic_frozen(), &limits, &[]).unwrap();
    assert!(is_fail(&r.verdict) && has(&r.verdict, "family:handling:reserve_below_1:yield"));
    let r = compute_reserve(&synthetic_frozen(), &base_limits()[..3], &[]).unwrap();
    assert!(has(&r.verdict, "family:handling:no_usable_limit"));
    // the unresolved draft set can never produce a pass
    let draft = LoadFamilySet::from_json(DRAFT).unwrap().freeze().unwrap();
    let mut limits = base_limits();
    for l in &mut limits {
        l.load_basis_sha256 = draft.sha256.clone();
    }
    let r = compute_reserve(&draft, &limits, &[]).unwrap();
    assert!(has(&r.verdict, "required_factor:not_admitted"));
    assert!(!r.verdict.is_pass() && r.governing.is_none());
    assert!(compute_reserve(
        &synthetic_frozen(),
        &[limit(LoadFamily::Thermal, LimitKind::Yield, f64::NAN)],
        &[]
    )
    .is_err());
}
