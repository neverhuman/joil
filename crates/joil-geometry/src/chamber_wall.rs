//! Analytic implicit lowering of the versioned CT-35 chamber-wall proposal.
//! This represents the intended Fourier boundary, not the CAD spline/loft error.
pub mod ct35;
use crate::csg::{self, Boolean, BoundedField, Builder};
use crate::{Axis, EvaluationBudget, GeometryError, Op};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FourierMode {
    pub azimuthal_order: u32,
    pub cos_mm: f64,
    pub sin_mm: f64,
    pub twist_rad: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChamberWallProposal {
    pub schema_version: u32,
    pub family: String,
    pub baseline_file_sha256: String,
    pub modes: Vec<FourierMode>,
}
impl ChamberWallProposal {
    pub fn baseline() -> Self {
        Self {
            schema_version: 1,
            family: "ct35-chamber-wall-field-v1".into(),
            baseline_file_sha256: ct35::GENOME_SHA256.into(),
            modes: vec![],
        }
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, GeometryError> {
        let p: Self =
            joil_evidence::identity::strict_decode(bytes).map_err(GeometryError::Backend)?;
        p.validate()?;
        Ok(p)
    }
    pub fn validate(&self) -> Result<(), GeometryError> {
        if self.schema_version != 1
            || self.family != "ct35-chamber-wall-field-v1"
            || self.baseline_file_sha256 != ct35::GENOME_SHA256
            || self.modes.len() > 8
        {
            return Err(GeometryError::InvalidInput(
                "wall schema/baseline/mode budget",
            ));
        }
        let mut amplitude = 0.0;
        for m in &self.modes {
            if !(2..=8).contains(&m.azimuthal_order)
                || ![m.cos_mm, m.sin_mm, m.twist_rad]
                    .iter()
                    .all(|x| x.is_finite())
                || m.twist_rad.abs() > 2.0 * std::f64::consts::PI
            {
                return Err(GeometryError::InvalidInput(
                    "wall mode/order/coefficient/twist",
                ));
            }
            amplitude += m.cos_mm.hypot(m.sin_mm);
        }
        // Same envelope as cad/program/wall_field.py; no rounding/clipping genes.
        if amplitude > 1.8 + 1e-12 {
            return Err(GeometryError::InvalidInput(
                "wall amplitude outside CT-35 envelope",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct LoweredWall {
    pub field: BoundedField,
    pub proposal_sha256: String,
    pub baseline_sha256: String,
    pub reference_sha256: String,
    pub maximum_radial_change_m: f64,
    pub frozen: ct35::FrozenFacts,
    pub approximation_scope: &'static str,
}
fn cap(b: &mut Builder, radial: usize, z: usize, lower: f64, upper: f64) -> usize {
    let minus = b.scale(z, -1.0);
    let lo = b.offset(minus, lower);
    let hi = b.offset(z, -upper);
    let zbound = b.push(Op::Maximum(lo, hi));
    b.push(Op::Maximum(radial, zbound))
}
fn frustum(lower: f64, upper: f64, r0: f64, r1: f64) -> Result<BoundedField, GeometryError> {
    let mut b = Builder::default();
    let x = b.push(Op::Coordinate(Axis::X));
    let y = b.push(Op::Coordinate(Axis::Y));
    let z = b.push(Op::Coordinate(Axis::Z));
    let rho = b.length(&[x, y]);
    let dz = b.offset(z, -lower);
    let slope = (r1 - r0) / (upper - lower);
    let scaled = b.scale(dz, slope);
    let radius = b.offset(scaled, r0);
    let radial = b.push(Op::Subtract(rho, radius));
    let out = cap(&mut b, radial, z, lower, upper);
    let mut f = BoundedField::algebraic(b.finish(out)?);
    f.lipschitz = Some(1.0f64.hypot(slope).next_up());
    Ok(f)
}
fn chamber(p: &ChamberWallProposal) -> Result<BoundedField, GeometryError> {
    let active: Vec<_> = p
        .modes
        .iter()
        .filter(|m| m.cos_mm != 0.0 || m.sin_mm != 0.0)
        .collect();
    if active.is_empty() {
        return frustum(0.0, ct35::L1_M, ct35::D1_M / 2.0, ct35::D1_M / 2.0);
    }
    let mut b = Builder::default();
    let x = b.push(Op::Coordinate(Axis::X));
    let y = b.push(Op::Coordinate(Axis::Y));
    let z = b.push(Op::Coordinate(Axis::Z));
    let rho = b.length(&[x, y]);
    let theta = b.push(Op::Atan2(y, x));
    let s = b.offset(z, -0.020);
    let s = b.scale(s, 1.0 / 0.045);
    let zero = b.constant(0.0);
    let one = b.constant(1.0);
    let s = b.push(Op::Maximum(s, zero));
    let s = b.push(Op::Minimum(s, one));
    // sin(pi*s)*sin(pi*(1-s)) equals sin²(pi*s), and is exactly zero
    // in floating arithmetic outside both band endpoints (one argument is 0).
    let a = b.scale(s, std::f64::consts::PI);
    let a = b.push(Op::Sine(a));
    let complement = b.push(Op::Subtract(one, s));
    let d = b.scale(complement, std::f64::consts::PI);
    let d = b.push(Op::Sine(d));
    let window = b.push(Op::Multiply(a, d));
    let mut sum = zero;
    for mode in active {
        let angle = b.scale(theta, mode.azimuthal_order as f64);
        let twist = b.scale(s, mode.twist_rad);
        let phase = b.push(Op::Add(angle, twist));
        let cosine = b.push(Op::Cosine(phase));
        let sine = b.push(Op::Sine(phase));
        let cosine = b.scale(cosine, mode.cos_mm * 0.001);
        let sine = b.scale(sine, mode.sin_mm * 0.001);
        let term = b.push(Op::Add(cosine, sine));
        sum = b.push(Op::Add(sum, term));
    }
    let perturbation = b.push(Op::Multiply(window, sum));
    let radius = b.offset(perturbation, ct35::D1_M / 2.0);
    let radial = b.push(Op::Subtract(rho, radius));
    let out = cap(&mut b, radial, z, 0.0, ct35::L1_M);
    // atan2 has an axis singularity: no global distance/Lipschitz claim.
    Ok(BoundedField::algebraic(b.finish(out)?))
}
pub fn lower(
    p: &ChamberWallProposal,
    genome: &[u8],
    reference: &[u8],
    facts: &ct35::FrozenFacts,
    budget: &mut EvaluationBudget,
) -> Result<LoweredWall, GeometryError> {
    p.validate()?;
    ct35::verify_sources(genome, reference).map_err(GeometryError::Backend)?;
    ct35::verify_facts(facts).map_err(GeometryError::Backend)?;
    let mut field = chamber(p)?;
    for (lo, hi, r0, r1) in [
        (ct35::L1_M, ct35::Z_CONE_END_M, 0.035, 0.0175),
        (ct35::Z_CONE_END_M, ct35::Z_TAPER_END_M, 0.0175, 0.00875),
        (ct35::Z_TAPER_END_M, ct35::Z_END_M, 0.00875, 0.00875),
        (-0.0028, 0.0, 0.0007, 0.0007),
        (-0.0228, -0.0028, 0.006, 0.006),
    ] {
        field = csg::boolean(&field, &frustum(lo, hi, r0, r1)?, Boolean::Union)?;
    }
    let w = ct35::INLET_W_M;
    let h = ct35::INLET_H_M;
    for sign in [-1.0, 1.0] {
        let inlet = csg::cuboid(
            [sign * 0.030, -sign * (0.035 - w / 2.0), h / 2.0],
            [0.030, w / 2.0, h / 2.0],
        )?;
        field = csg::boolean(&field, &inlet, Boolean::Union)?;
    }
    budget.charge(field.program().nodes().len())?;
    Ok(LoweredWall{field,proposal_sha256:joil_evidence::identity::scientific_hash(p).map_err(GeometryError::Backend)?,baseline_sha256:ct35::GENOME_SHA256.into(),reference_sha256:ct35::REFERENCE_SHA256.into(),maximum_radial_change_m:p.modes.iter().map(|m|m.cos_mm.hypot(m.sin_mm)*0.001).sum(),frozen:facts.clone(),approximation_scope:"Analytic Fourier boundary only; CAD loft/B-rep distance, topology, manufactured solids and physical performance remain unverified"})
}
