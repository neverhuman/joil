mod common;
use joil_evidence::{campaign::Requirements, identity::scientific_hash, observation::Observation};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn rust_python_exact_wire_and_objectives_roundtrip() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let requirements = Requirements::from_toml(include_bytes!(
        "../../../requirements/joil-hydrocyclone-v1.toml"
    ))
    .unwrap();
    let observation = Observation::Measured {
        measurement: Box::new(common::measurement()),
    };
    let payload = serde_json::json!({"observation": observation, "hash": scientific_hash(&observation).unwrap(),
        "objective_hash": requirements.objective_hash().unwrap()});
    let mut child = Command::new("python3").arg("-c").arg(
        "import sys; from analysis.evidence import strict_json,scientific_hash; from tools.gen_status import objective_contract,ROOT; p=strict_json(sys.stdin.buffer.read()); assert scientific_hash(p['observation'])==p['hash']; assert objective_contract(ROOT)[2]==p['objective_hash']"
    ).current_dir(&root).stdin(Stdio::piped()).spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&payload).unwrap())
        .unwrap();
    assert!(child.wait().unwrap().success());
}

#[test]
fn python_contract_negative_controls() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status = Command::new("python3")
        .args([
            "-m",
            "unittest",
            "discover",
            "-s",
            "crates/joil-evidence/tests",
            "-p",
            "test_python_contract.py",
            "-v",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
}
