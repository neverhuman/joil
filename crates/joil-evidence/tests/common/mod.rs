#![allow(dead_code)]
use joil_evidence::{identity::*, observation::*, subject::*};

pub fn reference(id: &str) -> ContentRef {
    ContentRef::from_bytes(id, "1", id.as_bytes())
}
pub fn identity() -> ScientificIdentity {
    ScientificIdentity {
        schema: 2,
        subject: SubjectRef::Compiled {
            bundle: Box::new(SubjectBundle {
                genome: reference("genome"),
                ir: reference("ir"),
                compiler: reference("compiler"),
                geometry_kernel: reference("kernel"),
                compiler_config: reference("compile-config"),
                geometry: reference("geometry"),
                surface_mesh: reference("surface"),
                volume_mesh: Some(reference("volume")),
                solver_case: Some(reference("case")),
            }),
        },
        requirements: vec![reference("requirements")],
        source: reference("source"),
        build: reference("build"),
        cargo_lock: reference("lock"),
        config: reference("config"),
        evaluator: reference("evaluator"),
        scenario: reference("scenario"),
        objective: reference("objective"),
        descriptor: reference("descriptor"),
        solver_build: reference("solver"),
        model: reference("model"),
        numerics: reference("numerics"),
        process: reference("process"),
        material: MaterialState::Identified {
            passport: reference("material"),
        },
        dependencies: vec![],
    }
}

pub fn measurement() -> Measurement {
    Measurement {
        value: Finite::new(25.0).unwrap(),
        unit: Unit::MgPerLitreDispersedOil,
        quantity: "cout".into(),
        frame: "SI-right-handed".into(),
        region: "treated-outlet".into(),
        recovery: reference("recovery"),
        fidelity: Fidelity::F2,
        tier: EvidenceTier::NumericallyVerified,
        origin: Origin::Simulation,
        identity: identity(),
        evidence: vec![reference("raw")],
        applicability: reference("domain"),
        state: EvidenceState {
            execution: Execution::Succeeded,
            numerical: Numerical::Verified,
            validation: Validation::NotPerformed,
        },
        uncertainty: vec![Uncertainty {
            component: "numerical".into(),
            absolute: Finite::new(2.0).unwrap(),
            unit: Unit::MgPerLitreDispersedOil,
            method: "manufactured-test-fixture".into(),
            dependence: "uncombined-bound".into(),
            coverage: "deterministic-bound".into(),
        }],
    }
}
