use joil_core::program;
use std::path::PathBuf;

pub(crate) fn parse_arg(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

/// GPU use is explicit opt-in (`V5-01-04`). Without `--cuda-bin` no GPU is requested.
/// `--cuda-bin` needs an existing binary and `--gpu-lease <json>` with `"gpu": true` and an
/// `expires_utc` that parses strictly as UTC (see `parse_utc_expiry_seconds`) and lies after
/// `now_unix_seconds`. `--gpu-vram-target-pct` is passed only when given. Anything incomplete or
/// malformed is an error, never a silent CPU or GPU fallback.
pub(crate) fn explicit_gpu_opt_in(
    args: &[String],
    now_unix_seconds: i64,
) -> Result<(Option<PathBuf>, Option<f64>), String> {
    let lease_arg = parse_arg(args, "--gpu-lease");
    let vram_arg = parse_arg(args, "--gpu-vram-target-pct");
    let Some(bin) = parse_arg(args, "--cuda-bin") else {
        if lease_arg.is_some() || vram_arg.is_some() {
            return Err("--gpu-lease and --gpu-vram-target-pct require --cuda-bin".into());
        }
        return Ok((None, None));
    };
    let bin = PathBuf::from(bin);
    if !bin.is_file() {
        return Err(format!("--cuda-bin {} is not a file", bin.display()));
    }
    let lease_path =
        lease_arg.ok_or("--cuda-bin requires --gpu-lease <lease.json> granting gpu")?;
    let raw = std::fs::read_to_string(&lease_path)
        .map_err(|e| format!("cannot read GPU lease {lease_path}: {e}"))?;
    let lease: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("invalid GPU lease {lease_path}: {e}"))?;
    if lease.get("gpu") != Some(&serde_json::Value::Bool(true)) {
        return Err(format!(
            "GPU lease {lease_path} does not grant \"gpu\": true"
        ));
    }
    let expires = lease
        .get("expires_utc")
        .and_then(|v| v.as_str())
        .ok_or(format!("GPU lease {lease_path} lacks expires_utc"))?;
    let expires_at = parse_utc_expiry_seconds(expires).map_err(|reason| {
        format!("GPU lease {lease_path} has invalid expires_utc {expires:?}: {reason}")
    })?;
    if expires_at <= now_unix_seconds {
        return Err(format!("GPU lease {lease_path} expired at {expires}"));
    }
    let vram = match vram_arg {
        None => None,
        Some(s) => match s.parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 && v <= 100.0 => Some(v),
            _ => return Err(format!("--gpu-vram-target-pct must be in (0, 100]: {s}")),
        },
    };
    Ok((Some(bin), vram))
}

/// Output policy (`V5-01-05`): a default is a unique ignored path under `runs/`; an explicit path
/// must not exist yet. Commands call this before evaluating, so committed receipts and earlier runs
/// are never replaced. `None` default means the output is written only when the flag is given.
pub(crate) fn fresh_output(
    args: &[String],
    flag: &str,
    default_stem: Option<&str>,
    extension: &str,
) -> Result<Option<PathBuf>, String> {
    let path = match (parse_arg(args, flag), default_stem) {
        (Some(p), _) => PathBuf::from(p),
        (None, Some(stem)) => PathBuf::from(format!("runs/{stem}-{}{extension}", run_stamp()?)),
        (None, None) => return Ok(None),
    };
    if path.exists() || path.is_symlink() {
        return Err(format!(
            "refusing to replace existing output {}",
            path.display()
        ));
    }
    Ok(Some(path))
}

/// Same policy for a required output path.
pub(crate) fn fresh_required_output(
    args: &[String],
    flag: &str,
    default_stem: &str,
    extension: &str,
) -> Result<PathBuf, String> {
    fresh_output(args, flag, Some(default_stem), extension)
        .map(|p| p.expect("default stem always yields a path"))
}

fn run_stamp() -> Result<String, String> {
    Ok(format!(
        "{}Z-{}",
        utc_now_seconds()?.replace(['-', ':'], ""),
        std::process::id()
    ))
}

/// Create `path` (and parent directories) and write `text`; an existing file is an error.
pub(crate) fn write_new(path: &std::path::Path, text: &str) -> Result<(), String> {
    use std::io::Write;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .and_then(|mut f| f.write_all(text.as_bytes()))
        .map_err(|e| format!("cannot create new output {}: {e}", path.display()))
}

pub(crate) fn write_optional_new(path: &Option<PathBuf>, text: &str) -> Result<(), String> {
    match path {
        Some(p) => write_new(p, text),
        None => Ok(()),
    }
}

/// Parse a GPU lease expiry as Unix seconds (`V5-01-04`). Accepted form, ASCII only:
/// `YYYY-MM-DDTHH:MM:SS` with an optional `.fraction` of 1-9 digits, then `Z` or `+00:00`.
/// The calendar date (including leap years) and time of day are validated; any other offset,
/// a missing zone, trailing characters or out-of-range fields are rejected. A fraction is
/// truncated, which can only make the expiry earlier.
fn parse_utc_expiry_seconds(text: &str) -> Result<i64, String> {
    if !text.is_ascii() {
        return Err("not ASCII".into());
    }
    let syntax = || {
        "malformed timestamp syntax (expected YYYY-MM-DDTHH:MM:SS[.fraction] then Z or +00:00)"
            .to_string()
    };
    let b = text.as_bytes();
    if b.len() < 19
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
    {
        return Err(syntax());
    }
    let field = |lo: usize, hi: usize| -> Result<i64, String> {
        let digits = &b[lo..hi];
        if !digits.iter().all(u8::is_ascii_digit) {
            return Err(syntax());
        }
        Ok(digits
            .iter()
            .fold(0_i64, |acc, d| acc * 10 + i64::from(d - b'0')))
    };
    let (year, month, day) = (field(0, 4)?, field(5, 7)?, field(8, 10)?);
    let (hour, minute, second) = (field(11, 13)?, field(14, 16)?, field(17, 19)?);
    let mut zone = &b[19..];
    if let Some(fraction) = zone.strip_prefix(b".") {
        let n = fraction.iter().take_while(|c| c.is_ascii_digit()).count();
        if n == 0 || n > 9 {
            return Err(syntax());
        }
        zone = &fraction[n..];
    }
    match zone {
        b"Z" | b"+00:00" => {}
        b"" => return Err("missing time zone designator (Z or +00:00 required)".into()),
        [b'+' | b'-', _, _, b':', _, _] => {
            return Err("offset is not UTC (Z or +00:00 required)".into())
        }
        _ => return Err(syntax()),
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year < 1 || !(1..=days_in_month).contains(&day) {
        return Err(format!(
            "invalid calendar date {year:04}-{month:02}-{day:02}"
        ));
    }
    if hour > 23 || minute > 59 || second > 59 {
        return Err(format!(
            "invalid time of day {hour:02}:{minute:02}:{second:02}"
        ));
    }
    // Days from civil date (proleptic Gregorian), inverse of `utc_now_seconds`.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    Ok(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

pub(crate) fn utc_now_unix_seconds() -> Result<i64, String> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64)
}

/// Current UTC time as `YYYY-MM-DDTHH:MM:SS` (civil-from-days, proleptic Gregorian).
pub(crate) fn utc_now_seconds() -> Result<String, String> {
    let secs = utc_now_unix_seconds()?;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    ))
}

/// Import a historical Wave-2 genome without upgrading its archived evidence.
///
/// Report/Individual evaluation schemas have changed since the committed v1
/// receipt. CFD preparation needs only the selected program: validate and
/// canonicalize that program, then let the case builder compute its current
/// design_sha256. Archived genome hashes (which included authored topology
/// labels), metrics, lineage and qualification flags are not imported.
pub(crate) fn load_wave2_elite_program(
    raw: &str,
    family: &str,
) -> Result<program::DesignProgram, Box<dyn std::error::Error>> {
    let report: serde_json::Value = serde_json::from_str(raw)?;
    let elites = report
        .get("promoted_wave2_elites_by_family")
        .and_then(serde_json::Value::as_object)
        .ok_or("Wave-2 receipt has no promoted_wave2_elites_by_family object")?;
    let elite = elites
        .get(family)
        .ok_or_else(|| format!("Wave-2 family '{family}' not found"))?;
    let genome = elite
        .get("program")
        .ok_or_else(|| format!("Wave-2 family '{family}' has no program"))?;
    let program: program::DesignProgram = serde_json::from_value(genome.clone())?;
    Ok(program.canonicalize()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMITTED_WAVE2: &str =
        include_str!("../../../docs/results/wave2-multiseed-ablation-and-holdout-v1.json");

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
        assert!(
            explicit_gpu_opt_in(&argv(&["evolve", "--gpu-vram-target-pct", "85.5"]), now).is_err()
        );
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
            let err = fresh_output(&argv(&["x", "--out", &arg]), "--out", Some("x"), ".json")
                .unwrap_err();
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
        assert!(load_wave2_elite_program(
            &serde_json::to_string(&report).unwrap(),
            "baseline-ct35"
        )
        .is_err());
    }
}
