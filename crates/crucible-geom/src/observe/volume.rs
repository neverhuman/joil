//! One material partition. Face reachability is deliberately distinct from
//! closed-cell homology (point and edge contacts do not conduct fluid).
use super::ObserveError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Cell {
    Outside,
    Fluid,
    Solid,
    Unresolved,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PortSeeds {
    pub tag: u16,
    pub cells: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Volume {
    pub encoding: String,
    pub dimensions: [usize; 3],
    pub origin_m: [f64; 3],
    pub pitch_m: f64,
    /// x fastest, then y, then z. Each complete cell has exactly one owner.
    pub cells: Vec<Cell>,
    pub ports: Vec<PortSeeds>,
}
impl Volume {
    pub fn validate(&self) -> Result<(), ObserveError> {
        let n = self
            .dimensions
            .iter()
            .try_fold(1usize, |n, &d| if d == 0 { None } else { n.checked_mul(d) })
            .ok_or(ObserveError::Invalid("volume dimensions"))?;
        if self.encoding != "observer-cubical-volume/1" {
            return Err(ObserveError::Unsupported("volume encoding"));
        }
        if n > 4096
            || self.cells.len() != n
            || !self.pitch_m.is_finite()
            || self.pitch_m <= 0.0
            || self.origin_m.iter().any(|x| !x.is_finite())
        {
            return Err(ObserveError::Invalid("bounded SI volume"));
        }
        if (0..3).any(|a| {
            let end = self.origin_m[a] + self.dimensions[a] as f64 * self.pitch_m;
            !end.is_finite() || end <= self.origin_m[a]
        }) || !self.pitch_m.powi(3).is_normal()
        {
            return Err(ObserveError::Numerical(
                "volume scale or translation not representable",
            ));
        }
        let mut tags = BTreeSet::new();
        for p in &self.ports {
            if p.cells.is_empty()
                || !tags.insert(p.tag)
                || p.cells.iter().any(|&i| i >= n)
                || p.cells.iter().copied().collect::<BTreeSet<_>>().len() != p.cells.len()
            {
                return Err(ObserveError::Invalid("port cell mapping"));
            }
        }
        Ok(())
    }
    pub fn index(&self, p: [usize; 3]) -> usize {
        (p[2] * self.dimensions[1] + p[1]) * self.dimensions[0] + p[0]
    }
    pub fn point(&self, i: usize) -> [usize; 3] {
        [
            i % self.dimensions[0],
            i / self.dimensions[0] % self.dimensions[1],
            i / (self.dimensions[0] * self.dimensions[1]),
        ]
    }
    pub fn centre(&self, i: usize) -> [f64; 3] {
        let p = self.point(i);
        std::array::from_fn(|a| self.origin_m[a] + (p[a] as f64 + 0.5) * self.pitch_m)
    }
    pub(crate) fn neighbours(&self, i: usize) -> Vec<usize> {
        let p = self.point(i);
        let mut out = Vec::with_capacity(6);
        for a in 0..3 {
            for s in [-1isize, 1] {
                let c = p[a] as isize + s;
                if c >= 0 && c < self.dimensions[a] as isize {
                    let mut q = p;
                    q[a] = c as usize;
                    out.push(self.index(q));
                }
            }
        }
        out
    }
    pub fn unresolved_cells(&self) -> Vec<usize> {
        self.cells
            .iter()
            .enumerate()
            .filter_map(|(i, &c)| (c == Cell::Unresolved).then_some(i))
            .collect()
    }
}

pub(crate) fn flood(g: &Volume, seeds: &[usize], allowed: &[bool]) -> Vec<bool> {
    let mut seen = vec![false; g.cells.len()];
    let mut queue = VecDeque::new();
    for &i in seeds {
        if allowed[i] && !seen[i] {
            seen[i] = true;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        for j in g.neighbours(i) {
            if allowed[j] && !seen[j] {
                seen[j] = true;
                queue.push_back(j);
            }
        }
    }
    seen
}
pub(crate) fn components(g: &Volume, allowed: &[bool]) -> Vec<Vec<usize>> {
    let mut seen = vec![false; allowed.len()];
    let mut result = Vec::new();
    for i in 0..allowed.len() {
        if allowed[i] && !seen[i] {
            let found = flood(g, &[i], allowed);
            let group = found
                .iter()
                .enumerate()
                .filter_map(|(j, &v)| v.then_some(j))
                .collect();
            for (s, v) in seen.iter_mut().zip(found) {
                *s |= v;
            }
            result.push(group);
        }
    }
    result
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connectivity {
    pub face_components: usize,
    pub unseeded_components: usize,
    pub unreachable_cells: Vec<usize>,
    pub port_reachability: Vec<Vec<bool>>,
    pub unresolved_cells: Vec<usize>,
    /// False whenever unresolved cells could alter connectivity/attachment.
    pub resolved: bool,
}
/// Fluid ports and solid support seeds use the same independent face flood.
/// Missing seeds are an invalid request, never evidence for attachment.
pub fn connectivity(
    g: &Volume,
    phase: Cell,
    seeds: &[PortSeeds],
) -> Result<Connectivity, ObserveError> {
    g.validate()?;
    if !matches!(phase, Cell::Fluid | Cell::Solid) || seeds.is_empty() {
        return Err(ObserveError::Invalid("phase or absent physical seeds"));
    }
    let mut tags = BTreeSet::new();
    if seeds.iter().any(|p| {
        !tags.insert(p.tag)
            || p.cells.is_empty()
            || p.cells
                .iter()
                .any(|&i| i >= g.cells.len() || g.cells[i] != phase)
    }) {
        return Err(ObserveError::Invalid("physical seeds"));
    }
    let mask: Vec<_> = g.cells.iter().map(|&c| c == phase).collect();
    let groups = components(g, &mask);
    let paths: Vec<_> = seeds.iter().map(|p| flood(g, &p.cells, &mask)).collect();
    let unreachable_cells = (0..mask.len())
        .filter(|&i| mask[i] && paths.iter().all(|p| !p[i]))
        .collect();
    let unseeded_components = groups
        .iter()
        .filter(|g| paths.iter().all(|p| !p[g[0]]))
        .count();
    let port_reachability = paths
        .iter()
        .map(|p| {
            seeds
                .iter()
                .map(|s| s.cells.iter().all(|&i| p[i]))
                .collect()
        })
        .collect();
    let unresolved_cells = g.unresolved_cells();
    let resolved = unresolved_cells.is_empty();
    Ok(Connectivity {
        face_components: groups.len(),
        unseeded_components,
        unreachable_cells,
        port_reachability,
        unresolved_cells,
        resolved,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct PartitionMeasures {
    pub solid_volume_m3: f64,
    pub fluid_volume_m3: f64,
    pub unresolved_volume_m3: f64,
    pub solid_centroid_m: Option<[f64; 3]>,
    /// Integral x_i*x_j dV about origin_m, not inertia about the global origin.
    pub solid_second_moment_m5: [[f64; 3]; 3],
}
/// Exact cubical artifact integrals. They are not bounds on an unobserved CAD
/// solid; unresolved cells and the manifest approximation remain separate.
pub fn measure(g: &Volume) -> Result<PartitionMeasures, ObserveError> {
    g.validate()?;
    let dv = g.pitch_m.powi(3);
    let mut counts = [0usize; 3];
    let mut first = [0.0; 3];
    let mut second = [[0.0; 3]; 3];
    for (i, &c) in g.cells.iter().enumerate() {
        match c {
            Cell::Solid => {
                counts[0] += 1;
                let p = g.point(i).map(|v| (v as f64 + 0.5) * g.pitch_m);
                for a in 0..3 {
                    first[a] += dv * p[a];
                    for b in 0..3 {
                        second[a][b] += dv
                            * (p[a] * p[b]
                                + if a == b {
                                    g.pitch_m.powi(2) / 12.0
                                } else {
                                    0.0
                                });
                    }
                }
            }
            Cell::Fluid => counts[1] += 1,
            Cell::Unresolved => counts[2] += 1,
            _ => {}
        }
    }
    let solid = counts[0] as f64 * dv;
    if !solid.is_finite()
        || first
            .iter()
            .chain(second.iter().flatten())
            .any(|x| !x.is_finite())
    {
        return Err(ObserveError::Numerical("partition integral overflow"));
    }
    Ok(PartitionMeasures {
        solid_volume_m3: solid,
        fluid_volume_m3: counts[1] as f64 * dv,
        unresolved_volume_m3: counts[2] as f64 * dv,
        solid_centroid_m: (solid > 0.0)
            .then(|| std::array::from_fn(|a| g.origin_m[a] + first[a] / solid)),
        solid_second_moment_m5: second,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrozenDifference {
    pub changed_volume_m3: f64,
    pub unresolved_volume_m3: f64,
    pub changed_cells: Vec<usize>,
}
pub fn frozen_difference(
    a: &Volume,
    b: &Volume,
    region: &[usize],
) -> Result<FrozenDifference, ObserveError> {
    a.validate()?;
    b.validate()?;
    if a.dimensions != b.dimensions || a.origin_m != b.origin_m || a.pitch_m != b.pitch_m {
        return Err(ObserveError::Unsupported(
            "frozen grids require explicit registered alignment",
        ));
    }
    if region.is_empty()
        || region.iter().any(|&i| i >= a.cells.len())
        || region.iter().copied().collect::<BTreeSet<_>>().len() != region.len()
    {
        return Err(ObserveError::Invalid("frozen cell region"));
    }
    let changed_cells: Vec<_> = region
        .iter()
        .copied()
        .filter(|&i| {
            a.cells[i] != Cell::Unresolved
                && b.cells[i] != Cell::Unresolved
                && a.cells[i] != b.cells[i]
        })
        .collect();
    let unresolved = region
        .iter()
        .filter(|&&i| a.cells[i] == Cell::Unresolved || b.cells[i] == Cell::Unresolved)
        .count();
    Ok(FrozenDifference {
        changed_volume_m3: changed_cells.len() as f64 * a.pitch_m.powi(3),
        unresolved_volume_m3: unresolved as f64 * a.pitch_m.powi(3),
        changed_cells,
    })
}
