//! Standard engineering views of a meshed cartridge: a full-length cutaway strip, head
//! detail cutaway, an x-ray view, and gallery thumbnails. The cartridge axis is laid
//! horizontally (flow left → right).

use super::raster::{render, Camera, Image, Layer, V};
use crate::cad::export::owner_color;
use crate::cad::mesh::Mesh;

pub const BG: [[f32; 3]; 2] = [[0.030, 0.036, 0.048], [0.010, 0.012, 0.018]];
const TITANIUM: [f32; 3] = [0.50, 0.52, 0.56];
const CUT: [f32; 3] = [0.80, 0.77, 0.70];

/// Design frame (x, y, z downstream; mm) → render frame (X along flow, Y up, Z to viewer).
fn xf(length: f64) -> impl Fn(V) -> V {
    move |p: V| [p[2] - 0.5 * length, -p[0], -p[1]]
}

fn layers<'a>(
    fluid: &'a Mesh,
    part: &'a Mesh,
    xray: bool,
    fluid_col: &'a dyn Fn(u16) -> [f32; 3],
    part_col: &'a dyn Fn(u16) -> [f32; 3],
) -> Vec<Layer<'a>> {
    if xray {
        vec![
            Layer {
                mesh: fluid,
                color: fluid_col,
                alpha: 1.0,
                clip: None,
                cap: CUT,
                specular: 0.35,
            },
            Layer {
                mesh: part,
                color: part_col,
                alpha: 0.22,
                clip: None,
                cap: CUT,
                specular: 0.6,
            },
        ]
    } else {
        vec![
            Layer {
                mesh: fluid,
                color: fluid_col,
                alpha: 1.0,
                clip: None,
                cap: CUT,
                specular: 0.35,
            },
            Layer {
                mesh: part,
                color: part_col,
                alpha: 1.0,
                clip: Some(([0.0, 0.0, 1.0], 0.0)),
                cap: CUT,
                specular: 0.7,
            },
        ]
    }
}

fn camera_for(
    center: V,
    span: f64,
    h_over_w: f64,
    min_vis: f64,
    elev: f64,
    az: f64,
    fov: f64,
) -> Camera {
    let vis_h = (span * h_over_w * 1.2).max(min_vis);
    let d = 0.5 * vis_h / (fov.to_radians() * 0.5).tan();
    let eye = [
        center[0] + d * az.sin() * elev.cos(),
        center[1] + d * elev.sin(),
        center[2] + d * az.cos() * elev.cos(),
    ];
    Camera {
        eye,
        target: center,
        up: [0.0, 1.0, 0.0],
        fov_deg: fov,
    }
}

/// Full-length cutaway strip.
pub fn strip(
    fluid: &Mesh,
    part: &Mesh,
    length: f64,
    radius: f64,
    w: usize,
    h: usize,
    xray: bool,
) -> Image {
    let fc = |o: u16| owner_color(o);
    let pc = |_: u16| TITANIUM;
    let ls = layers(fluid, part, xray, &fc, &pc);
    let cam = camera_for(
        [0.0, 0.0, 0.0],
        length,
        h as f64 / w as f64,
        3.0 * radius,
        0.22,
        -0.10,
        14.0,
    );
    render(&ls, &cam, &xf(length), w, h, 2, BG)
}

/// Close-up of the axial window `[z0, z1]` (mm from the top face).
#[allow(clippy::too_many_arguments)]
pub fn detail(
    fluid: &Mesh,
    part: &Mesh,
    length: f64,
    radius: f64,
    z0: f64,
    z1: f64,
    w: usize,
    h: usize,
    xray: bool,
) -> Image {
    let fc = |o: u16| owner_color(o);
    let pc = |_: u16| TITANIUM;
    let ls = layers(fluid, part, xray, &fc, &pc);
    let span = z1 - z0;
    let center = [0.5 * (z0 + z1) - 0.5 * length, 0.0, 0.0];
    let (elev, az) = if xray { (0.42, 0.55) } else { (0.30, -0.45) };
    let cam = camera_for(
        center,
        span,
        h as f64 / w as f64,
        2.9 * radius,
        elev,
        az,
        28.0,
    );
    render(&ls, &cam, &xf(length), w, h, 2, BG)
}

/// Engineering plate: full strip on top, cutaway and x-ray head details below.
pub fn plate(fluid: &Mesh, part: &Mesh, length: f64, radius: f64, window: (f64, f64)) -> Image {
    let (w, top, bottom) = (2400usize, 360usize, 760usize);
    let mut img = Image::filled(w, top + bottom, [8, 10, 13]);
    img.blit(&strip(fluid, part, length, radius, w, top, false), 0, 0);
    img.blit(
        &detail(
            fluid,
            part,
            length,
            radius,
            window.0,
            window.1,
            w / 2,
            bottom,
            false,
        ),
        0,
        top,
    );
    img.blit(
        &detail(
            fluid,
            part,
            length,
            radius,
            window.0,
            window.1,
            w / 2,
            bottom,
            true,
        ),
        w / 2,
        top,
    );
    img
}

/// Hero view: the whole cartridge seen from above and in front, the printed part as a
/// translucent shell over the colour-coded fluid network.
pub fn hero(fluid: &Mesh, part: &Mesh, length: f64, radius: f64, w: usize, h: usize) -> Image {
    let fc = |o: u16| owner_color(o);
    let pc = |_: u16| TITANIUM;
    let ls = layers(fluid, part, true, &fc, &pc);
    let cam = camera_for(
        [0.0, 0.0, 0.0],
        0.86 * length,
        h as f64 / w as f64,
        3.0 * radius,
        0.40,
        0.62,
        20.0,
    );
    render(&ls, &cam, &xf(length), w, h, 2, BG)
}

/// Small x-ray thumbnail for galleries.
pub fn thumbnail(fluid: &Mesh, part: &Mesh, length: f64, radius: f64, w: usize, h: usize) -> Image {
    strip(fluid, part, length, radius, w, h, true)
}
