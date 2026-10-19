//! Shared process and JSON helpers for the environment and fixture ports.
use crate::ordered_json::Ordered;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub fn sha(raw: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(raw.as_ref()))
}
pub fn obj(fields: Vec<(&str, Ordered)>) -> Ordered {
    Ordered::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
pub fn val(v: impl Into<Value>) -> Ordered {
    Ordered::from_value(v.into())
}
pub fn write(path: impl AsRef<Path>, v: &Ordered) -> Result<()> {
    fs::write(path, v.pretty(0) + "\n")?;
    Ok(())
}
pub fn value(v: &Ordered) -> Result<Value> {
    Ok(serde_json::from_str(&v.encode(false))?)
}
pub fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(message.into())
    }
}
// Resolve existing ancestors too, so symlinks cannot bypass the run/private boundary.
pub fn resolve(path: &Path) -> io::Result<PathBuf> {
    if path.exists() {
        return path.canonicalize();
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let parent = resolve(
        absolute
            .parent()
            .ok_or_else(|| io::Error::other("missing parent"))?,
    )?;
    match absolute.file_name() {
        Some(name) => Ok(parent.join(name)),
        None => Err(io::Error::other("invalid path")),
    }
}
pub fn files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if root.exists() {
        for entry in fs::read_dir(root)? {
            let path = entry?.path();
            if path.is_dir() && !path.is_symlink() {
                out.extend(files(&path)?)
            } else if path.is_file() {
                out.push(path)
            }
        }
    }
    out.sort();
    Ok(out)
}
// Drain both pipes concurrently; the timeout includes descendants holding pipes open.
pub fn capture(command: &mut Command, timeout: Duration) -> Result<Output> {
    use std::{io::Read, os::unix::process::CommandExt, sync::mpsc};
    let mut child = command
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdout = child.stdout.take().ok_or("missing stdout")?;
    let mut stderr = child.stderr.take().ok_or("missing stderr")?;
    let (tx, rx) = mpsc::channel();
    let other = tx.clone();
    let out = std::thread::spawn(move || {
        let mut b = Vec::new();
        let r = stdout.read_to_end(&mut b).map(|_| b);
        let _ = tx.send((true, r));
    });
    let err = std::thread::spawn(move || {
        let mut b = Vec::new();
        let r = stderr.read_to_end(&mut b).map(|_| b);
        let _ = other.send((false, r));
    });
    let start = Instant::now();
    let mut status = None;
    let mut stdout = None;
    let mut stderr = None;
    loop {
        if status.is_none() {
            status = child.try_wait()?;
        }
        while let Ok((is_stdout, result)) = rx.try_recv() {
            if is_stdout {
                stdout = Some(result)
            } else {
                stderr = Some(result)
            }
        }
        if status.is_some() && stdout.is_some() && stderr.is_some() {
            break;
        }
        if start.elapsed() > timeout {
            // SAFETY: the positive child PID is the process group created above;
            // this signals only this invocation and its descendants, no pointers.
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            child.wait()?;
            let _ = out.join();
            let _ = err.join();
            return Err("command timeout".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    out.join().map_err(|_| "stdout reader panicked")?;
    err.join().map_err(|_| "stderr reader panicked")?;
    Ok(Output {
        status: status.ok_or("missing status")?,
        stdout: stdout.ok_or("missing stdout")??,
        stderr: stderr.ok_or("missing stderr")??,
    })
}
