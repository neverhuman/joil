//! Explicit preservation/lowering of historical macro records; no unsupported radial proxy.
use crate::schema::{InterfaceRef, ModuleKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedMacro {
    pub name: String,
    pub source_fields: Vec<String>,
    pub interpretation: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "detail")]
pub enum Lowering {
    Supported {
        macros: Vec<NamedMacro>,
        primitives: Vec<ModuleKind>,
    },
    Unsupported {
        reason: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationRecord {
    pub source_schema: String,
    pub source_sha256: String,
    pub interface: InterfaceRef,
    pub retained_fields: BTreeMap<String, Value>,
    pub lowering: Lowering,
    /// None means no certified baseline bound was present in the supplied reference.
    pub certified_baseline_error_bounds: Option<BTreeMap<String, Value>>,
    pub zero_mutation_contract_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyError {
    Wire(String),
    Missing(&'static str),
    Invalid(&'static str),
}
impl std::fmt::Display for LegacyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for LegacyError {}
fn digest_bytes(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// Keeps every source key/value. CT-35 scaling choices remain labelled as choices,
/// not independently audited physical truth.
pub fn lower_ct35(bytes: &[u8], interface: InterfaceRef) -> Result<MigrationRecord, LegacyError> {
    let value: Value = joil_evidence::identity::strict_decode(bytes).map_err(LegacyError::Wire)?;
    if value.get("schema_version").and_then(Value::as_u64) != Some(1)
        || value.get("family").and_then(Value::as_str) != Some("ct35-parametric-v1")
    {
        return Err(LegacyError::Invalid("not CT-35 parametric v1"));
    }
    let params = value
        .get("parameters")
        .and_then(Value::as_object)
        .ok_or(LegacyError::Missing("parameters"))?;
    let retained_fields = value
        .as_object()
        .ok_or(LegacyError::Invalid("CT-35 source object"))?
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut macros = vec![
        NamedMacro {
            name: "ct35-inlet-pair".into(),
            source_fields: vec![
                "n_inlets".into(),
                "inlet_area_ratio".into(),
                "inlet_h_over_w".into(),
                "inlet_z_offset_mm".into(),
                "inlet_length_cfd_mm".into(),
                "inlet_length_part_mm".into(),
            ],
            interpretation: "two equal tangential feeds; CFD and part lengths remain distinct"
                .into(),
        },
        NamedMacro {
            name: "ct35-chamber-cone".into(),
            source_fields: vec![
                "d1_over_d2".into(),
                "l1_over_d1".into(),
                "cone_half_angle_deg".into(),
                "l2_over_d2".into(),
            ],
            interpretation: "legacy chamber and reducing cone dimensions".into(),
        },
        NamedMacro {
            name: "ct35-tail-overflow".into(),
            source_fields: vec![
                "d3_over_d2".into(),
                "l3_over_d3".into(),
                "do_over_d2".into(),
                "orifice_length_over_do".into(),
                "plenum_radius_mm".into(),
                "plenum_length_mm".into(),
            ],
            interpretation: "legacy vortex finder, overflow bore and CFD plenum".into(),
        },
        NamedMacro {
            name: "ct35-part-head".into(),
            source_fields: vec!["head_spigot_over_d2".into(), "wall_thickness_mm".into()],
            interpretation: "part-only head/spigot choices preserved separately".into(),
        },
    ];
    for m in &macros {
        for f in &m.source_fields {
            if !params.contains_key(f) {
                return Err(LegacyError::Missing("CT-35 field required by macro"));
            }
        }
    }
    macros.shrink_to_fit();
    let contract =
        joil_evidence::identity::scientific_hash(&interface).map_err(LegacyError::Wire)?;
    Ok(MigrationRecord {
        source_schema: "ct35-parametric-v1".into(),
        source_sha256: digest_bytes(bytes),
        interface,
        retained_fields,
        lowering: Lowering::Supported {
            macros,
            primitives: vec![
                ModuleKind::Chamber,
                ModuleKind::Sweep,
                ModuleKind::Junction,
                ModuleKind::Shell,
            ],
        },
        certified_baseline_error_bounds: None,
        zero_mutation_contract_sha256: contract,
    })
}

/// Legacy layer migration preserves all fields and refuses exotic families whose
/// geometry semantics are not implemented by the staged vocabulary.
pub fn lower_legacy_layers(
    version: u8,
    bytes: &[u8],
    interface: InterfaceRef,
) -> Result<MigrationRecord, LegacyError> {
    if !(1..=4).contains(&version) {
        return Err(LegacyError::Invalid("legacy genome version"));
    }
    let value: Value = joil_evidence::identity::strict_decode(bytes).map_err(LegacyError::Wire)?;
    let object = value
        .as_object()
        .ok_or(LegacyError::Invalid("legacy record must be object"))?;
    let retained_fields = object
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<BTreeMap<_, _>>();
    let exotic = object.iter().any(|(k, v)| {
        (k.contains("tpms") || k.contains("radial_proxy") || k.contains("neural"))
            && !v.is_null()
            && v != &Value::Bool(false)
    });
    let lowering = if exotic {
        Lowering::Unsupported {
            reason: "legacy exotic geometry has no measured equivalent; radial proxy forbidden"
                .into(),
        }
    } else {
        Lowering::Supported{macros:vec![NamedMacro{name:format!("legacy-v{version}-layers"),source_fields:object.keys().cloned().collect(),interpretation:"historical fields retained; unsupported mechanisms are not silently approximated".into()}],primitives:vec![ModuleKind::Chamber,ModuleKind::Sweep,ModuleKind::Junction,ModuleKind::Vane,ModuleKind::Shell]}
    };
    let contract =
        joil_evidence::identity::scientific_hash(&interface).map_err(LegacyError::Wire)?;
    Ok(MigrationRecord {
        source_schema: format!("legacy-v{version}"),
        source_sha256: digest_bytes(bytes),
        interface,
        retained_fields,
        lowering,
        certified_baseline_error_bounds: None,
        zero_mutation_contract_sha256: contract,
    })
}
