//! Finite-radius escape of cell-centre sites, not an assertion of literal zero
//! residual powder. Erosion uses Euclidean distances to blocked cell boxes.
use super::{
    volume::{self, Cell, Volume},
    ObserveError,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ManufacturingProfile {
    pub profile_sha256: crucible_contract::subject::BlobDigest,
    pub escape_diameter_m: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClearanceGate {
    Accessible,
    Inaccessible,
    Unresolved,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TargetPath {
    pub target_cell: usize,
    /// Entire centre-to-centre polyline has at least this clearance diameter.
    pub bottleneck_lower_m: f64,
    pub path_cells: Vec<usize>,
    pub gate: ClearanceGate,
}
#[derive(Clone, Debug, PartialEq)]
pub struct EscapeReport {
    pub profile_sha256: crucible_contract::subject::BlobDigest,
    pub paths: Vec<TargetPath>,
    pub isolated_void_components: usize,
    pub inaccessible_site_volume_m3: f64,
    pub unresolved_cells: Vec<usize>,
    pub gate: ClearanceGate,
}

/// Conservative clearance at every centre and along every incident half edge:
/// distance to the union of all non-fluid boxes and outside-domain minus h/2
/// and the manifest bound. Distance is 1-Lipschitz, so this bounds each edge.
pub fn radii(g: &Volume, error_m: f64) -> Result<Vec<f64>, ObserveError> {
    g.validate()?;
    if !error_m.is_finite() || error_m < 0.0 {
        return Err(ObserveError::Invalid("clearance bound"));
    }
    let mut radii = vec![0.0; g.cells.len()];
    for (i, r) in radii.iter_mut().enumerate() {
        if g.cells[i] != Cell::Fluid {
            continue;
        }
        let p = g.point(i);
        let mut d = (0..3)
            .map(|a| {
                ((p[a] as f64 + 0.5) * g.pitch_m)
                    .min((g.dimensions[a] as f64 - p[a] as f64 - 0.5) * g.pitch_m)
            })
            .fold(f64::INFINITY, f64::min);
        for (j, &c) in g.cells.iter().enumerate() {
            if c != Cell::Fluid {
                let q = g.point(j);
                let dist = (0..3)
                    .map(|a| {
                        (((p[a] as f64 - q[a] as f64).abs() - 0.5) * g.pitch_m)
                            .max(0.0)
                            .powi(2)
                    })
                    .sum::<f64>()
                    .sqrt();
                d = d.min(dist);
            }
        }
        let arithmetic_allowance =
            64.0 * f64::EPSILON * g.pitch_m * (*g.dimensions.iter().max().unwrap() as f64);
        *r = (d - g.pitch_m / 2.0 - error_m - arithmetic_allowance).max(0.0);
    }
    Ok(radii)
}

/// A declared escape boundary must have represented exterior extension; the
/// domain edge is blocked, so a seed alone never invents space outside the mesh.
pub fn escape(
    g: &Volume,
    profile: &ManufacturingProfile,
    escape_seeds: &[usize],
    targets: &[usize],
    error_m: f64,
) -> Result<EscapeReport, ObserveError> {
    g.validate()?;
    if !profile.escape_diameter_m.is_finite()
        || profile.escape_diameter_m <= 0.0
        || escape_seeds.is_empty()
        || targets.is_empty()
        || escape_seeds
            .iter()
            .chain(targets)
            .any(|&i| i >= g.cells.len() || g.cells[i] != Cell::Fluid)
    {
        return Err(ObserveError::Invalid(
            "powder profile, escape or target sites",
        ));
    }
    let radii = radii(g, error_m)?;
    let allowed: Vec<_> = g.cells.iter().map(|&c| c == Cell::Fluid).collect();
    let reach = volume::flood(g, escape_seeds, &allowed);
    let isolated_void_components = volume::components(g, &allowed)
        .iter()
        .filter(|c| !reach[c[0]])
        .count();
    let mut best = vec![-1.0; g.cells.len()];
    let mut parent = vec![None; g.cells.len()];
    let mut done = vec![false; g.cells.len()];
    for &s in escape_seeds {
        best[s] = radii[s];
    }
    for _ in 0..g.cells.len() {
        let Some(i) = (0..best.len())
            .filter(|&i| !done[i] && best[i] >= 0.0)
            .max_by(|&a, &b| best[a].total_cmp(&best[b]))
        else {
            break;
        };
        done[i] = true;
        for j in g.neighbours(i) {
            if allowed[j] && !done[j] {
                let next = best[i].min(radii[j]);
                if next > best[j] {
                    best[j] = next;
                    parent[j] = Some(i);
                }
            }
        }
    }
    let unresolved_cells = g.unresolved_cells();
    let unresolved = !unresolved_cells.is_empty();
    let paths = targets
        .iter()
        .map(|&target| {
            let diameter = 2.0 * best[target].max(0.0);
            let mut path = Vec::new();
            if best[target] >= 0.0 {
                let mut i = target;
                path.push(i);
                while let Some(p) = parent[i] {
                    path.push(p);
                    i = p;
                }
            }
            let gate = if unresolved {
                ClearanceGate::Unresolved
            } else if diameter >= profile.escape_diameter_m {
                ClearanceGate::Accessible
            } else {
                ClearanceGate::Inaccessible
            };
            TargetPath {
                target_cell: target,
                bottleneck_lower_m: diameter,
                path_cells: path,
                gate,
            }
        })
        .collect::<Vec<_>>();
    let gate = if unresolved {
        ClearanceGate::Unresolved
    } else if paths.iter().all(|p| p.gate == ClearanceGate::Accessible) {
        ClearanceGate::Accessible
    } else {
        ClearanceGate::Inaccessible
    };
    let inaccessible_site_volume_m3 = (0..best.len())
        .filter(|&i| allowed[i] && 2.0 * best[i].max(0.0) < profile.escape_diameter_m)
        .count() as f64
        * g.pitch_m.powi(3);
    Ok(EscapeReport {
        profile_sha256: profile.profile_sha256.clone(),
        paths,
        isolated_void_components,
        inaccessible_site_volume_m3,
        unresolved_cells,
        gate,
    })
}
