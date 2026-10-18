//! Transitional CLI parity against the unchanged legacy implementation.
use std::{path::Path, process::Command};

#[test]
fn publication_cli_matches_legacy_gate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let legacy = Command::new("python3")
        .arg("tools/check_public.py")
        .current_dir(&root)
        .output()
        .unwrap();
    let rust = Command::new(env!("CARGO_BIN_EXE_check-public"))
        .current_dir(root)
        .output()
        .unwrap();
    assert!(legacy.status.success(), "legacy gate failed: {legacy:?}");
    assert_eq!(rust.status.code(), legacy.status.code());
    assert_eq!(rust.stdout, legacy.stdout);
    assert_eq!(rust.stderr, legacy.stderr);
}
