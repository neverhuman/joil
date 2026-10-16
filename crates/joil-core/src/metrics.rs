//! Pure arithmetic on supplied observations, not a trusted evidence importer.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum QuantileEstimate {
    Estimated {
        n: usize,
        minimum_n: usize,
        quantile: f64,
        confidence: f64,
        value: f64,
        ci_low: f64,
        ci_high: f64,
        method: String,
    },
    Insufficient {
        n: usize,
        minimum_n: usize,
        quantile: f64,
        confidence: f64,
        reason: String,
    },
}

impl QuantileEstimate {
    pub fn value(&self) -> Option<f64> {
        match self {
            Self::Estimated { value, .. } => Some(*value),
            Self::Insufficient { .. } => None,
        }
    }
}

/// Distribution-free order-statistic interval, conditional on IID samples.
/// A frozen synthetic sampling distribution does not establish physical risk.
pub fn quantile_estimate(
    samples: &[f64],
    quantile: f64,
    confidence: f64,
    minimum_n: usize,
) -> Result<QuantileEstimate, String> {
    if samples.iter().any(|v| !v.is_finite())
        || !quantile.is_finite()
        || !(0.0..1.0).contains(&quantile)
        || quantile == 0.0
        || !confidence.is_finite()
        || !(0.0..1.0).contains(&confidence)
        || confidence == 0.0
    {
        return Err("finite samples and quantile/confidence strictly in (0,1) required".into());
    }
    let alpha_half = (1.0 - confidence) * 0.5;
    let required_n = (alpha_half.ln() / quantile.max(1.0 - quantile).ln()).ceil() as usize;
    let minimum_n = minimum_n.max(required_n);
    let n = samples.len();
    let insufficient = |reason: &str| QuantileEstimate::Insufficient {
        n,
        minimum_n,
        quantile,
        confidence,
        reason: reason.into(),
    };
    if n < minimum_n {
        return Ok(insufficient(
            "too_few_samples_for_two_finite_order_statistic_bounds",
        ));
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    // Accumulate binomial masses in log space: (1-q)^n underflows for high
    // quantiles, although masses near the mode remain perfectly resolved.
    let mut log_mass = n as f64 * (1.0 - quantile).ln();
    let mut masses = Vec::with_capacity(n + 1);
    for k in 0..=n {
        masses.push(log_mass.exp());
        if k < n {
            log_mass += ((n - k) as f64).ln() - ((k + 1) as f64).ln() + quantile.ln()
                - (1.0 - quantile).ln();
        }
    }
    let total: f64 = masses.iter().sum();
    if !total.is_finite() || total <= 0.0 {
        return Err("binomial interval calculation failed".into());
    }
    let mut cumulative = 0.0;
    let mut lower_rank = None;
    let mut upper_rank = None;
    for rank in 1..=n {
        cumulative += masses[rank - 1] / total;
        if cumulative <= alpha_half {
            lower_rank = Some(rank);
        }
        if cumulative >= 1.0 - alpha_half && upper_rank.is_none() {
            upper_rank = Some(rank);
        }
    }
    let (Some(lower), Some(upper)) = (lower_rank, upper_rank) else {
        return Ok(insufficient("finite_order_statistic_interval_not_resolved"));
    };
    let position = quantile * (n - 1) as f64;
    let fraction = position.fract();
    let value = (1.0 - fraction) * sorted[position.floor() as usize]
        + fraction * sorted[position.ceil() as usize];
    if !value.is_finite() {
        return Err("quantile interpolation overflow".into());
    }
    Ok(QuantileEstimate::Estimated {
        n,
        minimum_n,
        quantile,
        confidence,
        value,
        ci_low: sorted[lower - 1],
        ci_high: sorted[upper - 1],
        method: "binomial_order_statistic_interval_assuming_iid".into(),
    })
}

/// Wilson 95% sampling interval. This is not an empirical physical probability.
pub fn sampled_fraction_ci95(passes: usize, n: usize) -> Result<[f64; 2], String> {
    if n == 0 || passes > n {
        return Err("positive n and passes <= n required".into());
    }
    let z = 1.959_963_984_540_054_f64;
    let n = n as f64;
    let p = passes as f64 / n;
    let denominator = 1.0 + z * z / n;
    let center = (p + z * z / (2.0 * n)) / denominator;
    let radius = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt() / denominator;
    Ok([(center - radius).max(0.0), (center + radius).min(1.0)])
}

/// Exact dominated union of axis-aligned boxes for minimization objectives.
/// Points outside the reference box contribute no volume. No archive-size or
/// diversity bonus is included; dimensions and reference values are explicit.
pub fn pareto_hypervolume(points: &[Vec<f64>], reference: &[f64]) -> Result<f64, String> {
    if reference.is_empty()
        || reference.iter().any(|v| !v.is_finite())
        || points
            .iter()
            .any(|p| p.len() != reference.len() || p.iter().any(|v| !v.is_finite()))
    {
        return Err("finite equal-dimensional points and nonempty reference required".into());
    }
    fn union(mut points: Vec<Vec<f64>>, reference: &[f64]) -> f64 {
        if points.is_empty() {
            return 0.0;
        }
        let dimension = reference.len();
        if dimension == 1 {
            return reference[0] - points.iter().map(|p| p[0]).fold(reference[0], f64::min);
        }
        points.sort_by(|a, b| {
            a.iter()
                .zip(b)
                .find_map(|(a, b)| (a != b).then(|| a.total_cmp(b)))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        points.dedup();
        if dimension == 2 {
            let mut lowest_y = reference[1];
            let mut area = 0.0;
            for point in points {
                if point[1] < lowest_y {
                    area += (reference[0] - point[0]) * (lowest_y - point[1]);
                    lowest_y = point[1];
                }
            }
            return area;
        }
        // Projected dominance differs at every sweep level. Pruning only once
        // leaves exponentially many redundant lower-dimensional boxes.
        let points: Vec<Vec<f64>> = points
            .iter()
            .enumerate()
            .filter(|(index, point)| {
                !points.iter().enumerate().any(|(other_index, other)| {
                    other_index != *index && other.iter().zip(point.iter()).all(|(a, b)| a <= b)
                })
            })
            .map(|(_, p)| p.clone())
            .collect();
        let axis = dimension - 1;
        let mut coordinates: Vec<f64> = points.iter().map(|p| p[axis]).collect();
        coordinates.push(reference[axis]);
        coordinates.sort_by(f64::total_cmp);
        coordinates.dedup();
        coordinates
            .windows(2)
            .map(|slab| {
                let active = points
                    .iter()
                    .filter(|p| p[axis] <= slab[0])
                    .map(|p| p[..axis].to_vec())
                    .collect();
                (slab[1] - slab[0]) * union(active, &reference[..axis])
            })
            .sum()
    }
    let eligible = points
        .iter()
        .filter(|p| p.iter().zip(reference).all(|(v, r)| v < r))
        .cloned()
        .collect();
    let volume = union(eligible, reference);
    if volume.is_finite() {
        Ok(volume)
    } else {
        Err("hypervolume overflow".into())
    }
}

fn split(r: f64) -> Result<(), String> {
    if !r.is_finite() || !(0.0..1.0).contains(&r) {
        Err("split must be finite in [0,1)".into())
    } else {
        Ok(())
    }
}
fn fraction(x: f64) -> bool {
    x.is_finite() && (0.0..=1.0).contains(&x)
}
pub fn concentration_mg_l(cin: f64, capture: f64, r: f64) -> Result<f64, String> {
    split(r)?;
    if !cin.is_finite() || cin < 0.0 || !fraction(capture) {
        return Err("invalid concentration or capture".into());
    }
    let out = cin * (1.0 - capture) / (1.0 - r);
    if !out.is_finite() {
        return Err("concentration overflow".into());
    }
    Ok(out)
}
pub fn reduced_efficiency(capture: f64, r: f64) -> Result<f64, String> {
    split(r)?;
    if !fraction(capture) {
        return Err("invalid capture".into());
    }
    Ok((capture - r) / (1.0 - r))
}
pub fn weighted_capture(weights: &[f64], efficiencies: &[f64]) -> Result<f64, String> {
    if weights.is_empty()
        || weights.len() != efficiencies.len()
        || weights.iter().any(|x| !x.is_finite() || *x < 0.0)
        || efficiencies.iter().any(|x| !fraction(*x))
        || (weights.iter().sum::<f64>() - 1.0).abs() > 1e-10
    {
        return Err("normalized oil-mass weights and finite capture bins required".into());
    }
    Ok(weights.iter().zip(efficiencies).map(|(w, e)| w * e).sum())
}
pub fn pdr(pin: f64, po: f64, pu: f64) -> Result<f64, String> {
    if [pin, po, pu].iter().any(|x| !x.is_finite()) || pin <= pu || pin < po {
        return Err("invalid pressure ordering or units".into());
    }
    let out = (pin - po) / (pin - pu);
    if out.is_finite() {
        Ok(out)
    } else {
        Err("PDR overflow".into())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OilBalance {
    pub injected_kg: f64,
    pub reject_kg: f64,
    pub treated_kg: f64,
    pub remaining_kg: f64,
    pub wall_kg: f64,
}
impl OilBalance {
    pub fn residual_fraction(&self) -> Result<f64, String> {
        let xs = [
            self.injected_kg,
            self.reject_kg,
            self.treated_kg,
            self.remaining_kg,
            self.wall_kg,
        ];
        if xs.iter().any(|x| !x.is_finite() || *x < 0.0) || self.injected_kg <= 0.0 {
            return Err("finite nonnegative inventory and positive injected mass required".into());
        }
        let total = self.reject_kg + self.treated_kg + self.remaining_kg + self.wall_kg;
        if !total.is_finite() {
            return Err("inventory overflow".into());
        }
        Ok((self.injected_kg - total).abs() / self.injected_kg)
    }
    pub fn capture_bounds(&self, max_closure: f64) -> Result<(f64, f64), String> {
        if !fraction(max_closure) || self.residual_fraction()? > max_closure {
            return Err("oil inventory does not close to requested tolerance".into());
        }
        let captured = self.reject_kg / self.injected_kg;
        let error = self.residual_fraction()?;
        // Include numerical discrepancy adversarially; wall hits never count as known capture.
        let hi = captured + (self.remaining_kg + self.wall_kg) / self.injected_kg + error;
        if captured > 1.0 {
            return Err("reject mass exceeds injected mass".into());
        }
        Ok(((captured - error).max(0.0), hi.min(1.0)))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Sample {
    Observed {
        scenario: String,
        cout_upper_mg_l: f64,
        dp_upper_pa: f64,
        energy_upper_j_per_treated_m3: f64,
        reject_fraction: f64,
    },
    Unknown {
        scenario: String,
        reason: String,
    },
}
impl Sample {
    fn scenario(&self) -> &str {
        match self {
            Self::Observed { scenario, .. } | Self::Unknown { scenario, .. } => scenario,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatingPoint {
    pub feed_m3_h: f64,
    pub samples: Vec<Sample>,
}
#[derive(Clone, Debug)]
pub struct Limits {
    pub cout_mg_l: f64,
    pub dp_pa: f64,
    pub energy_j_per_treated_m3: f64,
    pub reject_min: f64,
    pub reject_max: f64,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SampledCapacity {
    pub feed_m3_h: f64,
    pub treated_m3_h: f64,
}

/// Common *tested* point only. Does not certify physics, evidence, or an intervening band.
pub fn common_tested_capacity(
    points: &[OperatingPoint],
    required: &[String],
    l: &Limits,
) -> Result<Option<SampledCapacity>, String> {
    let names: BTreeSet<_> = required.iter().map(String::as_str).collect();
    if names.is_empty()
        || names.len() != required.len()
        || names.contains("")
        || [l.cout_mg_l, l.dp_pa, l.energy_j_per_treated_m3]
            .iter()
            .any(|x| !x.is_finite() || *x < 0.0)
        || !fraction(l.reject_min)
        || l.reject_max >= 1.0
        || !fraction(l.reject_max)
        || l.reject_min > l.reject_max
    {
        return Err("invalid duty or duplicate scenarios".into());
    }
    let mut seen_flows = Vec::new();
    let mut best: Option<SampledCapacity> = None;
    for p in points {
        if !p.feed_m3_h.is_finite() || p.feed_m3_h <= 0.0 || seen_flows.contains(&p.feed_m3_h) {
            return Err("finite distinct positive flow points required".into());
        }
        seen_flows.push(p.feed_m3_h);
        let mut seen = BTreeSet::new();
        let mut passes = true;
        let mut worst_reject: f64 = 0.0;
        for s in &p.samples {
            if !seen.insert(s.scenario()) || !names.contains(s.scenario()) {
                return Err("duplicate or unexpected scenario".into());
            }
            match s {
                Sample::Unknown { .. } => passes = false,
                Sample::Observed {
                    cout_upper_mg_l: c,
                    dp_upper_pa: d,
                    energy_upper_j_per_treated_m3: e,
                    reject_fraction: r,
                    ..
                } => {
                    split(*r)?;
                    if [c, d, e].iter().any(|x| !x.is_finite() || **x < 0.0) {
                        return Err("invalid observed metric".into());
                    }
                    passes &= *c <= l.cout_mg_l
                        && *d <= l.dp_pa
                        && *e <= l.energy_j_per_treated_m3
                        && *r >= l.reject_min
                        && *r <= l.reject_max;
                    worst_reject = worst_reject.max(*r);
                }
            }
        }
        passes &= seen == names;
        if passes && best.as_ref().is_none_or(|b| p.feed_m3_h > b.feed_m3_h) {
            best = Some(SampledCapacity {
                feed_m3_h: p.feed_m3_h,
                treated_m3_h: p.feed_m3_h * (1.0 - worst_reject),
            });
        }
    }
    Ok(best)
}
pub fn gain(candidate: f64, baseline: f64) -> Result<f64, String> {
    if !candidate.is_finite() || candidate < 0.0 || !baseline.is_finite() || baseline <= 0.0 {
        return Err("gain undefined without a positive feasible baseline".into());
    }
    let out = candidate / baseline - 1.0;
    if out.is_finite() {
        Ok(out)
    } else {
        Err("gain overflow".into())
    }
}
