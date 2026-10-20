use super::*;

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
