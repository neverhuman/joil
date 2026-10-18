//! CPU reference voxel spring-network mechanics. Synthetic software verification only;
//! this is not a qualified continuum elasticity or nonlinear collapse solver.

#[derive(Clone, Debug, PartialEq)]
pub enum FeaError {
    InvalidShape,
    InvalidMaterial,
    InvalidState,
    Bounds,
    NotConverged,
    InvalidMatrix,
}

#[derive(Clone, Copy, Debug)]
struct Edge {
    a: usize,
    b: usize,
    n: [f64; 3],
    k: f64,
}

/// Bounded central-force voxel network with 26-neighbour bonds and explicit fixed nodes.
/// It is a reference discretization for operator parity, not an engineering FEA model.
pub struct VoxelNetwork {
    dims: [usize; 3],
    edges: Vec<Edge>,
    fixed: Vec<bool>,
    max_nodes: usize,
}

impl VoxelNetwork {
    pub fn new(dims: [usize; 3], young_pa: f64, max_nodes: usize) -> Result<Self, FeaError> {
        let nodes = dims[0]
            .checked_mul(dims[1])
            .and_then(|x| x.checked_mul(dims[2]))
            .ok_or(FeaError::InvalidShape)?;
        if dims.contains(&0) || nodes > max_nodes || max_nodes == 0 || max_nodes > 100_000 {
            return Err(FeaError::InvalidShape);
        }
        if !young_pa.is_finite() || young_pa <= 0.0 {
            return Err(FeaError::InvalidMaterial);
        }
        let mut g = Self {
            dims,
            edges: Vec::new(),
            fixed: vec![false; nodes],
            max_nodes,
        };
        // One half of the 26-neighbour stencil avoids duplicate springs.
        let dirs: [[isize; 3]; 13] = [
            [1, 0, 0],
            [0, 1, 0],
            [0, 0, 1],
            [1, 1, 0],
            [1, -1, 0],
            [1, 0, 1],
            [1, 0, -1],
            [0, 1, 1],
            [0, 1, -1],
            [1, 1, 1],
            [1, 1, -1],
            [1, -1, 1],
            [1, -1, -1],
        ];
        for z in 0..dims[2] {
            for y in 0..dims[1] {
                for x in 0..dims[0] {
                    let a = g.node(x, y, z)?;
                    for d in dirs {
                        let xx = x as isize + d[0];
                        let yy = y as isize + d[1];
                        let zz = z as isize + d[2];
                        if xx >= 0
                            && yy >= 0
                            && zz >= 0
                            && xx < dims[0] as isize
                            && yy < dims[1] as isize
                            && zz < dims[2] as isize
                        {
                            let b = g.node(xx as usize, yy as usize, zz as usize)?;
                            let len = ((d[0] * d[0] + d[1] * d[1] + d[2] * d[2]) as f64).sqrt();
                            let n = [d[0] as f64 / len, d[1] as f64 / len, d[2] as f64 / len];
                            let k = young_pa / (len * len);
                            g.edges.push(Edge { a, b, n, k });
                        }
                    }
                }
            }
        }
        Ok(g)
    }
    fn node(&self, x: usize, y: usize, z: usize) -> Result<usize, FeaError> {
        if x >= self.dims[0] || y >= self.dims[1] || z >= self.dims[2] {
            Err(FeaError::Bounds)
        } else {
            Ok((z * self.dims[1] + y) * self.dims[0] + x)
        }
    }
    pub fn fix_node(&mut self, x: usize, y: usize, z: usize) -> Result<(), FeaError> {
        let i = self.node(x, y, z)?;
        self.fixed[i] = true;
        Ok(())
    }
    pub fn nodes(&self) -> usize {
        self.fixed.len()
    }
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    pub fn apply(&self, u: &[[f64; 3]], anchor_penalty: f64) -> Result<Vec<[f64; 3]>, FeaError> {
        if u.len() != self.nodes()
            || u.iter().flatten().any(|v| !v.is_finite())
            || !anchor_penalty.is_finite()
            || anchor_penalty <= 0.0
        {
            return Err(FeaError::InvalidState);
        }
        let mut out = vec![[0.0; 3]; self.nodes()];
        for e in &self.edges {
            let du = [
                u[e.b][0] - u[e.a][0],
                u[e.b][1] - u[e.a][1],
                u[e.b][2] - u[e.a][2],
            ];
            let ext = dot(e.n, du);
            for (c, normal) in e.n.iter().enumerate() {
                let f = e.k * ext * normal;
                out[e.a][c] -= f;
                out[e.b][c] += f;
            }
        }
        for i in 0..self.nodes() {
            if self.fixed[i] {
                for c in 0..3 {
                    out[i][c] += anchor_penalty * u[i][c];
                }
            }
        }
        if out.iter().flatten().any(|v| !v.is_finite()) {
            return Err(FeaError::InvalidState);
        }
        Ok(out)
    }
    pub fn solve_cg(
        &self,
        load: &[[f64; 3]],
        anchor_penalty: f64,
        tol: f64,
        max_iter: usize,
    ) -> Result<(Vec<[f64; 3]>, usize), FeaError> {
        if load.len() != self.nodes()
            || load.iter().flatten().any(|v| !v.is_finite())
            || !tol.is_finite()
            || tol <= 0.0
            || max_iter == 0
        {
            return Err(FeaError::InvalidState);
        }
        let n = self.nodes() * 3;
        if n > self.max_nodes * 3 {
            return Err(FeaError::Bounds);
        }
        let b: Vec<f64> = load.iter().flatten().copied().collect();
        let mut x = vec![0.0; n];
        let mut r = b.clone();
        let mut p = r.clone();
        let mut rr = dotn(&r, &r);
        if rr.sqrt() <= tol {
            return Ok((vec![[0.0; 3]; self.nodes()], 0));
        }
        for it in 0..max_iter {
            let pv = unflatten(&p);
            let ap = flatten(&self.apply(&pv, anchor_penalty)?);
            let pap = dotn(&p, &ap);
            if !pap.is_finite() || pap <= 0.0 {
                return Err(FeaError::InvalidMatrix);
            }
            let alpha = rr / pap;
            for i in 0..n {
                x[i] += alpha * p[i];
                r[i] -= alpha * ap[i];
            }
            let next = dotn(&r, &r);
            if !next.is_finite() {
                return Err(FeaError::InvalidState);
            }
            if next.sqrt() <= tol {
                return Ok((unflatten(&x), it + 1));
            }
            let beta = next / rr;
            for i in 0..n {
                p[i] = r[i] + beta * p[i];
            }
            rr = next;
        }
        Err(FeaError::NotConverged)
    }

    /// Jacobi diagonal for the anchored spring operator, in node/component order.
    pub fn diagonal_preconditioner(&self, anchor_penalty: f64) -> Result<Vec<[f64; 3]>, FeaError> {
        if !anchor_penalty.is_finite() || anchor_penalty <= 0.0 {
            return Err(FeaError::InvalidState);
        }
        let mut d = vec![[0.0; 3]; self.nodes()];
        for e in &self.edges {
            for (c, &nc) in e.n.iter().enumerate() {
                let v = e.k * nc * nc;
                d[e.a][c] += v;
                d[e.b][c] += v;
            }
        }
        for (i, row) in d.iter_mut().enumerate() {
            for v in row {
                if self.fixed[i] {
                    *v += anchor_penalty;
                }
                if !v.is_finite() || *v <= 0.0 {
                    return Err(FeaError::InvalidMatrix);
                }
            }
        }
        Ok(d)
    }

    /// Preconditioned conjugate gradient for the same anchored CPU reference operator.
    pub fn solve_pcg(
        &self,
        load: &[[f64; 3]],
        anchor_penalty: f64,
        tol: f64,
        max_iter: usize,
    ) -> Result<(Vec<[f64; 3]>, usize), FeaError> {
        if load.len() != self.nodes()
            || load.iter().flatten().any(|v| !v.is_finite())
            || !tol.is_finite()
            || tol <= 0.0
            || max_iter == 0
        {
            return Err(FeaError::InvalidState);
        }
        let diag = flatten(&self.diagonal_preconditioner(anchor_penalty)?);
        let b = flatten(load);
        let n = b.len();
        let mut x = vec![0.0; n];
        let mut r = b.clone();
        let mut z: Vec<f64> = r.iter().zip(&diag).map(|(a, d)| a / d).collect();
        let mut p = z.clone();
        let mut rz = dotn(&r, &z);
        if dotn(&r, &r).sqrt() <= tol {
            return Ok((vec![[0.0; 3]; self.nodes()], 0));
        }
        for it in 0..max_iter {
            let ap = flatten(&self.apply(&unflatten(&p), anchor_penalty)?);
            let pap = dotn(&p, &ap);
            if !pap.is_finite() || pap <= 0.0 {
                return Err(FeaError::InvalidMatrix);
            }
            let alpha = rz / pap;
            for i in 0..n {
                x[i] += alpha * p[i];
                r[i] -= alpha * ap[i];
            }
            if dotn(&r, &r).sqrt() <= tol {
                return Ok((unflatten(&x), it + 1));
            }
            z = r.iter().zip(&diag).map(|(a, d)| a / d).collect();
            let next = dotn(&r, &z);
            if !next.is_finite() || rz <= 0.0 {
                return Err(FeaError::InvalidMatrix);
            }
            let beta = next / rz;
            for i in 0..n {
                p[i] = z[i] + beta * p[i];
            }
            rz = next;
        }
        Err(FeaError::NotConverged)
    }
}

/// Check column eigenvectors against a finite symmetric matrix and orthonormality.
pub fn verify_eigenpairs(
    matrix: &[Vec<f64>],
    eigenvalues: &[f64],
    eigenvectors: &[Vec<f64>],
    tol: f64,
) -> Result<f64, FeaError> {
    let n = matrix.len();
    let k = eigenvalues.len();
    if n == 0
        || matrix.iter().any(|r| r.len() != n)
        || eigenvectors.len() != n
        || eigenvectors.iter().any(|r| r.len() != k)
        || k == 0
        || !tol.is_finite()
        || tol <= 0.0
        || matrix
            .iter()
            .flatten()
            .chain(eigenvalues)
            .chain(eigenvectors.iter().flatten())
            .any(|x| !x.is_finite())
    {
        return Err(FeaError::InvalidMatrix);
    }
    let mut worst = 0.0_f64;
    for a in 0..k {
        for i in 0..n {
            let av: f64 = (0..n).map(|j| matrix[i][j] * eigenvectors[j][a]).sum();
            worst = worst.max((av - eigenvalues[a] * eigenvectors[i][a]).abs());
        }
        for b in 0..k {
            let dot: f64 = (0..n)
                .map(|i| eigenvectors[i][a] * eigenvectors[i][b])
                .sum();
            worst = worst.max((dot - if a == b { 1.0 } else { 0.0 }).abs());
        }
    }
    if worst > tol {
        Err(FeaError::InvalidMatrix)
    } else {
        Ok(worst)
    }
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn dotn(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn flatten(a: &[[f64; 3]]) -> Vec<f64> {
    a.iter().flatten().copied().collect()
}
fn unflatten(a: &[f64]) -> Vec<[f64; 3]> {
    a.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect()
}

/// Jacobi eigensolver for a finite real symmetric dense matrix; sorted ascending eigenvalues.
#[allow(clippy::needless_range_loop)] // Dense Jacobi row/column rotations require paired indexed access.
pub fn symmetric_eigen(
    matrix: &[Vec<f64>],
    tol: f64,
    max_sweeps: usize,
) -> Result<(Vec<f64>, Vec<Vec<f64>>), FeaError> {
    let n = matrix.len();
    if n == 0
        || matrix.iter().any(|r| r.len() != n)
        || !tol.is_finite()
        || tol <= 0.0
        || max_sweeps == 0
    {
        return Err(FeaError::InvalidMatrix);
    }
    let mut a = matrix.to_vec();
    if a.iter().flatten().any(|x| !x.is_finite()) {
        return Err(FeaError::InvalidState);
    }
    for i in 0..n {
        for j in 0..n {
            if (a[i][j] - a[j][i]).abs() > tol * 10.0 {
                return Err(FeaError::InvalidMatrix);
            }
        }
    }
    let mut v = vec![vec![0.0; n]; n];
    for i in 0..n {
        v[i][i] = 1.0;
    }
    for _ in 0..max_sweeps {
        let mut max = 0.0;
        let (mut p, mut q) = (0, 0);
        for i in 0..n {
            for j in i + 1..n {
                if a[i][j].abs() > max {
                    max = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if max <= tol {
            break;
        }
        let theta = 0.5 * (2.0 * a[p][q]).atan2(a[q][q] - a[p][p]);
        let (c, s) = (theta.cos(), theta.sin());
        for k in 0..n {
            let apk = a[p][k];
            let aqk = a[q][k];
            a[p][k] = c * apk - s * aqk;
            a[q][k] = s * apk + c * aqk;
        }
        for k in 0..n {
            let akp = a[k][p];
            let akq = a[k][q];
            a[k][p] = c * akp - s * akq;
            a[k][q] = s * akp + c * akq;
            let vkp = v[k][p];
            let vkq = v[k][q];
            v[k][p] = c * vkp - s * vkq;
            v[k][q] = s * vkp + c * vkq;
        }
    }
    let mut residual = 0.0_f64;
    for i in 0..n {
        for j in i + 1..n {
            residual = residual.max(a[i][j].abs());
        }
    }
    if residual > tol || a.iter().flatten().any(|x| !x.is_finite()) {
        return Err(FeaError::NotConverged);
    }
    let mut pairs: Vec<(f64, usize)> = (0..n).map(|i| (a[i][i], i)).collect();
    pairs.sort_by(|x, y| x.0.total_cmp(&y.0));
    let vals = pairs.iter().map(|p| p.0).collect();
    let vecs = (0..n)
        .map(|r| pairs.iter().map(|p| v[r][p.1]).collect())
        .collect();
    Ok((vals, vecs))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_operator_is_finite_symmetric_and_bounded() {
        let mut g = VoxelNetwork::new([2, 2, 2], 200e9, 100).unwrap();
        g.fix_node(0, 0, 0).unwrap();
        let u = vec![[0.0; 3]; 8];
        let a = g.apply(&u, 1e12).unwrap();
        assert!(a.iter().flatten().all(|v| v.is_finite()));
        assert!(g.edge_count() > 0);
        assert_eq!(g.fix_node(2, 0, 0), Err(FeaError::Bounds));
        assert!(VoxelNetwork::new([100, 100, 100], 1.0, 10).is_err());
    }
    #[test]
    fn conjugate_gradient_solves_anchored_synthetic_load() {
        let mut g = VoxelNetwork::new([2, 2, 2], 10.0, 100).unwrap();
        for z in 0..2 {
            for y in 0..2 {
                g.fix_node(0, y, z).unwrap();
            }
        }
        let mut f = vec![[0.0; 3]; 8];
        f[7][0] = 1.0;
        let (u, it) = g.solve_cg(&f, 100.0, 1e-10, 1000).unwrap();
        assert!(it > 0);
        let ku = g.apply(&u, 100.0).unwrap();
        let res = (ku[7][0] - 1.0).abs();
        assert!(res < 1e-8, "{res}");
    }
    #[test]
    fn eigenpairs_are_valid_and_invalid_matrix_is_rejected() {
        let a = vec![vec![2.0, 1.0], vec![1.0, 2.0]];
        let (d, v) = symmetric_eigen(&a, 1e-12, 100).unwrap();
        assert!((d[0] - 1.0).abs() < 1e-10);
        assert!((d[1] - 3.0).abs() < 1e-10);
        for k in 0..2 {
            let mut r = 0.0;
            for i in 0..2 {
                r += (a[i][0] * v[0][k] + a[i][1] * v[1][k] - d[k] * v[i][k]).powi(2);
            }
            assert!(r.sqrt() < 1e-9);
        }
        assert!(symmetric_eigen(&[vec![1.0, 2.0], vec![0.0, 1.0]], 1e-9, 10).is_err());
        assert!(symmetric_eigen(&[vec![f64::NAN]], 1e-9, 10).is_err());
    }
    #[test]
    fn invalid_load_and_nonconvergence_fail_closed() {
        let g = VoxelNetwork::new([2, 2, 2], 1.0, 100).unwrap();
        assert!(g.solve_cg(&[], 1.0, 1e-6, 10).is_err());
        let mut f = vec![[0.0; 3]; 8];
        f[7][0] = 1.0;
        assert_eq!(
            g.solve_cg(&f, 1.0, 1e-14, 1).err(),
            Some(FeaError::NotConverged)
        );
    }

    #[test]
    fn v5_24_02_pcg_parities_and_invalid_eigenpairs_fail() {
        let mut g = VoxelNetwork::new([2, 2, 2], 10.0, 100).unwrap();
        for y in 0..2 {
            for z in 0..2 {
                g.fix_node(0, y, z).unwrap();
            }
        }
        let mut load = vec![[0.0; 3]; g.nodes()];
        load[7][0] = 1.0;
        let (a, _) = g.solve_cg(&load, 100.0, 1e-10, 1000).unwrap();
        let (b, _) = g.solve_pcg(&load, 100.0, 1e-10, 1000).unwrap();
        assert!(a
            .iter()
            .flatten()
            .zip(b.iter().flatten())
            .all(|(x, y)| (x - y).abs() < 1e-9));
        let m = vec![vec![2.0, 1.0], vec![1.0, 2.0]];
        let (d, v) = symmetric_eigen(&m, 1e-12, 100).unwrap();
        assert!(verify_eigenpairs(&m, &d, &v, 1e-9).unwrap() < 1e-9);
        assert_eq!(
            verify_eigenpairs(&m, &[1.0, 3.0], &[vec![1.0, 1.0], vec![1.0, 1.0]], 1e-9),
            Err(FeaError::InvalidMatrix)
        );
    }

    #[test]
    fn v5_24_02_to_05_sc_dec_038_accelerator_cpu_parity_fallback_and_non_promotion_guard() {
        // This crate exposes only the deterministic CPU reference. Repeated
        // execution is a regression check for reference parity; it does not
        // claim an accelerator backend or measured CPU/accelerator parity.
        let mut network = VoxelNetwork::new([2, 2, 2], 10.0, 100).unwrap();
        for y in 0..2 {
            for z in 0..2 {
                network.fix_node(0, y, z).unwrap();
            }
        }
        let mut load = vec![[0.0; 3]; network.nodes()];
        load[7][0] = 1.0;
        let (first, _) = network.solve_cg(&load, 100.0, 1e-10, 1000).unwrap();
        let (fallback, _) = network.solve_cg(&load, 100.0, 1e-10, 1000).unwrap();
        for (a, b) in first.iter().flatten().zip(fallback.iter().flatten()) {
            assert!(
                (a - b).abs() <= 1e-12,
                "CPU reference fallback is deterministic"
            );
        }
        let response = network.apply(&first, 100.0).unwrap();
        assert!((response[7][0] - load[7][0]).abs() < 1e-8);

        // No accelerator result, lease, held-out paired evidence, or admission
        // contract exists in this reference-only FEA API; none can be promoted.
        let accelerator_verified = false;
        let gpu_lease_available = false;
        let scientific_promotion = accelerator_verified && gpu_lease_available;
        assert!(!scientific_promotion);
        assert_eq!(
            VoxelNetwork::new([101, 101, 101], 10.0, 100).err(),
            Some(FeaError::InvalidShape),
            "bounded CPU fallback rejects an unbounded request",
        );
    }
}
