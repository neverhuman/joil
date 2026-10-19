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
// Python json.dumps uses ensure_ascii=True for receipt paths and command strings.
fn write(path: impl AsRef<Path>, v: &Ordered) -> Result<()> {
    let mut encoded = String::new();
    for c in (v.pretty(0) + "\n").chars() {
        if c.is_ascii() && c != '\u{7f}' {
            encoded.push(c);
        } else {
            for unit in c.encode_utf16(&mut [0; 2]) {
                encoded.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    fs::write(path, encoded)?;
    Ok(())
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .ok_or_else(|| format!("missing string {k}").into())
}
fn output_bytes(root: &Path) -> Result<u64> {
    let mut total = 0;
    for folder in ["runs", "artifacts"] {
        for p in files(&root.join(folder))? {
            if !p.is_symlink() {
                match fs::metadata(p) {
                    Ok(metadata) => total += metadata.len(),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
        }
    }
    Ok(total)
}
fn allocation(root: &Path) -> Result<(Vec<usize>, String)> {
    let raw = match std::env::var("JOIL_CPU_SET") {
        Ok(v) => v,
        Err(_) => "0,1,2,3".into(),
    };
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
fn probe(
    root: &Path,
    case: &Path,
    lock: &Value,
    name: &str,
    argv: &[&str],
    timeout: u64,
) -> Result<Ordered> {
    require(
        case.starts_with(root.join("runs")),
        "only project runs may be writable solver cases",
    )?;
    let (cpus, lease) = allocation(root)?;
    let source_sha256 = sha(fs::read(
        root.join("physics/openfoam/environment/runtime.py"),
    )?);
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
    fs::create_dir_all(case)?;
    let stream = fs::File::create(&path)?;
    let mut child = cmd
        .stdout(Stdio::from(stream.try_clone()?))
        .stderr(Stdio::from(stream))
        .spawn()?;
    let mut reason = None;
    let mut next = Duration::from_secs(10);
    let code = loop {
        if let Some(status) = child.try_wait()? {
            use std::os::unix::process::ExitStatusExt;
            break match status.code() {
                Some(code) => code,
                None => -status.signal().ok_or("missing process signal")?,
            };
        }
        if start.elapsed() >= next {
            peak = peak.max(output_bytes(root)?);
            next = start.elapsed() + Duration::from_secs(10);
            if peak > 5 * 1024u64.pow(3) {
                reason = Some("shared_output_allocation_exhausted");
            }
        }
        if start.elapsed() > Duration::from_secs(timeout) {
            reason = Some("timeout");
        }
        if reason.is_some() {
            capture(
                Command::new("docker").args(["stop", "-t", "5", &container]),
                Duration::from_secs(20),
            )?;
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
        ("driver_source_sha256", val(source_sha256)),
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
// Python's default :g is six significant digits, including scientific exponents.
fn time_label(value: f64) -> Result<String> {
    let format = b"%.6g\0";
    let mut buffer = [0 as libc::c_char; 64];
    // SAFETY: fixed format expects one double; buffer capacity is supplied exactly.
    let n = unsafe {
        libc::snprintf(
            buffer.as_mut_ptr(),
            buffer.len(),
            format.as_ptr().cast(),
            value,
        )
    };
    require(n >= 0 && (n as usize) < buffer.len(), "invalid saved time")?;
    Ok(buffer[..n as usize]
        .iter()
        .map(|b| *b as u8 as char)
        .collect())
}
fn rewrite_boundary(case: &Path) -> Result<()> {
    let path = case.join("constant/polyMesh/boundary");
    let before = fs::read(&path)?;
    let source = std::str::from_utf8(&before)?;
    let mut matches = Vec::new();
    for (start, _) in source.match_indices("wall") {
        if start > 0
            && source[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let mut rest = &source[start + 4..];
        rest = match rest.trim_start().strip_prefix('{') {
            Some(v) => v,
            None => continue,
        };
        rest = match rest.trim_start().strip_prefix("type") {
            Some(v) => v,
            None => continue,
        };
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        rest = rest.trim_start();
        if let Some(tail) = rest.strip_prefix("patch") {
            if tail.trim_start().starts_with(';') {
                matches.push(source.len() - rest.len());
            }
        }
    }
    require(
        matches.len() == 1,
        "expected exactly one generic wall group from gmshToFoam",
    )?;
    let mut after = source.to_string();
    after.replace_range(matches[0]..matches[0] + 5, "wall");
    fs::write(&path, &after)?;
    write(
        case.join("boundary-type-receipt.json"),
        &obj(vec![
            (
                "operation",
                val("set wall physical group to OpenFOAM wall patch type"),
            ),
            ("before_sha256", val(sha(before))),
            ("after_sha256", val(sha(after))),
        ]),
    )
}
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let stage = args.next().ok_or("stage required")?;
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut case = None;
    let mut binary = None;
    let mut time = None;
    while let Some(arg) = args.next() {
        let v = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--root" => root = v.into(),
            "--case" => case = Some(PathBuf::from(v)),
            "--metrics-bin" => binary = Some(PathBuf::from(v)),
            "--time" => time = Some(v.parse::<f64>()?),
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    let root = root.canonicalize()?;
    let case = resolve(&case.ok_or("--case required")?)?;
    let label = match time {
        Some(t) => format!("-t{}", time_label(t)?),
        None => String::new(),
    };
    let mut stages: Vec<Vec<String>> = match stage.as_str() {
        "mesh" => vec![vec!["blockMesh"],vec!["snappyHexMesh","-overwrite"],vec!["checkMesh","-allTopology","-allGeometry"]],
        "tet" => vec![vec!["gmshToFoam","ct35.msh"]],
        "water" => vec![vec!["decomposePar","-force"],vec!["mpirun","--bind-to","none","-np","4","foamRun","-parallel"]],
        "reconstruct" => vec![vec!["reconstructPar","-latestTime"]],
        "centres" => vec![vec!["foamPostProcess","-func","writeCellCentres","-latestTime"]],
        "build-metrics" => vec![vec!["bash","-c","cp -r /work/physics/openfoam/carrier/hydraulicMetrics build && export FOAM_USER_APPBIN=/case/bin && mkdir -p \"$FOAM_USER_APPBIN\" && cd build && wmake"]],
        "metrics" => vec![], _ => return Err("invalid stage".into()),
    }.into_iter().map(|a| a.into_iter().map(str::to_owned).collect()).collect();
    if stage == "metrics" {
        let binary =
            resolve(&binary.ok_or("--metrics-bin must identify the compiled audited utility")?)?;
        require(
            binary.is_file() && binary.starts_with(root.join("runs")),
            "metrics binary must be a project run artifact",
        )?;
        let relative = binary.strip_prefix(&root)?.to_string_lossy().into_owned();
        stages.push(vec![format!("/work/{relative}"), "-latestTime".into()]);
        require(
            !case.join("hydraulic-integrals.json").exists(),
            "preserve existing native integral output before another extraction",
        )?;
        let metadata = case.join(format!("metrics-binary{label}.json"));
        require(
            !metadata.exists(),
            "metrics extraction already recorded for this saved time",
        )?;
        write(
            metadata,
            &obj(vec![
                ("path", val(relative)),
                ("sha256", val(sha(fs::read(binary)?))),
            ]),
        )?;
    }
    for (i, argv) in stages.iter_mut().enumerate() {
        if let Some(t) = time {
            if matches!(stage.as_str(), "reconstruct" | "centres" | "metrics") {
                argv.pop();
                argv.extend(["-time".into(), time_label(t)?]);
            }
        }
        if stage == "metrics" && case.join("device-plane.json").exists() {
            let plane: Value = serde_json::from_slice(&fs::read(case.join("device-plane.json"))?)?;
            require(
                sha(fs::read(
                    case.join("constant/polyMesh/sets/deviceOverflow"),
                )?) == text(&plane, "face_set_sha256")?,
                "original device measurement plane changed",
            )?;
            argv.push("-devicePlane".into());
        }
        let name = format!(
            "{stage}{label}-{i}-{}",
            argv[0].rsplit('/').next().ok_or("empty command")?
        );
        require(
            !case.join(format!("{name}.log")).exists(),
            "stage log exists; use a new named attempt/case",
        )?;
        let lock: Value = serde_json::from_slice(&fs::read(
            root.join("physics/openfoam/environment/lock.json"),
        )?)?;
        probe(
            &root,
            &case,
            &lock,
            &name,
            &argv.iter().map(String::as_str).collect::<Vec<_>>(),
            3600,
        )?;
    }
    if stage == "metrics" {
        if let Some(t) = time {
            let target = case.join(format!("hydraulic-integrals{label}.json"));
            require(!target.exists(), "integral receipt already exists")?;
            let native = case.join("hydraulic-integrals.json");
            let value: Value = serde_json::from_slice(&fs::read(&native)?)?;
            require(
                value["time"].as_f64() == Some(t),
                "wrong saved time extracted",
            )?;
            fs::rename(native, target)?;
        }
    }
    if stage == "tet" {
        rewrite_boundary(&case)?;
        let lock: Value = serde_json::from_slice(&fs::read(
            root.join("physics/openfoam/environment/lock.json"),
        )?)?;
        probe(
            &root,
            &case,
            &lock,
            "mesh-2-checkMesh",
            &["checkMesh", "-allTopology", "-allGeometry"],
            300,
        )?;
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
