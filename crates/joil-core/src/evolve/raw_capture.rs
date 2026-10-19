//! Immutable Wave-4 raw proof and per-seed JSONL/JSON capture writer and unit tests.

use super::*;

pub(crate) struct RawProofCapture {
    pub(crate) root: PathBuf,
    pub(crate) identity: serde_json::Value,
    pub(crate) config: serde_json::Value,
}

pub(crate) fn raw_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn raw_write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("create immutable capture {}: {e}", path.display()))?;
    let mut out = BufWriter::new(file);
    serde_json::to_writer(&mut out, value).map_err(|e| e.to_string())?;
    out.write_all(b"\n")
        .and_then(|_| out.flush())
        .map_err(|e| e.to_string())
}

pub(crate) fn raw_event(
    out: &mut BufWriter<fs::File>,
    value: &serde_json::Value,
) -> Result<(), String> {
    serde_json::to_writer(&mut *out, value).map_err(|e| e.to_string())?;
    out.write_all(b"\n").map_err(|e| e.to_string())
}

impl RawProofCapture {
    pub(crate) fn new(root: &Path, seeds: &[u64], cfg: &EvolutionConfig) -> Result<Self, String> {
        let admission = crate::experiment::admit_search_campaign(cfg.campaign_mode)?;
        let unique: std::collections::BTreeSet<_> = seeds.iter().collect();
        if seeds.len() < 5 || unique.len() != seeds.len() {
            return Err("raw proof requires at least five distinct master seeds".into());
        }
        let pop = cfg
            .islands
            .checked_mul(cfg.pop_per_island)
            .filter(|n| *n > 0)
            .ok_or("positive finite population budget required")?;
        let budget = cfg
            .generations
            .checked_add(1)
            .and_then(|g| g.checked_mul(pop))
            .ok_or("raw proposal budget overflow")?;
        if cfg.mc_samples == 0 {
            return Err("positive declared Monte Carlo sample count required".into());
        }
        let command = |program: &str, args: &[&str]| -> Result<String, String> {
            let output = Command::new(program)
                .args(args)
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err(format!(
                    "capture identity command failed: {program} {args:?}"
                ));
            }
            Ok(String::from_utf8_lossy(&output.stdout).trim().into())
        };
        let source_commit = command("git", &["rev-parse", "HEAD"])?;
        let tracked_dirty =
            !command("git", &["status", "--porcelain", "--untracked-files=no"])?.is_empty();
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let identity = serde_json::json!({
            "campaign_id": format!("wave4-repro-{timestamp}-{}", std::process::id()),
            "source_commit": source_commit, "tracked_source_dirty": tracked_dirty,
            "contract_status": "NEW; original Wave-4 raw records were never emitted",
            "run_kind": "new_sandbox_reproducibility_run_not_original_campaign",
            "objective_sha256": admission.objective_sha256,
            "requirements": admission.requirements,
            "evidence_limitations": ["prior-laden reduced screen", "authored family folds are not independent geometry holdouts", "CT35 repeated subjects are not independent physical replicates", "archive hypervolume remains emitted telemetry"],
            "compiler": command("rustc", &["-vV"] )?,
            "executable_sha256": raw_sha256(&fs::read(executable).map_err(|e| e.to_string())?),
            "compiled_sources_sha256": {
                "evolve.rs": raw_sha256(include_bytes!("../evolve.rs")),
                "program.rs": raw_sha256(include_bytes!("../program.rs")),
                "phenotype.rs": raw_sha256(include_bytes!("../phenotype.rs")),
                "geometry.rs": raw_sha256(include_bytes!("../geometry.rs")),
                "structure.rs": raw_sha256(include_bytes!("../structure.rs")),
                "redteam.rs": raw_sha256(include_bytes!("../redteam.rs")),
                "surrogate.rs": raw_sha256(include_bytes!("../surrogate.rs")),
                "Cargo.lock": raw_sha256(include_bytes!("../../../../Cargo.lock")),
            },
            "evolution_config": cfg,
            "evolution_config_sha256": raw_sha256(&serde_json::to_vec(cfg).map_err(|e| e.to_string())?),
            "models": {
                "reduced_screen": {"id": "joil.sandbox-reduced-prior", "source_sha256": raw_sha256(include_bytes!("../evolve.rs")), "calibration": "prior_laden_not_independently_validated"},
                "phenotype_compiler": {"source_sha256": raw_sha256(include_bytes!("../phenotype.rs")), "geometry_source_sha256": raw_sha256(include_bytes!("../geometry.rs"))},
                "structural_screen": {"source_sha256": raw_sha256(include_bytes!("../structure.rs")), "kind": "analytic_screen_not_FEA"},
                "surrogate": {"source_sha256": raw_sha256(include_bytes!("../surrogate.rs")), "fold_kernel_length_scale": 0.85, "fold_noise": 0.02, "acquisition_kernel_length_scale": 0.85, "acquisition_noise": 0.015},
            },
            "rng": {"algorithm": "SplitMix64", "master_seeds": seeds,
                "gen0_seed": "master_seed XOR 0x517CC1B727220A95",
                "mc_seed": "master_seed XOR first_16_hex_digits(genome_sha256)",
                "variation_seed": "master_seed", "fold_kernel_length_scale": 0.85, "fold_noise": 0.02},
            "argv": std::env::args().collect::<Vec<_>>(),
            "cpu_set": std::env::var("JOIL_CPU_SET").ok(),
            "rayon_num_threads": std::env::var("RAYON_NUM_THREADS").ok(),
        });
        // Refuse even an empty existing directory: no overwrite/resume ambiguity.
        if let Some(parent) = root.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::create_dir(root)
            .map_err(|e| format!("create NEW raw directory {}: {e}", root.display()))?;
        let raw = Self {
            root: root.into(),
            identity,
            config: serde_json::json!({
                "master_seeds": seeds, "generations": cfg.generations,
                "population_per_generation": pop, "proposal_budget": budget,
                "quality_limit_mg_l": crate::experiment::research_requirements()?.thresholds.quality_limit_mg_l.get(),
                "mc_samples": cfg.mc_samples,
            }),
        };
        raw.json("capture-provenance.json", &raw.identity)?;
        Ok(raw)
    }

    pub(crate) fn json(&self, name: &str, value: &impl Serialize) -> Result<(), String> {
        raw_write_json(&self.root.join(name), value)
    }

    pub(crate) fn reference(
        &self,
        evaluation_key: &str,
        id: String,
        program: DesignProgram,
        cfg: &EvolutionConfig,
    ) -> Result<Individual, String> {
        let mut mc = Vec::new();
        let (individual, _) =
            evaluate_individual_recorded(id.clone(), program, 0, 0, cfg, Some(&mut mc))?;
        self.json(
            &format!("reference-{evaluation_key}.jsonl"),
            &serde_json::json!({
                "event": "reference_evaluation", "evaluation_id": evaluation_key,
                "individual": individual, "mc_observations": mc,
            }),
        )?;
        Ok(individual)
    }

    pub(crate) fn finish(&self, report: &Wave3EvolutionProofReport) -> Result<(), String> {
        self.json("proof-summary.json", report)?;
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                return Err("unexpected non-file in raw capture".into());
            }
            let bytes = fs::read(entry.path()).map_err(|e| e.to_string())?;
            files.push(serde_json::json!({"path": entry.file_name().to_str().ok_or("invalid capture filename")?,
                "bytes": bytes.len(), "sha256": raw_sha256(&bytes)}));
        }
        files.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
        self.json("manifest.json", &serde_json::json!({
            "schema": "joil.wave4-raw-manifest.v1", "state": "complete",
            "identity": self.identity, "config": self.config, "files": files,
            "summary_budget_note": "Original five-arm proposal slots unchanged; equal-budget CT35 adds one separately identified arm; per-seed and family references are outside proposal slots",
            "completion_note": "Manifest emitted only after every seed, fold and summary write completed",
        }))?;
        let bytes = fs::read(self.root.join("manifest.json")).map_err(|e| e.to_string())?;
        let mut digest = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join("manifest.sha256"))
            .map_err(|e| e.to_string())?;
        writeln!(digest, "{}  manifest.json", raw_sha256(&bytes)).map_err(|e| e.to_string())
    }
}

pub(crate) struct RawSeedCapture {
    writer: BufWriter<fs::File>,
    document_path: PathBuf,
    document: serde_json::Value,
    proposals: Vec<serde_json::Value>,
    generations: Vec<serde_json::Value>,
}

impl RawSeedCapture {
    pub(crate) fn new(
        raw: &RawProofCapture,
        seed: u64,
        arm: Wave3ComparatorArm,
        reference: &Individual,
        mc: Vec<serde_json::Value>,
    ) -> Result<Self, String> {
        let arm = match arm {
            Wave3ComparatorArm::B0BaselineCt35 => "B0_CT35_Equal_Budget",
            Wave3ComparatorArm::B1RandomSearchExotic3d => "B1_Random_Search",
            Wave3ComparatorArm::B2ConventionalAxisymNsga2 => "B2_Conventional_Axisym_NSGA2",
            Wave3ComparatorArm::A1NoMutationCrossoverOnly => "A1_No_Mutation_Crossover_Only",
            Wave3ComparatorArm::A2NoSelectionRandomWalk => "A2_No_Selection_Random_Walk",
            Wave3ComparatorArm::B3Wave3Exotic3dQdCei => "B3_Wave3_Exotic3D_QD_CEI",
        };
        let stem = format!("wave4-proof-raw-{arm}-s{seed}");
        let mut writer = BufWriter::new(
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(raw.root.join(format!("{stem}.jsonl")))
                .map_err(|e| e.to_string())?,
        );
        raw_event(
            &mut writer,
            &serde_json::json!({
                "event": "seed_reference", "seed": seed, "arm": arm,
                "identity": raw.identity, "config": raw.config,
                "individual": reference, "mc_observations": mc,
            }),
        )?;
        Ok(Self {
            writer,
            document_path: raw.root.join(format!("{stem}.json")),
            document: serde_json::json!({"schema": "joil.wave4-raw-seed.v1",
                "seed": seed, "arm": arm, "config": raw.config, "reference": reference,
                "campaign_id": raw.identity["campaign_id"],
                "source_commit": raw.identity["source_commit"],
                "objective_sha256": raw.identity["objective_sha256"],
            }),
            proposals: Vec::new(),
            generations: Vec::new(),
        })
    }

    pub(crate) fn proposal(
        &mut self,
        outcome: &VariationOutcome,
        individual: Option<&Individual>,
        mc: Vec<serde_json::Value>,
    ) -> Result<(), String> {
        let state = if outcome.failure.is_some() || outcome.child.is_none() {
            "invalid"
        } else if !outcome.repairs.is_empty() {
            "repaired"
        } else {
            "evaluated"
        };
        let proposal = serde_json::json!({"state": state, "individual": individual,
            "variation": outcome});
        raw_event(
            &mut self.writer,
            &serde_json::json!({"event": "proposal",
            "proposal": proposal, "mc_observations": mc}),
        )?;
        self.proposals.push(proposal);
        Ok(())
    }

    pub(crate) fn generation(
        &mut self,
        generation: usize,
        pop: &[Individual],
        hv: f64,
    ) -> Result<(), String> {
        let active_ids: Vec<_> = pop.iter().map(|i| &i.id).collect();
        let row = serde_json::json!({"generation": generation,
            "active_ids": active_ids, "pareto_hypervolume": hv,
            "proposals": std::mem::take(&mut self.proposals)});
        raw_event(
            &mut self.writer,
            &serde_json::json!({"event": "survival",
            "generation": generation, "active_ids": active_ids, "pareto_hypervolume": hv}),
        )?;
        self.writer.flush().map_err(|e| e.to_string())?;
        self.generations.push(row);
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<(), String> {
        if !self.proposals.is_empty() {
            return Err("uncheckpointed raw proposals".into());
        }
        self.writer.flush().map_err(|e| e.to_string())?;
        self.document["generation_populations"] =
            serde_json::to_value(self.generations).map_err(|e| e.to_string())?;
        raw_write_json(&self.document_path, &self.document)
    }
}

/// Opt-in NEW raw export. Historical runs cannot be reconstructed from summaries.
/// An existing destination is rejected; interrupted runs have no complete manifest.
/// Equal-budget CT35 is additional reference work, outside the legacy five arms.
pub fn run_wave3_evolution_proof_with_raw_capture(
    seeds: &[u64],
    cfg: &EvolutionConfig,
    raw_root: &Path,
) -> Result<Wave3EvolutionProofReport, String> {
    let raw = RawProofCapture::new(raw_root, seeds, cfg)?;
    let report = run_wave3_evolution_proof_recorded(seeds, cfg, Some(&raw));
    match report {
        Ok(report) => {
            raw.finish(&report)?;
            Ok(report)
        }
        Err(error) => {
            let _ = raw.json(
                "capture-error.json",
                &serde_json::json!({"state": "incomplete", "reason": error}),
            );
            Err(error)
        }
    }
}

#[cfg(test)]
mod raw_capture_tests {
    use super::*;

    #[test]
    fn truth_raw_capture_keeps_rejected_and_failed_proposals_without_labels() {
        let cfg = EvolutionConfig {
            islands: 1,
            pop_per_island: 2,
            generations: 0,
            mc_samples: 4,
            ..EvolutionConfig::default()
        };
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/truth-raw-capture-tests")
            .join(format!(
                "negative-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let raw = RawProofCapture::new(&root, &[1, 2, 3, 4, 5], &cfg).unwrap();
        let base = evaluate_individual(
            "baseline-ct35".into(),
            DesignProgram::baseline_ct35(),
            0,
            0,
            &cfg,
        )
        .unwrap()
        .0;
        let mut capture = RawSeedCapture::new(
            &raw,
            1,
            Wave3ComparatorArm::B1RandomSearchExotic3d,
            &base,
            vec![],
        )
        .unwrap();
        let mut budget = ProposalBudget::default();
        let mut rejected = VariationOutcome::introduce(
            DesignProgram::baseline_ct35(),
            "synthetic_rejection_fixture",
            &mut Rng::new(1),
        )
        .unwrap();
        rejected.child = None;
        rejected.failure = Some("synthetic invalid proposal fixture".into());
        assert!(evaluate_proposal_recorded(
            Ok(rejected),
            "invalid".into(),
            0,
            0,
            &cfg,
            &mut budget,
            Some(&mut capture)
        )
        .unwrap()
        .is_none());
        let failed = VariationOutcome::introduce(
            DesignProgram::baseline_ct35(),
            "synthetic_evaluation_failure_fixture",
            &mut Rng::new(1),
        );
        let no_mc = EvolutionConfig {
            mc_samples: 0,
            ..cfg.clone()
        };
        assert!(evaluate_proposal_recorded(
            failed,
            "failed".into(),
            0,
            0,
            &no_mc,
            &mut budget,
            Some(&mut capture)
        )
        .unwrap()
        .is_none());
        assert_eq!(budget.proposals_invalid, 2);
        for row in &capture.proposals {
            assert_eq!(row["state"], "invalid");
            assert!(row["individual"].is_null());
            assert!(row["variation"]["failure"].is_string());
            assert!(row["variation"]["proposal"].is_object());
        }
        capture.generation(0, &[base], 0.).unwrap();
        capture.finish().unwrap();
        assert!(
            !root.join("manifest.json").exists(),
            "partial capture must not look complete"
        );
        let events =
            fs::read_to_string(root.join("wave4-proof-raw-B1_Random_Search-s1.jsonl")).unwrap();
        let events: Vec<serde_json::Value> = events
            .lines()
            .map(|r| serde_json::from_str(r).unwrap())
            .collect();
        assert!(events[2]["proposal"]["variation"]["failure"]
            .as_str()
            .unwrap()
            .contains("Monte Carlo"));
        assert_eq!(events[1]["mc_observations"], serde_json::json!([]));
    }
}
