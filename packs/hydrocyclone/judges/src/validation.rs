//! V5-15 validation accession and controlled vortex/PDR study judges.
//!
//! A literature overlay is a comparison, not validation, until its geometry, fluids, boundary
//! conditions, stations, methods and uncertainty are admitted and equal to the evaluator's own
//! identity. Missing evidence is `Unresolved` and never `Pass`. Nothing in this module is a CT-35
//! measurement, a literature result, or a physical or scientific PASS.
use crate::{Field, Sha256Hex, Verdict};
use std::collections::BTreeSet;

fn open(reason: impl Into<String>) -> Verdict {
    Verdict::Unresolved(vec![reason.into()])
}

fn fail(reason: impl Into<String>) -> Verdict {
    Verdict::Fail(vec![reason.into()])
}

/// Published candidate behind an accession record. Zhou and Hsieh have no source in the repo yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Candidate {
    Bai,
    Young,
    Zhou,
    Hsieh,
}

/// Identity tags of one accessed source. Each tag is a digest or registry id, never free prose.
#[derive(Clone, Debug, PartialEq)]
pub struct Accession {
    pub candidate: Candidate,
    /// Digest of the private copy of the source. The raw document is never committed.
    pub source: Field<Sha256Hex>,
    pub license: Field<String>,
    pub geometry: Field<String>,
    pub fluids: Field<String>,
    pub boundary_conditions: Field<String>,
    pub stations: Field<String>,
    pub methods: Field<String>,
    pub uncertainty: Field<String>,
}

impl Accession {
    fn matchable(&self) -> [(&'static str, &Field<String>); 6] {
        [
            ("geometry", &self.geometry),
            ("fluids", &self.fluids),
            ("boundary_conditions", &self.boundary_conditions),
            ("stations", &self.stations),
            ("methods", &self.methods),
            ("uncertainty", &self.uncertainty),
        ]
    }
}

/// The evaluator's own identity that a literature value has to match.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelIdentity {
    pub geometry: String,
    pub fluids: String,
    pub boundary_conditions: String,
    pub stations: String,
    pub methods: String,
    pub uncertainty: String,
}

impl ModelIdentity {
    fn get(&self, key: &str) -> &str {
        match key {
            "geometry" => &self.geometry,
            "fluids" => &self.fluids,
            "boundary_conditions" => &self.boundary_conditions,
            "stations" => &self.stations,
            "methods" => &self.methods,
            "uncertainty" => &self.uncertainty,
            _ => "",
        }
    }
}

/// An accession passes only when its source digest, licence and every identity tag are admitted.
pub fn judge_accession(a: &Accession) -> Verdict {
    let mut open_items = Vec::new();
    if a.source.admitted().is_none() {
        open_items.push("accession:source_digest_not_admitted".to_string());
    }
    if a.license.admitted().is_none() {
        open_items.push("accession:license_not_admitted".to_string());
    }
    for (key, field) in a.matchable() {
        if field.admitted().is_none() {
            open_items.push(format!("accession:{key}_not_admitted"));
        }
    }
    if open_items.is_empty() {
        Verdict::Pass
    } else {
        Verdict::Unresolved(open_items)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ComparisonLevel {
    /// Shape, magnitude or trend comparison only. Not a validation claim.
    LiteratureOnly,
    /// Every identity tag is admitted and equal to the evaluator's.
    Quantitative,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComparisonCard {
    pub candidate: Candidate,
    pub level: ComparisonLevel,
    pub matched: Vec<String>,
    pub unresolved: Vec<String>,
}

/// Quantitative only when every identity tag is admitted and equal to the model's. A mismatch
/// keeps the overlay a literature comparison: it does not become a physical Fail.
pub fn compare(a: &Accession, model: &ModelIdentity) -> ComparisonCard {
    let (mut matched, mut unresolved) = (Vec::new(), Vec::new());
    for (key, field) in a.matchable() {
        match field.admitted() {
            Some(value) if value.as_str() == model.get(key) => matched.push(key.to_string()),
            Some(_) => unresolved.push(format!("{key}:literature_differs_from_model")),
            None => unresolved.push(format!("{key}:not_admitted")),
        }
    }
    if a.source.admitted().is_none() {
        unresolved.push("source:not_admitted".into());
    }
    if a.license.admitted().is_none() {
        unresolved.push("license:not_admitted".into());
    }
    let level = if unresolved.is_empty() {
        ComparisonLevel::Quantitative
    } else {
        ComparisonLevel::LiteratureOnly
    };
    ComparisonCard {
        candidate: a.candidate,
        level,
        matched,
        unresolved,
    }
}

/// Factors the preregistration may change, one per arm, relative to the frozen baseline.
pub const SWEPT_FACTORS: [&str; 5] = [
    "convection",
    "wall_layers",
    "run_duration",
    "initialization",
    "outlet_orifice_treatment",
];

/// Items frozen before any outcome is inspected.
pub const FROZEN_ITEMS: [&str; 4] = ["ct35_geometry", "taps", "duty", "analysis"];

#[derive(Clone, Debug, PartialEq)]
pub struct Preregistration {
    pub frozen: BTreeSet<String>,
    /// False once any outcome has been inspected; the study is then post hoc.
    pub frozen_before_outcomes: bool,
    /// One entry per arm, naming the single factor that arm changes.
    pub arms: Vec<String>,
    /// Competing explanations the study distinguishes. Absence of a vortex is one hypothesis, not a premise.
    pub hypotheses: BTreeSet<String>,
}

pub fn judge_preregistration(p: &Preregistration) -> Verdict {
    if !p.frozen_before_outcomes {
        return fail("preregistration:outcomes_inspected_before_freeze");
    }
    let mut parts = Vec::new();
    for item in FROZEN_ITEMS {
        if !p.frozen.contains(item) {
            parts.push(open(format!("frozen_item_missing:{item}")));
        }
    }
    if p.arms.is_empty() {
        parts.push(open("preregistration:no_factor_arms"));
    }
    for arm in &p.arms {
        if FROZEN_ITEMS.contains(&arm.as_str()) {
            parts.push(fail(format!("frozen_item_changed:{arm}")));
        } else if !SWEPT_FACTORS.contains(&arm.as_str()) {
            parts.push(fail(format!("factor_not_preregistered:{arm}")));
        }
    }
    if p.hypotheses.len() < 2 {
        parts.push(open("competing_explanations_required"));
    }
    Verdict::combine(parts)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Observable {
    TangentialProfile,
    AxialProfile,
    ReverseCore,
    PressureSplit,
    OilGrade,
}

/// What was actually measured, and on which geometry family.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasuredCoverage {
    pub observables: BTreeSet<Observable>,
    pub geometry_family: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoverageCard {
    pub observable: Observable,
    pub regime: String,
    pub supported: bool,
    pub reason: String,
}

/// An observable is supported only by a measurement of that same observable on the same family.
/// Pressure agreement never supports oil grade or another geometry family.
pub fn coverage(
    m: &MeasuredCoverage,
    model_family: &str,
    observable: Observable,
    regime: &str,
) -> CoverageCard {
    let (supported, reason) = if m.geometry_family != model_family {
        (false, "geometry_family_differs")
    } else if !m.observables.contains(&observable) {
        (false, "no_measurement_of_this_observable")
    } else {
        (true, "measured_on_same_family")
    };
    CoverageCard {
        observable,
        regime: regime.to_string(),
        supported,
        reason: reason.to_string(),
    }
}

/// Numerical, input and experimental discrepancy, each in the observable's units. Pass means all
/// three are quantified, finite and non-negative. It never means agreement and is never summed.
#[derive(Clone, Debug, PartialEq)]
pub struct Discrepancy {
    pub numerical: Option<f64>,
    pub input: Option<f64>,
    pub experimental: Option<f64>,
}

pub fn judge_discrepancy(d: &Discrepancy) -> Verdict {
    let mut parts = Vec::new();
    for (name, value) in [
        ("numerical", d.numerical),
        ("input", d.input),
        ("experimental", d.experimental),
    ] {
        match value {
            None => parts.push(open(format!("discrepancy_unquantified:{name}"))),
            Some(x) if !x.is_finite() || x < 0.0 => {
                parts.push(open(format!("discrepancy_invalid:{name}")))
            }
            Some(_) => {}
        }
    }
    Verdict::combine(parts)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdrOrigin {
    /// Published commercial range for other liners. Diagnostic context only.
    BorrowedCommercial,
    /// Range admitted from measurements on the CT-35 reference.
    MeasuredCt35,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PdrGate {
    pub origin: PdrOrigin,
    pub range: Field<(f64, f64)>,
    /// True when any input was changed after the PDR result was seen.
    pub tuned_after_pdr: bool,
}

pub fn judge_pdr(model_pdr: f64, gate: &PdrGate) -> Verdict {
    if gate.tuned_after_pdr {
        return fail("pdr:tuned_after_inspection");
    }
    if gate.origin == PdrOrigin::BorrowedCommercial {
        return open("pdr:borrowed_commercial_range_is_diagnostic_only");
    }
    let Some(&(lo, hi)) = gate.range.admitted() else {
        return open("pdr:range_not_admitted");
    };
    if !model_pdr.is_finite() {
        return open("pdr:model_value_not_finite");
    }
    if (lo..=hi).contains(&model_pdr) {
        Verdict::Pass
    } else {
        fail("pdr:outside_admitted_range")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ramp {
    Up,
    Down,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    pub initialization: String,
    pub ramp: Ramp,
    pub duty: f64,
    pub converged: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchStudy {
    pub branches: Vec<Branch>,
    /// Startup or control condition that selects a branch in operation.
    pub startup_condition: Option<String>,
    /// Keeping only the best initialized branch is not a general operating map.
    pub best_of_selection: bool,
    pub calibration: BTreeSet<String>,
    pub held_out: BTreeSet<String>,
}

pub fn judge_branches(s: &BranchStudy) -> Verdict {
    let mut parts = Vec::new();
    let initializations: BTreeSet<&str> = s
        .branches
        .iter()
        .map(|b| b.initialization.as_str())
        .collect();
    let has_up = s.branches.iter().any(|b| b.ramp == Ramp::Up);
    let has_down = s.branches.iter().any(|b| b.ramp == Ramp::Down);
    if initializations.len() < 2 || !has_up || !has_down {
        parts.push(open("multiple_states_not_challenged"));
    }
    for b in &s.branches {
        if !b.duty.is_finite() {
            parts.push(open(format!("duty_not_finite:{}", b.initialization)));
        }
        if !b.converged {
            parts.push(open(format!(
                "branch_not_converged:{}:{:?}",
                b.initialization, b.ramp
            )));
        }
    }
    if s.best_of_selection {
        parts.push(open("best_branch_selection_cannot_represent_operation"));
    }
    if s.startup_condition.is_none() {
        parts.push(open("startup_control_selection_missing"));
    }
    if s.held_out.is_empty() {
        parts.push(open("held_out_geometry_missing"));
    }
    if s.calibration.intersection(&s.held_out).next().is_some() {
        parts.push(fail("calibration_and_held_out_overlap"));
    }
    Verdict::combine(parts)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatasetStatus {
    Available,
    Withdrawn,
    Inaccessible,
}

/// Admission of the reference comparison for one evaluator build and a stated scope.
#[derive(Clone, Debug, PartialEq)]
pub struct VortexAdmission {
    pub evaluator_build: Sha256Hex,
    pub scope: BTreeSet<String>,
    pub dataset: DatasetStatus,
    /// Digest of the matching validation evidence, when it exists.
    pub evidence: Option<Sha256Hex>,
}

/// Ranking authority needs an admission bound to this evaluator build, a scope that covers the
/// request, an available dataset and matching evidence. Anything else stays open.
pub fn judge_admission(adm: Option<&VortexAdmission>, build: &Sha256Hex, scope: &str) -> Verdict {
    let Some(a) = adm else {
        return open("gvtx:not_admitted");
    };
    if a.evaluator_build != *build {
        return open("gvtx:revoked_by_evaluator_build_change");
    }
    if !a.scope.contains(scope) {
        return open("gvtx:scope_outside_admission");
    }
    match a.dataset {
        DatasetStatus::Available => {}
        DatasetStatus::Withdrawn => return open("gvtx:dataset_withdrawn_revokes_ranking"),
        DatasetStatus::Inaccessible => return open("gvtx:dataset_inaccessible"),
    }
    if a.evidence.is_none() {
        return open("gvtx:matching_evidence_missing");
    }
    Verdict::Pass
}

/// Flow archives stay disabled unless the admission verdict is Pass.
pub fn flow_archive_enabled(admission: &Verdict) -> bool {
    admission.is_pass()
}

/// Owned CT-35 measurement protocol. Defining it is allowed; launching it is not.
#[derive(Clone, Debug, PartialEq)]
pub struct Ct35Protocol {
    pub launched: bool,
    pub replicates: u32,
    pub drift_conc_rel: f64,
    pub drift_dp_rel: f64,
    pub taps_frozen: bool,
}

/// Gates may be stricter than the rig protocol (3 replicates, 10 % concentration and 5 % pressure
/// drift) but never looser. Pass only certifies that the definition is well formed.
pub fn judge_protocol(p: &Ct35Protocol) -> Verdict {
    let mut parts = Vec::new();
    if p.launched {
        parts.push(fail("ct35_protocol:launched_under_v5_15"));
    }
    if p.replicates < 3 {
        parts.push(fail("ct35_protocol:replicates_below_minimum"));
    }
    let conc_ok = p.drift_conc_rel > 0.0 && p.drift_conc_rel <= 0.10;
    let dp_ok = p.drift_dp_rel > 0.0 && p.drift_dp_rel <= 0.05;
    if !conc_ok || !dp_ok {
        parts.push(fail("ct35_protocol:drift_gate_loosened"));
    }
    if !p.taps_frozen {
        parts.push(open("ct35_protocol:taps_not_frozen"));
    }
    Verdict::combine(parts)
}
