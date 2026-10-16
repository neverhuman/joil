//! Read/write only explicitly supplied EV-private synthetic fixtures in review.
use joil_core::cfd_case;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let case = Path::new(&args[2]);
    let result: Result<serde_json::Value, String> = match args[1].as_str() {
        "prepare" => cfd_case::build_openfoam_candidate_case(
            &joil_core::program::DesignProgram::baseline_ct35(),
            case,
            &cfd_case::CfdCaseConfig::default(),
        )
        .and_then(|x| serde_json::to_value(x).map_err(|e| e.to_string())),
        "build" => cfd_case::build_wave4_case_identity(case)
            .and_then(|x| serde_json::to_value(x).map_err(|e| e.to_string())),
        "finalize" => cfd_case::finalize_wave4_case_identity(case)
            .and_then(|x| serde_json::to_value(x).map_err(|e| e.to_string())),
        "verify" => cfd_case::verify_wave4_case_identity(case)
            .and_then(|x| serde_json::to_value(x).map_err(|e| e.to_string())),
        "archive" => cfd_case::archive_and_verify_wave4_case_inputs(case, Path::new(&args[3]))
            .and_then(|x| serde_json::to_value(x).map_err(|e| e.to_string())),
        "prune" => cfd_case::prune_polymesh_after_verified_archive(case, Path::new(&args[3]))
            .and_then(|x| serde_json::to_value(x).map_err(|e| e.to_string())),
        _ => Err("unknown review action".into()),
    };
    match result {
        Ok(value) => println!("{}", serde_json::json!({"ok": true, "value": value})),
        Err(reason) => {
            println!("{}", serde_json::json!({"ok": false, "reason": reason}));
            std::process::exit(1);
        }
    }
}
