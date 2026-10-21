//! Random genomes for de novo search, and optional hand-written archetypes.
//!
//! De novo search starts only from [`random_genome`]. Archetypes are agent-authored
//! comparison designs; campaigns use them only when `seeding` asks for them, and the
//! CT-35 archetype is the forge expression of the baseline.

use crate::genome::{
    bounds, Body, CoalescerGene, CycloneGene, Genome, Route, Segment, Target, UnitGene, SCHEMA,
};
use crate::rng::Rng;

fn channel(r: &mut Rng) -> f64 {
    r.log_range(4.0, 16.0)
}

pub fn random_cyclone(r: &mut Rng) -> CycloneGene {
    let d = r.log_range(bounds::DIAMETER_MM.0, bounds::DIAMETER_MM.1);
    let nseg = 1 + r.below(4);
    let mut end = 1.0;
    let segments = (0..nseg)
        .map(|_| {
            end = if r.chance(0.8) {
                end * r.range(0.3, 1.0)
            } else {
                r.range(0.3, 1.2)
            };
            Segment {
                len_ratio: r.log_range(0.5, 25.0),
                end_ratio: end.max(0.12),
            }
        })
        .collect();
    let max_cells = ((bounds::DIAMETER_MM.1 / d).powi(2) as u32).clamp(1, 12);
    CycloneGene {
        diameter_mm: d,
        chamber_len_ratio: r.range(0.4, 2.0),
        segments,
        jets: r.int(1, 6),
        inlet_area_ratio: r.log_range(0.02, 0.3),
        inlet_aspect: r.range(0.5, 3.0),
        involute: r.chance(0.3),
        orifice_ratio: r.log_range(0.02, 0.2),
        split: r.log_range(0.005, 0.2),
        cells: if r.chance(0.5) {
            1
        } else {
            r.int(1, max_cells)
        },
    }
}

pub fn random_coalescer(r: &mut Rng) -> CoalescerGene {
    CoalescerGene {
        diameter_mm: r.range(20.0, 70.0),
        length_mm: r.log_range(30.0, 400.0),
        porosity: r.range(0.6, 0.92),
        strut_mm: r.range(1.0, 2.5),
        cells: 1,
    }
}

pub fn random_body(r: &mut Rng) -> Body {
    let x = r.f64();
    if x < 0.78 {
        Body::Cyclone(random_cyclone(r))
    } else if x < 0.88 {
        Body::Coalescer(random_coalescer(r))
    } else {
        Body::Splitter {
            fraction: r.range(0.2, 0.8),
        }
    }
}

/// A random network of 1–4 units with mostly forward wiring and occasional pumped recycles.
pub fn random_genome(r: &mut Rng, first_id: u32) -> Genome {
    let n = [1usize, 1, 2, 2, 2, 3, 3, 4][r.below(8)];
    let ids: Vec<u32> = (0..n as u32).map(|k| first_id + k).collect();
    let mut units = Vec::with_capacity(n);
    for (pos, id) in ids.iter().enumerate() {
        let body = random_body(r);
        let later: Vec<u32> = ids[pos + 1..].to_vec();
        let earlier: Vec<u32> = ids[..pos].to_vec();
        let routes = (0..body.outlets())
            .map(|k| {
                let reject_side = matches!(body, Body::Cyclone(_)) && k == 0;
                let x = r.f64();
                let (to, pumped) = if reject_side {
                    if x < 0.75 || (later.is_empty() && earlier.is_empty()) {
                        (Target::Reject, false)
                    } else if x < 0.92 && !later.is_empty() {
                        (Target::Unit(*r.pick(&later).unwrap()), false)
                    } else if let Some(e) = r.pick(&earlier) {
                        (Target::Unit(*e), true)
                    } else {
                        (Target::Reject, false)
                    }
                } else if !later.is_empty() && x < 0.7 {
                    (Target::Unit(*r.pick(&later).unwrap()), false)
                } else if !earlier.is_empty() && x < 0.78 {
                    (Target::Unit(*r.pick(&earlier).unwrap()), true)
                } else {
                    (Target::Product, false)
                };
                Route {
                    to,
                    pumped,
                    channel_mm: channel(r),
                }
            })
            .collect();
        units.push(UnitGene {
            id: *id,
            body,
            routes,
        });
    }
    Genome {
        schema: SCHEMA.into(),
        feed: Route {
            to: Target::Unit(ids[0]),
            pumped: false,
            channel_mm: r.log_range(8.0, 20.0),
        },
        units,
    }
    .normalized()
}

fn ct35_gene(split: f64, len_scale: f64) -> CycloneGene {
    // Colman–Thew ratios of baselines/ct35/genome.json on D1 = 70 mm.
    let cone = 0.5 * (70.0 - 35.0) / 10.0_f64.to_radians().tan() / 70.0;
    CycloneGene {
        diameter_mm: 70.0,
        chamber_len_ratio: 1.0,
        segments: vec![
            Segment {
                len_ratio: cone,
                end_ratio: 0.5,
            },
            Segment {
                len_ratio: 11.0 * len_scale,
                end_ratio: 0.25,
            },
            Segment {
                len_ratio: 10.0 * len_scale,
                end_ratio: 0.25,
            },
        ],
        jets: 2,
        inlet_area_ratio: 0.0625,
        inlet_aspect: 2.0,
        involute: false,
        orifice_ratio: 0.02,
        split,
        cells: 1,
    }
}

fn unit(id: u32, body: Body, routes: Vec<Target>) -> UnitGene {
    UnitGene {
        id,
        body,
        routes: routes
            .into_iter()
            .map(|t| Route {
                to: t,
                pumped: false,
                channel_mm: 8.0,
            })
            .collect(),
    }
}

fn wrap(units: Vec<UnitGene>) -> Genome {
    Genome {
        schema: SCHEMA.into(),
        feed: Route {
            to: Target::Unit(units[0].id),
            pumped: false,
            channel_mm: 10.0,
        },
        units,
    }
    .normalized()
}

/// Named hand-written comparison designs (agent-authored; not used by de novo search).
pub fn archetypes() -> Vec<(&'static str, Genome)> {
    let mini = |d: f64, cells: u32, split: f64| {
        let mut c = ct35_gene(split, 0.25);
        let s = d / 70.0;
        c.diameter_mm = d;
        c.orifice_ratio = 0.02 / s.sqrt();
        c.cells = cells;
        c
    };
    let mut scav = mini(30.0, 1, 0.25);
    scav.orifice_ratio = 0.08;
    let mut recycle = vec![
        unit(
            1,
            Body::Cyclone(ct35_gene(0.06, 0.5)),
            vec![Target::Unit(2), Target::Product],
        ),
        unit(
            2,
            Body::Cyclone(scav),
            vec![Target::Reject, Target::Unit(1)],
        ),
    ];
    recycle[1].routes[1].pumped = true;
    vec![
        (
            "ct35",
            wrap(vec![unit(
                1,
                Body::Cyclone(ct35_gene(0.02, 1.0)),
                vec![Target::Reject, Target::Product],
            )]),
        ),
        (
            "series-pair",
            wrap(vec![
                unit(
                    1,
                    Body::Cyclone(ct35_gene(0.01, 0.45)),
                    vec![Target::Reject, Target::Unit(2)],
                ),
                unit(
                    2,
                    Body::Cyclone(ct35_gene(0.01, 0.45)),
                    vec![Target::Reject, Target::Product],
                ),
            ]),
        ),
        (
            "mini-cells",
            wrap(vec![unit(
                1,
                Body::Cyclone(mini(16.0, 7, 0.02)),
                vec![Target::Reject, Target::Product],
            )]),
        ),
        ("scavenger-recycle", wrap(recycle)),
        (
            "coalescer-first",
            wrap(vec![
                unit(
                    1,
                    Body::Coalescer(CoalescerGene {
                        diameter_mm: 60.0,
                        length_mm: 200.0,
                        porosity: 0.8,
                        strut_mm: 1.2,
                        cells: 1,
                    }),
                    vec![Target::Unit(2)],
                ),
                unit(
                    2,
                    Body::Cyclone(ct35_gene(0.02, 0.85)),
                    vec![Target::Reject, Target::Product],
                ),
            ]),
        ),
    ]
}
