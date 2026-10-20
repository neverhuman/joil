//! Quadratic P2 tetrahedral elevation, cubic Bernstein Jacobian enclosures, and predicate tests.

use super::*;

#[derive(Clone, Debug, Serialize)]
pub struct P2Mesh {
    provenance: MeshProvenance,
    source_p1_sha256: String,
    surface_sha256: String,
    nodes_m: Vec<Point>,
    elements: Vec<[usize; 10]>,
    audit: TetAudit,
}
impl P2Mesh {
    pub fn nodes_m(&self) -> &[Point] {
        &self.nodes_m
    }
    pub fn elements(&self) -> &[[usize; 10]] {
        &self.elements
    }
    pub fn audit(&self) -> &TetAudit {
        &self.audit
    }
    pub fn sha256(&self) -> Result<String> {
        identity(self)
    }
}
/// One shared midpoint per edge. Projection can move boundary edge nodes only;
/// supplied coordinates are checked against a uniform displacement budget.
/// Curved boundary fidelity still requires upstream partition evidence.
pub fn elevate_p2(mesh: &P1Mesh, projected: &BTreeMap<[usize; 2], Point>) -> Result<P2Mesh> {
    let mut nodes = mesh.vertices_m.clone();
    let mut edge_nodes = BTreeMap::new();
    let mut boundary_counts = BTreeMap::new();
    for t in &mesh.elements {
        for f in faces(*t) {
            let mut k = f;
            k.sort();
            *boundary_counts.entry(k).or_insert(0usize) += 1;
        }
    }
    let boundary_edges: BTreeSet<_> = boundary_counts
        .into_iter()
        .filter(|(_, n)| *n == 1)
        .flat_map(|(f, _)| [(f[0], f[1]), (f[0], f[2]), (f[1], f[2])])
        .map(|(a, b)| [a.min(b), a.max(b)])
        .collect();
    if projected.keys().any(|k| !boundary_edges.contains(k))
        || projected.values().flatten().any(|x| !x.is_finite())
    {
        return Err(invalid(
            "projection must name existing boundary edges with finite metre nodes",
        ));
    }
    let mut elements = Vec::new();
    let mut max_move: f64 = 0.0;
    for &t in &mesh.elements {
        let mut e = [0; 10];
        e[..4].copy_from_slice(&t);
        for (slot, (a, b)) in EDGES.into_iter().enumerate() {
            let key = [t[a].min(t[b]), t[a].max(t[b])];
            let index = *edge_nodes.entry(key).or_insert_with(|| {
                let midpoint = mul(add(mesh.vertices_m[key[0]], mesh.vertices_m[key[1]]), 0.5);
                let p = *projected.get(&key).unwrap_or(&midpoint);
                max_move = max_move.max(norm(sub(p, midpoint)));
                let index = nodes.len();
                nodes.push(p);
                index
            });
            e[4 + slot] = index;
        }
        elements.push(e);
    }
    // Sum of quadratic edge basis functions <= 3/2 on a tetrahedron.
    let projection_bound = 1.5 * max_move;
    if projection_bound > mesh.policy.maximum_projection_m {
        return Err(numerical("P2 projection exceeds geometric budget"));
    }
    let mut min = f64::INFINITY;
    let mut max: f64 = 0.0;
    let mut qmin = f64::INFINITY;
    let mut volume = 0.0;
    for e in &elements {
        let p = e.map(|i| nodes[i]);
        let bounds = jacobian_bernstein(p)?;
        let jmin = bounds.iter().copied().fold(f64::INFINITY, f64::min);
        let jmax = bounds.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let longest = EDGES
            .iter()
            .map(|(a, b)| norm(sub(p[*a], p[*b])))
            .fold(0.0, f64::max);
        // Each derivative column is an affine convex combination of its four
        // vertex values. A bound on its norm yields a conservative scaled metric.
        let jverts = (0..4).map(|i| jacobian_at_vertex(p, i)).collect::<Vec<_>>();
        let scale = jverts
            .iter()
            .flatten()
            .map(|col| norm(*col))
            .fold(longest, f64::max);
        let quality = jmin / scale.powi(3);
        if jmin < mesh.policy.minimum_jacobian_m3 || quality < mesh.policy.minimum_scaled_jacobian {
            return Err(numerical(
                "P2 nonpositive or unresolved Jacobian Bernstein enclosure after projection",
            ));
        }
        min = min.min(jmin);
        max = max.max(jmax);
        qmin = qmin.min(quality);
        volume += bounds.iter().sum::<f64>() / (20.0 * 6.0);
    }
    if !volume.is_finite()
        || (volume - mesh.audit.volume_m3).abs() > mesh.policy.volume_tolerance_m3
    {
        return Err(numerical("P2 projected volume budget exceeded"));
    }
    let mut provenance = mesh.provenance.clone();
    provenance.backend = "seeded-p2-bernstein/1".into();
    provenance.admissible_consumers =
        vec!["p2-structural-software-fixture; projected boundary admission still required".into()];
    Ok(P2Mesh {
        provenance,
        source_p1_sha256: mesh.sha256()?,
        surface_sha256: mesh.surface_sha256.clone(),
        nodes_m: nodes,
        elements,
        audit: TetAudit {
            minimum_jacobian_bound_m3: min,
            maximum_jacobian_bound_m3: max,
            minimum_scaled_jacobian: qmin,
            volume_m3: volume,
            projection_bound_m: projection_bound,
            arithmetic: "f64 cubic Bernstein coefficient enclosure, no inherited mesher theorem; no directed-rounding certificate".into(),
        },
    })
}
fn jacobian_at_vertex(p: [Point; 10], vertex: usize) -> [Point; 3] {
    let gradients = [
        [-1.0, -1.0, -1.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];
    let mut j = [[0.0; 3]; 3];
    // Translate first to avoid catastrophic cancellation from global origin.
    let p = p.map(|x| sub(x, p[0]));
    for i in 0..4 {
        for (axis, col) in j.iter_mut().enumerate() {
            *col = add(
                *col,
                mul(
                    p[i],
                    (if i == vertex { 3.0 } else { -1.0 }) * gradients[i][axis],
                ),
            );
        }
    }
    for (e, (a, b)) in EDGES.into_iter().enumerate() {
        for (axis, col) in j.iter_mut().enumerate() {
            let d = 4.0
                * ((if a == vertex { 1.0 } else { 0.0 }) * gradients[b][axis]
                    + (if b == vertex { 1.0 } else { 0.0 }) * gradients[a][axis]);
            *col = add(*col, mul(p[4 + e], d));
        }
    }
    j
}
/// Twenty degree-three tetrahedral Bernstein coefficients. Expanding
/// det(sum Li Ji) by columns and averaging identical barycentric monomials
/// gives these coefficients; their convex hull encloses all determinants.
pub fn jacobian_bernstein(p: [Point; 10]) -> Result<Vec<f64>> {
    if p.iter().flatten().any(|x| !x.is_finite()) {
        return Err(invalid("nonfinite P2 nodes"));
    }
    let j: [_; 4] = std::array::from_fn(|i| jacobian_at_vertex(p, i));
    let mut terms: BTreeMap<[u8; 4], (f64, usize)> = BTreeMap::new();
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                let mut powers = [0; 4];
                powers[a] += 1;
                powers[b] += 1;
                powers[c] += 1;
                let v = det(j[a][0], j[b][1], j[c][2]);
                if !v.is_finite() {
                    return Err(numerical("nonfinite P2 Jacobian"));
                }
                let term = terms.entry(powers).or_default();
                term.0 += v;
                term.1 += 1;
            }
        }
    }
    Ok(terms.into_values().map(|(sum, n)| sum / n as f64).collect())
}

#[cfg(test)]
mod predicate_tests {
    use super::*;
    #[test]
    fn expansion_preserves_a_determinant_lost_by_cancellation() {
        // Hand-derived determinant is e^2, exactly positive in binary rational
        // arithmetic. The six ordinary f64 triple products cancel to zero.
        let e = 2.0_f64.powi(-40);
        let zero = [0.0; 3];
        let a = [1.0; 3];
        let b = [1.0, 1.0 + e, 1.0];
        let c = [1.0, 1.0, 1.0 + e];
        assert_eq!(triple_sign(zero, a, zero, b, zero, c).unwrap(), 1);
        assert_eq!(triple_sign(zero, b, zero, a, zero, c).unwrap(), -1);
        assert_eq!(triple_sign(zero, a, zero, b, zero, a).unwrap(), 0);
    }
    #[test]
    fn overlap_distinguishes_contacts_from_positive_slivers() {
        let a = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let touching = [[0., 0., 0.], [-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        assert!(!positive_overlap(a, touching).unwrap());
        assert!(positive_overlap(a, a).unwrap());
        let shifted = touching.map(|p| [p[0] + 2.0_f64.powi(-40), p[1], p[2]]);
        assert!(positive_overlap(a, shifted).unwrap());
        let separated = a.map(|p| [p[0] + 2.0, p[1], p[2]]);
        assert!(!positive_overlap(a, separated).unwrap());
    }
}
