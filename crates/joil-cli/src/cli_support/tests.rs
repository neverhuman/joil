use super::*;

const COMMITTED_WAVE2: &str =
    include_str!("../../../../docs/results/wave2-multiseed-ablation-and-holdout-v1.json");

fn argv(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// SYNTHETIC files in a fresh temp directory; nothing is executed.
fn gpu_fixture(name: &str, lease: Option<&str>) -> (PathBuf, String, String) {
    let dir = std::env::temp_dir().join(format!("joil-v5-01-04-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join("joil_cuda_eval");
    std::fs::write(&bin, b"SYNTHETIC placeholder, never executed").unwrap();
    let lease_path = dir.join("lease.json");
    if let Some(text) = lease {
        std::fs::write(&lease_path, text).unwrap();
    }
    (
        dir,
        bin.display().to_string(),
        lease_path.display().to_string(),
    )
}

/// 2026-10-17T22:00:00Z, independently computed with Python datetime.
const NOW_2026_10_17T22: i64 = 1_792_274_400;

#[test]
fn v5_01_04_ordinary_runs_request_no_gpu() {
    let now = NOW_2026_10_17T22;
    assert_eq!(
        explicit_gpu_opt_in(&argv(&["evolve", "--out", "x"]), now).unwrap(),
        (None, None)
    );
    // A present default binary path no longer activates anything implicitly.
    let (dir, _bin, lease) = gpu_fixture("ordinary", Some(r#"{"gpu": true}"#));
    assert!(explicit_gpu_opt_in(&argv(&["evolve", "--gpu-lease", &lease]), now).is_err());
    assert!(explicit_gpu_opt_in(&argv(&["evolve", "--gpu-vram-target-pct", "85.5"]), now).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn v5_01_04_missing_or_invalid_gpu_lease_cannot_execute() {
    let now = NOW_2026_10_17T22;
    let good = r#"{"gpu": true, "expires_utc": "2026-10-18T08:52:43+00:00"}"#;
    let cases = [
        ("no-lease-file", None, "cannot read GPU lease"),
        (
            "gpu-false",
            Some(r#"{"gpu": false, "expires_utc": "2026-10-18T08:52:43Z"}"#),
            "does not grant",
        ),
        (
            "gpu-string",
            Some(r#"{"gpu": "true", "expires_utc": "2026-10-18T08:52:43Z"}"#),
            "does not grant",
        ),
        ("no-expiry", Some(r#"{"gpu": true}"#), "lacks expires_utc"),
        (
            "expired",
            Some(r#"{"gpu": true, "expires_utc": "2026-10-17T21:59:59Z"}"#),
            "expired",
        ),
        ("not-json", Some("gpu: true"), "invalid GPU lease"),
    ];
    for (name, lease, needle) in cases {
        let (dir, bin, lease_path) = gpu_fixture(name, lease);
        let err = explicit_gpu_opt_in(
            &argv(&["evolve", "--cuda-bin", &bin, "--gpu-lease", &lease_path]),
            now,
        )
        .unwrap_err();
        assert!(err.contains(needle), "{name}: {err}");
        std::fs::remove_dir_all(dir).unwrap();
    }
    let (dir, bin, lease_path) = gpu_fixture("no-lease-arg", Some(good));
    let err = explicit_gpu_opt_in(&argv(&["evolve", "--cuda-bin", &bin]), now).unwrap_err();
    assert!(err.contains("requires --gpu-lease"), "{err}");
    let missing_bin = format!("{}.missing", bin);
    assert!(explicit_gpu_opt_in(
        &argv(&[
            "evolve",
            "--cuda-bin",
            &missing_bin,
            "--gpu-lease",
            &lease_path
        ]),
        now
    )
    .unwrap_err()
    .contains("not a file"));
    for bad in ["0", "-5", "100.5", "NaN", "inf", "x"] {
        assert!(explicit_gpu_opt_in(
            &argv(&[
                "evolve",
                "--cuda-bin",
                &bin,
                "--gpu-lease",
                &lease_path,
                "--gpu-vram-target-pct",
                bad,
            ]),
            now
        )
        .is_err());
    }
    // Positive control: explicit binary, current lease and explicit VRAM target.
    let (got_bin, vram) = explicit_gpu_opt_in(
        &argv(&[
            "evolve",
            "--cuda-bin",
            &bin,
            "--gpu-lease",
            &lease_path,
            "--gpu-vram-target-pct",
            "50",
        ]),
        now,
    )
    .unwrap();
    assert_eq!(got_bin, Some(PathBuf::from(&bin)));
    assert_eq!(vram, Some(50.0));
    assert_eq!(
        explicit_gpu_opt_in(
            &argv(&["evolve", "--cuda-bin", &bin, "--gpu-lease", &lease_path]),
            now
        )
        .unwrap()
        .1,
        None
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn v5_01_05_existing_outputs_and_committed_receipts_are_refused() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for rel in [
        "docs/results/wave2-multiseed-ablation-and-holdout-v1.json",
        "docs/results/wave3-evolution-proof-v1.json",
        "docs/results/interactive-3d-archetypes-v1.json",
    ] {
        let committed = root.join(rel);
        let before = std::fs::read(&committed).unwrap();
        let arg = committed.display().to_string();
        let err =
            fresh_output(&argv(&["x", "--out", &arg]), "--out", Some("x"), ".json").unwrap_err();
        assert!(err.contains("refusing to replace existing output"), "{err}");
        assert!(write_new(&committed, "SYNTHETIC").is_err());
        assert_eq!(std::fs::read(&committed).unwrap(), before);
    }
    let (dir, _bin, _lease) = gpu_fixture("sentinel", None);
    let sentinel = dir.join("sentinel.json");
    std::fs::write(&sentinel, b"keep").unwrap();
    let arg = sentinel.display().to_string();
    assert!(fresh_output(&argv(&["x", "--glb", &arg]), "--glb", None, ".glb").is_err());
    assert!(write_optional_new(&Some(sentinel.clone()), "SYNTHETIC").is_err());
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep");
    // A fresh explicit path is created exactly once.
    let fresh = dir.join("nested/fresh.json");
    let got = fresh_output(
        &argv(&["x", "--out", &fresh.display().to_string()]),
        "--out",
        None,
        ".json",
    )
    .unwrap()
    .unwrap();
    write_new(&got, "{}").unwrap();
    assert!(write_new(&got, "{}").is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn v5_01_05_defaults_are_unique_ignored_run_paths() {
    let no_out = argv(&["wave3-proof"]);
    let file = fresh_required_output(&no_out, "--out", "wave3-proof", ".json").unwrap();
    let text = file.display().to_string();
    assert!(
        text.starts_with("runs/wave3-proof-") && text.ends_with(".json"),
        "{text}"
    );
    assert!(
        text.contains(&format!("-{}.json", std::process::id())),
        "{text}"
    );
    let dir = fresh_required_output(&no_out, "--case", "cfd-case", "").unwrap();
    assert!(dir.display().to_string().starts_with("runs/cfd-case-"));
    assert_eq!(fresh_output(&no_out, "--out", None, ".json").unwrap(), None);
}

#[test]
fn v5_01_04_utc_clock_format() {
    let now = utc_now_seconds().unwrap();
    assert_eq!(now.len(), 19);
    assert_eq!(&now[10..11], "T");
    assert!(now.as_str() > "2026-01-01T00:00:00");
    // The clock formatter and the expiry parser are inverses (within a second boundary).
    let parsed = parse_utc_expiry_seconds(&format!("{now}Z")).unwrap();
    assert!((utc_now_unix_seconds().unwrap() - parsed).abs() <= 2);
}

#[test]
fn v5_01_04_expiry_parser_matches_independent_epoch_oracles() {
    // Expected values computed independently with Python datetime (UTC).
    for (text, want) in [
        ("1970-01-01T00:00:00Z", 0_i64),
        ("2000-02-29T12:00:00Z", 951_825_600),
        ("2026-10-17T22:00:00+00:00", NOW_2026_10_17T22),
        ("2026-10-18T08:52:43Z", 1_792_313_563),
        ("2026-10-18T08:52:43.818175+00:00", 1_792_313_563),
        ("1900-03-01T00:00:00Z", -2_203_891_200),
        ("9999-12-31T23:59:59Z", 253_402_300_799),
        ("0001-01-01T00:00:00Z", -62_135_596_800),
    ] {
        assert_eq!(parse_utc_expiry_seconds(text), Ok(want), "{text}");
    }
}

/// Review 2 controls through the production `explicit_gpu_opt_in` path; nothing is executed.
#[test]
fn v5_01_04_malformed_or_non_utc_expiry_never_authorizes_gpu() {
    let now = NOW_2026_10_17T22;
    let run = |name: &str, expires: &str| {
        let lease = serde_json::json!({"gpu": true, "expires_utc": expires}).to_string();
        let (dir, bin, lease_path) = gpu_fixture(name, Some(&lease));
        let result = explicit_gpu_opt_in(
            &argv(&["evolve", "--cuda-bin", &bin, "--gpu-lease", &lease_path]),
            now,
        );
        std::fs::remove_dir_all(dir).unwrap();
        result
    };
    // Positive: valid future UTC expiry, with Z, +00:00 and a fraction.
    for (i, ok) in [
        "2026-10-18T08:52:43Z",
        "2026-10-18T08:52:43+00:00",
        "2026-10-18T08:52:43.818175+00:00",
    ]
    .into_iter()
    .enumerate()
    {
        let (bin, vram) = run(&format!("valid-{i}"), ok).unwrap();
        assert!(bin.is_some(), "{ok}");
        assert_eq!(vram, None);
    }
    let cases = [
        (
            "feb-30",
            "2026-02-30T00:00:00Z",
            "invalid calendar date 2026-02-30",
        ),
        (
            "non-leap-feb-29",
            "2027-02-29T00:00:00Z",
            "invalid calendar date 2027-02-29",
        ),
        (
            "century-feb-29",
            "2100-02-29T00:00:00Z",
            "invalid calendar date 2100-02-29",
        ),
        (
            "all-nines",
            "9999-99-99T99:99:99Z",
            "invalid calendar date 9999-99-99",
        ),
        (
            "all-nines-no-zone",
            "9999-99-99T99:99:99",
            "missing time zone designator",
        ),
        (
            "month-13",
            "2026-13-01T00:00:00Z",
            "invalid calendar date 2026-13-01",
        ),
        (
            "year-zero",
            "0000-01-01T00:00:00Z",
            "invalid calendar date 0000-01-01",
        ),
        (
            "hour-24",
            "2026-10-18T24:00:00Z",
            "invalid time of day 24:00:00",
        ),
        (
            "minute-60",
            "2026-10-18T08:60:00Z",
            "invalid time of day 08:60:00",
        ),
        (
            "leap-second",
            "2026-10-18T08:52:60Z",
            "invalid time of day 08:52:60",
        ),
        ("plus-two", "2026-10-18T08:52:43+02:00", "offset is not UTC"),
        (
            "minus-five",
            "2026-10-18T08:52:43-05:00",
            "offset is not UTC",
        ),
        (
            "minus-zero",
            "2026-10-18T08:52:43-00:00",
            "offset is not UTC",
        ),
        (
            "no-zone",
            "2026-10-18T08:52:43",
            "missing time zone designator",
        ),
        (
            "trailing",
            "2026-10-18T08:52:43Zjunk",
            "malformed timestamp syntax",
        ),
        (
            "lower-z",
            "2026-10-18T08:52:43z",
            "malformed timestamp syntax",
        ),
        (
            "space-sep",
            "2026-10-18 08:52:43Z",
            "malformed timestamp syntax",
        ),
        (
            "five-digit-year",
            "20261-10-18T08:52:43Z",
            "malformed timestamp syntax",
        ),
        (
            "long-fraction",
            "2026-10-18T08:52:43.1234567890Z",
            "malformed timestamp syntax",
        ),
        (
            "empty-fraction",
            "2026-10-18T08:52:43.Z",
            "malformed timestamp syntax",
        ),
        (
            "signed-field",
            "2026-10-+8T08:52:43Z",
            "malformed timestamp syntax",
        ),
        ("short", "2026-10-18", "malformed timestamp syntax"),
        // Byte 18-19 is a two-byte character: the old slice at [..19] panicked here.
        ("utf8-split", "2026-10-18T08:52:4\u{e9}Z", "not ASCII"),
        ("utf8-fullwidth", "2026-10-18T08:52:4\u{ff13}Z", "not ASCII"),
        ("past", "2026-10-17T21:59:59Z", "expired at"),
        ("equal-now", "2026-10-17T22:00:00Z", "expired at"),
    ];
    for (name, expires, needle) in cases {
        let err = run(name, expires).unwrap_err();
        assert!(err.contains(needle), "{name}: {err}");
        if needle != "expired at" {
            assert!(err.contains("invalid expires_utc"), "{name}: {err}");
        }
    }
    // A non-string expiry is still refused.
    let (dir, bin, lease_path) = gpu_fixture(
        "numeric-expiry",
        Some(r#"{"gpu": true, "expires_utc": 1792313563}"#),
    );
    let err = explicit_gpu_opt_in(
        &argv(&["evolve", "--cuda-bin", &bin, "--gpu-lease", &lease_path]),
        now,
    )
    .unwrap_err();
    assert!(err.contains("lacks expires_utc"), "{err}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn historical_wave2_receipt_loads_all_promoted_genomes() {
    let report: serde_json::Value = serde_json::from_str(COMMITTED_WAVE2).unwrap();
    let elites = report["promoted_wave2_elites_by_family"]
        .as_object()
        .unwrap();
    assert_eq!(elites.len(), 11);
    for (family, elite) in elites {
        let loaded = load_wave2_elite_program(COMMITTED_WAVE2, family).unwrap();
        let archived: program::DesignProgram =
            serde_json::from_value(elite["program"].clone()).unwrap();
        assert_eq!(loaded, archived.canonicalize().unwrap(), "{family}");
        assert_eq!(loaded.schema_version, 2);
        assert_eq!(loaded.family, "ct35-exotic-head-v2");
        let current_hash = loaded.canonical_sha256().unwrap();
        assert_eq!(current_hash.len(), 64);
        // Authored topology labels no longer belong to the serialized
        // subject. The archived hash cannot identify the prepared case.
        assert_ne!(Some(current_hash.as_str()), elite["genome_sha256"].as_str());
    }
}

#[test]
fn historical_wave2_import_does_not_require_or_trust_evaluations() {
    let mut report: serde_json::Value = serde_json::from_str(COMMITTED_WAVE2).unwrap();
    let family = "trifurcated-6jet-vortex-head";
    let elite = &mut report["promoted_wave2_elites_by_family"][family];
    let genome = elite["program"].clone();
    // A genome-only archive is sufficient. No current Individual, screen,
    // admission or objective evidence is synthesized for missing fields.
    *elite = serde_json::json!({"program": genome});
    let raw = serde_json::to_string(&report).unwrap();
    assert_eq!(
        load_wave2_elite_program(&raw, family).unwrap(),
        load_wave2_elite_program(COMMITTED_WAVE2, family).unwrap()
    );
}

#[test]
fn historical_wave2_import_rejects_missing_or_invalid_genomes() {
    assert!(load_wave2_elite_program(COMMITTED_WAVE2, "missing-family").is_err());
    for raw in [
        "not json",
        "{}",
        r#"{"promoted_wave2_elites_by_family":[]}"#,
    ] {
        assert!(load_wave2_elite_program(raw, "baseline-ct35").is_err());
    }
    let mut report: serde_json::Value = serde_json::from_str(COMMITTED_WAVE2).unwrap();
    for genome in [
        serde_json::Value::Null,
        serde_json::json!({"schema_version": 2}),
        {
            let mut genome =
                report["promoted_wave2_elites_by_family"]["baseline-ct35"]["program"].clone();
            genome["inlet_split"]["branches_per_port"] = serde_json::json!(0);
            genome
        },
    ] {
        report["promoted_wave2_elites_by_family"]["baseline-ct35"]["program"] = genome;
        let raw = serde_json::to_string(&report).unwrap();
        assert!(load_wave2_elite_program(&raw, "baseline-ct35").is_err());
    }
    report["promoted_wave2_elites_by_family"]["baseline-ct35"] = serde_json::json!({});
    assert!(
        load_wave2_elite_program(&serde_json::to_string(&report).unwrap(), "baseline-ct35")
            .is_err()
    );
}
