use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::f64::consts::PI;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Genome {
    pub schema_version: u32,
    pub family: String,
    pub parameters: Parameters,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub d2_mm: f64,
    pub d1_over_d2: f64,
    pub l1_over_d1: f64,
    pub cone_half_angle_deg: f64,
    pub l2_over_d2: f64,
    pub d3_over_d2: f64,
    pub l3_over_d3: f64,
    pub do_over_d2: f64,
    pub inlet_area_ratio: f64,
    pub n_inlets: u32,
    pub inlet_h_over_w: f64,
    pub inlet_z_offset_mm: f64,
    pub inlet_length_cfd_mm: f64,
    pub inlet_length_part_mm: f64,
    pub orifice_length_over_do: f64,
    pub plenum_radius_mm: f64,
    pub plenum_length_mm: f64,
    pub head_spigot_over_d2: f64,
    pub wall_thickness_mm: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Dimensions {
    pub d1_mm: f64,
    pub d2_mm: f64,
    pub d3_mm: f64,
    pub chamber_length_mm: f64,
    pub cone_length_mm: f64,
    pub taper_length_mm: f64,
    pub tail_length_mm: f64,
    pub overflow_diameter_mm: f64,
    pub inlet_area_each_mm2: f64,
    pub inlet_width_mm: f64,
    pub inlet_height_mm: f64,
    pub z_cone_end_mm: f64,
    pub z_taper_end_mm: f64,
    pub z_head_end_mm: f64,
    pub z_underflow_mm: f64,
    pub fluid_volume_mm3: f64,
}

impl Genome {
    pub fn derive(&self) -> Result<Dimensions, String> {
        if self.schema_version != 1 || self.family != "ct35-parametric-v1" {
            return Err("unsupported design schema or family; no fallback geometry".into());
        }
        let p = &self.parameters;
        let positive = [
            p.d2_mm,
            p.d1_over_d2,
            p.l1_over_d1,
            p.l2_over_d2,
            p.d3_over_d2,
            p.l3_over_d3,
            p.do_over_d2,
            p.inlet_area_ratio,
            p.inlet_h_over_w,
            p.inlet_length_cfd_mm,
            p.inlet_length_part_mm,
            p.orifice_length_over_do,
            p.plenum_radius_mm,
            p.plenum_length_mm,
            p.head_spigot_over_d2,
            p.wall_thickness_mm,
        ];
        if positive.iter().any(|x| !x.is_finite() || *x <= 0.0)
            || !p.cone_half_angle_deg.is_finite()
            || p.cone_half_angle_deg <= 0.0
            || p.cone_half_angle_deg >= 89.0
            || !p.inlet_z_offset_mm.is_finite()
            || p.inlet_z_offset_mm < 0.0
            || p.n_inlets != 2
            || p.d1_over_d2 <= 1.0
            || p.d3_over_d2 >= 1.0
        {
            return Err("invalid finite geometry parameters or unsupported inlet count".into());
        }
        let d2 = p.d2_mm;
        let d1 = p.d1_over_d2 * d2;
        let d3 = p.d3_over_d2 * d2;
        let (r1, r2, r3) = (d1 / 2.0, d2 / 2.0, d3 / 2.0);
        let l1 = p.l1_over_d1 * d1;
        let lc = (r1 - r2) / p.cone_half_angle_deg.to_radians().tan();
        let l2 = p.l2_over_d2 * d2;
        let l3 = p.l3_over_d3 * d3;
        let area = p.inlet_area_ratio * PI * d1 * d1 / 4.0 / 2.0;
        let w = (area / p.inlet_h_over_w).sqrt();
        let h = p.inlet_h_over_w * w;
        let ro = p.do_over_d2 * d2 / 2.0;
        if w >= r1
            || p.inlet_z_offset_mm + h >= l1
            || p.inlet_length_cfd_mm <= r1
            || p.inlet_length_part_mm <= r1
            || p.head_spigot_over_d2 * d2 >= l2
            || p.plenum_radius_mm <= ro
        {
            return Err("inlet, spigot or plenum does not fit baseline geometry".into());
        }
        let cap_integral =
            |y: f64| y * (r1 * r1 - y * y).max(0.0).sqrt() / 2.0 + r1 * r1 * (y / r1).asin() / 2.0;
        let frustum = |l: f64, a: f64, b: f64| PI * l * (a * a + a * b + b * b) / 3.0;
        let volume = PI * r1 * r1 * l1
            + frustum(lc, r1, r2)
            + frustum(l2, r2, r3)
            + PI * r3 * r3 * l3
            + 2.0 * h * (p.inlet_length_cfd_mm * w - (cap_integral(r1) - cap_integral(r1 - w)))
            + PI * ro * ro * (2.0 * ro * p.orifice_length_over_do)
            + PI * p.plenum_radius_mm.powi(2) * p.plenum_length_mm;
        let d = Dimensions {
            d1_mm: d1,
            d2_mm: d2,
            d3_mm: d3,
            chamber_length_mm: l1,
            cone_length_mm: lc,
            taper_length_mm: l2,
            tail_length_mm: l3,
            overflow_diameter_mm: 2.0 * ro,
            inlet_area_each_mm2: area,
            inlet_width_mm: w,
            inlet_height_mm: h,
            z_cone_end_mm: l1 + lc,
            z_taper_end_mm: l1 + lc + l2,
            z_head_end_mm: l1 + lc + p.head_spigot_over_d2 * d2,
            z_underflow_mm: l1 + lc + l2 + l3,
            fluid_volume_mm3: volume,
        };
        if !d.fluid_volume_mm3.is_finite()
            || d.fluid_volume_mm3 <= 0.0
            || !d.z_underflow_mm.is_finite()
        {
            return Err("derived geometry overflow".into());
        }
        Ok(d)
    }

    /// Version-local identity. Cross-language canonicalization is a P01 task.
    pub fn sha256(&self) -> Result<String, String> {
        self.derive()?;
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        Ok(Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }
}

pub fn baseline() -> Genome {
    serde_json::from_str(include_str!("../../../baselines/ct35/genome.json"))
        .expect("committed baseline JSON must decode")
}
