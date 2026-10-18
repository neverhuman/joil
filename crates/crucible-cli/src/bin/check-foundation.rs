//! Foundation suite runner; receipts describe software checks, never physics.
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::{fs::PermissionsExt, process::ExitStatusExt};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
const SUITES: &[&str] = &[
    "fmt",
    "clippy",
    "rust-tests",
    "baseline",
    "python-tests",
    "public-check",
    "claims-audit",
    "ui",
];
fn sha(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn steps(
    root: &Path,
    suite: Option<&str>,
    skip_ui: bool,
    cad: bool,
    runid: &str,
) -> Vec<(String, Vec<String>, PathBuf)> {
    let definitions = [
        ("fmt", vec!["cargo", "fmt", "--all", "--", "--check"]),
        (
            "clippy",
            vec![
                "cargo",
                "clippy",
                "--workspace",
                "--locked",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        (
            "rust-tests",
            vec!["cargo", "test", "--workspace", "--locked"],
        ),
        (
            "baseline",
            vec![
                "cargo", "run", "--locked", "-p", "joil-cli", "--", "baseline",
            ],
        ),
        (
            "python-tests",
            vec!["python3", "-m", "unittest", "discover", "-s", "tests", "-v"],
        ),
        ("public-check", vec!["python3", "tools/check_public.py"]),
        ("claims-audit", vec!["python3", "tools/claims_audit.py"]),
    ];
    let mut result: Vec<_> = definitions
        .into_iter()
        .map(|(n, a)| {
            (
                n.to_string(),
                a.into_iter().map(String::from).collect::<Vec<_>>(),
                root.to_path_buf(),
            )
        })
        .collect();
    if !skip_ui {
        for (n, a) in [
            ("npm-ci", vec!["npm", "ci", "--no-audit", "--no-fund"]),
            ("ui-build", vec!["npm", "run", "build"]),
        ] {
            result.push((
                n.into(),
                a.into_iter().map(String::from).collect(),
                root.join("apps/explorer"),
            ));
        }
    }
    if cad {
        result.push((
            "cad".into(),
            vec![
                root.join(".venv/bin/python").display().to_string(),
                "cad/ct35/build.py".into(),
                "--out".into(),
                root.join("artifacts")
                    .join(runid)
                    .join("ct35")
                    .display()
                    .to_string(),
            ],
            root.into(),
        ));
    }
    if let Some(s) = suite {
        result.retain(|(n, _, _)| {
            if s == "ui" {
                n == "npm-ci" || n == "ui-build"
            } else {
                n == s
            }
        });
    }
    if suite.is_none() || suite == Some("python-tests") {
        result.insert(
            0,
            (
                "generated-status".into(),
                vec![
                    "python3".into(),
                    "tools/gen_status.py".into(),
                    "--check".into(),
                ],
                root.into(),
            ),
        );
    }
    result
}
fn execute(cmd: &[String], cwd: &Path, out: &Path, name: &str) -> io::Result<(Vec<u8>, i32)> {
    let stdout = out.join(format!("{name}.stdout"));
    let stderr = out.join(format!("{name}.stderr"));
    let mut child = Command::new(&cmd[0])
        .args(&cmd[1..])
        .current_dir(cwd)
        .stdout(Stdio::from(fs::File::create(&stdout)?))
        .stderr(Stdio::from(fs::File::create(&stderr)?))
        .spawn()?;
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if start.elapsed() >= Duration::from_secs(600) {
            child.kill()?;
            child.wait()?;
            return Ok((
                format!("Command {cmd:?} timed out after 600 seconds").into_bytes(),
                124,
            ));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let mut data = fs::read(stdout)?;
    data.extend(fs::read(stderr)?);
    #[cfg(unix)]
    let code = status
        .code()
        .unwrap_or_else(|| -status.signal().unwrap_or(1));
    #[cfg(not(unix))]
    let code = status.code().unwrap_or(1);
    Ok((data, code))
}
fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let mut suite = None;
    let mut cad = false;
    let mut skip_ui = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cad" => cad = true,
            "--skip-ui" => skip_ui = true,
            "--suite" => {
                suite = Some(args.next().ok_or("--suite requires a value")?);
            }
            "--help" | "-h" => {
                println!("check-foundation [--suite NAME | --skip-ui] [--cad]");
                return Ok(true);
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    if suite
        .as_ref()
        .is_some_and(|s| !SUITES.contains(&s.as_str()))
        || (suite.is_some() && (skip_ui || cad))
    {
        return Err("invalid suite or incompatible --suite/--skip-ui/--cad".into());
    }
    let root = env::current_dir()?.canonicalize()?;
    let stamp = Command::new("date")
        .args(["-u", "+%Y%m%dT%H%M%SZ"])
        .output()?;
    if !stamp.status.success() {
        return Err("date failed".into());
    }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let runid = format!(
        "{}-{:08x}",
        String::from_utf8(stamp.stdout)?.trim(),
        (nonce as u32) ^ std::process::id()
    );
    let out = root.join(".private/checks").join(&runid);
    fs::create_dir_all(&out)?;
    #[cfg(unix)]
    fs::set_permissions(root.join(".private"), fs::Permissions::from_mode(0o700))?;
    let home = PathBuf::from(env::var_os("HOME").ok_or("HOME missing")?);
    let mut paths = vec![
        root.join(".private/node-runtime/bin"),
        root.join(".private/node-runtime/node_modules/.bin"),
        home.join("joil/.private/node-runtime/bin"),
        home.join(".cargo/bin"),
    ];
    paths.extend(env::split_paths(
        &env::var_os("PATH").ok_or("PATH missing")?,
    ));
    env::set_var("PATH", env::join_paths(paths)?);
    for (k, v) in [
        ("CARGO_BUILD_JOBS", "2"),
        ("OMP_NUM_THREADS", "1"),
        ("OPENBLAS_NUM_THREADS", "1"),
        ("MKL_NUM_THREADS", "1"),
        ("RAYON_NUM_THREADS", "1"),
    ] {
        env::set_var(k, v);
    }
    for (k, v) in [
        ("CARGO_INCREMENTAL", "0"),
        ("CARGO_PROFILE_DEV_DEBUG", "line-tables-only"),
    ] {
        if env::var_os(k).is_none() {
            env::set_var(k, v);
        }
    }
    let steps = steps(&root, suite.as_deref(), skip_ui, cad, &runid);
    let revision = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    let head = if revision.status.success() {
        Some(String::from_utf8(revision.stdout)?.trim().to_owned())
    } else {
        None
    };
    let names = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()?;
    if !names.status.success() {
        return Err("git ls-files failed".into());
    }
    let mut hashes = Map::new();
    for raw in names.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let name = std::str::from_utf8(raw)?;
        let path = root.join(name);
        if path.is_file() && !name.starts_with("docs/receipts/") {
            hashes.insert(name.into(), json!(sha(&fs::read(path)?)));
        }
    }
    let mut receipt = json!({"schema_version":1,"run_id":runid,"kind":"foundation_checks_not_physical_validation","steps":[],"requested_suite":suite.as_deref().unwrap_or("all"),"ui_executed":steps.iter().any(|(n,_,_)| n == "ui-build"),"cad_executed":cad,"cfd_executed":false,"input_head":head,"source_files_sha256":hashes,"environment":{"CARGO_BUILD_JOBS":"2","OMP_NUM_THREADS":"1","OPENBLAS_NUM_THREADS":"1","RAYON_NUM_THREADS":"1"}});
    for (name, cmd, cwd) in &steps {
        let start = Instant::now();
        let (data, code) =
            execute(cmd, cwd, &out, name).unwrap_or_else(|e| (e.to_string().into_bytes(), 124));
        fs::write(out.join(format!("{name}.log")), &data)?;
        let relative = cwd.strip_prefix(&root)?.to_string_lossy();
        receipt["steps"].as_array_mut().unwrap().push(json!({"name":name,"command":cmd,"cwd":if relative.is_empty(){"."}else{&relative},"exit_code":code,"wall_seconds":start.elapsed().as_secs_f64(),"log_bytes":data.len(),"log_sha256":sha(&data)}));
        println!("{name}: exit {code}, {:.3}s", start.elapsed().as_secs_f64());
        if code != 0 {
            eprintln!(
                "{}",
                String::from_utf8_lossy(&data[data.len().saturating_sub(7500)..])
            );
            break;
        }
    }
    let actual = receipt["steps"].as_array().unwrap();
    let passed = actual.len() == steps.len() && actual.iter().all(|s| s["exit_code"] == 0);
    receipt["all_requested_pass"] = Value::Bool(passed);
    fs::write(
        out.join("receipt.json"),
        format!("{}\n", serde_json::to_string_pretty(&receipt)?),
    )?;
    println!("receipt: {}", out.join("receipt.json").display());
    Ok(passed)
}
fn main() {
    match run() {
        Ok(true) => (),
        Ok(false) => std::process::exit(1),
        Err(e) => {
            eprintln!("FAIL: {e}");
            let usage_error = e.to_string().starts_with("unknown argument:")
                || e.to_string() == "--suite requires a value"
                || e.to_string() == "invalid suite or incompatible --suite/--skip-ui/--cad";
            std::process::exit(if usage_error { 2 } else { 1 });
        }
    }
}
