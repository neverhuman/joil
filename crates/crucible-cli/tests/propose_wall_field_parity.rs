mod mesh_support;
use mesh_support::*;
use std::{fs, path::Path, process::Command};
#[test]
fn exact_proposals_and_exclusive_output() {
    let root = setup();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (seed, count) in [
        ("20261015", "8"),
        ("0", "0"),
        ("-42", "11"),
        ("4294967296", "5"),
        ("1234567890123456789012345678901234567890", "20"),
        ("17", "-1"),
        ("1234567", "256"),
    ] {
        let py = root.join(format!("py-{seed}-{count}.json"));
        let rs = root.join(format!("rs-{seed}-{count}.json"));
        let a = Command::new("python3")
            .arg(repo.join("cad/program/propose.py"))
            .args(["--seed", seed, "--count", count, "--out"])
            .arg(&py)
            .output()
            .unwrap();
        let b = Command::new(env!("CARGO_BIN_EXE_propose-wall-field"))
            .args(["--seed", seed, "--count", count, "--out"])
            .arg(&rs)
            .output()
            .unwrap();
        assert!(a.status.success(), "{}", String::from_utf8_lossy(&a.stderr));
        assert!(b.status.success(), "{}", String::from_utf8_lossy(&b.stderr));
        assert_eq!(
            fs::read(&py).unwrap(),
            fs::read(&rs).unwrap(),
            "seed {seed}"
        );
        let before = fs::read(&rs).unwrap();
        assert!(!Command::new(env!("CARGO_BIN_EXE_propose-wall-field"))
            .arg("--out")
            .arg(&rs)
            .status()
            .unwrap()
            .success());
        assert_eq!(before, fs::read(rs).unwrap());
    }
}
