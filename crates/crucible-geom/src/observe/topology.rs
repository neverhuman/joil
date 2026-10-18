//! Two finite volume chain complexes over GF(2): closed cubes and a conforming
//! six-tetrahedron subdivision. Boundary surfaces have a separate result type.
use super::{
    mesh,
    volume::{Cell, Volume},
    Mesh, ObserveError,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VolumeBetti(pub [usize; 3]);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundaryBetti(pub [usize; 3]);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VolumeTopology {
    pub cubical: VolumeBetti,
    pub tetrahedral: VolumeBetti,
    pub cubical_cells: [usize; 4],
    pub tetrahedral_cells: [usize; 4],
}
impl VolumeTopology {
    pub fn agrees(&self) -> bool {
        self.cubical == self.tetrahedral
    }
}

// Sparse column reduction, with explicit work and size limits. Exhausting a
// numerical budget never returns a partial Betti tuple or physical failure.
fn rank(columns: Vec<Vec<usize>>) -> Result<usize, ObserveError> {
    let mut pivots = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut work = 0usize;
    for col in columns {
        let mut c: BTreeSet<_> = col.into_iter().collect();
        while let Some(&p) = c.last() {
            work = work
                .checked_add(c.len())
                .ok_or(ObserveError::Unsupported("homology work overflow"))?;
            if work > 20_000_000 {
                return Err(ObserveError::Unsupported("homology reduction budget"));
            }
            if let Some(old) = pivots.get(&p) {
                c = c.symmetric_difference(old).copied().collect();
            } else {
                pivots.insert(p, c);
                break;
            }
        }
    }
    Ok(pivots.len())
}
type Cube = ([usize; 3], u8);
fn cube_boundary((p, mask): Cube) -> Vec<Cube> {
    let mut out = Vec::new();
    for a in 0..3 {
        if mask & (1 << a) != 0 {
            let m = mask & !(1 << a);
            out.push((p, m));
            let mut q = p;
            q[a] += 1;
            out.push((q, m));
        }
    }
    out
}
fn cubical(g: &Volume, phase: Cell) -> Result<([usize; 3], [usize; 4]), ObserveError> {
    let mut cells: [BTreeSet<Cube>; 4] = std::array::from_fn(|_| BTreeSet::new());
    for (i, &c) in g.cells.iter().enumerate() {
        if c == phase {
            cells[3].insert((g.point(i), 7));
        }
    }
    for d in (1..=3).rev() {
        for c in cells[d].clone() {
            cells[d - 1].extend(cube_boundary(c));
        }
    }
    let sizes = cells.each_ref().map(|c| c.len());
    let mut ranks = [0usize; 4];
    for d in 1..=3 {
        let rows: BTreeMap<_, _> = cells[d - 1]
            .iter()
            .enumerate()
            .map(|(i, c)| (*c, i))
            .collect();
        ranks[d] = rank(
            cells[d]
                .iter()
                .map(|&c| cube_boundary(c).iter().map(|b| rows[b]).collect())
                .collect(),
        )?;
    }
    Ok((betti(sizes, ranks)?, sizes))
}
fn simplex_boundary(c: &[usize]) -> Vec<Vec<usize>> {
    (0..c.len())
        .map(|skip| {
            c.iter()
                .enumerate()
                .filter_map(|(i, &v)| (i != skip).then_some(v))
                .collect()
        })
        .collect()
}
fn simplicial(cells: &[BTreeSet<Vec<usize>>; 4]) -> Result<([usize; 3], [usize; 4]), ObserveError> {
    let sizes = cells.each_ref().map(|c| c.len());
    let mut ranks = [0usize; 4];
    for d in 1..=3 {
        let rows: BTreeMap<_, _> = cells[d - 1]
            .iter()
            .enumerate()
            .map(|(i, c)| (c.clone(), i))
            .collect();
        ranks[d] = rank(
            cells[d]
                .iter()
                .map(|c| simplex_boundary(c).iter().map(|b| rows[b]).collect())
                .collect(),
        )?;
    }
    Ok((betti(sizes, ranks)?, sizes))
}
fn betti(n: [usize; 4], r: [usize; 4]) -> Result<[usize; 3], ObserveError> {
    let mut b = [0; 3];
    for d in 0..3 {
        b[d] = n[d]
            .checked_sub(r[d] + r[d + 1])
            .ok_or(ObserveError::Numerical("chain rank inconsistency"))?;
    }
    Ok(b)
}
fn tetrahedral(g: &Volume, phase: Cell) -> Result<([usize; 3], [usize; 4]), ObserveError> {
    let mut cells: [BTreeSet<Vec<usize>>; 4] = std::array::from_fn(|_| BTreeSet::new());
    let vertex =
        |p: [usize; 3]| (p[2] * (g.dimensions[1] + 1) + p[1]) * (g.dimensions[0] + 1) + p[0];
    // Freudenthal ordering uses the same global diagonal on every shared face.
    for (i, &c) in g.cells.iter().enumerate() {
        if c == phase {
            for order in [
                [0, 1, 2],
                [0, 2, 1],
                [1, 0, 2],
                [1, 2, 0],
                [2, 0, 1],
                [2, 1, 0],
            ] {
                let mut p = g.point(i);
                let mut tet = vec![vertex(p)];
                for a in order {
                    p[a] += 1;
                    tet.push(vertex(p));
                }
                tet.sort_unstable();
                cells[3].insert(tet);
            }
        }
    }
    for d in (1..=3).rev() {
        for c in cells[d].clone() {
            cells[d - 1].extend(simplex_boundary(&c));
        }
    }
    simplicial(&cells)
}
/// Exact topology of the represented finite complexes only. Nonzero input
/// approximation cannot exclude unsampled subcell topology in the nominal CAD.
pub fn volume(g: &Volume, phase: Cell) -> Result<VolumeTopology, ObserveError> {
    g.validate()?;
    if !matches!(phase, Cell::Fluid | Cell::Solid) {
        return Err(ObserveError::Invalid("topology phase"));
    }
    if !g.unresolved_cells().is_empty() {
        return Err(ObserveError::Unsupported("unresolved topology cells"));
    }
    if g.cells.iter().filter(|&&c| c == phase).count() > 512 {
        return Err(ObserveError::Unsupported("topology occupied-cell budget"));
    }
    let (a, na) = cubical(g, phase)?;
    let (b, nb) = tetrahedral(g, phase)?;
    if a != b {
        return Err(ObserveError::Numerical("volume complex disagreement"));
    }
    Ok(VolumeTopology {
        cubical: VolumeBetti(a),
        tetrahedral: VolumeBetti(b),
        cubical_cells: na,
        tetrahedral_cells: nb,
    })
}
pub fn boundary(m: &Mesh) -> Result<BoundaryBetti, ObserveError> {
    mesh::validate(m)?;
    if m.triangles.len() > 6144 {
        return Err(ObserveError::Unsupported("boundary topology budget"));
    }
    let mut cells: [BTreeSet<Vec<usize>>; 4] = std::array::from_fn(|_| BTreeSet::new());
    for t in &m.triangles {
        let mut t = t.to_vec();
        t.sort_unstable();
        cells[2].insert(t);
    }
    for d in (1..=2).rev() {
        for c in cells[d].clone() {
            cells[d - 1].extend(simplex_boundary(&c));
        }
    }
    let (b, _) = simplicial(&cells)?;
    Ok(BoundaryBetti(b))
}
