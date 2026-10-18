//! Source-bound public research reconstruction constants. No installed-hardware fit claim.
use serde::{Deserialize, Serialize};
pub const GENOME_SHA256: &str = "9d03400f48b68043c125611de927983efafb73122c9b03278e31f81b2c666ad5";
pub const REFERENCE_SHA256: &str =
    "9db5bbef73005888ca42d2cfe8c2b7fab938b18fa174dc2c41e7a9c95563f994";
pub const D1_M: f64 = 0.070;
pub const D2_M: f64 = 0.035;
pub const D3_M: f64 = 0.0175;
pub const L1_M: f64 = 0.070;
pub const L2_M: f64 = 0.770;
pub const L3_M: f64 = 0.700;
pub const Z_CONE_END_M: f64 = 0.16924743184330993;
pub const Z_TAPER_END_M: f64 = 0.9392474318433099;
// Preserve the source reference decimal verbatim; its f64 rounding is deliberate.
#[allow(clippy::excessive_precision)]
pub const Z_END_M: f64 = 1.6392474318433099;
pub const Z_HEAD_M: f64 = 0.18674743184330993;
/// Last 0.5D2 of the reducing cone, not the head spigot or an invented 166mm slice.
pub const COLLAR_M: [f64; 2] = [Z_CONE_END_M - 0.0175, Z_CONE_END_M];
pub const INLET_W_M: f64 = 0.007754485597711633;
pub const INLET_H_M: f64 = 0.015508971195423266;
pub const INLET_AREA_M2: f64 = 0.00012026409377023427;
pub const OVERFLOW_PLANE_M: f64 = -0.0228;
pub const FLUID_VOLUME_M3: f64 = 0.0011058025227442954;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Terminal {
    pub tag: u16,
    pub centre_m: [f64; 3],
    pub normal: [f64; 3],
    pub area_m2: f64,
}
pub fn terminals() -> Vec<Terminal> {
    let y = 0.035 - INLET_W_M / 2.0;
    let z = INLET_H_M / 2.0;
    vec![
        Terminal {
            tag: 1,
            centre_m: [-0.060, y, z],
            normal: [-1.0, 0.0, 0.0],
            area_m2: INLET_AREA_M2,
        },
        Terminal {
            tag: 2,
            centre_m: [0.060, -y, z],
            normal: [1.0, 0.0, 0.0],
            area_m2: INLET_AREA_M2,
        },
        Terminal {
            tag: 3,
            centre_m: [0.0, 0.0, OVERFLOW_PLANE_M],
            normal: [0.0, 0.0, -1.0],
            area_m2: std::f64::consts::PI * 0.006f64.powi(2),
        },
        Terminal {
            tag: 4,
            centre_m: [0.0, 0.0, Z_END_M],
            normal: [0.0, 0.0, 1.0],
            area_m2: std::f64::consts::PI * (D3_M / 2.0).powi(2),
        },
    ]
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FrozenFacts {
    pub d3_m: f64,
    pub l2_m: f64,
    pub l3_m: f64,
    pub collar_m: [f64; 2],
    pub terminals: Vec<Terminal>,
}
pub fn frozen() -> FrozenFacts {
    FrozenFacts {
        d3_m: D3_M,
        l2_m: L2_M,
        l3_m: L3_M,
        collar_m: COLLAR_M,
        terminals: terminals(),
    }
}
pub fn verify_facts(actual: &FrozenFacts) -> Result<(), String> {
    if actual != &frozen() {
        return Err("frozen CT-35 contract changed; no silent repair".into());
    }
    Ok(())
}
pub fn verify_sources(genome: &[u8], reference: &[u8]) -> Result<(), String> {
    if crucible_contract::subject::BlobDigest::of_bytes(genome).as_str() != GENOME_SHA256
        || crucible_contract::subject::BlobDigest::of_bytes(reference).as_str() != REFERENCE_SHA256
    {
        return Err("CT-35 source bytes mismatch".into());
    }
    let g: serde_json::Value = crucible_contract::strict_decode(genome)?;
    let r: serde_json::Value = crucible_contract::strict_decode(reference)?;
    let p = &g["parameters"];
    if p["d2_mm"] != 35.0 || p["l2_over_d2"] != 22.0 || p["l3_over_d3"] != 40.0 {
        return Err("baseline parameter mismatch".into());
    }
    for (key, expected) in [
        ("D1", D1_M),
        ("D2", D2_M),
        ("D3", D3_M),
        ("L1", L1_M),
        ("L2", L2_M),
        ("L3", L3_M),
        ("z_cone_end", Z_CONE_END_M),
        ("z_taper_end", Z_TAPER_END_M),
        ("z_end", Z_END_M),
        ("z_head", Z_HEAD_M),
        ("inlet_w", INLET_W_M),
        ("inlet_h", INLET_H_M),
    ] {
        let measured = r["derived_mm"][key]
            .as_f64()
            .ok_or("missing derived reference")?
            * 0.001;
        if (measured - expected).abs() > 1e-15 {
            return Err(format!("reference constant {key}"));
        }
    }
    Ok(())
}
