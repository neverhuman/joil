//! Homologous crossover by innovation identity (NEAT-style alignment).
//!
//! Units present in both parents are matched by `id`; the child takes each matched unit's
//! body and wiring from a random parent, sometimes blending continuous genes. Units found
//! in only one parent are inherited from the fitter parent, and from the other parent with
//! probability one half. Normalisation repairs dangling references.

use crate::genome::{Body, Genome, UnitGene};
use crate::rng::Rng;
use std::collections::BTreeMap;

fn blend(a: f64, b: f64, r: &mut Rng) -> f64 {
    let t = r.f64();
    (a.ln() * t + b.ln() * (1.0 - t)).exp()
}

fn blend_body(a: &Body, b: &Body, r: &mut Rng) -> Body {
    match (a, b) {
        (Body::Cyclone(x), Body::Cyclone(y)) => {
            let mut c = x.clone();
            c.diameter_mm = blend(x.diameter_mm, y.diameter_mm, r);
            c.chamber_len_ratio = blend(x.chamber_len_ratio, y.chamber_len_ratio, r);
            c.inlet_area_ratio = blend(x.inlet_area_ratio, y.inlet_area_ratio, r);
            c.inlet_aspect = blend(x.inlet_aspect, y.inlet_aspect, r);
            c.orifice_ratio = blend(x.orifice_ratio, y.orifice_ratio, r);
            c.split = blend(x.split, y.split, r);
            if x.segments.len() == y.segments.len() {
                for (s, t) in c.segments.iter_mut().zip(&y.segments) {
                    s.len_ratio = blend(s.len_ratio, t.len_ratio, r);
                    s.end_ratio = blend(s.end_ratio, t.end_ratio, r);
                }
            }
            Body::Cyclone(c)
        }
        _ => {
            if r.chance(0.5) {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

/// Child of `fit` (the fitter parent) and `other`.
pub fn crossover(fit: &Genome, other: &Genome, r: &mut Rng) -> Genome {
    let b: BTreeMap<u32, &UnitGene> = other.units.iter().map(|u| (u.id, u)).collect();
    let mut units = Vec::new();
    for u in &fit.units {
        match b.get(&u.id) {
            Some(v) => {
                let take_other = r.chance(0.5);
                let src = if take_other { *v } else { u };
                let mut child = src.clone();
                if r.chance(0.3) {
                    child.body = blend_body(&u.body, &v.body, r);
                    if child.body.outlets() != child.routes.len() {
                        child.routes = src.routes.clone();
                        child.body = src.body.clone();
                    }
                }
                units.push(child);
            }
            None => units.push(u.clone()),
        }
    }
    for v in &other.units {
        if fit.index_of(v.id).is_none() && r.chance(0.5) {
            units.push(v.clone());
        }
    }
    let feed = if r.chance(0.8) { fit.feed } else { other.feed };
    Genome {
        schema: fit.schema.clone(),
        feed,
        units,
    }
    .normalized()
}
