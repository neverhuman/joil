//! V5-15 validation accession and controlled vortex/PDR study judges.
//!
//! A literature overlay is a comparison, not validation, until its geometry, fluids, boundary
//! conditions, stations, methods and uncertainty are admitted and equal to the evaluator's own
//! identity. Missing evidence is `Unresolved` and never `Pass`. Nothing in this module is a CT-35
//! measurement, a literature result, or a physical or scientific PASS.
use crate::{Field, Sha256Hex, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

fn open(reason: impl Into<String>) -> Verdict {
    Verdict::Unresolved(vec![reason.into()])
}

fn fail(reason: impl Into<String>) -> Verdict {
    Verdict::Fail(vec![reason.into()])
}

/// Published candidate behind an accession record. Zhou and Hsieh have no source in the repo yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Candidate {
    Bai,
    Young,
    Zhou,
    Hsieh,
    Wang,
    Xu,
    Delgadillo,
}

/// Identity tags of one accessed source. Each tag is a digest or registry id, never free prose.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Comparison eligibility alone never grants scientific admission.
    pub scientific_admission: bool,
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
        scientific_admission: false,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

/// Values are SI station samples, with separate uncertainty components in the same unit.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkStationDigest {
    pub quantity: String,
    pub unit: String,
    pub values: Vec<f64>,
    pub sha256: Sha256Hex,
    pub numerical_uncertainty: f64,
    pub input_uncertainty: f64,
    pub experimental_uncertainty: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkAccessionRecord {
    pub accession: Accession,
    pub synthetic_or_proxy: bool,
    pub scientific_admission: bool,
    pub stations: Vec<BenchmarkStationDigest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkAccessionCorpus {
    pub schema_version: u32,
    pub records: Vec<BenchmarkAccessionRecord>,
}

/// Tracked synthetic software fixture, never a claim to have digitized published measurements.
pub fn build_default_benchmark_accession_corpus() -> BenchmarkAccessionCorpus {
    serde_json::from_str(include_str!(
        "../../../../experiments/datasets/v5-15-01-benchmark-accessions.json"
    ))
    .expect("tracked benchmark corpus must deserialize")
}

pub fn ingest_benchmark_accession_corpus(
    json_str: &str,
    model: &ModelIdentity,
) -> Result<Vec<ComparisonCard>, String> {
    let corpus: BenchmarkAccessionCorpus =
        serde_json::from_str(json_str).map_err(|e| e.to_string())?;
    if corpus.schema_version != 1 || corpus.records.is_empty() {
        return Err("empty or unsupported benchmark corpus".into());
    }
    let mut seen = BTreeSet::new();
    let mut cards = Vec::new();
    for record in corpus.records {
        if !seen.insert(record.accession.candidate)
            || (record.synthetic_or_proxy && record.scientific_admission)
        {
            return Err("duplicate benchmark or synthetic scientific admission".into());
        }
        let required = [
            ("axial_profile", "m/s"),
            ("tangential_profile", "m/s"),
            ("patch_flux_balance", "m3/s"),
            ("pressure_drop", "Pa"),
            ("split_ratio", "1"),
        ];
        if record.stations.len() != required.len() {
            return Err("incomplete station set".into());
        }
        for (quantity, unit) in required {
            let samples: Vec<_> = record
                .stations
                .iter()
                .filter(|s| s.quantity == quantity)
                .collect();
            if samples.len() != 1 {
                return Err("missing or duplicate station quantity".into());
            }
            let station = samples[0];
            if station.unit != unit
                || station.values.is_empty()
                || station.values.iter().any(|v| !v.is_finite())
                || [
                    station.numerical_uncertainty,
                    station.input_uncertainty,
                    station.experimental_uncertainty,
                ]
                .iter()
                .any(|u| !u.is_finite() || *u < 0.0)
                || Sha256Hex::of_bytes(
                    &serde_json::to_vec(&station.values).map_err(|e| e.to_string())?,
                ) != station.sha256
            {
                return Err(format!(
                    "invalid station values, units, uncertainty or digest: {quantity}"
                ));
            }
            if quantity == "split_ratio" && station.values.iter().any(|v| !(0.0..=1.0).contains(v))
            {
                return Err("split ratio outside [0,1]".into());
            }
        }
        let mut card = compare(&record.accession, model);
        if record.synthetic_or_proxy {
            card.level = ComparisonLevel::LiteratureOnly;
            card.unresolved
                .push("synthetic_or_proxy:not_scientifically_admitted".into());
        }
        if record.scientific_admission && card.level != ComparisonLevel::Quantitative {
            return Err("scientific admission requires matching admitted identity".into());
        }
        // Empty identity strings cannot be evidence even if declared Admitted.
        if record
            .accession
            .matchable()
            .iter()
            .any(|(_, f)| f.admitted().is_some_and(|v| v.trim().is_empty()))
            || record
                .accession
                .license
                .admitted()
                .is_some_and(|v| v.trim().is_empty())
        {
            return Err("empty admitted identity or license".into());
        }
        cards.push(card);
    }
    Ok(cards)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransientBranchWindow {
    pub start_s: f64,
    pub end_s: f64,
    pub core_reversal_fraction: f64,
    pub split_ratio: f64,
    pub euler_number: f64,
}
impl TransientBranchWindow {
    fn observables(&self) -> [f64; 3] {
        [
            self.core_reversal_fraction,
            self.split_ratio,
            self.euler_number,
        ]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BranchHysteresisTrajectory {
    pub geometry: String,
    pub initialization: String,
    pub ramp: Ramp,
    pub duty_m3_s: f64,
    pub windows: Vec<TransientBranchWindow>,
    pub max_allowed_drift: f64,
    pub startup_control_rule: String,
    pub best_of_selection: bool,
    pub synthetic: bool,
    pub scientific_admission: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BranchHysteresisAudit {
    pub trajectories: usize,
    pub converged: Vec<bool>,
    pub window_drifts: Vec<f64>,
    /// Absolute up/down gaps in core fraction, split ratio, and Euler number, respectively.
    pub matched_hysteresis_gaps: Vec<[f64; 3]>,
    pub scientific_admission: bool,
}

pub fn evaluate_branch_hysteresis_ramps(
    trajectories: &[BranchHysteresisTrajectory],
    held_out_geometries: &BTreeSet<String>,
    calibration_geometries: &BTreeSet<String>,
) -> Result<BranchHysteresisAudit, String> {
    if trajectories.is_empty()
        || held_out_geometries.is_empty()
        || calibration_geometries.is_empty()
        || !held_out_geometries.is_disjoint(calibration_geometries)
    {
        return Err("missing trajectories or disjoint geometry populations".into());
    }
    let mut audit = BranchHysteresisAudit {
        trajectories: trajectories.len(),
        converged: vec![],
        window_drifts: vec![],
        matched_hysteresis_gaps: vec![],
        scientific_admission: false,
    };
    for t in trajectories {
        if t.geometry.trim().is_empty()
            || (!held_out_geometries.contains(&t.geometry)
                && !calibration_geometries.contains(&t.geometry))
            || !["zero_swirl", "potential_core", "steady_rans_handoff"]
                .contains(&t.initialization.as_str())
            || !t.duty_m3_s.is_finite()
            || t.duty_m3_s <= 0.0
            || !t.max_allowed_drift.is_finite()
            || !(0.0..=1.0).contains(&t.max_allowed_drift)
            || t.windows.len() < 2
            || t.startup_control_rule.trim().is_empty()
            || t.best_of_selection
            || (t.synthetic && t.scientific_admission)
        {
            return Err("invalid branch trajectory or selection policy".into());
        }
        for (i, w) in t.windows.iter().enumerate() {
            if !w.start_s.is_finite()
                || !w.end_s.is_finite()
                || w.start_s < 0.0
                || w.end_s <= w.start_s
                || (i > 0 && w.start_s < t.windows[i - 1].end_s)
                || w.observables().iter().any(|v| !v.is_finite())
                || !(0.0..=1.0).contains(&w.core_reversal_fraction)
                || !(0.0..=1.0).contains(&w.split_ratio)
                || w.euler_number <= 0.0
            {
                return Err("invalid or overlapping averaging windows".into());
            }
        }
        let previous = t.windows[t.windows.len() - 2].observables();
        let last = t.windows[t.windows.len() - 1].observables();
        // Symmetric relative drift: zero/zero = 0; zero/nonzero = 1. No arbitrary epsilon.
        let drift = previous
            .into_iter()
            .zip(last)
            .map(|(a, b)| {
                let scale = a.abs().max(b.abs());
                if scale == 0.0 {
                    0.0
                } else {
                    (a - b).abs() / scale
                }
            })
            .fold(0.0_f64, f64::max);
        audit.window_drifts.push(drift);
        audit.converged.push(drift <= t.max_allowed_drift);
        for initialization in ["zero_swirl", "potential_core", "steady_rans_handoff"] {
            for ramp in [Ramp::Up, Ramp::Down] {
                if trajectories
                    .iter()
                    .filter(|other| {
                        other.geometry == t.geometry
                            && other.duty_m3_s == t.duty_m3_s
                            && other.initialization == initialization
                            && other.ramp == ramp
                    })
                    .count()
                    != 1
                {
                    return Err(
                        "each duty needs exactly all three initializations and both ramps".into(),
                    );
                }
            }
        }
        if t.ramp == Ramp::Up {
            let down = trajectories
                .iter()
                .find(|other| {
                    other.geometry == t.geometry
                        && other.initialization == t.initialization
                        && other.duty_m3_s == t.duty_m3_s
                        && other.ramp == Ramp::Down
                })
                .ok_or("unmatched ramp")?;
            let other = down
                .windows
                .last()
                .ok_or("missing down window")?
                .observables();
            audit
                .matched_hysteresis_gaps
                .push(std::array::from_fn(|i| (last[i] - other[i]).abs()));
        }
    }
    for geometry in held_out_geometries.union(calibration_geometries) {
        let mut duties: Vec<_> = trajectories
            .iter()
            .filter(|t| &t.geometry == geometry)
            .map(|t| t.duty_m3_s)
            .collect();
        duties.sort_by(f64::total_cmp);
        duties.dedup();
        if duties.len() < 2 {
            return Err("duty sweep requires at least two duties per geometry".into());
        }
    }
    Ok(audit)
}

pub fn verify_branch_hysteresis_json(json_str: &str) -> Result<BranchHysteresisAudit, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Input {
        trajectories: Vec<BranchHysteresisTrajectory>,
        held_out_geometries: BTreeSet<String>,
        calibration_geometries: BTreeSet<String>,
    }
    let input: Input = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
    evaluate_branch_hysteresis_ramps(
        &input.trajectories,
        &input.held_out_geometries,
        &input.calibration_geometries,
    )
}

#[cfg(test)]
mod accession_branch_tests {
    use super::*;

    #[test]
    fn v5_15_01_benchmark_accession_corpus_enforces_identity_tags_and_literature_only_separation() {
        let corpus = build_default_benchmark_accession_corpus();
        let model = ModelIdentity {
            geometry: "synthetic-geometry".into(),
            fluids: "synthetic-fluids".into(),
            boundary_conditions: "synthetic-boundary_conditions".into(),
            stations: "synthetic-stations".into(),
            methods: "synthetic-methods".into(),
            uncertainty: "synthetic-uncertainty".into(),
        };
        let evaluate = |c: &BenchmarkAccessionCorpus| {
            ingest_benchmark_accession_corpus(&serde_json::to_string(c).unwrap(), &model)
        };
        let cards = evaluate(&corpus).unwrap();
        assert_eq!(cards.len(), 4);
        assert!(cards
            .iter()
            .all(|c| c.level == ComparisonLevel::LiteratureOnly));
        let mut admitted = corpus.clone();
        admitted.records.truncate(1);
        let record = &mut admitted.records[0];
        record.synthetic_or_proxy = false;
        let digest = Sha256Hex::of_bytes(b"test-only admission provenance");
        record.accession.source = Field::Admitted {
            value: digest.clone(),
            source: digest.clone(),
        };
        for field in [
            &mut record.accession.license,
            &mut record.accession.geometry,
            &mut record.accession.fluids,
            &mut record.accession.boundary_conditions,
            &mut record.accession.stations,
            &mut record.accession.methods,
            &mut record.accession.uncertainty,
        ] {
            let Field::Proposed { value, .. } = field.clone() else {
                panic!("fixture must be proposed")
            };
            *field = Field::Admitted {
                value,
                source: digest.clone(),
            };
        }
        assert_eq!(
            evaluate(&admitted).unwrap()[0].level,
            ComparisonLevel::Quantitative
        );
        for key in [
            "geometry",
            "fluids",
            "boundary_conditions",
            "stations",
            "methods",
            "uncertainty",
            "license",
            "source",
        ] {
            let mut value = serde_json::to_value(&admitted).unwrap();
            value["records"][0]["accession"][key]["status"] = "proposed".into();
            value["records"][0]["accession"][key]
                .as_object_mut()
                .unwrap()
                .remove("source");
            value["records"][0]["accession"][key]["reason"] = "not admitted".into();
            assert_eq!(
                ingest_benchmark_accession_corpus(&value.to_string(), &model).unwrap()[0].level,
                ComparisonLevel::LiteratureOnly,
                "{key}"
            );
        }
        for mutation in 0..7 {
            let mut bad = corpus.clone();
            match mutation {
                0 => bad.records[0].scientific_admission = true,
                1 => bad.records[0].stations[0].values.clear(),
                2 => bad.records[0].stations[0].values[0] = f64::INFINITY,
                3 => bad.records[0].stations[0].values[0] += 1.0,
                4 => bad.records[0].stations[0].unit = "mm/s".into(),
                5 => bad.records[0].stations[0].input_uncertainty = -1.0,
                _ => bad.records[0].stations.pop().map(|_| ()).unwrap(),
            }
            assert!(evaluate(&bad).is_err(), "mutation {mutation}");
        }
        let mut mismatch = admitted.clone();
        mismatch.records[0].accession.geometry = Field::Admitted {
            value: "different".into(),
            source: digest,
        };
        assert_eq!(
            evaluate(&mismatch).unwrap()[0].level,
            ComparisonLevel::LiteratureOnly
        );
        mismatch.records[0].scientific_admission = true;
        assert!(evaluate(&mismatch).is_err());
        admitted.records[0].synthetic_or_proxy = true;
        assert_eq!(
            evaluate(&admitted).unwrap()[0].level,
            ComparisonLevel::LiteratureOnly
        );
    }

    #[test]
    fn v5_15_04_branch_hysteresis_ramps_verify_window_convergence_and_reject_best_branch_selection()
    {
        let text =
            include_str!("../../../../experiments/datasets/v5-15-04-branch-hysteresis-ramps.json");
        let audit = verify_branch_hysteresis_json(text).unwrap();
        assert_eq!(audit.trajectories, 24);
        assert!(audit.converged.iter().all(|v| *v));
        assert_eq!(audit.matched_hysteresis_gaps.len(), 12);
        assert!((audit.matched_hysteresis_gaps[0][0] - 0.05).abs() < 1e-12);
        assert!(!audit.scientific_admission);
        let original: serde_json::Value = serde_json::from_str(text).unwrap();
        for mutation in 0..8 {
            let mut bad = original.clone();
            match mutation {
                0 => bad["trajectories"][0]["best_of_selection"] = true.into(),
                1 => bad["trajectories"][0]["startup_control_rule"] = "".into(),
                2 => bad["held_out_geometries"] = bad["calibration_geometries"].clone(),
                3 => {
                    bad["trajectories"].as_array_mut().unwrap().pop();
                }
                4 => bad["trajectories"][0]["scientific_admission"] = true.into(),
                5 => bad["trajectories"][0]["windows"][1]["start_s"] = 0.0.into(),
                6 => bad["trajectories"][0]["windows"][2]["split_ratio"] = 1.1.into(),
                _ => bad["trajectories"][0]["max_allowed_drift"] = (-0.1).into(),
            }
            assert!(
                verify_branch_hysteresis_json(&bad.to_string()).is_err(),
                "mutation {mutation}"
            );
        }
        let mut drifting = original.clone();
        drifting["trajectories"][0]["windows"][2]["split_ratio"] = 0.8.into();
        assert!(
            !verify_branch_hysteresis_json(&drifting.to_string())
                .unwrap()
                .converged[0]
        );
        let mut zero = original;
        for i in [1, 2] {
            zero["trajectories"][0]["windows"][i]["core_reversal_fraction"] = 0.0.into();
        }
        assert!(
            verify_branch_hysteresis_json(&zero.to_string())
                .unwrap()
                .converged[0]
        );
    }
}
