use joil_core::{geometry, metrics};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = std::env::args().nth(1).unwrap_or_else(|| "baseline".into());
    match command.as_str() {
        "baseline" => {
            let g = geometry::baseline();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status":"analytic_geometry_only_not_cfd_validated",
                    "genome_sha256":g.sha256()?,"dimensions":g.derive()?,
                    "regression_example":{"cin_mg_l":500,"capture":0.94,"split":0.02,
                      "cout_mg_l":metrics::concentration_mg_l(500.0,0.94,0.02)?,"passes_30_mg_l":false}
                }))?
            );
        }
        _ => {
            return Err(
                "supported command: baseline; no CFD or evolution engine is implemented".into(),
            )
        }
    }
    Ok(())
}
