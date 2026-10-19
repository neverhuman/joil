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
mod tests;
