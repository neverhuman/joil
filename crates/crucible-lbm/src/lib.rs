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
}
