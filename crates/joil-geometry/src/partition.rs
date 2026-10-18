//! One ownership authority. Fluid and solid are complementary consumers of it.
use crate::interval::{self, Cell, Interval, Sign};
use crate::{EvaluationBudget, FieldProgram, GeometryError, Point3, ReferenceInterpreter};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    Fluid,
    Solid,
    Void,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRole {
    Port,
    Wall,
    Keepout,
    FrozenFluid,
    FrozenSolid,
}
#[derive(Clone, Debug)]
pub struct FrozenRegion {
    pub id: String,
    pub role: SemanticRole,
    pub material: Material,
    pub region: FieldProgram,
}
#[derive(Clone, Debug)]
pub struct MaterialPartition {
    domain: FieldProgram,
    fluid: FieldProgram,
    frozen: Vec<FrozenRegion>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ownership {
    pub material: Material,
    pub semantic_tags: Vec<String>,
}
impl MaterialPartition {
    pub fn new(
        domain: FieldProgram,
        fluid: FieldProgram,
        frozen: Vec<FrozenRegion>,
    ) -> Result<Self, GeometryError> {
        if frozen.len() > 64 {
            return Err(GeometryError::InvalidInput("frozen region budget"));
        }
        let mut seen = BTreeSet::new();
        for r in &frozen {
            if joil_evidence::identity::check_id(&r.id).is_err() || !seen.insert(&r.id) {
                return Err(GeometryError::InvalidInput("frozen semantic identity"));
            }
            if matches!(r.role, SemanticRole::FrozenFluid | SemanticRole::Port)
                && r.material != Material::Fluid
                || matches!(r.role, SemanticRole::FrozenSolid | SemanticRole::Wall)
                    && r.material != Material::Solid
            {
                return Err(GeometryError::InvalidInput(
                    "semantic role/material mismatch",
                ));
            }
        }
        Ok(Self {
            domain,
            fluid,
            frozen,
        })
    }
    pub fn domain(&self) -> &FieldProgram {
        &self.domain
    }
    pub fn fluid(&self) -> &FieldProgram {
        &self.fluid
    }
    pub fn identity(&self) -> Result<String, GeometryError> {
        let frozen: Result<Vec<_>, GeometryError> = self
            .frozen
            .iter()
            .map(|r| Ok((&r.id, &r.role, r.material, r.region.identity()?)))
            .collect();
        joil_evidence::identity::scientific_hash(&(
            self.domain.identity()?,
            self.fluid.identity()?,
            frozen?,
            "closed-domain/fluid-wins-zero/1",
        ))
        .map_err(GeometryError::Backend)
    }
    pub fn ownership(
        &self,
        p: Point3,
        budget: &mut EvaluationBudget,
    ) -> Result<Ownership, GeometryError> {
        let value = |f: &FieldProgram, budget: &mut EvaluationBudget| {
            ReferenceInterpreter::compile(f.clone())?.evaluate_with_budget(p, budget)
        };
        let material = if value(&self.domain, budget)? > 0.0 {
            Material::Void
        } else if value(&self.fluid, budget)? <= 0.0 {
            Material::Fluid
        } else {
            Material::Solid
        };
        let mut tags = vec![];
        for region in &self.frozen {
            if value(&region.region, budget)? <= 0.0 {
                if material != region.material {
                    return Err(GeometryError::Unsupported(
                        "frozen material ownership violated",
                    ));
                }
                tags.push(region.id.clone());
            }
        }
        tags.sort();
        Ok(Ownership {
            material,
            semantic_tags: tags,
        })
    }
    /// Independently produced mesh/voxel labels must have exactly one owner and
    /// agree with the authority. Separate morphological filters cannot bypass it.
    pub fn verify_derived(
        &self,
        point: Point3,
        labels: [bool; 3],
        budget: &mut EvaluationBudget,
    ) -> Result<(), GeometryError> {
        if labels.iter().filter(|x| **x).count() != 1 {
            return Err(GeometryError::InvalidInput(
                "derived material overlap or gap",
            ));
        }
        let expected = match self.ownership(point, budget)?.material {
            Material::Fluid => 0,
            Material::Solid => 1,
            Material::Void => 2,
        };
        if !labels[expected] {
            return Err(GeometryError::Unsupported(
                "derived owner differs from shared partition",
            ));
        }
        Ok(())
    }
    pub fn classify_cell(
        &self,
        cell: Cell,
        budget: &mut EvaluationBudget,
    ) -> Result<Option<Material>, GeometryError> {
        let domain = interval::classify(interval::evaluate(&self.domain, cell, budget)?);
        let owner = if domain == Sign::Outside {
            Some(Material::Void)
        } else if domain != Sign::Inside {
            None
        } else {
            match interval::classify(interval::evaluate(&self.fluid, cell, budget)?) {
                Sign::Inside => Some(Material::Fluid),
                Sign::Outside => Some(Material::Solid),
                Sign::Unresolved => None,
            }
        };
        if let Some(material) = owner {
            for region in &self.frozen {
                if material != region.material {
                    match interval::classify(interval::evaluate(&region.region, cell, budget)?) {
                        Sign::Outside => {}
                        Sign::Inside => {
                            return Err(GeometryError::Unsupported(
                                "frozen material ownership violated over cell",
                            ))
                        }
                        Sign::Unresolved => return Ok(None),
                    }
                }
            }
        }
        Ok(owner)
    }
}
#[derive(Clone, Debug)]
pub struct VolumeBound {
    /// Volume clipped to the supplied spatial cell, never an unbounded whole shape.
    pub fluid_m3: Interval,
    pub ambiguous_volume_m3: Interval,
    pub visited_cells: usize,
    pub ambiguous_cells: usize,
    pub requested_error_met: bool,
}
pub fn bound_fluid_volume(
    partition: &MaterialPartition,
    region: Cell,
    max_cells: usize,
    max_ambiguous_volume_m3: f64,
    budget: &mut EvaluationBudget,
) -> Result<VolumeBound, GeometryError> {
    if max_cells == 0
        || max_cells > 1_000_000
        || !max_ambiguous_volume_m3.is_finite()
        || max_ambiguous_volume_m3 < 0.0
    {
        return Err(GeometryError::InvalidInput("volume policy/cell budget"));
    }
    let mut pending = vec![region];
    let mut definite = Interval::point(0.0)?;
    let mut uncertain = Interval::point(0.0)?;
    let mut visited = 0;
    let mut ambiguous = 0;
    while let Some(cell) = pending.pop() {
        if visited == max_cells {
            uncertain = uncertain.plus(cell.volume_bounds()?)?;
            ambiguous += 1;
            for c in pending.drain(..) {
                uncertain = uncertain.plus(c.volume_bounds()?)?;
                ambiguous += 1;
            }
            break;
        }
        visited += 1;
        match partition.classify_cell(cell, budget)? {
            Some(Material::Fluid) => definite = definite.plus(cell.volume_bounds()?)?,
            Some(_) => {}
            None => {
                let volume = cell.volume_bounds()?;
                if volume.hi <= max_ambiguous_volume_m3 / (max_cells as f64) {
                    uncertain = uncertain.plus(volume)?;
                    ambiguous += 1;
                } else {
                    match cell.split() {
                        Ok(children) => pending.extend(children),
                        Err(_) => {
                            uncertain = uncertain.plus(volume)?;
                            ambiguous += 1;
                        }
                    }
                }
            }
        }
    }
    let fluid_m3 = Interval::new(definite.lo.max(0.0), definite.plus(uncertain)?.hi)?;
    Ok(VolumeBound {
        fluid_m3,
        ambiguous_volume_m3: uncertain,
        visited_cells: visited,
        ambiguous_cells: ambiguous,
        requested_error_met: uncertain.hi <= max_ambiguous_volume_m3,
    })
}
