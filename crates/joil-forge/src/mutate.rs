//! Variation operators. Structural operators change the process network (insert a
//! series stage, number up cells, add a reject scavenger, branch, recycle, coalescer,
//! delete, rewire, duplicate); parametric operators perturb continuous genes in log
//! space with occasional large jumps. Every result is normalised; nothing ever falls
//! back to a baseline design.

use crate::genome::{bounds, Body, Genome, Route, Segment, Target, UnitGene};
use crate::layout::depths;
use crate::random::{random_body, random_coalescer, random_cyclone};
use crate::rng::Rng;

/// Structural operator names, in the order of their selection weights.
pub const STRUCTURAL: [(&str, f64); 12] = [
    ("add_series_stage", 1.0),
    ("number_up", 1.0),
    ("number_down", 0.6),
    ("add_reject_scavenger", 0.8),
    ("add_branch", 0.5),
    ("add_recycle", 0.6),
    ("add_coalescer", 0.3),
    ("delete_unit", 0.8),
    ("rewire", 0.8),
    ("duplicate_unit", 0.5),
    ("edit_profile", 1.2),
    ("toggle_pump", 0.3),
];

fn pick_unit(g: &Genome, r: &mut Rng, cyclone_only: bool) -> Option<usize> {
    let idx: Vec<usize> = (0..g.units.len())
        .filter(|i| !cyclone_only || matches!(g.units[*i].body, Body::Cyclone(_)))
        .collect();
    r.pick(&idx).copied()
}

fn fresh(next_id: &mut u32) -> u32 {
    *next_id += 1;
    *next_id
}

fn jitter(x: &mut f64, r: &mut Rng, sigma: f64) {
    let s = if r.chance(0.1) { 4.0 * sigma } else { sigma };
    *x *= (s * r.normal()).exp();
}

/// Perturb every gene of a body with probability `p`.
pub fn perturb_body(b: &mut Body, r: &mut Rng, p: f64) {
    match b {
        Body::Cyclone(c) => {
            if r.chance(p) {
                jitter(&mut c.diameter_mm, r, 0.08);
            }
            if r.chance(p) {
                jitter(&mut c.chamber_len_ratio, r, 0.12);
            }
            for s in &mut c.segments {
                if r.chance(p) {
                    jitter(&mut s.len_ratio, r, 0.15);
                }
                if r.chance(p) {
                    jitter(&mut s.end_ratio, r, 0.08);
                }
            }
            if r.chance(p * 0.5) {
                c.jets = (c.jets as i64 + if r.chance(0.5) { 1 } else { -1 }).clamp(1, 8) as u32;
            }
            if r.chance(p) {
                jitter(&mut c.inlet_area_ratio, r, 0.12);
            }
            if r.chance(p) {
                jitter(&mut c.inlet_aspect, r, 0.12);
            }
            if r.chance(p * 0.2) {
                c.involute = !c.involute;
            }
            if r.chance(p) {
                jitter(&mut c.orifice_ratio, r, 0.12);
            }
            if r.chance(p) {
                jitter(&mut c.split, r, 0.15);
            }
        }
        Body::Coalescer(c) => {
            if r.chance(p) {
                jitter(&mut c.diameter_mm, r, 0.08);
            }
            if r.chance(p) {
                jitter(&mut c.length_mm, r, 0.15);
            }
            if r.chance(p) {
                c.porosity = (c.porosity + 0.03 * r.normal()).clamp(0.5, 0.95);
            }
            if r.chance(p) {
                jitter(&mut c.strut_mm, r, 0.1);
            }
        }
        Body::Splitter { fraction } => {
            if r.chance(p) {
                *fraction = (*fraction + 0.08 * r.normal()).clamp(0.05, 0.95);
            }
        }
    }
}

/// Parametric mutation of the whole genome; each gene mutates with probability `p`.
pub fn perturb(g: &mut Genome, r: &mut Rng, p: f64) {
    for u in &mut g.units {
        perturb_body(&mut u.body, r, p);
        for route in &mut u.routes {
            if r.chance(p * 0.5) {
                jitter(&mut route.channel_mm, r, 0.1);
            }
        }
    }
    if r.chance(p * 0.5) {
        jitter(&mut g.feed.channel_mm, r, 0.1);
    }
}

fn targets(g: &Genome) -> Vec<Target> {
    let mut t: Vec<Target> = g.units.iter().map(|u| Target::Unit(u.id)).collect();
    t.push(Target::Product);
    t.push(Target::Reject);
    t
}

/// Apply one structural operator by name. Returns `false` if it did not apply.
pub fn apply_structural(g: &mut Genome, op: &str, r: &mut Rng, next_id: &mut u32) -> bool {
    if g.units.len() >= bounds::MAX_UNITS
        && matches!(
            op,
            "add_series_stage"
                | "add_reject_scavenger"
                | "add_branch"
                | "add_coalescer"
                | "duplicate_unit"
        )
    {
        return false;
    }
    match op {
        "add_series_stage" => {
            let Some(i) = pick_unit(g, r, false) else {
                return false;
            };
            let k = r.below(g.units[i].routes.len());
            let old = g.units[i].routes[k];
            let id = fresh(next_id);
            let mut body = if r.chance(0.5) {
                g.units
                    .iter()
                    .find(|u| matches!(u.body, Body::Cyclone(_)))
                    .map(|u| u.body.clone())
                    .unwrap_or_else(|| Body::Cyclone(random_cyclone(r)))
            } else {
                Body::Cyclone(random_cyclone(r))
            };
            perturb_body(&mut body, r, 0.5);
            g.units[i].routes[k].to = Target::Unit(id);
            g.units[i].routes[k].pumped = false;
            g.units.push(UnitGene {
                id,
                body,
                routes: vec![
                    Route::to(Target::Reject),
                    Route {
                        to: old.to,
                        pumped: old.pumped,
                        channel_mm: old.channel_mm,
                    },
                ],
            });
            true
        }
        "number_up" | "number_down" => {
            let Some(i) = pick_unit(g, r, true) else {
                return false;
            };
            if let Body::Cyclone(c) = &mut g.units[i].body {
                let n = c.cells as f64;
                let up = op == "number_up";
                if (up && c.cells >= bounds::CELLS.1) || (!up && c.cells <= 1) {
                    return false;
                }
                let m = if up { n + 1.0 } else { n - 1.0 };
                // Keep total swirl-chamber cross-section, so velocities stay comparable.
                c.diameter_mm *= (n / m).sqrt();
                c.cells = m as u32;
                true
            } else {
                false
            }
        }
        "add_reject_scavenger" => {
            let Some(i) = pick_unit(g, r, true) else {
                return false;
            };
            let id = fresh(next_id);
            let mut c = random_cyclone(r);
            c.split = r.range(0.1, 0.45);
            c.orifice_ratio = r.range(0.08, 0.25);
            let back = if r.chance(0.6) {
                (Target::Unit(g.units[i].id), true)
            } else {
                (Target::Product, false)
            };
            g.units[i].routes[0] = Route {
                to: Target::Unit(id),
                pumped: false,
                channel_mm: g.units[i].routes[0].channel_mm,
            };
            g.units.push(UnitGene {
                id,
                body: Body::Cyclone(c),
                routes: vec![
                    Route::to(Target::Reject),
                    Route {
                        to: back.0,
                        pumped: back.1,
                        channel_mm: 6.0,
                    },
                ],
            });
            true
        }
        "add_branch" => {
            let Some(i) = pick_unit(g, r, false) else {
                return false;
            };
            let target_id = g.units[i].id;
            let (sid, bid) = (fresh(next_id), fresh(next_id));
            // Redirect every route into unit i through a new splitter.
            for u in &mut g.units {
                for route in &mut u.routes {
                    if route.to == Target::Unit(target_id) && !route.pumped {
                        route.to = Target::Unit(sid);
                    }
                }
            }
            if g.feed.to == Target::Unit(target_id) {
                g.feed.to = Target::Unit(sid);
            }
            let mut twin = g.units[i].clone();
            twin.id = bid;
            perturb_body(&mut twin.body, r, 0.7);
            g.units.push(UnitGene {
                id: sid,
                body: crate::genome::Body::Splitter {
                    fraction: r.range(0.3, 0.7),
                },
                routes: vec![
                    Route::to(Target::Unit(target_id)),
                    Route::to(Target::Unit(bid)),
                ],
            });
            g.units.push(twin);
            true
        }
        "add_recycle" => {
            let d = depths(g);
            let mut options = Vec::new();
            for (i, u) in g.units.iter().enumerate() {
                for (k, route) in u.routes.iter().enumerate() {
                    if route.to == Target::Product || route.to == Target::Reject {
                        for (j, v) in g.units.iter().enumerate() {
                            if d[j] <= d[i]
                                && !(matches!(u.body, Body::Cyclone(_))
                                    && k == 1
                                    && route.to == Target::Product
                                    && j == i)
                            {
                                options.push((i, k, v.id));
                            }
                        }
                    }
                }
            }
            let Some(&(i, k, to)) = r.pick(&options) else {
                return false;
            };
            g.units[i].routes[k].to = Target::Unit(to);
            g.units[i].routes[k].pumped = true;
            true
        }
        "add_coalescer" => {
            let Some(i) = pick_unit(g, r, true) else {
                return false;
            };
            let target_id = g.units[i].id;
            let id = fresh(next_id);
            for u in &mut g.units {
                for route in &mut u.routes {
                    if route.to == Target::Unit(target_id) && !route.pumped {
                        route.to = Target::Unit(id);
                    }
                }
            }
            if g.feed.to == Target::Unit(target_id) {
                g.feed.to = Target::Unit(id);
            }
            g.units.push(UnitGene {
                id,
                body: Body::Coalescer(random_coalescer(r)),
                routes: vec![Route::to(Target::Unit(target_id))],
            });
            true
        }
        "delete_unit" => {
            if g.units.len() <= 1 {
                return false;
            }
            let i = r.below(g.units.len());
            let gone = g.units.remove(i);
            // Bypass: streams into the deleted unit continue to its treated-side outlet.
            let bypass = gone.routes.last().map(|x| x.to).unwrap_or(Target::Product);
            let bypass = if bypass == Target::Unit(gone.id) {
                Target::Product
            } else {
                bypass
            };
            if g.feed.to == Target::Unit(gone.id) {
                g.feed.to = bypass;
            }
            for u in &mut g.units {
                for route in &mut u.routes {
                    if route.to == Target::Unit(gone.id) {
                        route.to = bypass;
                    }
                }
            }
            true
        }
        "rewire" => {
            let Some(i) = pick_unit(g, r, false) else {
                return false;
            };
            let k = r.below(g.units[i].routes.len());
            let options = targets(g);
            let to = *r.pick(&options).unwrap();
            let d = depths(g);
            let pumped = match to {
                Target::Unit(id) => g.index_of(id).is_some_and(|j| d[j] <= d[i]),
                _ => false,
            };
            g.units[i].routes[k].to = to;
            g.units[i].routes[k].pumped = pumped;
            true
        }
        "duplicate_unit" => {
            let Some(i) = pick_unit(g, r, false) else {
                return false;
            };
            let id = fresh(next_id);
            let mut copy = g.units[i].clone();
            copy.id = id;
            perturb_body(&mut copy.body, r, 0.3);
            // Insert in series after unit i on its treated-side outlet.
            let last = g.units[i].routes.len() - 1;
            let old = g.units[i].routes[last];
            g.units[i].routes[last] = Route {
                to: Target::Unit(id),
                pumped: false,
                channel_mm: old.channel_mm,
            };
            let n = copy.routes.len();
            copy.routes[n - 1] = old;
            if matches!(copy.body, Body::Cyclone(_)) {
                copy.routes[0] = Route::to(Target::Reject);
            }
            g.units.push(copy);
            true
        }
        "edit_profile" => {
            let Some(i) = pick_unit(g, r, true) else {
                return false;
            };
            if let Body::Cyclone(c) = &mut g.units[i].body {
                let x = r.f64();
                if x < 0.35 && c.segments.len() < bounds::SEGMENTS.1 {
                    let at = r.below(c.segments.len() + 1);
                    let prev = if at == 0 {
                        1.0
                    } else {
                        c.segments[at - 1].end_ratio
                    };
                    c.segments.insert(
                        at,
                        Segment {
                            len_ratio: r.log_range(0.3, 10.0),
                            end_ratio: prev * r.range(0.5, 1.15),
                        },
                    );
                } else if x < 0.6 && c.segments.len() > 1 {
                    let at = r.below(c.segments.len());
                    c.segments.remove(at);
                } else {
                    let at = r.below(c.segments.len());
                    jitter(&mut c.segments[at].len_ratio, r, 0.4);
                    jitter(&mut c.segments[at].end_ratio, r, 0.2);
                }
                true
            } else {
                false
            }
        }
        "toggle_pump" => {
            let mut options = Vec::new();
            for (i, u) in g.units.iter().enumerate() {
                for (k, route) in u.routes.iter().enumerate() {
                    if matches!(route.to, Target::Unit(_)) {
                        options.push((i, k));
                    }
                }
            }
            let Some(&(i, k)) = r.pick(&options) else {
                return false;
            };
            g.units[i].routes[k].pumped = !g.units[i].routes[k].pumped;
            true
        }
        _ => false,
    }
}

/// Mutate a genome: maybe one structural operator, then parametric perturbation.
/// Returns the child and the operator names applied.
pub fn mutate(
    parent: &Genome,
    r: &mut Rng,
    next_id: &mut u32,
    structural_rate: f64,
) -> (Genome, Vec<String>) {
    let parent_hash = parent.canonical_sha256();
    for _attempt in 0..6 {
        let mut g = parent.clone();
        let mut ops = Vec::new();
        if r.chance(structural_rate) {
            let total: f64 = STRUCTURAL.iter().map(|o| o.1).sum();
            let mut x = r.f64() * total;
            let mut op = STRUCTURAL[0].0;
            for (name, w) in STRUCTURAL {
                if x < w {
                    op = name;
                    break;
                }
                x -= w;
            }
            if apply_structural(&mut g, op, r, next_id) {
                ops.push(op.to_string());
            }
        }
        let p = (1.5 / (1 + g.units.len() * 8) as f64).max(0.08);
        perturb(&mut g, r, p);
        ops.push("perturb".into());
        if r.chance(0.02) {
            // Rare transposon: replace one body wholesale.
            if let Some(i) = pick_unit(&g, r, false) {
                let outlets = g.units[i].body.outlets();
                let body = random_body(r);
                if body.outlets() == outlets {
                    g.units[i].body = body;
                    ops.push("replace_body".into());
                }
            }
        }
        g.normalize();
        if !g.units.is_empty() && g.canonical_sha256() != parent_hash {
            return (g, ops);
        }
    }
    let mut g = parent.clone();
    perturb(&mut g, r, 1.0);
    g.normalize();
    (g, vec!["perturb_all".into()])
}
