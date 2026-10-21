//! SVG figures: process-flow diagram, grade-efficiency curves and evolution panels.
//! Colours come from a validated categorical palette in fixed slot order (unit index),
//! a single-hue sequential ramp for magnitude, and text inks that adapt to dark mode.

use crate::develop::Phenotype;
use crate::evolve::GenStats;
use crate::layout::depths;
use crate::score::Score;
use joil_forge_judges::network::PlantEvaluation;
use joil_forge_judges::plant::{Dest, UnitSpec};
use joil_forge_judges::sizes::SizeGrid;
use std::fmt::Write;

/// Categorical slots (light / dark), fixed order.
pub const SERIES: [(&str, &str); 8] = [
    ("#2a78d6", "#3987e5"),
    ("#eb6834", "#d95926"),
    ("#1baf7a", "#199e70"),
    ("#eda100", "#c98500"),
    ("#e87ba4", "#d55181"),
    ("#008300", "#008300"),
    ("#6250d6", "#9085e9"),
    ("#e34948", "#e66767"),
];

/// Neutral inks: (class, property, light, dark).
const INKS: [(&str, &str, &str, &str); 6] = [
    ("bg", "fill", "#fcfcfb", "#1a1a19"),
    ("ks", "stroke", "#fcfcfb", "#1a1a19"),
    ("t1", "fill", "#0b0b0b", "#f0efec"),
    ("t2", "fill", "#52514e", "#c3c2b7"),
    ("mu", "fill", "#8a8984", "#8f8e86"),
    ("ax", "stroke", "#e6e5e0", "#33332f"),
];

/// Theme rules for every class used by the figures, scoped under `scope`.
/// Shapes also carry the light colour as a presentation attribute, so renderers without
/// CSS support still draw the light theme.
pub fn theme_rules(scope: &str, dark: bool) -> String {
    let mut s = String::new();
    for (c, prop, l, d) in INKS {
        let v = if dark { d } else { l };
        let _ = write!(s, "{scope} .{c}{{{prop}:{v}}}");
        if c == "ks" {
            let _ = write!(
                s,
                "{scope} .kk{{stroke:{}}}{scope} .fg{{fill:{}}}",
                if dark { INKS[3].3 } else { INKS[3].2 },
                if dark { "#33332f" } else { "#e6e5e0" }
            );
        }
    }
    let _ = write!(
        s,
        "{scope} text{{fill:{}}}",
        if dark { INKS[2].3 } else { INKS[2].2 }
    );
    let _ = write!(
        s,
        "{scope} .halo{{fill:none;stroke:{}}}",
        if dark { INKS[0].3 } else { INKS[0].2 }
    );
    let _ = write!(
        s,
        "{scope} .t2{{fill:{}}}{scope} .mu{{fill:{}}}",
        if dark { INKS[3].3 } else { INKS[3].2 },
        if dark { INKS[4].3 } else { INKS[4].2 }
    );
    for (i, (l, d)) in SERIES.iter().enumerate() {
        let v = if dark { d } else { l };
        let _ = write!(
            s,
            "{scope} .f{n}{{fill:{v}}}{scope} .k{n}{{stroke:{v}}}",
            n = i + 1
        );
    }
    s
}

fn style() -> String {
    format!(
        "<style>text{{font-family:ui-sans-serif,system-ui,-apple-system,'Segoe UI',sans-serif}}{}@media (prefers-color-scheme: dark){{{}}}</style>",
        theme_rules(".viz", false),
        theme_rules(".viz", true)
    )
}

/// `fill` attribute and class of categorical slot `i`, with a surface-coloured outline.
fn fill_slot(i: usize) -> String {
    let k = i % SERIES.len();
    format!(
        "fill='{}' stroke='#fcfcfb' class='f{} ks'",
        SERIES[k].0,
        k + 1
    )
}

/// `stroke` attribute and class of categorical slot `i`.
fn stroke_slot(i: usize) -> String {
    let k = i % SERIES.len();
    format!("stroke='{}' class='k{}'", SERIES[k].0, k + 1)
}

const BG: &str = "fill='#fcfcfb' class='bg'";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Process-flow diagram of a developed plant, annotated with an evaluation if given.
pub fn pfd(
    ph: &Phenotype,
    ev: Option<&PlantEvaluation>,
    score: Option<&Score>,
    title: &str,
) -> String {
    let n = ph.plant.units.len();
    // Columns by series depth from the feed (recompute on the plant graph).
    let mut depth = vec![0usize; n];
    for _ in 0..=n {
        for (i, u) in ph.plant.units.iter().enumerate() {
            for l in &u.outlets {
                if let Dest::Unit(j) = l.to {
                    if !l.pumped && depth[j] < depth[i] + 1 && depth[i] < n {
                        depth[j] = depth[i] + 1;
                    }
                }
            }
        }
    }
    let _ = depths;
    let cols = depth.iter().copied().max().unwrap_or(0) + 1;
    let mut rows = vec![0usize; cols];
    let mut pos = vec![(0.0, 0.0); n];
    for i in 0..n {
        let c = depth[i];
        pos[i] = (300.0 + 240.0 * c as f64, 190.0 + 160.0 * rows[c] as f64);
        rows[c] += 1;
    }
    let height = 150.0 + 160.0 * *rows.iter().max().unwrap_or(&1) as f64 + 60.0;
    let width = (300.0 + 240.0 * cols as f64 + 150.0).max(980.0);
    let feed = (70.0, 190.0);
    let product = (width - 60.0, height - 80.0);
    let reject = (width - 60.0, 110.0);
    let q_feed = ev.map(|e| e.feed.q_m3_s()).unwrap_or(1.0);
    let mut s = String::new();
    let _ = write!(s, "<svg xmlns='http://www.w3.org/2000/svg' class='viz' viewBox='0 0 {width:.0} {height:.0}' width='{width:.0}' height='{height:.0}' role='img' aria-label='{}'>{}", esc(title), style());
    let _ = write!(s, "<rect {BG} width='100%' height='100%' rx='8'/>");
    let _ = write!(
        s,
        "<text x='24' y='34' font-size='19' font-weight='600'>{}</text>",
        esc(title)
    );
    if let (Some(e), Some(sc)) = (ev, score) {
        let _ = write!(
            s,
            "<text class='t2' x='24' y='58' font-size='13'>{} · screen model, not validated</text><text class='t2' x='24' y='78' font-size='13'>feed {:.2} m³/h at {:.0} kPa (pumps included) · Cu {:.1} mg/L · reject {:.2}% · {:.0} kJ/m³ treated · shell {:.2} kg</text>",
            if sc.feasible { "Compliant in screen (reference feed, all closures)" } else { "Not compliant in screen" },
            e.feed.flow_m3_h,
            e.equivalent_pressure_pa / 1e3,
            e.cout_mg_l,
            100.0 * e.reject_fraction,
            e.specific_energy_j_m3 / 1e3,
            sc.shell_mass_kg,
        );
    }
    let port = |i: usize, k: usize| -> (f64, f64) {
        let (x, y) = pos[i];
        match (&ph.plant.units[i].spec, k) {
            (UnitSpec::Cyclone(_), 0) => (x, y - 34.0),
            (UnitSpec::Cyclone(_), _) => (x, y + 40.0),
            (UnitSpec::Splitter { .. }, 0) => (x + 22.0, y - 10.0),
            (UnitSpec::Splitter { .. }, _) => (x + 22.0, y + 10.0),
            _ => (x + 34.0, y),
        }
    };
    let inlet = |i: usize| (pos[i].0 - 36.0, pos[i].1 - 16.0);
    let unit_oil = |i: usize| {
        ev.and_then(|e| {
            e.units
                .iter()
                .find(|u| u.index == i)
                .map(|u| (u.oil_in_kg_s, u.q_in_m3_s))
        })
    };
    // Edges
    let edge = |s: &mut String,
                a: (f64, f64),
                b: (f64, f64),
                q: f64,
                oil: Option<f64>,
                pumped: bool,
                up: bool| {
        let w = 1.5 + 6.0 * (q / q_feed).sqrt().min(2.0);
        let (c1, c2) = if up {
            ((a.0, a.1 - 60.0), (b.0 - 60.0, b.1))
        } else {
            ((a.0 + 70.0, a.1 + 20.0), (b.0 - 70.0, b.1))
        };
        let conc = oil.map(|o| 1000.0 * o / q.max(1e-12));
        let shade = conc
            .map(|c| (c.max(1.0).ln() / 1000f64.ln()).clamp(0.0, 1.0))
            .unwrap_or(0.3);
        let col = ramp(shade);
        let _ = write!(
            s,
            "<path d='M{:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}' fill='none' stroke='{col}' stroke-width='{w:.1}' stroke-linecap='round' opacity='0.9'{}><title>{:.3} m³/h{}</title></path>",
            a.0, a.1, c1.0, c1.1, c2.0, c2.1, b.0, b.1,
            if pumped { " stroke-dasharray='7 5'" } else { "" },
            q * 3600.0,
            conc.map(|c| format!(", {c:.1} mg/L")).unwrap_or_default()
        );
        let at = |t: f64| {
            let u = 1.0 - t;
            (
                u * u * u * a.0 + 3.0 * u * u * t * c1.0 + 3.0 * u * t * t * c2.0 + t * t * t * b.0,
                u * u * u * a.1 + 3.0 * u * u * t * c1.1 + 3.0 * u * t * t * c2.1 + t * t * t * b.1,
            )
        };
        let (mx, my) = at(0.5);
        // Labels of upward (reject-side) edges sit near their source, the rest mid-edge.
        let (lx, ly) = if up { at(0.32) } else { (mx, my) };
        let flow = if q * 3600.0 < 0.1 {
            format!("{:.1} L/h", q * 3.6e6)
        } else {
            format!("{:.2} m³/h", q * 3600.0)
        };
        let label = format!(
            "{flow}{}",
            conc.map(|c| format!(" · {c:.0} mg/L")).unwrap_or_default()
        );
        if pumped {
            let _ = write!(s, "<circle cx='{mx:.1}' cy='{my:.1}' r='9' fill='#fcfcfb' stroke='#52514e' class='bg kk' stroke-width='1.5'/><path d='M{:.1},{:.1} l7,4 l-7,4z' fill='#52514e' class='t2'/>", mx - 3.0, my - 4.0);
        }
        let _ = write!(
            s,
            "<text class='halo' x='{lx:.1}' y='{:.1}' font-size='11' text-anchor='middle' fill='none' stroke='#fcfcfb' stroke-width='4' stroke-linejoin='round'>{label}</text><text class='t2' x='{lx:.1}' y='{:.1}' font-size='11' text-anchor='middle' fill='#52514e'>{label}</text>",
            ly - 9.0,
            ly - 9.0,
        );
    };
    if let Some(e) = ev {
        let feed_oil = e.links.first().map(|l| l.oil_kg_s);
        let to = match ph.plant.feed.to {
            Dest::Unit(j) => inlet(j),
            Dest::Product => product,
            Dest::Reject => reject,
        };
        edge(&mut s, feed, to, q_feed, feed_oil, false, false);
        for l in e.links.iter().skip(1) {
            let Some(i) = l.from else { continue };
            let a = port(i, l.port);
            let (b, up) = match l.to {
                Dest::Unit(j) => (
                    inlet(j),
                    l.port == 0 && matches!(ph.plant.units[i].spec, UnitSpec::Cyclone(_)),
                ),
                Dest::Product => (product, false),
                Dest::Reject => (reject, true),
            };
            edge(
                &mut s,
                a,
                b,
                l.flow_m3_s,
                Some(l.oil_kg_s),
                ph.plant.units[i].outlets[l.port].pumped,
                up,
            );
        }
    }
    // Nodes
    for (i, u) in ph.plant.units.iter().enumerate() {
        let (x, y) = pos[i];
        let col = fill_slot(i);
        match &u.spec {
            UnitSpec::Cyclone(c) => {
                let _ = write!(
                    s,
                    "<path d='M{},{} h60 v18 l-24,46 h-12 l-24,-46z' {col} stroke-width='2'/>",
                    x - 30.0,
                    y - 34.0
                );
                let _ = write!(s, "<text x='{x}' y='{}' font-size='13' font-weight='600' text-anchor='middle' fill='#fff'>{}</text>", y - 14.0, if c.cells > 1 { format!("×{}", c.cells) } else { String::new() });
                let _ = write!(
                    s,
                    "<text class='t2' x='{x}' y='{}' font-size='11' text-anchor='middle'>D {:.0} mm · L {:.0} mm · r {:.1}%</text>",
                    y + 58.0,
                    2000.0 * c.stations[0][1],
                    1000.0 * c.length(),
                    100.0 * c.split
                );
            }
            UnitSpec::Coalescer(c) => {
                let _ = write!(
                    s,
                    "<rect x='{}' y='{}' width='56' height='64' rx='5' {col} stroke-width='2'/>",
                    x - 28.0,
                    y - 32.0
                );
                for k in 0..6 {
                    let _ = write!(
                        s,
                        "<path d='M{},{} l20,-20' stroke='#fff' stroke-width='1.5' opacity='0.7'/>",
                        x - 26.0 + 9.0 * k as f64,
                        y + 30.0
                    );
                }
                let _ = write!(s, "<text class='t2' x='{x}' y='{}' font-size='11' text-anchor='middle'>lattice bed {:.0}×{:.0} mm · ε {:.2}</text>", y + 50.0, 1000.0 * c.diameter_m, 1000.0 * c.length_m, c.porosity);
            }
            UnitSpec::Splitter { fraction } => {
                let _ = write!(
                    s,
                    "<path d='M{x},{} l22,22 l-22,22 l-22,-22z' {col} stroke-width='2'/>",
                    y - 22.0
                );
                let _ = write!(s, "<text class='t2' x='{x}' y='{}' font-size='11' text-anchor='middle'>split {:.0}/{:.0}</text>", y + 38.0, 100.0 * fraction, 100.0 - 100.0 * fraction);
            }
        }
        if let Some((oil, q)) = unit_oil(i) {
            let _ = write!(s, "<text class='mu' x='{x}' y='{}' font-size='10' text-anchor='middle'>in {:.0} mg/L</text>", y + 72.0, 1000.0 * oil / q.max(1e-12));
        }
    }
    for (p, label) in [(feed, "Feed"), (product, "Product"), (reject, "Reject")] {
        let _ = write!(s, "<circle cx='{}' cy='{}' r='16' fill='#fcfcfb' stroke='#52514e' class='bg kk' stroke-width='2'/><text x='{}' y='{}' font-size='12' font-weight='600' text-anchor='middle'>{label}</text>", p.0, p.1, p.0, p.1 + 32.0);
    }
    let _ = write!(s, "<text class='mu' x='24' y='{:.0}' font-size='11'>Edge width grows with the square root of flow; edge shade = oil concentration (log, light 1 → dark 1000 mg/L); dashed = pumped recycle.</text>", height - 16.0);
    s.push_str("</svg>");
    s
}

/// Sequential blue ramp (light → dark) for `t ∈ [0, 1]`.
pub fn ramp(t: f64) -> &'static str {
    const STEPS: [&str; 9] = [
        "#cde2fb", "#9ec5f4", "#6da7ec", "#3987e5", "#2a78d6", "#256abf", "#1c5cab", "#104281",
        "#0d366b",
    ];
    STEPS[((t.clamp(0.0, 1.0) * 8.0).round()) as usize]
}

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

/// Three stacked panels over generations (no dual axis): best shortfall, conservative
/// treated capacity, archive niches.
/// Round tick values: steps of 1, 2, 2.5 or 5 times a power of ten covering `[lo, hi]`.
fn nice_ticks(lo: f64, hi: f64, target: usize) -> (Vec<f64>, usize) {
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
    for (pi, (label, lines)) in panels.iter().enumerate() {
        let y0 = 52.0 + pi as f64 * (ph + gap);
        let (mut lo, mut hi) = lines
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
        let integer = lines
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
        // Keep end labels at least 13 px apart.
        ends.sort_by(|a, b| a.0.total_cmp(&b.0));
        for i in 1..ends.len() {
            if ends[i].0 - ends[i - 1].0 < 13.0 {
                ends[i].0 = ends[i - 1].0 + 13.0;
            }
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
