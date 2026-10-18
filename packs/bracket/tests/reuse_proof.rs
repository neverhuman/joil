use crucible_contract::{keys::EvaluationKey, subject::ArtifactAcceptance};
use crucible_store::{
    manifest::{Metadata, ObjectKind},
    store::Store,
};
use pack_bracket::{data_card, run_campaign, BracketProgram, Campaign, InapplicabilityRule};
use std::collections::BTreeSet;
use std::{
    io::Write,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const INPUT: &str = include_str!("../../../tests/fixtures/v5-bracket/input.json");
const CAMPAIGN: &str = include_str!("../../../campaigns/bracket-v5-fixture.toml");
const CARD: &str = include_str!("../../../tests/fixtures/v5-bracket/expected-card.svg");

fn program() -> BracketProgram {
    serde_json::from_str(INPUT).unwrap()
}

#[test]
fn bounded_campaign_runs_with_exact_attempt_ledger_and_pack_card() {
    let campaign: Campaign = toml::from_str(CAMPAIGN).unwrap();
    assert_eq!(campaign.reservation().cpu_ms, 75);
    assert_eq!(campaign.reservation().gpu_ms, 0);
    let result = run_campaign(&campaign).unwrap();
    assert!(result.synthetic_only);
    assert_eq!((result.attempted, result.committed), (3, 3));
    assert_eq!(
        result
            .attempts
            .iter()
            .map(|a| a.ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert!(result
        .attempts
        .iter()
        .all(|a| a.subject == result.observation.subject));
    assert_eq!(result.observation.structural_state, "fixture_screen_pass");
    assert!(result
        .observation
        .physical_qualification
        .starts_with("blocked_"));
    let card = data_card(&campaign.fixture, &result.observation).unwrap();
    let expected = CARD.replace("{{SUBJECT_SHA256}}", result.observation.subject.as_str());
    assert_eq!(card, expected.trim_end());
    assert!(card.contains(result.observation.subject.as_str()));
    assert!(card.contains("Force [N]"));
    assert!(card.contains("Moment [N m]"));
    assert!(card.contains("qualification blocked"));
    assert!(!card.contains("flow"));
}

#[test]
fn absent_oil_judge_cannot_be_marked_not_applicable_by_caller_alone() {
    let mut campaign: Campaign = toml::from_str(CAMPAIGN).unwrap();
    campaign
        .inapplicability_rules
        .retain(|r| r.judge != "oil_grade");
    assert!(campaign.check().is_err());
    campaign.inapplicability_rules.push(InapplicabilityRule {
        judge: "oil_grade".into(),
        admitted_domain: "another_domain".into(),
        reason: "caller says so".into(),
    });
    assert!(campaign.check().is_err());
}

#[test]
fn nonempty_checkpoint_round_trips_and_resumes_without_losing_attempts() {
    let campaign: Campaign = toml::from_str(CAMPAIGN).unwrap();
    let root = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .expect("lane target directory required");
    let path = root.join(format!("bracket-kill-resume-{}.json", std::process::id()));
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "checkpoint_child_persists_nonempty_state",
            "--nocapture",
        ])
        .env("BRACKET_CHECKPOINT_PATH", &path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while !path.exists() && started.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        path.exists(),
        "checkpoint child did not persist its attempt"
    );
    child.kill().unwrap();
    let status = child.wait().unwrap();
    assert!(
        !status.success(),
        "child should have been killed after checkpoint sync"
    );
    let bytes = std::fs::read(&path).unwrap();
    let mut restored: pack_bracket::CampaignCheckpoint = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored.next_ordinal, 1);
    assert_eq!(restored.attempts.len(), 1);
    std::fs::remove_file(path).unwrap();
    pack_bracket::advance_campaign(&campaign, &mut restored, 8).unwrap();
    assert_eq!(restored.next_ordinal, 3);
    assert_eq!(
        restored
            .attempts
            .iter()
            .map(|a| a.ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert!(pack_bracket::advance_campaign(&campaign, &mut restored, 1).is_ok());
    assert_eq!(restored.attempts.len(), 3);
    let mut changed = campaign;
    changed.fixture.interface.bolt_centres_mm[0][0] = -18.0;
    assert!(pack_bracket::advance_campaign(&changed, &mut restored, 1).is_err());
}

#[test]
fn checkpoint_child_persists_nonempty_state() {
    let Some(path) = std::env::var_os("BRACKET_CHECKPOINT_PATH") else {
        return;
    };
    let campaign: Campaign = toml::from_str(CAMPAIGN).unwrap();
    let mut state = pack_bracket::start_campaign(&campaign).unwrap();
    pack_bracket::advance_campaign(&campaign, &mut state, 1).unwrap();
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(&serde_json::to_vec(&state).unwrap())
        .unwrap();
    file.sync_all().unwrap();
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[test]
fn material_and_bolt_interface_changes_change_subject_and_evaluation_identity() {
    let base = program();
    let id = base.identity().unwrap();
    let key = pack_bracket::evaluation_key(&base)
        .unwrap()
        .identity()
        .unwrap();
    let mut changed = base.clone();
    changed.interface.bolt_centres_mm[0][0] = -18.0;
    assert_ne!(id, changed.identity().unwrap());
    assert_ne!(
        key,
        pack_bracket::evaluation_key(&changed)
            .unwrap()
            .identity()
            .unwrap()
    );
    let mut changed_material = base.clone();
    changed_material.material.fixture_yield_mpa = 230.0;
    assert_ne!(id, changed_material.identity().unwrap());
    assert_ne!(
        base.observe().unwrap().fixture_reserve_factor,
        changed_material.observe().unwrap().fixture_reserve_factor
    );
}

#[test]
fn invalid_geometry_and_hydrocyclone_shaped_fields_are_rejected() {
    let mut invalid = program();
    invalid.support.thickness_mm = 2.0;
    assert!(invalid.check().is_err());
    let unknown_field = CAMPAIGN.replace("attempts = 3", "attempts = 3\nflow_rate_m3_s = 0.1");
    assert!(toml::from_str::<Campaign>(&unknown_field).is_err());
}

#[test]
fn compatible_observation_reuses_exact_cache_and_bolt_change_misses() {
    let root = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .expect("lane target directory required");
    let path = root.join(format!("bracket-store-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    let store = Store::open(&path).unwrap();
    let subject = program();
    let key = pack_bracket::evaluation_key(&subject).unwrap();
    let key_id = key.identity().unwrap();
    let geometry = serde_json::to_vec(&("synthetic-bracket-geometry/1", &subject)).unwrap();
    let surface = serde_json::to_vec(&(
        "synthetic-bracket-surface/1",
        &subject.interface,
        &subject.support,
    ))
    .unwrap();
    let interface = serde_json::to_vec(&("pack-bracket-interface/1", &subject.interface)).unwrap();
    let refs = &key.subject.artifacts;
    let acceptances = refs
        .iter()
        .map(|a| {
            serde_json::to_vec(&ArtifactAcceptance {
                artifact: a.blob.clone(),
                format: a.format,
                length_unit: a.length_unit,
                approximation_bound_m: a.approximation_bound_m,
                interface_contract: key.subject.interface_contract.clone(),
            })
            .unwrap()
        })
        .collect::<Vec<_>>();
    let material_state =
        serde_json::to_vec(&("pack-bracket-material-state/1", &subject.material)).unwrap();
    let load_scenario =
        serde_json::to_vec(&("pack-bracket-load-scenario/1", &subject.load)).unwrap();
    let mut required_bytes = vec![
        geometry,
        surface,
        interface.clone(),
        acceptances[0].clone(),
        acceptances[1].clone(),
        material_state,
        load_scenario,
    ];
    required_bytes.extend(
        [
            "elementary-bracket-screen-v1",
            "software-fixture",
            "closed-form-rectangular-beam",
            "no-mesh-elementary-screen",
            "deterministic",
            "none",
            "pack-bracket-extractor-v1",
        ]
        .into_iter()
        .map(|s| s.as_bytes().to_vec()),
    );
    let mut input_objects = BTreeSet::new();
    for bytes in required_bytes {
        input_objects.insert(
            store
                .publish(&bytes, Metadata::private(ObjectKind::Input))
                .unwrap(),
        );
    }
    let observation = serde_json::to_vec(&subject.observe().unwrap()).unwrap();
    let result = store
        .publish(
            &observation,
            Metadata {
                kind: ObjectKind::Observation,
                semantic: Some(key_id),
                dependencies: input_objects,
                ..Metadata::private(ObjectKind::Observation)
            },
        )
        .unwrap();
    store.bind_exact_cache(&key, &result).unwrap();
    assert!(store.lookup_exact_cache(&key).unwrap().is_some());
    let mut changed = subject;
    changed.interface.bolt_centres_mm[0][0] = -18.0;
    let changed_key: EvaluationKey = pack_bracket::evaluation_key(&changed).unwrap();
    assert!(store.lookup_exact_cache(&changed_key).unwrap().is_none());
    drop(store);
    std::fs::remove_dir_all(path).unwrap();
}
