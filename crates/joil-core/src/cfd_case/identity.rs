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

/// Build the exact `joil.wave4-case-identity.v1` record from retained artifacts on disk.
///
/// Enforces every path, symlink, canonical-subject, volume-mesh, and solver-input invariant
/// defined in `analysis/case_identity.py` and `crates/joil-evidence/reports/EV12-capture-checklist.md`.
pub fn build_wave4_case_identity(case_dir: &Path) -> Result<Wave4CaseIdentity, String> {
    let prepared_path = safe_case_relpath(case_dir, "input-manifest.json")?;
    let prepared_bytes = fs::read(&prepared_path).map_err(|e| e.to_string())?;
    let prepared_val = joil_evidence::identity::strict_json(&prepared_bytes)?;
    let prepared = prepared_val
        .as_object()
        .ok_or_else(|| "input-manifest.json must contain a JSON object".to_string())?;
    let bundle = prepared
        .get("subject_bundle")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "prepared compiler subject bundle missing".to_string())?;

    let mut artifacts = BTreeMap::new();
    for name in ["design-program.json", "design-ir.json"] {
        let p = safe_case_relpath(case_dir, name)?;
        artifacts.insert(name.to_string(), digest_file(&p)?);
    }
    let surface_rel = prepared
        .get("surface_stl_relpath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "case artifact path must be a nonempty string".to_string())?;
    let surface_path = safe_case_relpath(case_dir, surface_rel)?;
    artifacts.insert(surface_rel.to_string(), digest_file(&surface_path)?);

    for (key, name) in [
        ("genome", "design-program.json"),
        ("ir", "design-ir.json"),
        ("surface_mesh", surface_rel),
    ] {
        let cref = require_content_ref(bundle, key)?;
        if cref.sha256 != artifacts[name].sha256 {
            return Err(format!(
                "prepared {key} digest differs from actual artifact"
            ));
        }
    }
    let prep_stl_sha = prepared
        .get("surface_stl_sha256")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if prep_stl_sha != artifacts[surface_rel].sha256 {
        return Err("prepared surface digest mismatch".into());
    }

    let canonical_genome = require_hash_id(prepared.get("design_sha256"))?;
    let canonical_ir = require_hash_id(prepared.get("design_ir_sha256"))?;

    let prog_path = safe_case_relpath(case_dir, "design-program.json")?;
    let prog_bytes = fs::read(&prog_path).map_err(|e| e.to_string())?;
    let prog_val = joil_evidence::identity::strict_json(&prog_bytes)?;
    if !prog_val.is_object() {
        return Err("design-program.json must contain a JSON object".into());
    }
    let ordered_prog: OrderedJson =
        serde_json::from_slice(&prog_bytes).map_err(|e| e.to_string())?;
    let unsorted_prog_compact = serde_json::to_vec(&ordered_prog).map_err(|e| e.to_string())?;
    let unsorted_prog_hash = joil_ir::sha256_hex(&unsorted_prog_compact);

    let ir_path = safe_case_relpath(case_dir, "design-ir.json")?;
    let ir_bytes = fs::read(&ir_path).map_err(|e| e.to_string())?;
    let ir_val = joil_evidence::identity::strict_json(&ir_bytes)?;
    let ir_obj = ir_val
        .as_object()
        .ok_or_else(|| "design-ir.json must contain a JSON object".to_string())?;

    let ir_source_genome = require_hash_id(ir_obj.get("program_sha256"))?;
    let ir_sha = ir_obj
        .get("ir_sha256")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let sorted_prog_compact = serde_json::to_vec(&prog_val).map_err(|e| e.to_string())?;
    let sorted_prog_hash = joil_ir::sha256_hex(&sorted_prog_compact);
    if ir_sha != canonical_ir
        || canonical_genome != unsorted_prog_hash
        || ir_source_genome != sorted_prog_hash
    {
        return Err("IR and prepared manifest disagree on canonical subjects".into());
    }
    if let Ok(dp) = serde_json::from_slice::<DesignProgram>(&prog_bytes) {
        if dp.canonical_sha256()? != canonical_genome {
            return Err("IR and prepared manifest disagree on canonical subjects".into());
        }
    }

    let mesh = build_inventory(case_dir, &["constant/polyMesh"], &[])?;
    for name in ["points", "faces", "owner", "neighbour", "boundary"] {
        let plain = format!("constant/polyMesh/{name}");
        let gz = format!("constant/polyMesh/{name}.gz");
        if !mesh.contains_key(&plain) && !mesh.contains_key(&gz) {
            return Err(format!("missing executed volume mesh component: {name}"));
        }
    }
    for name in [
        "0/U",
        "0/p",
        "system/controlDict",
        "system/fvSchemes",
        "system/fvSolution",
    ] {
        let p = safe_case_relpath(case_dir, name)?;
        if !p.is_file() {
            return Err(format!("missing solver input: {name}"));
        }
    }
    let inputs = build_inventory(
        case_dir,
        &["0", "constant", "system"],
        &["constant/polyMesh"],
    )?;

    let mesh_hash = joil_evidence::identity::scientific_hash(&serde_json::json!({
        "contract": WAVE4_CASE_IDENTITY_SCHEMA,
        "stage": "volume_mesh",
        "files": &mesh,
    }))?;

    let genome_hash = artifacts["design-program.json"].sha256.clone();
    let ir_hash = artifacts["design-ir.json"].sha256.clone();
    let geometry_hash = artifacts[surface_rel].sha256.clone();

    let mut compiler = BTreeMap::new();
    for key in ["compiler", "geometry_kernel", "compiler_config"] {
        compiler.insert(key.to_string(), require_content_ref(bundle, key)?);
    }

    let case_hash = joil_evidence::identity::scientific_hash(&serde_json::json!({
        "contract": WAVE4_CASE_IDENTITY_SCHEMA,
        "stage": "solver_case",
        "upstream": {
            "genome_hash": &genome_hash,
            "ir_hash": &ir_hash,
            "geometry_hash": &geometry_hash,
            "mesh_hash": &mesh_hash,
        },
        "compiler": &compiler,
        "files": &inputs,
    }))?;

    Ok(Wave4CaseIdentity {
        canonical_genome_hash: canonical_genome,
        canonical_ir_hash: canonical_ir,
        canonical_ir_source_genome_hash: ir_source_genome,
        case_hash,
        compiler,
        genome_hash,
        geometry_hash,
        hash_basis: "genome/IR/geometry exact file bytes; mesh/case domain-separated inventories"
            .to_string(),
        ir_hash,
        kind: "openfoam_artifact_identity_not_qualification".to_string(),
        mesh_files: mesh,
        mesh_hash,
        prepared_manifest: digest_file(&prepared_path)?,
        schema: WAVE4_CASE_IDENTITY_SCHEMA.to_string(),
        solver_input_files: inputs,
        source_artifacts: artifacts,
    })
}

pub(crate) fn run_ev_python_case_identity(
    case_dir: &Path,
    finalize_first: bool,
) -> Result<(Wave4CaseIdentity, Wave4CaseVerification), String> {
    let canon_case = check_no_symlink_case_root(case_dir)?;
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let script = if finalize_first {
        "import json, sys\n\
from analysis.case_identity import finalize_case_identity, verify_case_identity\n\
rec = finalize_case_identity(sys.argv[1])\n\
ver = verify_case_identity(sys.argv[1])\n\
print(json.dumps({'record': rec, 'verification': ver}))\n"
    } else {
        "import json, pathlib, sys\n\
from analysis.case_identity import MANIFEST, verify_case_identity\n\
from analysis.evidence import strict_json\n\
case = pathlib.Path(sys.argv[1])\n\
ver = verify_case_identity(case)\n\
rec = strict_json((case / MANIFEST).read_bytes())\n\
print(json.dumps({'record': rec, 'verification': ver}))\n"
    };
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&canon_case)
        .current_dir(&repo_root)
        .env("PYTHONPATH", &repo_root)
        .output()
        .map_err(|e| format!("failed to invoke EV case_identity Python API: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "EV analysis.case_identity failed ({}): {}{}",
            output.status, stderr, stdout
        ));
    }
    let payload: serde_json::Value = joil_evidence::identity::strict_json(&output.stdout)?;
    let rec: Wave4CaseIdentity = serde_json::from_value(payload["record"].clone())
        .map_err(|e| format!("decode EV Wave4CaseIdentity: {e}"))?;
    let ver: Wave4CaseVerification = serde_json::from_value(payload["verification"].clone())
        .map_err(|e| format!("decode EV Wave4CaseVerification: {e}"))?;
    Ok((rec, ver))
}

/// Finalize `case-identity.json` (`joil.wave4-case-identity.v1`) after the last
/// `mesh`/`0/`/`dictionary`/`RSM` edit and before solver execution or cleanup,
/// using EV's `analysis.case_identity.finalize_case_identity` and `verify_case_identity` API
/// (`crates/joil-evidence/reports/EV12-capture-checklist.md`) and cross-verifying in Rust.
pub fn finalize_wave4_case_identity(case_dir: &Path) -> Result<Wave4CaseIdentity, String> {
    let rust_expected = build_wave4_case_identity(case_dir)?;
    let sidecar_path = safe_case_relpath(case_dir, WAVE4_CASE_IDENTITY_FILE)?;
    if sidecar_path.exists() {
        let existing_bytes = fs::read(&sidecar_path).map_err(|e| e.to_string())?;
        let existing: Wave4CaseIdentity = joil_evidence::identity::strict_decode(&existing_bytes)?;
        if existing != rust_expected {
            return Err("existing case identity is stale; never overwrite it".into());
        }
    }
    let (py_record, py_ver) = run_ev_python_case_identity(case_dir, true)?;
    if py_record != rust_expected {
        return Err("Rust and EV Python joil.wave4-case-identity.v1 records diverged".into());
    }
    if py_ver.state != "exact_artifact_bytes_verified"
        || py_ver.case_hash != rust_expected.case_hash
        || py_ver.mesh_hash != rust_expected.mesh_hash
    {
        return Err("EV verify_case_identity did not confirm exact_artifact_bytes_verified".into());
    }
    Ok(py_record)
}

/// Verify an existing `case-identity.json` (`joil.wave4-case-identity.v1`) against
/// retained case artifacts using both Rust and EV's `analysis.case_identity.verify_case_identity`.
pub fn verify_wave4_case_identity(case_dir: &Path) -> Result<Wave4CaseVerification, String> {
    let sidecar_path = safe_case_relpath(case_dir, WAVE4_CASE_IDENTITY_FILE)?;
    if !sidecar_path.is_file() {
        return Err("missing case-identity.json sidecar".into());
    }
    let recorded_bytes = fs::read(&sidecar_path).map_err(|e| e.to_string())?;
    let recorded: Wave4CaseIdentity = joil_evidence::identity::strict_decode(&recorded_bytes)?;
    let actual = build_wave4_case_identity(case_dir)?;
    if recorded != actual {
        return Err("case identity differs from retained artifacts".into());
    }
    let (py_record, py_ver) = run_ev_python_case_identity(case_dir, false)?;
    if py_record != actual || py_ver.state != "exact_artifact_bytes_verified" {
        return Err("EV verify_case_identity failed".into());
    }
    Ok(py_ver)
}

pub(crate) fn collect_archived_case_inventory(
    case_dir: &Path,
    identity: &Wave4CaseIdentity,
) -> Result<BTreeMap<String, FileDigestEntry>, String> {
    let sidecar_digest = digest_file(&safe_case_relpath(case_dir, WAVE4_CASE_IDENTITY_FILE)?)?;
    let mut all_files: BTreeMap<String, FileDigestEntry> = BTreeMap::new();
    all_files.insert(
        "input-manifest.json".to_string(),
        identity.prepared_manifest.clone(),
    );
    all_files.insert(WAVE4_CASE_IDENTITY_FILE.to_string(), sidecar_digest);
    for inv in [
        &identity.source_artifacts,
        &identity.mesh_files,
        &identity.solver_input_files,
    ] {
        for (rel, dig) in inv {
            if let Some(prev) = all_files.insert(rel.clone(), dig.clone()) {
                if prev != *dig {
                    return Err(format!("conflicting inventory digest for {rel}"));
                }
            }
        }
    }
    Ok(all_files)
}

/// Archive all indexed case inputs (`input-manifest.json`, `case-identity.json`,
/// `source_artifacts`, `mesh_files`, `solver_input_files`) to a separate directory and
/// run `verify_wave4_case_identity` on the restored copy per `EV12-capture-checklist.md`.
pub fn archive_and_verify_wave4_case_inputs(
    case_dir: &Path,
    archive_dir: &Path,
) -> Result<Wave4ArchiveVerificationReceipt, String> {
    let canon_case = check_no_symlink_case_root(case_dir)?;
    if let Ok(meta) = fs::symlink_metadata(archive_dir) {
        if meta.file_type().is_symlink() {
            return Err("archive_dir cannot be a symlink".into());
        }
        return Err(format!(
            "archive_dir already exists; refusing to overwrite: {}",
            archive_dir.display()
        ));
    }
    let _ = verify_wave4_case_identity(case_dir)?;
    let identity = build_wave4_case_identity(case_dir)?;
    let all_files = collect_archived_case_inventory(case_dir, &identity)?;

    fs::create_dir_all(archive_dir).map_err(|e| e.to_string())?;
    let canon_arch = check_no_symlink_case_root(archive_dir)?;
    if canon_arch.starts_with(&canon_case) || canon_case.starts_with(&canon_arch) {
        let _ = fs::remove_dir_all(archive_dir);
        return Err("archive_dir must be disjoint from case_dir".into());
    }

    for (rel, expected_dig) in &all_files {
        let src = safe_case_relpath(case_dir, rel)?;
        let dst = archive_dir.join(rel);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(&src, &dst).map_err(|e| e.to_string())?;
        let copied_dig = digest_file(&dst)?;
        if copied_dig != *expected_dig {
            return Err(format!("archived file digest mismatch for {rel}"));
        }
    }

    let restored_ver = verify_wave4_case_identity(archive_dir)?;
    let source_ver = verify_wave4_case_identity(case_dir)?;
    if restored_ver.case_hash != source_ver.case_hash
        || restored_ver.mesh_hash != source_ver.mesh_hash
    {
        return Err("restored archive verification hashes differ from source case".into());
    }

    let total_bytes: u64 = all_files.values().map(|d| d.bytes).sum();
    let receipt = Wave4ArchiveVerificationReceipt {
        schema: "joil.wave4-case-archive-verification.v1".to_string(),
        source_case: case_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("case")
            .to_string(),
        archive_dir: archive_dir.display().to_string(),
        case_hash: identity.case_hash,
        mesh_hash: identity.mesh_hash,
        file_count: all_files.len(),
        total_bytes,
        restored_verification: restored_ver,
    };
    let receipt_json = serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?;
    fs::write(
        archive_dir.join("ev12-archive-verification-receipt.json"),
        format!("{receipt_json}\n"),
    )
    .map_err(|e| e.to_string())?;
    Ok(receipt)
}

/// Prune `constant/polyMesh` from `case_dir` ONLY after verifying that:
/// 1. `case_dir` has a valid, non-stale `case-identity.json` (`verify_wave4_case_identity(case_dir)`), and
/// 2. `archive_dir` is a disjoint directory whose restored copy passes `verify_wave4_case_identity(archive_dir)`
///    with identical `Wave4CaseIdentity` (`case_hash`, `mesh_hash`, and all file inventories).
///
/// Refuses to delete `constant/polyMesh` if either verification fails.
pub fn prune_polymesh_after_verified_archive(
    case_dir: &Path,
    archive_dir: &Path,
) -> Result<Wave4ArchiveVerificationReceipt, String> {
    let canon_case = check_no_symlink_case_root(case_dir)?;
    let canon_arch = check_no_symlink_case_root(archive_dir)
        .map_err(|e| format!("refusing to prune polyMesh without verified archive_dir: {e}"))?;
    if canon_arch.starts_with(&canon_case) || canon_case.starts_with(&canon_arch) {
        return Err(
            "refusing to prune polyMesh: archive_dir must be disjoint from case_dir".into(),
        );
    }

    let case_ver = verify_wave4_case_identity(case_dir)
        .map_err(|e| format!("refusing to prune polyMesh: source case unverified: {e}"))?;
    let arch_ver = verify_wave4_case_identity(archive_dir)
        .map_err(|e| format!("refusing to prune polyMesh: archive copy unverified: {e}"))?;

    let case_id = build_wave4_case_identity(case_dir)?;
    let arch_id = build_wave4_case_identity(archive_dir)?;
    if case_id != arch_id
        || case_ver.case_hash != arch_ver.case_hash
        || case_ver.mesh_hash != arch_ver.mesh_hash
    {
        return Err(
            "refusing to prune polyMesh: archive Wave4CaseIdentity does not match source case"
                .into(),
        );
    }

    let all_files = collect_archived_case_inventory(archive_dir, &arch_id)?;
    let file_count = all_files.len();
    let total_bytes: u64 = all_files.values().map(|d| d.bytes).sum();

    let poly_mesh_dir = safe_case_relpath(case_dir, "constant/polyMesh")?;
    fs::remove_dir_all(&poly_mesh_dir).map_err(|e| e.to_string())?;

    Ok(Wave4ArchiveVerificationReceipt {
        schema: "joil.wave4-case-archive-verification.v1".to_string(),
        source_case: case_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("case")
            .to_string(),
        archive_dir: archive_dir.display().to_string(),
        case_hash: arch_id.case_hash,
        mesh_hash: arch_id.mesh_hash,
        file_count,
        total_bytes,
        restored_verification: arch_ver,
    })
}
