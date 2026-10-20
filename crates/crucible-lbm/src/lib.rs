//! Small CPU-reference D2Q9 kernels. Synthetic numerical software fixtures only.

pub const Q: usize = 9;
pub const C: [[i32; 2]; Q] = [
    [0, 0],
    [1, 0],
    [0, 1],
    [-1, 0],
    [0, -1],
    [1, 1],
    [-1, 1],
    [-1, -1],
    [1, -1],
];
pub const W: [f64; Q] = [
    4.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
];
const OPP: [usize; Q] = [0, 3, 4, 1, 2, 7, 8, 5, 6];

#[derive(Clone, Debug, PartialEq)]
pub enum LbmError {
    InvalidShape,
    InvalidRelaxation,
    InvalidState,
    Bounds,
}

#[derive(Clone, Debug)]
pub struct D2Q9 {
    width: usize,
    height: usize,
    tau: f64,
    f: Vec<[f64; Q]>,
    solid: Vec<bool>,
}

impl D2Q9 {
    pub fn new(width: usize, height: usize, tau: f64) -> Result<Self, LbmError> {
        let n = width.checked_mul(height).ok_or(LbmError::InvalidShape)?;
        if width < 2 || height < 2 || n > 1_000_000 {
            return Err(LbmError::InvalidShape);
        }
        if !tau.is_finite() || tau <= 0.5 {
            return Err(LbmError::InvalidRelaxation);
        }
        Ok(Self {
            width,
            height,
            tau,
            f: vec![[0.0; Q]; n],
            solid: vec![false; n],
        })
    }
    fn idx(&self, x: usize, y: usize) -> Result<usize, LbmError> {
        if x >= self.width || y >= self.height {
            Err(LbmError::Bounds)
        } else {
            Ok(y * self.width + x)
        }
    }
    pub fn set_solid(&mut self, x: usize, y: usize, solid: bool) -> Result<(), LbmError> {
        let i = self.idx(x, y)?;
        self.solid[i] = solid;
        Ok(())
    }
    pub fn set_equilibrium(
        &mut self,
        x: usize,
        y: usize,
        rho: f64,
        u: [f64; 2],
    ) -> Result<(), LbmError> {
        let i = self.idx(x, y)?;
        validate_macro(rho, u)?;
        self.f[i] = equilibrium(rho, u);
        Ok(())
    }
    pub fn populations(&self, x: usize, y: usize) -> Result<[f64; Q], LbmError> {
        Ok(self.f[self.idx(x, y)?])
    }
    pub fn macro_at(&self, x: usize, y: usize) -> Result<(f64, [f64; 2]), LbmError> {
        macros(&self.f[self.idx(x, y)?])
    }
    pub fn collide_stream(&mut self) -> Result<(), LbmError> {
        let mut post = vec![[0.0; Q]; self.f.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width + x;
                if self.solid[i] {
                    continue;
                }
                let (rho, u) = macros(&self.f[i])?;
                let eq = equilibrium(rho, u);
                for q in 0..Q {
                    post[i][q] = self.f[i][q] - (self.f[i][q] - eq[q]) / self.tau;
                }
            }
        }
        let mut next = vec![[0.0; Q]; self.f.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width + x;
                if self.solid[i] {
                    continue;
                }
                for q in 0..Q {
                    let nx = x as i64 + C[q][0] as i64;
                    let ny = y as i64 + C[q][1] as i64;
                    // Periodic domain; solid-link bounce-back is local and mass conservative.
                    let tx = nx.rem_euclid(self.width as i64) as usize;
                    let ty = ny.rem_euclid(self.height as i64) as usize;
                    let j = ty * self.width + tx;
                    if self.solid[j] {
                        next[i][OPP[q]] += post[i][q];
                    } else {
                        next[j][q] += post[i][q];
                    }
                }
            }
        }
        if next.iter().flatten().any(|v| !v.is_finite()) {
            return Err(LbmError::InvalidState);
        }
        self.f = next;
        Ok(())
    }
    pub fn total_mass(&self) -> Result<f64, LbmError> {
        let mut sum = Kahan::default();
        for (i, f) in self.f.iter().enumerate() {
            if !self.solid[i] {
                for v in f {
                    if !v.is_finite() {
                        return Err(LbmError::InvalidState);
                    }
                    sum.add(*v);
                }
            }
        }
        Ok(sum.sum)
    }
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Apply interpolated halfway wall reflection on explicitly listed fluid links.
    pub fn collide_stream_curved_wall(
        &mut self,
        links: &[CurvedWallLink],
    ) -> Result<[f64; 2], LbmError> {
        let mut force = [0.0; 2];
        let mut updates = Vec::with_capacity(links.len());
        for link in links {
            if link.direction == 0
                || link.direction >= Q
                || !link.wall_fraction_q.is_finite()
                || link.wall_fraction_q <= 0.0
                || link.wall_fraction_q > 1.0
            {
                return Err(LbmError::InvalidState);
            }
            let i = self.idx(link.fluid_cell.0, link.fluid_cell.1)?;
            if self.solid[i] {
                return Err(LbmError::InvalidState);
            }
            let (cx, cy) = (C[link.direction][0], C[link.direction][1]);
            let nx = link.fluid_cell.0 as i64 + cx as i64;
            let ny = link.fluid_cell.1 as i64 + cy as i64;
            let j = self.idx(
                nx.rem_euclid(self.width as i64) as usize,
                ny.rem_euclid(self.height as i64) as usize,
            )?;
            if !self.solid[j] {
                return Err(LbmError::InvalidState);
            }
            let q = link.wall_fraction_q;
            let opp = OPP[link.direction];
            let outgoing = self.f[i][link.direction];
            let incoming = if q < 0.5 {
                let behind_x =
                    (link.fluid_cell.0 as i64 - cx as i64).rem_euclid(self.width as i64) as usize;
                let behind_y =
                    (link.fluid_cell.1 as i64 - cy as i64).rem_euclid(self.height as i64) as usize;
                let behind = self.f[self.idx(behind_x, behind_y)?][link.direction];
                2.0 * q * outgoing + (1.0 - 2.0 * q) * behind
            } else {
                let other = self.f[i][opp];
                outgoing / (2.0 * q) + (2.0 * q - 1.0) * other / (2.0 * q)
            };
            if !incoming.is_finite() {
                return Err(LbmError::InvalidState);
            }
            force[0] += (outgoing + incoming) * cx as f64;
            force[1] += (outgoing + incoming) * cy as f64;
            updates.push((i, opp, incoming));
        }
        if force.iter().any(|v| !v.is_finite()) {
            return Err(LbmError::InvalidState);
        }
        for (i, opp, value) in updates {
            self.f[i][opp] = value;
        }
        Ok(force)
    }

    /// D2Q9 MRT collision using the standard Lallemand-Luo moment basis.
    pub fn collide_stream_mrt(&mut self, s_e: f64, s_eps: f64, s_q: f64) -> Result<(), LbmError> {
        let sv = 1.0 / self.tau;
        if [s_e, s_eps, s_q, sv]
            .iter()
            .any(|s| !s.is_finite() || *s <= 0.0 || *s >= 2.0)
        {
            return Err(LbmError::InvalidRelaxation);
        }
        const M: [[f64; Q]; Q] = [
            [1., 1., 1., 1., 1., 1., 1., 1., 1.],
            [-4., -1., -1., -1., -1., 2., 2., 2., 2.],
            [4., -2., -2., -2., -2., 1., 1., 1., 1.],
            [0., 1., 0., -1., 0., 1., -1., -1., 1.],
            [0., -2., 0., 2., 0., 1., -1., -1., 1.],
            [0., 0., 1., 0., -1., 1., 1., -1., -1.],
            [0., 0., -2., 0., 2., 1., 1., -1., -1.],
            [0., 1., -1., 1., -1., 0., 0., 0., 0.],
            [0., 0., 0., 0., 0., 1., -1., 1., -1.],
        ];
        let rates: [f64; Q] = [0., s_e, s_eps, 0., s_q, 0., s_q, sv, sv];
        let mut next = self.f.clone();
        for (i, next_cell) in next.iter_mut().enumerate().take(self.f.len()) {
            if self.solid[i] {
                continue;
            }
            let (rho, u) = macros(&self.f[i])?;
            let eq = equilibrium(rho, u);
            let mut m = [0.; Q];
            let mut me = [0.; Q];
            for a in 0..Q {
                for q in 0..Q {
                    m[a] += M[a][q] * self.f[i][q];
                    me[a] += M[a][q] * eq[q];
                }
            }
            let target = [
                rho,
                -2. * rho + 3. * rho * (u[0] * u[0] + u[1] * u[1]),
                rho - 3. * rho * (u[0] * u[0] + u[1] * u[1]),
                rho * u[0],
                -rho * u[0],
                rho * u[1],
                -rho * u[1],
                rho * (u[0] * u[0] - u[1] * u[1]),
                rho * u[0] * u[1],
            ];
            for a in 0..Q {
                let eqm = if a == 0 || a == 3 || a == 5 {
                    me[a]
                } else {
                    target[a]
                };
                m[a] -= rates[a] * (m[a] - eqm);
            }
            // Invert the fixed basis by solving its small dense linear system.
            let mut a = [[0.; Q + 1]; Q];
            for r in 0..Q {
                a[r][..Q].copy_from_slice(&M[r]);
                a[r][Q] = m[r];
            }
            for c in 0..Q {
                let pivot = (c..Q)
                    .max_by(|x, y| a[*x][c].abs().total_cmp(&a[*y][c].abs()))
                    .unwrap();
                if a[pivot][c].abs() < 1e-14 {
                    return Err(LbmError::InvalidState);
                }
                a.swap(c, pivot);
                let d = a[c][c];
                for val in a[c].iter_mut().take(Q + 1).skip(c) {
                    *val /= d;
                }
                let pivot_row = a[c];
                for (r, row) in a.iter_mut().enumerate().take(Q) {
                    if r != c {
                        let f = row[c];
                        for (val, &piv) in row.iter_mut().zip(pivot_row.iter()).take(Q + 1).skip(c)
                        {
                            *val -= f * piv;
                        }
                    }
                }
            }
            for (q, row) in a.iter().enumerate().take(Q) {
                next_cell[q] = row[Q];
            }
        }
        if next.iter().flatten().any(|v| !v.is_finite()) {
            return Err(LbmError::InvalidState);
        }
        // Periodic streaming follows collision; solid links reflect locally.
        let mut streamed = vec![[0.0; Q]; self.f.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width + x;
                if self.solid[i] {
                    continue;
                }
                for q in 0..Q {
                    let tx = (x as i64 + C[q][0] as i64).rem_euclid(self.width as i64) as usize;
                    let ty = (y as i64 + C[q][1] as i64).rem_euclid(self.height as i64) as usize;
                    let j = ty * self.width + tx;
                    if self.solid[j] {
                        streamed[i][OPP[q]] += next[i][q];
                    } else {
                        streamed[j][q] += next[i][q];
                    }
                }
            }
        }
        if streamed.iter().flatten().any(|v| !v.is_finite()) {
            return Err(LbmError::InvalidState);
        }
        self.f = streamed;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CurvedWallLink {
    pub fluid_cell: (usize, usize),
    pub direction: usize,
    pub wall_fraction_q: f64,
}

fn validate_macro(rho: f64, u: [f64; 2]) -> Result<(), LbmError> {
    if !rho.is_finite()
        || rho <= 0.0
        || u.iter().any(|v| !v.is_finite())
        || u[0] * u[0] + u[1] * u[1] >= 1.0
    {
        Err(LbmError::InvalidState)
    } else {
        Ok(())
    }
}

pub fn equilibrium(rho: f64, u: [f64; 2]) -> [f64; Q] {
    let u2 = u[0] * u[0] + u[1] * u[1];
    std::array::from_fn(|q| {
        let cu = C[q][0] as f64 * u[0] + C[q][1] as f64 * u[1];
        W[q] * rho * (1.0 + 3.0 * cu + 4.5 * cu * cu - 1.5 * u2)
    })
}

fn macros(f: &[f64; Q]) -> Result<(f64, [f64; 2]), LbmError> {
    if f.iter().any(|v| !v.is_finite()) {
        return Err(LbmError::InvalidState);
    }
    let mut mass = Kahan::default();
    let mut px = Kahan::default();
    let mut py = Kahan::default();
    for q in 0..Q {
        mass.add(f[q]);
        px.add(f[q] * C[q][0] as f64);
        py.add(f[q] * C[q][1] as f64);
    }
    if mass.sum <= 0.0 {
        return Err(LbmError::InvalidState);
    }
    let u = [px.sum / mass.sum, py.sum / mass.sum];
    validate_macro(mass.sum, u)?;
    Ok((mass.sum, u))
}

#[derive(Default)]
struct Kahan {
    sum: f64,
    correction: f64,
}
impl Kahan {
    fn add(&mut self, x: f64) {
        let y = x - self.correction;
        let t = self.sum + y;
        self.correction = (t - self.sum) - y;
        self.sum = t;
    }
}

#[path = "multigrid.rs"]
mod multigrid;
pub use multigrid::*;
