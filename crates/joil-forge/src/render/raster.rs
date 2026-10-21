//! Software rasteriser for engineering cutaways: perspective camera, z-buffer, clip plane
//! with flat cut caps, Lambert + Blinn–Phong + rim lighting, translucent overlay layer and
//! supersampled anti-aliasing. Deterministic and dependency-free.

use crate::cad::mesh::Mesh;

pub type V = [f64; 3];

fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: V) -> V {
    let l = dot(a, a).sqrt().max(1e-30);
    [a[0] / l, a[1] / l, a[2] / l]
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub eye: V,
    pub target: V,
    pub up: V,
    pub fov_deg: f64,
}

pub struct Layer<'a> {
    pub mesh: &'a Mesh,
    pub color: &'a dyn Fn(u16) -> [f32; 3],
    /// 1 = opaque; below 1 the layer is composited as a translucent overlay.
    pub alpha: f32,
    /// Keep points with `dot(p, n) <= d`; back faces exposed by the cut show `cap`.
    pub clip: Option<(V, f64)>,
    pub cap: [f32; 3],
    pub specular: f32,
}

pub struct Image {
    pub w: usize,
    pub h: usize,
    pub rgb: Vec<u8>,
}

struct Frame {
    w: usize,
    h: usize,
    depth: Vec<f32>,
    color: Vec<[f32; 3]>,
    over: Vec<[f32; 4]>,
}

/// Map a design-frame point (mm) to the render frame by a caller-provided transform.
pub type Xform<'a> = &'a dyn Fn(V) -> V;

fn shade(n: V, view: V, base: [f32; 3], spec: f32) -> [f32; 3] {
    let key = norm([-0.45, 0.75, 0.55]);
    let fill = norm([0.6, -0.2, 0.4]);
    let lam = dot(n, key).max(0.0);
    let lam2 = dot(n, fill).max(0.0);
    let hemi = 0.5 + 0.5 * n[1];
    let h = norm([key[0] + view[0], key[1] + view[1], key[2] + view[2]]);
    let s = dot(n, h).max(0.0).powf(48.0) * spec as f64;
    let rim = (1.0 - dot(n, view).abs()).powf(3.0) * 0.35;
    let mut c = [0.0f32; 3];
    for k in 0..3 {
        let b = base[k] as f64;
        let v = b * (0.18 + 0.22 * hemi + 0.75 * lam + 0.25 * lam2) + s + rim * (0.35 + 0.65 * b);
        c[k] = v as f32;
    }
    c
}

pub fn render(
    layers: &[Layer],
    cam: &Camera,
    xf: Xform,
    w: usize,
    h: usize,
    ss: usize,
    background: [[f32; 3]; 2],
) -> Image {
    let (sw, sh) = (w * ss, h * ss);
    let mut fr = Frame {
        w: sw,
        h: sh,
        depth: vec![f32::INFINITY; sw * sh],
        color: vec![[0.0; 3]; sw * sh],
        over: vec![[0.0; 4]; sw * sh],
    };
    for y in 0..sh {
        let t = y as f32 / sh as f32;
        for x in 0..sw {
            let cx = x as f32 / sw as f32 - 0.5;
            let vig = 1.0 - 0.35 * (cx * cx * 1.2 + (t - 0.5) * (t - 0.5));
            let mut c = [0.0; 3];
            for k in 0..3 {
                c[k] = (background[0][k] * (1.0 - t) + background[1][k] * t) * vig;
            }
            fr.color[y * sw + x] = c;
        }
    }
    let fwd = norm(sub(cam.target, cam.eye));
    let right = norm(cross(fwd, cam.up));
    let up = cross(right, fwd);
    let f = 1.0 / (cam.fov_deg.to_radians() * 0.5).tan();
    let aspect = sw as f64 / sh as f64;
    let project = |p: V| -> (f64, f64, f64) {
        let d = sub(p, cam.eye);
        let (x, y, z) = (dot(d, right), dot(d, up), dot(d, fwd));
        let z = z.max(1e-3);
        let sx = (0.5 + 0.5 * f * x / (z * aspect)) * sw as f64;
        let sy = (0.5 - 0.5 * f * y / z) * sh as f64;
        (sx, sy, z)
    };
    // Opaque layers first, then translucent overlays tested against opaque depth.
    let mut order: Vec<&Layer> = layers.iter().filter(|l| l.alpha >= 1.0).collect();
    order.extend(layers.iter().filter(|l| l.alpha < 1.0));
    for layer in order {
        let m = layer.mesh;
        let pts: Vec<V> =
            m.v.iter()
                .map(|v| xf([v[0] as f64, v[1] as f64, v[2] as f64]))
                .collect();
        let scr: Vec<(f64, f64, f64)> = pts.iter().map(|p| project(*p)).collect();
        let vn: Vec<V> = crate::cad::export::vertex_normals(m)
            .iter()
            .zip(&m.v)
            .zip(&pts)
            .map(|((n, v), p)| {
                let q = xf([
                    v[0] as f64 + n[0] as f64,
                    v[1] as f64 + n[1] as f64,
                    v[2] as f64 + n[2] as f64,
                ]);
                norm(sub(q, *p))
            })
            .collect();
        for t in &m.f {
            let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
            let (pa, pb, pc) = (scr[a], scr[b], scr[c]);
            if pa.2 < 1.0 || pb.2 < 1.0 || pc.2 < 1.0 {
                continue; // behind or at the near plane
            }
            let n = norm(cross(sub(pts[b], pts[a]), sub(pts[c], pts[a])));
            let center = [
                (pts[a][0] + pts[b][0] + pts[c][0]) / 3.0,
                (pts[a][1] + pts[b][1] + pts[c][1]) / 3.0,
                (pts[a][2] + pts[b][2] + pts[c][2]) / 3.0,
            ];
            let view = norm(sub(cam.eye, center));
            let facing = dot(n, view) > 0.0;
            let base = (layer.color)(m.owner.get(a).copied().unwrap_or(u16::MAX));
            let x0 = pa.0.min(pb.0).min(pc.0).floor().max(0.0) as i64;
            let x1 = pa.0.max(pb.0).max(pc.0).ceil().min(sw as f64 - 1.0) as i64;
            let y0 = pa.1.min(pb.1).min(pc.1).floor().max(0.0) as i64;
            let y1 = pa.1.max(pb.1).max(pc.1).ceil().min(sh as f64 - 1.0) as i64;
            if x0 > x1 || y0 > y1 {
                continue;
            }
            let area = (pb.0 - pa.0) * (pc.1 - pa.1) - (pb.1 - pa.1) * (pc.0 - pa.0);
            if area.abs() < 1e-12 {
                continue;
            }
            let cap_col = {
                let cn = layer.clip.map(|(cn, _)| cn).unwrap_or(n);
                let cn = if dot(cn, view) < 0.0 {
                    [-cn[0], -cn[1], -cn[2]]
                } else {
                    cn
                };
                shade(cn, view, layer.cap, 0.15)
            };
            if !facing && layer.clip.is_none() && layer.alpha >= 1.0 {
                continue;
            }
            for py in y0..=y1 {
                for px in x0..=x1 {
                    let (qx, qy) = (px as f64 + 0.5, py as f64 + 0.5);
                    let w0 = ((pb.0 - qx) * (pc.1 - qy) - (pb.1 - qy) * (pc.0 - qx)) / area;
                    let w1 = ((pc.0 - qx) * (pa.1 - qy) - (pc.1 - qy) * (pa.0 - qx)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    // perspective-correct interpolation
                    let iz = w0 / pa.2 + w1 / pb.2 + w2 / pc.2;
                    let z = 1.0 / iz;
                    if let Some((cn, cd)) = layer.clip {
                        let p = [
                            (w0 * pts[a][0] / pa.2 + w1 * pts[b][0] / pb.2 + w2 * pts[c][0] / pc.2)
                                * z,
                            (w0 * pts[a][1] / pa.2 + w1 * pts[b][1] / pb.2 + w2 * pts[c][1] / pc.2)
                                * z,
                            (w0 * pts[a][2] / pa.2 + w1 * pts[b][2] / pb.2 + w2 * pts[c][2] / pc.2)
                                * z,
                        ];
                        if dot(p, cn) > cd {
                            continue;
                        }
                    }
                    let idx = py as usize * sw + px as usize;
                    let zf = z as f32;
                    if zf >= fr.depth[idx] {
                        continue;
                    }
                    let col = if facing {
                        let mut nn = [0.0; 3];
                        for k in 0..3 {
                            nn[k] = (w0 * vn[a][k] / pa.2
                                + w1 * vn[b][k] / pb.2
                                + w2 * vn[c][k] / pc.2)
                                * z;
                        }
                        let nn = norm(nn);
                        let nn = if dot(nn, view) < 0.0 { n } else { nn };
                        shade(nn, view, base, layer.specular)
                    } else {
                        cap_col
                    };
                    if layer.alpha >= 1.0 {
                        if zf < fr.depth[idx] {
                            fr.depth[idx] = zf;
                            fr.color[idx] = col;
                        }
                    } else if zf < fr.depth[idx] {
                        let o = &mut fr.over[idx];
                        let a = layer.alpha;
                        for k in 0..3 {
                            o[k] += col[k] * a;
                        }
                        o[3] += a;
                    }
                }
            }
        }
    }
    resolve(&fr, ss, w, h)
}

fn resolve(fr: &Frame, ss: usize, w: usize, h: usize) -> Image {
    let mut rgb = vec![0u8; 3 * w * h];
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 3];
            for dy in 0..ss {
                for dx in 0..ss {
                    let i = (y * ss + dy) * fr.w + x * ss + dx;
                    let mut c = fr.color[i];
                    let o = fr.over[i];
                    if o[3] > 0.0 {
                        // weighted-average translucent layers, total coverage saturating
                        let cov = 1.0 - (-1.2 * o[3]).exp();
                        for k in 0..3 {
                            c[k] = c[k] * (1.0 - cov) + (o[k] / o[3]) * cov;
                        }
                    }
                    for k in 0..3 {
                        acc[k] += c[k];
                    }
                }
            }
            for k in 0..3 {
                let v = acc[k] / (ss * ss) as f32;
                // filmic-ish tone curve and gamma
                let t = v / (1.0 + 0.6 * v) * 1.6;
                rgb[3 * (y * w + x) + k] = (t.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8;
            }
        }
    }
    let _ = fr.h;
    Image { w, h, rgb }
}

impl Image {
    /// Paste `other` at `(x, y)`.
    pub fn blit(&mut self, other: &Image, x: usize, y: usize) {
        for j in 0..other.h {
            if y + j >= self.h {
                break;
            }
            for i in 0..other.w {
                if x + i >= self.w {
                    break;
                }
                let s = 3 * (j * other.w + i);
                let d = 3 * ((y + j) * self.w + x + i);
                self.rgb[d..d + 3].copy_from_slice(&other.rgb[s..s + 3]);
            }
        }
    }

    pub fn filled(w: usize, h: usize, c: [u8; 3]) -> Self {
        Self {
            w,
            h,
            rgb: (0..w * h).flat_map(|_| c).collect(),
        }
    }

    pub fn png(&self) -> Vec<u8> {
        super::png::encode_rgb(self.w, self.h, &self.rgb)
    }
}
