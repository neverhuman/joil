//! NEW raw contract tests. Tiny sandbox runs are software fixtures, not physics evidence.
use joil_core::evolve::{
    run_wave3_evolution_proof, run_wave3_evolution_proof_with_raw_capture, EvolutionConfig,
};
use joil_evidence::campaign::CampaignMode;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn destination(label: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/truth-raw-capture-tests")
        .join(format!(
            "{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
}
fn config() -> EvolutionConfig {
    EvolutionConfig {
        islands: 1,
        pop_per_island: 2,
        generations: 1,
        mc_samples: 4,
        ..EvolutionConfig::default()
    }
}
fn read(path: impl AsRef<std::path::Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn truth_raw_capture_preserves_search_and_retains_actual_observations() {
    let seeds = [4567, 4568, 4569, 4570, 4571];
    let cfg = config();
    let plain = run_wave3_evolution_proof(&seeds, &cfg).unwrap();
    let root = destination("parity");
    let captured = run_wave3_evolution_proof_with_raw_capture(&seeds, &cfg, &root).unwrap();
    assert_eq!(
        serde_json::to_value(plain).unwrap(),
        serde_json::to_value(captured).unwrap(),
        "capture and extra reference work must not change the five-arm report"
    );
    let manifest = read(root.join("manifest.json"));
    assert_eq!(manifest["state"], "complete");
    let files = manifest["files"].as_array().unwrap();
    let listed: BTreeSet<_> = files
        .iter()
        .map(|f| f["path"].as_str().unwrap().to_owned())
        .collect();
    let actual: BTreeSet<_> = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_str().unwrap().to_owned())
        .filter(|s| s != "manifest.json" && s != "manifest.sha256")
        .collect();
    assert_eq!(listed, actual);
    for file in files {
        let bytes = fs::read(root.join(file["path"].as_str().unwrap())).unwrap();
        assert_eq!(file["bytes"], bytes.len());
        assert_eq!(file["sha256"], format!("{:x}", Sha256::digest(&bytes)));
    }
    let digest = fs::read_to_string(root.join("manifest.sha256")).unwrap();
    assert_eq!(
        digest,
        format!(
            "{:x}  manifest.json\n",
            Sha256::digest(fs::read(root.join("manifest.json")).unwrap())
        )
    );
    let provenance = read(root.join("capture-provenance.json"));
    assert!(provenance["contract_status"]
        .as_str()
        .unwrap()
        .starts_with("NEW"));
    assert_eq!(provenance["rng"]["algorithm"], "SplitMix64");
    assert_eq!(
        provenance["compiled_sources_sha256"]["evolve.rs"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    let mut seed_files = 0;
    for name in listed
        .iter()
        .filter(|s| s.starts_with("wave4-proof-raw-") && s.ends_with(".json"))
    {
        let document = read(root.join(name));
        if document["schema"] != "joil.wave4-raw-seed.v1" {
            continue;
        }
        seed_files += 1;
        assert_eq!(document["config"]["proposal_budget"], 4);
        let events: Vec<Value> = fs::read_to_string(root.join(name.replace(".json", ".jsonl")))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let mut seen = BTreeSet::from(["baseline-ct35".to_owned()]);
        let proposals: Vec<_> = events.iter().filter(|e| e["event"] == "proposal").collect();
        assert_eq!(proposals.len(), 4);
        for proposal in &proposals {
            let ind = &proposal["proposal"]["individual"];
            if proposal["proposal"]["state"] == "invalid" {
                assert!(ind.is_null());
                continue;
            }
            assert!(seen.insert(ind["id"].as_str().unwrap().to_owned()));
            let mc = proposal["mc_observations"].as_array().unwrap();
            assert_eq!(mc.len(), 4);
            let mut rng = joil_core::program::Rng::new(
                document["seed"].as_u64().unwrap()
                    ^ u64::from_str_radix(&ind["genome_sha256"].as_str().unwrap()[..16], 16)
                        .unwrap(),
            );
            for (i, draw) in mc.iter().enumerate() {
                assert_eq!(draw["draw_index"], i);
                for (field, lo, hi) in [
                    ("cin_mg_l", 475., 525.),
                    ("feed_d50_um", 35., 41.),
                    ("split_delta", -0.0012, 0.0012),
                    ("wall_tolerance_fraction", -0.05, 0.05),
                ] {
                    assert!((draw[field].as_f64().unwrap() - rng.range(lo, hi)).abs() < 1e-12);
                }
            }
            let upper = mc
                .iter()
                .map(|r| r["metrics"]["cout_mg_l"].as_f64().unwrap())
                .fold(f64::NEG_INFINITY, f64::max);
            assert!(
                (ind["mc_robustness"]["cout_sample_screen_upper_mg_l"]
                    .as_f64()
                    .unwrap()
                    - upper)
                    .abs()
                    < 1e-12
            );
        }
        let survivals: Vec<_> = events.iter().filter(|e| e["event"] == "survival").collect();
        assert_eq!(survivals.len(), 2);
        for (g, row) in document["generation_populations"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            assert_eq!(row["generation"], g);
            assert_eq!(row["active_ids"], survivals[g]["active_ids"]);
            assert_eq!(row["proposals"].as_array().unwrap().len(), 2);
            for id in row["active_ids"].as_array().unwrap() {
                assert!(seen.contains(id.as_str().unwrap()));
            }
        }
    }
    assert_eq!(
        seed_files, 30,
        "all five arms plus separately charged CT35, five seeds each"
    );
    let holdouts = read(root.join("wave4-proof-raw-holdouts.json"));
    assert_eq!(holdouts["split_basis"], "authored_family_screen");
    assert!(!holdouts["folds"].as_array().unwrap().is_empty());
    for fold in holdouts["folds"].as_array().unwrap() {
        let train: BTreeSet<_> = fold["train_subject_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect();
        for row in fold["predictions"].as_array().unwrap() {
            assert!(!train.contains(row["subject_id"].as_str().unwrap()));
            assert!(row["target_kpi"].is_null() && row["predicted_kpi"].is_null());
            assert!(row["sigma_cout"].as_f64().unwrap() >= 0.);
        }
    }
    let before = fs::read(root.join("manifest.json")).unwrap();
    assert!(run_wave3_evolution_proof_with_raw_capture(&seeds, &cfg, &root).is_err());
    assert_eq!(before, fs::read(root.join("manifest.json")).unwrap());
}

#[test]
fn truth_raw_capture_refuses_invalid_or_claim_bearing_campaign_before_writing() {
    let seeds = [4567, 4568, 4569, 4570, 4571];
    for (label, values, cfg) in [
        ("duplicates", vec![4567; 5], config()),
        ("too-few", vec![4567; 4], config()),
        (
            "claim",
            seeds.to_vec(),
            EvolutionConfig {
                campaign_mode: CampaignMode::ClaimBearing,
                ..config()
            },
        ),
        (
            "no-mc",
            seeds.to_vec(),
            EvolutionConfig {
                mc_samples: 0,
                ..config()
            },
        ),
        (
            "no-pop",
            seeds.to_vec(),
            EvolutionConfig {
                islands: 0,
                ..config()
            },
        ),
        (
            "overflow",
            seeds.to_vec(),
            EvolutionConfig {
                generations: usize::MAX,
                ..config()
            },
        ),
    ] {
        let root = destination(label);
        assert!(run_wave3_evolution_proof_with_raw_capture(&values, &cfg, &root).is_err());
        assert!(!root.exists(), "{label}: invalid request created output");
    }
}
