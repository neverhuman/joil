//! Guarded Foundation 14 carrier execution and dimensional diagnostic extraction.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const PA_DIMENSIONS: [i8; 7] = [1, -1, -2, 0, 0, 0, 0];
pub const KINEMATIC_PRESSURE_DIMENSIONS: [i8; 7] = [0, 2, -2, 0, 0, 0, 0];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCard {
    pub schema_version: u32,
    pub verification_state: String,
    pub actual_runs: Vec<String>,
    pub absent: Vec<String>,
    pub environment_lock_sha256: String,
    pub source_revision: String,
    pub build: String,
    pub container_image_digest: String,
    pub compiler: String,
    pub mpi: String,
    pub models_available: Vec<String>,
    pub models_absent: Vec<String>,
    pub native_restart_supported: bool,
    pub native_restart_evidence: Option<String>,
    pub solver_executable_sha256: String,
    pub identity_evidence: IdentityEvidence,
    pub qualification: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityEvidence {
    pub package: String,
    pub precision: String,
    pub platform: String,
    pub solver_executable: String,
    pub model_evidence_limit: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedIdentity {
    pub source_revision: String,
    pub build: String,
    pub container_image_digest: String,
    pub compiler: String,
    pub mpi: String,
    pub model: String,
    pub solver_executable_sha256: String,
}

/// Exact executable identity reported by the pinned image's probe, not PATH presence.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeIdentity {
    pub source_revision: String,
    pub build: String,
    pub container_image_digest: String,
    pub compiler: String,
    pub mpi: String,
    pub models_available: Vec<String>,
    pub models_absent: Vec<String>,
    pub solver_executable_sha256: String,
}

pub fn read_capability_card(path: &Path) -> Result<CapabilityCard, String> {
    let card: CapabilityCard =
        serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("invalid OpenFOAM capability card: {e}"))?;
    if card.schema_version != 1
        || card.verification_state != "pinned_image_and_read_only_runtime_probes_verified"
        || card.native_restart_supported
        || card.solver_executable_sha256.len() != 64
        || card.environment_lock_sha256.len() != 64
    {
        return Err("capability card is unverified or has invalid identity evidence".into());
    }
    Ok(card)
}

pub fn runtime_identity(card: &CapabilityCard) -> RuntimeIdentity {
    RuntimeIdentity {
        source_revision: card.source_revision.clone(),
        build: card.build.clone(),
        container_image_digest: card.container_image_digest.clone(),
        compiler: card.compiler.clone(),
        mpi: card.mpi.clone(),
        models_available: card.models_available.clone(),
        models_absent: card.models_absent.clone(),
        solver_executable_sha256: card.solver_executable_sha256.clone(),
    }
}

pub fn verify_identity(
    expected: &ExpectedIdentity,
    actual: &RuntimeIdentity,
) -> Result<(), String> {
    if expected.source_revision != actual.source_revision
        || expected.build != actual.build
        || expected.container_image_digest != actual.container_image_digest
        || expected.compiler != actual.compiler
        || expected.mpi != actual.mpi
        || expected.solver_executable_sha256 != actual.solver_executable_sha256
        || !actual.models_available.iter().any(|m| m == &expected.model)
    {
        return Err("native executable identity/model does not match pinned capability".into());
    }
    Ok(())
}

/// Run only an executable already bound to a verified runtime identity.
pub fn execute_verified(
    executable: &Path,
    expected: &ExpectedIdentity,
    actual: &RuntimeIdentity,
    case: &Path,
    args: &[String],
    wall_limit: Duration,
) -> Result<NativeExecution, String> {
    verify_identity(expected, actual)?;
    if !executable.is_absolute() || !executable.is_file() || !case.is_dir() {
        return Err("solver and existing case must use explicit absolute paths".into());
    }
    if sha256_file(executable)? != actual.solver_executable_sha256 {
        return Err("solver executable digest does not match pinned capability card".into());
    }
    if wall_limit.is_zero() {
        return Err("native execution requires a positive wall-time limit".into());
    }
    let log_path = case.join(format!(
        "native-execution-{}.log",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));
    let log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&log_path)
        .map_err(|e| format!("cannot create native execution log: {e}"))?;
    let stderr = log.try_clone().map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut child = Command::new(executable)
        .args(args)
        .current_dir(case)
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|e| format!("failed to execute verified native solver: {e}"))?;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if start.elapsed() >= wall_limit {
            child.kill().map_err(|e| e.to_string())?;
            let _ = child.wait();
            return Err(format!(
                "native solver exceeded {}s wall limit; log retained at {}",
                wall_limit.as_secs_f64(),
                log_path.display()
            ));
        }
        std::thread::sleep(
            Duration::from_millis(20).min(wall_limit.saturating_sub(start.elapsed())),
        );
    };
    Ok(NativeExecution {
        exit_code: status.code(),
        elapsed_seconds: start.elapsed().as_secs_f64(),
        log_sha256: sha256_file(&log_path)?,
        log_path,
    })
}

#[derive(Clone, Debug)]
pub struct NativeExecution {
    pub exit_code: Option<i32>,
    pub elapsed_seconds: f64,
    pub log_sha256: String,
    pub log_path: std::path::PathBuf,
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PressureKind {
    Static,
    Total,
    Reduced,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressureReference {
    pub rho_kg_m3: f64,
    pub gravity_m_s2: [f64; 3],
    pub datum_position_m: [f64; 3],
    pub datum_absolute_pa: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressureResult {
    pub static_gauge_pa: f64,
    pub total_gauge_pa: f64,
    pub static_absolute_pa: Option<f64>,
    pub total_absolute_pa: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressureTapDrop {
    pub static_pa: f64,
    pub total_pa: f64,
}

pub fn pressure_tap_drop(upstream: PressureResult, downstream: PressureResult) -> PressureTapDrop {
    PressureTapDrop {
        static_pa: upstream.static_gauge_pa - downstream.static_gauge_pa,
        total_pa: upstream.total_gauge_pa - downstream.total_gauge_pa,
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Convert OpenFOAM p/p_rgh values to Pa and reconstruct total pressure.
/// Reduced pressure follows p_rgh = p - rho*g·(x-x_datum).
pub fn pressure(
    value: f64,
    dimensions: [i8; 7],
    kind: PressureKind,
    velocity_m_s: [f64; 3],
    position_m: [f64; 3],
    reference: PressureReference,
) -> Result<PressureResult, String> {
    if !value.is_finite()
        || !velocity_m_s
            .iter()
            .chain(position_m.iter())
            .chain(reference.gravity_m_s2.iter())
            .all(|x| x.is_finite())
        || !reference.rho_kg_m3.is_finite()
        || reference.rho_kg_m3 <= 0.0
        || reference
            .datum_absolute_pa
            .is_some_and(|p| !p.is_finite() || p <= 0.0)
    {
        return Err("invalid pressure input, density, gravity, or absolute datum".into());
    }
    let specific_pressure = match dimensions {
        PA_DIMENSIONS => value / reference.rho_kg_m3,
        KINEMATIC_PRESSURE_DIMENSIONS => value,
        _ => return Err("pressure field has unsupported dimensions".into()),
    };
    let hydrostatic = reference.rho_kg_m3
        * dot(
            reference.gravity_m_s2,
            [
                position_m[0] - reference.datum_position_m[0],
                position_m[1] - reference.datum_position_m[1],
                position_m[2] - reference.datum_position_m[2],
            ],
        );
    let static_gauge_pa = match kind {
        PressureKind::Static => specific_pressure * reference.rho_kg_m3,
        PressureKind::Total => {
            specific_pressure * reference.rho_kg_m3
                - 0.5 * reference.rho_kg_m3 * dot(velocity_m_s, velocity_m_s)
        }
        PressureKind::Reduced => specific_pressure * reference.rho_kg_m3 + hydrostatic,
    };
    let total_gauge_pa =
        static_gauge_pa + 0.5 * reference.rho_kg_m3 * dot(velocity_m_s, velocity_m_s);
    let absolute = reference.datum_absolute_pa;
    Ok(PressureResult {
        static_gauge_pa,
        total_gauge_pa,
        static_absolute_pa: absolute.map(|p| p + static_gauge_pa),
        total_absolute_pa: absolute.map(|p| p + total_gauge_pa),
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluxState {
    pub pressure_pa: f64,
    pub velocity_m_s: [f64; 3],
    pub area_m2: f64,
    pub mass_flow_kg_s: f64,
    pub elevation_m: f64,
}

/// Integrated mechanical energy flux; distinct from any pair of pressure taps.
pub fn mechanical_flux_w(state: FluxState, rho: f64, gravity_m_s2: f64) -> Result<f64, String> {
    let values = [
        state.pressure_pa,
        state.area_m2,
        state.mass_flow_kg_s,
        state.elevation_m,
        rho,
        gravity_m_s2,
    ];
    if !values.into_iter().all(f64::is_finite)
        || !state.velocity_m_s.into_iter().all(f64::is_finite)
        || state.area_m2 <= 0.0
        || rho <= 0.0
    {
        return Err("invalid control-volume flux state".into());
    }
    let speed2 = dot(state.velocity_m_s, state.velocity_m_s);
    Ok(state.mass_flow_kg_s
        * (state.pressure_pa / rho + 0.5 * speed2 + gravity_m_s2 * state.elevation_m))
}

pub mod controller;
pub use controller::*;
pub mod oil;
