//! Bounded stdin/stdout interface to the unqualified cohort accounting toolkit.
use pack_hydrocyclone_judges::process_network::{evaluate, NetworkRequest, Termination};
use std::io::{self, Read, Write};

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    if std::env::args_os().len() != 1 {
        return Err("usage: cargo run --locked -p pack-hydrocyclone-judges --example process_network < request.json".into());
    }
    let mut input = Vec::new();
    io::stdin()
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    if input.len() > 2 * 1024 * 1024 {
        return Err("network request exceeds the 2 MiB input budget".into());
    }
    let request: NetworkRequest = serde_json::from_slice(&input)?;
    let report = evaluate(&request)?;
    serde_json::to_writer_pretty(io::stdout().lock(), &report)?;
    io::stdout().write_all(b"\n")?;
    Ok(if report.termination == Termination::WorkBudgetExhausted {
        2
    } else {
        0
    })
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("process network: {error}");
            std::process::exit(1);
        }
    }
}
