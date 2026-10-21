//! Compile a bounded 3D channel recipe to the shared implicit field format.
use joil_geometry::channel_network::ChannelNetwork;
use serde_json::json;
use std::io::{self, Read, Write};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args_os().len() != 1 {
        return Err(
            "usage: cargo run --locked -p joil-geometry --example channel_network < recipe.json"
                .into(),
        );
    }
    let mut input = Vec::new();
    io::stdin().take(256 * 1024 + 1).read_to_end(&mut input)?;
    if input.len() > 256 * 1024 {
        return Err("channel recipe exceeds the 256 KiB input budget".into());
    }
    let recipe: ChannelNetwork = serde_json::from_slice(&input)?;
    let compiled = recipe.compile()?;
    let result = json!({
        "schema_version": 1,
        "evidence_class": "unverified_implicit_geometry",
        "recipe_sha256": compiled.recipe_sha256,
        "field_sha256": compiled.fluid.program().identity()?,
        "declared_channels": compiled.declared_channels,
        "segment_count": compiled.segment_count,
        "fluid_field": compiled.fluid.program(),
        "limitations": ["independent topology and interface audit required", "mesh and manufacturing coverage not established", "no hydraulic or physical qualification"]
    });
    serde_json::to_writer_pretty(io::stdout().lock(), &result)?;
    io::stdout().write_all(b"\n")?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("channel network: {error}");
        std::process::exit(1);
    }
}
