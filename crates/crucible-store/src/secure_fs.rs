//! Linux descriptor-relative access. No payload chooses a filesystem path.
use crate::{Result, StoreError};
use std::{
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, OpenOptionsExt},
    },
    path::{Component, Path, PathBuf},
};

pub fn child(parent: &File, name: &str) -> Result<PathBuf> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', '\0']) {
        return Err(StoreError::Invalid("unsafe_component"));
    }
    Ok(PathBuf::from(format!("/proc/self/fd/{}", parent.as_raw_fd())).join(name))
}
pub fn directory(parent: &File, name: &str, create: bool) -> Result<File> {
    let path = child(parent, name)?;
    if create {
        match fs::DirBuilder::new().mode(0o700).create(&path) {
            Ok(()) => {
                parent.sync_all()?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(path)?)
}
pub fn private_root(path: &Path) -> Result<File> {
    if !path.is_absolute() || !path.components().any(|c| matches!(c, Component::Normal(v) if v == OsStr::new(".private") || v == OsStr::new("runs") || v == OsStr::new("artifacts"))) { return Err(StoreError::Private); }
    let mut current = File::open("/")?;
    for component in path.components() {
        match component {
            Component::RootDir => (),
            Component::Normal(name) => {
                current = directory(
                    &current,
                    name.to_str().ok_or(StoreError::Invalid("non_utf8_path"))?,
                    true,
                )?
            }
            _ => return Err(StoreError::Invalid("unsafe_root")),
        }
    }
    Ok(current)
}
pub fn new_file(parent: &File, name: &str, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(child(parent, name)?)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
pub fn read(parent: &File, name: &str, limit: u64) -> Result<Vec<u8>> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(child(parent, name)?)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > limit {
        return Err(StoreError::Invalid("file_type_or_size"));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(StoreError::Invalid("file_grew_over_limit"));
    }
    Ok(bytes)
}
pub fn lock(parent: &File, name: &str) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(child(parent, name)?)?;
    if !file.metadata()?.is_file() {
        return Err(StoreError::Invalid("lock_type"));
    }
    file.lock()?;
    Ok(file)
}
pub fn append(parent: &File, name: &str, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(child(parent, name)?)?;
    if !file.metadata()?.is_file() {
        return Err(StoreError::Invalid("journal_type"));
    }
    file.write_all(bytes)?;
    file.sync_all()?;
    parent.sync_all()?;
    Ok(())
}
pub fn rename(from: &File, from_name: &str, to: &File, to_name: &str) -> Result<()> {
    fs::rename(child(from, from_name)?, child(to, to_name)?)?;
    to.sync_all()?;
    from.sync_all()?;
    Ok(())
}
pub fn entries(dir: &File) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(format!("/proc/self/fd/{}", dir.as_raw_fd()))? {
        names.push(
            entry?
                .file_name()
                .into_string()
                .map_err(|_| StoreError::Invalid("non_utf8_entry"))?,
        );
    }
    names.sort();
    Ok(names)
}
