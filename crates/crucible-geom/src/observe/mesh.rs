//! Reviewed/adapted from lane1 private preparation. Closed-edge checks do not
//! certify self-intersection absence; accepted artifact review remains required.
use super::{cross, dot, norm, sub, Mesh, ObserveError};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq)]
pub struct Measures {
    pub volume_m3: f64,
    pub area_m2: f64,
    pub bounds_m: [[f64; 3]; 2],
    pub centroid_m: [f64; 3],
    pub boundary_euler: i64,
    pub second_moment_about_vertex0_m5: [[f64; 3]; 3],
}
pub fn validate(m: &Mesh) -> Result<(), ObserveError> {
    if m.vertices_m.len() < 4
        || m.vertices_m.len() > 100_000
        || m.triangles.len() < 4
        || m.triangles.len() > 200_000
        || m.vertices_m.iter().flatten().any(|x| !x.is_finite())
    {
        return Err(ObserveError::Invalid("mesh bounds/coordinates"));
    }
    let mut edges = BTreeMap::<(usize, usize), (usize, i32)>::new();
    let mut used = BTreeSet::new();
    let mut faces = BTreeSet::new();
    let mut links = vec![BTreeMap::<usize, Vec<usize>>::new(); m.vertices_m.len()];
    for &t in &m.triangles {
        if t.iter().any(|&i| i >= m.vertices_m.len())
            || t[0] == t[1]
            || t[1] == t[2]
            || t[0] == t[2]
        {
            return Err(ObserveError::Invalid("mesh index"));
        }
        let mut key = t;
        key.sort();
        if !faces.insert(key) {
            return Err(ObserveError::Invalid("duplicate triangle"));
        }
        let [a, b, c] = t.map(|i| m.vertices_m[i]);
        let area = norm(cross(sub(b, a), sub(c, a)));
        if !area.is_finite() || area <= 1e-24 {
            return Err(ObserveError::Invalid("degenerate triangle"));
        }
        used.extend(t);
        for axis in 0..3 {
            let link = &mut links[t[axis]];
            let a = t[(axis+1)%3]; let b = t[(axis+2)%3];
            link.entry(a).or_default().push(b);
            link.entry(b).or_default().push(a);
        }
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (k, s) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let e = edges.entry(k).or_default();
            e.0 += 1;
            e.1 += s;
        }
    }
    if used.len() != m.vertices_m.len() || edges.values().any(|&(n, s)| n != 2 || s != 0) {
        return Err(ObserveError::Invalid("open/nonmanifold/unoriented mesh"));
    }

    for link in links {
        if link.values().any(|v| v.len() != 2) {
            return Err(ObserveError::Invalid("nonmanifold vertex link"));
        }
        let start = *link
            .keys()
            .next()
            .ok_or(ObserveError::Invalid("unused vertex"))?;
        let mut visited = BTreeSet::new();
        let mut queue = vec![start];
        while let Some(v) = queue.pop() {
            if visited.insert(v) {
                queue.extend(&link[&v]);
            }
        }
        if visited.len() != link.len() {
            return Err(ObserveError::Invalid("disconnected vertex link"));
        }
    }
    Ok(())
}
/// Divergence integration about the first vertex reduces translation cancellation.
/// Edge checks do not certify self-intersection absence; this is an integration diagnostic.
pub fn measure(m: &Mesh) -> Result<Measures, ObserveError> {
    validate(m)?;
    let origin = m.vertices_m[0];
    let mut volume = 0.0;
    let mut area = 0.0;
    let mut moment = [0.0; 3];
    let mut second = [[0.0; 3]; 3];
    let mut edges = BTreeSet::new();
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for p in &m.vertices_m {
        for i in 0..3 {
            bounds[0][i] = bounds[0][i].min(p[i]);
            bounds[1][i] = bounds[1][i].max(p[i]);
        }
    }
    for &t in &m.triangles {
        let [a, b, c] = t.map(|i| sub(m.vertices_m[i], origin));
        let v = dot(a, cross(b, c)) / 6.0;
        volume += v;
        area += norm(cross(sub(b, a), sub(c, a))) / 2.0;
        for i in 0..3 {
            moment[i] += v * ((a[i] + b[i] + c[i]) / 4.0);
            for j in 0..3 {
                let sum_i = a[i] + b[i] + c[i];
                let sum_j = a[j] + b[j] + c[j];
                second[i][j] +=
                    v * (sum_i * sum_j + a[i] * a[j] + b[i] * b[j] + c[i] * c[j]) / 20.0;
            }
        }
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            edges.insert((a.min(b), a.max(b)));
        }
    }
    if !volume.is_finite()
        || volume <= 0.0
        || !area.is_finite()
        || moment
            .iter()
            .chain(second.iter().flatten())
            .any(|x| !x.is_finite())
    {
        return Err(ObserveError::Numerical("signed integral/orientation"));
    }
    Ok(Measures {
        volume_m3: volume,
        area_m2: area,
        bounds_m: bounds,
        centroid_m: std::array::from_fn(|i| origin[i] + moment[i] / volume),
        second_moment_about_vertex0_m5: second,
        boundary_euler: m.vertices_m.len() as i64 - edges.len() as i64 + m.triangles.len() as i64,
    })
}
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub centre_m: [f64; 3],
    pub normal: [f64; 3],
}
/// Integrate actual coplanar terminal triangles with outward normals, without coordinate-plane guessing.
pub fn port_plane(m: &Mesh, p: Plane, tolerance_m: f64) -> Result<(f64, [f64; 3]), ObserveError> {
    validate(m)?;
    if p.centre_m
        .iter()
        .chain(p.normal.iter())
        .any(|x| !x.is_finite())
        || (norm(p.normal) - 1.0).abs() > 1e-12
        || !tolerance_m.is_finite()
        || tolerance_m < 0.0
    {
        return Err(ObserveError::Invalid("port plane"));
    }
    let mut area = 0.0;
    let mut centre = [0.0; 3];
    for &t in &m.triangles {
        let q = t.map(|i| m.vertices_m[i]);
        if q.iter()
            .all(|&v| dot(sub(v, p.centre_m), p.normal).abs() <= tolerance_m)
        {
            let n = cross(sub(q[1], q[0]), sub(q[2], q[0]));
            let a = norm(n) / 2.0;
            if dot(n, p.normal) <= 0.0 {
                return Err(ObserveError::Invalid("port orientation"));
            }
            area += a;
            for i in 0..3 {
                centre[i] += a * (q[0][i] + q[1][i] + q[2][i]) / 3.0;
            }
        }
    }
    if area <= 0.0 || !area.is_finite() {
        return Err(ObserveError::Unsupported("port not resolved"));
    }
    Ok((area, centre.map(|x| x / area)))
}
