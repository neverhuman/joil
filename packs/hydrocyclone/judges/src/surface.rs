//! V5-13-04: finish and orientation tied to physical evaluation.
//!
//! Profilometer `Ra` and equivalent sand-grain roughness `ks` are different observables. `ks`
//! is derived only through an admitted calibration for the face's own finish class and `Ra`
//! range; a universal ratio (the legacy `ks = 3.1 * Ra`) has no such calibration and stays
//! unresolved. A [`SurfaceVariant`] identity binds orientation and per-face finish; every
//! flow, structure and export evaluation must carry the same identity or it is stale.
use crate::{canonical_hash, invalid, Field, JudgeError, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaceFinish {
    pub face_id: String,
    pub finish_class: String,
    /// Measured or declared profilometer Ra in micrometres; `None` = not measured.
    pub ra_um: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceVariant {
    pub subject_sha256: Sha256Hex,
    pub orientation: String,
    pub faces: Vec<FaceFinish>,
}

impl SurfaceVariant {
    pub fn check(&self) -> Result<(), JudgeError> {
        if self.orientation.trim().is_empty() || self.faces.is_empty() {
            return Err(invalid(
                "variant needs an orientation and at least one face",
            ));
        }
        let mut ids = BTreeSet::new();
        for f in &self.faces {
            if f.face_id.trim().is_empty()
                || f.finish_class.trim().is_empty()
                || !ids.insert(f.face_id.as_str())
            {
                return Err(invalid("empty or duplicate face id or finish class"));
            }
            if let Some(ra) = f.ra_um {
                if !(ra.is_finite() && ra > 0.0) {
                    return Err(invalid("Ra must be finite and positive"));
                }
            }
        }
        Ok(())
    }

    /// Identity of this exact subject/orientation/finish combination.
    pub fn variant_id(&self) -> Result<Sha256Hex, JudgeError> {
        self.check()?;
        let mut faces = self.faces.clone();
        faces.sort_by(|a, b| a.face_id.cmp(&b.face_id));
        canonical_hash(&SurfaceVariant {
            faces,
            ..self.clone()
        })
    }
}

/// Calibration of equivalent sand roughness against Ra for one finish class.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RaToKs {
    pub finish_class: String,
    pub ra_min_um: f64,
    pub ra_max_um: f64,
    pub ks_over_ra_lower: f64,
    pub ks_over_ra_upper: f64,
    pub calibration_points: u32,
}

impl RaToKs {
    pub fn check(&self) -> Result<(), JudgeError> {
        let finish = self.finish_class.trim();
        if finish.is_empty() || matches!(finish, "*" | "universal" | "any") {
            return Err(invalid(
                "a calibration names one finish class; a universal ratio is not a calibration",
            ));
        }
        let ok = [
            self.ra_min_um,
            self.ra_max_um,
            self.ks_over_ra_lower,
            self.ks_over_ra_upper,
        ]
        .iter()
        .all(|v| v.is_finite() && *v > 0.0)
            && self.ra_min_um <= self.ra_max_um
            && self.ks_over_ra_lower <= self.ks_over_ra_upper;
        if !ok || self.calibration_points < 3 {
            return Err(invalid("calibration needs an ordered positive Ra range, ordered ratio bounds and >= 3 points"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Roughness {
    /// Equivalent sand roughness bounds in micrometres, with the calibration that produced them.
    Ks {
        lower_um: f64,
        upper_um: f64,
        calibration_source: Sha256Hex,
    },
    Unresolved(Vec<String>),
}

/// Equivalent sand roughness of one face. Ra alone never becomes `ks`.
pub fn equivalent_sand_roughness(
    face: &FaceFinish,
    calibrations: &BTreeMap<String, Field<RaToKs>>,
) -> Roughness {
    let name = format!("face:{}", face.face_id);
    let Some(ra) = face.ra_um else {
        return Roughness::Unresolved(vec![format!("{name}:ra_not_measured")]);
    };
    let Some(field) = calibrations.get(&face.finish_class) else {
        return Roughness::Unresolved(vec![format!(
            "{name}:no_calibration_for_finish_class:{}",
            face.finish_class
        )]);
    };
    let Field::Admitted { value: cal, source } = field else {
        return Roughness::Unresolved(vec![format!(
            "{name}:calibration_not_admitted:{}",
            face.finish_class
        )]);
    };
    if cal.check().is_err() || cal.finish_class != face.finish_class {
        return Roughness::Unresolved(vec![format!(
            "{name}:calibration_invalid_or_wrong_finish_class"
        )]);
    }
    if ra < cal.ra_min_um || ra > cal.ra_max_um {
        return Roughness::Unresolved(vec![format!("{name}:ra_outside_calibrated_range")]);
    }
    Roughness::Ks {
        lower_um: ra * cal.ks_over_ra_lower,
        upper_um: ra * cal.ks_over_ra_upper,
        calibration_source: source.clone(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationKind {
    Flow,
    Structure,
    Export,
    Mass,
}

impl EvaluationKind {
    fn name(self) -> &'static str {
        match self {
            Self::Flow => "flow",
            Self::Structure => "structure",
            Self::Export => "export",
            Self::Mass => "mass",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evaluation {
    pub kind: EvaluationKind,
    pub variant_id: Sha256Hex,
}

/// Flow, structure and export (and mass, when supplied) must all use this variant. A changed
/// orientation or finish changes the identity, so dependent evaluations become stale.
pub fn check_same_variant(
    variant: &SurfaceVariant,
    evaluations: &[Evaluation],
) -> Result<Verdict, JudgeError> {
    let id = variant.variant_id()?;
    let mut missing: BTreeSet<EvaluationKind> = [
        EvaluationKind::Flow,
        EvaluationKind::Structure,
        EvaluationKind::Export,
    ]
    .into();
    let mut stale = Vec::new();
    for e in evaluations {
        missing.remove(&e.kind);
        if e.variant_id != id {
            stale.push(format!("stale:{}", e.kind.name()));
        }
    }
    if !stale.is_empty() {
        stale.sort();
        stale.dedup();
        return Ok(Verdict::Fail(stale));
    }
    if !missing.is_empty() {
        return Ok(Verdict::Unresolved(
            missing
                .into_iter()
                .map(|k| format!("missing:{}", k.name()))
                .collect(),
        ));
    }
    Ok(Verdict::Pass)
}
