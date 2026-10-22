//! Fleet figures for the README and paper: every run's best score so far against evaluations
//! with the final score of every run and the winner marked, one panel per replicate seed, and
//! a winner card with the rendered design. Lines are best-so-far step functions drawn through
//! the points where they change, so they reproduce the logged trajectories exactly; evaluations
//! are on a log scale so that early and late search both show.

use super::charts::nice_ticks;
use super::html::base64;
use super::svg::{esc, style, BG, SERIES};
use crate::evolve::GenStats;
use crate::fleet::{ArmSummary, FleetSummary};
use std::fmt::Write;

/// Neutral stroke for random sampling: readable on light and dark backgrounds.
const GREY: &str = "#8a8984";

/// A vertical marker at a budget, such as the end of an earlier run.
pub struct Mark {
    pub evaluations: f64,
    pub label: String,
}

/// A horizontal reference score, such as the previous best design.
pub struct Reference {
    pub value: f64,
    pub label: String,
}

/// The points where a best-so-far trajectory changes value, plus its last point. Drawn as
/// steps they reproduce the trajectory at every logged budget.
pub fn change_points(t: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for (i, p) in t.iter().enumerate() {
        if out.last().is_none_or(|q| q.1 != p.1) || i + 1 == t.len() {
            out.push(*p);
        }
    }
    out
}

/// Value of a step trajectory at budget `x`: its last logged point at or before `x`.
pub fn value_at(t: &[(f64, f64)], x: f64) -> Option<f64> {
    t.iter().take_while(|p| p.0 <= x).last().map(|p| p.1)
}

/// Final feasible score of a run, if it found a feasible design.
fn final_fitness(a: &ArmSummary) -> Option<f64> {
    a.best.as_ref().filter(|b| b.feasible).map(|b| b.fitness)
}

fn median(v: &[f64]) -> Option<f64> {
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    let n = v.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(v[n / 2]),
        _ => Some(0.5 * (v[n / 2 - 1] + v[n / 2])),
    }
}

/// Integer with thousands separators.
fn sep(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

struct LogAxis {
    lo: f64,
    hi: f64,
    x0: f64,
    w: f64,
}

impl LogAxis {
    fn at(&self, x: f64) -> f64 {
        let t = (x.clamp(self.lo, self.hi).ln() - self.lo.ln()) / (self.hi.ln() - self.lo.ln());
        self.x0 + self.w * t
    }
    fn decades(&self) -> Vec<f64> {
        (0..12)
            .map(|k| 10f64.powi(k))
            .filter(|d| *d >= self.lo && *d <= self.hi)
            .collect()
    }
}

fn decade_label(x: f64) -> String {
    if x >= 1e6 {
        format!("{}M", x / 1e6)
    } else if x >= 1e3 {
        format!("{}k", x / 1e3)
    } else {
        format!("{x}")
    }
}

/// Linear score axis: `hi` at `top`, `lo` at `top + h` (or left to right when horizontal).
struct Lin {
    lo: f64,
    hi: f64,
    top: f64,
    h: f64,
}

impl Lin {
    fn at(&self, v: f64) -> f64 {
        self.top + self.h * (1.0 - ((v - self.lo) / (self.hi - self.lo)).clamp(0.0, 1.0))
    }
    fn across(&self, v: f64) -> f64 {
        self.top + self.h * ((v - self.lo) / (self.hi - self.lo)).clamp(0.0, 1.0)
    }
}

fn step_path(pts: &[(f64, f64)], x: &LogAxis, y: &Lin) -> String {
    let mut d = String::new();
    for (i, (a, b)) in pts.iter().enumerate() {
        let cmd = if i == 0 { "M" } else { "H" };
        let _ = write!(d, "{cmd}{:.1}", x.at(*a));
        if i == 0 {
            let _ = write!(d, ",{:.1}", y.at(*b));
        } else {
            let _ = write!(d, "V{:.1}", y.at(*b));
        }
    }
    d
}

/// One arm of the fleet: every replicate's run of it.
struct Arm<'a> {
    label: &'static str,
    short: &'static str,
    stroke: String,
    width: f64,
    runs: Vec<&'a ArmSummary>,
}

fn arms(s: &FleetSummary) -> [Arm<'_>; 3] {
    let pick = |f: fn(&crate::fleet::Replicate) -> &Option<ArmSummary>| {
        s.replicates.iter().filter_map(|r| f(r).as_ref()).collect()
    };
    [
        Arm {
            label: "Random sampling at equal budget (R)",
            short: "R",
            stroke: format!("stroke='{GREY}'"),
            width: 1.2,
            runs: pick(|r| &r.random),
        },
        Arm {
            label: "Seeded from the previous best (S)",
            short: "S",
            stroke: format!("stroke='{}' class='k2'", SERIES[1].0),
            width: 1.4,
            runs: pick(|r| &r.seeded),
        },
        Arm {
            label: "De novo evolution (D)",
            short: "D",
            stroke: format!("stroke='{}' class='k1'", SERIES[0].0),
            width: 1.6,
            runs: pick(|r| &r.de_novo),
        },
    ]
}

/// Score range over every trajectory and reference, rounded to ticks.
fn score_range(s: &FleetSummary, refs: &[Reference]) -> (Vec<f64>, usize) {
    let vals = arms(s)
        .iter()
        .flat_map(|a| a.runs.iter().flat_map(|r| r.trajectory.iter().map(|p| p.1)))
        .chain(s.other_runs.iter().filter_map(final_fitness))
        .chain(refs.iter().map(|r| r.value))
        .collect::<Vec<f64>>();
    let lo = vals.iter().cloned().fold(f64::MAX, f64::min);
    let hi = vals.iter().cloned().fold(f64::MIN, f64::max);
    if lo > hi {
        return nice_ticks(0.0, 1.0, 5);
    }
    nice_ticks(lo, hi, 8)
}

fn budget_axis(s: &FleetSummary, x0: f64, w: f64) -> LogAxis {
    let xs = arms(s)
        .iter()
        .flat_map(|a| a.runs.iter().flat_map(|r| r.trajectory.iter().map(|p| p.0)))
        .collect::<Vec<f64>>();
    let lo = xs.iter().cloned().fold(f64::MAX, f64::min).max(1.0);
    let hi = xs.iter().cloned().fold(1.0, f64::max);
    LogAxis {
        lo: 0.8 * lo.min(hi),
        hi: 1.08 * hi,
        x0,
        w,
    }
}

fn header(s: &mut String, w: f64, h: f64, label: &str) {
    let _ = write!(s, "<svg xmlns='http://www.w3.org/2000/svg' class='viz' viewBox='0 0 {w} {h}' width='{w}' height='{h}' role='img' aria-label='{}'>{}<rect {BG} width='100%' height='100%' rx='8'/>", esc(label), style());
}

/// Best score so far against evaluations for every D, S and R run (left), and the final score
/// of every run, pooled runs included, with the winner marked (right).
pub fn evolution_figure(s: &FleetSummary, marks: &[Mark], refs: &[Reference]) -> String {
    let (w, h) = (1200.0, 660.0);
    let (x0, top, pw, ph) = (70.0, 116.0, 640.0, 450.0);
    let arms = arms(s);
    let (ticks, dec) = score_range(s, refs);
    let y = Lin {
        lo: ticks[0],
        hi: *ticks.last().unwrap_or(&1.0),
        top,
        h: ph,
    };
    let x = budget_axis(s, x0, pw);
    let budget = arms[2]
        .runs
        .iter()
        .map(|r| r.evaluations)
        .max()
        .unwrap_or(0);
    let mut o = String::new();
    header(
        &mut o,
        w,
        h,
        "Fleet evolution: best score so far and final score of every run",
    );
    let _ = write!(o, "<text x='{x0}' y='34' font-size='19' font-weight='600'>Evolution against random sampling: best design score found so far</text>");
    let _ = write!(o, "<text class='t2' x='{x0}' y='58' font-size='13'>{} replicate seeds per arm, {} evaluations per run. Each line is one run; it starts at the run's first feasible design.</text>", arms[2].runs.len(), sep(budget));
    let mut lx = x0;
    for a in arms.iter().rev() {
        let _ = write!(o, "<line x1='{lx}' x2='{}' y1='84' y2='84' {} stroke-width='3'/><text class='t2' x='{}' y='88' font-size='12'>{}</text>", lx + 22.0, a.stroke, lx + 28.0, a.label);
        lx += 36.0 + 6.6 * a.label.len() as f64;
    }
    // score grid, shared by both panels
    for t in &ticks {
        let yy = y.at(*t);
        let _ = write!(o, "<line class='ax' stroke='#e6e5e0' x1='{x0}' x2='{}' y1='{yy:.1}' y2='{yy:.1}'/><text class='mu' x='{}' y='{:.1}' font-size='11' text-anchor='end'>{t:.dec$}</text>", x0 + pw, x0 - 8.0, yy + 4.0);
    }
    for d in x.decades() {
        let xx = x.at(d);
        let _ = write!(o, "<line class='ax' stroke='#e6e5e0' x1='{xx:.1}' x2='{xx:.1}' y1='{top}' y2='{}'/><text class='mu' x='{xx:.1}' y='{}' font-size='11' text-anchor='middle'>{}</text>", top + ph, top + ph + 18.0, decade_label(d));
    }
    let _ = write!(o, "<text class='t2' x='{}' y='{}' font-size='12' text-anchor='middle'>evaluations (log scale)</text><text class='t2' x='{}' y='{}' font-size='12' text-anchor='middle' transform='rotate(-90 {} {})'>best weighted score (higher is better)</text>", x0 + pw / 2.0, top + ph + 40.0, x0 - 46.0, top + ph / 2.0, x0 - 46.0, top + ph / 2.0);
    for m in marks {
        let xx = x.at(m.evaluations);
        let _ = write!(o, "<line x1='{xx:.1}' x2='{xx:.1}' y1='{top}' y2='{}' stroke='{GREY}' stroke-width='1' stroke-dasharray='2 3'/><text class='mu' x='{:.1}' y='{}' font-size='11' text-anchor='end'>{}</text>", top + ph, xx - 5.0, top + 14.0, esc(&m.label));
    }
    for a in &arms {
        for r in &a.runs {
            let pts = change_points(&r.trajectory);
            if pts.is_empty() {
                continue;
            }
            let _ = write!(o, "<path d='{}' fill='none' {} stroke-width='{}' stroke-linejoin='round' opacity='0.9'><title>{}: {}</title></path>", step_path(&pts, &x, &y), a.stroke, a.width, esc(&r.source), final_fitness(r).map(|f| format!("{f:.3}")).unwrap_or("no feasible design".into()));
        }
    }
    for r in refs {
        let (yy, x1, tx) = (y.at(r.value), x0 + pw, x0 + 6.0);
        let text = esc(&format!("{} {:.3}", r.label, r.value));
        let _ = write!(o, "<line x1='{x0}' x2='{x1}' y1='{yy:.1}' y2='{yy:.1}' class='kk' stroke='#52514e' stroke-width='1.2' stroke-dasharray='6 4'/>{}", halo_text(tx, yy - 6.0, 11.0, &text, "start"));
    }
    // The winner's score across the whole panel, so every run reads against it.
    if let Some((w, f)) = s
        .overall_best
        .as_ref()
        .and_then(|w| final_fitness(w).map(|f| (w, f)))
    {
        let yy = y.at(f);
        let text = esc(&format!("winner ({}) {f:.3}", w.source));
        let _ = write!(o, "<line x1='{x0}' x2='{}' y1='{yy:.1}' y2='{yy:.1}' class='kk' stroke='#52514e' stroke-width='1.6'/>{}", x0 + pw, halo_text(x0 + pw - 4.0, yy - 6.0, 12.0, &text, "end"));
    }
    dot_panel(&mut o, s, &arms, &y, refs, (790.0, top, 370.0, ph));
    let _ = write!(o, "<text class='mu' x='{x0}' y='{}' font-size='11'>Every run of a seed scores designs under one shared objective; D, S and R spend equal evaluations. Pooled runs start from the elites of all S and D runs.</text><text class='mu' x='{x0}' y='{}' font-size='11'>Forge reduced-order screen ({}): search statistics, not physical performance.</text>", h - 30.0, h - 13.0, esc(&s.evidence_class));
    o.push_str("</svg>");
    o
}

/// Final score of every run as one dot per run, one row per arm; the winner is a star.
fn dot_panel(
    o: &mut String,
    s: &FleetSummary,
    arms: &[Arm],
    y: &Lin,
    refs: &[Reference],
    (x0, top, pw, ph): (f64, f64, f64, f64),
) {
    let x = Lin {
        lo: y.lo,
        hi: y.hi,
        top: x0,
        h: pw,
    };
    let winner = s.overall_best.as_ref();
    let pooled: Vec<&ArmSummary> = s
        .other_runs
        .iter()
        .filter(|r| {
            r.source == "P"
                || r.source.starts_with("F-")
                || winner.is_some_and(|w| w.source == r.source)
        })
        .collect();
    let mut rows: Vec<(String, String, Vec<&ArmSummary>)> = Vec::new();
    if !pooled.is_empty() {
        rows.push((
            "Pooled final runs (P, F)".into(),
            format!("stroke='{}' class='k3'", SERIES[2].0),
            pooled,
        ));
    }
    for a in arms.iter().rev() {
        rows.push((a.label.into(), a.stroke.clone(), a.runs.clone()));
    }
    let row_h = ph / rows.len().max(1) as f64;
    let _ = write!(o, "<text class='t2' x='{x0}' y='{}' font-size='13' font-weight='600'>Final score of every run</text>", top - 14.0);
    let (ticks, dec) = nice_ticks(y.lo, y.hi, 4);
    for t in ticks.iter().filter(|t| **t >= y.lo && **t <= y.hi) {
        let xx = x.across(*t);
        let _ = write!(o, "<line class='ax' stroke='#e6e5e0' x1='{xx:.1}' x2='{xx:.1}' y1='{top}' y2='{}'/><text class='mu' x='{xx:.1}' y='{}' font-size='11' text-anchor='middle'>{t:.dec$}</text>", top + ph, top + ph + 18.0);
    }
    let _ = write!(o, "<text class='t2' x='{}' y='{}' font-size='12' text-anchor='middle'>final best score</text>", x0 + pw / 2.0, top + ph + 40.0);
    for r in refs {
        let xx = x.across(r.value);
        let _ = write!(o, "<line x1='{xx:.1}' x2='{xx:.1}' y1='{top}' y2='{}' class='kk' stroke='#52514e' stroke-width='1.2' stroke-dasharray='6 4'/>", top + ph);
    }
    for (k, (label, stroke, runs)) in rows.iter().enumerate() {
        let ry = top + k as f64 * row_h;
        let vals: Vec<f64> = runs.iter().filter_map(|r| final_fitness(r)).collect();
        let med = median(&vals)
            .map(|m| format!(" · median {m:.3}"))
            .unwrap_or_default();
        o.push_str(&halo_text(
            x0,
            ry + 16.0,
            12.0,
            &format!("{}{med}", esc(label)),
            "start",
        ));
        let fill = stroke
            .replacen("stroke=", "fill=", 1)
            .replace("class='k", "class='f");
        let mut order: Vec<&&ArmSummary> = runs.iter().collect();
        order.sort_by(|a, b| {
            final_fitness(a)
                .unwrap_or(f64::MIN)
                .total_cmp(&final_fitness(b).unwrap_or(f64::MIN))
        });
        for (i, r) in order.iter().enumerate() {
            let cy = ry + 0.58 * row_h + ((i % 3) as f64 - 1.0) * 7.0;
            match final_fitness(r) {
                Some(f) if winner.is_some_and(|w| w.source == r.source) => {
                    let cx = x.across(f);
                    let _ = write!(o, "{}<text x='{cx:.1}' y='{:.1}' font-size='12' font-weight='600' text-anchor='end'>winner {f:.3}</text><text class='t2' x='{cx:.1}' y='{:.1}' font-size='11' text-anchor='end'>{}</text>", star(cx, cy, 9.0), cy - 14.0, cy + 24.0, esc(&r.source));
                }
                Some(f) => {
                    let _ = write!(o, "<circle cx='{:.1}' cy='{cy:.1}' r='4.5' {fill} fill-opacity='0.85'><title>{}: {f:.3}</title></circle>", x.across(f), esc(&r.source));
                }
                None => {
                    let _ = write!(o, "<circle cx='{:.1}' cy='{cy:.1}' r='4.5' fill='none' {stroke} stroke-width='1.5'><title>{}: no feasible design</title></circle>", x.across(y.lo), esc(&r.source));
                }
            }
        }
    }
}

/// Text with a background-coloured halo, readable over lines; `anchor` is the SVG text-anchor.
fn halo_text(x: f64, y: f64, size: f64, text: &str, anchor: &str) -> String {
    format!("<text class='halo' x='{x:.1}' y='{y:.1}' font-size='{size}' text-anchor='{anchor}' fill='none' stroke='#fcfcfb' stroke-width='4' stroke-linejoin='round'>{text}</text><text class='t2' x='{x:.1}' y='{y:.1}' font-size='{size}' text-anchor='{anchor}'>{text}</text>")
}

/// Five-pointed star centred at (`cx`, `cy`).
fn star(cx: f64, cy: f64, r: f64) -> String {
    let mut d = String::new();
    for k in 0..10 {
        let rad = if k % 2 == 0 { r } else { 0.45 * r };
        let a = std::f64::consts::PI * (k as f64 / 5.0 - 0.5);
        let _ = write!(
            d,
            "{}{:.1},{:.1}",
            if k == 0 { "M" } else { "L" },
            cx + rad * a.cos(),
            cy + rad * a.sin()
        );
    }
    format!("<path d='{d}Z' class='t1' fill='#0b0b0b' stroke='#fcfcfb' stroke-width='1.5'/>")
}

/// One small panel per replicate seed with its D, S and R runs on shared axes.
pub fn seed_panels(s: &FleetSummary, marks: &[Mark]) -> String {
    let cols = 4usize;
    let n = s.replicates.len();
    let rows = n.div_ceil(cols).max(1);
    let (pw, ph, gx, gy, left, top) = (232.0, 150.0, 58.0, 78.0, 64.0, 150.0);
    let w = 1200.0;
    let h = top + (rows - 1) as f64 * (ph + gy) + ph + 56.0;
    let (ticks, dec) = score_range(s, &[]);
    let mut o = String::new();
    header(&mut o, w, h, "Best score so far in each replicate seed");
    let _ = write!(o, "<text x='{left}' y='34' font-size='19' font-weight='600'>Every replicate seed: de novo evolution (D), seeded search (S) and random sampling (R)</text><text class='t2' x='{left}' y='58' font-size='13'>Best weighted score found so far against evaluations (log scale); shared axes. Header values are each run's final score.</text>");
    let arms = arms(s);
    let mut lx = left;
    for a in arms.iter().rev() {
        let _ = write!(o, "<line x1='{lx}' x2='{}' y1='82' y2='82' {} stroke-width='3'/><text class='t2' x='{}' y='86' font-size='12'>{}</text>", lx + 22.0, a.stroke, lx + 28.0, a.label);
        lx += 36.0 + 6.6 * a.label.len() as f64;
    }
    for (i, rep) in s.replicates.iter().enumerate() {
        let px = left + (i % cols) as f64 * (pw + gx);
        let py = top + (i / cols) as f64 * (ph + gy);
        let y = Lin {
            lo: ticks[0],
            hi: *ticks.last().unwrap_or(&1.0),
            top: py,
            h: ph,
        };
        let x = budget_axis(s, px, pw);
        let _ = write!(
            o,
            "<text x='{px}' y='{:.1}' font-size='13' font-weight='600'>seed {}</text>",
            py - 26.0,
            rep.seed
        );
        let mut vals = String::new();
        for (arm, run) in [("D", &rep.de_novo), ("S", &rep.seeded), ("R", &rep.random)] {
            let v = run
                .as_ref()
                .and_then(final_fitness)
                .map(|f| format!("{f:.3}"))
                .unwrap_or("none".into());
            let cls = match arm {
                "D" => format!("class='f1' fill='{}'", SERIES[0].0),
                "S" => format!("class='f2' fill='{}'", SERIES[1].0),
                _ => format!("fill='{GREY}'"),
            };
            let _ = write!(vals, "<tspan {cls}>{arm} {v}</tspan>  ");
        }
        let _ = write!(
            o,
            "<text x='{px}' y='{:.1}' font-size='12'>{}</text>",
            py - 10.0,
            vals.trim_end()
        );
        for t in &ticks {
            let yy = y.at(*t);
            let _ = write!(
                o,
                "<line class='ax' stroke='#e6e5e0' x1='{px}' x2='{}' y1='{yy:.1}' y2='{yy:.1}'/>",
                px + pw
            );
            if i % cols == 0 {
                let _ = write!(o, "<text class='mu' x='{}' y='{:.1}' font-size='10' text-anchor='end'>{t:.dec$}</text>", px - 6.0, yy + 3.0);
            }
        }
        for d in x.decades() {
            let xx = x.at(d);
            let _ = write!(
                o,
                "<text class='mu' x='{xx:.1}' y='{}' font-size='10' text-anchor='middle'>{}</text>",
                py + ph + 14.0,
                decade_label(d)
            );
        }
        for m in marks {
            let xx = x.at(m.evaluations);
            let _ = write!(o, "<line x1='{xx:.1}' x2='{xx:.1}' y1='{py}' y2='{}' stroke='{GREY}' stroke-width='1' stroke-dasharray='2 3'/>", py + ph);
            if i == 0 {
                let _ = write!(
                    o,
                    "<text class='mu' x='{:.1}' y='{}' font-size='10' text-anchor='end'>{}</text>",
                    xx - 4.0,
                    py + 10.0,
                    esc(&m.label)
                );
            }
        }
        for a in &arms {
            let run = match a.short {
                "D" => &rep.de_novo,
                "S" => &rep.seeded,
                _ => &rep.random,
            };
            if let Some(r) = run {
                let pts = change_points(&r.trajectory);
                if !pts.is_empty() {
                    let _ = write!(
                        o,
                        "<path d='{}' fill='none' {} stroke-width='{}' stroke-linejoin='round'/>",
                        step_path(&pts, &x, &y),
                        a.stroke,
                        a.width + 0.4
                    );
                }
            }
        }
    }
    let _ = write!(o, "<text class='mu' x='{left}' y='{}' font-size='11'>Forge reduced-order screen ({}): search statistics, not physical performance.</text>", h - 12.0, esc(&s.evidence_class));
    o.push_str("</svg>");
    o
}

/// Inputs of the winner card.
pub struct Winner<'a> {
    pub source: &'a str,
    /// `receipt.json` of the winner's `joil-forge report`.
    pub receipt: &'a serde_json::Value,
    /// Feasible count, total and worst score under further closure sets, if evaluated.
    pub closures: Option<(usize, usize, f64)>,
    pub hero_png: &'a [u8],
    pub pfd_svg: &'a str,
    pub history: &'a [GenStats],
}

/// The winning design: rendered CAD, its process network, the key screen numbers and how its
/// lineage's best design evolved.
pub fn winner_card(wn: &Winner) -> String {
    let (w, h) = (1200.0, 830.0);
    let r = wn.receipt;
    let num = |p: &str| r.pointer(p).and_then(serde_json::Value::as_f64);
    let int = |p: &str| r.pointer(p).and_then(serde_json::Value::as_u64);
    let mut o = String::new();
    header(&mut o, w, h, "The winning design");
    let stages = int("/best/behavior/series_depth").unwrap_or(0);
    let cells = int("/best/behavior/cyclone_cells").unwrap_or(0);
    let _ = write!(o, "<text x='24' y='36' font-size='20' font-weight='600'>The winner: {} stage{} in series, {} cyclone cells, one printed part</text>", stages, if stages == 1 { "" } else { "s" }, cells);
    let _ = write!(o, "<text class='t2' x='24' y='60' font-size='13'>Best design of the fleet ({}), rendered from its printable CAD: translucent printed part over the fluid network, one colour per stage.</text>", esc(wn.source));
    let (iw, ih) = (760.0, 427.5);
    let _ = write!(
        o,
        "<image x='24' y='76' width='{iw}' height='{ih}' href='data:image/png;base64,{}'/>",
        base64(wn.hero_png)
    );
    let ph = iw * 370.0 / 980.0;
    let _ = write!(
        o,
        "<image x='24' y='{}' width='{iw}' height='{ph:.1}' href='data:image/svg+xml;base64,{}'/>",
        76.0 + ih + 14.0,
        base64(wn.pfd_svg.as_bytes())
    );
    let mut facts: Vec<(String, String)> = Vec::new();
    if let (Some(a), Some(b)) = (
        num("/best/search_score/fitness"),
        num("/best/report_score/fitness"),
    ) {
        facts.push((
            "Score (search / report resolution)".into(),
            format!("{a:.3} / {b:.3}"),
        ));
    }
    let duties = r
        .pointer("/campaign/duties")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    for (k, d) in duties.iter().enumerate() {
        let name = d
            .pointer("/name")
            .and_then(|v| v.as_str())
            .unwrap_or("duty");
        let cin = d
            .pointer("/feed/cin_mg_l")
            .and_then(serde_json::Value::as_f64);
        if let (Some(cin), Some(out)) =
            (cin, num(&format!("/best/report_score/duty_cout_mg_l/{k}")))
        {
            facts.push((
                format!("Oil out: {name}"),
                format!(
                    "{cin:.0} → {out:.1} mg/L ({:.1}% removed)",
                    100.0 * (1.0 - out / cin)
                ),
            ));
        }
    }
    if let (Some(q), Some(p)) = (
        num("/best/report_score/q_treated_m3_h"),
        num("/best/report_score/feed_pressure_kpa"),
    ) {
        facts.push(("Treated flow".into(), format!("{q:.2} m³/h at {p:.0} kPa")));
    }
    if let (Some(l), Some(rad), Some(m)) = (
        num("/cad_best/length_mm"),
        num("/cad_best/envelope_radius_mm"),
        num("/cad_best/printed_mass_kg"),
    ) {
        facts.push((
            "Printed part".into(),
            format!("{l:.0} mm long, {:.0} mm across, {m:.2} kg", 2.0 * rad),
        ));
    }
    if let (Some(posts), Some(un), Some(all)) = (
        int("/cad_best/print/support_posts"),
        num("/cad_best/print/unsupported_ceiling_area_mm2"),
        num("/cad_best/print/ceiling_area_mm2"),
    ) {
        facts.push((
            "Print audit".into(),
            format!(
                "{posts} posts; {:.1}% of ceilings unsupported",
                100.0 * un / all.max(1e-9)
            ),
        ));
    }
    if let Some(e) = num("/best/report_score/specific_energy_kj_m3") {
        facts.push(("Specific energy".into(), format!("{e:.0} kJ/m³ treated")));
    }
    if let Some((ok, n, worst)) = wn.closures {
        let low = if worst.is_finite() {
            format!("; lowest score {worst:.3}")
        } else {
            String::new()
        };
        facts.push((
            "Under other closure sets".into(),
            format!("feasible in {ok} of {n}{low}"),
        ));
    }
    let fx = 810.0;
    for (k, (label, value)) in facts.iter().enumerate() {
        let fy = 92.0 + k as f64 * 46.0;
        let _ = write!(o, "<text class='t2' x='{fx}' y='{fy:.1}' font-size='12'>{}</text><text x='{fx}' y='{:.1}' font-size='15' font-weight='600'>{}</text>", esc(label), fy + 19.0, esc(value));
    }
    lineage(
        &mut o,
        wn.history,
        (fx, 92.0 + facts.len() as f64 * 46.0 + 18.0, 360.0, h - 40.0),
    );
    let _ = write!(o, "<text class='mu' x='24' y='{}' font-size='11'>Forge reduced-order screen ({}): an evolved candidate for CFD and bench testing, not a validated separator. Rendering is not validation.</text>", h - 14.0, esc(r.pointer("/evidence_class").and_then(|v| v.as_str()).unwrap_or("")));
    o.push_str("</svg>");
    o
}

/// How the lineage's best design evolved: score, treated flow and outlet oil per generation.
fn lineage(o: &mut String, hist: &[GenStats], (x0, top, w, bottom): (f64, f64, f64, f64)) {
    type Get = fn(&GenStats) -> Option<f64>;
    let series: [(&str, Get, usize); 3] = [
        ("best score", |g| g.best_fitness, 2),
        (
            "treated flow, m³/h",
            |g| g.best_feasible.then_some(g.best_q_treated_m3_h),
            0,
        ),
        (
            "outlet oil, mg/L (rating duty)",
            |g| g.best_cout_mg_l.filter(|_| g.best_feasible),
            1,
        ),
    ];
    let _ = write!(o, "<text class='t2' x='{x0}' y='{top:.1}' font-size='13' font-weight='600'>How its lineage's best design evolved</text>");
    let gmax = hist
        .last()
        .map(|g| g.generation as f64)
        .unwrap_or(1.0)
        .max(1.0);
    let ph = ((bottom - top - 24.0) / 3.0 - 26.0).max(20.0);
    for (k, (label, get, slot)) in series.iter().enumerate() {
        let py = top + 24.0 + k as f64 * (ph + 26.0);
        let pts: Vec<(f64, f64)> = hist
            .iter()
            .filter_map(|g| get(g).map(|v| (g.generation as f64, v)))
            .collect();
        let (Some(first), Some(last)) = (pts.first(), pts.last()) else {
            continue;
        };
        let lo = pts.iter().map(|p| p.1).fold(f64::MAX, f64::min);
        let hi = pts.iter().map(|p| p.1).fold(f64::MIN, f64::max);
        let y = Lin {
            lo,
            hi: if hi - lo < 1e-12 { lo + 1.0 } else { hi },
            top: py + 12.0,
            h: ph - 12.0,
        };
        let xs = |g: f64| x0 + w * g / gmax;
        let pts = change_points(&pts);
        let mut d = String::new();
        for (i, (g, v)) in pts.iter().enumerate() {
            if i == 0 {
                let _ = write!(d, "M{:.1},{:.1}", xs(*g), y.at(*v));
            } else {
                let _ = write!(d, "H{:.1}V{:.1}", xs(*g), y.at(*v));
            }
        }
        let _ = write!(o, "<text class='mu' x='{x0}' y='{py:.1}' font-size='11'>{} · gen {:.0}: {:.3} → gen {:.0}: {:.3}</text><line class='ax' stroke='#e6e5e0' x1='{x0}' x2='{}' y1='{:.1}' y2='{:.1}'/><path d='{d}' fill='none' stroke='{}' class='k{}' stroke-width='1.8'/>", esc(label), first.0, first.1, last.0, last.1, x0 + w, py + ph, py + ph, SERIES[*slot].0, slot + 1);
    }
}
