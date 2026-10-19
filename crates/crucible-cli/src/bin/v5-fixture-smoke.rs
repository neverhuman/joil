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
    collections::BTreeSet,
    fs,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};
struct Witness {
    checkout: PathBuf,
    evidence: PathBuf,
    commands: Vec<Ordered>,
}
impl Witness {
    fn call(&mut self, argv: Vec<String>, expected: i32) -> Result<Vec<u8>> {
        let start = Instant::now();
        let result = capture(
            Command::new(&argv[0])
                .args(&argv[1..])
                .current_dir(&self.checkout),
            Duration::from_secs(240),
        )?;
        let number = self.commands.len();
        fs::write(
            self.evidence.join(format!("{number:02}.stdout")),
            &result.stdout,
        )?;
        fs::write(
            self.evidence.join(format!("{number:02}.stderr")),
            &result.stderr,
        )?;
        self.commands.push(obj(vec![
            ("argv", val(json!(argv))),
            ("exit_code", val(result.status.code().unwrap_or(-1))),
            ("expected_exit_code", val(expected)),
            ("wall_seconds", val(start.elapsed().as_secs_f64())),
            ("stdout_sha256", val(sha(&result.stdout))),
            ("stderr_sha256", val(sha(&result.stderr))),
        ]));
        write(
            self.evidence.join("commands.json"),
            &Ordered::Array(self.commands.clone()),
        )?;
        require(
            result.status.code() == Some(expected),
            &format!(
                "command {number} exit {:?}: {}",
                result.status.code(),
                String::from_utf8_lossy(&result.stderr)
            ),
        )?;
        Ok(result.stdout)
    }
    fn git(&mut self, args: &[&str]) -> Result<String> {
        let mut argv = vec!["git".into()];
        argv.extend(args.iter().map(|s| s.to_string()));
        Ok(String::from_utf8(self.call(argv, 0)?)?.trim().into())
    }
    fn cli(&mut self, args: &[&str], expected: i32) -> Result<Ordered> {
        let mut argv = ["cargo", "run", "--locked", "-p", "crucible-cli", "--"]
            .map(str::to_string)
            .to_vec();
        argv.extend(args.iter().map(|s| s.to_string()));
        let raw = self.call(argv, expected)?;
        if expected == 0 {
            Ok(serde_json::from_slice(&raw)?)
        } else {
            Ok(val(Value::Null))
        }
    }
}
fn run() -> Result<()> {
    let mut checkout = None;
    let mut evidence = None;
    let mut expected = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let v = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--checkout" => checkout = Some(PathBuf::from(v)),
            "--evidence" => evidence = Some(PathBuf::from(v)),
            "--expected-commit" => expected = Some(v),
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    let checkout = checkout.ok_or("--checkout required")?.canonicalize()?;
    let evidence = resolve(&evidence.ok_or("--evidence required")?)?;
    let expected = expected.ok_or("--expected-commit required")?;
    let target = PathBuf::from(std::env::var("CARGO_TARGET_DIR")?).canonicalize()?;
    require(
        [&evidence, &target]
            .iter()
            .all(|p| p.components().any(|c| c.as_os_str() == ".private")),
        "evidence and existing per-worker target must be private",
    )?;
    fs::create_dir_all(evidence.parent().ok_or("missing evidence parent")?)?;
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(&evidence)?;
    let mut w = Witness {
        checkout: checkout.clone(),
        evidence: evidence.clone(),
        commands: Vec::new(),
    };
    let commit = w.git(&["rev-parse", "HEAD"])?;
    require(
        commit == expected
            && w.git(&["status", "--porcelain", "--untracked-files=all"])?
                .is_empty(),
        "fresh clone must start at the exact clean accepted source",
    )?;
    let mut common = PathBuf::from(w.git(&["rev-parse", "--git-common-dir"])?);
    if !common.is_absolute() {
        common = checkout.join(common).canonicalize()?;
    }
    require(
        !common.join("objects/info/alternates").exists(),
        "fresh clone must have its own objects, no alternates",
    )?;
    let origin = w.git(&["remote", "get-url", "origin"])?;
    require(
        [
            "https://github.com/neverhuman/joil.git",
            "git@github.com:neverhuman/joil.git",
        ]
        .contains(&origin.as_str()),
        "Joil origin required",
    )?;
    let mut cores = BTreeSet::new();
    for cpu in crucible_runtime::lease::current_affinity()? {
        let top = PathBuf::from(format!("/sys/devices/system/cpu/cpu{cpu}/topology"));
        cores.insert((
            fs::read_to_string(top.join("physical_package_id"))?
                .trim()
                .to_string(),
            fs::read_to_string(top.join("core_id"))?.trim().to_string(),
        ));
    }
    let capability = obj(vec![
        ("physical_cores_available", val(cores.len())),
        ("workers_requested", val(2)),
        (
            "workers_exercised",
            val(if cores.len() >= 2 { 2 } else { 0 }),
        ),
        ("source_commit", val(commit.clone())),
        ("no_gpu", val(true)),
    ]);
    write(evidence.join("worker-capability.json"), &capability)?;
    require(
        cores.len() >= 2,
        "shipped two-worker fixture requires two leased physical cores; no substitution",
    )?;
    let campaign = "campaigns/fixture-v5.toml";
    let initial = obj(vec![
        ("commit", val(commit.clone())),
        ("tree", val(w.git(&["rev-parse", "HEAD^{tree}"])?)),
        ("common_dir", val(common.to_string_lossy().to_string())),
        ("origin", val(origin)),
        ("no_object_alternates", val(true)),
        ("initial_status_clean", val(true)),
        ("target_reused", val(target.to_string_lossy().to_string())),
        (
            "campaign_sha256",
            val(sha(fs::read(checkout.join(campaign))?)),
        ),
    ]);
    write(evidence.join("source.json"), &initial)?;
    let whole = w.cli(&["run", campaign], 0)?;
    let partial = w.cli(&["run", campaign, "--until", "1"], 0)?;
    let p = value(&partial)?;
    let root = p["root"].as_str().ok_or("missing partial root")?;
    let resumed = w.cli(&["run", campaign, "--resume", root], 0)?;
    let verified = w.cli(&["checkpoint", "verify", root], 0)?;
    let status = w.cli(&["status", root], 0)?;
    let a = value(&whole)?;
    let b = value(&resumed)?;
    let v = value(&verified)?;
    require(
        a["epochs"] == 2 && b["epochs"] == 2 && p["epochs"] == 1,
        "epoch mismatch",
    )?;
    require(
        a["accounting"]["charges"] == 4
            && b["accounting"]["charges"] == 4
            && p["accounting"]["charges"] == 2,
        "charge mismatch",
    )?;
    require(
        a["accounting"]["exceeded"] == false && b["accounting"]["exceeded"] == false,
        "accounting exceeded",
    )?;
    require(
        a["logical_digest"].is_string()
            && a["logical_digest"] == b["logical_digest"]
            && a["logical_digest"] == v["logical_digest"],
        "logical replay mismatch",
    )?;
    require(
        v["offline"] == true && v["physical_eligibility"] == "Unsupported",
        "offline/physical boundary mismatch",
    )?;
    let text = fs::read_to_string(checkout.join(campaign))?;
    require(
        text.matches("workers = 2").count() == 1,
        "two-worker fixture required",
    )?;
    let one_config = evidence.join("one-worker.toml");
    fs::write(&one_config, text.replace("workers = 2", "workers = 1"))?;
    let one = w.cli(
        &["run", one_config.to_str().ok_or("non-UTF8 config path")?],
        0,
    )?;
    let o = value(&one)?;
    require(
        o["epochs"] == 2 && o["accounting"]["charges"] == 4 && o["accounting"]["exceeded"] == false,
        "one-worker accounting mismatch",
    )?;
    require(
        o["logical_digest"] == a["logical_digest"]
            && o["actual_cost_digest"].is_string()
            && a["actual_cost_digest"].is_string()
            && o["actual_cost_digest"] != a["actual_cost_digest"],
        "worker replay/cost mismatch",
    )?;
    let bad = evidence.join("invalid-physical.toml");
    fs::write(
        &bad,
        text.replace("synthetic_integer_fixture", "physical_solver"),
    )?;
    let refused = evidence.join("must-not-exist");
    w.cli(
        &[
            "run",
            bad.to_str().ok_or("non-UTF8 config path")?,
            "--root",
            refused.to_str().ok_or("non-UTF8 root path")?,
        ],
        64,
    )?;
    require(!refused.exists(), "invalid input created root")?;
    require(
        fs::read_to_string(evidence.join(format!("{:02}.stderr", w.commands.len() - 1)))?
            .contains("bounded_software_campaign_admission"),
        "missing admission refusal",
    )?;
    require(
        w.git(&["status", "--porcelain", "--untracked-files=all"])?
            .is_empty(),
        "checkout changed",
    )?;
    let report = obj(vec![
        ("source", initial),
        ("worker_capability", capability),
        ("whole", whole),
        ("one_worker_whole", one),
        ("worker_counts_actually_exercised", val(json!([1, 2]))),
        ("partial", partial),
        ("resumed", resumed),
        ("verified", verified),
        ("status", status),
        (
            "invalid_physical_input_refused_before_root_creation",
            val(true),
        ),
        ("final_checkout_clean", val(true)),
        ("physical_qualification", val(false)),
        (
            "limitations",
            val(json!([
                "Synthetic integer fixture only; existing toolchain/cache reused",
                "No dependency reinstall or physical solver/GPU execution",
                "Native attempts use declared runtime accounting scope"
            ])),
        ),
    ]);
    write(evidence.join("report.json"), &report)?;
    let mut inventory = Vec::new();
    for p in files(&evidence)? {
        let raw = fs::read(&p)?;
        inventory.push(obj(vec![
            (
                "path",
                val(p.strip_prefix(&evidence)?.to_string_lossy().to_string()),
            ),
            ("sha256", val(sha(&raw))),
            ("bytes", val(raw.len())),
        ]));
    }
    write(evidence.join("inventory.json"), &Ordered::Array(inventory))?;
    let summary = obj(vec![
        ("source_commit", val(commit)),
        ("commands", val(w.commands.len())),
        ("charges_uninterrupted", val(4)),
        ("charges_partial", val(2)),
        ("charges_resumed", val(4)),
        ("logical_replay_equal", val(true)),
        ("offline_verified", val(true)),
        (
            "whole_worker1_worker2_logical_equal_cost_digest_distinct",
            val(true),
        ),
        ("invalid_input_refused", val(true)),
        ("physical_qualification", val(false)),
        (
            "inventory_sha256",
            val(sha(fs::read(evidence.join("inventory.json"))?)),
        ),
    ]);
    // Python's default separators, preserving insertion order.
    if let Ordered::Object(fields) = summary {
        println!(
            "{{{}}}",
            fields
                .iter()
                .map(|(k, v)| format!("{}: {}", json!(k), v.encode(false)))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
