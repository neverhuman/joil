//! Explicit seeded P1/P2 routes. No Delaunay/isosurface-stuffing quality theorem
//! is claimed. P2 determinants are cubic Bernstein polynomials; coefficient
//! extrema enclose the determinant everywhere, unlike quadrature-point tests.
use crate::surface::{
    add, det, finite_nonnegative, identity, invalid, mul, norm, numerical, sub, unsupported, Point,
    Result, TaggedSurface,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const EDGES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TetPolicy {
    pub minimum_jacobian_m3: f64,
    pub minimum_scaled_jacobian: f64,
    pub maximum_projection_m: f64,
    pub volume_tolerance_m3: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct TetAudit {
    pub minimum_jacobian_bound_m3: f64,
    pub maximum_jacobian_bound_m3: f64,
    pub minimum_scaled_jacobian: f64,
    pub volume_m3: f64,
    pub projection_bound_m: f64,
    pub arithmetic: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct MeshProvenance {
    pub backend: String,
    pub partition_sha256: String,
    pub upstream_acceptance_receipt_sha256: String,
    pub compiler: String,
    pub kernel: String,
    pub input_tolerance_m: f64,
    pub upstream_deviation_bound_m: f64,
    pub admissible_consumers: Vec<String>,
    pub active_features: BTreeSet<crate::surface::Feature>,
}
#[derive(Clone, Debug, Serialize)]
pub struct P1Mesh {
    provenance: MeshProvenance,
    surface_sha256: String,
    vertices_m: Vec<Point>,
    elements: Vec<[usize; 4]>,
    policy: TetPolicy,
    audit: TetAudit,
}
impl P1Mesh {
    pub fn vertices_m(&self) -> &[Point] {
        &self.vertices_m
    }
    pub fn elements(&self) -> &[[usize; 4]] {
        &self.elements
    }
    pub fn audit(&self) -> &TetAudit {
        &self.audit
    }
    pub fn surface_sha256(&self) -> &str {
        &self.surface_sha256
    }
    pub fn sha256(&self) -> Result<String> {
        identity(self)
    }
}
fn policy_check(p: TetPolicy) -> Result<()> {
    if !finite_nonnegative(p.minimum_jacobian_m3)
        || p.minimum_jacobian_m3 == 0.0
        || !finite_nonnegative(p.minimum_scaled_jacobian)
        || p.minimum_scaled_jacobian == 0.0
        || !finite_nonnegative(p.maximum_projection_m)
        || !finite_nonnegative(p.volume_tolerance_m3)
    {
        return Err(invalid("tet quality/projection policy"));
    }
    Ok(())
}
fn faces(t: [usize; 4]) -> [[usize; 3]; 4] {
    let [a, b, c, d] = t;
    [[b, c, d], [a, d, c], [a, b, d], [a, c, b]]
}
fn parity(f: [usize; 3]) -> i32 {
    if ((f[0] > f[1]) as u8 + (f[0] > f[2]) as u8 + (f[1] > f[2]) as u8).is_multiple_of(2) {
        1
    } else {
        -1
    }
}
/// Seed nodes begin with the exact surface vertex array. Boundary facets must
/// match it bijectively with orientation; hidden holes/extraneous facets fail.
pub fn qualify_p1(
    surface: &TaggedSurface,
    vertices_m: Vec<Point>,
    elements: Vec<[usize; 4]>,
    policy: TetPolicy,
) -> Result<P1Mesh> {
    policy_check(policy)?;
    if elements.is_empty() || elements.len() > 1024 || vertices_m.len() > 4096 {
        return Err(unsupported("bounded seeded tet route capacity"));
    }
    if vertices_m.iter().flatten().any(|x| !x.is_finite())
        || !vertices_m.starts_with(&surface.boundary().vertices_m)
    {
        return Err(invalid("tet boundary identity/finite metre nodes"));
    }
    let mut face_map: BTreeMap<[usize; 3], Vec<i32>> = BTreeMap::new();
    let mut used = BTreeSet::new();
    let mut unique = BTreeSet::new();
    let mut min_j = f64::INFINITY;
    let mut max_j: f64 = 0.0;
    let mut min_q = f64::INFINITY;
    let mut volume = 0.0;
    for &t in &elements {
        if t.iter().any(|i| *i >= vertices_m.len()) {
            return Err(invalid("tet index"));
        }
        let mut key = t;
        key.sort();
        if key.windows(2).any(|w| w[0] == w[1]) || !unique.insert(key) {
            return Err(invalid("repeated/duplicate tetrahedron"));
        }
        let p = t.map(|i| vertices_m[i]);
        let j = det(sub(p[1], p[0]), sub(p[2], p[0]), sub(p[3], p[0]));
        let max_edge = EDGES
            .iter()
            .map(|(a, b)| norm(sub(p[*a], p[*b])))
            .fold(0.0, f64::max);
        let q = j / max_edge.powi(3);
        if !j.is_finite()
            || j < policy.minimum_jacobian_m3
            || !q.is_finite()
            || q < policy.minimum_scaled_jacobian
        {
            return Err(numerical("P1 inverted/poor-quality element"));
        }
        min_j = min_j.min(j);
        max_j = max_j.max(j);
        min_q = min_q.min(q);
        volume += j / 6.0;
        used.extend(t);
        for f in faces(t) {
            let mut key = f;
            key.sort();
            face_map.entry(key).or_default().push(parity(f));
        }
    }
    if used.len() != vertices_m.len() {
        return Err(invalid("unused tet node"));
    }
    let mut expected = BTreeMap::new();
    for f in &surface.boundary().facets {
        let mut k = f.vertices;
        k.sort();
        expected.insert(k, parity(f.vertices));
    }
    for (key, orientation) in face_map {
        match orientation.as_slice() {
            [x] if expected.remove(&key) == Some(*x) => (),
            [a, b] if a + b == 0 => (),
            _ => return Err(numerical("tet boundary/face incidence mismatch")),
        }
    }
    if !expected.is_empty() {
        return Err(numerical("missing tet boundary facets"));
    }
    for i in 0..elements.len() {
        for j in i + 1..elements.len() {
            if positive_overlap(
                elements[i].map(|v| vertices_m[v]),
                elements[j].map(|v| vertices_m[v]),
            )? {
                return Err(numerical("overlapping tetrahedron interiors"));
            }
        }
    }
    if !volume.is_finite()
        || (volume - surface.audit().volume_m3).abs() > policy.volume_tolerance_m3
    {
        return Err(numerical("tet/surface volume mismatch"));
    }
    Ok(P1Mesh {
        provenance: MeshProvenance {
            backend: "seeded-p1/1".into(),
            partition_sha256: surface.boundary().partition_sha256.clone(),
            upstream_acceptance_receipt_sha256: surface
                .boundary()
                .acceptance_receipt_sha256
                .clone(),
            compiler: surface.boundary().compiler.clone(),
            kernel: surface.boundary().kernel.clone(),
            input_tolerance_m: surface.policy().length_tolerance_m,
            upstream_deviation_bound_m: surface.audit().deviation_bound_m,
            admissible_consumers: vec![
                "p1-structural-software-fixture; native solver admission still required".into(),
            ],
            active_features: surface.boundary().active_features.clone(),
        },
        surface_sha256: surface.boundary_sha256().into(),
        vertices_m,
        elements,
        policy,
        audit: TetAudit {
            minimum_jacobian_bound_m3: min_j,
            maximum_jacobian_bound_m3: max_j,
            minimum_scaled_jacobian: min_q,
            volume_m3: volume,
            projection_bound_m: 0.0,
            arithmetic: "f64 affine determinants; bounded seeded complex".into(),
        },
    })
}
/// Generate one tetrahedron per oriented boundary facet around an explicit
/// interior seed. Only star-shaped inputs whose generated complex passes all
/// checks are served. No repair, hidden fallback, or theoretical quality claim.
pub fn mesh_from_interior_seed(
    surface: &TaggedSurface,
    seed_m: Point,
    policy: TetPolicy,
) -> Result<P1Mesh> {
    if seed_m.iter().any(|x| !x.is_finite()) {
        return Err(invalid("finite interior seed in metres"));
    }
    let mut nodes = surface.boundary().vertices_m.clone();
    let center = nodes.len();
    nodes.push(seed_m);
    let elements = surface
        .boundary()
        .facets
        .iter()
        .map(|f| {
            let [a, b, c] = f.vertices;
            [center, a, b, c]
        })
        .collect();
    let mut mesh = qualify_p1(surface, nodes, elements, policy)?;
    mesh.provenance.backend = "star-shaped-interior-seed-p1/1".into();
    Ok(mesh)
}

// Exact expansion fallback for near-zero separating predicates. The design is
// based on error-free sums/products and expansion arithmetic described by
// Shewchuk, https://www.cs.cmu.edu/~quake/robust.html . This is an independently
// implemented bounded fallback, not adoption of Triangle or its mesh guarantees.
// Overflow, subnormal arithmetic and unresolved exponent extremes abstain.
fn checked_float(x: f64) -> Result<f64> {
    if !x.is_finite() || x.is_subnormal() {
        Err(unsupported("predicate arithmetic exponent range"))
    } else {
        Ok(x)
    }
}
fn two_sum(a: f64, b: f64) -> Result<(f64, f64)> {
    let sum = checked_float(a + b)?;
    let bv = checked_float(sum - a)?;
    let error = checked_float((a - (sum - bv)) + (b - bv))?;
    Ok((sum, error))
}
fn grow(expansion: &mut Vec<f64>, value: f64) -> Result<()> {
    let mut q = value;
    let mut out = Vec::new();
    for &x in expansion.iter() {
        let (sum, error) = two_sum(q, x)?;
        if error != 0.0 {
            out.push(error);
        }
        q = sum;
    }
    if q != 0.0 {
        out.push(q);
    }
    *expansion = out;
    Ok(())
}
fn difference(a: f64, b: f64) -> Result<Vec<f64>> {
    let (sum, error) = two_sum(a, -b)?;
    Ok([error, sum].into_iter().filter(|x| *x != 0.0).collect())
}
fn product(a: &[f64], b: &[f64]) -> Result<Vec<f64>> {
    let mut out = Vec::new();
    for &x in a {
        for &y in b {
            let p = checked_float(x * y)?;
            if p == 0.0 && x != 0.0 && y != 0.0 {
                return Err(unsupported("predicate product underflow"));
            }
            let residual = checked_float(x.mul_add(y, -p))?;
            // Below this exponent, an FMA residual may round away nonzero bits.
            if p != 0.0 && p.abs() < f64::MIN_POSITIVE / (f64::EPSILON * f64::EPSILON) {
                return Err(unsupported("predicate product exponent guard"));
            }
            if residual != 0.0 {
                grow(&mut out, residual)?;
            }
            if p != 0.0 {
                grow(&mut out, p)?;
            }
        }
    }
    Ok(out)
}
/// Sign of det(b-a,d-c,q-p), including subtraction residuals. The generous
/// filter resolves ordinary values; exact expansions resolve shared contacts.
fn triple_sign(a: Point, b: Point, c: Point, d: Point, p: Point, q: Point) -> Result<i8> {
    if p == q || a == b || c == d {
        return Ok(0);
    }
    let x = sub(b, a);
    let y = sub(d, c);
    let z = sub(q, p);
    let terms = [
        x[0] * y[1] * z[2],
        x[1] * y[2] * z[0],
        x[2] * y[0] * z[1],
        -x[0] * y[2] * z[1],
        -x[1] * y[0] * z[2],
        -x[2] * y[1] * z[0],
    ];
    let approx = terms.iter().sum::<f64>();
    let permanent = terms.iter().map(|x| x.abs()).sum::<f64>();
    if approx.is_finite()
        && permanent.is_finite()
        && permanent >= f64::MIN_POSITIVE / f64::EPSILON
        && approx.abs() > 128.0 * f64::EPSILON * permanent
    {
        return Ok(if approx > 0.0 { 1 } else { -1 });
    }
    let xe = (0..3)
        .map(|i| difference(b[i], a[i]))
        .collect::<Result<Vec<_>>>()?;
    let ye = (0..3)
        .map(|i| difference(d[i], c[i]))
        .collect::<Result<Vec<_>>>()?;
    let ze = (0..3)
        .map(|i| difference(q[i], p[i]))
        .collect::<Result<Vec<_>>>()?;
    let mut expansion = Vec::new();
    for (i, j, k, sign) in [
        (0, 1, 2, 1.0),
        (1, 2, 0, 1.0),
        (2, 0, 1, 1.0),
        (0, 2, 1, -1.0),
        (1, 0, 2, -1.0),
        (2, 1, 0, -1.0),
    ] {
        for term in product(&product(&xe[i], &ye[j])?, &ze[k])? {
            grow(&mut expansion, term * sign)?;
        }
    }
    Ok(match expansion.last() {
        Some(x) if *x > 0.0 => 1,
        Some(_) => -1,
        None => 0,
    })
}
fn positive_overlap(a: [Point; 4], b: [Point; 4]) -> Result<bool> {
    let mut axes = Vec::new();
    for p in [a, b] {
        for f in faces([0, 1, 2, 3]) {
            axes.push((p[f[0]], p[f[1]], p[f[0]], p[f[2]]));
        }
    }
    for (i, j) in EDGES {
        for (k, l) in EDGES {
            axes.push((a[i], a[j], b[k], b[l]));
        }
    }
    for (u, v, w, x) in axes {
        let mut positive = false;
        let mut negative = false;
        'pairs: for p in a {
            for q in b {
                let sign = triple_sign(u, v, w, x, p, q)?;
                positive |= sign > 0;
                negative |= sign < 0;
                if positive && negative {
                    break 'pairs;
                }
            }
        }
        if positive != negative {
            return Ok(false);
        }
        // All zero means parallel edges (zero axis); full-dimensional tets
        // cannot both lie in a nonzero axis's single projection plane.
    }
    Ok(true)
}
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
    Ok(P2Mesh {provenance, source_p1_sha256:mesh.sha256()?,surface_sha256:mesh.surface_sha256.clone(),nodes_m:nodes,elements,
        audit:TetAudit {minimum_jacobian_bound_m3:min,maximum_jacobian_bound_m3:max,minimum_scaled_jacobian:qmin,
            volume_m3:volume,projection_bound_m:projection_bound,arithmetic:"f64 cubic Bernstein coefficient enclosure, no inherited mesher theorem; no directed-rounding certificate".into()}})
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
