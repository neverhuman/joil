//! Computed conformance of a finite cubical artifact. This does not certify a
//! continuous CAD surface, an unobserved thin feature, or physical performance.
use crate::interval::Interval;
use crate::partition::{Material, MaterialPartition};
use crate::{EvaluationBudget, GeometryError, Point3};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, Serialize)]
pub struct CubicalSubject {
    dimensions: [usize; 3],
    origin_m: [f64; 3],
    pitch_m: f64,
    labels: Vec<Material>,
    /// Frozen interfaces and semantic feature witnesses come from the caller's
    /// reviewed contract. They cannot be inferred from a module name.
    protected: BTreeMap<String, BTreeSet<usize>>,
    source_partition_sha256: String,
}
impl CubicalSubject {
    pub fn new(
        dimensions: [usize; 3],
        origin_m: [f64; 3],
        pitch_m: f64,
        labels: Vec<Material>,
        protected: BTreeMap<String, BTreeSet<usize>>,
        partition: &MaterialPartition,
        budget: &mut EvaluationBudget,
    ) -> Result<Self, GeometryError> {
        let cells = dimensions
            .iter()
            .try_fold(1usize, |n, &d| n.checked_mul(d))
            .ok_or(GeometryError::InvalidInput("cubical size overflow"))?;
        if cells == 0
            || cells > 4096
            || dimensions.contains(&0)
            || labels.len() != cells
            || !pitch_m.is_finite()
            || pitch_m <= 0.0
            || origin_m.iter().any(|x| !x.is_finite())
        {
            return Err(GeometryError::InvalidInput(
                "cubical dimensions/origin/pitch/labels",
            ));
        }
        for (id, indices) in &protected {
            if joil_evidence::identity::check_id(id).is_err()
                || indices.is_empty()
                || indices.iter().any(|i| *i >= cells)
            {
                return Err(GeometryError::InvalidInput("protected feature witness"));
            }
        }
        let result = Self {
            dimensions,
            origin_m,
            pitch_m,
            labels,
            protected,
            source_partition_sha256: partition.identity()?,
        };
        for (i, label) in result.labels.iter().enumerate() {
            if partition.ownership(result.centre(i)?, budget)?.material != *label {
                return Err(GeometryError::InvalidInput(
                    "artifact labels differ from source partition",
                ));
            }
        }
        Ok(result)
    }
    pub fn identity(&self) -> Result<String, GeometryError> {
        joil_evidence::identity::scientific_hash(self).map_err(GeometryError::Backend)
    }
    pub fn labels(&self) -> &[Material] {
        &self.labels
    }
    fn coordinates(&self, index: usize) -> [usize; 3] {
        let [nx, ny, _] = self.dimensions;
        [index % nx, index / nx % ny, index / (nx * ny)]
    }
    fn centre(&self, index: usize) -> Result<Point3, GeometryError> {
        let c = self.coordinates(index);
        Point3::metres(std::array::from_fn(|i| {
            self.origin_m[i] + (c[i] as f64 + 0.5) * self.pitch_m
        }))
    }
    fn neighbours(&self, index: usize) -> Vec<usize> {
        let c = self.coordinates(index);
        let mut out = vec![];
        let [nx, ny, _] = self.dimensions;
        for axis in 0..3 {
            for delta in [-1isize, 1] {
                if let Some(v) = c[axis].checked_add_signed(delta) {
                    if v < self.dimensions[axis] {
                        let mut p = c;
                        p[axis] = v;
                        out.push(p[0] + nx * (p[1] + ny * p[2]));
                    }
                }
            }
        }
        out
    }
    fn components(&self, material: Material) -> Vec<usize> {
        let mut ids = vec![usize::MAX; self.labels.len()];
        let mut next = 0;
        for seed in 0..ids.len() {
            if self.labels[seed] != material || ids[seed] != usize::MAX {
                continue;
            }
            let mut queue = VecDeque::from([seed]);
            ids[seed] = next;
            while let Some(i) = queue.pop_front() {
                for j in self.neighbours(i) {
                    if self.labels[j] == material && ids[j] == usize::MAX {
                        ids[j] = next;
                        queue.push_back(j);
                    }
                }
            }
            next += 1;
        }
        ids
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct ConformancePolicy {
    pub id: String,
    pub maximum_displacement_m: f64,
    pub maximum_changed_volume_m3: f64,
    /// Six face-neighbours; five agreeing neighbours required. One synchronous
    /// pass only, never repeated until a proposal happens to become acceptable.
    pub required_neighbours: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct ConformanceReport {
    pub original_subject_sha256: String,
    pub resulting_subject_sha256: String,
    pub policy_sha256: String,
    pub changed_cells: usize,
    pub changed_volume_m3: [f64; 2],
    pub displacement_upper_bound_m: f64,
    pub scope: &'static str,
}
fn preserve_components(
    before: &CubicalSubject,
    after: &CubicalSubject,
) -> Result<(), GeometryError> {
    for material in [Material::Fluid, Material::Solid, Material::Void] {
        let a = before.components(material);
        let b = after.components(material);
        let mut forward: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        let mut reverse: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for i in 0..a.len() {
            if a[i] != usize::MAX {
                forward.entry(a[i]).or_default();
            }
            if b[i] != usize::MAX {
                reverse.entry(b[i]).or_default();
            }
            if a[i] != usize::MAX && b[i] != usize::MAX {
                forward.entry(a[i]).or_default().insert(b[i]);
                reverse.entry(b[i]).or_default().insert(a[i]);
            }
        }
        if forward
            .values()
            .chain(reverse.values())
            .any(|v| v.len() != 1)
        {
            return Err(GeometryError::Unsupported(
                "conformance deletes/splits/merges a material component",
            ));
        }
    }
    Ok(())
}
/// A bounded, explicit artifact filter. The source partition is checked during
/// artifact construction; every actual relabel is audited here before return.
pub fn conform(
    subject: &CubicalSubject,
    policy: &ConformancePolicy,
    budget: &mut EvaluationBudget,
) -> Result<(CubicalSubject, ConformanceReport), GeometryError> {
    if joil_evidence::identity::check_id(&policy.id).is_err()
        || policy.required_neighbours != 5
        || ![
            policy.maximum_displacement_m,
            policy.maximum_changed_volume_m3,
        ]
        .iter()
        .all(|x| x.is_finite() && *x >= 0.0)
    {
        return Err(GeometryError::InvalidInput(
            "unsupported or invalid conformance policy",
        ));
    }
    budget.charge(
        subject
            .labels
            .len()
            .checked_mul(6)
            .ok_or(GeometryError::Unsupported("filter budget overflow"))?,
    )?;
    let mut after = subject.clone();
    let mut changes = vec![];
    for i in 0..subject.labels.len() {
        let ns = subject.neighbours(i);
        if ns.len() != 6 {
            continue;
        }
        for label in [Material::Fluid, Material::Solid, Material::Void] {
            if label != subject.labels[i]
                && ns.iter().filter(|j| subject.labels[**j] == label).count()
                    >= policy.required_neighbours
            {
                after.labels[i] = label;
                changes.push(i);
                break;
            }
        }
    }
    if subject.protected.values().any(|indices| {
        indices
            .iter()
            .any(|i| subject.labels[*i] != after.labels[*i])
    }) {
        return Err(GeometryError::Unsupported(
            "conformance changes frozen interface or semantic feature witness",
        ));
    }
    preserve_components(subject, &after)?;
    let mut displacement = 0.0f64;
    for (from, to) in [(subject, &after), (&after, subject)] {
        for &i in &changes {
            budget.charge(to.labels.len())?;
            let p = from.coordinates(i);
            let mut nearest_squared_steps = u64::MAX;
            for (j, label) in to.labels.iter().enumerate() {
                if *label == from.labels[i] {
                    let q = to.coordinates(j);
                    let squared = (0..3).map(|k| (p[k].abs_diff(q[k]) as u64).pow(2)).sum();
                    nearest_squared_steps = nearest_squared_steps.min(squared);
                }
            }
            // Every point in the removed/added voxel is within half a cell
            // diagonal of its centre; the target voxel centre belongs to target.
            if nearest_squared_steps == u64::MAX {
                return Err(GeometryError::Unsupported("conformance removes a material"));
            }
            let pitch = Interval::point(subject.pitch_m)?;
            let centre_distance = Interval::point(nearest_squared_steps as f64)?
                .sqrt()?
                .times(pitch)?;
            let half_diagonal = Interval::point(3.0)?
                .sqrt()?
                .times(pitch)?
                .times(Interval::point(0.5)?)?;
            displacement = displacement.max(centre_distance.plus(half_diagonal)?.hi);
        }
    }
    let pitch = Interval::point(subject.pitch_m)?;
    let volume = if changes.is_empty() {
        Interval::point(0.0)?
    } else {
        pitch
            .times(pitch)?
            .times(pitch)?
            .times(Interval::point(changes.len() as f64)?)?
    };
    if displacement > policy.maximum_displacement_m || volume.hi > policy.maximum_changed_volume_m3
    {
        return Err(GeometryError::Unsupported(
            "computed conformance change exceeds policy",
        ));
    }
    let report=ConformanceReport{original_subject_sha256:subject.identity()?,resulting_subject_sha256:after.identity()?,policy_sha256:joil_evidence::identity::scientific_hash(policy).map_err(GeometryError::Backend)?,changed_cells:changes.len(),changed_volume_m3:[volume.lo.max(0.0),volume.hi],displacement_upper_bound_m:displacement,scope:"Exact relabel/volume of finite cubical artifact; displacement is conservative. Centre samples do not certify source CAD equivalence or unobserved continuous features."};
    Ok((after, report))
}
