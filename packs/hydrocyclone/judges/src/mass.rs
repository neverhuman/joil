//! V5-13-02: mass from the accepted solid, never from an analytic estimate.
//!
//! Mass is density times the volume of the *same* accepted solid that CFD, FEA and the print
//! use. Nominal printed, support/allowance, machined-removed and weighed mass are separate
//! quantities. Legacy D-07 (`phenotype.rs` credited an annulus volume times a clamped solid
//! fraction to a wall that the mesh only corrugates) is refused: a claimed solid volume must
//! name the accepted subject and agree with its integrated volume.
use crate::material::Bounds;
use crate::{positive, Field, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub enum MassError {
    Invalid(String),
    Unresolved(Vec<String>),
    SubjectMismatch,
    MeshDisagrees {
        solid_volume_m3: f64,
        mesh_volume_m3: f64,
        tolerance_relative: f64,
    },
    UnsupportedCredit(String),
}

fn invalid<T>(message: impl Into<String>) -> Result<T, MassError> {
    Err(MassError::Invalid(message.into()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureKind {
    Wall,
    InternalSolid,
    Rib,
}

/// One resolved solid feature of the accepted partition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contribution {
    pub feature_id: String,
    pub kind: FeatureKind,
    pub subject_sha256: Sha256Hex,
    pub volume_m3: f64,
}

/// The accepted solid: integrated volume, its relative bound, an independent tetrahedral
/// volume-mesh volume of the same subject, and the resolved features that partition it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedSolid {
    pub subject_sha256: Sha256Hex,
    pub solid_volume_m3: f64,
    pub volume_relative_bound: f64,
    pub tetra_mesh_volume_m3: f64,
    pub mesh_relative_tolerance: f64,
    pub contributions: Vec<Contribution>,
}

impl AcceptedSolid {
    pub fn check(&self) -> Result<(), MassError> {
        positive("solid volume", self.solid_volume_m3).or_else(|e| invalid(e.to_string()))?;
        positive("tetrahedral mesh volume", self.tetra_mesh_volume_m3)
            .or_else(|e| invalid(e.to_string()))?;
        for (name, v) in [
            ("volume bound", self.volume_relative_bound),
            ("mesh tolerance", self.mesh_relative_tolerance),
        ] {
            if !(0.0..=0.5).contains(&v) {
                return invalid(format!("{name} must be in [0, 0.5]"));
            }
        }
        if self.contributions.is_empty() {
            return invalid("accepted solid lists no resolved features");
        }
        let mut ids = BTreeSet::new();
        let mut total = 0.0;
        for c in &self.contributions {
            if c.feature_id.trim().is_empty() || !ids.insert(c.feature_id.as_str()) {
                return invalid("empty or duplicate feature id");
            }
            if c.subject_sha256 != self.subject_sha256 {
                return Err(MassError::SubjectMismatch);
            }
            positive("feature volume", c.volume_m3).or_else(|e| invalid(e.to_string()))?;
            total += c.volume_m3;
        }
        if (total - self.solid_volume_m3).abs() > self.volume_relative_bound * self.solid_volume_m3
        {
            return invalid(
                "resolved features do not partition the accepted solid within its bound",
            );
        }
        let gap = (self.solid_volume_m3 - self.tetra_mesh_volume_m3).abs();
        if gap > self.mesh_relative_tolerance * self.solid_volume_m3 {
            return Err(MassError::MeshDisagrees {
                solid_volume_m3: self.solid_volume_m3,
                mesh_volume_m3: self.tetra_mesh_volume_m3,
                tolerance_relative: self.mesh_relative_tolerance,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

impl Interval {
    pub fn contains(&self, x: f64) -> bool {
        self.lower <= x && x <= self.upper
    }
    pub fn plus(self, o: Interval) -> Interval {
        Interval {
            lower: self.lower + o.lower,
            upper: self.upper + o.upper,
        }
    }
    pub fn minus(self, o: Interval) -> Interval {
        Interval {
            lower: self.lower - o.upper,
            upper: self.upper - o.lower,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MassObservation {
    pub subject_sha256: Sha256Hex,
    pub density_source: Sha256Hex,
    pub solid_volume_m3: f64,
    pub volume_m3: Interval,
    pub nominal_printed_kg: Interval,
    pub basis: &'static str,
}

/// Nominal printed mass = admitted density bounds x integrated accepted-solid volume bounds.
pub fn integrate_nominal_printed_mass(
    solid: &AcceptedSolid,
    density: &Field<Bounds>,
) -> Result<MassObservation, MassError> {
    solid.check()?;
    let (bounds, source) = match density {
        Field::Admitted { value, source } => (value, source),
        _ => return Err(MassError::Unresolved(vec!["density:not_admitted".into()])),
    };
    bounds
        .check("density")
        .or_else(|e| invalid(e.to_string()))?;
    let b = solid.volume_relative_bound;
    let volume = Interval {
        lower: solid.solid_volume_m3 * (1.0 - b),
        upper: solid.solid_volume_m3 * (1.0 + b),
    };
    Ok(MassObservation {
        subject_sha256: solid.subject_sha256.clone(),
        density_source: source.clone(),
        solid_volume_m3: solid.solid_volume_m3,
        volume_m3: volume,
        nominal_printed_kg: Interval {
            lower: volume.lower * bounds.lower,
            upper: volume.upper * bounds.upper,
        },
        basis: "admitted_density_times_integrated_accepted_solid_volume",
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeighCondition {
    AsPrintedWithSupports,
    Finished,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeighedMass {
    pub kg: f64,
    pub uncertainty_kg: f64,
    pub condition: WeighCondition,
    pub scale_source: Sha256Hex,
}

/// Four distinct mass quantities; none is derived from another unless the term is supplied.
#[derive(Clone, Debug, PartialEq)]
pub struct MassLedger {
    pub nominal_printed_kg: Interval,
    /// Support/allowance stock present before removal; `None` = not yet planned (unknown, not zero).
    pub supports_allowances_kg: Option<Interval>,
    /// Material removed by machining; `None` = not planned.
    pub machined_removed_kg: Option<Interval>,
    pub weighed: Option<WeighedMass>,
}

impl MassLedger {
    pub fn as_printed_with_supports_kg(&self) -> Option<Interval> {
        self.supports_allowances_kg
            .map(|s| self.nominal_printed_kg.plus(s))
    }
    pub fn finished_kg(&self) -> Option<Interval> {
        self.machined_removed_kg
            .map(|m| self.nominal_printed_kg.minus(m))
    }
    /// Compare a weighed mass with the expectation for its own condition. A weighed mass never
    /// replaces a computed one, and an unplanned term leaves the comparison unresolved.
    pub fn weighed_consistency(&self) -> Verdict {
        let Some(w) = &self.weighed else {
            return Verdict::Unresolved(vec!["weighed_mass:not_measured".into()]);
        };
        if !(w.kg.is_finite()
            && w.kg > 0.0
            && w.uncertainty_kg.is_finite()
            && w.uncertainty_kg >= 0.0)
        {
            return Verdict::Fail(vec!["weighed_mass:invalid_measurement".into()]);
        }
        let expected = match w.condition {
            WeighCondition::AsPrintedWithSupports => self.as_printed_with_supports_kg(),
            WeighCondition::Finished => self.finished_kg(),
        };
        match expected {
            None => Verdict::Unresolved(vec!["weighed_mass:comparison_term_not_planned".into()]),
            Some(e) if w.kg + w.uncertainty_kg >= e.lower && w.kg - w.uncertainty_kg <= e.upper => {
                Verdict::Pass
            }
            Some(_) => Verdict::Fail(vec!["weighed_mass:outside_expected_bounds".into()]),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum VolumeBasis {
    AcceptedSolid(Sha256Hex),
    /// Closed-form estimate or caller-declared deltas (the legacy `analytic_estimate_v2`).
    AnalyticEstimate(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClaimedSolidVolume {
    pub basis: VolumeBasis,
    pub volume_m3: f64,
}

/// Refuse any solid-volume credit that is not the accepted solid's own integrated volume.
pub fn check_claimed_volume(
    claim: &ClaimedSolidVolume,
    solid: &AcceptedSolid,
) -> Result<(), MassError> {
    solid.check()?;
    match &claim.basis {
        VolumeBasis::AnalyticEstimate(_) => Err(MassError::UnsupportedCredit(
            "analytic_estimate_is_not_integrated_from_the_accepted_solid".into(),
        )),
        VolumeBasis::AcceptedSolid(subject) if *subject != solid.subject_sha256 => {
            Err(MassError::SubjectMismatch)
        }
        VolumeBasis::AcceptedSolid(_) => {
            if !claim.volume_m3.is_finite()
                || (claim.volume_m3 - solid.solid_volume_m3).abs()
                    > solid.volume_relative_bound * solid.solid_volume_m3
            {
                Err(MassError::UnsupportedCredit(
                    "claimed_volume_differs_from_the_accepted_solid".into(),
                ))
            } else {
                Ok(())
            }
        }
    }
}
