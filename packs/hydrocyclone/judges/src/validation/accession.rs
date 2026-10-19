use super::*;

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
        "../../../../../experiments/datasets/v5-15-01-benchmark-accessions.json"
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
        let text = include_str!(
            "../../../../../experiments/datasets/v5-15-04-branch-hysteresis-ramps.json"
        );
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
