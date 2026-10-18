//! Independent control: run the architecture/interchange suites without rtk.
use std::{
    env,
    os::unix::{fs::PermissionsExt, process::ExitStatusExt},
    path::PathBuf,
    process::Command,
};

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = Command::new(rustc).args(["--print", "sysroot"]).output()?;
    if !output.status.success() {
        return Err("rustc sysroot lookup failed".into());
    }
    let bin = PathBuf::from(std::str::from_utf8(&output.stdout)?.trim()).join("bin");
    let paths = [
        bin.clone(),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ];
    if paths.iter().any(|p| {
        p.join("rtk")
            .metadata()
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }) {
        return Err("control PATH unexpectedly has rtk".into());
    }
    let path = env::join_paths(paths)?;
    println!(
        "{}",
        serde_json::json!({"negative_control":"rtk is absent from child PATH","old_wrapper_would_not_start":true})
    );
    for args in [
        vec![
            "test",
            "--locked",
            "--offline",
            "-p",
            "crucible-architecture",
            "--test",
            "blind_boundary",
        ],
        vec![
            "test",
            "--locked",
            "--offline",
            "-p",
            "crucible-contract",
            "--test",
            "architecture",
            "--test",
            "interchange",
        ],
    ] {
        let cargo = bin.join("cargo");
        let status = Command::new(&cargo)
            .args(&args)
            .env("PATH", &path)
            .status()?;
        let code = status
            .code()
            .unwrap_or_else(|| -status.signal().unwrap_or(1));
        let argv: Vec<String> = std::iter::once(cargo.to_string_lossy().into_owned())
            .chain(args.iter().map(|s| s.to_string()))
            .collect();
        println!("{}", serde_json::json!({"argv":argv,"exit_code":code}));
        if code != 0 {
            return Ok(code);
        }
    }
    Ok(0)
}
fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("FAIL: {e}");
            1
        }
    };
    std::process::exit(code);
}
