//! Test-only private scratch: clean successful scopes and retain panicking scopes.
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
thread_local! {
    static ACTIVE: RefCell<Option<Vec<PathBuf>>> = const { RefCell::new(None) };
}
static SERIAL: AtomicU64 = AtomicU64::new(0);

pub struct TestScratch {
    owner: u32,
}
impl TestScratch {
    pub fn new() -> Self {
        ACTIVE.with(|active| {
            assert!(active.borrow().is_none(), "nested test scratch scope");
            *active.borrow_mut() = Some(Vec::new());
        });
        Self {
            owner: std::process::id(),
        }
    }
    pub fn path(label: &str) -> PathBuf {
        let base = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute() && p.components().any(|c| c.as_os_str() == ".private"))
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join(".private/test-evidence")
            });
        Self::path_in(&base, label)
    }
    pub fn path_in(base: &Path, label: &str) -> PathBuf {
        assert!(base.is_absolute() && base.components().any(|c| c.as_os_str() == ".private"));
        assert!(
            !label.is_empty()
                && label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
        );
        let path = base.join(format!(
            "v5-test-{label}-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        ACTIVE.with(|active| {
            active
                .borrow_mut()
                .as_mut()
                .expect("test scratch requires a scope guard")
                .push(path.clone())
        });
        path
    }
}
impl Drop for TestScratch {
    fn drop(&mut self) {
        // A forked child cannot clean its parent's registered paths.
        if self.owner != std::process::id() {
            return;
        }
        let paths = ACTIVE.with(|active| active.borrow_mut().take().unwrap());
        if std::thread::panicking() {
            for path in paths {
                eprintln!("retained failed-test scratch: {}", path.display());
            }
        } else {
            for path in paths {
                match std::fs::remove_dir_all(&path) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => panic!(
                        "cannot clean successful-test scratch {}: {e}",
                        path.display()
                    ),
                }
            }
        }
    }
}
