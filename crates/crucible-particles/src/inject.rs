//! Injection quadrature over semantic boundary faces, using inward liquid normal flux.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InletFace {
    pub semantic_id: String,
    /// Unit outward normal from the fluid domain.
    pub outward_normal: [f64; 3],
    pub area_m2: f64,
    pub liquid_velocity_m_s: [f64; 3],
    /// Oil mass concentration in kg oil per m³ liquid.
    pub oil_concentration_kg_m3: f64,
    pub size_mass_fractions: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InjectionStratum {
    pub face_id: String,
    pub size_bin: usize,
    pub represented_oil_mass_kg: f64,
    pub momentum_kg_m_s: [f64; 3],
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Integrate a piecewise-constant flux for one synchronized interval.
/// Concentration basis is explicitly kg oil / m³ liquid; marker size is unrelated.
pub fn integrate_inlet(faces: &[InletFace], dt_s: f64) -> Result<Vec<InjectionStratum>, String> {
    if faces.is_empty() || !dt_s.is_finite() || dt_s <= 0.0 {
        return Err("nonempty faces and positive timestep required".into());
    }
    let mut out = Vec::new();
    for f in faces {
        let nmag = dot(f.outward_normal, f.outward_normal).sqrt();
        if f.semantic_id.is_empty()
            || !f.area_m2.is_finite()
            || f.area_m2 <= 0.0
            || (nmag - 1.0).abs() > 1e-8
            || !f
                .liquid_velocity_m_s
                .iter()
                .chain(f.outward_normal.iter())
                .all(|x| x.is_finite())
            || !f.oil_concentration_kg_m3.is_finite()
            || f.oil_concentration_kg_m3 < 0.0
            || f.size_mass_fractions.is_empty()
            || f.size_mass_fractions
                .iter()
                .any(|x| !x.is_finite() || *x < 0.0)
            || (f.size_mass_fractions.iter().sum::<f64>() - 1.0).abs() > 1e-10
        {
            return Err("invalid semantic inlet face or DSD mass basis".into());
        }
        let inward = (-dot(f.liquid_velocity_m_s, f.outward_normal)).max(0.0);
        let total = inward * f.area_m2 * dt_s * f.oil_concentration_kg_m3;
        for (bin, fraction) in f.size_mass_fractions.iter().enumerate() {
            let mass = total * fraction;
            out.push(InjectionStratum {
                face_id: f.semantic_id.clone(),
                size_bin: bin,
                represented_oil_mass_kg: mass,
                momentum_kg_m_s: f.liquid_velocity_m_s.map(|v| mass * v),
            });
        }
    }
    Ok(out)
}

pub fn reject_backflow_as_injection(face: &InletFace) -> Result<(), String> {
    let outward = dot(face.liquid_velocity_m_s, face.outward_normal);
    if outward > 0.0 {
        Err("backflow is outward and contributes zero injection; classify as outlet/backflow evidence".into())
    } else {
        Ok(())
    }
}
