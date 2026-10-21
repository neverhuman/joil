//! Whole-plant evaluation: flows, unit hydraulics, pressures, breakup and size-resolved
//! oil transport through series, parallel and recycle paths.

use crate::breakup::{breakup_transfer, max_stable_diameter};
use crate::closure::Closure;
use crate::coalescer;
use crate::cyclone::{self, CycloneHydraulics, Resolution};
use crate::flows::{self, Flows};
use crate::fluid::{FeedState, Fluids};
use crate::math::darcy_friction;
use crate::plant::{Dest, Link, Plant, UnitSpec};
use crate::pressure::{self, PressureSolution, Terminals};
use crate::separation;
use crate::sizes::{reduced_cut_size, SizeGrid};
use crate::transfer::Transfer;
use crate::{finite, JudgeError, Result, EVIDENCE_CLASS, MODEL_VERSION};
use serde::{Deserialize, Serialize};

/// Reported stable diameter when no breakup limit applies (µm), keeping reports finite.
pub const STABLE_CAP_UM: f64 = 1.0e4;

fn um(d_m: f64) -> f64 {
    (d_m * 1e6).min(STABLE_CAP_UM)
}

/// Minor-loss coefficient charged to every printed channel (entry, exit and two bends).
pub const CHANNEL_MINOR_K: f64 = 1.5;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub fluids: Fluids,
    pub grid: SizeGrid,
    pub closure: Closure,
    pub resolution: Resolution,
    pub terminals: Terminals,
}

impl Settings {
    pub fn reference() -> Self {
        Self {
            fluids: Fluids::reference(),
            grid: SizeGrid::standard(),
            closure: Closure::nominal(),
            resolution: Resolution::default(),
            terminals: Terminals::default(),
        }
    }
}

pub use crate::evaluation::{LinkReport, PlantEvaluation, UnitReport};

/// Hydraulic state shared by the pressure-only and full evaluations.
struct Hydraulic {
    flows: Flows,
    cyc: Vec<Option<CycloneHydraulics>>,
    coal: Vec<Option<coalescer::CoalescerResponse>>,
    port_drop: Vec<Vec<f64>>,
    link_loss: Vec<Vec<f64>>,
    feed_loss: f64,
    pressures: PressureSolution,
}

fn channel_loss(l: &Link, q: f64, f: &Fluids) -> (f64, f64) {
    if q <= 0.0 {
        return (0.0, 0.0);
    }
    let area = 0.25 * std::f64::consts::PI * l.channel_d_m * l.channel_d_m;
    let v = q / area;
    let re = v * l.channel_d_m / f.nu_water();
    let fd = darcy_friction(re, 20e-6 / l.channel_d_m);
    let dp = (fd * l.length_m / l.channel_d_m + CHANNEL_MINOR_K) * 0.5 * f.rho_water_kg_m3 * v * v;
    (dp, v)
}

fn hydraulic(plant: &Plant, q_feed: f64, s: &Settings) -> Result<Hydraulic> {
    let flows = flows::solve(plant, q_feed)?;
    let n = plant.units.len();
    let (mut cyc, mut coal) = (vec![None; n], vec![None; n]);
    let mut port_drop = Vec::with_capacity(n);
    let mut link_loss = Vec::with_capacity(n);
    for (i, u) in plant.units.iter().enumerate() {
        let q_cell = flows.unit_in[i] / u.spec.cells() as f64;
        let drops = if !flows.active[i] {
            vec![0.0; u.outlets.len()]
        } else {
            match &u.spec {
                UnitSpec::Cyclone(c) => {
                    let h = cyclone::hydraulics(c, q_cell, &s.fluids, &s.closure, s.resolution)?;
                    let d = vec![h.dp_overflow_pa, h.dp_underflow_pa];
                    cyc[i] = Some(h);
                    d
                }
                UnitSpec::Coalescer(c) => {
                    let r = coalescer::respond(c, q_cell, &s.grid, &s.fluids, &s.closure)?;
                    let d = vec![r.dp_pa];
                    coal[i] = Some(r);
                    d
                }
                UnitSpec::Splitter { .. } => vec![0.0, 0.0],
            }
        };
        port_drop.push(drops);
        link_loss.push(
            u.outlets
                .iter()
                .enumerate()
                .map(|(k, l)| channel_loss(l, flows.link[i][k], &s.fluids).0)
                .collect(),
        );
    }
    let feed_loss = channel_loss(&plant.feed, q_feed, &s.fluids).0;
    let pressures = pressure::solve(
        plant,
        &flows,
        &port_drop,
        &link_loss,
        feed_loss,
        &s.terminals,
    )?;
    Ok(Hydraulic {
        flows,
        cyc,
        coal,
        port_drop,
        link_loss,
        feed_loss,
        pressures,
    })
}

/// Budget-relevant pressure (Pa) at feed flow `q_feed` without the separation march:
/// minimum feed pressure plus pump shaft work per unit feed volume.
pub fn feed_pressure(plant: &Plant, q_feed: f64, s: &Settings) -> Result<f64> {
    let p = hydraulic(plant, q_feed, s)?.pressures;
    Ok(p.feed_pa + p.pump_power_w / q_feed)
}

/// Dissipation-limited breakup on a link: channel friction, throttle jet and pump.
fn link_dmax(l: &Link, q: f64, throttle: f64, lift: f64, s: &Settings) -> f64 {
    if q <= 0.0 {
        return f64::INFINITY;
    }
    let f = &s.fluids;
    let rho = f.rho_water_kg_m3;
    let (dp_ch, _) = channel_loss(l, q, f);
    let vol_ch =
        0.25 * std::f64::consts::PI * l.channel_d_m.powi(2) * (l.length_m + 5.0 * l.channel_d_m);
    let mut eps = dp_ch * q / (rho * vol_ch);
    if throttle > 0.0 {
        let vj = (2.0 * throttle / rho).sqrt();
        let dj = (4.0 * q / (std::f64::consts::PI * vj)).sqrt();
        eps = eps.max(throttle * q / (rho * s.closure.jet_volume_factor * dj.powi(3)));
    }
    if lift > 0.0 {
        let p_shaft = lift * q / s.terminals.pump_efficiency;
        eps = eps.max(p_shaft / (rho * s.closure.pump_dissipation_volume_m3));
    }
    max_stable_diameter(eps, f, s.closure.davies_c)
}

/// Full evaluation of a plant at one feed state.
pub fn evaluate(plant: &Plant, feed: &FeedState, s: &Settings) -> Result<PlantEvaluation> {
    s.fluids.validate()?;
    feed.validate()?;
    let q_feed = feed.q_m3_s();
    let h = hydraulic(plant, q_feed, s)?;
    let n = plant.units.len();
    let nc = s.grid.n();
    let mut units = Vec::new();
    // port operators T[u][k] (unit physics) and full link operators C = B_link ∘ T
    let mut ops: Vec<Vec<Transfer>> = Vec::with_capacity(n);
    let mut min_dmax = f64::INFINITY;
    for (i, u) in plant.units.iter().enumerate() {
        if !h.flows.active[i] {
            ops.push(vec![Transfer::diagonal(&vec![0.0; nc]); u.outlets.len()]);
            continue;
        }
        let q_cell = h.flows.unit_in[i] / u.spec.cells() as f64;
        let mut report = UnitReport {
            index: i,
            kind: String::new(),
            cells: u.spec.cells(),
            q_in_m3_s: h.flows.unit_in[i],
            inlet_pa: h.pressures.unit_inlet_pa[i],
            port_drop_pa: h.port_drop[i].clone(),
            eps_peak_w_kg: 0.0,
            d_max_um: STABLE_CAP_UM,
            residence_s: 0.0,
            inlet_velocity_m_s: 0.0,
            peak_wall_g: 0.0,
            capture: Vec::new(),
            reduced_d50_um: None,
            tracer_error: 0.0,
            budget: None,
            oil_in_kg_s: 0.0,
            inlet_dsd: Vec::new(),
        };
        let port_ops = match &u.spec {
            UnitSpec::Cyclone(c) => {
                let hy = h.cyc[i]
                    .as_ref()
                    .ok_or_else(|| JudgeError::Numerical("missing hydraulics".into()))?;
                let d_max = max_stable_diameter(hy.eps_peak_w_kg, &s.fluids, s.closure.davies_c);
                let pre = breakup_transfer(&s.grid, d_max, s.closure.satellite_fraction);
                let sep =
                    separation::separate(c, hy, &s.grid, &s.fluids, &s.closure, s.resolution.ns)?;
                let under: Vec<f64> = sep.capture.iter().map(|e| 1.0 - e).collect();
                report.kind = "cyclone".into();
                report.eps_peak_w_kg = hy.eps_peak_w_kg;
                report.d_max_um = um(d_max);
                report.residence_s = hy.residence_s;
                report.inlet_velocity_m_s = hy.inlet_velocity_m_s;
                report.peak_wall_g = hy.peak_wall_g;
                report.reduced_d50_um =
                    reduced_cut_size(&s.grid, &sep.capture, c.split, 0.5).map(|d| d * 1e6);
                report.tracer_error = (sep.tracer_capture - c.split).abs();
                report.budget = Some(hy.budget);
                report.capture = sep.capture.clone();
                min_dmax = min_dmax.min(d_max);
                vec![pre.scale_outputs(&sep.capture), pre.scale_outputs(&under)]
            }
            UnitSpec::Coalescer(_) => {
                let r = h.coal[i]
                    .as_ref()
                    .ok_or_else(|| JudgeError::Numerical("missing bed".into()))?;
                let d_max = max_stable_diameter(r.eps_w_kg, &s.fluids, s.closure.davies_c);
                report.kind = "coalescer".into();
                report.eps_peak_w_kg = r.eps_w_kg;
                report.d_max_um = um(d_max);
                if let UnitSpec::Coalescer(c) = &u.spec {
                    report.residence_s = c.cell_volume() * c.porosity / q_cell;
                }
                report.capture = r.capture.clone();
                min_dmax = min_dmax.min(d_max);
                vec![r.transfer.clone()]
            }
            UnitSpec::Splitter { fraction } => {
                report.kind = "splitter".into();
                vec![
                    Transfer::identity(nc).scaled(*fraction),
                    Transfer::identity(nc).scaled(1.0 - fraction),
                ]
            }
        };
        let mut composed = Vec::with_capacity(port_ops.len());
        for (k, t) in port_ops.into_iter().enumerate() {
            let l = &u.outlets[k];
            let d = link_dmax(
                l,
                h.flows.link[i][k],
                h.pressures.throttle_pa[i][k],
                h.pressures.lift_pa[i][k],
                s,
            );
            if matches!(l.to, Dest::Unit(_)) {
                // Breakup after the last separation does not change product oil mass.
                min_dmax = min_dmax.min(d);
            }
            composed.push(t.then(&breakup_transfer(&s.grid, d, s.closure.satellite_fraction)));
        }
        ops.push(composed);
        units.push(report);
    }
    let feed_mass: Vec<f64> = s
        .grid
        .lognormal_fractions(&feed.dsd)
        .iter()
        .map(|w| w * feed.oil_kg_s())
        .collect();
    let feed_dmax = link_dmax(&plant.feed, q_feed, 0.0, 0.0, s);
    let feed_op = breakup_transfer(&s.grid, feed_dmax, s.closure.satellite_fraction);
    let feed_in = feed_op.apply(&feed_mass);
    let (m_in, iterations) = propagate(plant, &h.flows, &ops, &feed_in, nc)?;
    let (mut m_p, mut m_r) = (vec![0.0; nc], vec![0.0; nc]);
    match plant.feed.to {
        Dest::Product => m_p.iter_mut().zip(&feed_in).for_each(|(a, b)| *a += b),
        Dest::Reject => m_r.iter_mut().zip(&feed_in).for_each(|(a, b)| *a += b),
        Dest::Unit(_) => {}
    }
    let mut links = vec![LinkReport {
        from: None,
        port: 0,
        to: plant.feed.to,
        flow_m3_s: q_feed,
        velocity_m_s: channel_loss(&plant.feed, q_feed, &s.fluids).1,
        channel_loss_pa: h.feed_loss,
        throttle_pa: 0.0,
        lift_pa: 0.0,
        d_max_um: um(feed_dmax),
        oil_kg_s: feed_in.iter().sum(),
    }];
    for (i, u) in plant.units.iter().enumerate() {
        for (k, l) in u.outlets.iter().enumerate() {
            let out = ops[i][k].apply(&m_in[i]);
            match l.to {
                Dest::Product => m_p.iter_mut().zip(&out).for_each(|(a, b)| *a += b),
                Dest::Reject => m_r.iter_mut().zip(&out).for_each(|(a, b)| *a += b),
                Dest::Unit(_) => {}
            }
            if h.flows.active[i] {
                let q = h.flows.link[i][k];
                links.push(LinkReport {
                    from: Some(i),
                    port: k,
                    to: l.to,
                    flow_m3_s: q,
                    velocity_m_s: channel_loss(l, q, &s.fluids).1,
                    channel_loss_pa: h.link_loss[i][k],
                    throttle_pa: h.pressures.throttle_pa[i][k],
                    lift_pa: h.pressures.lift_pa[i][k],
                    d_max_um: um(link_dmax(
                        l,
                        q,
                        h.pressures.throttle_pa[i][k],
                        h.pressures.lift_pa[i][k],
                        s,
                    )),
                    oil_kg_s: out.iter().sum(),
                });
            }
        }
    }
    for r in &mut units {
        r.oil_in_kg_s = m_in[r.index].iter().sum();
        r.inlet_dsd = m_in[r.index].clone();
    }
    let total_in: f64 = feed_mass.iter().sum();
    let (sum_p, sum_r): (f64, f64) = (m_p.iter().sum(), m_r.iter().sum());
    let balance = if total_in > 0.0 {
        (sum_p + sum_r - total_in).abs() / total_in
    } else {
        0.0
    };
    if balance > 1e-8 {
        return Err(JudgeError::Numerical(format!(
            "oil balance error {balance:e}"
        )));
    }
    if h.flows.q_product <= 0.0 {
        return Err(JudgeError::Hydraulic(
            "no treated-water product stream".into(),
        ));
    }
    let cout = 1000.0 * sum_p / h.flows.q_product;
    Ok(PlantEvaluation {
        model_version: MODEL_VERSION.into(),
        evidence_class: EVIDENCE_CLASS.into(),
        feed: *feed,
        q_product_m3_s: h.flows.q_product,
        q_reject_m3_s: h.flows.q_reject,
        reject_fraction: h.flows.q_reject / q_feed,
        cout_mg_l: finite("cout", cout)?,
        oil_capture: if total_in > 0.0 {
            sum_r / total_in
        } else {
            0.0
        },
        feed_pressure_pa: h.pressures.feed_pa,
        equivalent_pressure_pa: h.pressures.feed_pa + h.pressures.pump_power_w / q_feed,
        pump_power_w: h.pressures.pump_power_w,
        specific_energy_j_m3: (h.pressures.feed_pa * q_feed + h.pressures.pump_power_w)
            / h.flows.q_product,
        amplification: h.flows.amplification,
        oil_balance_rel_error: balance,
        oil_iterations: iterations,
        min_d_max_um: um(min_dmax.min(feed_dmax)),
        feed_dsd: feed_mass,
        product_dsd: m_p,
        reject_dsd: m_r,
        units,
        links,
    })
}

/// Gauss–Seidel propagation of oil mass through the graph, in depth-first order from the feed.
fn propagate(
    plant: &Plant,
    flows: &Flows,
    ops: &[Vec<Transfer>],
    feed_in: &[f64],
    nc: usize,
) -> Result<(Vec<Vec<f64>>, usize)> {
    let n = plant.units.len();
    let mut incoming: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for (i, u) in plant.units.iter().enumerate() {
        for (k, l) in u.outlets.iter().enumerate() {
            if let Dest::Unit(j) = l.to {
                incoming[j].push((i, k));
            }
        }
    }
    let order = dfs_order(plant);
    let scale: f64 = feed_in.iter().sum::<f64>().max(1e-300);
    let mut m = vec![vec![0.0; nc]; n];
    for it in 1..=4000 {
        let mut delta: f64 = 0.0;
        for &u in &order {
            if !flows.active[u] {
                continue;
            }
            let mut next = vec![0.0; nc];
            if plant.feed.to == Dest::Unit(u) {
                next.copy_from_slice(feed_in);
            }
            for &(v, k) in &incoming[u] {
                ops[v][k].apply_add(&m[v], &mut next);
            }
            for (a, b) in next.iter().zip(&m[u]) {
                delta = delta.max((a - b).abs());
            }
            m[u] = next;
        }
        if delta <= 1e-14 * scale {
            return Ok((m, it));
        }
    }
    Err(JudgeError::Numerical(
        "oil recycle propagation did not converge".into(),
    ))
}

fn dfs_order(plant: &Plant) -> Vec<usize> {
    let n = plant.units.len();
    let mut seen = vec![false; n];
    let mut post = Vec::with_capacity(n);
    fn visit(u: usize, plant: &Plant, seen: &mut [bool], post: &mut Vec<usize>) {
        if seen[u] {
            return;
        }
        seen[u] = true;
        for l in &plant.units[u].outlets {
            if let Dest::Unit(v) = l.to {
                visit(v, plant, seen, post);
            }
        }
        post.push(u);
    }
    if let Dest::Unit(s) = plant.feed.to {
        visit(s, plant, &mut seen, &mut post);
    }
    for u in 0..n {
        visit(u, plant, &mut seen, &mut post);
    }
    post.reverse();
    post
}
