//! SVG charts: grade efficiency, the drop sizes each stage receives, search progress and
//! search-versus-control comparisons. One axis per panel, fixed categorical slots, round
//! ticks, direct end labels.

use super::svg::{esc, fill_slot, stroke_slot, style, BG};
use crate::evolve::GenStats;
use joil_forge_judges::network::PlantEvaluation;
use joil_forge_judges::sizes::SizeGrid;
use std::fmt::Write;

struct Axes {
    x0: f64,
    y0: f64,
    w: f64,
    h: f64,
}

/// Grade efficiency per cyclone unit with the feed size distribution beneath.
pub fn grade_chart(ev: &PlantEvaluation, grid: &SizeGrid) -> String {
    let (w, h) = (720.0, 380.0);
    let ax = Axes {
        x0: 64.0,
        y0: 40.0,
        w: 600.0,
        h: 280.0,
    };
    let (dlo, dhi) = (1.0f64, 200.0f64);
    let xs =
        |d_um: f64| ax.x0 + ax.w * ((d_um.clamp(dlo, dhi).ln() - dlo.ln()) / (dhi.ln() - dlo.ln()));
    let ys = |v: f64| ax.y0 + ax.h * (1.0 - v.clamp(0.0, 1.0));
    let mut s = String::new();
    let _ = write!(s, "<svg xmlns='http://www.w3.org/2000/svg' class='viz' viewBox='0 0 {w} {h}' width='{w}' height='{h}' role='img' aria-label='Grade efficiency by stage'>{}<rect {BG} width='100%' height='100%' rx='8'/>", style());
    let _ = write!(s, "<text x='{}' y='24' font-size='15' font-weight='600'>Grade efficiency by stage (overflow capture)</text>", ax.x0);
    for v in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let _ = write!(s, "<line class='ax' stroke='#e6e5e0' x1='{}' x2='{}' y1='{:.1}' y2='{:.1}'/><text class='mu' x='{}' y='{:.1}' font-size='11' text-anchor='end'>{:.0}%</text>", ax.x0, ax.x0 + ax.w, ys(v), ys(v), ax.x0 - 8.0, ys(v) + 4.0, 100.0 * v);
    }
    for d in [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0] {
        let _ = write!(s, "<line class='ax' stroke='#e6e5e0' x1='{:.1}' x2='{:.1}' y1='{}' y2='{}'/><text class='mu' x='{:.1}' y='{}' font-size='11' text-anchor='middle'>{d}</text>", xs(d), xs(d), ax.y0, ax.y0 + ax.h, xs(d), ax.y0 + ax.h + 16.0);
    }
    let _ = write!(s, "<text class='t2' x='{}' y='{}' font-size='12' text-anchor='middle'>droplet diameter (µm, log)</text>", ax.x0 + ax.w / 2.0, ax.y0 + ax.h + 34.0);
    // feed DSD as a muted area (scaled to its peak)
    let total: f64 = ev.feed_dsd.iter().sum::<f64>().max(1e-300);
    let dens: Vec<f64> = ev
        .feed_dsd
        .iter()
        .enumerate()
        .map(|(i, m)| m / total / (grid.edges_m[i + 1] / grid.edges_m[i]).ln())
        .collect();
    let peak = dens.iter().cloned().fold(0.0, f64::max).max(1e-300);
    let mut area = format!("M{:.1},{:.1}", xs(grid.d_m[0] * 1e6), ys(0.0));
    for (i, d) in grid.d_m.iter().enumerate() {
        let _ = write!(
            area,
            " L{:.1},{:.1}",
            xs(d * 1e6),
            ys(0.45 * dens[i] / peak)
        );
    }
    let _ = write!(
        area,
        " L{:.1},{:.1}Z",
        xs(grid.d_m[grid.n() - 1] * 1e6),
        ys(0.0)
    );
    let _ = write!(s, "<path d='{area}' fill='#e6e5e0' class='fg' opacity='0.9'><title>Feed oil-volume distribution (shape only)</title></path>");
    let mut legend_y = ax.y0 + 14.0;
    for u in ev.units.iter().filter(|u| u.kind == "cyclone") {
        let mut path = String::new();
        for (i, d) in grid.d_m.iter().enumerate() {
            let _ = write!(
                path,
                "{}{:.1},{:.1}",
                if i == 0 { "M" } else { " L" },
                xs(d * 1e6),
                ys(u.capture[i])
            );
        }
        let col = stroke_slot(u.index);
        let _ = write!(s, "<path d='{path}' fill='none' {col} stroke-width='2'><title>unit {} · reduced d50 {}</title></path>", u.index + 1, u.reduced_d50_um.map(|d| format!("{d:.1} µm")).unwrap_or("-".into()));
        let _ = write!(
            s,
            "<line x1='{}' x2='{}' y1='{legend_y}' y2='{legend_y}' {col} stroke-width='3'/><text class='t2' x='{}' y='{}' font-size='11'>unit {}{}</text>",
            ax.x0 + ax.w - 150.0,
            ax.x0 + ax.w - 130.0,
            ax.x0 + ax.w - 124.0,
            legend_y + 4.0,
            u.index + 1,
            u.reduced_d50_um.map(|d| format!(" · d50′ {d:.1} µm")).unwrap_or_default()
        );
        legend_y += 18.0;
    }
    let _ = write!(s, "<text class='mu' x='{}' y='{}' font-size='11'>Grey area: feed oil-volume distribution (shape). Reduced d50′ = 50% of (E − r)/(1 − r). Screen model.</text>", ax.x0, h - 10.0);
    s.push_str("</svg>");
    s
}

/// Oil drop-size distribution arriving at each unit (and leaving as product), relative to
/// the peak of the feed distribution, on a log scale: what every stage of a chain is
/// actually asked to separate.
pub fn stage_dsd_chart(ev: &PlantEvaluation, grid: &SizeGrid, title: &str) -> String {
    let (w, h) = (720.0, 380.0);
    let ax = Axes {
        x0: 64.0,
        y0: 40.0,
        w: 600.0,
        h: 280.0,
    };
    let (dlo, dhi) = (1.0f64, 200.0f64);
    let (ylo, yhi) = (-4.0f64, 0.0f64);
    let xs =
        |d_um: f64| ax.x0 + ax.w * ((d_um.clamp(dlo, dhi).ln() - dlo.ln()) / (dhi.ln() - dlo.ln()));
    let ys = |v: f64| {
        let l = v.max(1e-12).log10().clamp(ylo, yhi);
        ax.y0 + ax.h * (1.0 - (l - ylo) / (yhi - ylo))
    };
    let dens = |m: &[f64]| -> Vec<f64> {
        m.iter()
            .enumerate()
            .map(|(i, v)| v / (grid.edges_m[i + 1] / grid.edges_m[i]).ln())
            .collect()
    };
    let feed_total: f64 = ev.feed_dsd.iter().sum::<f64>().max(1e-300);
    let feed_d = dens(&ev.feed_dsd);
    let peak = feed_d.iter().cloned().fold(0.0, f64::max).max(1e-300);
    let mut s = String::new();
    let _ = write!(s, "<svg xmlns='http://www.w3.org/2000/svg' class='viz' viewBox='0 0 {w} {h}' width='{w}' height='{h}' role='img' aria-label='{}'>{}<rect {BG} width='100%' height='100%' rx='8'/>", esc(title), style());
    let _ = write!(
        s,
        "<text x='{}' y='24' font-size='15' font-weight='600'>{}</text>",
        ax.x0,
        esc(title)
    );
    for (k, label) in ["100%", "10%", "1%", "0.1%", "0.01%"].iter().enumerate() {
        let v = 10f64.powi(-(k as i32));
        let _ = write!(s, "<line class='ax' stroke='#e6e5e0' x1='{}' x2='{}' y1='{:.1}' y2='{:.1}'/><text class='mu' x='{}' y='{:.1}' font-size='11' text-anchor='end'>{label}</text>", ax.x0, ax.x0 + ax.w, ys(v), ys(v), ax.x0 - 8.0, ys(v) + 4.0);
    }
    for d in [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0] {
        let _ = write!(s, "<line class='ax' stroke='#e6e5e0' x1='{:.1}' x2='{:.1}' y1='{}' y2='{}'/><text class='mu' x='{:.1}' y='{}' font-size='11' text-anchor='middle'>{d}</text>", xs(d), xs(d), ax.y0, ax.y0 + ax.h, xs(d), ax.y0 + ax.h + 16.0);
    }
    let _ = write!(s, "<text class='t2' x='{}' y='{}' font-size='12' text-anchor='middle'>droplet diameter (µm, log)</text>", ax.x0 + ax.w / 2.0, ax.y0 + ax.h + 34.0);
    let curve = |m: &[f64]| -> String {
        let d = dens(m);
        let mut path = String::new();
        let mut open = false;
        for (i, dm) in grid.d_m.iter().enumerate() {
            let v = d[i] / peak;
            if v < 1e-5 {
                open = false;
                continue;
            }
            let _ = write!(
                path,
                "{}{:.1},{:.1}",
                if open { " L" } else { "M" },
                xs(dm * 1e6),
                ys(v)
            );
            open = true;
        }
        path
    };
    // Feed as a muted area.
    let mut area = format!("M{:.1},{:.1}", xs(grid.d_m[0] * 1e6), ys(1e-12));
    for (i, d) in grid.d_m.iter().enumerate() {
        let _ = write!(area, " L{:.1},{:.1}", xs(d * 1e6), ys(feed_d[i] / peak));
    }
    let _ = write!(
        area,
        " L{:.1},{:.1}Z",
        xs(grid.d_m[grid.n() - 1] * 1e6),
        ys(1e-12)
    );
    let _ = write!(
        s,
        "<path d='{area}' fill='#e6e5e0' class='fg' opacity='0.9'><title>Feed</title></path>"
    );
    let mut legend_y = ax.y0 + 14.0;
    let lx = ax.x0 + ax.w - 190.0;
    let _ = write!(s, "<rect x='{lx}' y='{}' width='20' height='8' fill='#e6e5e0' class='fg'/><text class='t2' x='{}' y='{}' font-size='11'>feed · 100% of oil</text>", legend_y - 4.0, lx + 26.0, legend_y + 4.0);
    legend_y += 18.0;
    for u in ev.units.iter().filter(|u| !u.inlet_dsd.is_empty()) {
        let share = 100.0 * u.inlet_dsd.iter().sum::<f64>() / feed_total;
        let col = stroke_slot(u.index);
        let _ = write!(s, "<path d='{}' fill='none' {col} stroke-width='2'><title>unit {} receives {share:.2}% of the feed oil</title></path>", curve(&u.inlet_dsd), u.index + 1);
        let _ = write!(s, "<line x1='{lx}' x2='{}' y1='{legend_y}' y2='{legend_y}' {col} stroke-width='3'/><text class='t2' x='{}' y='{}' font-size='11'>unit {} {} · {share:.2}%</text>", lx + 20.0, lx + 26.0, legend_y + 4.0, u.index + 1, esc(&u.kind));
        legend_y += 18.0;
    }
    let share = 100.0 * ev.product_dsd.iter().sum::<f64>() / feed_total;
    let _ = write!(s, "<path d='{}' fill='none' stroke='#52514e' class='kk' stroke-width='2' stroke-dasharray='5 4'><title>product carries {share:.3}% of the feed oil</title></path>", curve(&ev.product_dsd));
    let _ = write!(s, "<line x1='{lx}' x2='{}' y1='{legend_y}' y2='{legend_y}' stroke='#52514e' class='kk' stroke-width='2' stroke-dasharray='5 4'/><text class='t2' x='{}' y='{}' font-size='11'>product · {share:.3}%</text>", lx + 20.0, lx + 26.0, legend_y + 4.0);
    let _ = write!(s, "<text class='mu' x='{}' y='{}' font-size='11'>Oil per log size, relative to the feed peak. With recycle a stage can receive more oil than the feed. Screen model.</text>", ax.x0, h - 10.0);
    s.push_str("</svg>");
    s
}

/// Round tick values: steps of 1, 2, 2.5 or 5 times a power of ten covering `[lo, hi]`.
pub(crate) fn nice_ticks(lo: f64, hi: f64, target: usize) -> (Vec<f64>, usize) {
    ticks_with(lo, hi, target, false)
}

/// `integer`: counts, so the step is at least one and labels carry no decimals.
fn ticks_with(lo: f64, hi: f64, target: usize, integer: bool) -> (Vec<f64>, usize) {
    let span = (hi - lo).abs().max(1e-9);
    let raw = (span / target.max(1) as f64).max(if integer { 1.0 } else { 0.0 });
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 2.5, 5.0, 10.0]
        .iter()
        .filter(|m| !(integer && **m == 2.5))
        .map(|m| m * mag)
        .find(|st| *st >= raw)
        .unwrap_or(10.0 * mag);
    let first = (lo / step).floor() * step;
    let mut ticks = Vec::new();
    let mut v = first;
    while v <= hi + 1e-9 * span {
        ticks.push(if v.abs() < 1e-12 { 0.0 } else { v });
        v += step;
    }
    if ticks.last().is_some_and(|t| *t < hi - 1e-9 * span) {
        ticks.push(v);
    }
    let decimals = if step >= 1.0 && step.fract() == 0.0 {
        0
    } else {
        (-(step.log10().floor())).max(0.0) as usize + usize::from((step / mag - 2.5).abs() < 1e-9)
    };
    (ticks, decimals)
}

/// Three stacked panels over generations (no dual axis): best fitness, conservative
/// treated capacity, archive niches.
pub fn evolution_chart(hist: &[GenStats], title: &str) -> String {
    let (w, ph, gap) = (760.0, 150.0, 58.0);
    let h = 84.0 + 3.0 * (ph + gap);
    let gmax = hist.last().map(|g| g.generation).unwrap_or(1).max(1) as f64;
    let x0 = 70.0;
    let pw = w - x0 - 96.0;
    let xs = |g: f64| x0 + pw * g / gmax;
    let mut s = String::new();
    let _ = write!(s, "<svg xmlns='http://www.w3.org/2000/svg' class='viz' viewBox='0 0 {w} {h}' width='{w}' height='{h}' role='img' aria-label='{}'>{}<rect {BG} width='100%' height='100%' rx='8'/>", esc(title), style());
    let _ = write!(
        s,
        "<text x='{x0}' y='28' font-size='16' font-weight='600'>{}</text>",
        esc(title)
    );
    type Series = Box<dyn Fn(&GenStats) -> Option<f64>>;
    type Panel = (&'static str, Vec<(&'static str, Series)>, bool);
    let panels: [Panel; 3] = [
        (
            "Best design: weighted fitness (drawn once a design is feasible)",
            vec![("", Box::new(|g: &GenStats| g.best_fitness))],
            false,
        ),
        (
            "Best design: treated capacity, minimum over closure samples (m³/h)",
            vec![(
                "",
                Box::new(|g: &GenStats| g.best_feasible.then_some(g.best_q_treated_m3_h)),
            )],
            true,
        ),
        (
            "Archive niches filled",
            vec![
                (
                    "all",
                    Box::new(|g: &GenStats| Some(g.archive_niches as f64)),
                ),
                (
                    "feasible",
                    Box::new(|g: &GenStats| Some(g.archive_feasible as f64)),
                ),
            ],
            true,
        ),
    ];
    let (gticks, _) = nice_ticks(0.0, gmax, 6);
    for (pi, (label, series, zero)) in panels.iter().enumerate() {
        let y0 = 52.0 + pi as f64 * (ph + gap);
        let data: Vec<Vec<(f64, f64)>> = series
            .iter()
            .map(|(_, f)| {
                hist.iter()
                    .filter_map(|g| f(g).map(|v| (g.generation as f64, v)))
                    .collect()
            })
            .collect();
        let (mut lo, mut hi) = data
            .iter()
            .flatten()
            .fold((f64::MAX, f64::MIN), |m, v| (m.0.min(v.1), m.1.max(v.1)));
        if lo > hi {
            (lo, hi) = (0.0, 1.0);
        }
        if *zero {
            lo = lo.min(0.0);
        }
        if hi - lo < 1e-9 {
            hi = lo + 1.0;
        }
        let (ticks, decimals) = nice_ticks(lo, hi, 4);
        let (lo, hi) = (ticks[0], *ticks.last().unwrap_or(&hi));
        let ys = |v: f64| y0 + 20.0 + (ph - 20.0) * (1.0 - ((v - lo) / (hi - lo)).clamp(0.0, 1.0));
        let _ = write!(
            s,
            "<text class='t2' x='{x0}' y='{}' font-size='12'>{}</text>",
            y0 + 8.0,
            esc(label)
        );
        for v in &ticks {
            let y = ys(*v);
            let _ = write!(s, "<line class='ax' stroke='#e6e5e0' x1='{x0}' x2='{}' y1='{y:.1}' y2='{y:.1}'/><text class='mu' x='{}' y='{:.1}' font-size='10' text-anchor='end'>{v:.decimals$}</text>", x0 + pw, x0 - 6.0, y + 3.0);
        }
        for g in &gticks {
            let _ = write!(s, "<text class='mu' x='{:.1}' y='{:.1}' font-size='10' text-anchor='middle'>{g:.0}</text>", xs(*g), y0 + ph + 14.0);
        }
        for (si, ((name, _), vals)) in series.iter().zip(&data).enumerate() {
            let mut path = String::new();
            let mut prev: Option<f64> = None;
            for (g, v) in vals {
                let cmd = if prev.is_some_and(|p| *g - p <= 1.0) {
                    " L"
                } else {
                    "M"
                };
                let _ = write!(path, "{cmd}{:.1},{:.1}", xs(*g), ys(*v));
                prev = Some(*g);
            }
            let dash = if si > 0 {
                " stroke-dasharray='4 3'"
            } else {
                ""
            };
            let _ = write!(
                s,
                "<path d='{path}' fill='none' {} stroke-width='2'{dash}/>",
                stroke_slot(pi)
            );
            if let Some(last) = vals.last() {
                let shown = if decimals == 0 && last.1.fract() == 0.0 {
                    format!("{:.0}", last.1)
                } else {
                    format!("{:.3}", last.1)
                };
                let text = if name.is_empty() {
                    shown
                } else {
                    format!("{name} {shown}")
                };
                let _ = write!(s, "<circle cx='{:.1}' cy='{:.1}' r='4' {} stroke-width='2'/><text class='t2' x='{:.1}' y='{:.1}' font-size='11'>{}</text>", xs(last.0), ys(last.1), fill_slot(pi), xs(last.0) + 8.0, ys(last.1) + 4.0, esc(&text));
            }
        }
    }
    let _ = write!(s, "<text class='mu' x='{x0}' y='{:.0}' font-size='11'>Generation. The leader is ranked by fitness, so its capacity can fall when a more robust design takes over.</text><text class='mu' x='{x0}' y='{:.0}' font-size='11'>Screen-model search diagnostics, not physical evidence.</text>", h - 26.0, h - 11.0);
    s.push_str("</svg>");
    s
}

/// One line per series, per panel, against a shared x axis. Series keep their categorical
/// slot across panels, so a method has one colour everywhere; the end of each line is
/// labelled directly.
pub type Line<'a> = (&'a str, Vec<(f64, f64)>);

pub fn comparison_chart(title: &str, x_label: &str, panels: &[(&str, Vec<Line>)]) -> String {
    comparison(title, x_label, panels, false)
}

/// As [`comparison_chart`], with one y range for every panel, for panels that plot the same
/// quantity, so their heights compare directly.
pub fn comparison_chart_shared_y(
    title: &str,
    x_label: &str,
    panels: &[(&str, Vec<Line>)],
) -> String {
    comparison(title, x_label, panels, true)
}

fn comparison(title: &str, x_label: &str, panels: &[(&str, Vec<Line>)], shared_y: bool) -> String {
    let (w, ph, gap) = (760.0, 150.0, 58.0);
    let h = 84.0 + panels.len() as f64 * (ph + gap);
    let xmax = panels
        .iter()
        .flat_map(|p| p.1.iter())
        .flat_map(|l| l.1.iter())
        .fold(1.0_f64, |m, v| m.max(v.0));
    let x0 = 70.0;
    let pw = w - x0 - 150.0;
    let xs = |x: f64| x0 + pw * x / xmax;
    let mut s = String::new();
    let _ = write!(s, "<svg xmlns='http://www.w3.org/2000/svg' class='viz' viewBox='0 0 {w} {h}' width='{w}' height='{h}' role='img' aria-label='{}'>{}<rect {BG} width='100%' height='100%' rx='8'/>", esc(title), style());
    let _ = write!(
        s,
        "<text x='{x0}' y='28' font-size='16' font-weight='600'>{}</text>",
        esc(title)
    );
    let (xticks, xdec) = nice_ticks(0.0, xmax, 6);
    let all: Vec<&Line> = panels.iter().flat_map(|p| p.1.iter()).collect();
    for (pi, (label, lines)) in panels.iter().enumerate() {
        let y0 = 52.0 + pi as f64 * (ph + gap);
        let scope: Vec<&Line> = if shared_y {
            all.clone()
        } else {
            lines.iter().collect()
        };
        let (mut lo, mut hi) = scope
            .iter()
            .flat_map(|l| l.1.iter())
            .fold((f64::MAX, f64::MIN), |m, v| (m.0.min(v.1), m.1.max(v.1)));
        if lo > hi {
            (lo, hi) = (0.0, 1.0);
        }
        if lo >= 0.0 {
            lo = 0.0;
        }
        if hi - lo < 1e-9 {
            hi = lo + 1.0;
        }
        let integer = scope
            .iter()
            .flat_map(|l| l.1.iter())
            .all(|v| v.1.fract() == 0.0);
        let (ticks, dec) = ticks_with(lo, hi, 4, integer);
        let (lo, hi) = (ticks[0], *ticks.last().unwrap_or(&hi));
        let ys = |v: f64| y0 + 20.0 + (ph - 20.0) * (1.0 - ((v - lo) / (hi - lo)).clamp(0.0, 1.0));
        let _ = write!(
            s,
            "<text class='t2' x='{x0}' y='{}' font-size='12'>{}</text>",
            y0 + 8.0,
            esc(label)
        );
        for v in &ticks {
            let y = ys(*v);
            let _ = write!(s, "<line class='ax' stroke='#e6e5e0' x1='{x0}' x2='{}' y1='{y:.1}' y2='{y:.1}'/><text class='mu' x='{}' y='{:.1}' font-size='10' text-anchor='end'>{v:.dec$}</text>", x0 + pw, x0 - 6.0, y + 3.0);
        }
        for x in &xticks {
            let _ = write!(s, "<text class='mu' x='{:.1}' y='{:.1}' font-size='10' text-anchor='middle'>{x:.xdec$}</text>", xs(*x), y0 + ph + 14.0);
        }
        let mut ends: Vec<(f64, String, usize)> = Vec::new();
        for (si, (name, pts)) in lines.iter().enumerate() {
            let mut path = String::new();
            for (i, (x, y)) in pts.iter().enumerate() {
                let _ = write!(
                    path,
                    "{}{:.1},{:.1}",
                    if i == 0 { "M" } else { " L" },
                    xs(*x),
                    ys(*y)
                );
            }
            let _ = write!(
                s,
                "<path d='{path}' fill='none' {} stroke-width='2'/>",
                stroke_slot(si)
            );
            if pts.is_empty() {
                ends.push((ys(lo) - 4.0, format!("{name}: none"), si));
            }
            if let Some(last) = pts.last() {
                let _ = write!(
                    s,
                    "<circle cx='{:.1}' cy='{:.1}' r='4' {} stroke-width='2'/>",
                    xs(last.0),
                    ys(last.1),
                    fill_slot(si)
                );
                let v = if integer {
                    format!("{:.0}", last.1)
                } else {
                    format!("{:.3}", last.1)
                };
                ends.push((ys(last.1) + 4.0, format!("{name} {v}"), si));
            }
        }
        // Keep end labels at least 13 px apart and above the panel's x axis labels.
        ends.sort_by(|a, b| a.0.total_cmp(&b.0));
        for i in 1..ends.len() {
            if ends[i].0 - ends[i - 1].0 < 13.0 {
                ends[i].0 = ends[i - 1].0 + 13.0;
            }
        }
        let mut floor = y0 + ph;
        for e in ends.iter_mut().rev() {
            e.0 = e.0.min(floor);
            floor = e.0 - 13.0;
        }
        for (y, text, _) in ends {
            let _ = write!(
                s,
                "<text class='t2' x='{:.1}' y='{y:.1}' font-size='11'>{}</text>",
                x0 + pw + 10.0,
                esc(&text)
            );
        }
    }
    let _ = write!(s, "<text class='mu' x='{x0}' y='{:.0}' font-size='11'>{}</text><text class='mu' x='{x0}' y='{:.0}' font-size='11'>Screen-model search diagnostics, not physical evidence.</text>", h - 26.0, esc(x_label), h - 11.0);
    s.push_str("</svg>");
    s
}
