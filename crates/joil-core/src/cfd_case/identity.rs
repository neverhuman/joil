//! Wave-4 EV-12 case identity sidecar builder, verifier, archive verifier, and polyMesh pruning.

use super::*;

/// Frozen EV-12 Wave-4 OpenFOAM case identity sidecar schema (`crates/joil-evidence/reports/EV12-capture-checklist.md`).
pub const WAVE4_CASE_IDENTITY_SCHEMA: &str = "joil.wave4-case-identity.v1";

/// Sidecar filename written at the root of each finalized OpenFOAM case directory.
pub const WAVE4_CASE_IDENTITY_FILE: &str = "case-identity.json";

/// Exact file digest entry (`{bytes, sha256}`) used in `joil.wave4-case-identity.v1` inventories.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileDigestEntry {
    pub bytes: u64,
    pub sha256: String,
}

/// Complete `joil.wave4-case-identity.v1` sidecar record (`case-identity.json`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave4CaseIdentity {
    pub canonical_genome_hash: String,
    pub canonical_ir_hash: String,
    pub canonical_ir_source_genome_hash: String,
    pub case_hash: String,
    pub compiler: BTreeMap<String, joil_evidence::identity::ContentRef>,
    pub genome_hash: String,
    pub geometry_hash: String,
    pub hash_basis: String,
    pub ir_hash: String,
    pub kind: String,
    pub mesh_files: BTreeMap<String, FileDigestEntry>,
    pub mesh_hash: String,
    pub prepared_manifest: FileDigestEntry,
    pub schema: String,
    pub solver_input_files: BTreeMap<String, FileDigestEntry>,
    pub source_artifacts: BTreeMap<String, FileDigestEntry>,
}

/// Verification result returned by `verify_wave4_case_identity` (`analysis.case_identity.verify_case_identity`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave4CaseVerification {
    pub case: String,
    pub case_hash: String,
    pub mesh_hash: String,
    pub state: String,
}

/// Receipt proving that all indexed inputs + `case-identity.json` were archived to a separate
/// directory and verified via `verify_wave4_case_identity` prior to any `constant/polyMesh` pruning.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave4ArchiveVerificationReceipt {
    pub schema: String,
    pub source_case: String,
    pub archive_dir: String,
    pub case_hash: String,
    pub mesh_hash: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub restored_verification: Wave4CaseVerification,
}

pub(crate) fn check_no_symlink_case_root(case_dir: &Path) -> Result<std::path::PathBuf, String> {
    let meta = fs::symlink_metadata(case_dir).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err("symlink is not an exact case artifact".into());
    }
    if !meta.is_dir() {
        return Err(format!(
            "case directory does not exist: {}",
            case_dir.display()
        ));
    }
    case_dir.canonicalize().map_err(|e| e.to_string())
}

pub(crate) fn safe_case_relpath(
    case_dir: &Path,
    relative: &str,
) -> Result<std::path::PathBuf, String> {
    if relative.is_empty() {
        return Err("case artifact path must be a nonempty string".into());
    }
    let rel_path = Path::new(relative);
    if rel_path.is_absolute() {
        return Err("case artifact path escapes case".into());
    }
    for comp in rel_path.components() {
        match comp {
            std::path::Component::Normal(_) => {}
            _ => return Err("case artifact path escapes case".into()),
        }
    }
    let _canon_case = check_no_symlink_case_root(case_dir)?;
    let mut current = case_dir.to_path_buf();
    for comp in rel_path.components() {
        current.push(comp);
        if let Ok(meta) = fs::symlink_metadata(&current) {
            if meta.file_type().is_symlink() {
                return Err("symlink is not an exact case artifact".into());
            }
        }
    }
    Ok(current)
}

pub(crate) fn digest_file(path: &Path) -> Result<FileDigestEntry, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err("symlink is not an exact case artifact".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    Ok(FileDigestEntry {
        bytes: bytes.len() as u64,
        sha256: joil_ir::sha256_hex(&bytes),
    })
}

pub(crate) fn collect_inventory_recursive(
    case_dir: &Path,
    cur_dir: &Path,
    exclude: &[&str],
    out: &mut BTreeMap<String, FileDigestEntry>,
) -> Result<(), String> {
    let mut entries: Vec<_> = fs::read_dir(cur_dir)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let rel = path
            .strip_prefix(case_dir)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if exclude
            .iter()
            .any(|ex| rel == *ex || rel.starts_with(&format!("{ex}/")))
        {
            continue;
        }
        safe_case_relpath(case_dir, &rel)?;
        let ftype = fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type();
        if ftype.is_symlink() {
            return Err("symlink is not an exact case artifact".into());
        }
        if ftype.is_dir() {
            collect_inventory_recursive(case_dir, &path, exclude, out)?;
        } else if ftype.is_file() {
            out.insert(rel, digest_file(&path)?);
        }
    }
    Ok(())
}

pub(crate) fn build_inventory(
    case_dir: &Path,
    roots: &[&str],
    exclude: &[&str],
) -> Result<BTreeMap<String, FileDigestEntry>, String> {
    let mut found = BTreeMap::new();
    for root in roots {
        let dir = safe_case_relpath(case_dir, root)?;
        let Ok(meta) = fs::symlink_metadata(&dir) else {
            return Err(format!("missing artifact directory: {root}"));
        };
        if meta.file_type().is_symlink() {
            return Err("symlink is not an exact case artifact".into());
        }
        if !meta.is_dir() {
            return Err(format!("missing artifact directory: {root}"));
        }
        collect_inventory_recursive(case_dir, &dir, exclude, &mut found)?;
    }
    if found.is_empty() {
        return Err("empty artifact inventory".into());
    }
    Ok(found)
}

pub(crate) fn require_hash_id(val: Option<&serde_json::Value>) -> Result<String, String> {
    let s = val
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing or invalid canonical source hash".to_string())?;
    if !joil_evidence::identity::is_hash(s) {
        return Err("missing or invalid canonical source hash".into());
    }
    Ok(s.to_string())
}

pub(crate) fn require_content_ref(
    bundle: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<joil_evidence::identity::ContentRef, String> {
    let v = bundle
        .get(key)
        .ok_or_else(|| format!("missing or invalid prepared {key} reference"))?;
    let cref: joil_evidence::identity::ContentRef = serde_json::from_value(v.clone())
        .map_err(|_| format!("missing or invalid prepared {key} reference"))?;
    cref.check()
        .map_err(|_| format!("missing or invalid prepared {key} reference"))?;
    Ok(cref)
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum OrderedJson {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<OrderedJson>),
    Object(Vec<(String, OrderedJson)>),
}

impl Serialize for OrderedJson {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Number(n) => n.serialize(serializer),
            Self::String(s) => serializer.serialize_str(s),
            Self::Array(arr) => serializer.collect_seq(arr),
            Self::Object(entries) => serializer.collect_map(entries.iter().map(|(k, v)| (k, v))),
        }
    }
}

impl<'de> Deserialize<'de> for OrderedJson {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct OrderedVisitor;
        impl<'de> serde::de::Visitor<'de> for OrderedVisitor {
            type Value = OrderedJson;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("any valid JSON value")
            }
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
                Ok(OrderedJson::Bool(v))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                Ok(OrderedJson::Number(v.into()))
            }
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(OrderedJson::Number(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(OrderedJson::Number)
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
                Ok(OrderedJson::String(v.to_owned()))
            }
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
                Ok(OrderedJson::String(v))
            }
            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(OrderedJson::Null)
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(OrderedJson::Null)
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut vec = Vec::new();
                while let Some(elem) = seq.next_element()? {
                    vec.push(elem);
                }
                Ok(OrderedJson::Array(vec))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some((k, v)) = map.next_entry()? {
                    entries.push((k, v));
                }
                Ok(OrderedJson::Object(entries))
            }
        }
        deserializer.deserialize_any(OrderedVisitor)
    }
}

#[path = "identity_builder.rs"]
pub mod builder;
pub use builder::*;
