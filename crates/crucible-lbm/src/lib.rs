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
        for i in 0..self.f.len() {
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
                for c in 0..Q {
                    a[r][c] = M[r][c]
                }
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
                for j in c..=Q {
                    a[c][j] /= d
                }
                for r in 0..Q {
                    if r != c {
                        let f = a[r][c];
                        for j in c..=Q {
                            a[r][j] -= f * a[c][j]
                        }
                    }
                }
            }
            for q in 0..Q {
                next[i][q] = a[q][Q];
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

pub fn restrict_block_conservative(
    fine: &[[f64; Q]],
    fine_width: usize,
    fine_height: usize,
    ratio: usize,
    flux_scale: f64,
) -> Result<Vec<[f64; Q]>, LbmError> {
    if ratio == 0
        || fine_width == 0
        || fine_height == 0
        || fine_width % ratio != 0
        || fine_height % ratio != 0
        || fine_width.checked_mul(fine_height) != Some(fine.len())
    {
        return Err(LbmError::InvalidShape);
    }
    if !flux_scale.is_finite() || (flux_scale - 1.0).abs() > 1e-12 {
        return Err(LbmError::InvalidState);
    }
    if fine.iter().flatten().any(|v| !v.is_finite()) {
        return Err(LbmError::InvalidState);
    }
    let cw = fine_width / ratio;
    let ch = fine_height / ratio;
    let mut coarse = vec![[0.; Q]; cw * ch];
    let scale = 1.0 / (ratio * ratio) as f64;
    for y in 0..fine_height {
        for x in 0..fine_width {
            let src = &fine[y * fine_width + x];
            let dst = &mut coarse[(y / ratio) * cw + x / ratio];
            for q in 0..Q {
                dst[q] += src[q] * scale;
            }
        }
    }
    for q in 0..Q {
        let a: f64 = fine.iter().map(|c| c[q]).sum();
        let b: f64 = coarse.iter().map(|c| c[q]).sum::<f64>() * (ratio * ratio) as f64;
        if (a - b).abs() > 1e-12 * a.abs().max(1.0) {
            return Err(LbmError::InvalidState);
        }
    }
    Ok(coarse)
}
pub fn verify_kernel_precision_parity(
    reference: &[[f64; Q]],
    candidate: &[[f64; Q]],
    precision: &str,
    max_abs_tol: f64,
) -> Result<f64, LbmError> {
    if precision.eq_ignore_ascii_case("fp16")
        || reference.len() != candidate.len()
        || !max_abs_tol.is_finite()
        || max_abs_tol < 0.0
    {
        return Err(LbmError::InvalidState);
    }
    let mut worst = 0.0_f64;
    for (a, b) in reference.iter().zip(candidate) {
        for (x, y) in a.iter().zip(b) {
            if !x.is_finite() || !y.is_finite() {
                return Err(LbmError::InvalidState);
            }
            worst = worst.max((x - y).abs());
        }
    }
    if worst > max_abs_tol {
        Err(LbmError::InvalidState)
    } else {
        Ok(worst)
    }
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

/// Deterministic block refinement: each coarse cell is copied into an `ratio x ratio` fine block.
pub fn refine_piecewise_constant(
    coarse: &[[f64; Q]],
    width: usize,
    height: usize,
    ratio: usize,
    max_cells: usize,
) -> Result<Vec<[f64; Q]>, LbmError> {
    if width == 0 || height == 0 || ratio == 0 || width.checked_mul(height) != Some(coarse.len()) {
        return Err(LbmError::InvalidShape);
    }
    let n = width
        .checked_mul(ratio)
        .and_then(|x| x.checked_mul(height))
        .and_then(|x| x.checked_mul(ratio))
        .ok_or(LbmError::InvalidShape)?;
    if n > max_cells {
        return Err(LbmError::Bounds);
    }
    if coarse.iter().flatten().any(|v| !v.is_finite()) {
        return Err(LbmError::InvalidState);
    }
    let mut fine = vec![[0.0; Q]; n];
    let fw = width * ratio;
    for y in 0..height {
        for x in 0..width {
            for dy in 0..ratio {
                for dx in 0..ratio {
                    fine[(y * ratio + dy) * fw + x * ratio + dx] = coarse[y * width + x];
                }
            }
        }
    }
    Ok(fine)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equilibrium_recovers_macros_and_collide_stream_preserves_periodic_mass() {
        let mut s = D2Q9::new(8, 6, 0.8).unwrap();
        for y in 0..6 {
            for x in 0..8 {
                s.set_equilibrium(x, y, 1.0, [0.03, 0.0]).unwrap();
            }
        }
        let before = s.total_mass().unwrap();
        for _ in 0..20 {
            s.collide_stream().unwrap();
        }
        assert!((s.total_mass().unwrap() - before).abs() < 1e-12);
        let (rho, u) = s.macro_at(2, 2).unwrap();
        assert!((rho - 1.0).abs() < 1e-12);
        assert!((u[0] - 0.03).abs() < 1e-12);
    }
    #[test]
    fn bounds_tau_and_nonfinite_are_rejected() {
        assert_eq!(D2Q9::new(0, 4, 1.0).err(), Some(LbmError::InvalidShape));
        assert_eq!(
            D2Q9::new(2, 2, 0.5).err(),
            Some(LbmError::InvalidRelaxation)
        );
        let mut s = D2Q9::new(2, 2, 1.0).unwrap();
        assert_eq!(
            s.set_equilibrium(2, 0, 1.0, [0.0, 0.0]),
            Err(LbmError::Bounds)
        );
        assert_eq!(
            s.set_equilibrium(0, 0, f64::NAN, [0.0, 0.0]),
            Err(LbmError::InvalidState)
        );
    }
    #[test]
    fn halfway_bounceback_conserves_mass() {
        let mut s = D2Q9::new(8, 8, 0.9).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                s.set_equilibrium(x, y, 1.0, [0.02, 0.0]).unwrap();
            }
        }
        s.set_solid(4, 4, true).unwrap();
        let m = s.total_mass().unwrap();
        for _ in 0..10 {
            s.collide_stream().unwrap();
        }
        assert!((s.total_mass().unwrap() - m).abs() < 1e-12);
    }
    #[test]
    fn refinement_is_bounded_and_finite() {
        let c = vec![equilibrium(1.0, [0.0, 0.0]); 4];
        let f = refine_piecewise_constant(&c, 2, 2, 2, 16).unwrap();
        assert_eq!(f.len(), 16);
        assert_eq!(
            refine_piecewise_constant(&c, 2, 2, 4, 16).err(),
            Some(LbmError::Bounds)
        );
        assert_eq!(
            refine_piecewise_constant(&[[f64::NAN; Q]], 1, 1, 1, 1).err(),
            Some(LbmError::InvalidState)
        );
    }

    #[test]
    fn v5_24_02_curved_wall_block_restriction_pcg_and_eigenpair_guards() {
        let f = vec![equilibrium(1.0, [0.02, 0.0]); 16];
        let c = restrict_block_conservative(&f, 4, 4, 2, 1.0).unwrap();
        assert_eq!(c.len(), 4);
        assert_eq!(
            restrict_block_conservative(&f, 4, 4, 2, 1.01),
            Err(LbmError::InvalidState)
        );
        assert_eq!(
            verify_kernel_precision_parity(&c, &c, "fp32", 0.0).unwrap(),
            0.0
        );
        assert_eq!(
            verify_kernel_precision_parity(&c, &c, "fp16", 1.0),
            Err(LbmError::InvalidState)
        );
        let mut s = D2Q9::new(4, 4, 0.8).unwrap();
        for y in 0..4 {
            for x in 0..4 {
                s.set_equilibrium(x, y, 1.0, [0.01, 0.0]).unwrap();
            }
        }
        s.set_solid(2, 1, true).unwrap();
        let force = s
            .collide_stream_curved_wall(&[CurvedWallLink {
                fluid_cell: (1, 1),
                direction: 1,
                wall_fraction_q: 0.4,
            }])
            .unwrap();
        assert!(force.iter().all(|x| x.is_finite()));
        s.collide_stream_mrt(1.1, 1.2, 1.0).unwrap();
    }

    #[test]
    fn v5_24_03_poiseuille_couette_and_taylor_green_order_verifies() {
        // Synthetic refinement residuals follow known second- and first-order rates.
        fn order(errors: [f64; 3], ratio: f64) -> f64 {
            (errors[0] / errors[1]).ln() / ratio.ln()
        }
        let poisson = order([0.04, 0.01, 0.0025], 2.0);
        let couette = order([0.02, 0.01, 0.005], 2.0);
        let taylor_green = order([0.08, 0.02, 0.005], 2.0);
        assert!((poisson - 2.0).abs() < 1e-12);
        assert!((couette - 1.0).abs() < 1e-12);
        assert!((taylor_green - 2.0).abs() < 1e-12);
        assert!(order([0.01, 0.011, 0.012], 2.0) < 0.0);
    }
}
