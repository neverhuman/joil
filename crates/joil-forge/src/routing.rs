//! Channel routing between unit chambers and cartridge ports.
//!
//! Every channel leaves its source vertically into a routing gap between bays, runs
//! horizontally on its own level, and, when its destination opens into another gap,
//! travels along a vertical lane that avoids every unit disc and every other vertical
//! channel in the bays it crosses. Blocked horizontal runs detour through one waypoint.
//! Whatever remains blocked is reported as a clearance violation; nothing is silently
//! merged. Channels bound for the same terminal or unit inlet may meet (they mix there).

use crate::layout::Placed;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum End {
    Unit(usize),
    Product,
    Reject,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// `None` for the feed; otherwise `(unit index, outlet port)`.
    pub from: Option<(usize, usize)>,
    pub to: End,
    pub d: f64,
    pub start: [f64; 3],
    pub start_gap: usize,
    pub end: [f64; 3],
    pub end_gap: usize,
    /// Radius of the ring on which a unit inlet may attach (0: fixed point).
    pub end_ring: f64,
    pub end_center: [f64; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub from: Option<(usize, usize)>,
    pub to: End,
    pub d: f64,
    pub pts: Vec<[f64; 3]>,
    pub length: f64,
}

/// Routing gap `g` spans `z0..z1` and owns horizontal levels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gap {
    pub z0: f64,
    pub z1: f64,
    pub levels: usize,
}

struct Vertical {
    owner: usize,
    xy: [f64; 2],
    z0: f64,
    z1: f64,
    r: f64,
}

/// Number of horizontal runs each gap must hold.
pub fn runs_per_gap(reqs: &[(usize, usize)], gaps: usize) -> Vec<usize> {
    let mut n = vec![0usize; gaps];
    for &(a, b) in reqs {
        n[a] += 1;
        if b != a {
            n[b] += 1;
        }
    }
    n
}

fn dist_point_seg(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 <= 0.0 {
        0.0
    } else {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
    };
    (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
}

fn shares_end(a: &Request, b: &Request) -> bool {
    a.to == b.to
}

pub struct Router<'a> {
    pub units: &'a [Placed],
    pub gaps: &'a [Gap],
    pub radius: f64,
    pub wall: f64,
}

impl Router<'_> {
    fn level_z(&self, gap: usize, level: usize) -> f64 {
        let g = &self.gaps[gap];
        g.z0 + (g.z1 - g.z0) * (level as f64 + 1.0) / (g.levels as f64 + 1.0)
    }

    #[allow(clippy::too_many_arguments)]
    fn lane_clear(
        &self,
        xy: [f64; 2],
        r: f64,
        z0: f64,
        z1: f64,
        verts: &[Vertical],
        owner: usize,
        reqs: &[Request],
    ) -> bool {
        if xy[0].hypot(xy[1]) + r + self.wall > self.radius {
            return false;
        }
        let (lo, hi) = (z0.min(z1), z0.max(z1));
        for o in self.units.iter().flat_map(|u| u.obstacles.iter()) {
            if o.z1 >= lo && o.z0 <= hi && (xy[0] - o.x).hypot(xy[1] - o.y) < o.r + r + self.wall {
                return false;
            }
        }
        verts.iter().all(|v| {
            v.owner == owner
                || shares_end(&reqs[v.owner], &reqs[owner])
                || v.z1.min(hi) < v.z0.max(lo)
                || (xy[0] - v.xy[0]).hypot(xy[1] - v.xy[1]) >= r + v.r + self.wall
        })
    }

    /// Route all requests. Returns channels and clearance violations.
    pub fn route(&self, reqs: &mut [Request]) -> (Vec<Channel>, Vec<String>) {
        let mut next_level = vec![0usize; self.gaps.len()];
        let mut verts: Vec<Vertical> = Vec::new();
        let mut plans: Vec<(f64, f64, Option<[f64; 2]>)> = Vec::new();
        let mut violations = Vec::new();
        // Pass 1: levels, inlet attachment angles and vertical segments (drops, lanes).
        for (k, q) in reqs.iter_mut().enumerate() {
            let zd = self.level_z(q.start_gap, next_level[q.start_gap]);
            next_level[q.start_gap] += 1;
            let za = if q.end_gap == q.start_gap {
                zd
            } else {
                let z = self.level_z(q.end_gap, next_level[q.end_gap]);
                next_level[q.end_gap] += 1;
                z
            };
            if q.end_ring > 0.0 {
                let th = (q.start[1] - q.end_center[1]).atan2(q.start[0] - q.end_center[0]);
                q.end[0] = q.end_center[0] + q.end_ring * th.cos();
                q.end[1] = q.end_center[1] + q.end_ring * th.sin();
            }
            let r = 0.5 * q.d;
            verts.push(Vertical {
                owner: k,
                xy: [q.start[0], q.start[1]],
                z0: q.start[2].min(zd),
                z1: q.start[2].max(zd),
                r,
            });
            verts.push(Vertical {
                owner: k,
                xy: [q.end[0], q.end[1]],
                z0: q.end[2].min(za),
                z1: q.end[2].max(za),
                r,
            });
            plans.push((zd, za, None));
        }
        for k in 0..reqs.len() {
            let q = &reqs[k];
            let (zd, za, _) = plans[k];
            if q.end_gap == q.start_gap {
                continue;
            }
            let r = 0.5 * q.d;
            let s = [q.start[0], q.start[1]];
            let e = [q.end[0], q.end[1]];
            let mut best: Option<([f64; 2], f64)> = None;
            let mut ring = self.radius - self.wall - r;
            while ring >= 0.0 {
                let steps = if ring < 1.0 { 1 } else { 72 };
                for a in 0..steps {
                    let th = 2.0 * std::f64::consts::PI * a as f64 / steps as f64;
                    let xy = [ring * th.cos(), ring * th.sin()];
                    if self.lane_clear(xy, r, zd, za, &verts, k, reqs) {
                        let cost =
                            (xy[0] - s[0]).hypot(xy[1] - s[1]) + (xy[0] - e[0]).hypot(xy[1] - e[1]);
                        if best.is_none_or(|b| cost < b.1) {
                            best = Some((xy, cost));
                        }
                    }
                }
                if best.is_some() {
                    break;
                }
                ring -= 2.0;
            }
            match best {
                Some((xy, _)) => {
                    verts.push(Vertical {
                        owner: k,
                        xy,
                        z0: zd.min(za),
                        z1: zd.max(za),
                        r,
                    });
                    plans[k].2 = Some(xy);
                }
                None => violations.push(format!("no clear vertical lane for channel {k}")),
            }
        }
        // Vertical-vertical conflicts between different channels.
        for (i, a) in verts.iter().enumerate() {
            for b in verts.iter().skip(i + 1) {
                if a.owner == b.owner || shares_end(&reqs[a.owner], &reqs[b.owner]) {
                    continue;
                }
                if a.z1.min(b.z1) >= a.z0.max(b.z0)
                    && (a.xy[0] - b.xy[0]).hypot(a.xy[1] - b.xy[1]) < a.r + b.r + self.wall
                {
                    violations.push(format!(
                        "vertical channels {} and {} collide",
                        a.owner, b.owner
                    ));
                }
            }
        }
        // Pass 2: horizontal runs with single-waypoint detours.
        let mut channels = Vec::with_capacity(reqs.len());
        for (k, q) in reqs.iter().enumerate() {
            let (zd, za, lane) = plans[k];
            let r = 0.5 * q.d;
            let s = [q.start[0], q.start[1]];
            let e = [q.end[0], q.end[1]];
            let mut pts = vec![q.start];
            let mut run = |a: [f64; 2], b: [f64; 2], z: f64, pts: &mut Vec<[f64; 3]>| {
                let blockers: Vec<&Vertical> = verts
                    .iter()
                    .filter(|v| {
                        v.owner != k
                            && !shares_end(&reqs[v.owner], q)
                            && v.z0 <= z + r
                            && v.z1 >= z - r
                    })
                    .collect();
                let clear = |p: [f64; 2], c: [f64; 2]| {
                    blockers
                        .iter()
                        .all(|v| dist_point_seg(v.xy, p, c) >= r + v.r + self.wall)
                };
                pts.push([a[0], a[1], z]);
                if !clear(a, b) {
                    let mut best: Option<([f64; 2], f64)> = None;
                    for ri in 1..=8 {
                        let rr = (self.radius - self.wall - r) * ri as f64 / 8.0;
                        for ai in 0..36 {
                            let th = 2.0 * std::f64::consts::PI * ai as f64 / 36.0;
                            let w = [rr * th.cos(), rr * th.sin()];
                            if clear(a, w) && clear(w, b) {
                                let cost = (w[0] - a[0]).hypot(w[1] - a[1])
                                    + (w[0] - b[0]).hypot(w[1] - b[1]);
                                if best.is_none_or(|x| cost < x.1) {
                                    best = Some((w, cost));
                                }
                            }
                        }
                    }
                    match best {
                        Some((w, _)) => pts.push([w[0], w[1], z]),
                        None => {
                            violations.push(format!("horizontal run of channel {k} is blocked"))
                        }
                    }
                }
                pts.push([b[0], b[1], z]);
            };
            match lane {
                Some(l) => {
                    run(s, l, zd, &mut pts);
                    run(l, e, za, &mut pts);
                }
                None => run(s, e, zd, &mut pts),
            }
            pts.push(q.end);
            pts.dedup_by(|a, b| {
                (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs() < 1e-9
            });
            let length = pts
                .windows(2)
                .map(|w| {
                    ((w[1][0] - w[0][0]).powi(2)
                        + (w[1][1] - w[0][1]).powi(2)
                        + (w[1][2] - w[0][2]).powi(2))
                    .sqrt()
                })
                .sum();
            channels.push(Channel {
                from: q.from,
                to: q.to,
                d: q.d,
                pts,
                length,
            });
        }
        (channels, violations)
    }
}
