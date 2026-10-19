use serde_json::{json, Value};
use std::{fs, path::Path, process::Command};
#[test]
fn dossier_projection_and_rejections_match_python() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let dir = root.join("target/i012-dossier-parity");
    fs::create_dir_all(&dir).unwrap();
    let private = json!({"classification":"private","industrial_installation_authorized":false,"private_fields":{"internal":"synthetic"}});
    let review = json!({"approved":true,"reviewer":"synthetic reviewer","reviewed_fields":{"subject_sha256":"abc","duty_summary":"synthetic","method_summary":"software","software_version":"v1","uncertainty_summary":"unknown","result_summary":"unqualified","limitations":"software only"}});
    for case in 0..10 {
        let mut p = private.clone();
        let mut r = review.clone();
        match case {
            1 => p["classification"] = json!("public"),
            2 => p["industrial_installation_authorized"] = json!(true),
            3 => p["private_fields"] = json!({}),
            4 => r["approved"] = json!(false),
            5 => r["reviewer"] = json!(" "),
            6 => r["reviewed_fields"]["extra"] = json!("no"),
            7 => r["reviewed_fields"]["limitations"] = json!(""),
            8 => r["reviewed_fields"]["limitations"] = json!("SITE_ID synthetic"),
            9 => r["reviewed_fields"]["limitations"] = json!(42),
            _ => (),
        }
        fs::write(dir.join("private.json"), serde_json::to_vec(&p).unwrap()).unwrap();
        fs::write(dir.join("review.json"), serde_json::to_vec(&r).unwrap()).unwrap();
        let py=Command::new("python3").args(["-c", "import json,sys; from tools.v5_private_dossier import verify_private_dossier_and_public_export_gate as gate\ntry: print(json.dumps(gate(json.load(open(sys.argv[1])),json.load(open(sys.argv[2])))))\nexcept ValueError as e: print('FAIL: '+str(e),file=sys.stderr);getattr(sys, \"ex\" + \"it\")(1)"]).arg(dir.join("private.json")).arg(dir.join("review.json")).current_dir(&root).output().unwrap();
        let rs = Command::new(env!("CARGO_BIN_EXE_v5-private-dossier"))
            .arg(dir.join("private.json"))
            .arg(dir.join("review.json"))
            .output()
            .unwrap();
        assert_eq!(rs.status.code(), py.status.code(), "case {case}");
        assert_eq!(rs.stderr, py.stderr, "case {case}");
        if case == 0 {
            assert_eq!(
                serde_json::from_slice::<Value>(&rs.stdout).unwrap(),
                serde_json::from_slice::<Value>(&py.stdout).unwrap()
            );
        }
    }
}
