//! Lossless, manifest-bound checkpoints written atomically after every generation.
//!
//! JSON numbers use shortest round-trip formatting, so every `f64` (genes, scores, RNG
//! state as integers) restores exactly. A torn write can never replace the last good
//! checkpoint: data goes to a temporary file, is synced, and is renamed into place; the
//! previous checkpoint is kept as `checkpoint.prev.json`.

use crate::campaign::{Campaign, Manifest};
use crate::evolve::RunState;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const FORMAT: &str = "joil-forge-checkpoint/1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub format: String,
    pub manifest: Manifest,
    pub campaign: Campaign,
    pub state: RunState,
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    {
        let mut f =
            std::fs::File::create(&tmp).map_err(|e| format!("create {}: {e}", tmp.display()))?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| format!("write {}: {e}", tmp.display()))?;
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("rename {}: {e}", path.display()))
}

pub fn checkpoint_path(run: &Path) -> PathBuf {
    run.join("checkpoint.json")
}

pub fn save(run: &Path, cp: &Checkpoint) -> Result<(), String> {
    let path = checkpoint_path(run);
    if path.exists() {
        let prev = run.join("checkpoint.prev.json");
        std::fs::copy(&path, &prev).map_err(|e| format!("keep previous checkpoint: {e}"))?;
    }
    let bytes = serde_json::to_vec(cp).map_err(|e| format!("serialise checkpoint: {e}"))?;
    write_atomic(&path, &bytes)
}

pub fn load(path: &Path) -> Result<Checkpoint, String> {
    let file = if path.is_dir() {
        checkpoint_path(path)
    } else {
        path.to_path_buf()
    };
    let bytes = std::fs::read(&file).map_err(|e| format!("read {}: {e}", file.display()))?;
    let cp: Checkpoint =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse {}: {e}", file.display()))?;
    if cp.format != FORMAT {
        return Err(format!("unsupported checkpoint format {}", cp.format));
    }
    if cp.manifest.campaign_sha256 != cp.campaign.sha256() {
        return Err("checkpoint campaign does not match its manifest digest".into());
    }
    Ok(cp)
}
