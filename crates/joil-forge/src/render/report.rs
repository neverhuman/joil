//! Report orchestration: design figures, CT-35 control and the run dashboard.

use super::html::{dashboard, MeshPayload};
use super::svg::{comparison_chart, evolution_chart, grade_chart, pfd};
use super::views::{plate, thumbnail};
use crate::cad::{self, write_outputs};
use crate::campaign::Campaign;
use crate::checkpoint;
use crate::evolve::Individual;
use crate::genome::Genome;
use crate::random::archetypes;
use crate::score::Evaluator;
use joil_forge_judges::capacity::capacity;
use joil_forge_judges::cyclone::Resolution;
use joil_forge_judges::network::{evaluate, PlantEvaluation};
use joil_forge_judges::reference::ct35_plant;
use std::path::{Path, PathBuf};

fn write(path: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    std::fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path.to_path_buf())
}

/// Evaluation at the capacity point (or the reference flow) at report resolution.
pub fn reference_evaluation(
    g: &Genome,
    c: &Campaign,
) -> Result<
    (
        crate::score::Score,
        crate::develop::Phenotype,
        Option<PlantEvaluation>,
    ),
    String,
> {
    let ev = Evaluator::new(c);
    let (score, ph, cap) = ev.evaluate_full(g, Resolution::report());
    let ph = ph.ok_or_else(|| score.reasons.join("; "))?;
    let at = match cap.and_then(|cap| cap.reference) {
        Some(e) => Some(e),
        None => evaluate(&ph.plant, &c.feed, &ev.settings(0, Resolution::report())).ok(),
    };
    Ok((score, ph, at))
}

/// Window of the head region for detail views (mm from the top face).
fn head_window(ph: &crate::develop::Phenotype) -> (f64, f64) {
    let first = ph.layout.bays.first().copied().unwrap_or([0.0, 200.0]);
    let span = (first[1] - 0.0).clamp(120.0, 260.0);
    (0.0, span.min(ph.layout.length_mm))
}

pub fn design_figures(
    g: &Genome,
    c: &Campaign,
    out: &Path,
    voxel: f64,
) -> Result<Vec<PathBuf>, String> {
    let (score, ph, at) = reference_evaluation(g, c)?;
    let meshed = cad::build(g, c, voxel)?;
    let mut files = Vec::new();
    let img = plate(
        &meshed.fluid,
        &meshed.part,
        meshed.report.length_mm,
        meshed.report.envelope_radius_mm,
        head_window(&ph),
    );
    files.push(write(&out.join("plate.png"), &img.png())?);
    files.push(write(
        &out.join("pfd.svg"),
        pfd(&ph, at.as_ref(), Some(&score), "Evolved process network").as_bytes(),
    )?);
    if let Some(e) = &at {
        files.push(write(
            &out.join("grade.svg"),
            grade_chart(e, &joil_forge_judges::sizes::SizeGrid::standard()).as_bytes(),
        )?);
        files.push(write(
            &out.join("evaluation.json"),
            &serde_json::to_vec_pretty(e).map_err(|e| e.to_string())?,
        )?);
    }
    files.push(write(
        &out.join("score.json"),
        &serde_json::to_vec_pretty(&score).map_err(|e| e.to_string())?,
    )?);
    files.push(write(
        &out.join("cad-audit.json"),
        &serde_json::to_vec_pretty(&meshed.report).map_err(|e| e.to_string())?,
    )?);
    Ok(files)
}

/// Same-model CT-35 controls: the judges' liner-only reconstruction and the forge
/// archetype with plenums, channels and envelope gates.
pub fn baseline_json(c: &Campaign) -> Result<serde_json::Value, String> {
    let ev = Evaluator::new(c);
    let plant = ct35_plant(0.02);
    let mut per_closure = Vec::new();
    for k in 0..ev.closures.len() {
        let cap = capacity(
            &plant,
            &c.feed,
            &ev.settings(k, Resolution::report()),
            &c.limits,
        )
        .map_err(|e| e.to_string())?;
        per_closure.push(serde_json::json!({
            "closure": k,
            "q_pressure_limit_m3_h": cap.q_pressure_limit_m3_h,
            "q_star_m3_h": cap.q_star_m3_h,
            "cleanest_probe_cout_mg_l": cap.reference.as_ref().map(|r| r.cout_mg_l),
            "shortfall": cap.shortfall,
        }));
    }
    let at4 = evaluate(&plant, &c.feed, &ev.settings(0, Resolution::report()))
        .map_err(|e| e.to_string())?;
    let arche = archetypes()
        .into_iter()
        .find(|(n, _)| *n == "ct35")
        .map(|(_, g)| g)
        .expect("ct35 archetype");
    let (score, _, _) = ev.evaluate_full(&arche, Resolution::report());
    Ok(serde_json::json!({
        "evidence_class": joil_forge_judges::EVIDENCE_CLASS,
        "model_version": joil_forge_judges::MODEL_VERSION,
        "liner_only": {
            "at_reference_flow": {
                "flow_m3_h": c.feed.flow_m3_h,
                "cout_mg_l": at4.cout_mg_l,
                "oil_capture": at4.oil_capture,
                "feed_pressure_pa": at4.feed_pressure_pa,
                "reduced_d50_um": at4.units.first().and_then(|u| u.reduced_d50_um),
                "dp_underflow_pa": at4.units.first().map(|u| u.port_drop_pa[1]),
                "dp_overflow_pa": at4.units.first().map(|u| u.port_drop_pa[0]),
            },
            "capacity_by_closure": per_closure,
        },
        "forge_archetype": score,
    }))
}

fn pick_gallery(cp: &checkpoint::Checkpoint, top: usize) -> Vec<Individual> {
    let mut elites: Vec<Individual> = cp.state.archive.elites().into_iter().cloned().collect();
    elites.sort_by(|a, b| a.score.compare(&b.score).then_with(|| a.hash.cmp(&b.hash)));
    elites.truncate(top);
    elites
}

/// `voxel: None` picks [`cad::auto_voxel`] for the best design.
/// `control` is an equal-budget random-search report for the same campaign (optional).
pub fn run_report(
    run: &Path,
    out: &Path,
    voxel: Option<f64>,
    top: usize,
    control: Option<&Path>,
) -> Result<PathBuf, String> {
    let cp = checkpoint::load(run)?;
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let c = &cp.campaign;
    let control: Option<crate::control::ControlReport> = match control {
        Some(p) => {
            let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
            let r: crate::control::ControlReport =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))?;
            if r.campaign_sha256 != c.sha256() {
                return Err("the control was run with a different campaign".into());
            }
            Some(r)
        }
        None => None,
    };
    let best = cp.state.best.clone().ok_or("run has no best design")?;
    let voxel = match voxel {
        Some(v) => v,
        None => cad::auto_voxel(&best.genome, c)?,
    };
    let (score, ph, at) = reference_evaluation(&best.genome, c)?;
    let meshed = cad::build(&best.genome, c, voxel)?;
    let best_dir = out.join("best");
    std::fs::create_dir_all(&best_dir).map_err(|e| e.to_string())?;
    write_outputs(&best.genome, &meshed, &best_dir)?;
    let plate_img = plate(
        &meshed.fluid,
        &meshed.part,
        meshed.report.length_mm,
        meshed.report.envelope_radius_mm,
        head_window(&ph),
    );
    write(&out.join("best-plate.png"), &plate_img.png())?;
    let pfd_svg = pfd(
        &ph,
        at.as_ref(),
        Some(&score),
        "Best design: process network",
    );
    write(&out.join("best-pfd.svg"), pfd_svg.as_bytes())?;
    let grade_svg = at
        .as_ref()
        .map(|e| grade_chart(e, &joil_forge_judges::sizes::SizeGrid::standard()))
        .unwrap_or_default();
    write(&out.join("best-grade.svg"), grade_svg.as_bytes())?;
    let mut evo_svg = evolution_chart(&cp.state.history, &format!("Search progress: {}", c.name));
    write(&out.join("evolution.svg"), evo_svg.as_bytes())?;
    if let Some(r) = &control {
        let hist = &cp.state.history;
        let evo_fit: Vec<(f64, f64)> = hist
            .iter()
            .filter_map(|g| g.best_fitness.map(|f| (g.evaluations as f64, f)))
            .collect();
        let ctl_fit: Vec<(f64, f64)> = r
            .trajectory
            .iter()
            .filter_map(|t| t.best_fitness.map(|f| (t.evaluations as f64, f)))
            .collect();
        let evo_q: Vec<(f64, f64)> = hist
            .iter()
            .map(|g| (g.evaluations as f64, g.archive_feasible as f64))
            .collect();
        let ctl_q: Vec<(f64, f64)> = r
            .trajectory
            .iter()
            .map(|t| (t.evaluations as f64, t.feasible_niches as f64))
            .collect();
        let cmp = comparison_chart(
            "Search versus random sampling at equal budget",
            "Evaluations. Random sampling draws from the generation-0 sampler; both use the same evaluator.",
            &[
                (
                    "Best weighted fitness found so far (drawn once feasible)",
                    vec![("search", evo_fit), ("random", ctl_fit)],
                ),
                (
                    "Feasible archive niches",
                    vec![("search", evo_q), ("random", ctl_q)],
                ),
            ],
        );
        write(&out.join("search-vs-random.svg"), cmp.as_bytes())?;
        evo_svg.push_str(&cmp);
    }
    if let Some(e) = &at {
        write(
            &out.join("best-evaluation.json"),
            &serde_json::to_vec_pretty(e).map_err(|e| e.to_string())?,
        )?;
    }
    // Gallery of distinct niches (the "zoo"), also stacked into one plate.
    let mut gallery = Vec::new();
    let mut thumbs = Vec::new();
    for (k, ind) in pick_gallery(&cp, top).iter().enumerate() {
        let Ok(m) = cad::build(&ind.genome, c, (voxel * 1.5).max(1.2)) else {
            continue;
        };
        let img = thumbnail(
            &m.fluid,
            &m.part,
            m.report.length_mm,
            m.report.envelope_radius_mm,
            900,
            150,
        );
        let name = format!("zoo-{k}.png");
        let png_bytes = img.png();
        write(&out.join(&name), &png_bytes)?;
        thumbs.push(img);
        let (s2, p2, a2) = reference_evaluation(&ind.genome, c)?;
        let svg = pfd(
            &p2,
            a2.as_ref(),
            Some(&s2),
            &format!(
                "Niche {}",
                ind.behavior
                    .as_ref()
                    .map(crate::archive::niche)
                    .unwrap_or_default()
            ),
        );
        let svg_name = format!("zoo-{k}.svg");
        write(&out.join(&svg_name), svg.as_bytes())?;
        let uri = format!("data:image/png;base64,{}", super::html::base64(&png_bytes));
        gallery.push((uri, svg, ind.clone()));
    }
    if !thumbs.is_empty() {
        let mut zoo = super::raster::Image::filled(900, 150 * thumbs.len(), [8, 10, 13]);
        for (k, t) in thumbs.iter().enumerate() {
            zoo.blit(t, 0, 150 * k);
        }
        write(&out.join("zoo.png"), &zoo.png())?;
    }
    // Interactive viewer mesh: coarsen until the page stays small (≈ 450k triangles).
    let mut vv = (voxel * 1.6).max(1.4);
    let mut coarse = cad::build(&best.genome, c, vv)?;
    for _ in 0..4 {
        if coarse.fluid.f.len() + coarse.part.f.len() <= 450_000 {
            break;
        }
        vv *= 1.3;
        coarse = cad::build(&best.genome, c, vv)?;
    }
    let payload = MeshPayload::new(&coarse.fluid, &coarse.part, coarse.report.length_mm);
    let html = dashboard(
        &cp,
        &best,
        &score,
        &meshed.report,
        &payload,
        &pfd_svg,
        &grade_svg,
        &evo_svg,
        &gallery,
    );
    let index = out.join("index.html");
    write(&index, html.as_bytes())?;
    write_receipt(
        &cp,
        &best,
        &score,
        &meshed.report,
        &gallery,
        control.as_ref(),
        out,
    )?;
    Ok(index)
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

/// Machine-readable summary of a run report, with digests of every figure it wrote.
fn write_receipt(
    cp: &checkpoint::Checkpoint,
    best: &Individual,
    score: &crate::score::Score,
    cad: &crate::cad::CadReport,
    gallery: &[(String, String, Individual)],
    control: Option<&crate::control::ControlReport>,
    out: &Path,
) -> Result<(), String> {
    let mut figures = serde_json::Map::new();
    let mut names: Vec<_> = std::fs::read_dir(out)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "png" || x == "svg"))
        .collect();
    names.sort();
    for p in names {
        let bytes = std::fs::read(&p).map_err(|e| e.to_string())?;
        figures.insert(
            p.file_name().unwrap().to_string_lossy().into_owned(),
            serde_json::json!(sha256_hex(&bytes)),
        );
    }
    let elites: Vec<serde_json::Value> = gallery
        .iter()
        .map(|(_, _, i)| {
            serde_json::json!({
                "niche": i.behavior.as_ref().map(crate::archive::niche),
                "genome_sha256": i.hash,
                "feasible": i.score.feasible,
                "q_treated_m3_h": i.score.q_treated_m3_h,
                "cout_mg_l": i.score.cout_mg_l,
                "units": i.score.units,
                "behavior": i.behavior,
            })
        })
        .collect();
    let receipt = serde_json::json!({
        "evidence_class": joil_forge_judges::EVIDENCE_CLASS,
        "model_version": joil_forge_judges::MODEL_VERSION,
        "forge_version": crate::campaign::FORGE_VERSION,
        "manifest": cp.manifest,
        "campaign": cp.campaign,
        "generation": cp.state.generation,
        "evaluations": cp.state.evaluations,
        "archive_niches": cp.state.archive.cells.len(),
        "archive_feasible": cp.state.archive.feasible(),
        "best": {"genome_sha256": best.hash, "genome": best.genome, "behavior": best.behavior, "search_score": best.score, "report_score": score},
        "gallery": elites,
        "cad_best": cad,
        "history_last": cp.state.history.last(),
        "ct35_same_model": baseline_json(&cp.campaign)?,
        "random_search_control": control.map(|r| serde_json::json!({
            "kind": r.kind,
            "campaign_sha256": r.campaign_sha256,
            "evaluations": r.evaluations,
            "feasible_designs": r.feasible,
            "final": r.trajectory.last(),
            "best_genome_sha256": r.best.as_ref().map(|b| b.hash.clone()),
            "best_score": r.best.as_ref().map(|b| b.score.clone()),
        })),
        "figures_sha256": figures,
        "note": "Screen-model search output. Not CFD, not a measurement, not physical validation.",
    });
    write(
        &out.join("receipt.json"),
        &serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?,
    )?;
    Ok(())
}
