//! Typed, fenced worker messages. Transport authentication is supplied by the
//! coordinator connection; these payloads never contain shell command strings.
use crate::{lease::AdmittedLease, BlobDigest, Result, RuntimeError};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, PathBuf},
};

pub const WORKER_SCHEMA: u32 = 1;
pub const MAX_ARGUMENTS: usize = 64;
pub const MAX_ARGUMENT_BYTES: usize = 4096;
pub const MAX_ARTIFACTS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    RunSolver,
    ReadArtifact,
    WriteArtifact,
    CancelOwnedAttempt,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub operation: Operation,
    pub solver: Option<String>,
    pub task_root: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerRequest {
    pub schema_version: u32,
    pub attempt_id: String,
    pub fence: u64,
    pub lease_id: String,
    pub operation: Operation,
    pub relative_paths: Vec<PathBuf>,
    pub arguments: Vec<String>,
}

/// Implemented only by the authenticated coordinator channel adapter.
pub trait AuthenticatedPeer {
    fn peer_identity(&self) -> &BlobDigest;
}

#[derive(Clone, Copy)]
pub struct ValidationContext<'a> {
    pub trusted_coordinator: &'a BlobDigest,
    pub attempt_id: &'a str,
    pub fence: u64,
    pub lease: &'a AdmittedLease,
    pub now_ms: u64,
    pub capability: &'a Capability,
}

pub fn validate_request<P: AuthenticatedPeer>(
    peer: &P,
    req: &WorkerRequest,
    context: &ValidationContext<'_>,
) -> Result<Vec<PathBuf>> {
    if peer.peer_identity() != context.trusted_coordinator {
        return Err(RuntimeError::Invalid("unauthenticated_coordinator"));
    }
    if req.schema_version != WORKER_SCHEMA {
        return Err(RuntimeError::Unsupported("worker_schema"));
    }
    if req.attempt_id != context.attempt_id
        || req.fence != context.fence
        || req.lease_id != context.lease.record().id
    {
        return Err(RuntimeError::StaleFence);
    }
    if context.now_ms >= context.lease.record().expires_ms {
        return Err(RuntimeError::ExpiredLease);
    }
    if req.operation != context.capability.operation || req.relative_paths.len() > MAX_ARTIFACTS {
        return Err(RuntimeError::Invalid("worker_capability_or_artifact_count"));
    }
    if req.arguments.len() > MAX_ARGUMENTS
        || req
            .arguments
            .iter()
            .any(|a| a.len() > MAX_ARGUMENT_BYTES || a.contains('\0'))
    {
        return Err(RuntimeError::Invalid("bounded_process_arguments"));
    }
    if req.operation != Operation::RunSolver && !req.arguments.is_empty() {
        return Err(RuntimeError::Invalid(
            "arguments_only_for_registered_solver",
        ));
    }
    if req.operation == Operation::RunSolver
        && context
            .capability
            .solver
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return Err(RuntimeError::Invalid("solver_not_registered"));
    }
    let root = context.capability.task_root.canonicalize()?;
    if !root.is_dir() {
        return Err(RuntimeError::Invalid("task_root_not_directory"));
    }
    let mut paths = Vec::with_capacity(req.relative_paths.len());
    for rel in &req.relative_paths {
        if rel.as_os_str().is_empty()
            || rel.is_absolute()
            || rel.components().any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(RuntimeError::Invalid("untrusted_artifact_path"));
        }
        let full = root.join(rel);
        let parent = full
            .parent()
            .ok_or(RuntimeError::Invalid("artifact_parent"))?
            .canonicalize()?;
        if !parent.starts_with(&root) {
            return Err(RuntimeError::Invalid("artifact_path_escape"));
        }
        if full.exists()
            && full
                .canonicalize()?
                .parent()
                .is_none_or(|p| !p.starts_with(&root))
        {
            return Err(RuntimeError::Invalid("artifact_symlink_escape"));
        }
        paths.push(full);
    }
    Ok(paths)
}

/// Launch arguments are passed as argv to the operator-registered executable;
/// no shell or campaign-provided executable is accepted here.
pub fn bounded_argv(args: &[String]) -> Result<Vec<std::ffi::OsString>> {
    if args.len() > MAX_ARGUMENTS
        || args
            .iter()
            .any(|a| a.len() > MAX_ARGUMENT_BYTES || a.contains('\0'))
    {
        return Err(RuntimeError::Invalid("bounded_process_arguments"));
    }
    Ok(args.iter().map(Into::into).collect())
}

pub fn allowed_operations(caps: &[Capability]) -> BTreeSet<Operation> {
    caps.iter().map(|c| c.operation).collect()
}
