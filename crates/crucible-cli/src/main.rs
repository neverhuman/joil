use crucible_cli::*;
use crucible_runtime::{Result, RuntimeError};
use std::path::{Path, PathBuf};
fn error() -> RuntimeError {
    RuntimeError::Invalid("usage: explore verify FILE --revision REV --subject ID | admit FILE | run FILE [--root FRESH | --resume ROOT] [--until N] [--continue] | status ROOT | stop ROOT | checkpoint verify ROOT | fork ROOT --weights A,B | seed-import FILE")
}
fn main_command(args: &[String]) -> Result<i32> {
    if args.first().map(String::as_str) == Some("--fixture-child") {
        if args.len() != 2 {
            return Err(error());
        }
        return crucible_runtime::task::fixture_child(Path::new(&args[1]));
    }
    install_interrupt();
    let first = args.first().ok_or_else(error)?.as_str();
    let output = match first {
        "explore"
            if args.len() == 7
                && args[1] == "verify"
                && args[3] == "--revision"
                && args[5] == "--subject" =>
        {
            explore::inspect(Path::new(&args[2]), &args[4], &args[6])?
        }
        "admit" if args.len() == 2 => {
            let c = Campaign::load(Path::new(&args[1]))?;
            serde_json::json!({"admitted":true,"synthetic_only":true,"physical_eligibility":"Unsupported","campaign":c})
        }
        "run" if args.len() >= 2 => {
            let c = Campaign::load(Path::new(&args[1]))?;
            let mut root = None;
            let mut resume = false;
            let mut until = c.epochs;
            let mut continue_stop = false;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--root" | "--resume" if i + 1 < args.len() && root.is_none() => {
                        resume = args[i] == "--resume";
                        root = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    }
                    "--until" if i + 1 < args.len() => {
                        until = args[i + 1].parse().map_err(|_| error())?;
                        i += 2;
                    }
                    "--continue" if !continue_stop => {
                        continue_stop = true;
                        i += 1;
                    }
                    _ => return Err(error()),
                }
            }
            let root = match root {
                Some(r) => r,
                None => default_root()?,
            };
            let result = run(&c, &root, resume, until, continue_stop)?;
            let stopped = result.stopped;
            println!(
                "{}",
                serde_json::to_string(&result).map_err(|e| RuntimeError::Wire(e.to_string()))?
            );
            return Ok(if stopped { 130 } else { 0 });
        }
        "status" if args.len() == 2 => status(Path::new(&args[1]))?,
        "stop" if args.len() == 2 => {
            stop(Path::new(&args[1]))?;
            serde_json::json!({"stop_requested":true})
        }
        "checkpoint" if args.len() == 3 && args[1] == "verify" => verify(Path::new(&args[2]))?,
        "fork" if args.len() == 4 && args[2] == "--weights" => {
            let w = args[3]
                .split(',')
                .map(str::parse::<i64>)
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|_| error())?;
            let w: [i64; 2] = w.try_into().map_err(|_| error())?;
            fork(Path::new(&args[1]), w)?
        }
        "seed-import" if args.len() == 2 => {
            let seed = crucible_runtime::replay::import_seed(&read_private_or_source(
                Path::new(&args[1]),
                16384,
            )?)?;
            serde_json::json!({"synthetic_only":true,"compiled_seed":seed,"inherited_verdicts":false})
        }
        _ => return Err(error()),
    };
    println!(
        "{}",
        serde_json::to_string(&output).map_err(|e| RuntimeError::Wire(e.to_string()))?
    );
    Ok(0)
}
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let code = match main_command(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            match e {
                RuntimeError::Invalid(_) | RuntimeError::Wire(_) => 64,
                RuntimeError::Conflict(_)
                | RuntimeError::Budget(_)
                | RuntimeError::Unsupported(_)
                | RuntimeError::ExpiredLease => 78,
                _ => 74,
            }
        }
    };
    std::process::exit(code);
}
