//! Complete patch distances plus independent bidirectional normal-ray diagnostics.
use super::{cross, dot, norm, sub, ObserveError};
/// Conservative interval for the complete patches, plus a floating point
/// nearest-feature estimate. The estimate and rays cannot upgrade the bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gap {
    pub lower_m: f64,
    pub upper_m: f64,
    pub estimate_m: f64,
    pub input_error_m: f64,
    pub independent_ray_min_m: Option<f64>,
}
fn closest_triangle(p: [f64; 3], a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    // Voronoi-region point/triangle distance; tests include edge and interior projections.
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return norm(ap);
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return norm(bp);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return norm(sub(p, std::array::from_fn(|i| a[i] + v * ab[i])));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return norm(cp);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return norm(sub(p, std::array::from_fn(|i| a[i] + w * ac[i])));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return norm(sub(p, std::array::from_fn(|i| b[i] + w * (c[i] - b[i]))));
    }
    let n = cross(ab, ac);
    dot(ap, n).abs() / norm(n)
}
fn check(patch: &[[[f64; 3]; 3]]) -> Result<(), ObserveError> {
    if patch.is_empty()
        || patch.len() > 4096
        || patch.iter().flatten().flatten().any(|x| !x.is_finite())
        || patch.iter().any(|t| {
            let area = norm(cross(sub(t[1], t[0]), sub(t[2], t[0])));
            !area.is_finite() || area <= 1e-24
        })
    {
        return Err(ObserveError::Invalid("triangle patches"));
    }
    Ok(())
}
fn segment_distance(p: [f64; 3], q: [f64; 3], r: [f64; 3], s: [f64; 3]) -> f64 {
    let u = sub(q, p);
    let v = sub(s, r);
    let w = sub(p, r);
    let a = dot(u, u);
    let b = dot(u, v);
    let c = dot(v, v);
    let d = dot(u, w);
    let e = dot(v, w);
    let denom = a * c - b * b;
    let mut t = if denom > f64::EPSILON * a * c {
        (b * e - c * d) / denom
    } else {
        0.0
    };
    t = t.clamp(0.0, 1.0);
    let mut z = (b * t + e) / c;
    if z < 0.0 {
        z = 0.0;
        t = (-d / a).clamp(0.0, 1.0);
    } else if z > 1.0 {
        z = 1.0;
        t = ((b - d) / a).clamp(0.0, 1.0);
    }
    norm(std::array::from_fn(|i| w[i] + t * u[i] - z * v[i]))
}
fn line_hit(origin: [f64; 3], dir: [f64; 3], q: [[f64; 3]; 3]) -> Option<f64> {
    let e1 = sub(q[1], q[0]);
    let e2 = sub(q[2], q[0]);
    let h = cross(dir, e2);
    let det = dot(e1, h);
    if det.abs() <= 64.0 * f64::EPSILON * norm(e1) * norm(e2) * norm(dir) {
        return None;
    }
    let s = sub(origin, q[0]);
    let u = dot(s, h) / det;
    let k = cross(s, e1);
    let v = dot(dir, k) / det;
    if u >= 0.0 && v >= 0.0 && u + v <= 1.0 {
        Some(dot(e2, k) / det)
    } else {
        None
    }
}
fn triangle_distance(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> f64 {
    let mut d = f64::INFINITY;
    for (x, y) in [(a, b), (b, a)] {
        for &p in &x {
            d = d.min(closest_triangle(p, y[0], y[1], y[2]));
        }
        for i in 0..3 {
            if let Some(t) = line_hit(x[i], sub(x[(i + 1) % 3], x[i]), y) {
                if (0.0..=1.0).contains(&t) {
                    return 0.0;
                }
            }
        }
    }
    for i in 0..3 {
        for j in 0..3 {
            d = d.min(segment_distance(a[i], a[(i + 1) % 3], b[j], b[(j + 1) % 3]));
        }
    }
    d
}
// Directed-rounding enclosures for projections. Any direction supplies a lower
// separation bound, even if its construction was rounded. Normal and edge-cross
// directions tighten that bound without trusting a closest-feature predicate.
#[derive(Clone, Copy)]
struct Interval {
    lo: f64,
    hi: f64,
}
impl Interval {
    fn exact(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn add(self, b: Self) -> Self {
        Self {
            lo: (self.lo + b.lo).next_down(),
            hi: (self.hi + b.hi).next_up(),
        }
    }
    fn subtract(self, b: Self) -> Self {
        Self {
            lo: (self.lo - b.hi).next_down(),
            hi: (self.hi - b.lo).next_up(),
        }
    }
    fn multiply(self, b: Self) -> Self {
        let products = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        Self {
            lo: products
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min)
                .next_down(),
            hi: products
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        }
    }
}
fn projection(p: [f64; 3], axis: [f64; 3]) -> Interval {
    (0..3)
        .map(|a| Interval::exact(p[a]).multiply(Interval::exact(axis[a])))
        .fold(Interval::exact(0.), Interval::add)
}
fn norm_upper(v: [Interval; 3]) -> f64 {
    v.iter()
        .map(|v| {
            let x = v.lo.abs().max(v.hi.abs());
            Interval {
                lo: 0.,
                hi: (x * x).next_up(),
            }
        })
        .fold(Interval::exact(0.), Interval::add)
        .hi
        .sqrt()
        .next_up()
}
fn pair_bounds(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> (f64, f64) {
    let mut axes = vec![
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        cross(sub(a[1], a[0]), sub(a[2], a[0])),
        cross(sub(b[1], b[0]), sub(b[2], b[0])),
    ];
    for i in 0..3 {
        for j in 0..3 {
            axes.push(cross(sub(a[(i + 1) % 3], a[i]), sub(b[(j + 1) % 3], b[j])));
        }
    }
    let mut lower: f64 = 0.;
    let mut upper = f64::INFINITY;
    for axis in axes {
        let scale = axis.iter().map(|x| x.abs()).fold(0., f64::max);
        if !scale.is_finite() || scale == 0. {
            continue;
        }
        let axis = axis.map(|x| x / scale);
        let length = norm_upper(axis.map(Interval::exact));
        let range = |t: [[f64; 3]; 3]| {
            let p = t.map(|p| projection(p, axis));
            Interval {
                lo: p.iter().map(|p| p.lo).fold(f64::INFINITY, f64::min),
                hi: p.iter().map(|p| p.hi).fold(f64::NEG_INFINITY, f64::max),
            }
        };
        let x = range(a);
        let y = range(b);
        let gap = (y.lo - x.hi)
            .next_down()
            .max((x.lo - y.hi).next_down())
            .max(0.);
        if length.is_finite() {
            lower = lower.max((gap / length).next_down().max(0.));
        }
    }
    for x in a {
        for y in b {
            upper = upper.min(norm_upper(std::array::from_fn(|i| {
                Interval::exact(x[i]).subtract(Interval::exact(y[i]))
            })));
        }
    }
    (lower, upper)
}
/// The estimate considers edge/edge and piercing intersections. The conservative
/// interval uses outward-rounded separating projections and feasible vertex
/// distances instead of trusting the estimate's floating point predicates.
/// A caller must supply all governing faces. Each input
/// patch is assumed displaced by at most error_m under its accepted review.
/// Arbitrary nominal thickness/topology is not inferred from Hausdorff error.
pub fn patch_gap(
    a: &[[[f64; 3]; 3]],
    b: &[[[f64; 3]; 3]],
    error_m: f64,
) -> Result<Gap, ObserveError> {
    check(a)?;
    check(b)?;
    if !error_m.is_finite() || error_m < 0.0 {
        return Err(ObserveError::Invalid("gap error"));
    }
    if a.len().saturating_mul(b.len()) > 1_000_000 {
        return Err(ObserveError::Unsupported("patch pair budget"));
    }
    let mut minimum = f64::INFINITY;
    let mut lower = f64::INFINITY;
    let mut upper = f64::INFINITY;
    let mut ray_min = f64::INFINITY;
    for t in a {
        let origin = std::array::from_fn(|i| (t[0][i] + t[1][i] + t[2][i]) / 3.0);
        let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
        let dir = n.map(|x| x / norm(n));
        for q in b {
            let (lo, hi) = pair_bounds(*t, *q);
            lower = lower.min(lo);
            upper = upper.min(hi);
            minimum = minimum.min(triangle_distance(*t, *q));
            if let Some(d) = line_hit(origin, dir, *q) {
                ray_min = ray_min.min(d.abs());
            }
        }
    }
    let input_error = (2.0 * error_m).next_up();
    if !minimum.is_finite() || !(upper + input_error).is_finite() {
        return Err(ObserveError::Numerical("gap overflow"));
    }
    Ok(Gap {
        lower_m: (lower - input_error).next_down().max(0.0),
        upper_m: (upper + input_error).next_up(),
        estimate_m: minimum,
        input_error_m: error_m,
        independent_ray_min_m: ray_min.is_finite().then_some(ray_min),
    })
}
