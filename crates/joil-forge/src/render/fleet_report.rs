//! Fleet figures on disk: reads a fleet root, and optionally the winner's report and its
//! re-evaluations under further closure sets, and writes the evolution figure, the per-seed
//! panels and the winner card.

use super::fleet_figures::{evolution_figure, seed_panels, winner_card, Mark, Reference, Winner};
use crate::checkpoint;
use crate::fleet::summarize;
use std::path::{Path, PathBuf};

pub struct FigureOptions<'a> {
    pub marks: Vec<Mark>,
    pub references: Vec<Reference>,
    /// Output directory of `joil-forge report` for the fleet's winner.
    pub winner_report: Option<&'a Path>,
    /// Directory holding `eval --report` outputs of the winner under further closure sets.
    pub closures: Option<&'a Path>,
}

fn read(p: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(p).map_err(|e| format!("read {}: {e}", p.display()))
}

fn json(p: &Path) -> Result<serde_json::Value, String> {
    serde_json::from_slice(&read(p)?).map_err(|e| format!("parse {}: {e}", p.display()))
}

fn put(files: &mut Vec<PathBuf>, out: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    let p = out.join(name);
    std::fs::write(&p, bytes).map_err(|e| format!("write {}: {e}", p.display()))?;
    files.push(p);
    Ok(())
}

/// Feasible count, total and lowest feasible score (NaN if none) of the `eval --report`
/// outputs in `dir`; other JSON files there (such as the campaigns used) are skipped.
pub fn closure_summary(dir: &Path) -> Result<(usize, usize, f64), String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let (mut ok, mut n, mut worst) = (0, 0, f64::NAN);
    for p in files {
        let v = json(&p)?;
        let Some(s) = v.get("score") else {
            continue;
        };
        n += 1;
        if s.get("feasible").and_then(serde_json::Value::as_bool) == Some(true) {
            ok += 1;
            if let Some(f) = s.get("fitness").and_then(serde_json::Value::as_f64) {
                worst = if worst.is_nan() { f } else { worst.min(f) };
            }
        }
    }
    if n == 0 {
        return Err(format!("{}: no evaluations found", dir.display()));
    }
    Ok((ok, n, worst))
}

/// Write `evolution.svg`, `seeds.svg` and, given the winner's report, `winner.svg` to `out`.
/// The report must be of the fleet's best design.
pub fn write_figures(root: &Path, out: &Path, o: &FigureOptions) -> Result<Vec<PathBuf>, String> {
    let s = summarize(root)?;
    std::fs::create_dir_all(out).map_err(|e| format!("create {}: {e}", out.display()))?;
    let mut files = Vec::new();
    let evolution = evolution_figure(&s, &o.marks, &o.references);
    put(&mut files, out, "evolution.svg", evolution.as_bytes())?;
    put(
        &mut files,
        out,
        "seeds.svg",
        seed_panels(&s, &o.marks).as_bytes(),
    )?;
    if let Some(dir) = o.winner_report {
        let best = s
            .overall_best
            .as_ref()
            .ok_or("the fleet has no best design")?;
        let receipt = json(&dir.join("receipt.json"))?;
        let reported = receipt
            .pointer("/best/genome_sha256")
            .and_then(serde_json::Value::as_str);
        if reported != best.best_genome_sha256.as_deref() {
            return Err(format!(
                "{}: the report is not of the fleet's best design ({})",
                dir.display(),
                best.source
            ));
        }
        let cp = checkpoint::load(&root.join("runs").join(&best.source))?;
        let closures = o.closures.map(closure_summary).transpose()?;
        let pfd = String::from_utf8_lossy(&read(&dir.join("best-pfd.svg"))?).into_owned();
        let card = winner_card(&Winner {
            source: &best.source,
            receipt: &receipt,
            closures,
            hero_png: &read(&dir.join("best-hero.png"))?,
            pfd_svg: &pfd,
            history: &cp.state.history,
        });
        put(&mut files, out, "winner.svg", card.as_bytes())?;
    }
    Ok(files)
}
