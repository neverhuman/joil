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
        "import sys,json; from analysis.evidence import strict_json,scientific_hash,ingest_observation; from tools.gen_status import objective_contract,ROOT; p=strict_json(sys.stdin.buffer.read()); obs=ingest_observation(json.dumps(p['observation']).encode()); assert scientific_hash(obs)==p['hash']; assert objective_contract(ROOT)[2]==p['objective_hash']; print(json.dumps(obs,allow_nan=False))"
    ).current_dir(&root).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&payload).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        joil_evidence::observation::ingest_observation(&output.stdout).unwrap(),
        observation
    );
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

#[test]
fn rust_python_agree_on_observation_mutants() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original = serde_json::to_value(Observation::Measured {
        measurement: Box::new(common::measurement()),
    })
    .unwrap();
    let mutations = [
        ("/measurement/unit", serde_json::json!("unknown-unit")),
        ("/measurement/value", serde_json::json!(null)),
        ("/measurement/state/execution", serde_json::json!("failed")),
        (
            "/measurement/state/validation",
            serde_json::json!("validated"),
        ),
        ("/measurement/origin", serde_json::json!("synthetic")),
        ("/measurement/uncertainty", serde_json::json!([])),
        ("/measurement/uncertainty/0/absolute", serde_json::json!(-1)),
        (
            "/measurement/uncertainty/0/unit",
            serde_json::json!("pascal"),
        ),
        ("/measurement/evidence", serde_json::json!([])),
        ("/measurement/identity/schema", serde_json::json!(2.0)),
        ("/measurement/tier", serde_json::json!("qualified")),
    ];
    let mut wires = vec![serde_json::to_string(&original).unwrap()];
    for (path, new_value) in mutations {
        let mut mutant = original.clone();
        *mutant.pointer_mut(path).unwrap() = new_value;
        wires.push(serde_json::to_string(&mutant).unwrap());
    }
    wires.extend([
        r#"{"status":"unknown","reason":"missing"}"#.into(),
        r#"{"status":"failed","class":"numerical","reason":"diverged","diagnostics":[]}"#.into(),
        r#"{"status":"unknown","reason":"missing","value":0}"#.into(),
    ]);
    let expected: Vec<_> = wires
        .iter()
        .map(|raw| joil_evidence::observation::ingest_observation(raw.as_bytes()).is_ok())
        .collect();
    assert!(expected[0]);
    assert!(expected[1..12].iter().all(|pass| !pass));
    let mut child = Command::new("python3")
        .args([
            "-c",
            r#"
import json,sys
from analysis.evidence import ingest_observation
result=[]
for raw in json.load(sys.stdin):
    try: ingest_observation(raw); result.append(True)
    except ValueError: result.append(False)
print(json.dumps(result))
"#,
        ])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&wires).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let actual: Vec<bool> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual, expected);
}
