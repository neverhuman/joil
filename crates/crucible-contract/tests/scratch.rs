#[path = "common/scratch.rs"]
mod scratch;
use scratch::TestScratch;

#[test]
fn successful_scope_removes_only_its_registered_directory() {
    let path;
    {
        let _scratch = TestScratch::new();
        path = TestScratch::path("success-control");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("diagnostic"), b"successful synthetic control").unwrap();
        assert!(path.is_dir());
    }
    assert!(!path.exists());
}

#[test]
fn actual_failing_test_retains_diagnostics_until_parent_verifies_them() {
    if let Some(marker) = std::env::var_os("JOIL_SCRATCH_FAILURE_CONTROL") {
        let _scratch = TestScratch::new();
        let path = TestScratch::path("failure-control");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join("diagnostic"),
            b"retained failing synthetic control",
        )
        .unwrap();
        std::fs::write(marker, path.to_str().unwrap()).unwrap();
        panic!("intentional child failure: retain diagnostic directory");
    }
    let _scratch = TestScratch::new();
    let parent = TestScratch::path("failure-parent");
    std::fs::create_dir_all(&parent).unwrap();
    let marker = parent.join("child-path");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "actual_failing_test_retains_diagnostics_until_parent_verifies_them",
            "--nocapture",
        ])
        .env("JOIL_SCRATCH_FAILURE_CONTROL", &marker)
        .output()
        .unwrap();
    eprintln!(
        "intentional child exit: {:?}\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(101));
    assert!(String::from_utf8_lossy(&output.stderr).contains("retained failed-test scratch:"));
    let retained = std::path::PathBuf::from(std::fs::read_to_string(marker).unwrap());
    assert_eq!(retained.parent(), parent.parent());
    assert!(retained
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("v5-test-failure-control-"));
    assert_eq!(
        std::fs::read(retained.join("diagnostic")).unwrap(),
        b"retained failing synthetic control"
    );
    // The failure did retain its bytes. The parent owns this deliberate negative
    // control and bounds its output only after checking the retained directory.
    std::fs::remove_dir_all(retained).unwrap();
}
