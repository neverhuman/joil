use crucible_paper::PaperBundle;
use std::{env, fs, process};

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "paper/fixtures/negative-result.json".into());
    let bytes = fs::read(&path).unwrap_or_else(|error| {
        eprintln!("cannot read paper bundle {path}: {error}");
        process::exit(2);
    });
    let bundle: PaperBundle = serde_json::from_slice(&bytes).unwrap_or_else(|error| {
        eprintln!("invalid paper bundle: {error}");
        process::exit(1);
    });
    if let Err(error) = bundle.validate() {
        eprintln!("paper query failed: {error}");
        process::exit(1);
    }
    println!("# Reproducible research result\n\nThis generated paper uses {} exact typed observation query/queries and {} source-bound figure manifest(s). All values are synthetic fixture evidence; no physical performance claim is made.\n", bundle.queries.len(), bundle.figures.len());
    for query in bundle.queries {
        let row = bundle
            .observations
            .iter()
            .find(|o| o.id == query.observation_id)
            .expect("validated query");
        println!(
            "- `{}`: `{}` {} (denominator `{}`; tier `{}`). Interpretation: {}.",
            row.id, row.value, row.unit, row.denominator, row.tier, row.interpretation
        );
    }
    println!("\n## Cost-quality confirmation points\n");
    println!("Attempted finalists retained: {}. Confirmation data are explicitly labeled per row; independent units are listed once each.", bundle.attempted_finalist_ids.len());
    for point in bundle.cost_quality {
        println!("- `{}` / `{}`: cost {}, estimate {} [{}, {}], n={} independent units, outcome `{}` ({})",
            point.finalist_id, point.attempt_id, point.normalized_resource_cost,
            point.quality_estimate, point.uncertainty_low, point.uncertainty_high,
            point.independent_unit_ids.len(), point.outcome, point.confirmation_data_id);
    }
    println!("\nSeed/analysis package: `experiments/v5-study/preregistration-v1.json`.\n");
}
