//! Run orchestration: fresh start, exact resume, explicit fork, and seeding a new run from
//! the best designs of an earlier one.

use crate::campaign::{Campaign, Manifest};
use crate::checkpoint::{self, write_atomic, Checkpoint, FORMAT};
use crate::evolve::{init, rescore, step, Cache, Individual, RunState};
use crate::genome::Genome;
use crate::score::Evaluator;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub campaign: Option<PathBuf>,
    pub run_dir: PathBuf,
    /// Total generations to reach (a resumed run continues up to this number).
    pub generations: usize,
    pub resume: bool,
    pub fork: bool,
    pub seed_from: Option<PathBuf>,
    pub quiet: bool,
}

pub fn load_campaign(path: Option<&Path>) -> Result<Campaign, String> {
    let c = match path {
        Some(p) => {
            let text =
                std::fs::read_to_string(p).map_err(|e| format!("read {}: {e}", p.display()))?;
            serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", p.display()))?
        }
        None => Campaign::default(),
    };
    c.validate()?;
    Ok(c)
}

/// The designs a run hands on as seeds: archive elites, the best design and the four
/// leaders of each island, best first and deduplicated.
pub fn seed_individuals(cp: &Checkpoint) -> Vec<&Individual> {
    let mut inds: Vec<&Individual> = cp.state.archive.elites();
    inds.extend(cp.state.best.iter());
    for isl in &cp.state.islands {
        inds.extend(isl.pop.iter().take(4));
    }
    inds.sort_by(|a, b| a.score.compare(&b.score).then_with(|| a.hash.cmp(&b.hash)));
    let mut seen = std::collections::HashSet::new();
    inds.retain(|i| seen.insert(i.hash.clone()));
    inds
}

/// Genomes to seed from: a run directory or checkpoint (archive elites, best, island
/// leaders), or a JSON file holding one genome or a list of genomes.
pub fn load_seeds(path: &Path) -> Result<Vec<Genome>, String> {
    let is_ckpt = path.is_dir()
        || path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("checkpoint"));
    if is_ckpt {
        let cp = checkpoint::load(path)?;
        return Ok(seed_individuals(&cp)
            .into_iter()
            .map(|i| i.genome.clone())
            .collect());
    }
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if let Ok(g) = serde_json::from_str::<Genome>(&text) {
        return Ok(vec![g]);
    }
    serde_json::from_str::<Vec<Genome>>(&text)
        .map_err(|e| format!("parse genomes {}: {e}", path.display()))
}

fn write_best(run: &Path, best: &Individual) -> Result<(), String> {
    let dir = run.join("best");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    write_atomic(
        &dir.join("genome.json"),
        &serde_json::to_vec_pretty(&best.genome).map_err(|e| e.to_string())?,
    )?;
    write_atomic(
        &dir.join("individual.json"),
        &serde_json::to_vec_pretty(best).map_err(|e| e.to_string())?,
    )
}

fn log_line(run: &Path, state: &RunState) -> Result<(), String> {
    let Some(h) = state.history.last() else {
        return Ok(());
    };
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(run.join("history.jsonl"))
        .map_err(|e| e.to_string())?;
    writeln!(
        f,
        "{}",
        serde_json::to_string(h).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())
}

fn describe(state: &RunState) -> String {
    let b = state.best.as_ref().expect("best exists");
    let s = &b.score;
    format!(
        "gen {:>4} | evals {:>7} | best {} q_treated {:6.2} m3/h fit {:7.3} viol {:8.4} cout {} | niches {:>3} ({} feasible)",
        state.generation,
        state.evaluations,
        if s.feasible { "FEASIBLE  " } else { "infeasible" },
        s.q_treated_m3_h,
        s.fitness,
        s.violation,
        s.cout_mg_l.map(|c| format!("{c:7.2} mg/L")).unwrap_or_else(|| "      - mg/L".into()),
        state.archive.cells.len(),
        state.archive.feasible(),
    )
}

pub fn run(opts: &Options) -> Result<RunState, String> {
    let run = &opts.run_dir;
    let ckpt = checkpoint::checkpoint_path(run);
    let mut cache = Cache::default();
    let (campaign, manifest, mut state, forked) = if opts.resume {
        let cp = checkpoint::load(&ckpt)?;
        let campaign = match &opts.campaign {
            Some(p) => load_campaign(Some(p))?,
            None => cp.campaign.clone(),
        };
        campaign.validate()?;
        let fresh = Manifest::new(&campaign);
        let changed = fresh.identity() != cp.manifest.identity();
        let manifest = if !changed {
            cp.manifest.clone()
        } else if opts.fork {
            let parent_dir = run.join("lineage").join(cp.manifest.sha256());
            std::fs::create_dir_all(&parent_dir).map_err(|e| e.to_string())?;
            // Keep the exact parent bytes before replacing any manifest or checkpoint.
            write_atomic(
                &parent_dir.join("checkpoint.json"),
                &std::fs::read(&ckpt).map_err(|e| e.to_string())?,
            )?;
            let history = run.join("history.jsonl");
            if history.exists() {
                write_atomic(
                    &parent_dir.join("history.jsonl"),
                    &std::fs::read(&history).map_err(|e| e.to_string())?,
                )?;
            }
            Manifest {
                parent_manifest_sha256: Some(cp.manifest.sha256()),
                seeded_from: cp.manifest.seeded_from.clone(),
                ..fresh
            }
        } else {
            return Err("campaign or model version differs from the checkpoint manifest; pass --fork to continue as a new lineage".into());
        };
        (campaign, manifest, cp.state, changed)
    } else {
        if ckpt.exists() {
            return Err(format!(
                "{} already holds a run; use --resume, or choose a new --run directory",
                run.display()
            ));
        }
        let campaign = load_campaign(opts.campaign.as_deref())?;
        let mut manifest = Manifest::new(&campaign);
        let seeds = match &opts.seed_from {
            Some(p) => {
                manifest.seeded_from = Some(p.display().to_string());
                load_seeds(p)?
            }
            None => Vec::new(),
        };
        std::fs::create_dir_all(run).map_err(|e| format!("create {}: {e}", run.display()))?;
        let ev = Evaluator::new(&campaign);
        let state = init(&ev, &mut cache, seeds);
        (campaign, manifest, state, false)
    };
    let ev = Evaluator::new(&campaign);
    if forked {
        rescore(&ev, &mut cache, &mut state);
    }
    std::fs::create_dir_all(run).map_err(|e| e.to_string())?;
    write_atomic(
        &run.join("forge-campaign.json"),
        &serde_json::to_vec_pretty(&campaign).map_err(|e| e.to_string())?,
    )?;
    write_atomic(
        &run.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )?;
    let save = |state: &RunState, manifest: &Manifest| {
        checkpoint::save(
            run,
            &Checkpoint {
                format: FORMAT.into(),
                manifest: manifest.clone(),
                campaign: campaign.clone(),
                state: state.clone(),
            },
        )
    };
    if !opts.resume || forked {
        save(&state, &manifest)?;
        if forked {
            write_atomic(&run.join("history.jsonl"), b"")?;
        }
        if let Some(b) = &state.best {
            write_best(run, b)?;
        }
        if !opts.quiet {
            eprintln!("{}", describe(&state));
        }
    }
    let mut best_hash = state.best.as_ref().map(|b| b.hash.clone());
    while state.generation < opts.generations {
        step(&ev, &mut cache, &mut state);
        save(&state, &manifest)?;
        log_line(run, &state)?;
        let now = state.best.as_ref().map(|b| b.hash.clone());
        if now != best_hash {
            if let Some(b) = &state.best {
                write_best(run, b)?;
            }
            best_hash = now;
        }
        if !opts.quiet {
            eprintln!("{}", describe(&state));
        }
    }
    Ok(state)
}
