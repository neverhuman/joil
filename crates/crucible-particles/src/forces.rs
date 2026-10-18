//! Stationary-frame light-drop force convention: gravity, undisturbed pressure
//! gradient `-grad(p)/rho_drop`, virtual mass `0.5 rho_f (D U/Dt - dV/dt)`,
//! and Schiller–Naumann drag. No centrifugal force is added: rotation enters
//! through the prescribed carrier field and pressure gradient.
use std::f64::consts::PI;

pub const C_VM: f64 = 0.5;

#[derive(Clone, Copy, Debug)]
pub struct FluidState {
    pub velocity: [f64; 3],
    /// Material acceleration `D U / Dt` in m/s².
    pub material_acceleration: [f64; 3],
    /// Static pressure gradient in Pa/m.
    pub pressure_gradient: [f64; 3],
    pub gravity: [f64; 3],
    pub rho: f64,
    pub mu: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct DropState {
    pub velocity: [f64; 3],
    pub diameter: f64,
    pub rho: f64,
}

fn norm(v: [f64; 3]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add_scaled(a: [f64; 3], b: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] + s * b[0], a[1] + s * b[1], a[2] + s * b[2]]
}

/// Acceleration from the BBO pressure-gradient, virtual-mass, gravity and drag terms.
/// Schiller–Naumann applies for spherical drops with particle Reynolds number below 1000.
pub fn acceleration(drop: DropState, fluid: FluidState) -> Result<[f64; 3], String> {
    if !drop.diameter.is_finite()
        || drop.diameter <= 0.0
        || !drop.rho.is_finite()
        || drop.rho <= 0.0
        || !fluid.rho.is_finite()
        || fluid.rho <= 0.0
        || !fluid.mu.is_finite()
        || fluid.mu <= 0.0
        || !drop
            .velocity
            .iter()
            .chain(fluid.velocity.iter())
            .chain(fluid.material_acceleration.iter())
            .chain(fluid.pressure_gradient.iter())
            .chain(fluid.gravity.iter())
            .all(|v| v.is_finite())
    {
        return Err("invalid force state".into());
    }
    let slip = sub(fluid.velocity, drop.velocity);
    let re = fluid.rho * norm(slip) * drop.diameter / fluid.mu;
    if re >= 1000.0 {
        return Err("Schiller–Naumann regime requires Re_p < 1000".into());
    }
    let drag_force_per_volume =
        18.0 * fluid.mu / drop.diameter.powi(2) * (1.0 + 0.15 * re.powf(0.687));
    let denom = drop.rho + C_VM * fluid.rho;
    let mut out = [0.0; 3];
    for i in 0..3 {
        let force_per_volume = -fluid.pressure_gradient[i]
            + C_VM * fluid.rho * fluid.material_acceleration[i]
            + (drop.rho - fluid.rho) * fluid.gravity[i]
            + drag_force_per_volume * slip[i];
        out[i] = force_per_volume / denom;
    }
    Ok(out)
}

/// Advance position/velocity with explicit midpoint substeps over a frozen uniform field.
/// Field variation must be handled by the caller at every substep.
pub fn advance_midpoint(
    position: [f64; 3],
    drop: DropState,
    fluid: FluidState,
    dt: f64,
    substeps: usize,
) -> Result<([f64; 3], DropState), String> {
    if !dt.is_finite() || dt <= 0.0 || substeps == 0 {
        return Err("positive dt and substeps are required".into());
    }
    let h = dt / substeps as f64;
    let mut x = position;
    let mut p = drop;
    for _ in 0..substeps {
        let a0 = acceleration(p, fluid)?;
        let mid_v = add_scaled(p.velocity, a0, h * 0.5);
        let mid = DropState {
            velocity: mid_v,
            ..p
        };
        let am = acceleration(mid, fluid)?;
        x = add_scaled(x, add_scaled(p.velocity, am, h * 0.5), h);
        p.velocity = add_scaled(p.velocity, am, h);
    }
    Ok((x, p))
}

pub fn spherical_mass(diameter_m: f64, rho: f64) -> Result<f64, String> {
    if !diameter_m.is_finite() || diameter_m <= 0.0 || !rho.is_finite() || rho <= 0.0 {
        return Err("invalid sphere".into());
    }
    Ok(PI / 6.0 * diameter_m.powi(3) * rho)
}
