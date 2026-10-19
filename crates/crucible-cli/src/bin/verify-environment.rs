#[allow(dead_code)]
#[path = "../ordered_json.rs"]
mod ordered_json;
#[allow(dead_code)]
#[path = "../port_support.rs"]
mod port_support;
use ordered_json::Ordered;
use port_support::*;
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .ok_or_else(|| format!("missing string {k}").into())
}
fn output_bytes(root: &Path) -> Result<u64> {
    let mut total = 0;
    for folder in ["runs", "artifacts"] {
        for p in files(&root.join(folder))? {
            if !p.is_symlink() {
                total += fs::metadata(p)?.len();
            }
        }
    }
    Ok(total)
}
fn allocation(root: &Path) -> Result<(Vec<usize>, String)> {
    let raw = std::env::var("JOIL_CPU_SET").unwrap_or_else(|_| "0,1,2,3".into());
    let cpus = raw
        .split(',')
        .map(|x| x.trim().parse())
        .collect::<std::result::Result<Vec<usize>, _>>()?;
    require(
        cpus.len() == 4 && cpus.iter().collect::<std::collections::BTreeSet<_>>().len() == 4,
        "pilot runner requires four distinct explicitly allocated CPUs",
    )?;
    if cpus == [0, 1, 2, 3] {
        return Ok((cpus, "initial_project_allocation".into()));
    }
    let raw = fs::read(root.join(".private/coordination/resource-lease.json"))?;
    let lease: Value = serde_json::from_slice(&raw)?;
    let expiry = Command::new("date")
        .args(["-d", text(&lease, "expires_utc")?, "+%s"])
        .output()?;
    require(expiry.status.success(), "invalid resource lease expiry")?;
    let expires = String::from_utf8(expiry.stdout)?.trim().parse::<u64>()?;
    require(
        expires > SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        "resource lease expired",
    )?;
    require(
        lease["leases"]
            .as_array()
            .ok_or("missing leases")?
            .iter()
            .any(|v| v["cpus"] == json!(cpus)),
        "CPU set is not allocated in the coordination lease",
    )?;
    Ok((cpus, sha(raw)))
}
fn probe(root: &Path, case: &Path, lock: &Value, name: &str, argv: &[&str]) -> Result<Ordered> {
    let (cpus, lease) = allocation(root)?;
    let image = text(lock, "image_id")?;
    require(
        image.starts_with("sha256:") && image.len() == 71,
        "runtime must be bound to an immutable local image ID",
    )?;
    let mut random = [0u8; 8];
    fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
    let container = format!(
        "joil-{}",
        random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    // SAFETY: getuid/getgid query the caller and accept no pointers.
    let user = unsafe { format!("{}:{}", libc::getuid(), libc::getgid()) };
    let mut cmd = Command::new("docker");
    cmd.args([
        "run",
        "--name",
        &container,
        "--rm",
        "--init",
        "--network=none",
        "--read-only",
        "--cpus=4",
    ])
    .arg(format!(
        "--cpuset-cpus={}",
        cpus.iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ))
    .args([
        "--memory=12g",
        "--memory-swap=12g",
        "--pids-limit=256",
        "--cap-drop=ALL",
        "--security-opt=no-new-privileges",
        "--user",
        &user,
        "--tmpfs",
        "/tmp:rw,nosuid,nodev,size=256m,mode=1777",
        "-e",
        "HOME=/tmp",
        "-e",
        "OMP_NUM_THREADS=1",
        "-e",
        "OPENBLAS_NUM_THREADS=1",
        "-v",
    ])
    .arg(format!("{}:/work:ro", root.display()))
    .arg("-v")
    .arg(format!("{}:/case:rw", case.display()))
    .args(["-w", "/case", image])
    .args(argv);
    let start = Instant::now();
    let mut peak = output_bytes(root)?;
    require(
        peak <= 5 * 1024u64.pow(3),
        "shared 5 GiB output allocation exhausted",
    )?;
    let path = case.join(format!("{name}.log"));
    let stream = fs::File::create(&path)?;
    let mut child = cmd
        .stdout(Stdio::from(stream.try_clone()?))
        .stderr(Stdio::from(stream))
        .spawn()?;
    let mut reason = None;
    let mut next = Duration::from_secs(10);
    let code = loop {
        if let Some(status) = child.try_wait()? {
            break status.code().unwrap_or(-1);
        }
        if start.elapsed() >= next {
            peak = peak.max(output_bytes(root)?);
            next = start.elapsed() + Duration::from_secs(10);
            if peak > 5 * 1024u64.pow(3) {
                reason = Some("shared_output_allocation_exhausted");
            }
        }
        if start.elapsed() > Duration::from_secs(60) {
            reason = Some("timeout");
        }
        if reason.is_some() {
            let _ = capture(
                Command::new("docker").args(["stop", "-t", "5", &container]),
                Duration::from_secs(20),
            );
            let stop = Instant::now();
            while child.try_wait()?.is_none() && stop.elapsed() < Duration::from_secs(10) {
                std::thread::sleep(Duration::from_millis(100));
            }
            if child.try_wait()?.is_none() {
                child.kill()?;
            }
            child.wait()?;
            break 124;
        }
        std::thread::sleep(Duration::from_millis(250));
    };
    let seconds = start.elapsed().as_secs_f64();
    let raw = fs::read(&path)?;
    let receipt = obj(vec![
        ("command", val(json!(argv))),
        ("image_id", val(image)),
        ("exit_code", val(code)),
        ("wall_seconds", val(seconds)),
        ("log_sha256", val(sha(&raw))),
        ("log_bytes", val(raw.len())),
        ("stop_reason", val(json!(reason))),
        (
            "driver_source_sha256",
            val(sha(fs::read(
                root.join("physics/openfoam/environment/runtime.py"),
            )?)),
        ),
        ("resource_lease_id", val(lease)),
        ("max_observed_shared_output_bytes", val(peak)),
        (
            "allocation",
            obj(vec![
                ("logical_cpus", val(json!(cpus))),
                ("quota_cores", val(4)),
                ("memory_gib", val(12)),
                ("gpu", val(false)),
            ]),
        ),
    ]);
    write(case.join(format!("{name}.log.receipt.json")), &receipt)?;
    println!("{name}.log: exit {code}, {seconds:.2}s");
    require(code == 0, &String::from_utf8_lossy(&raw))?;
    Ok(receipt)
}
fn run() -> Result<()> {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut case = None;
    let mut doctor = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let v = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--root" => root = v.into(),
            "--case" => case = Some(PathBuf::from(v)),
            "--doctor-source" => doctor = Some(PathBuf::from(v)),
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    let root = root.canonicalize()?;
    let env = root.join("physics/openfoam/environment");
    let lock_raw = fs::read(env.join("lock.json"))?;
    let lock: Value = serde_json::from_slice(&lock_raw)?;
    for (name, key) in [
        ("Dockerfile", "dockerfile_sha256"),
        ("openfoam.asc", "repository_key_sha256"),
    ] {
        require(
            sha(fs::read(env.join(name))?) == text(&lock, key)?,
            "environment recipe/key changed from tested lock",
        )?;
    }
    let inspect = capture(
        Command::new("docker").args(["image", "inspect", text(&lock, "image_id")?]),
        Duration::from_secs(60),
    )?;
    require(inspect.status.success(), "docker image inspect failed")?;
    let info: Value = serde_json::from_slice(&inspect.stdout)?;
    require(
        info[0]["Id"] == lock["image_id"]
            && info[0]["Architecture"] == "amd64"
            && info[0]["Os"] == "linux",
        "wrong native image identity/platform",
    )?;
    let case = resolve(&case.ok_or("--case required")?)?;
    require(
        case.starts_with(root.join("runs")) && !case.exists(),
        "fresh project run required",
    )?;
    fs::create_dir_all(&case)?;
    let mut commands = Vec::new();
    for (name, argv) in [
        ("solver", vec!["foamRun", "-help"]),
        ("compiler", vec!["g++", "--version"]),
        ("mpi", vec!["mpirun", "--version"]),
        (
            "packages",
            vec!["dpkg-query", "-W", "-f=${Package}\t${Version}\n"],
        ),
    ] {
        commands.push(probe(&root, &case, &lock, name, &argv)?);
    }
    require(
        fs::read_to_string(case.join("solver.log"))?.contains(text(&lock, "build")?),
        "native OpenFOAM build differs from lock",
    )?;
    let mut versions = Vec::new();
    for name in ["compiler", "mpi"] {
        let raw = fs::read_to_string(case.join(format!("{name}.log")))?;
        versions.push((name, val(raw.lines().next().ok_or("empty version output")?)));
    }
    let packages = fs::read(case.join("packages.log"))?;
    let installed = std::str::from_utf8(&packages)?
        .split('\n')
        .find_map(|s| s.strip_prefix("openfoam14\t"));
    require(
        installed == Some(text(&lock, "package_version")?),
        "installed solver package mismatch",
    )?;
    let doctor_source = match doctor {
        Some(path) => path,
        None => root.join("tools/verify_environment.py"),
    };
    let result = obj(vec![
        (
            "kind",
            val("executed_CPU_environment_capability_not_physics_validation"),
        ),
        ("passed", val(true)),
        ("image_id", val(lock["image_id"].clone())),
        ("build", val(lock["build"].clone())),
        ("platform", val("linux/amd64")),
        ("precision", val("double")),
        ("versions", obj(versions)),
        ("package_manifest_sha256", val(sha(packages))),
        ("lock_sha256", val(sha(lock_raw))),
        ("doctor_source_sha256", val(sha(fs::read(doctor_source)?))),
        ("commands", Ordered::Array(commands)),
        (
            "qualified_for",
            val(json!(["native binary startup and version identity only"])),
        ),
        (
            "not_qualified",
            val(json!([
                "turbulent carrier",
                "oil transport",
                "MPI scaling/restart",
                "GPU",
                "hydrocyclone performance"
            ])),
        ),
        (
            "rebuild_limitation",
            val("transitive apt dependency snapshot not pinned; revalidate a new build identity"),
        ),
    ]);
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(case.join("capability.json"))?;
    writeln!(file, "{}", result.pretty(0))?;
    println!("{}", result.pretty(0));
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
