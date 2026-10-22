//! `joil-forge` command line.
//!
//! ```text
//! joil-forge evolve   --run DIR [--campaign FILE] [--generations N] [--resume [--fork]] [--seed-from PATH] [--threads N]
//! joil-forge eval     --genome FILE [--campaign FILE] [--report]
//! joil-forge baseline [--campaign FILE]
//! joil-forge cad      --genome FILE --out DIR [--campaign FILE] [--voxel MM]
//! joil-forge render   --genome FILE --out DIR [--campaign FILE] [--voxel MM]
//! joil-forge report   --run DIR [--out DIR] [--voxel MM] [--top N]
//! joil-forge seeds    --out DIR
//! joil-forge best     --run DIR
//! ```

use joil_forge::genome::Genome;
use joil_forge::run::{self, load_campaign, Options};
use joil_forge::score::Evaluator;
use joil_forge_judges::cyclone::Resolution;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn usage() -> ! {
    eprintln!("{}", include_str!("usage.txt"));
    std::process::exit(2)
}

struct Args {
    cmd: String,
    kv: BTreeMap<String, String>,
    flags: Vec<String>,
}

fn parse() -> Args {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_else(|| usage());
    let (mut kv, mut flags) = (BTreeMap::new(), Vec::new());
    let rest: Vec<String> = it.collect();
    let mut i = 0;
    while i < rest.len() {
        let a = &rest[i];
        let Some(name) = a.strip_prefix("--") else {
            usage()
        };
        let takes_value = !matches!(name, "resume" | "fork" | "report" | "quiet");
        if takes_value {
            let v = rest.get(i + 1).cloned().unwrap_or_else(|| usage());
            kv.insert(name.to_string(), v);
            i += 2;
        } else {
            flags.push(name.to_string());
            i += 1;
        }
    }
    Args { cmd, kv, flags }
}

fn path(a: &Args, k: &str) -> Option<PathBuf> {
    a.kv.get(k).map(PathBuf::from)
}

fn need(a: &Args, k: &str) -> PathBuf {
    path(a, k).unwrap_or_else(|| {
        eprintln!("missing --{k}");
        usage()
    })
}

fn read_genome(p: &Path) -> Result<Genome, String> {
    let text = std::fs::read_to_string(p).map_err(|e| format!("read {}: {e}", p.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", p.display()))
}

fn main() {
    let a = parse();
    if let Some(n) = a.kv.get("threads") {
        let n: usize = n.parse().unwrap_or_else(|_| usage());
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .expect("thread pool");
    }
    let result = match a.cmd.as_str() {
        "evolve" => run::run(&Options {
            campaign: path(&a, "campaign"),
            run_dir: need(&a, "run"),
            generations: a
                .kv
                .get("generations")
                .map(|g| g.parse().unwrap_or_else(|_| usage()))
                .unwrap_or(50),
            resume: a.flags.iter().any(|f| f == "resume"),
            fork: a.flags.iter().any(|f| f == "fork"),
            seed_from: path(&a, "seed-from"),
            quiet: a.flags.iter().any(|f| f == "quiet"),
        })
        .map(|_| ()),
        "eval" => (|| {
            let c = load_campaign(path(&a, "campaign").as_deref())?;
            let g = read_genome(&need(&a, "genome"))?;
            let ev = Evaluator::new(&c);
            let (score, ph, cap) = ev.evaluate_full(&g, Resolution::report());
            let out = if a.flags.iter().any(|f| f == "report") {
                serde_json::json!({"score": score, "phenotype": ph, "capacity": cap})
            } else {
                serde_json::json!({"score": score, "behavior": ph.map(|p| p.behavior)})
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?
            );
            Ok(())
        })(),
        "baseline" => (|| {
            let c = load_campaign(path(&a, "campaign").as_deref())?;
            let out = joil_forge::render::report::baseline_json(&c)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?
            );
            Ok(())
        })(),
        "cad" | "render" => (|| {
            let c = load_campaign(path(&a, "campaign").as_deref())?;
            let g = read_genome(&need(&a, "genome"))?;
            let voxel: f64 = match a.kv.get("voxel") {
                Some(v) => v.parse().unwrap_or_else(|_| usage()),
                None => joil_forge::cad::auto_voxel(&g, &c)?,
            };
            let out = need(&a, "out");
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            let files = if a.cmd == "cad" {
                joil_forge::cad::export(&g, &c, &out, voxel)?
            } else {
                joil_forge::render::report::design_figures(&g, &c, &out, voxel)?
            };
            for f in files {
                println!("{}", f.display());
            }
            Ok(())
        })(),
        "report" => (|| {
            let run = need(&a, "run");
            let out = path(&a, "out").unwrap_or_else(|| run.join("report"));
            let voxel: Option<f64> =
                a.kv.get("voxel")
                    .map(|v| v.parse().unwrap_or_else(|_| usage()));
            let top: usize =
                a.kv.get("top")
                    .map(|v| v.parse().unwrap_or_else(|_| usage()))
                    .unwrap_or(6);
            let control = path(&a, "control");
            let index =
                joil_forge::render::report::run_report(&run, &out, voxel, top, control.as_deref())?;
            println!("{}", index.display());
            Ok(())
        })(),
        "seeds" => (|| {
            let out = need(&a, "out");
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            for (name, g) in joil_forge::random::archetypes() {
                let p = out.join(format!("{name}.json"));
                std::fs::write(
                    &p,
                    serde_json::to_vec_pretty(&g).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                println!("{}", p.display());
            }
            Ok(())
        })(),
        "control" => (|| {
            let c = load_campaign(path(&a, "campaign").as_deref())?;
            let budget: u64 =
                a.kv.get("evaluations")
                    .map(|v| v.parse().unwrap_or_else(|_| usage()))
                    .unwrap_or((c.population * c.islands * 300) as u64);
            let out = need(&a, "out");
            let report = joil_forge::control::random_search(&c, budget, c.population * c.islands);
            std::fs::write(
                &out,
                serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let best = report.best.as_ref().map(|b| &b.score);
            println!(
                "{} random designs: {} feasible; best fitness {}; best treated capacity {:.2} m3/h",
                report.evaluations,
                report.feasible,
                best.filter(|s| s.feasible)
                    .map(|s| format!("{:.3}", s.fitness))
                    .unwrap_or_else(|| "none (no feasible design)".into()),
                best.map(|s| s.q_treated_m3_h).unwrap_or(0.0)
            );
            Ok(())
        })(),
        "campaign" => (|| {
            let c = joil_forge::campaign::Campaign::default();
            println!(
                "{}",
                serde_json::to_string_pretty(&c).map_err(|e| e.to_string())?
            );
            Ok(())
        })(),
        "best" => (|| {
            let cp = joil_forge::checkpoint::load(&need(&a, "run"))?;
            let best = cp.state.best.ok_or("run has no best design")?;
            println!(
                "{}",
                serde_json::to_string_pretty(&best.genome).map_err(|e| e.to_string())?
            );
            Ok(())
        })(),
        "fleet-summary" => (|| {
            let root = need(&a, "root");
            let summary = joil_forge::fleet::summarize(&root)?;
            std::fs::write(
                need(&a, "out"),
                serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if let Some(p) = path(&a, "svg") {
                std::fs::write(p, joil_forge::fleet::chart(&summary)).map_err(|e| e.to_string())?;
            }
            if let Some(p) = path(&a, "pool") {
                let genomes = joil_forge::fleet::pool(&root)?;
                std::fs::write(p, serde_json::to_vec(&genomes).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            }
            let t = &summary.search_vs_random;
            println!(
                "{} replicates: search beat random in {}, random beat search in {}, {} tied or not comparable (one-sided sign test p = {:.4})",
                t.replicates, t.first_better, t.second_better, t.ties_or_not_comparable, t.p_one_sided
            );
            Ok(())
        })(),
        _ => usage(),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
