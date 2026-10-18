//! Bounded faceted-partition adapter, pending the V5-08 accepted partition ABI.
//!
//! Input is the partition's oriented boundary, never a second fluid/solid field.
//! Tags come from that authority, not coordinate planes. This backend supports
//! one connected manifold shell and at most 2048 facets. Its f64 predicates are
//! numerical checks, not exact-arithmetic or continuous implicit-field proofs.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub type Point = [f64; 3];
pub type Result<T> = std::result::Result<T, ArtifactError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactError {
    Invalid(String),
    Unsupported(String),
    NumericalFailure(String),
    Io(String),
}
impl std::fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ArtifactError {}
impl From<std::io::Error> for ArtifactError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
pub(crate) fn invalid(s: &str) -> ArtifactError {
    ArtifactError::Invalid(s.into())
}
pub(crate) fn numerical(s: &str) -> ArtifactError {
    ArtifactError::NumericalFailure(s.into())
}
pub(crate) fn unsupported(s: &str) -> ArtifactError {
    ArtifactError::Unsupported(s.into())
}
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn identity<T: Serialize>(value: &T) -> Result<String> {
    Ok(sha256(
        &serde_json::to_vec(value).map_err(|e| invalid(&e.to_string()))?,
    ))
}
pub(crate) fn hash_valid(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub(crate) fn finite_nonnegative(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}
pub(crate) fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(crate) fn add(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] + b[i])
}
pub(crate) fn mul(a: Point, s: f64) -> Point {
    a.map(|x| x * s)
}
pub(crate) fn dot(a: Point, b: Point) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
pub(crate) fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn norm(a: Point) -> f64 {
    dot(a, a).sqrt()
}
pub(crate) fn det(a: Point, b: Point, c: Point) -> f64 {
    dot(a, cross(b, c))
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Feature {
    FacetedBoundary,
    CurvedBoundary,
    ThinShell,
    ResolvedLattice,
    HomogenizedPorous,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundaryTag {
    pub patch: String,
    pub inside_material: String,
    pub outside_material: String,
    pub roughness_m: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Facet {
    pub vertices: [usize; 3],
    pub tag: BoundaryTag,
}
/// Upstream acceptance is an external dependency. A digest names the receipt;
/// this adapter does not promote a caller's receipt to scientific admission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartitionBoundary {
    pub partition_sha256: String,
    pub acceptance_receipt_sha256: String,
    pub compiler: String,
    pub kernel: String,
    pub vertices_m: Vec<Point>,
    pub facets: Vec<Facet>,
    pub active_features: BTreeSet<Feature>,
    /// Source-bound upstream enclosure, including partition approximation.
    pub volume_bounds_m3: [f64; 2],
    pub upstream_deviation_bound_m: f64,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SurfacePolicy {
    pub length_tolerance_m: f64,
    pub minimum_area_m2: f64,
    pub minimum_shape_ratio: f64,
    pub maximum_deviation_m: f64,
    pub volume_tolerance_m3: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct SurfaceAudit {
    pub volume_m3: f64,
    pub minimum_area_m2: f64,
    pub minimum_shape_ratio: f64,
    pub bounding_box_m: [Point; 2],
    pub patch_area_m2: BTreeMap<String, f64>,
    pub deviation_bound_m: f64,
    pub checked_triangle_pairs: usize,
}
/// Private fields prevent consumers from mutating a qualified surface in place.
#[derive(Clone, Debug, Serialize)]
pub struct TaggedSurface {
    boundary: PartitionBoundary,
    policy: SurfacePolicy,
    audit: SurfaceAudit,
    boundary_sha256: String,
}
impl TaggedSurface {
    pub fn boundary(&self) -> &PartitionBoundary {
        &self.boundary
    }
    pub fn policy(&self) -> SurfacePolicy {
        self.policy
    }
    pub fn audit(&self) -> &SurfaceAudit {
        &self.audit
    }
    pub fn boundary_sha256(&self) -> &str {
        &self.boundary_sha256
    }
}

pub fn extract(boundary: PartitionBoundary, policy: SurfacePolicy) -> Result<TaggedSurface> {
    if !hash_valid(&boundary.partition_sha256)
        || !hash_valid(&boundary.acceptance_receipt_sha256)
        || boundary.compiler.trim().is_empty()
        || boundary.kernel.trim().is_empty()
    {
        return Err(invalid("partition/acceptance/compiler/kernel identity"));
    }
    if !policy.length_tolerance_m.is_finite()
        || policy.length_tolerance_m <= 0.0
        || !policy.minimum_area_m2.is_finite()
        || policy.minimum_area_m2 <= 0.0
        || !policy.minimum_shape_ratio.is_finite()
        || policy.minimum_shape_ratio <= 0.0
        || policy.minimum_shape_ratio > 1.0
        || !finite_nonnegative(policy.maximum_deviation_m)
        || !finite_nonnegative(policy.volume_tolerance_m3)
        || !finite_nonnegative(boundary.upstream_deviation_bound_m)
        || boundary.upstream_deviation_bound_m > policy.maximum_deviation_m
        || boundary
            .volume_bounds_m3
            .iter()
            .any(|x| !x.is_finite() || *x <= 0.0)
        || boundary.volume_bounds_m3[0] > boundary.volume_bounds_m3[1]
    {
        return Err(invalid("surface tolerance/enclosure"));
    }
    if boundary.facets.len() > 2048 || boundary.vertices_m.len() > 4096 {
        return Err(unsupported("bounded pairwise surface backend capacity"));
    }
    if boundary.facets.len() < 4
        || boundary.vertices_m.len() < 4
        || boundary.vertices_m.iter().flatten().any(|x| !x.is_finite())
        || !boundary.active_features.contains(&Feature::FacetedBoundary)
    {
        return Err(invalid(
            "finite nonempty metre boundary with declared features",
        ));
    }
    for i in 0..boundary.vertices_m.len() {
        for j in i + 1..boundary.vertices_m.len() {
            if norm(sub(boundary.vertices_m[i], boundary.vertices_m[j]))
                <= policy.length_tolerance_m
            {
                return Err(unsupported(
                    "coincident or unresolved boundary vertices; no automatic weld",
                ));
            }
        }
    }
    let mut edges: BTreeMap<(usize, usize), Vec<(usize, bool)>> = BTreeMap::new();
    let mut used = BTreeSet::new();
    let mut triangles = BTreeSet::new();
    let mut patches = BTreeMap::<String, f64>::new();
    let mut patch_tags = BTreeMap::new();
    let mut min_area = f64::INFINITY;
    let mut min_shape = f64::INFINITY;
    let mut volume6 = 0.0;
    let origin = boundary.vertices_m[0];
    for (fi, f) in boundary.facets.iter().enumerate() {
        let [a, b, c] = f.vertices;
        if f.vertices.iter().any(|i| *i >= boundary.vertices_m.len()) || a == b || b == c || a == c
        {
            return Err(invalid("facet vertex index"));
        }
        let mut key = f.vertices;
        key.sort();
        if !triangles.insert(key) {
            return Err(numerical("duplicate facet"));
        }
        if f.tag.patch.is_empty()
            || f.tag.inside_material.is_empty()
            || f.tag.outside_material.is_empty()
            || f.tag.inside_material == f.tag.outside_material
            || !finite_nonnegative(f.tag.roughness_m)
        {
            return Err(invalid("partition semantic material/patch/roughness tag"));
        }
        if patch_tags
            .insert(f.tag.patch.clone(), f.tag.clone())
            .is_some_and(|t| t != f.tag)
        {
            return Err(invalid("conflicting patch semantic tags"));
        }
        let [pa, pb, pc] = f.vertices.map(|i| boundary.vertices_m[i]);
        let area = norm(cross(sub(pb, pa), sub(pc, pa))) * 0.5;
        let l2 = dot(sub(pb, pa), sub(pb, pa))
            + dot(sub(pc, pb), sub(pc, pb))
            + dot(sub(pa, pc), sub(pa, pc));
        let shape = 4.0 * 3.0_f64.sqrt() * area / l2;
        if !area.is_finite()
            || area < policy.minimum_area_m2
            || !shape.is_finite()
            || shape < policy.minimum_shape_ratio
        {
            return Err(numerical("degenerate/sliver surface facet"));
        }
        min_area = min_area.min(area);
        min_shape = min_shape.min(shape);
        *patches.entry(f.tag.patch.clone()).or_default() += area;
        volume6 += det(sub(pa, origin), sub(pb, origin), sub(pc, origin));
        for (u, v) in [(a, b), (b, c), (c, a)] {
            used.insert(u);
            edges
                .entry((u.min(v), u.max(v)))
                .or_default()
                .push((fi, u < v));
        }
    }
    if used.len() != boundary.vertices_m.len() {
        return Err(invalid("unreferenced boundary vertex"));
    }
    if edges.values().any(|e| e.len() != 2 || e[0].1 == e[1].1) {
        return Err(numerical(
            "open/nonmanifold/inconsistently oriented boundary edge",
        ));
    }
    // Require one connected shell. Nested/disconnected shells need a partition
    // containment classifier; their signed-volume sum alone is not admission.
    let mut seen = BTreeSet::from([0]);
    loop {
        let old = seen.len();
        for edge in edges.values() {
            if seen.contains(&edge[0].0) || seen.contains(&edge[1].0) {
                seen.insert(edge[0].0);
                seen.insert(edge[1].0);
            }
        }
        if old == seen.len() {
            break;
        }
    }
    if seen.len() != boundary.facets.len() {
        return Err(unsupported(
            "multiple boundary shells require containment authority",
        ));
    }
    // A manifold edge census alone misses a pinched vertex. Its link must be
    // one cycle, checked through the incident-face adjacency graph.
    for v in used {
        let incident: BTreeSet<_> = boundary
            .facets
            .iter()
            .enumerate()
            .filter(|(_, f)| f.vertices.contains(&v))
            .map(|(i, _)| i)
            .collect();
        let mut reached = BTreeSet::from([*incident.first().unwrap()]);
        loop {
            let old = reached.len();
            for (&(a, b), e) in &edges {
                if (a == v || b == v) && (reached.contains(&e[0].0) || reached.contains(&e[1].0)) {
                    reached.insert(e[0].0);
                    reached.insert(e[1].0);
                }
            }
            if old == reached.len() {
                break;
            }
        }
        if reached != incident {
            return Err(numerical("nonmanifold vertex link"));
        }
    }
    let mut pairs = 0;
    for i in 0..boundary.facets.len() {
        for j in i + 1..boundary.facets.len() {
            pairs += 1;
            if unexpected_intersection(&boundary, i, j, policy.length_tolerance_m)? {
                return Err(numerical(
                    "self-intersecting or unresolved near-contact boundary",
                ));
            }
        }
    }
    let volume = volume6 / 6.0;
    if !volume.is_finite()
        || volume <= 0.0
        || volume < boundary.volume_bounds_m3[0] - policy.volume_tolerance_m3
        || volume > boundary.volume_bounds_m3[1] + policy.volume_tolerance_m3
    {
        return Err(numerical("orientation/partition volume enclosure mismatch"));
    }
    let bbox = bounding_box(&boundary.vertices_m);
    let audit = SurfaceAudit {
        volume_m3: volume,
        minimum_area_m2: min_area,
        minimum_shape_ratio: min_shape,
        bounding_box_m: bbox,
        patch_area_m2: patches,
        deviation_bound_m: boundary.upstream_deviation_bound_m,
        checked_triangle_pairs: pairs,
    };
    let boundary_sha256 = identity(&boundary)?;
    Ok(TaggedSurface {
        boundary,
        policy,
        audit,
        boundary_sha256,
    })
}

pub fn bounding_box(points: &[Point]) -> [Point; 2] {
    let mut b = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for p in points {
        for (i, x) in p.iter().enumerate() {
            b[0][i] = b[0][i].min(*x);
            b[1][i] = b[1][i].max(*x);
        }
    }
    b
}
fn unexpected_intersection(b: &PartitionBoundary, i: usize, j: usize, eps: f64) -> Result<bool> {
    let ai = b.facets[i].vertices;
    let bi = b.facets[j].vertices;
    let a = ai.map(|v| b.vertices_m[v]);
    let c = bi.map(|v| b.vertices_m[v]);
    let ba = bounding_box(&a);
    let bc = bounding_box(&c);
    if (0..3).any(|k| ba[1][k] + eps < bc[0][k] || bc[1][k] + eps < ba[0][k]) {
        return Ok(false);
    }
    let shared: Vec<_> = ai
        .iter()
        .filter(|v| bi.contains(v))
        .map(|v| b.vertices_m[*v])
        .collect();
    let allowed = |p: Point| -> bool {
        if shared.iter().any(|q| norm(sub(p, *q)) <= eps) {
            return true;
        }
        if shared.len() == 2 {
            let d = sub(shared[1], shared[0]);
            let t = dot(sub(p, shared[0]), d) / dot(d, d);
            return (0.0..=1.0).contains(&t) && norm(sub(p, add(shared[0], mul(d, t)))) <= eps;
        }
        false
    };
    let n = cross(sub(a[1], a[0]), sub(a[2], a[0]));
    let n = mul(n, 1.0 / norm(n));
    let distances = c.map(|p| dot(n, sub(p, a[0])));
    if distances.iter().all(|d| d.abs() <= eps) {
        // Coplanar triangles: test vertices and all edge crossings in 2D.
        let axis = (0..3)
            .max_by(|x, y| n[*x].abs().total_cmp(&n[*y].abs()))
            .unwrap();
        let project = |p: Point| -> [f64; 2] { [p[(axis + 1) % 3], p[(axis + 2) % 3]] };
        for (t, other) in [(a, c), (c, a)] {
            for p in t {
                if in_triangle_2d(project(p), other.map(project), eps) && !allowed(p) {
                    return Ok(true);
                }
            }
        }
        for k in 0..3 {
            for l in 0..3 {
                let p = a[k];
                let q = c[l];
                let r = sub(a[(k + 1) % 3], p);
                let s = sub(c[(l + 1) % 3], q);
                let r2 = project(r);
                let s2 = project(s);
                let qp = project(sub(q, p));
                let cross2 = |x: [f64; 2], y: [f64; 2]| x[0] * y[1] - x[1] * y[0];
                let den = cross2(r2, s2);
                if den.abs() > eps * norm(r).max(norm(s)) {
                    let t = cross2(qp, s2) / den;
                    let u = cross2(qp, r2) / den;
                    if (0.0..=1.0).contains(&t)
                        && (0.0..=1.0).contains(&u)
                        && !allowed(add(p, mul(r, t)))
                    {
                        return Ok(true);
                    }
                }
            }
        }
        return Ok(false);
    }
    for (t, other) in [(a, c), (c, a)] {
        let normal = cross(sub(other[1], other[0]), sub(other[2], other[0]));
        let normal = mul(normal, 1.0 / norm(normal));
        for k in 0..3 {
            let p = t[k];
            let q = t[(k + 1) % 3];
            let dp = dot(normal, sub(p, other[0]));
            let dq = dot(normal, sub(q, other[0]));
            if dp.abs() <= eps && inside_triangle(p, other, eps) && !allowed(p) {
                return Ok(true);
            }
            if (dp > eps && dq < -eps) || (dp < -eps && dq > eps) {
                let hit = add(p, mul(sub(q, p), dp / (dp - dq)));
                if inside_triangle(hit, other, eps) && !allowed(hit) {
                    return Ok(true);
                }
            }
            if dp.abs() <= eps && dq.abs() <= eps {
                // Ambiguous coplanar edge against a noncoplanar triangle.
                // Shared mesh edges are expected; unrelated near-contact abstains.
                if coplanar_segment_intersection(p, q, other, eps, &allowed) {
                    return Err(unsupported(
                        "unresolved coplanar edge/triangle intersection",
                    ));
                }
            }
        }
    }
    Ok(false)
}
fn inside_triangle(p: Point, t: [Point; 3], eps: f64) -> bool {
    let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
    (0..3).all(|k| {
        dot(cross(sub(t[(k + 1) % 3], t[k]), sub(p, t[k])), n)
            >= -eps * norm(n) * norm(sub(t[(k + 1) % 3], t[k]))
    })
}
fn in_triangle_2d(p: [f64; 2], t: [[f64; 2]; 3], eps: f64) -> bool {
    let c = |a: [f64; 2], b: [f64; 2], q: [f64; 2]| {
        (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])
    };
    let d = [c(t[0], t[1], p), c(t[1], t[2], p), c(t[2], t[0], p)];
    let scale = (0..3)
        .map(|i| {
            ((t[(i + 1) % 3][0] - t[i][0]).powi(2) + (t[(i + 1) % 3][1] - t[i][1]).powi(2)).sqrt()
        })
        .fold(0.0, f64::max);
    d.iter().all(|v| *v >= -eps * scale) || d.iter().all(|v| *v <= eps * scale)
}

fn coplanar_segment_intersection(
    p: Point,
    q: Point,
    t: [Point; 3],
    eps: f64,
    allowed: &impl Fn(Point) -> bool,
) -> bool {
    if (inside_triangle(p, t, eps) && !allowed(p)) || (inside_triangle(q, t, eps) && !allowed(q)) {
        return true;
    }
    let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
    let axis = (0..3)
        .max_by(|i, j| n[*i].abs().total_cmp(&n[*j].abs()))
        .unwrap();
    let project = |a: Point| [a[(axis + 1) % 3], a[(axis + 2) % 3]];
    let cross2 = |a: [f64; 2], b: [f64; 2]| a[0] * b[1] - a[1] * b[0];
    let r = sub(q, p);
    for i in 0..3 {
        let s = sub(t[(i + 1) % 3], t[i]);
        let qp = sub(t[i], p);
        let denominator = cross2(project(r), project(s));
        if denominator.abs() > eps * norm(r).max(norm(s)) {
            let u = cross2(project(qp), project(s)) / denominator;
            let v = cross2(project(qp), project(r)) / denominator;
            if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) && !allowed(add(p, mul(r, u))) {
                return true;
            }
        } else if !allowed(t[i]) {
            // Parallel/collinear edge ambiguity: fail when a triangle endpoint
            // lies on the segment. No coordinate-plane patch inference occurs.
            let length2 = dot(r, r);
            let parameter = dot(qp, r) / length2;
            if (0.0..=1.0).contains(&parameter) && norm(sub(t[i], add(p, mul(r, parameter)))) <= eps
            {
                return true;
            }
        }
    }
    false
}
