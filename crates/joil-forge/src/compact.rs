//! Compaction and print gates applied during development: folding consecutive bays side
//! by side when they fit the build footprint, and the gates that depend only on the
//! hardware (narrowest swirl bore, cell length in swirl-chamber diameters).

use crate::develop::Phenotype;
use crate::genome::{Body, Genome};
use crate::layout::{inlet_channel, pack_discs, unit_shape, Envelope};

/// Merge consecutive bays while all their units still pack inside the footprint.
pub(crate) fn fold(g: &Genome, env: &Envelope, stacked: &[usize]) -> Vec<usize> {
    let t = env.wall_mm;
    let d_in = inlet_channel(g);
    let foot: Vec<f64> = (0..g.units.len())
        .map(|i| unit_shape(g, i, d_in[i], t).footprint)
        .collect();
    let nb = stacked.iter().copied().max().unwrap_or(0) + 1;
    let mut map = vec![0usize; nb];
    let mut members: Vec<usize> = Vec::new();
    let mut bay = 0;
    for (b, slot) in map.iter_mut().enumerate() {
        let add: Vec<usize> = (0..g.units.len()).filter(|i| stacked[*i] == b).collect();
        let trial: Vec<f64> = members.iter().chain(&add).map(|i| foot[*i]).collect();
        if b > 0 && pack_discs(&trial, env.radius_mm).is_none() {
            bay += 1;
            members.clear();
        }
        members.extend(add);
        *slot = bay;
    }
    stacked.iter().map(|b| map[*b]).collect()
}

/// Print and applicability gates that depend only on the hardware: the narrowest swirl
/// passage and the cell length in swirl-chamber diameters.
pub(crate) fn print_gates(g: &Genome, env: &Envelope, ph: &mut Phenotype) {
    for u in &g.units {
        if let Body::Cyclone(c) = &u.body {
            let d = c.diameter_mm;
            let bore = c.segments.iter().map(|s| s.end_ratio * d).fold(d, f64::min);
            if bore < env.min_bore_mm {
                ph.violations.push(format!(
                    "unit {}: {bore:.2} mm bore is below the {:.1} mm minimum",
                    u.id, env.min_bore_mm
                ));
                ph.excess += env.min_bore_mm / bore.max(1e-3) - 1.0;
            }
            let aspect = c.chamber_len_ratio + c.segments.iter().map(|s| s.len_ratio).sum::<f64>();
            if aspect > env.max_cell_aspect {
                ph.violations.push(format!(
                    "unit {}: cell is {aspect:.1} diameters long, beyond {:.0}",
                    u.id, env.max_cell_aspect
                ));
                ph.excess += aspect / env.max_cell_aspect - 1.0;
            }
        }
    }
}
